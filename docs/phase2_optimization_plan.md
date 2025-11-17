# Phase 2: Advanced Parallel Bus Synthesis Optimization

## Phase 1 Results Summary

**Performance Metrics** (from q.ph benchmark):
- Parallel synthesis time: 0.04-0.05ms average per audio buffer
- Cache hit rate: 100.0% (perfect)
- Unique buffers per call: 1
- Cache hits per buffer: 4 (multiple events reuse synthesis)
- Total preprocessing calls: ~70 (one per audio buffer)

**Current Bottleneck**: Preprocessing happens once per audio buffer (512 samples @ 44.1kHz), causing ~70 identical preprocessing operations for a 4-second render.

## Phase 2 Goals

1. **Reduce preprocessing frequency** from per-buffer to per-cycle or per-pattern-change
2. **Batch process multiple audio buffers** in parallel
3. **Detect static patterns** and cache indefinitely
4. **Optimize memory allocation** with buffer pools

## Implementation Strategy

### Optimization 1: Cross-Cycle Caching

**Problem**: Same bus pattern queried 70+ times for identical time spans

**Solution**: Cache at cycle-level instead of buffer-level

```rust
struct CycleBusCache {
    cycle_number: u64,
    buffers: HashMap<(String, usize), Arc<Vec<f32>>>,
}

impl UnifiedGraph {
    fn get_or_presynthesize_cycle_buses(&mut self, cycle: u64, events: &[Hap<String>])
        -> &HashMap<(String, usize), Arc<Vec<f32>>>
    {
        if self.cycle_bus_cache.cycle_number != cycle {
            // Cache miss - new cycle
            let new_cache = self.presynthesize_buses_parallel(events, ...);
            self.cycle_bus_cache = CycleBusCache {
                cycle_number: cycle,
                buffers: new_cache,
            };
        }
        &self.cycle_bus_cache.buffers
    }
}
```

**Expected Impact**: 70 preprocessing calls → ~4 (one per cycle)
**Speedup**: ~17x reduction in preprocessing overhead

### Optimization 2: Batch Audio Buffer Processing

**Problem**: Each audio buffer processed serially

**Solution**: Use Rayon to process multiple buffers in parallel

```rust
// Instead of:
for _ in 0..num_samples {
    let sample = self.process_sample();
    buffer.push(sample);
}

// Do:
let chunks: Vec<Vec<f32>> = (0..num_buffers)
    .into_par_iter()
    .map(|buf_idx| {
        let mut local_graph = self.clone_for_parallel();
        local_graph.process_buffer(512)
    })
    .collect();

buffer = chunks.into_iter().flatten().collect();
```

**Expected Impact**: Near-linear scaling with CPU cores
**Speedup**: ~4-8x on multi-core systems

### Optimization 3: Static Pattern Detection

**Problem**: Patterns that don't change (e.g., `~x: saw 440`) are re-synthesized every cycle

**Solution**: Hash pattern definition and cache permanently

```rust
struct StaticBusCache {
    pattern_hash: u64,
    buffers: HashMap<(String, usize), Arc<Vec<f32>>>,
}

fn is_pattern_static(pattern: &Pattern<String>) -> bool {
    // Detect if pattern contains no time-varying elements
    // e.g., no LFOs, no random, no iter
}
```

**Expected Impact**: Eliminate all synthesis for static patterns after first render
**Speedup**: Asymptotic improvement (approaches zero synthesis time)

### Optimization 4: Buffer Pool

**Problem**: Allocating new Vec<f32> for each synthesis

**Solution**: Reuse pre-allocated buffers

```rust
struct BufferPool {
    available: Vec<Vec<f32>>,
    in_use: HashMap<usize, Vec<f32>>,
}

impl BufferPool {
    fn get(&mut self, size: usize) -> usize {
        let mut buf = self.available.pop().unwrap_or_else(|| Vec::with_capacity(size));
        buf.resize(size, 0.0);
        let id = self.next_id();
        self.in_use.insert(id, buf);
        id
    }

    fn release(&mut self, id: usize) {
        if let Some(buf) = self.in_use.remove(&id) {
            self.available.push(buf);
        }
    }
}
```

