# Buffer-Based Evaluation Refactor - Project Tracker

**Goal:** Transform Phonon from sample-by-sample recursive evaluation to buffer-based evaluation for 3-5x Phase 3 speedup.

**Expected Result:** Heavy patterns 10-22ms → 3-7ms (well under 11.61ms target)

**Timeline:** 4-6 weeks

**Status Legend:**
- 📚 **Study** - Research, design, understand requirements
- 🔨 **Implementation** - Code written, compiles
- 🧪 **Testing** - Tests written and running
- ✅ **Completion** - Tests passing, verified working

---

## Overview

### Current Architecture (Sample-by-Sample)
```rust
// Phase 3: For EACH sample, recursively evaluate entire graph
for i in 0..512 {
    buffer[i] = self.eval_node(&output_id);  // 512 recursive tree walks!
}
```

**Problems:**
- 4,096+ function calls per buffer
- Can't vectorize (SIMD)
- Poor cache locality
- Single-threaded recursion

### Target Architecture (Buffer-Based)
```rust
// Phase 3: Evaluate graph ONCE for entire buffer
self.eval_node_buffer(&output_id, &mut buffer[0..512]);  // 1 call, fills buffer!
```

**Benefits:**
- 512 → 1 function calls
- Compiler can SIMD vectorize
- Better cache locality
- Foundation for parallelization

---

## Design Principles

### 1. New Core API

**Before (sample-at-a-time):**
```rust
fn eval_node(&mut self, node_id: &NodeId) -> f32
```

**After (buffer-at-a-time):**
```rust
fn eval_node_buffer(&mut self, node_id: &NodeId, buffer: &mut [f32])
```

### 2. Signal Type Changes

**Before:**
```rust
enum Signal {
    Value(f32),           // Single sample
    Node(NodeId),         // Points to node
    Bus(String),          // Named bus
    // ...
}
```

**After:**
```rust
// Signals now represent buffer-generating expressions
// eval_signal_buffer() fills a buffer instead of returning single sample
```

### 3. State Management

**Stateful nodes (filters, delays, oscillators) need:**
- Internal buffer for intermediate results
- State updates happen AFTER full buffer processing
- Example: Filter processes 512 samples, THEN updates state

### 4. Backward Compatibility

**During transition:**
- Keep old `eval_node()` for testing
- New `eval_node_buffer()` runs alongside
- Compare outputs to verify correctness
- Remove old API once all nodes migrated

---

## Component Migration Checklist

### Core Infrastructure

#### 1. Buffer-Based Evaluation Framework
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Design `eval_node_buffer()` API signature ✅
- [x] 📚 **Study:** Design `eval_signal_buffer()` for Signal evaluation ✅
- [x] 📚 **Study:** Decide on buffer allocation strategy (stack? heap? reuse?) ✅
- [x] 🔨 **Implementation:** Add `eval_node_buffer()` to UnifiedSignalGraph ✅
- [x] 🔨 **Implementation:** Add `eval_signal_buffer()` for each Signal variant ✅
- [x] 🔨 **Implementation:** Add `eval_expression_buffer()` for arithmetic ops ✅
- [x] 🧪 **Testing:** Write test comparing buffer vs sample outputs ✅
- [x] 🧪 **Testing:** Test with constant signals (Value) ✅
- [x] 🧪 **Testing:** Test with bus references ✅
- [x] ✅ **Completion:** All tests pass, ready for node migration ✅

**Study Notes:** See `BUFFER_EVAL_STUDY.md` for complete analysis. Key decisions:
- API: `fn eval_node_buffer(&mut self, node_id: &NodeId, output: &mut [f32])`
- Allocation: Vec per call initially (simple), optimize later if needed
- Migration: Gradual, coexist with old API during transition

**Implementation Notes:** (Commit cc34939)
- Added ~200 lines of core buffer evaluation infrastructure
- Supports: Constant nodes, all Signal types, all arithmetic operations
- Includes fallback to sample-by-sample for not-yet-migrated nodes
- Compiles successfully, ready for testing

