# Thread Pool Architecture Design - Phase 2

**Goal**: Achieve 2× speedup from parallelism by processing multiple SIMD batches in parallel
**Combined with Phase 1**: SIMD (3×) × Threading (2×) = **6× total speedup**

## Current Bottleneck Analysis

### SIMD Path (Current - Sequential)

```rust
// Current: Process SIMD batches sequentially
#[cfg(target_arch = "x86_64")]
if is_avx2_supported() && self.voices.len() >= 8 {
    for batch_idx in 0..(num_batches) {
        process_voice_batch_simd(voices[batch*8..(batch+1)*8], ...);  // Sequential!
    }
    return output;
}
```

**Problem**: SIMD batches processed one at a time, not utilizing multiple cores

**Example with 64 voices**:
- 8 SIMD batches of 8 voices each
- Processed sequentially on 1 core
- 15 other cores sitting idle!

### Rayon Path (Fallback - Overhead)

```rust
// Fallback: Rayon spawns threads every buffer
let voice_buffers: Vec<_> = self.voices
    .par_iter_mut()  // Thread spawn overhead EVERY call
    .map(|voice| voice.process_stereo())
    .collect();
```

**Problem**: Rayon spawns/joins threads every audio buffer (~11ms)
- Thread creation: ~50μs per spawn
- For 16 threads: 800μs overhead
- On an 11.6ms budget: 7% wasted on thread management!

## Solution: Persistent Thread Pool with SIMD Batches

### Architecture

```
┌──────────────────────────────────────────────────────────┐
│ Audio Thread (Main)                                      │
│  - Receives audio callback every 11.6ms                  │
│  - Distributes SIMD batches to worker threads            │
│  - Waits for completion                                  │
│  - Mixes output                                          │
└────────────┬─────────────────────────────────────────────┘
             │
             ├─ Distribute work ─┐
             │                    │
    ┌────────▼─────┐    ┌────────▼─────┐    ┌────────▼─────┐
    │ Worker 1     │    │ Worker 2     │    │ Worker N     │
    │ (Persistent) │    │ (Persistent) │    │ (Persistent) │
    │              │    │              │    │              │
    │ Process:     │    │ Process:     │    │ Process:     │
    │ Batch 0      │    │ Batch 1      │    │ Batch N      │
    │ (SIMD: 8v)   │    │ (SIMD: 8v)   │    │ (SIMD: 8v)   │
    └────────┬─────┘    └────────┬─────┘    └────────┬─────┘
             │                    │                    │
             └────────── Sync ────┴────────────────────┘
                        │
                ┌───────▼────────┐
                │ Output Buffer  │
                │ (Mixed Results)│
                └────────────────┘
```

### Key Components

#### 1. VoiceThreadPool (Persistent Workers)

```rust
struct VoiceThreadPool {
    workers: Vec<Worker>,
    work_tx: crossbeam::channel::Sender<WorkItem>,
    result_rx: crossbeam::channel::Receiver<WorkResult>,
    num_workers: usize,
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

enum WorkItem {
    ProcessBatch {
        batch_id: usize,
        // Voice data passed via shared memory (lock-free)
    },
    Shutdown,
}

struct WorkResult {
    batch_id: usize,
    // Output data written to shared buffer
}
```

#### 2. Lock-Free Communication (Crossbeam)

**Why crossbeam over std::sync**:
- Zero allocation channels (important for real-time)
- Lock-free bounded queues (no mutex contention)
- Better performance than std::sync::mpsc

```rust
use crossbeam::channel::{bounded, Sender, Receiver};

// Create bounded channels (size = num_workers)
let (work_tx, work_rx) = bounded(num_workers);
let (result_tx, result_rx) = bounded(num_workers);
```

#### 3. Shared Memory for Voice Data (Lock-Free)

**Problem**: Can't send `&mut [Voice]` across threads safely

**Solution**: Atomic pointers with double-buffering

```rust
struct SharedVoiceData {
    // Double-buffered: one for reading, one for writing
    buffers: [Vec<Voice>; 2],
    active_idx: AtomicUsize,
}

impl SharedVoiceData {
    fn get_read_buffer(&self) -> &[Voice] {
        let idx = self.active_idx.load(Ordering::Acquire);
        &self.buffers[idx]
    }

    fn swap_buffers(&mut self) {
        let current = self.active_idx.load(Ordering::Acquire);
        self.active_idx.store(1 - current, Ordering::Release);
    }
}
```

#### 4. Worker Thread Logic

