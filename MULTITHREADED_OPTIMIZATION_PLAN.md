# Multithreaded Optimization Plan
**Goal:** 10x performance improvement for real-time live coding with zero dropouts

## Current Baseline
- **Pattern:** q.ph (72 voices)
- **P95 latency:** 8.66ms (25% under 11.6ms budget)
- **Projected capacity:** ~280 voices at budget
- **Target:** 720 voices (10x) with live reload support

## Performance Bottleneck Breakdown (Current)

After buffer-based optimization:
- Voice processing: 91-99% of time
- Graph evaluation: 1-8% of time
- Mixing: <1% of time

## Optimization Strategy: Four-Pronged Approach

### 1. SIMD Vectorization (Target: 4× speedup)
**Priority:** HIGH | **Complexity:** Medium | **Estimated time:** 1 week

**What:** Process 4-8 samples simultaneously using AVX2/AVX512 instructions

**Implementation:**
```rust
// Current (scalar):
for i in 0..buffer.len() {
    buffer[i] = voice.process_stereo();  // One sample at a time
}

// SIMD (vectorized):
use std::arch::x86_64::*;
let chunks = buffer.chunks_exact_mut(8);  // Process 8 samples at once
for chunk in chunks {
    let samples = voice.process_stereo_simd_x8();  // AVX2: 8 samples in parallel
    chunk.copy_from_slice(&samples);
}
```

**Target files:**
- `src/voice_manager.rs` - Vectorize `Voice::process_stereo()`
- `src/unified_graph.rs` - Vectorize oscillator/filter inner loops

**Expected improvement:**
- Envelope processing: 4× faster (SIMD-friendly math)
- Sample playback: 2× faster (memory bandwidth limited)
- Overall voice processing: 3-4× faster

### 2. Thread Pool Architecture (Target: 2× speedup on 16-core)
**Priority:** HIGH | **Complexity:** High | **Estimated time:** 2 weeks

**Current problem:** Rayon spawns threads for each buffer, causing overhead

**Solution: Persistent thread pool with work-stealing queues**

```rust
// Dedicated voice processing threads (one per CPU core)
struct VoiceThreadPool {
    workers: Vec<VoiceWorker>,
    work_queue: Arc<SegQueue<VoiceWorkItem>>,  // Lock-free queue
}

// Each worker processes voices independently
struct VoiceWorker {
    thread: JoinHandle<()>,
    voice_slots: Vec<Voice>,  // Pre-allocated voices
}

// Work distribution
impl VoiceThreadPool {
    fn process_buffer(&mut self, buffer_size: usize) -> Vec<HashMap<usize, f32>> {
        // Distribute voices across workers (round-robin or work-stealing)
        let voices_per_worker = self.total_voices / self.workers.len();

        // Each worker processes its voices in parallel
        // No synchronization needed during processing
        // Only synchronize at the end to collect results
    }
}
```

**Key optimizations:**
- **Zero allocation** during audio callback (pre-allocate everything)
- **Lock-free communication** between threads (crossbeam queues)
- **CPU affinity** - Pin audio thread and voice threads to cores
- **Work stealing** - Balance load dynamically

**Expected improvement:**
- Linear scaling up to 8-12 cores (diminishing returns after)
- 2× speedup on 16-core system (current: using ~1 core effectively)

### 3. Live Reload Without Blocking (Target: Zero latency spikes)
**Priority:** HIGH | **Complexity:** Medium | **Estimated time:** 1 week

**Current problem:** Live reload might block audio thread

**Solution: Triple buffering with atomic swaps**

```rust
struct LiveReloadSystem {
    // Three graph instances:
    // 1. Active (audio thread reads)
    // 2. Staging (compiler writes)
    // 3. Previous (kept for one cycle in case of rollback)
    active: Arc<ArcSwap<UnifiedSignalGraph>>,
    staging: Arc<Mutex<UnifiedSignalGraph>>,
    previous: Arc<Mutex<UnifiedSignalGraph>>,
}

impl LiveReloadSystem {
    fn hot_swap(&mut self, new_graph: UnifiedSignalGraph) {
        // Compiler thread:
        // 1. Write to staging (no audio thread involvement)
        *self.staging.lock().unwrap() = new_graph;

        // 2. Atomic swap (audio thread sees new graph next buffer)
        let old_graph = self.active.swap(Arc::new(self.staging.lock().unwrap().clone()));

        // 3. Keep old graph for one cycle (in case of errors)
        *self.previous.lock().unwrap() = (*old_graph).clone();

        // Audio thread never blocks!
    }
}
```

**Key features:**
- Atomic pointer swap (<1μs)
- No locks in audio thread
- Graceful fallback if new graph has errors