**Testing Notes:** (Commit eee8cef)
- Made eval_node_buffer(), eval_signal_buffer(), eval_expression_buffer() public
- 14 comprehensive tests covering:
  - Constant signals
  - All arithmetic operations (Add, Multiply, Subtract, Divide, Scale)
  - Nested expressions
  - Edge cases (empty buffer, large/small values, divide by zero)
  - Buffer size variations (1 to 2048 samples)
  - Performance sanity check (1000 iterations < 1 second)
- **All tests passing** (test result: ok. 14 passed; 0 failed)

**Design Notes:**
```rust
impl UnifiedSignalGraph {
    /// Evaluate a node for an entire buffer
    fn eval_node_buffer(&mut self, node_id: &NodeId, output: &mut [f32]) {
        match &self.nodes[node_id.0] {
            SignalNode::Sine { freq, .. } => self.eval_sine_buffer(node_id, freq, output),
            SignalNode::LowPass { .. } => self.eval_lpf_buffer(node_id, output),
            // ... other nodes
        }
    }

    /// Evaluate a signal for an entire buffer
    fn eval_signal_buffer(&mut self, signal: &Signal, output: &mut [f32]) {
        match signal {
            Signal::Value(v) => output.fill(*v),
            Signal::Node(id) => self.eval_node_buffer(id, output),
            Signal::Bus(name) => {
                if let Some(id) = self.buses.get(name) {
                    self.eval_node_buffer(id, output);
                }
            }
            // ... other variants
        }
    }
}
```

---

### Oscillators (Sources)

#### 2. Oscillator (All Waveforms)
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Review current oscillator implementation ✅
- [x] 📚 **Study:** Design phase accumulation for buffer ✅
- [x] 📚 **Study:** Handle frequency modulation (pattern-based freq) ✅
- [x] 🔨 **Implementation:** Write oscillator buffer evaluation ✅
- [x] 🔨 **Implementation:** Update phase tracking for buffer ✅
- [x] 🔨 **Implementation:** Handle freq signal evaluation (constant + dynamic) ✅
- [x] 🔨 **Implementation:** Support all waveforms (Sine, Saw, Square, Triangle) ✅
- [x] 🔨 **Implementation:** Preserve anti-click zero-crossing detection ✅
- [x] 🧪 **Testing:** Test sine wave amplitude and frequency accuracy ✅
- [x] 🧪 **Testing:** Test phase continuity across buffers ✅
- [x] 🧪 **Testing:** Test all waveform types ✅
- [x] 🧪 **Testing:** Test edge cases (zero freq, very high freq) ✅
- [x] ✅ **Completion:** All tests pass (10/10) ✅

**Implementation Notes:** (Commit 2534f83)
- Added buffer-based evaluation for SignalNode::Oscillator in eval_node_buffer()
- Optimizes constant frequency (evaluate once vs per-sample)
- Maintains phase continuity across multiple buffer calls
- Preserves zero-crossing detection for anti-click frequency changes
- Supports all waveforms: Sine, Saw, Square, Triangle

**Testing Notes:**
- 10 comprehensive tests, all passing
- Sine wave: amplitude, RMS, frequency accuracy via zero-crossing counting
- Phase continuity verified across consecutive buffers
- All waveform types tested
- Performance sanity check (< 1s for 1000 iterations)
- Edge cases: zero frequency, very high frequency

**Design Notes:**
```rust
fn eval_sine_buffer(&mut self, node_id: &NodeId, freq_signal: &Signal, output: &mut [f32]) {
    let buffer_size = output.len();

    // Allocate workspace for frequency values
    let mut freq_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(freq_signal, &mut freq_buffer);

    // Get current phase from node state
    let mut phase = self.get_sine_phase(node_id);

    // Generate samples
    for i in 0..buffer_size {
        output[i] = phase.sin();
        phase += 2.0 * PI * freq_buffer[i] / self.sample_rate;
        if phase >= 2.0 * PI {
            phase -= 2.0 * PI;
        }
    }

    // Update phase in node state
    self.update_sine_phase(node_id, phase);
}
```

