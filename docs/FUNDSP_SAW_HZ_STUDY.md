# fundsp saw_hz Study Notes

**Date**: 2025-10-30
**UGen**: saw_hz (Bandlimited sawtooth oscillator)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn saw_hz(frequency: f32) -> impl AudioUnit
```

### Parameters
- **frequency**: Oscillator frequency in Hz (typical: 20 Hz - 20 kHz)

### Input/Output
- **Inputs**: 0 (generator, no audio input)
- **Outputs**: 1 (mono sawtooth wave)

### Characteristics
- Bandlimited sawtooth oscillator (no aliasing)
- Bright, buzzy timbre
- Contains all harmonics (fundamental + overtones)
- DC-centered waveform

---

## Design Decisions for Phonon Integration

### Comparison to organ_hz

**organ_hz** (already implemented):
- Additive synthesis oscillator
- Multiple harmonics
- Organ-like timbre
- 1 parameter (frequency)

**saw_hz** (this implementation):
- Subtractive synthesis oscillator
- Bright, rich harmonic content
- Classic analog synth sound
- 1 parameter (frequency)

**Different use cases!** Both are useful.

### Phonon DSL Syntax

```phonon
-- Basic sawtooth oscillator
~saw: saw_hz 220

-- Pattern-controlled frequency (Phonon's killer feature!)
~melody: saw_hz "110 165 220 330"

-- LFO modulation of saw frequency
~lfo: sine 0.5
~freq: ~lfo * 100 + 220
~modulated: saw_hz ~freq

-- Output
out: ~saw * 0.3
```

### Naming

Use `saw_hz` to match fundsp naming convention and differentiate from Phonon's custom `saw` oscillator (if it exists).

---

## Implementation Plan

### 1. FundspState Variant

```rust
pub enum FundspUnitType {
    OrganHz,
    MoogHz,
    ReverbStereo,
    Chorus,
    SawHz,  // NEW
}
```

### 2. Constructor

```rust
pub fn new_saw_hz(frequency: f32, sample_rate: f64) -> Self {
    use fundsp::prelude::AudioUnit;

    let mut unit = fundsp::prelude::saw_hz(frequency);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |_input: f32| -> f32 {
        // saw_hz: 0 inputs -> 1 output (generator)
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::SawHz,
        params: vec![frequency],
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
pub fn update_saw_frequency(&mut self, new_freq: f32, sample_rate: f64) {
    let freq_changed = (self.params[0] - new_freq).abs() > 0.1;

    if freq_changed {
        *self = Self::new_saw_hz(new_freq, sample_rate);
    }
}
```

### 4. Compiler Function

```rust
fn compile_saw_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "saw_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Create fundsp saw_hz unit (initialized with default freq)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_saw_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SawHz,
        input: Signal::Node(ctx.graph.add_node(SignalNode::Constant { value: 0.0 })), // No input
        params: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. eval_node() Update

```rust
FundspUnitType::SawHz => {
    // Parameters: 0=frequency
    if param_values.len() >= 1 {
        let frequency = param_values[0];
        state_guard.update_saw_frequency(frequency, self.sample_rate as f64);
    }
}
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_saw_hz_basic() {
    // Test that fundsp saw_hz generates audio
    let mut unit = saw_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Generate audio
    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    // Should have output
    assert!(sum > 0.0, "Saw should produce output");
    println!("Saw 440 Hz - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_saw_hz_frequency() {
    // Test different frequencies
    let sample_rate = 44100.0;

    // Low frequency (bass)
    let mut unit_low = saw_hz(55.0);
    unit_low.reset();
    unit_low.set_sample_rate(sample_rate);

    // High frequency (treble)
    let mut unit_high = saw_hz(2000.0);
    unit_high.reset();
    unit_high.set_sample_rate(sample_rate);

    let mut low_sum = 0.0;
    let mut high_sum = 0.0;

    for _ in 0..44100 {  // 1 second
        let low_frame = unit_low.tick(&Default::default());
        let high_frame = unit_high.tick(&Default::default());

        low_sum += low_frame[0].abs();
        high_sum += high_frame[0].abs();
    }

    println!("Low (55 Hz) sum: {:.2}", low_sum);
    println!("High (2000 Hz) sum: {:.2}", high_sum);

    // Both should produce output
    assert!(low_sum > 0.0);
    assert!(high_sum > 0.0);
}

#[test]
fn test_fundsp_saw_hz_waveform_shape() {
    // Test that waveform has sawtooth shape
    let mut unit = saw_hz(100.0);  // Low frequency to see shape
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Collect one period of samples
    let period_samples = (44100.0 / 100.0) as usize;
    let mut samples = Vec::new();

    for _ in 0..period_samples * 2 {
        let frame = unit.tick(&Default::default());
        samples.push(frame[0]);
    }

    // Sawtooth should have both positive and negative values
    let has_positive = samples.iter().any(|&s| s > 0.0);
    let has_negative = samples.iter().any(|&s| s < 0.0);

    assert!(has_positive, "Sawtooth should have positive values");
    assert!(has_negative, "Sawtooth should have negative values");

    println!("Saw waveform range: {:.3} to {:.3}",
        samples.iter().cloned().fold(f32::INFINITY, f32::min),
        samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max));
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_saw_hz_level3_basic() {
    // Test basic saw oscillator
    let code = "out: saw_hz 220";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.1, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic saw_hz 220 - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_saw_hz_level3_frequency_sweep() {
    // Test different frequencies
    let frequencies = vec![55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!("out: saw_hz {}", freq);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.1, "Frequency {} should produce output", freq);
        println!("Frequency {} Hz: RMS {:.4}", freq, rms);
    }
}

#[test]
fn test_saw_hz_level3_pattern_control() {
    // Test pattern-controlled frequency (Phonon's killer feature!)
    let code = r#"
        tempo: 2.0
        out: saw_hz "110 165 220 330"
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.1, "Pattern-controlled saw should work: {}", rms);

    println!("Pattern control - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_lfo_modulation() {
    // Test LFO modulation of frequency
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5
        ~freq: ~lfo * 100 + 220
        out: saw_hz ~freq
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.1, "LFO modulated saw should work: {}", rms);

    println!("LFO modulation - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_through_filter() {
    // Test saw through filter
    let code = "out: saw_hz 110 # lpf 500 0.8";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Filtered saw should work");

    println!("Saw through LPF - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_bass() {
    // Test bass frequency
    let code = "out: saw_hz 55";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.1, "Bass saw should work");

    println!("Bass saw (55 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_saw_hz_level3_high() {
    // Test high frequency
    let code = "out: saw_hz 2000";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.1, "High saw should work");

    println!("High saw (2000 Hz) - RMS: {:.4}", rms);
}
```

---

## Expected Outcomes

After implementation:
- ✅ saw_hz works from Phonon DSL
- ✅ Pattern modulation of frequency at audio rate
- ✅ Classic sawtooth sound (bright, harmonic-rich)
- ✅ Bandlimited (no aliasing)
- ✅ Works well through filters

---

## Implementation Checklist

- [ ] Add SawHz variant to FundspUnitType
- [ ] Implement new_saw_hz constructor
- [ ] Update tick() to handle SawHz (generator, no input)
- [ ] Add update_saw_frequency method
- [ ] Update Clone implementation
- [ ] Update Debug implementation
- [ ] Add SawHz case to eval_node()
- [ ] Add compile_saw_hz function
- [ ] Register "saw_hz" keyword
- [ ] Create test_fundsp_saw.rs (direct API)
- [ ] Create test_saw_hz_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 30-40 minutes (getting faster!)

---

## Musical Use Cases

```phonon
-- Classic subtractive synth bass
~bass: saw_hz 55 # lpf 200 0.8

-- Lead synth with filter sweep
~lfo: sine 0.2
~cutoff: ~lfo * 2000 + 500
~lead: saw_hz 440 # lpf ~cutoff 0.7

-- Detuned saw stack (pseudo-supersaw)
~saw1: saw_hz 220
~saw2: saw_hz 221.5
~saw3: saw_hz 218.5
~stack: (~saw1 + ~saw2 + ~saw3) * 0.33

-- Pattern melody
~melody: saw_hz "110 165 220 330 220 165"
out: ~melody # lpf 1000 0.8
```

---

**Study Complete**: Ready to implement! 🚀
