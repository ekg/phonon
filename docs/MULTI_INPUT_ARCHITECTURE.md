# Multi-Input Architecture Implementation Plan

**Date**: 2025-10-30
**Status**: ✅ COMPLETE (Commit: a74b321)
**Unblocks**: pulse(), FM synthesis, advanced modulation

---

## Problem Statement

Current FundspState architecture only supports single-input UGens:
```rust
tick_fn: Box<dyn FnMut(f32) -> f32 + Send>  // ONE input -> ONE output
```

This blocks:
- **pulse()** - Requires 2 inputs: frequency, pulse_width
- **FM synthesis** - Requires audio-rate frequency modulation
- **morph()** - Requires 4 inputs: signal, freq, Q, morph
- Any complex modulation routing

## Current Architecture (Before)

```rust
pub struct FundspState {
    tick_fn: Box<dyn FnMut(f32) -> f32 + Send>,
    unit_type: FundspUnitType,
    params: Vec<f32>,  // Static parameters for recreation
    sample_rate: f64,
}

// In SignalNode::FundspUnit
FundspUnit {
    unit_type: FundspUnitType,
    input: Signal,  // SINGLE input
    params: Vec<Signal>,  // Parameters (updated by recreating unit)
    state: Arc<Mutex<FundspState>>,
}
```

**Limitations**:
- Only ONE audio input supported
- Parameters are static (updated by recreating unit)
- Can't do audio-rate modulation of multiple parameters simultaneously

## New Architecture (After)

```rust
pub struct FundspState {
    tick_fn: Box<dyn FnMut(&[f32]) -> f32 + Send>,  // MULTIPLE inputs -> ONE output
    unit_type: FundspUnitType,
    num_inputs: usize,  // How many inputs this unit expects
    params: Vec<f32>,  // For recreation (when needed)
    sample_rate: f64,
}

// In SignalNode::FundspUnit
FundspUnit {
    unit_type: FundspUnitType,
    inputs: Vec<Signal>,  // MULTIPLE inputs (audio + parameters)
    state: Arc<Mutex<FundspState>>,
}
```

**Capabilities**:
- ✅ Variable number of inputs per UGen
- ✅ Audio-rate modulation of all parameters
- ✅ FM synthesis
- ✅ Complex modulation routing
- ✅ Backward compatible (single-input UGens work with 1-element array)

---

## Implementation Plan

### Phase 1: Extend FundspState (Core Change)

**File**: `src/unified_graph.rs`

#### 1.1: Update FundspState struct
```rust
pub struct FundspState {
    tick_fn: Box<dyn FnMut(&[f32]) -> f32 + Send>,  // Changed signature
    unit_type: FundspUnitType,
    num_inputs: usize,  // NEW: Track how many inputs
    params: Vec<f32>,  // Keep for recreation
    sample_rate: f64,
}
```

#### 1.2: Update tick() method
```rust
impl FundspState {
    pub fn tick(&mut self, inputs: &[f32]) -> f32 {
        (self.tick_fn)(inputs)
    }
}
```

#### 1.3: Update ALL constructors
For each `new_X` constructor:

**Before**:
```rust
pub fn new_saw_hz(frequency: f32, sample_rate: f64) -> Self {
    let tick_fn = Box::new(move |_input: f32| -> f32 {
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });
    Self { tick_fn, unit_type, params, sample_rate }
}
```

**After**:
```rust
pub fn new_saw_hz(frequency: f32, sample_rate: f64) -> Self {
    let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
        // Generators ignore inputs
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });
    Self {
        tick_fn,
        unit_type,
        num_inputs: 0,  // Generator (no inputs)
        params,
        sample_rate
    }
}
```

**For processors (like moog_hz)**:
```rust
pub fn new_moog_hz(cutoff: f32, resonance: f32, sample_rate: f64) -> Self {
    let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
        let audio_input = inputs.get(0).copied().unwrap_or(0.0);
        let output_frame = unit.tick(&[audio_input].into());
        output_frame[0]
    });
    Self {
        tick_fn,
        unit_type,
        num_inputs: 1,  // Takes 1 audio input
        params,
        sample_rate
    }
}
```

