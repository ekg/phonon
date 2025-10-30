# fundsp reverb2_stereo Study Notes

**Date**: 2025-10-30
**UGen**: reverb2_stereo (Stereo reverb effect)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn reverb2_stereo(
    wet: f32,
    time: f32,
    damping: f32,
    diffusion: f32
) -> impl AudioUnit
```

### Parameters
- **wet**: Wet/dry mix (0.0 = dry, 1.0 = wet)
- **time**: Reverb time in seconds (decay time)
- **damping**: High frequency damping (0.0-1.0, higher = more damping)
- **diffusion**: Reverb diffusion (0.0-1.0, controls echo density)

### Input/Output
- **Inputs**: 2 (stereo audio signal to process)
- **Outputs**: 2 (stereo reverb output)

### Characteristics
- Stereo reverb (takes stereo in, outputs stereo)
- Algorithmic reverb (not convolution)
- Real-time friendly
- Suitable for musical applications

---

## Challenge: Stereo Handling

**This is a CRITICAL test for our fundsp wrapper architecture!**

### The Problem
- Phonon's unified_graph is currently **MONO** (f32 samples)
- fundsp reverb2_stereo expects **STEREO** input (2 channels)
- fundsp reverb2_stereo returns **STEREO** output (2 channels)

### Options

#### Option 1: Stay Mono (For Now)
- Use fundsp's `reverb_stereo` which takes mono input, outputs stereo
- Or convert mono to stereo internally in the wrapper
- **Downside**: Doesn't test true stereo path

#### Option 2: Skip reverb2_stereo, Use reverb_stereo Instead
```rust
pub fn reverb_stereo(wet: f32, time: f32) -> impl AudioUnit
```
- Takes 1 input (mono)
- Returns 2 outputs (stereo)
- Simpler, fits current mono architecture
- **Better choice for now!**

#### Option 3: Add Stereo Support to Phonon (Future)
- Extend SignalNode to support multi-channel signals
- Add stereo bus support
- Much larger architectural change
- **Future work after 60+ UGens wrapped**

---

## Decision: Use `reverb_stereo` Instead

Since Phonon's architecture is currently mono, I'll wrap `reverb_stereo` which:
- Takes **1 mono input**
- Returns **2 stereo outputs** (L/R)
- Simpler parameters (wet, time) vs (wet, time, damping, diffusion)
- Still tests multi-output handling

### reverb_stereo API
```rust
pub fn reverb_stereo(wet: f32, time: f32) -> impl AudioUnit
```

**Parameters:**
- wet: 0.0-1.0 (dry/wet mix)
- time: reverb time in seconds (0.1-10.0 typical range)

**Input/Output:**
- Inputs: 1 (mono audio)
- Outputs: 2 (stereo L/R)

---

## Implementation Challenge: Multi-Output Handling

### Current FundspState
```rust
pub fn tick(&mut self, input: f32) -> f32 {
    (self.tick_fn)(input)
}
```

**Problem:** Returns single f32, but reverb_stereo outputs 2 channels!

### Solution: Support Multi-Output in FundspState

```rust
pub enum FundspOutput {
    Mono(f32),
    Stereo(f32, f32),  // (left, right)
}