**Testing Strategy:**
```rust
#[test]
fn test_sine_buffer_matches_sample() {
    let mut graph = create_test_graph();
    let sine_id = graph.add_sine(440.0);

    // Sample-by-sample (old way)
    let mut sample_output = vec![0.0; 512];
    for i in 0..512 {
        sample_output[i] = graph.eval_node(&sine_id);
    }

    // Buffer-based (new way)
    graph.reset();  // Reset phase
    let mut buffer_output = vec![0.0; 512];
    graph.eval_node_buffer(&sine_id, &mut buffer_output);

    // Compare
    for i in 0..512 {
        assert!((sample_output[i] - buffer_output[i]).abs() < 0.0001);
    }
}
```

---

#### 3. Sawtooth Oscillator
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review sawtooth implementation
- [ ] 📚 **Study:** Design PolyBLEP anti-aliasing for buffer
- [ ] 🔨 **Implementation:** Write `eval_saw_buffer()`
- [ ] 🔨 **Implementation:** Handle frequency modulation
- [ ] 🔨 **Implementation:** Apply PolyBLEP per sample
- [ ] 🧪 **Testing:** Test constant frequency
- [ ] 🧪 **Testing:** Test pattern frequency
- [ ] 🧪 **Testing:** Verify anti-aliasing works
- [ ] ✅ **Completion:** All tests pass

---

#### 4. Square Oscillator
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review square wave implementation
- [ ] 📚 **Study:** Design PolyBLEP for buffer
- [ ] 🔨 **Implementation:** Write `eval_square_buffer()`
- [ ] 🔨 **Implementation:** Handle frequency modulation
- [ ] 🔨 **Implementation:** Apply PolyBLEP per sample
- [ ] 🧪 **Testing:** Test constant frequency
- [ ] 🧪 **Testing:** Test pattern frequency
- [ ] 🧪 **Testing:** Verify anti-aliasing works
- [ ] ✅ **Completion:** All tests pass

---

#### 5. Triangle Oscillator
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review triangle wave implementation
- [ ] 🔨 **Implementation:** Write `eval_triangle_buffer()`
- [ ] 🔨 **Implementation:** Handle frequency modulation
- [ ] 🧪 **Testing:** Test constant frequency
- [ ] 🧪 **Testing:** Test pattern frequency
- [ ] ✅ **Completion:** All tests pass

---

### Filters (Stateful Processors)

#### 6. LowPass Filter (SVF)
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Review SVF implementation with coefficient caching ✅
- [x] 📚 **Study:** Design buffer processing with state updates ✅
- [x] 📚 **Study:** Handle modulated cutoff/Q (buffer-based) ✅
- [x] 🔨 **Implementation:** Write `eval_lpf_buffer()` ✅
- [x] 🔨 **Implementation:** Process input buffer → output buffer ✅
- [x] 🔨 **Implementation:** Update filter state after buffer ✅
- [x] 🔨 **Implementation:** Handle parameter modulation ✅
- [x] 🧪 **Testing:** Test with constant cutoff/Q ✅
- [x] 🧪 **Testing:** Test with modulated cutoff (LFO) ✅
- [x] 🧪 **Testing:** Verify state continuity across buffers ✅
- [x] ✅ **Completion:** All tests pass ✅

**Implementation Notes:** (Commit 1adaf64)
- Added buffer-based evaluation for SignalNode::LowPass in eval_node_buffer()
- Implements State Variable Filter (Chamberlin) algorithm
- Evaluates input, cutoff, and Q signals to buffers
- Processes entire buffer with SVF equations per-sample
- Updates filter state (low, band, high) after processing entire buffer
- Added stability clamp: `f = f.min(1.99)` to prevent numerical instability at high cutoffs
- Supports modulated cutoff and Q parameters
- Added helper method add_lowpass_node() for testing

**Testing Notes:**
- 13 comprehensive tests, all passing
- Tests: basic filtering, cutoff effect, resonance effect
- State continuity across buffers verified
- Multiple buffer evaluation
- Modulated cutoff (LFO-driven)
- Edge cases: very low cutoff, very high cutoff, extreme Q values
- Constant vs signal parameters
- Chained filters
- Performance sanity check (< 1s for 1000 iterations)