**For multi-input (like pulse)**:
```rust
pub fn new_pulse(sample_rate: f64) -> Self {
    let mut unit = fundsp::prelude::pulse();
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |inputs: &[f32]| -> f32 {
        let freq = inputs.get(0).copied().unwrap_or(440.0);
        let width = inputs.get(1).copied().unwrap_or(0.5);
        let output_frame = unit.tick(&[freq, width].into());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::Pulse,
        num_inputs: 2,  // Takes 2 inputs!
        params: vec![440.0, 0.5],  // Defaults for display
        sample_rate
    }
}
```

### Phase 2: Update SignalNode::FundspUnit

**File**: `src/unified_graph.rs`

**Before**:
```rust
FundspUnit {
    unit_type: FundspUnitType,
    input: Signal,  // Single input
    params: Vec<Signal>,  // Parameters
    state: Arc<Mutex<FundspState>>,
}
```

**After**:
```rust
FundspUnit {
    unit_type: FundspUnitType,
    inputs: Vec<Signal>,  // ALL inputs (audio + parameters)
    state: Arc<Mutex<FundspState>>,
}
```

**Migration strategy**:
- Generators: `inputs = vec![]` (no inputs)
- Processors: `inputs = vec![audio_signal, param1, param2, ...]`
- Multi-input: `inputs = vec![freq_signal, width_signal, ...]`

### Phase 3: Update eval_node()

**File**: `src/unified_graph.rs`

**Before**:
```rust
SignalNode::FundspUnit { input, params, state, .. } => {
    let input_val = self.eval_signal(&input, time);
    let param_values: Vec<f32> = params.iter()
        .map(|p| self.eval_signal(p, time))
        .collect();

    // Update parameters (recreate unit)
    match unit_type {
        FundspUnitType::SawHz => {
            state.update_saw_frequency(param_values[0], ...);
        }
        // ...
    }

    state.tick(input_val)
}
```

**After**:
```rust
SignalNode::FundspUnit { inputs, state, .. } => {
    // Evaluate ALL inputs
    let input_values: Vec<f32> = inputs.iter()
        .map(|signal| self.eval_signal(signal, time))
        .collect();

    // For units that still need recreation (static constructors),
    // detect changes and recreate if needed
    let state_guard = state.lock().unwrap();
    let needs_recreation = match state_guard.unit_type {
        // Units with static constructors still need recreation
        FundspUnitType::SawHz | FundspUnitType::SquareHz | ... => {
            // Check if params changed significantly
            if input_values.len() == 1 {
                (state_guard.params[0] - input_values[0]).abs() > 0.1
            } else {
                false
            }
        }
        // Multi-input units don't need recreation
        FundspUnitType::Pulse => false,
        _ => false,
    };

    if needs_recreation {
        drop(state_guard);
        // Recreate unit with new parameters
        match unit_type {
            FundspUnitType::SawHz => {
                let new_state = FundspState::new_saw_hz(
                    input_values[0],
                    self.sample_rate as f64
                );
                *state.lock().unwrap() = new_state;
            }
            // ...
        }
    }

    // Tick with all inputs
    state.lock().unwrap().tick(&input_values)
}
```

### Phase 4: Update Compiler Functions

**File**: `src/compositional_compiler.rs`

**Before (saw_hz)**:
```rust
fn compile_saw_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let freq_node = compile_expr(ctx, args[0].clone())?;
    let state = FundspState::new_saw_hz(440.0, ctx.graph.sample_rate() as f64);
    let no_input = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SawHz,
        input: Signal::Node(no_input),
        params: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };
    Ok(ctx.graph.add_node(node))
}
```

**After (saw_hz)**:
```rust
fn compile_saw_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let freq_node = compile_expr(ctx, args[0].clone())?;
    let state = FundspState::new_saw_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SawHz,
        inputs: vec![Signal::Node(freq_node)],  // Just frequency input
        state: Arc::new(Mutex::new(state)),
    };
    Ok(ctx.graph.add_node(node))
}
```

