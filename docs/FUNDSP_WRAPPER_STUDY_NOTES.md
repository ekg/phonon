# fundsp Wrapper Study Notes

**Date**: 2025-10-29
**Purpose**: Understand fundsp API before implementing wrapper
**Status**: STUDYING

---

## fundsp Core Concepts

### AudioUnit Trait

fundsp provides the `AudioUnit` trait which all audio processing units implement:

```rust
pub trait AudioUnit: Send + Sync {
    fn inputs(&self) -> usize;
    fn outputs(&self) -> usize;
    fn reset(&mut self);
    fn set_sample_rate(&mut self, sample_rate: f64);
    fn tick(&mut self, input: &[f32], output: &mut [f32]);
    // ... other methods
}
```

**Key observations**:
- `tick()` processes ONE sample at a time
- Takes input buffer, writes to output buffer
- Stateful (has `reset()` and internal state)
- Sample rate configurable

### Audio Graph Notation

fundsp uses operators for composing audio units:
- `>>` - Pipe (serial processing)
- `|` - Stack (parallel, concatenate I/O)
- `&` - Bus (parallel, shared input)
- `^` - Branch (parallel, split output)
- `+`, `-`, `*` - Arithmetic on signals

**Example from fundsp docs**:
```rust
use fundsp::prelude::*;

// Simple filter chain
let unit = lowpass_hz(1000.0, 1.0) >> highpass_hz(100.0, 1.0);

// Process audio
let mut buffer = vec![0.0; 1024];
unit.tick(&[input_sample], &mut buffer);
```

---

## How Phonon Will Wrap fundsp

### Challenge: Pattern Modulation

**Phonon's killer feature**: Patterns modulate parameters at audio rate (44.1 kHz)

```phonon
~lfo: sine 0.1
~cutoff: ~lfo * 1000 + 500
out: saw 220 # lpf ~cutoff 0.8
--                  ^^^^^^^ This value changes every sample!
```

**fundsp limitation**: Most fundsp units have **static parameters** set at construction time.

**Solution approaches**:

#### Option 1: Re-create fundsp unit when parameters change (REJECTED - too slow)
```rust
// BAD: Creates new unit every sample
let cutoff_val = eval_signal(cutoff);
let unit = lowpass_hz(cutoff_val, q_val);  // SLOW!
unit.tick(&[input], &mut output);
```

#### Option 2: Use fundsp's parameter inputs (BEST)
Some fundsp units accept **audio-rate parameter inputs**:
```rust
// GOOD: Parameters are input channels
let unit = lowpass();  // Generic, takes freq as input
unit.tick(&[audio_input, freq_input, q_input], &mut output);
```

#### Option 3: Hybrid - Static structure, modulated parameters
```rust
// COMPROMISE: Update internal state between ticks
let mut unit = lowpass_hz(1000.0, 1.0);
// Each sample:
let freq = eval_signal(freq_signal);
unit.set_parameter(0, freq);  // If supported
unit.tick(&[input], &mut output);
```

### Decision: Use fundsp's stateful units + parameter updates

**Rationale**:
- fundsp units are designed for audio-rate processing
- Many fundsp units have `set()` methods for parameters
- We can evaluate Phonon signals per-sample and feed to fundsp
- Avoids re-allocating units every sample

---

## Wrapper Architecture

### SignalNode Variant

```rust
// In unified_graph.rs
pub enum SignalNode {
    // ... existing variants ...

    FundspUnit {
        unit_type: FundspUnitType,
        input: Signal,              // Audio input from Phonon
        params: Vec<Signal>,        // Parameters (pattern-modulatable!)
        state: Box<dyn AudioUnit>,  // fundsp unit instance
    },
}

pub enum FundspUnitType {
    OrganHz,
    MoogHz,
    Reverb2Stereo,
    Phaser,
    // ... more as we add them
}
```

### Evaluation Strategy