**Key Challenge:** SVF filter becomes numerically unstable near Nyquist frequency.
Added coefficient clamping (`f.min(1.99)`) to prevent instability. Tests use
cutoffs up to 5kHz to stay well below Nyquist (22.05kHz at 44.1kHz sample rate).

**Design Notes:**
```rust
fn eval_lpf_buffer(&mut self, node_id: &NodeId, output: &mut [f32]) {
    let buffer_size = output.len();

    // Evaluate input signal to buffer
    let mut input_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(&input_signal, &mut input_buffer);

    // Evaluate parameter signals
    let mut cutoff_buffer = vec![0.0; buffer_size];
    let mut q_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(&cutoff_signal, &mut cutoff_buffer);
    self.eval_signal_buffer(&q_signal, &mut q_buffer);

    // Get current filter state
    let mut state = self.get_lpf_state(node_id);

    // Process buffer
    for i in 0..buffer_size {
        let fc = cutoff_buffer[i].max(20.0).min(20000.0);
        let q = q_buffer[i].max(0.5).min(20.0);

        // Compute coefficients (with caching)
        let f = 2.0 * (PI * fc / self.sample_rate).sin();
        let damp = 1.0 / q;

        // SVF tick
        let high = input_buffer[i] - state.low - damp * state.band;
        state.band += f * high;
        state.low += f * state.band;

        output[i] = state.low;
    }

    // Update filter state
    self.update_lpf_state(node_id, state);
}
```

---

#### 7. HighPass Filter (SVF)
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Review HPF implementation ✅
- [x] 🔨 **Implementation:** Write `eval_hpf_buffer()` ✅
- [x] 🔨 **Implementation:** Process input buffer ✅
- [x] 🔨 **Implementation:** Handle parameter modulation ✅
- [x] 🧪 **Testing:** Test with constant parameters ✅
- [x] 🧪 **Testing:** Test with modulated cutoff ✅
- [x] ✅ **Completion:** All tests pass ✅

**Implementation Notes:** (Commit 5cfc122)
- Added buffer-based evaluation for SignalNode::HighPass in eval_node_buffer()
- Uses same SVF (Chamberlin) algorithm as LowPass, outputs 'high' instead of 'low'
- Identical implementation to LowPass except for output selection
- Includes stability clamp (f < 1.99) to prevent numerical instability
- Added helper method add_highpass_node() for testing

**Testing Notes:**
- 12 comprehensive tests, all passing
- Tests verify opposite behavior to LowPass (passes high, rejects low)
- State continuity, modulation, edge cases, chained filters, performance
- All verified working correctly

---

#### 8. BandPass Filter (SVF)
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Review BPF implementation ✅
- [x] 🔨 **Implementation:** Write `eval_bpf_buffer()` ✅
- [x] 🔨 **Implementation:** Process input buffer ✅
- [x] 🔨 **Implementation:** Handle parameter modulation ✅
- [x] 🧪 **Testing:** Test with constant parameters ✅
- [x] 🧪 **Testing:** Test with modulated center freq ✅
- [x] ✅ **Completion:** All tests pass ✅

**Implementation Notes:** (Commit 430aee2)
- Added buffer-based evaluation for SignalNode::BandPass
- Uses same SVF algorithm, outputs 'band' (vs 'low'/'high')
- Passes frequencies near center, rejects both low and high
- Higher Q = narrower passband + resonance boost
- Implemented via subagent (successful parallel approach test)

**Testing Notes:**
- 18 comprehensive tests, all passing
- Verified subagent can successfully implement buffer evaluation
- Pattern established for parallel deployment

---

### Effects (Complex Stateful Processors)

#### 9. Delay
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review delay line implementation
- [ ] 📚 **Study:** Design buffer read/write with circular buffer
- [ ] 🔨 **Implementation:** Write `eval_delay_buffer()`
- [ ] 🔨 **Implementation:** Handle modulated delay time
- [ ] 🔨 **Implementation:** Implement feedback
- [ ] 🧪 **Testing:** Test constant delay time
- [ ] 🧪 **Testing:** Test modulated delay time
- [ ] 🧪 **Testing:** Test feedback loop stability
- [ ] ✅ **Completion:** All tests pass