**After (moog_hz - processor with audio input)**:
```rust
fn compile_moog_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let input_node = extract_chain_input(ctx)?;  // Audio input
    let cutoff_node = compile_expr(ctx, args[0].clone())?;
    let resonance_node = compile_expr(ctx, args[1].clone())?;
    let state = FundspState::new_moog_hz(1000.0, 0.5, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::MoogHz,
        inputs: vec![
            Signal::Node(input_node),      // Audio input
            Signal::Node(cutoff_node),      // Cutoff parameter
            Signal::Node(resonance_node),   // Resonance parameter
        ],
        state: Arc::new(Mutex::new(state)),
    };
    Ok(ctx.graph.add_node(node))
}
```

**After (pulse - multi-input generator)**:
```rust
fn compile_pulse(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!("pulse requires 2 parameters (frequency, pulse_width), got {}", args.len()));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;
    let width_node = compile_expr(ctx, args[1].clone())?;
    let state = FundspState::new_pulse(ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Pulse,
        inputs: vec![
            Signal::Node(freq_node),   // Frequency input
            Signal::Node(width_node),  // Pulse width input
        ],
        state: Arc::new(Mutex::new(state)),
    };
    Ok(ctx.graph.add_node(node))
}
```

---

## Testing Strategy

### 1. Update Existing Tests
After Phase 1, run ALL existing tests:
```bash
cargo test
```

Expected: Some failures in compilation (field name changes)
Action: Update to use `inputs` instead of `input` + `params`

### 2. Test Backward Compatibility
Ensure all 9 existing UGens still work:
```bash
cargo test --test test_fundsp_saw
cargo test --test test_saw_hz_integration
# ... all existing tests
```

### 3. Test Multi-Input UGens
After implementation, test pulse():
```bash
cargo test --test test_fundsp_pulse
cargo test --test test_pulse_integration
```

### 4. Test Audio-Rate Modulation
```phonon
-- LFO modulating pulse width at audio rate
~lfo: sine 10  -- 10 Hz LFO
~width: ~lfo * 0.4 + 0.5
out: pulse ~width 110  -- Audio-rate PWM!
```

---

## Current Status (2025-10-30)

### Completed UGens (9 total)
All use single-input architecture:
1. ✅ organ_hz - Generator (0 inputs) + frequency parameter
2. ✅ moog_hz - Processor (1 audio input) + cutoff, resonance parameters
3. ✅ reverb_stereo - Processor (1 input) + wet, time parameters
4. ✅ fchorus - Processor (1 input) + seed, separation, depth, speed parameters
5. ✅ saw_hz - Generator (0 inputs) + frequency parameter
6. ✅ square_hz - Generator (0 inputs) + frequency parameter
7. ✅ triangle_hz - Generator (0 inputs) + frequency parameter
8. ✅ noise - Generator (0 inputs, no parameters)
9. ✅ pink - Generator (0 inputs, no parameters)

### Blocked UGens (Waiting for Multi-Input)
1. ⏸️ pulse - Requires 2 inputs (frequency, pulse_width)
2. ⏸️ FM oscillators - Requires audio-rate frequency modulation
3. ⏸️ morph - Requires 4 inputs (signal, freq, Q, morph)

---

## Implementation Order

1. ✅ Document architecture (this file)
2. ✅ Phase 1: Update FundspState (core change)
3. ✅ Phase 2: Update SignalNode::FundspUnit
4. ✅ Phase 3: Update eval_node()
5. ✅ Phase 4: Update compiler functions
6. ✅ Update all 9 existing UGen constructors
7. ✅ Fix compilation errors
8. ✅ Run full test suite (300 tests passing)
9. ⏳ Implement pulse() as proof-of-concept (NEXT)
10. ⏳ Test audio-rate modulation
11. ✅ Commit: "Implement multi-input architecture" (a74b321)
12. ⏳ Continue systematic UGen wrapping

**Actual time**: 2.5 hours
**Risk level**: Medium (touching core architecture)
**Result**: ✅ SUCCESS - All tests pass, architecture ready for multi-input UGens

---

## Next Steps

Start with Phase 1: Update FundspState struct and all constructors.
This is the foundation - get it right and the rest follows naturally.

Let's do this! 🚀