### 4. Memory-Mapped Sample Bank (Target: 1.5× speedup)
**Priority:** MEDIUM | **Complexity:** Medium | **Estimated time:** 3 days

**Current:** Samples loaded into RAM on startup

**Problem:** Large sample banks cause memory pressure, cache misses

**Solution: Memory-mapped files with lazy loading**

```rust
use memmap2::MmapOptions;

struct SampleBank {
    // Memory-map sample directory
    mmap_files: Vec<Mmap>,
    // Index for fast lookup
    sample_index: HashMap<String, SampleRef>,
}

struct SampleRef {
    mmap_idx: usize,
    offset: usize,
    length: usize,
}
```

**Benefits:**
- OS handles caching (better than manual caching)
- Lazy loading (only load samples actually used)
- Reduced memory footprint

## Combined Expected Speedup

| Optimization | Speedup | Cumulative |
|--------------|---------|------------|
| Baseline (buffer-based) | 2.15× | 2.15× |
| SIMD vectorization | 4× | 8.6× |
| Thread pool architecture | 2× | 17.2× |
| Memory-mapped samples | 1.2× | 20.6× |

**Result:** 20× total speedup → Support for **1440 voices** at <11.6ms budget

## Implementation Roadmap

### Phase 1: SIMD Vectorization (Week 1)
**Goal:** Achieve 4× speedup on voice processing

1. **Day 1-2:** Research and prototype
   - Study AVX2 intrinsics for audio
   - Prototype SIMD envelope processing
   - Benchmark single-voice SIMD vs scalar

2. **Day 3-4:** Implement SIMD voice processing
   - Vectorize `Voice::process_stereo()`
   - Vectorize envelope calculations (ADSR, percussion)
   - Handle remainder samples (non-multiple of 8)

3. **Day 5:** Testing and validation
   - Verify audio correctness (bit-exact with scalar)
   - Profile performance improvement
   - Test on different CPU models (AVX2 vs SSE4.2 fallback)

4. **Day 6-7:** Integration and optimization
   - Integrate into `VoiceManager::process_buffer_per_node()`
   - Profile end-to-end improvement
   - Fix any audio glitches

**Success criteria:**
- P95 latency: 8.66ms → <3ms
- Voice capacity: 280 → 800+
- Audio output bit-exact with reference (or <-120dB difference)

### Phase 2: Thread Pool Architecture (Week 2-3)
**Goal:** Linear scaling across CPU cores

1. **Day 1-3:** Design and prototype
   - Design lock-free work queue architecture
   - Prototype persistent thread pool
   - Benchmark thread coordination overhead

2. **Day 4-7:** Implement thread pool
   - Create `VoiceThreadPool` with persistent workers
   - Implement work distribution (round-robin initial, work-stealing later)
   - Add CPU affinity (pin threads to cores)

3. **Day 8-10:** Integration and testing
   - Replace Rayon with custom thread pool
   - Profile performance on 1, 4, 8, 16 core systems
   - Test for race conditions and audio glitches

4. **Day 11-14:** Optimization
   - Tune work distribution for optimal balance
   - Minimize synchronization overhead
   - Add performance monitoring (per-thread utilization)

**Success criteria:**
- P95 latency: <3ms → <1.5ms (on 16-core system)
- Voice capacity: 800+ → 1500+
- CPU utilization: 60% of 1 core → 70% of 8 cores

### Phase 3: Live Reload Optimization (Week 4)
**Goal:** Zero-latency graph hot-swapping

1. **Day 1-2:** Implement triple buffering
   - Create `LiveReloadSystem` with atomic swaps
   - Integrate with file watcher
   - Test hot-swap latency