**Design Notes:**
```rust
fn eval_delay_buffer(&mut self, node_id: &NodeId, output: &mut [f32]) {
    let buffer_size = output.len();

    // Evaluate input
    let mut input_buffer = vec![0.0; buffer_size];
    self.eval_signal_buffer(&input_signal, &mut input_buffer);

    // Get delay line state
    let delay_line = self.get_delay_line(node_id);

    // Process buffer
    for i in 0..buffer_size {
        // Read from delay line
        output[i] = delay_line.read(delay_samples);

        // Write to delay line with feedback
        let feedback_sample = output[i] * feedback_amount;
        delay_line.write(input_buffer[i] + feedback_sample);
    }
}
```

---

#### 10. Reverb
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review reverb implementation (Freeverb/etc)
- [ ] 📚 **Study:** Design buffer processing for all-pass/comb filters
- [ ] 🔨 **Implementation:** Write `eval_reverb_buffer()`
- [ ] 🔨 **Implementation:** Process through comb filters
- [ ] 🔨 **Implementation:** Process through all-pass filters
- [ ] 🧪 **Testing:** Test with dry signal
- [ ] 🧪 **Testing:** Test room size parameter
- [ ] 🧪 **Testing:** Test damping parameter
- [ ] ✅ **Completion:** All tests pass

---

#### 11. Chorus
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review chorus implementation
- [ ] 🔨 **Implementation:** Write `eval_chorus_buffer()`
- [ ] 🔨 **Implementation:** Handle LFO modulation
- [ ] 🧪 **Testing:** Test rate/depth parameters
- [ ] ✅ **Completion:** All tests pass

---

#### 12. Distortion
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review distortion implementation
- [ ] 🔨 **Implementation:** Write `eval_distortion_buffer()`
- [ ] 🔨 **Implementation:** Apply waveshaping
- [ ] 🧪 **Testing:** Test drive parameter
- [ ] ✅ **Completion:** All tests pass

---

### Arithmetic Operations

#### 13. Add
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Design buffer addition ✅
- [x] 🔨 **Implementation:** Write `eval_add_buffer()` ✅
- [x] 🔨 **Implementation:** Add two signal buffers element-wise ✅
- [x] 🧪 **Testing:** Test a + b ✅
- [x] ✅ **Completion:** All tests pass ✅

**Implementation Notes:** (Commit 51433e6)
- Added buffer-based evaluation for SignalNode::Add in eval_node_buffer()
- Evaluates both input signals to buffers, then adds element-wise
- Added helper method add_add_node() for testing
- Stateless operation - straightforward implementation

**Testing Notes:**
- 21 comprehensive tests covering Add and Multiply (combined)
- Tests: constants, signals, oscillators, scaling, ring modulation
- Complex combinations (nested operations)
- Multiple buffer evaluation (state persistence)
- Edge cases (large/small values, zero)
- Various buffer sizes (1 to 2048 samples)
- Performance sanity check (< 1s for 1000 iterations)
- **All tests passing** (test result: ok. 21 passed; 0 failed)

**Design Notes:**
```rust
fn eval_add_buffer(&mut self, a: &Signal, b: &Signal, output: &mut [f32]) {
    let mut a_buffer = vec![0.0; output.len()];
    let mut b_buffer = vec![0.0; output.len()];

    self.eval_signal_buffer(a, &mut a_buffer);
    self.eval_signal_buffer(b, &mut b_buffer);

    for i in 0..output.len() {
        output[i] = a_buffer[i] + b_buffer[i];
    }
}
```

---

#### 14. Multiply
**Status:** ✅ Completion

**Tasks:**
- [x] 📚 **Study:** Design buffer multiplication ✅
- [x] 🔨 **Implementation:** Write `eval_multiply_buffer()` ✅
- [x] 🔨 **Implementation:** Multiply two signal buffers element-wise ✅
- [x] 🧪 **Testing:** Test a * b ✅
- [x] ✅ **Completion:** All tests pass ✅

