# Message-Passing Architecture Design

## The Problem with Current Approach

**Shared Mutable State**:
- Oscillators use RefCell (single-threaded)
- Parallel processing causes "already borrowed" panics
- Arc<Mutex> would work but causes lock contention
- "Everything messing with everything else"

## The Solution: Message Passing

### Key Principles

1. **No Shared Mutable State**
   - Each thread gets its own graph copy
   - Independent oscillator phase, filter state, etc.
   - No locks, no contention

2. **Immutable Buffers**
   - Threads render to local Vec<f32>
   - Once created, buffers are read-only
   - Pass Arc<Vec<f32>> for zero-cost sharing

3. **Three-Phase Architecture**

```
Phase 1: Pattern Evaluation (Sequential, Main Thread)
├─ Evaluate patterns at sample-rate accuracy
├─ Determine which voices to trigger
└─ Create trigger messages for Phase 2

Phase 2: Voice Rendering (Parallel, Worker Threads)
├─ Each thread gets pre-cloned graph copy
├─ Render assigned voices independently
├─ No synchronization needed
└─ Return Arc<Vec<f32>> per voice

Phase 3: Mix Down (Sequential or Parallel)
├─ Combine voice buffers
├─ Apply master effects
└─ Write to output
```

## Implementation Strategy

### Step 1: Make Graph Cloneable

Current issue: Graph contains RefCell, Rc, etc.

**Solution**:
- Keep RefCell for single-threaded use (live mode)
- Add parallel rendering path that clones graph per-thread
- Each clone gets independent state

```rust
// Parallel rendering path
let graph_copies: Vec<UnifiedSignalGraph> = (0..num_threads)
    .map(|_| self.clone_for_parallel_rendering())
    .collect();
```

### Step 2: Per-Thread Buffer Rendering

```rust
// Each thread renders independently
let voice_buffers: Vec<Arc<Vec<f32>>> = voice_assignments
    .par_iter()
    .enumerate()
    .map(|(thread_id, voices)| {
        let graph = &mut graph_copies[thread_id];
        let mut buffer = vec![0.0; buffer_size];

        for voice in voices {
            voice.render_to_buffer(graph, &mut buffer);
        }

        Arc::new(buffer)  // Immutable from here on!
    })
    .collect();
```

### Step 3: Lock-Free Buffer Mixing

```rust
// No locks needed - buffers are immutable!
let mixed = voice_buffers.par_iter()
    .fold(|| vec![0.0; buffer_size], |mut acc, buf| {
        for (i, &sample) in buf.iter().enumerate() {
            acc[i] += sample;
        }
        acc
    })
    .reduce(|| vec![0.0; buffer_size], |mut a, b| {
        for (i, sample) in b.into_iter().enumerate() {
            a[i] += sample;
        }
        a
    });
```

## Expected Performance

**Scalability**:
- N threads → ~N x speedup (no lock contention)
- 16 cores → process 16x more voices in same time
- Real-time: If 1 thread does 10 voices in 11ms, 16 threads do 160 voices

**Memory Trade-off**:
- Each thread has graph copy (~few KB per thread)
- Much cheaper than lock contention overhead
- Cache-friendly (each thread owns its data)

## Implementation Plan

1. Add `clone_for_parallel_rendering()` to UnifiedSignalGraph
2. Modify voice rendering to use per-thread graphs
3. Implement parallel buffer mixing
4. Stress test with 100+ simultaneous voices
5. Measure real-time headroom

## Success Criteria

- ✅ 100+ voices render in < 11.6ms (real-time)
- ✅ Scales linearly with core count
- ✅ No RefCell panics
- ✅ No Arc<Mutex> needed
- ✅ Lock-free execution