```rust
impl UnifiedSignalGraph {
    fn eval_node(&mut self, node_id: NodeId, ...) -> f32 {
        match &mut self.nodes[node_id.0] {
            SignalNode::FundspUnit { unit_type, input, params, state } => {
                // 1. Evaluate Phonon input signal
                let input_sample = self.eval_signal(*input, ...);

                // 2. Evaluate Phonon parameter signals (PATTERN MODULATION!)
                let mut param_values = Vec::new();
                for param_signal in params {
                    param_values.push(self.eval_signal(*param_signal, ...));
                }

                // 3. Prepare fundsp inputs
                let mut inputs = vec![input_sample];
                inputs.extend(param_values);

                // 4. Call fundsp tick()
                let mut outputs = vec![0.0];
                state.tick(&inputs, &mut outputs);

                // 5. Return fundsp output
                outputs[0]
            }
        }
    }
}
```

---

## Parameter Mapping Strategy

### Example: moog_hz(cutoff, resonance)

**fundsp signature**:
```rust
pub fn moog_hz(cutoff: f32, resonance: f32) -> impl AudioUnit
```

**Problem**: Parameters are **constructor arguments**, not audio-rate inputs!

**Solution**: fundsp's Moog filter might have internal state we can update, OR we use a different approach.

**Research needed**: Check if fundsp units support parameter modulation

### Alternative: Use fundsp's `var` and `shared` for parameters

fundsp has mechanisms for shared parameters:
```rust
use fundsp::prelude::*;

let cutoff = shared(1000.0);
let resonance = shared(0.7);

let unit = cutoff.clone() | resonance.clone() >> moog();

// Later, modulate:
cutoff.set_value(new_cutoff);
```

**This might work for Phonon!**

---

## Test Plan for Wrapper

### Level 1: Basic fundsp Unit Works
```rust
#[test]
fn test_fundsp_wrapper_basic() {
    // Create simple fundsp unit (sine wave)
    let unit = sine_hz(440.0);

    // Wrap in Phonon
    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SineHz,
        input: Signal::Constant(0.0),
        params: vec![Signal::Constant(440.0)],
        state: Box::new(unit),
    };

    // Evaluate 1 second
    let mut graph = UnifiedSignalGraph::new(44100.0);
    let node_id = graph.add_node(node);

    let samples: Vec<f32> = (0..44100)
        .map(|i| graph.eval_node(node_id, i as f32 / 44100.0))
        .collect();

    // Should have sine wave
    let rms = calculate_rms(&samples);
    assert!(rms > 0.6 && rms < 0.8);  // Sine RMS ≈ 0.707
}
```

### Level 2: Pattern Modulation Works
```rust
#[test]
fn test_fundsp_pattern_modulation() {
    // Phonon LFO pattern
    let lfo_pattern = parse_mini_notation("0.0 0.5 1.0 0.5");

    // Use LFO to modulate fundsp parameter
    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::MoogHz,
        input: Signal::Node(saw_input),
        params: vec![
            Signal::Pattern(lfo_pattern),  // Modulated cutoff!
            Signal::Constant(0.7),         // Static resonance
        ],
        state: Box::new(moog_hz(1000.0, 0.7)),
    };

    // Render and verify modulation happens
    let audio = render_graph(node, 1.0);

    // Audio should change over time (not static)
    let first_half = &audio[0..22050];
    let second_half = &audio[22050..44100];
    let diff = calculate_difference(first_half, second_half);
    assert!(diff > 0.1, "Modulation should cause audible change");
}
```

### Level 3: Comparative Test vs Custom Implementation
```rust
#[test]
fn test_fundsp_vs_custom_moog() {
    // Our custom moogLadder
    let code_custom = "out: saw 220 # moogLadder 1000 0.7";
    let audio_custom = render_dsl(code_custom, 1.0);

    // fundsp moog_hz
    let code_fundsp = "out: saw 220 # fundspMoog 1000 0.7";
    let audio_fundsp = render_dsl(code_fundsp, 1.0);

    // Should be very similar
    let diff = calculate_difference(&audio_custom, &audio_fundsp);
    assert!(diff < 0.05, "Custom and fundsp differ by {:.1}%", diff * 100.0);
}
```

---

## Implementation Challenges

### Challenge 1: fundsp Unit Lifecycle

**Question**: When do we create/destroy fundsp units?