**Implementation Notes:** (Commit 51433e6)
- Added buffer-based evaluation for SignalNode::Multiply in eval_node_buffer()
- Evaluates both input signals to buffers, then multiplies element-wise
- Added helper method add_multiply_node() for testing
- Stateless operation - straightforward implementation
- Supports ring modulation (oscillator * oscillator)

**Testing Notes:** (See Component 13 - tests cover both Add and Multiply)

---

#### 15. Mix
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Design buffer mixing (average N signals)
- [ ] 🔨 **Implementation:** Write `eval_mix_buffer()`
- [ ] 🔨 **Implementation:** Mix multiple signal buffers with normalization
- [ ] 🧪 **Testing:** Test mixing 2-8 signals
- [ ] ✅ **Completion:** All tests pass

---

### Phase 3 Integration

#### 16. Update process_buffer_hybrid()
**Status:** 📚 Study

**Tasks:**
- [ ] 📚 **Study:** Review current Phase 3 loop
- [ ] 📚 **Study:** Design new Phase 3 with buffer evaluation
- [ ] 🔨 **Implementation:** Replace sample loop with buffer eval
- [ ] 🔨 **Implementation:** Handle multiple outputs
- [ ] 🔨 **Implementation:** Update profiling code
- [ ] 🧪 **Testing:** Test with simple patterns
- [ ] 🧪 **Testing:** Test with complex patterns
- [ ] 🧪 **Testing:** Test with multiple outputs
- [ ] 🧪 **Testing:** Compare audio to old implementation
- [ ] ✅ **Completion:** All tests pass, audio identical

**Target Code:**
```rust
// PHASE 3: DSP evaluation (NEW - buffer-based)
let phase3_start = if enable_profiling { Some(std::time::Instant::now()) } else { None };

// Allocate output buffer (reuse across calls)
let mut dsp_buffer = vec![0.0; buffer_size];

// Evaluate main output
if let Some(output_id) = self.output {
    if !self.hushed_channels.contains(&0) {
        self.eval_node_buffer(&output_id, &mut dsp_buffer);

        // Copy to output buffer
        for i in 0..buffer_size {
            buffer[i] = dsp_buffer[i];
        }
    }
}

// Mix in numbered outputs
for (ch, node_id) in &output_channels {
    if !self.hushed_channels.contains(ch) {
        self.eval_node_buffer(node_id, &mut dsp_buffer);

        // Add to output buffer
        for i in 0..buffer_size {
            buffer[i] += dsp_buffer[i];
        }
    }
}

let phase3_time_us = phase3_start.map(|t| t.elapsed().as_micros()).unwrap_or(0);
```

---

## Testing Strategy

### Unit Tests (Per Node)

Each node needs:
1. **Correctness test:** Buffer output matches sample-by-sample output
2. **State continuity test:** State carries correctly across buffers
3. **Modulation test:** Pattern-based parameters work correctly
4. **Edge case test:** Empty buffers, single sample, large buffers

**Example:**
```rust
#[test]
fn test_lpf_buffer_correctness() {
    let mut graph = create_test_graph();
    let input_id = graph.add_sine(440.0);
    let lpf_id = graph.add_lpf(input_id, 1000.0, 0.8);

    // Sample-by-sample
    let mut sample_output = vec![0.0; 512];
    for i in 0..512 {
        sample_output[i] = graph.eval_node(&lpf_id);
    }

    // Buffer-based
    graph.reset();
    let mut buffer_output = vec![0.0; 512];
    graph.eval_node_buffer(&lpf_id, &mut buffer_output);

    // Should match within floating point tolerance
    for i in 0..512 {
        assert!((sample_output[i] - buffer_output[i]).abs() < 0.0001,
            "Sample {} differs: {} vs {}", i, sample_output[i], buffer_output[i]);
    }
}
```

### Integration Tests

