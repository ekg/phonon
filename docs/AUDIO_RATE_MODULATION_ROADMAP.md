# Audio-Rate Modulation Everywhere: Implementation Roadmap

**Goal**: Achieve 100% compliance with "EVERY PARAMETER MUST BE A PATTERN" and optimize the engine for maximum performance.

---

## Status Overview

| Category | Status | Progress |
|----------|--------|----------|
| 1. Parallelization | 🔲 Not Started | 0% |
| 2. Block-based Effects | 🔲 Not Started | 0% |
| 3. Remaining Parameter Gaps | 🔲 Not Started | 0% |
| 4. Multi-output System | 🔲 Not Started | 0% |

---

## 1. Parallelization (rayon)

**Why**: The DAG processing identifies independent signal branches. These can be processed in parallel for significant speedup on multi-core systems.

### Tasks

- [ ] Add `rayon` dependency to Cargo.toml
- [ ] Identify parallelizable sections in `process_buffer_dag()`
- [ ] Group independent nodes into parallel batches
- [ ] Implement parallel iteration with `rayon::par_iter()`
- [ ] Add benchmarks to measure improvement
- [ ] Test for race conditions / thread safety

### Key Files
- `src/unified_graph.rs` - `process_buffer_dag()` function
- `Cargo.toml` - add rayon dependency

### Expected Benefit
- 2-4x speedup on 4+ core systems for complex graphs
- Better utilization of modern CPUs

---

## 2. Block-based Effects Processing

**Why**: Reverb and delay currently process sample-by-sample within the DAG loop. Block-based processing improves cache locality and enables SIMD optimization.

### Tasks

- [ ] Audit which nodes still use sample-by-sample processing
- [ ] Convert Reverb to block-based processing
- [ ] Convert Delay variants (Delay, TapeDelay, MultiTapDelay, PingPongDelay) to block-based
- [ ] Convert Comb filter to block-based
- [ ] Convert Allpass to block-based
- [ ] Update the DAG evaluation to use block-based methods
- [ ] Benchmark before/after

### Key Files
- `src/unified_graph.rs` - effect node implementations
- `src/dsp/` - if we factor out DSP code

### Expected Benefit
- Better cache utilization
- Foundation for SIMD optimization
- Reduced function call overhead

---

## 3. Remaining Parameter Gaps (~5%)

**Why**: A few utility nodes still use bare `f32` types instead of `Signal`. Full compliance means ANY parameter can be pattern-controlled.

### Tasks

- [ ] Audit all SignalNode variants for bare f32/f64 parameters
- [ ] Convert identified parameters to Signal
- [ ] Update evaluation code
- [ ] Update compiler

### Nodes to Audit
- [ ] Arpeggiator parameters (rate, division)
- [ ] Scale lock parameters
- [ ] Any remaining effect parameters
- [ ] Utility nodes (Wrap, Clamp, etc.)

### Key Files
- `src/unified_graph.rs` - SignalNode enum definition
- `src/compositional_compiler.rs` - node construction

---

## 4. Multi-output System

**Why**: Enable routing audio to multiple outputs for DAW integration, multi-track rendering, and live performance controls.

### Features

- [ ] `out1:`, `out2:`, etc. - Named output buses
- [ ] `hush` - Silence all outputs gracefully (with fadeout)
- [ ] `panic` - Immediate silence (emergency stop)
- [ ] `solo` / `mute` - Per-bus control

### Tasks

- [ ] Design multi-output architecture
- [ ] Add output bus registry
- [ ] Implement `out1:`, `out2:` syntax
- [ ] Implement `hush` command with fadeout
- [ ] Implement `panic` command (immediate zero)
- [ ] Add solo/mute support
- [ ] Update audio backend to support multiple outputs

### Key Files
- `src/unified_graph.rs` - output handling
- `src/main.rs` - command parsing
- `src/bin/phonon-audio.rs` - audio output

### Expected Benefit
- Multi-track rendering for DAW export
- Live performance safety controls
- Better integration with external audio systems

---

## Implementation Order

**Recommended sequence** (dependencies and quick wins first):

1. **Remaining Parameter Gaps** - Quick win, completes the vision
2. **Parallelization** - High impact, relatively isolated change
3. **Block-based Effects** - Performance improvement, more invasive
4. **Multi-output System** - New feature, most architectural work

---

## Progress Log

### Session: 2024-12-29

- Created this roadmap document
- Starting with: _____________