pub fn tick(&mut self, input: f32) -> FundspOutput {
    (self.tick_fn)(input)
}
```

**Then in eval_node():**
```rust
let output = state_guard.tick(input_sample);
match output {
    FundspOutput::Mono(sample) => sample,
    FundspOutput::Stereo(left, _right) => left,  // For now, use left channel
}
```

**Or simpler:** Just output left channel for now, add stereo later:
```rust
pub fn new_reverb_stereo(wet: f32, time: f32, sample_rate: f64) -> Self {
    let mut unit = fundsp::prelude::reverb_stereo(wet, time);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |input: f32| -> f32 {
        // reverb_stereo takes 1 input, returns 2 outputs
        let output_frame = unit.tick(&[input].into());
        output_frame[0]  // Return left channel only (for now)
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::ReverbStereo,
        params: vec![wet, time],
        sample_rate,
    }
}
```

---

## Implementation Plan

### 1. FundspState Variant
```rust
ReverbStereo {
    unit: Box<dyn FnMut(f32) -> f32 + Send>,
    wet: f32,
    time: f32,
}
```

### 2. Constructor
```rust
pub fn new_reverb_stereo(wet: f32, time: f32, sample_rate: f64) -> Self {
    let mut unit = fundsp::prelude::reverb_stereo(wet, time);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |input: f32| -> f32 {
        // reverb_stereo: 1 input -> 2 outputs
        let output_frame = unit.tick(&[input].into());
        output_frame[0]  // Left channel (mono output for now)
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::ReverbStereo,
        params: vec![wet, time],
        sample_rate,
    }
}
```

### 3. Update Parameters
```rust
pub fn update_reverb_params(&mut self, new_wet: f32, new_time: f32, sample_rate: f64) {
    let wet_changed = (self.params[0] - new_wet).abs() > 0.01;
    let time_changed = (self.params[1] - new_time).abs() > 0.05;

    if wet_changed || time_changed {
        *self = Self::new_reverb_stereo(new_wet, new_time, sample_rate);
    }
}
```

### 4. Compiler Function
```rust
fn compile_reverb_stereo(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles chained form)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "reverb_stereo requires 2 parameters (wet, time), got {}",
            params.len()
        ));
    }

    let wet_node = compile_expr(ctx, params[0].clone())?;
    let time_node = compile_expr(ctx, params[1].clone())?;

    // Create fundsp reverb_stereo unit (initialized with default params)
    let state = FundspState::new_reverb_stereo(0.5, 1.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::ReverbStereo,
        input: input_signal,
        params: vec![Signal::Node(wet_node), Signal::Node(time_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. DSL Syntax
```phonon
-- Basic usage
out: saw 110 # reverb_stereo 0.3 2.0

-- With pattern modulation
~wet: sine 0.2 * 0.3 + 0.3  -- Oscillate wet 0.3-0.6
~drums: s "bd sn hh*4 cp"
out: ~drums # reverb_stereo ~wet 1.5

-- Long reverb tail
~pad: sine "55 82.5 110"
out: ~pad # reverb_stereo 0.8 5.0
```

---

## Test Plan

### Level 1: Direct fundsp API Test
```rust
#[test]
fn test_fundsp_reverb_stereo_basic() {
    let mut unit = fundsp::prelude::reverb_stereo(0.5, 1.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Send impulse through reverb
    let impulse_frame = unit.tick(&[1.0].into());

    // Should have output on both channels
    assert!(impulse_frame[0].abs() > 0.0);  // Left
    assert!(impulse_frame[1].abs() > 0.0);  // Right

    // Subsequent samples should have reverb tail
    let mut tail_sum = 0.0;
    for _ in 0..44100 {
        let frame = unit.tick(&[0.0].into());  // No input
        tail_sum += frame[0].abs() + frame[1].abs();
    }

    // Should have significant reverb tail
    assert!(tail_sum > 1.0);
}
```

### Level 2: Phonon Integration Tests
```rust
#[test]
fn test_reverb_stereo_level3_basic() {
    let code = "out: saw 220 # reverb_stereo 0.5 1.0";
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    // Should have energy
    assert!(rms > 0.01);

    // Compare to dry signal
    let code_dry = "out: saw 220";
    let audio_dry = render_dsl(code_dry, 2.0);
    let rms_dry = calculate_rms(&audio_dry);

    // Reverb should affect RMS (might be lower or higher depending on mix)
    println!("Reverb RMS: {:.4}, Dry RMS: {:.4}", rms, rms_dry);
}

#[test]
fn test_reverb_stereo_level3_wet_sweep() {
    // Test different wet amounts
    for wet in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let code = format!("out: saw 220 # reverb_stereo {} 1.0", wet);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Wet {} should produce output", wet);
        println!("Wet {}: RMS {:.4}", wet, rms);
    }
}

#[test]
fn test_reverb_stereo_level3_time_sweep() {
    // Test different reverb times
    for time in [0.1, 0.5, 1.0, 2.0, 5.0] {
        let code = format!("out: saw 220 # reverb_stereo 0.5 {}", time);
        let audio = render_dsl(&code, 2.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Time {}s should produce output", time);
        println!("Time {}s: RMS {:.4}", time, rms);
    }
}

#[test]
fn test_reverb_stereo_level3_pattern_modulation() {
    let code = "
        tempo: 2.0
        ~wet: sine 0.5 * 0.3 + 0.3
        out: saw 110 # reverb_stereo ~wet 1.5
    ";
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern modulated reverb should work");
}

#[test]
fn test_reverb_stereo_level3_impulse_response() {
    // Test reverb tail by sending brief signal then measuring decay
    // This would require time-domain analysis
    // For now, just verify it produces output
    let code = "out: saw 220 # reverb_stereo 0.8 2.0";
    let audio = render_dsl(code, 3.0);  // 3 seconds to hear tail
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01);
}
```

---

## Expected Outcomes

After implementation:
- ✅ reverb_stereo works from Phonon DSL
- ✅ Pattern modulation of wet/time parameters
- ✅ Classic algorithmic reverb sound
- ✅ Tests multi-output fundsp units (stereo)

**Note:** Currently outputs mono (left channel only). Future: Add stereo bus support to Phonon for true L/R reverb.

---

## Implementation Checklist

- [ ] Add ReverbStereo variant to FundspUnitType
- [ ] Implement new_reverb_stereo constructor
- [ ] Update tick() to handle ReverbStereo (returns left channel)
- [ ] Add update_reverb_params method
- [ ] Update Clone implementation
- [ ] Update Debug implementation
- [ ] Add ReverbStereo case to eval_node()
- [ ] Add compile_reverb_stereo function
- [ ] Register "reverb_stereo" keyword
- [ ] Create test_fundsp_reverb.rs (direct API)
- [ ] Create test_reverb_stereo_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 45 minutes (getting faster!)

---

## Future Work: True Stereo Support

When Phonon gets stereo support:
- Change SignalNode to support Vec<f32> samples (multi-channel)
- Update eval_node() to handle stereo buses
- Return both channels from reverb_stereo
- Add pan2_l, pan2_r nodes (fundsp has these!)
- Support true stereo signal flow

**For now:** Mono path works, proves multi-output wrapping works.

---

**Study Complete**: Ready to implement! 🚀
