# fundsp moog_hz Study Notes

**Date**: 2025-10-30
**UGen**: moog_hz (Moog ladder filter)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn moog_hz(cutoff: f32, resonance: f32) -> impl AudioUnit
```

### Parameters
- **cutoff**: Cutoff frequency in Hz (typically 20-20000 Hz)
- **resonance**: Resonance/Q factor (0.0-1.0, can self-oscillate near 1.0)

### Input/Output
- **Inputs**: 1 (audio signal to filter)
- **Outputs**: 1 (filtered audio)

### Characteristics
- 4-pole 24dB/octave lowpass filter
- Classic analog Moog sound
- Self-oscillates at high resonance (near 1.0)
- Non-linear behavior adds warmth

---

## Comparison to Phonon's Custom moogLadder

We already have a custom moogLadder implementation! This is perfect for:
- **Level 4 comparative testing** (custom vs fundsp)
- Verifying our custom implementation correctness
- Benchmarking performance

### Custom Implementation Location
- `src/unified_graph.rs`: `SignalNode::MoogLadder`
- Takes 3 inputs: audio, cutoff, resonance

---

## Implementation Plan

### 1. FundspState Variant
```rust
MoogHz {
    unit: Box<dyn FnMut(f32) -> f32 + Send>,
    cutoff: f32,
    resonance: f32,
}
```

### 2. Constructor
```rust
pub fn new_moog_hz(cutoff: f32, resonance: f32, sample_rate: f64) -> Self {
    let mut unit = fundsp::prelude::moog_hz(cutoff, resonance);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |input: f32| -> f32 {
        // moog_hz takes 1 input, returns 1 output
        let input_frame = [input];
        let output_frame = unit.tick(&input_frame);
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::MoogHz,
        params: vec![cutoff, resonance],
        sample_rate,
    }
}
```

### 3. Update Parameters
```rust
pub fn update_moog_params(&mut self, new_cutoff: f32, new_resonance: f32, sample_rate: f64) {
    let cutoff_changed = (self.params[0] - new_cutoff).abs() > 1.0;
    let resonance_changed = (self.params[1] - new_resonance).abs() > 0.01;

    if cutoff_changed || resonance_changed {
        *self = Self::new_moog_hz(new_cutoff, new_resonance, sample_rate);
    }
}
```

### 4. Compiler Function
```rust
fn compile_moog_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err("moog_hz requires 3 arguments: input, cutoff, resonance".to_string());
    }

    let input_node = compile_expr(ctx, args[0].clone())?;
    let cutoff_node = compile_expr(ctx, args[1].clone())?;
    let resonance_node = compile_expr(ctx, args[2].clone())?;

    // Create fundsp moog_hz unit (initialized with default params)
    let state = FundspState::new_moog_hz(1000.0, 0.5, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::MoogHz,
        input: Signal::Node(input_node),
        params: vec![Signal::Node(cutoff_node), Signal::Node(resonance_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. DSL Syntax
```phonon
-- Basic usage
out: saw 220 # moog_hz 1000 0.7

-- With pattern modulation
~lfo: sine 0.5
~cutoff: ~lfo * 2000 + 1000
out: saw 110 # moog_hz ~cutoff 0.8

-- Compare to custom moogLadder
~saw: saw 220
~fundsp: ~saw # moog_hz 1000 0.7
~custom: ~saw # moogLadder 1000 0.7
out: ~fundsp  -- Or compare: (~fundsp + ~custom) * 0.5
```

---

## Test Plan

### Level 1: Direct fundsp API Test
```rust
#[test]
fn test_fundsp_moog_basic() {
    let mut unit = fundsp::prelude::moog_hz(1000.0, 0.7);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Generate saw wave input
    let mut saw_phase = 0.0;
    let mut output_sum = 0.0;

    for _ in 0..44100 {
        let saw = 2.0 * saw_phase - 1.0;
        saw_phase += 220.0 / 44100.0;
        if saw_phase >= 1.0 { saw_phase -= 1.0; }

        let input_frame = [saw];
        let output_frame = unit.tick(&input_frame);
        output_sum += output_frame[0].abs();
    }

    // Should have filtered output
    assert!(output_sum > 0.0);
}
```

### Level 2: Phonon Integration Tests
```rust
#[test]
fn test_moog_hz_level3_basic_filtering() {
    let code = "out: saw 220 # moog_hz 1000 0.7";
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    // Should have energy
    assert!(rms > 0.01);
    assert!(rms < 1.0);
}

#[test]
fn test_moog_hz_level3_cutoff_sweep() {
    // Test different cutoff frequencies
    for cutoff in [100, 500, 1000, 5000, 10000] {
        let code = format!("out: saw 220 # moog_hz {} 0.5", cutoff);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);

        // All should have energy
        assert!(rms > 0.01, "Cutoff {} Hz: RMS too low", cutoff);
    }
}

#[test]
fn test_moog_hz_level3_resonance_sweep() {
    // Test different resonance values
    for res in [0.0, 0.3, 0.5, 0.7, 0.9] {
        let code = format!("out: saw 220 # moog_hz 1000 {}", res);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);
        let peak = calculate_peak(&audio);

        // Higher resonance should increase peak
        println!("Resonance {}: RMS {:.4}, Peak {:.4}", res, rms, peak);
    }
}
```

### Level 3: Pattern Modulation Test
```rust
#[test]
fn test_moog_hz_level3_pattern_modulation() {
    let code = "
        tempo: 0.5
        ~lfo: sine 0.5
        ~cutoff: ~lfo * 2000 + 1000
        out: saw 110 # moog_hz ~cutoff 0.7
    ";
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    // Should have energy with modulated filter
    assert!(rms > 0.01);

    // Compare to static cutoff
    let code_static = "out: saw 110 # moog_hz 1000 0.7";
    let audio_static = render_dsl(code_static, 2.0);
    let rms_static = calculate_rms(&audio_static);

    // Should have similar energy (within 50%)
    let ratio = rms / rms_static;
    assert!(ratio > 0.5 && ratio < 1.5);
}
```

### Level 4: Comparative Test (fundsp vs Custom)
```rust
#[test]
fn test_moog_hz_level4_vs_custom_moog_ladder() {
    // fundsp implementation
    let code_fundsp = "out: saw 220 # moog_hz 1000 0.7";
    let audio_fundsp = render_dsl(code_fundsp, 1.0);

    // Our custom implementation
    let code_custom = "out: saw 220 # moogLadder 1000 0.7";
    let audio_custom = render_dsl(code_custom, 1.0);

    let rms_fundsp = calculate_rms(&audio_fundsp);
    let rms_custom = calculate_rms(&audio_custom);

    // Should have similar amplitude (within 50%)
    let ratio = rms_fundsp / rms_custom;
    assert!(
        ratio > 0.5 && ratio < 1.5,
        "fundsp/custom ratio too different: {:.2}",
        ratio
    );

    println!(
        "fundsp RMS: {:.4}, custom RMS: {:.4}, ratio: {:.2}",
        rms_fundsp, rms_custom, ratio
    );
}

#[test]
fn test_moog_hz_level4_spectral_comparison() {
    // Compare frequency content
    // fundsp's moog should have stronger low-pass characteristics
    let code_fundsp = "out: white_noise # moog_hz 1000 0.5";
    let code_custom = "out: white_noise # moogLadder 1000 0.5";

    // Both should filter high frequencies
    // (Could add FFT analysis here if needed)
}
```

---

## Expected Outcomes

After implementation:
- ✅ moog_hz works from Phonon DSL
- ✅ Pattern modulation of cutoff and resonance
- ✅ Comparable to custom moogLadder (validates both implementations)
- ✅ Classic Moog ladder sound available

---

## Implementation Checklist

- [ ] Add MoogHz variant to FundspState
- [ ] Implement new_moog_hz constructor
- [ ] Update tick() to handle MoogHz
- [ ] Add update_moog_params method
- [ ] Update Clone implementation
- [ ] Update Debug implementation
- [ ] Add MoogHz case to eval_node()
- [ ] Add compile_moog_hz function
- [ ] Register "moog_hz" keyword
- [ ] Create test_fundsp_moog.rs (direct API)
- [ ] Create test_moog_hz_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 1 hour (we're getting faster!)

---

**Study Complete**: Ready to implement! 🚀