**Options**:
1. Create once at graph construction, reuse forever
2. Create per-render pass (expensive!)
3. Pool units and reuse

**Decision**: Create once, store in SignalNode, reuse. fundsp units are designed for this.

### Challenge 2: Sample Rate Mismatch

**Phonon**: Fixed 44.1 kHz (currently)
**fundsp**: Supports arbitrary sample rates

**Solution**: Initialize fundsp units with Phonon's sample rate:
```rust
let mut unit = moog_hz(1000.0, 0.7);
unit.set_sample_rate(44100.0);
```

### Challenge 3: Stereo vs Mono

**Phonon**: Currently mono
**fundsp**: Many units are stereo (2 outputs)

**Solution for now**: Take left channel only, OR average L+R:
```rust
let mut outputs = vec![0.0, 0.0];  // Stereo
state.tick(&inputs, &mut outputs);
let output = (outputs[0] + outputs[1]) / 2.0;  // Mono
```

**Future**: Support multi-channel in Phonon

### Challenge 4: fundsp State is Not Clone

**Problem**: `SignalNode` needs to be `Clone`, but `Box<dyn AudioUnit>` is NOT `Clone`

**Solutions**:
1. Don't clone fundsp units (use `Rc<RefCell<>>`)
2. Implement custom Clone that creates new units
3. Store unit factory function instead of unit

**Decision**: Use `Rc<RefCell<>>` for shared mutable access:
```rust
pub struct FundspState {
    unit: Rc<RefCell<Box<dyn AudioUnit>>>,
}

impl Clone for FundspState {
    fn clone(&self) -> Self {
        Self { unit: self.unit.clone() }  // Shallow clone, shared unit
    }
}
```

---

## fundsp Units We'll Wrap First (5 test units)

### 1. organ_hz (Simple - good first test)
```rust
pub fn organ_hz(frequency: f32) -> impl AudioUnit
```
- 1 input (frequency)
- 1 output (audio)
- Simple additive synthesis

### 2. moog_hz (Filter - compare to our custom)
```rust
pub fn moog_hz(cutoff: f32, resonance: f32) -> impl AudioUnit
```
- 3 inputs (audio, cutoff, resonance)
- 1 output (filtered audio)
- Can compare to our moogLadder

### 3. reverb2_stereo (Effect - stereo handling)
```rust
pub fn reverb2_stereo(room_size: f32, time: f32) -> impl AudioUnit
```
- 2 inputs (L, R) OR 1 input (mono → stereo)
- 2 outputs (L, R)
- Tests stereo handling

### 4. phaser (Effect - modulation)
```rust
pub fn phaser(speed: f32, depth: f32, feedback: f32) -> impl AudioUnit
```
- 4 inputs (audio, speed, depth, feedback)
- 1 output (processed audio)
- Tests parameter modulation

### 5. dlowpass_hz (Nonlinear - unique to fundsp)
```rust
pub fn dlowpass_hz(cutoff: f32, q: f32) -> impl AudioUnit
```
- 3 inputs (audio, cutoff, q)
- 1 output (filtered audio)
- Jatin Chowdhury's nonlinear filter (NO equivalent in Phonon!)

---

## Expected Outcomes

After implementing fundsp wrapper:
- ✅ Can use fundsp units from Phonon DSL
- ✅ Pattern modulation works at audio rate
- ✅ Wrapping new fundsp UGens takes 1-2 hours each
- ✅ 37 UGens available quickly
- ✅ Can compare custom implementations to fundsp

---

## Next Steps (Implementation Phase)

1. Add `SignalNode::FundspUnit` variant
2. Implement evaluation logic
3. Add compiler function for first unit (organ)
4. Test basic functionality
5. Test pattern modulation
6. Test comparative (custom vs fundsp)
7. Wrap remaining 4 test units

**Estimated time**: 4-6 hours for complete infrastructure + 5 test units

---

**Study Phase Complete**: ✅
**Ready for Implementation**: YES
**Concerns**: Stereo handling, parameter modulation strategy
**Confidence**: HIGH (fundsp is well-designed for this use case)