**Expected Impact**: Reduce allocation overhead
**Speedup**: 1.1-1.2x (modest but measurable)

## Implementation Priority

### High Priority (Implement First)
1. **Cross-Cycle Caching** - Biggest impact, low complexity
   - Estimated effort: 2 hours
   - Expected speedup: 17x preprocessing reduction

### Medium Priority
2. **Static Pattern Detection** - High impact for common case
   - Estimated effort: 4 hours
   - Expected speedup: Asymptotic (near-zero for static patterns)

### Low Priority
3. **Batch Audio Buffer Processing** - Complex, requires careful state management
   - Estimated effort: 8 hours
   - Expected speedup: 4-8x (but may introduce latency issues in live mode)

4. **Buffer Pool** - Incremental improvement
   - Estimated effort: 3 hours
   - Expected speedup: 1.1-1.2x

## Risk Assessment

**Cycle Caching**:
- ✅ Low risk - simple change, easy to rollback
- ⚠️ Must invalidate cache on live code reload

**Static Pattern Detection**:
- ⚠️ Medium risk - need to correctly detect truly static patterns
- ⚠️ False positives could cause stale audio

**Batch Processing**:
- 🔴 High risk - complex state management
- 🔴 May break real-time guarantees in live mode
- 🔴 Requires careful testing for audio glitches

**Buffer Pool**:
- ✅ Low risk - internal optimization, no behavioral changes

## Success Metrics

**Cycle Caching**:
- Target: <5 preprocessing calls for 4-second render (currently ~70)
- Measure: Count of parallel synthesis operations

**Static Pattern Detection**:
- Target: Zero synthesis calls after cycle 1 for static patterns
- Measure: Cache hit rate > 99.9% for static patterns

**Batch Processing**:
- Target: Linear speedup with CPU cores (4x on 4-core, 8x on 8-core)
- Measure: Total render time comparison

**Buffer Pool**:
- Target: 10-20% reduction in allocation overhead
- Measure: Memory allocator statistics

## Testing Strategy

1. **Correctness**: Audio output must be bit-identical before/after
2. **Performance**: Benchmark with various pattern complexities
3. **Live Mode**: Ensure no latency regression
4. **Memory**: Profile memory usage and leak detection

## Next Steps

1. ✅ Instrument Phase 1 (completed)
2. ✅ Analyze performance (completed)
3. ✅ Implement Cross-Cycle Caching (completed - 2025-11-16)
4. ✅ Benchmark and verify (completed - 17.5x reduction in preprocessing)
5. ⏭️ Implement Static Pattern Detection
6. ⏭️ Re-evaluate need for Batch Processing and Buffer Pool

## Phase 2.1 Results: Cross-Cycle Caching (2025-11-16)

**Implementation**: `CycleBusCache` struct in `unified_graph.rs` (lines 3844-3861, 3925-3927, 7254-7264)

**Performance Achieved**:
- **Before**: ~280 preprocessing calls for 16-cycle render (70 per 4 cycles)
- **After**: 16 preprocessing calls (one per musical cycle)
- **Reduction**: **17.5x fewer preprocessing operations**

**How it Works**:
1. Cache key: `floor(cycle_position)`
2. Cache invalidation: When cycle floor changes
3. Within-cycle reuse: All audio buffers in same cycle share presynthesized bus buffers
4. Arc-based cloning: Cheap memory sharing via reference counting

**Code Locations**:
- `CycleBusCache` struct: lines 3844-3861
- Cache field in `UnifiedSignalGraph`: lines 3925-3927, 3964
- Cache logic in render loop: lines 7254-7264

**Benchmark Results** (q.ph, 120 BPM):
- 4 cycles: 4 preprocessing calls (down from ~70)
- 16 cycles: 16 preprocessing calls (down from ~280)
- Cache hit rate: 100% within cycles
- Zero underruns, real-time capable