Test full patterns:
```rust
#[test]
fn test_full_pattern_buffer_evaluation() {
    let pattern = "~osc: sine 440 # lpf 1000 0.8 # reverb 0.5\nout: ~osc";

    // Render with old method
    let old_audio = render_with_sample_eval(pattern, 8.0);

    // Render with new method
    let new_audio = render_with_buffer_eval(pattern, 8.0);

    // Audio should be identical
    assert_audio_match(&old_audio, &new_audio, 0.001);
}
```

### Performance Tests

```rust
#[test]
fn test_buffer_eval_performance() {
    let pattern = stress_heavy_pattern();

    let start = std::time::Instant::now();
    render_with_buffer_eval(pattern, 8.0);
    let duration = start.elapsed();

    // Should be under 11.61ms per buffer (44100 Hz, 512 samples)
    let avg_per_buffer = duration.as_secs_f64() / (8.0 * 44100.0 / 512.0);
    assert!(avg_per_buffer < 0.01161, "Too slow: {:.6}s per buffer", avg_per_buffer);
}
```

---

## Milestone Tracking

### Milestone 1: Foundation (Week 1-2)
**Goal:** Core buffer evaluation infrastructure working

- [ ] ✅ Buffer evaluation framework complete
- [ ] ✅ One oscillator (sine) working
- [ ] ✅ One arithmetic op (add) working
- [ ] ✅ Integration tests passing
- [ ] ✅ Performance baseline established

**Success Criteria:** Can render `sine 440 + sine 880` with buffer evaluation

---

### Milestone 2: Oscillators (Week 2-3)
**Goal:** All sound sources working

- [ ] ✅ All oscillators migrated (sine, saw, square, tri)
- [ ] ✅ All arithmetic ops migrated (add, multiply, mix)
- [ ] ✅ Pattern-based frequency modulation works
- [ ] ✅ Tests passing

**Success Criteria:** Can render complex FM patches

---

### Milestone 3: Filters (Week 3-4)
**Goal:** All filters working

- [ ] ✅ All filters migrated (lpf, hpf, bpf)
- [ ] ✅ Filter state continuity verified
- [ ] ✅ Modulated filters work
- [ ] ✅ Tests passing

**Success Criteria:** Can render filtered synthesis patterns

---

### Milestone 4: Effects (Week 4-5)
**Goal:** All effects working

- [ ] ✅ Delay working
- [ ] ✅ Reverb working
- [ ] ✅ Chorus, distortion, etc working
- [ ] ✅ Complex signal chains work
- [ ] ✅ Tests passing

**Success Criteria:** Can render production-quality effects chains

---

### Milestone 5: Integration (Week 5-6)
**Goal:** Full system working with buffer evaluation

- [ ] ✅ Phase 3 using buffer evaluation
- [ ] ✅ All existing tests passing
- [ ] ✅ Audio output matches old implementation
- [ ] ✅ Performance targets met (3-7ms for heavy patterns)
- [ ] ✅ Old sample-based code removed

**Success Criteria:** Production ready, all tests pass, 3-5x speedup achieved

---

## Performance Targets

### Before (Sample-by-Sample)
- Simple: 0.9ms
- Moderate: 3-5ms
- Heavy: 10-22ms ⚠️

### After (Buffer-Based)
- Simple: 0.3ms ✅
- Moderate: 1-2ms ✅
- Heavy: 3-7ms ✅

**Goal:** All patterns well under 11.61ms target with comfortable headroom.

---

## Notes & Learnings

### Design Decisions
*Document key decisions made during implementation*

### Gotchas
*Document tricky issues encountered*

### Performance Notes
*Document optimization opportunities discovered*

---

## Next Phase Preview

After buffer-based evaluation is complete:

**Phase 2: Feedback Loop Support** (2-3 weeks)
- Cycle detection in signal graph
- Stage-based evaluation
- Feedback delay buffers

**Phase 3: Parallel Phase 3** (2-3 weeks)
- Interior mutability refactor
- Parallel output evaluation
- 2-4x additional speedup

**Final Target:** 1-3ms for heavy patterns (10x faster than current!)