```rust
fn worker_main(
    id: usize,
    work_rx: Receiver<WorkItem>,
    result_tx: Sender<WorkResult>,
    shared_voices: Arc<RwLock<Vec<Voice>>>,  // Simplified for now
) {
    // Pin thread to CPU core (affinity)
    #[cfg(target_os = "linux")]
    set_cpu_affinity(id);

    loop {
        match work_rx.recv() {
            Ok(WorkItem::ProcessBatch { batch_id }) => {
                // Lock voices for reading (RwLock allows multiple readers)
                let voices = shared_voices.read().unwrap();

                // Process SIMD batch
                let start = batch_id * 8;
                let end = start + 8;
                let batch = &voices[start..end];

                // SIMD processing (3× speedup)
                let output = process_voice_batch_simd(batch, buffer_size);

                // Send result back
                result_tx.send(WorkResult { batch_id, output });
            }
            Ok(WorkItem::Shutdown) => break,
            Err(_) => break,  // Channel closed
        }
    }
}
```

### Integration into VoiceManager

```rust
pub struct VoiceManager {
    voices: Vec<Voice>,
    thread_pool: Option<VoiceThreadPool>,  // New!
    parallel_threshold: usize,
    // ... existing fields
}

impl VoiceManager {
    pub fn new() -> Self {
        let num_cores = num_cpus::get();
        let thread_pool = VoiceThreadPool::new(num_cores - 1);  // Reserve 1 core for audio thread

        Self {
            thread_pool: Some(thread_pool),
            parallel_threshold: 8,  // Use threading when >= 8 voices
            // ...
        }
    }

    pub fn process_buffer_per_node(&mut self, buffer_size: usize) -> Vec<HashMap<usize, f32>> {
        // SIMD + Threading fast path
        #[cfg(target_arch = "x86_64")]
        if is_avx2_supported() && self.voices.len() >= 16 {  // Need >= 2 batches for parallelism
            return self.process_buffer_parallel_simd(buffer_size);
        }

        // SIMD only (sequential batches)
        #[cfg(target_arch = "x86_64")]
        if is_avx2_supported() && self.voices.len() >= 8 {
            return self.process_buffer_simd_sequential(buffer_size);  // Current implementation
        }

        // Fallback: Sequential scalar
        self.process_buffer_sequential(buffer_size)
    }

    fn process_buffer_parallel_simd(&mut self, buffer_size: usize) -> Vec<HashMap<usize, f32>> {
        let mut output = vec![HashMap::new(); buffer_size];
        let num_batches = self.voices.len() / 8;

        // Distribute batches to workers
        let pool = self.thread_pool.as_ref().unwrap();

        for batch_id in 0..num_batches {
            pool.submit_work(WorkItem::ProcessBatch { batch_id });
        }

        // Wait for results
        for _ in 0..num_batches {
            let result = pool.wait_for_result();
            // Merge result into output
            merge_batch_output(&mut output, result);
        }

        // Process remainder voices (< 8) on main thread
        let remainder_start = num_batches * 8;
        for voice in &mut self.voices[remainder_start..] {
            // ... scalar processing
        }

        output
    }
}
```

## Performance Analysis

### Overhead Breakdown

**Current (Rayon per buffer)**:
- Thread spawn: ~50μs × 16 cores = 800μs
- Thread join: ~50μs × 16 cores = 800μs
- Total overhead: ~1.6ms per buffer (14% of 11.6ms budget!)

**Proposed (Persistent pool)**:
- Thread spawn: Once at startup (amortized to 0)
- Work distribution: Channel send ~100ns × 8 batches = 800ns
- Result collection: Channel recv ~100ns × 8 batches = 800ns
- Total overhead: ~1.6μs per buffer (0.014% of budget!)

**Overhead reduction**: 1.6ms → 1.6μs = **1000× better!**

### Expected Speedup

**Scenario: 64 voices on 8-core system**

**Current (SIMD only, sequential batches)**:
- 8 SIMD batches × 100μs/batch = 800μs
- 1 core utilized, 7 cores idle

**Proposed (SIMD + Threading)**:
- 8 SIMD batches distributed across 7 worker cores
- Each core processes ~1.14 batches
- Time: 114μs (ideal) + overhead
- Actual: ~150μs (accounting for synchronization)

**Speedup**: 800μs / 150μs = **5.3× faster**

**But we already have 3× from SIMD**, so the threading adds:
- 800μs → 150μs = additional **5.3× on top of SIMD**
- Wait, this doesn't make sense. Let me recalculate.

Actually, the SIMD speedup is **within** the batch processing. The threading speedup is **across** batches.