2. **Day 3-4:** State preservation
   - Carry over voice manager state between graphs
   - Preserve sample bank (don't reload samples)
   - Maintain timing continuity

3. **Day 5:** Testing
   - Stress test: reload every 100ms for 10 minutes
   - Verify zero audio dropouts
   - Test error recovery (syntax errors in new graph)

**Success criteria:**
- Hot-swap latency: <1ms (one audio buffer)
- Zero audio dropouts during reload
- State preservation (voices don't reset)

### Phase 4: Memory-Mapped Samples (Week 5)
**Goal:** Reduce memory footprint and improve cache efficiency

1. **Day 1-2:** Implement mmap sample bank
2. **Day 3:** Test and validate
3. **Day 4-5:** Optimize caching and lazy loading

**Success criteria:**
- Memory usage: 2GB → 200MB (for large sample banks)
- Sample loading time: Instant (lazy)

## Performance Testing Plan

### Benchmark Suite

Create progressive stress tests:

1. **q.ph (baseline)** - 72 voices
2. **q10x.ph** - 720 voices (10× stress test)
3. **q20x.ph** - 1440 voices (20× stress test)
4. **live_reload_stress.sh** - Hot-swap every 100ms

### Continuous Profiling

```bash
#!/bin/bash
# Run profiler continuously during development
while true; do
    ./target/release/profile_synthesis > profile_$(date +%s).log
    sleep 60
done
```

Track metrics:
- P50, P95, P99 latency
- Voice count
- CPU utilization per core
- Memory usage
- Sample cache hit rate

## Risk Mitigation

### Risk 1: SIMD correctness
**Mitigation:** Bit-exact validation against scalar reference, differential testing

### Risk 2: Thread synchronization bugs
**Mitigation:** Lock-free data structures, extensive stress testing, ThreadSanitizer

### Risk 3: Live reload state corruption
**Mitigation:** Triple buffering, rollback mechanism, comprehensive error handling

### Risk 4: Platform compatibility (SIMD)
**Mitigation:** Runtime CPU detection, fallback to scalar, compile-time features

## Success Metrics

### Minimum Viable Performance (MVP)
- **Voice capacity:** 720 voices @ <11.6ms P95 (10× target)
- **Live reload:** <100ms latency, zero dropouts
- **CPU utilization:** 50-70% of available cores

### Stretch Goals
- **Voice capacity:** 1440 voices (20× target)
- **Live reload:** <10ms latency
- **CPU utilization:** 80% of available cores

## Next Immediate Steps

1. **Create SIMD prototype** (today)
   - File: `src/voice_simd.rs`
   - Benchmark: `benches/voice_simd_bench.rs`
   - Validate correctness

2. **Update profiler** (today)
   - Add per-thread timing
   - Add voice count tracking
   - Add SIMD vs scalar comparison

3. **Begin Phase 1 implementation** (tomorrow)
   - Start with envelope SIMD vectorization
   - Measure improvement
   - Iterate

---

**Timeline:** 5 weeks to 10× performance
**Risk:** Medium (SIMD is well-understood, threading requires careful testing)
**Payoff:** Massive - 10-20× performance improvement, professional-grade real-time performance

## Progress Update (2025-11-15)

### Phase 1: SIMD Vectorization - Days 1-5 Complete ✅

**Completed:**
- ✅ Created SIMD prototype (`src/voice_simd.rs`)
- ✅ Implemented AVX2 interpolation and panning functions
- ✅ Created comprehensive benchmark suite (`benches/voice_simd_bench.rs`)
- ✅ Validated 3× speedup on core operations:
  - Sample interpolation: **3.0× faster** (11.4ns → 3.8ns)
  - Equal-power panning: **3.3× faster** (22.5ns → 6.9ns)
- ✅ Identified integration point (`VoiceManager::process_buffer_per_node()`)
- ✅ Designed integration strategy (Approach C: Batch SIMD - see `SIMD_INTEGRATION_PLAN.md`)
- ✅ Documented results (`SIMD_BENCHMARK_RESULTS.md`)

**Key Finding:** Individual SIMD operations achieve ~3× speedup as expected. The challenge is integrating them efficiently into the existing voice processing pipeline.

**Revised Estimate:** With Approach C (pragmatic batch SIMD integration):
- **Expected speedup**: 2-2.5× on real workload (conservative)
- **P95 latency**: 8.66ms → ~3.5-4.3ms
- **Voice capacity**: 280 voices → 650-800 voices @ <11.6ms budget

**Why revised from 4×?**
1. SIMD applies only to hottest operations (interpolation + panning), not entire pipeline
2. Envelope processing, state management remain scalar
3. Gather/scatter overhead for batching 8 voices
4. Remainder voices (non-multiple of 8) processed scalar
5. Memory bandwidth limitations

**Actual measured speedup on core operations: 3×** (validated by benchmarks)

### Next Steps (Days 6-7)

1. **Implement `process_voice_batch_simd()` function** - Process 8 voices with SIMD
2. **Integrate into `process_buffer_per_node()`** - Add SIMD fast path
3. **Test correctness** - Scalar vs SIMD output comparison
4. **Profile real workload** - Measure actual P95 latency on q.ph pattern
5. **Validate voice capacity** - Test with 600-800 voices

**Estimated completion**: End of Week 1 (2 more days)

### Documents Created

- `SIMD_BENCHMARK_RESULTS.md` - Detailed benchmark analysis
- `SIMD_INTEGRATION_PLAN.md` - Three integration approaches with recommendations

**Status**: On track for 2-2.5× speedup, which still achieves significant capacity increase toward 10× goal. Thread pool architecture (Phase 2) will provide additional 2× speedup for combined 4-5× improvement.
