# fundsp square_hz Study Notes

**Date**: 2025-10-30
**UGen**: square_hz (Bandlimited square wave oscillator)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn square_hz(frequency: f32) -> impl AudioUnit
```

### Parameters
- **frequency**: Oscillator frequency in Hz (typical: 20 Hz - 20 kHz)

### Input/Output
- **Inputs**: 0 (generator, no audio input)
- **Outputs**: 1 (mono square wave)

### Characteristics
- Bandlimited square wave oscillator (no aliasing)
- Hollow, woody timbre
- Contains only odd harmonics (1, 3, 5, 7...)
- DC-centered waveform (50% duty cycle)
- Classic for bass, leads, and chiptune sounds

---

## Design Decisions for Phonon Integration

### Comparison to saw_hz (just implemented)

**saw_hz**:
- All harmonics (fundamental + overtones)
- Bright, buzzy timbre
- Sawtooth ramp shape

**square_hz** (this implementation):
- Only odd harmonics
- Hollow, woody timbre
- Square pulse shape
- Different sonic character

**Both are essential oscillators!** Square wave is classic for bass and chiptune.

### Phonon DSL Syntax

```phonon
-- Basic square wave oscillator
~square: square_hz 220

-- Pattern-controlled frequency (Phonon's killer feature!)
~melody: square_hz "110 165 220 330"

-- LFO modulation of square frequency
~lfo: sine 0.5
~freq: ~lfo * 100 + 220
~modulated: square_hz ~freq

-- Classic bass sound
~bass: square_hz 55 # lpf 200 0.8

-- Output
out: ~square * 0.3
```

### Naming

Use `square_hz` to match fundsp naming convention and saw_hz for consistency.

---

## Implementation Plan

### 1. FundspState Variant

```rust
pub enum FundspUnitType {
    OrganHz,
    MoogHz,
    ReverbStereo,
    Chorus,
    SawHz,
    SquareHz,  // NEW
}
```

### 2. Constructor

```rust
pub fn new_square_hz(frequency: f32, sample_rate: f64) -> Self {
    use fundsp::prelude::AudioUnit;

    let mut unit = fundsp::prelude::square_hz(frequency);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |_input: f32| -> f32 {
        // square_hz: 0 inputs -> 1 output (generator)
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::SquareHz,
        params: vec![frequency],
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
pub fn update_square_frequency(&mut self, new_freq: f32, sample_rate: f64) {
    let freq_changed = (self.params[0] - new_freq).abs() > 0.1;

    if freq_changed {
        *self = Self::new_square_hz(new_freq, sample_rate);
    }
}
```

### 4. Compiler Function

```rust
fn compile_square_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "square_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_square_hz(440.0, ctx.graph.sample_rate() as f64);

    // Create constant node for "no input" (square_hz is a generator)
    let no_input = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SquareHz,
        input: Signal::Node(no_input),
        params: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. eval_node() Update

```rust
FundspUnitType::SquareHz => {
    // Parameters: 0=frequency
    if param_values.len() >= 1 {
        let frequency = param_values[0];
        state_guard.update_square_frequency(frequency, self.sample_rate as f64);
    }
}
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_square_hz_basic() {
    // Test that fundsp square_hz generates audio
    let mut unit = square_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    assert!(sum > 0.0, "Square should produce output");
    println!("Square 440 Hz - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_square_hz_frequency() {
    // Test different frequencies
    let sample_rate = 44100.0;

    let mut unit_low = square_hz(55.0);
    unit_low.reset();
    unit_low.set_sample_rate(sample_rate);

    let mut unit_high = square_hz(2000.0);
    unit_high.reset();
    unit_high.set_sample_rate(sample_rate);

    let mut low_sum = 0.0;
    let mut high_sum = 0.0;

    for _ in 0..44100 {
        let low_frame = unit_low.tick(&Default::default());
        let high_frame = unit_high.tick(&Default::default());

        low_sum += low_frame[0].abs();
        high_sum += high_frame[0].abs();
    }

    println!("Low (55 Hz) sum: {:.2}", low_sum);
    println!("High (2000 Hz) sum: {:.2}", high_sum);

    assert!(low_sum > 0.0);
    assert!(high_sum > 0.0);
}

#[test]
fn test_fundsp_square_hz_waveform_shape() {
    // Test that waveform has square shape
    let mut unit = square_hz(100.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let period_samples = (44100.0 / 100.0) as usize;
    let mut samples = Vec::new();

    for _ in 0..period_samples * 2 {
        let frame = unit.tick(&Default::default());
        samples.push(frame[0]);
    }

    // Square wave should have both positive and negative values
    let has_positive = samples.iter().any(|&s| s > 0.0);
    let has_negative = samples.iter().any(|&s| s < 0.0);

    assert!(has_positive, "Square should have positive values");
    assert!(has_negative, "Square should have negative values");

    println!("Square waveform range: {:.3} to {:.3}",
        samples.iter().cloned().fold(f32::INFINITY, f32::min),
        samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max));
}

#[test]
fn test_fundsp_square_hz_dc_centered() {
    // Test that waveform is DC-centered (average ~ 0)
    let mut unit = square_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100;

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // DC offset should be very small
    assert!(average.abs() < 0.01, "Square should be DC-centered");
    println!("Square DC offset: {:.6}", average);
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_square_hz_level3_basic() {
    let code = "out: square_hz 220";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic square_hz 220 Hz - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_square_hz_level3_frequency_sweep() {
    let frequencies = vec![55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!("out: square_hz {}", freq);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Frequency {} should produce output", freq);
        println!("Frequency {} Hz: RMS {:.4}", freq, rms);
    }
}

#[test]
fn test_square_hz_level3_pattern_control() {
    let code = r#"
        tempo: 2.0
        out: square_hz "110 165 220 330"
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled square should work");
    println!("Pattern control - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_bass() {
    // Classic square wave bass
    let code = "out: square_hz 55 # lpf 200 0.8";
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Square bass should work");
    println!("Square bass - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_chiptune() {
    // Chiptune-style melody
    let code = r#"
        tempo: 4.0
        out: square_hz "330 440 550 440"
    "#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Chiptune melody should work");
    println!("Chiptune - RMS: {:.4}", rms);
}

#[test]
fn test_square_hz_level3_vs_saw_hz() {
    // Compare square to saw (different harmonic content)
    let code_square = "out: square_hz 220";
    let code_saw = "out: saw_hz 220";

    let audio_square = render_dsl(code_square, 1.0);
    let audio_saw = render_dsl(code_saw, 1.0);

    let rms_square = calculate_rms(&audio_square);
    let rms_saw = calculate_rms(&audio_saw);

    // Both should have energy
    assert!(rms_square > 0.01);
    assert!(rms_saw > 0.01);

    println!("Square RMS: {:.4}, Saw RMS: {:.4}", rms_square, rms_saw);
}
```

---

## Expected Outcomes

After implementation:
- ✅ square_hz works from Phonon DSL
- ✅ Pattern modulation of frequency at audio rate
- ✅ Classic square wave sound (hollow, woody)
- ✅ Bandlimited (no aliasing)
- ✅ Perfect for bass, leads, chiptune

---

## Implementation Checklist

- [ ] Add SquareHz variant to FundspUnitType
- [ ] Implement new_square_hz constructor
- [ ] Update tick() to handle SquareHz (generator, no input)
- [ ] Add update_square_frequency method
- [ ] Update Clone implementation
- [ ] Add SquareHz case to eval_node()
- [ ] Add compile_square_hz function
- [ ] Register "square_hz" keyword
- [ ] Create test_fundsp_square.rs (direct API)
- [ ] Create test_square_hz_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 30-35 minutes (workflow is very fast now!)

---

## Musical Use Cases

```phonon
-- Classic bass
~bass: square_hz 55 # lpf 200 0.8

-- Lead with filter
~lfo: sine 0.2
~cutoff: ~lfo * 2000 + 500
~lead: square_hz 440 # lpf ~cutoff 0.7

-- Chiptune melody
~chip: square_hz "330 440 550 440 330 220"

-- PWM emulation (mix two detuned squares)
~sq1: square_hz 220
~sq2: square_hz 220.5
~pwm: (~sq1 + ~sq2) * 0.5

-- Classic synth stack
~saw: saw_hz 110
~square: square_hz 110
~mix: (~saw * 0.6 + ~square * 0.4) # lpf 800 0.7
```

---

## Technical Notes

- Square wave = only odd harmonics (1, 3, 5, 7...)
- Saw wave = all harmonics (1, 2, 3, 4...)
- Triangle wave = odd harmonics with 1/n² falloff (even more hollow than square)

This makes square wave perfect for:
- Bass sounds (fundamental + limited harmonics)
- Chiptune/retro game sounds
- Hollow lead tones
- Mixing with saw waves for richer timbres

---

**Study Complete**: Ready to implement! 🚀