**Without SIMD or threading** (baseline):
- 64 voices × 30μs/voice = 1920μs

**With SIMD only** (Phase 1):
- 8 batches × (8 voices / 3× SIMD) = 8 × 80μs = 640μs
- Speedup: 1920 / 640 = **3× (SIMD)**

**With SIMD + Threading** (Phase 2):
- 8 batches distributed across 7 cores
- 8 batches / 7 cores = 1.14 batches/core
- Time per core: 1.14 × 80μs = 91μs
- Total (slowest core): ~100μs (with overhead)
- Speedup vs SIMD-only: 640 / 100 = **6.4×**
- Speedup vs baseline: 1920 / 100 = **19.2×**

**Whoa! That's way better than our 2× estimate!**

### Revised Expectations

| Optimization | Time | Speedup vs Baseline | Cumulative |
|--------------|------|---------------------|------------|
| Baseline | 1920μs | 1× | 1× |
| SIMD (Phase 1) | 640μs | 3× | 3× |
| Threading (Phase 2) | 100μs | 19.2× | **19.2×** |

**This exceeds our 10× goal!** 🎉

But wait, let me be more conservative. Real-world factors:
- Synchronization overhead
- Cache contention
- Memory bandwidth limits
- Non-ideal work distribution

**Conservative estimate**:
- SIMD: 3× (validated)
- Threading: 2× (conservative, accounts for overhead)
- **Combined: 6× total speedup**

**Optimistic (if everything goes well)**:
- **15-20× speedup** (based on ideal parallel scaling)

## Implementation Roadmap

### Week 1: Thread Pool Foundation

**Day 1-2: Design and prototype**
- Design lock-free work queue
- Prototype worker thread
- Test crossbeam channels

**Day 3-4: Implement VoiceThreadPool**
- Create worker threads
- Implement work distribution
- Add shutdown mechanism

**Day 5: Test pool in isolation**
- Unit tests for thread pool
- Verify no deadlocks
- Test with mock workloads

### Week 2: Integration and Optimization

**Day 1-3: Integrate into VoiceManager**
- Replace sequential SIMD batches with parallel
- Add shared memory for voice data
- Handle synchronization

**Day 4-5: CPU Affinity and Tuning**
- Pin threads to cores
- Tune work distribution
- Measure cache effects

**Day 6-7: Testing and Profiling**
- Profile with q.ph pattern
- Measure actual speedup
- Fix any race conditions or performance issues

### Success Criteria

- [  ] Thread pool starts up without errors
- [  ] Workers process batches correctly
- [  ] Audio output matches scalar reference
- [  ] 2× minimum speedup on 4+ core systems
- [  ] 4-6× target speedup on 8+ core systems
- [  ] No audio dropouts under heavy load
- [  ] Graceful degradation on single-core systems

## Risk Mitigation

### Risk 1: Race Conditions
**Mitigation**: Extensive stress testing, ThreadSanitizer, lock-free data structures

### Risk 2: Synchronization Overhead
**Mitigation**: Measure overhead early, optimize channel usage, batch work items

### Risk 3: Cache Thrashing
**Mitigation**: CPU affinity pinning, cache-aligned data structures, NUMA awareness

### Risk 4: Real-Time Guarantees
**Mitigation**: Bounded channels, timeout handling, fallback to sequential on failure

## Alternative Approaches Considered

### 1. Keep Rayon, Optimize Usage
**Pros**: Less code to write
**Cons**: Can't eliminate thread spawn overhead, less control

### 2. Thread-per-Core Model
**Pros**: Simpler design
**Cons**: Harder to balance load, less flexible

### 3. Work-Stealing Queue (like Rayon)
**Pros**: Better load balancing
**Cons**: More complexity, higher overhead
**Decision**: Start simple (static distribution), add work-stealing later if needed

## Dependencies

**New crates needed**:
```toml
crossbeam = "0.8"  # Lock-free channels and data structures
num_cpus = "1.16"  # CPU count detection

# Optional (for CPU affinity):
core_affinity = "0.8"  # Thread pinning (Linux/Windows/macOS)
```

## Next Steps

1. **Add dependencies to Cargo.toml**
2. **Create `src/thread_pool.rs`**
3. **Implement VoiceThreadPool**
4. **Test in isolation**
5. **Integrate into VoiceManager**
6. **Profile and optimize**

---

**Status**: Design complete, ready for implementation
**Expected timeline**: 2 weeks
**Expected benefit**: 2-6× additional speedup (combined with SIMD: 6-20× total)
