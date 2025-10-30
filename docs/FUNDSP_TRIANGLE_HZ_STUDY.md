# fundsp triangle_hz Study Notes

**Date**: 2025-10-30
**UGen**: triangle_hz (Bandlimited triangle wave oscillator)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn triangle_hz(frequency: f32) -> impl AudioUnit
```

### Parameters
- **frequency**: Oscillator frequency in Hz (typical: 20 Hz - 20 kHz)

### Input/Output
- **Inputs**: 0 (generator, no audio input)
- **Outputs**: 1 (mono triangle wave)

### Characteristics
- Bandlimited triangle wave oscillator (no aliasing)
- Very mellow, soft timbre
- Contains only odd harmonics (1, 3, 5, 7...) with 1/n² falloff
- Much softer than square wave (1/n² vs 1/n falloff)
- DC-centered waveform
- Classic for smooth bass, flutes, soft leads

---

## Design Decisions for Phonon Integration

### Comparison to Other Oscillators

**saw_hz**:
- All harmonics (1, 2, 3, 4...)
- Bright, buzzy timbre
- Sawtooth ramp shape

**square_hz**:
- Only odd harmonics (1, 3, 5, 7...)
- Hollow, woody timbre
- Square pulse shape

**triangle_hz** (this implementation):
- Only odd harmonics with 1/n² falloff
- Very mellow, soft timbre
- Triangle shape
- Softest of the classic waveforms

**All three complete the classic oscillator set!**

### Phonon DSL Syntax

```phonon
-- Basic triangle wave oscillator
~tri: triangle_hz 220

-- Pattern-controlled frequency (Phonon's killer feature!)
~melody: triangle_hz "110 165 220 330"

-- LFO modulation of triangle frequency
~lfo: sine 0.5
~freq: ~lfo * 100 + 220
~modulated: triangle_hz ~freq

-- Soft bass sound
~bass: triangle_hz 55

-- Flute-like lead
~flute: triangle_hz 440 # lpf 1000 0.3

-- Output
out: ~tri * 0.3
```

### Naming

Use `triangle_hz` to match fundsp naming convention and other oscillators (saw_hz, square_hz) for consistency.

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
    SquareHz,
    TriangleHz,  // NEW
}
```

### 2. Constructor

```rust
pub fn new_triangle_hz(frequency: f32, sample_rate: f64) -> Self {
    use fundsp::prelude::AudioUnit;

    let mut unit = fundsp::prelude::triangle_hz(frequency);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |_input: f32| -> f32 {
        // triangle_hz: 0 inputs -> 1 output (generator)
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::TriangleHz,
        params: vec![frequency],
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
pub fn update_triangle_frequency(&mut self, new_freq: f32, sample_rate: f64) {
    let freq_changed = (self.params[0] - new_freq).abs() > 0.1;

    if freq_changed {
        *self = Self::new_triangle_hz(new_freq, sample_rate);
    }
}
```

### 4. Compiler Function

```rust
fn compile_triangle_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "triangle_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_triangle_hz(440.0, ctx.graph.sample_rate() as f64);

    // Create constant node for "no input" (triangle_hz is a generator)
    let no_input = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::TriangleHz,
        input: Signal::Node(no_input),
        params: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. eval_node() Update

```rust
FundspUnitType::TriangleHz => {
    // Parameters: 0=frequency
    if param_values.len() >= 1 {
        let frequency = param_values[0];
        state_guard.update_triangle_frequency(frequency, self.sample_rate as f64);
    }
}
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_triangle_hz_basic() {
    // Test that fundsp triangle_hz generates audio
    let mut unit = triangle_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    assert!(sum > 0.0, "Triangle should produce output");
    println!("Triangle 440 Hz - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_triangle_hz_frequency() {
    // Test different frequencies
    let sample_rate = 44100.0;

    let mut unit_low = triangle_hz(55.0);
    unit_low.reset();
    unit_low.set_sample_rate(sample_rate);

    let mut unit_high = triangle_hz(2000.0);
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
fn test_fundsp_triangle_hz_waveform_shape() {
    // Test that waveform has triangle shape
    let mut unit = triangle_hz(100.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let period_samples = (44100.0 / 100.0) as usize;
    let mut samples = Vec::new();

    for _ in 0..period_samples * 2 {
        let frame = unit.tick(&Default::default());
        samples.push(frame[0]);
    }

    // Triangle wave should have both positive and negative values
    let has_positive = samples.iter().any(|&s| s > 0.0);
    let has_negative = samples.iter().any(|&s| s < 0.0);

    assert!(has_positive, "Triangle should have positive values");
    assert!(has_negative, "Triangle should have negative values");

    println!("Triangle waveform range: {:.3} to {:.3}",
        samples.iter().cloned().fold(f32::INFINITY, f32::min),
        samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max));
}

#[test]
fn test_fundsp_triangle_hz_dc_centered() {
    // Test that waveform is DC-centered (average ~ 0)
    let mut unit = triangle_hz(440.0);
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
    assert!(average.abs() < 0.01, "Triangle should be DC-centered");
    println!("Triangle DC offset: {:.6}", average);
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_triangle_hz_level3_basic() {
    let code = "out: triangle_hz 220";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic triangle_hz 220 Hz - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_triangle_hz_level3_frequency_sweep() {
    let frequencies = vec![55.0, 110.0, 220.0, 440.0, 880.0];

    for freq in &frequencies {
        let code = format!("out: triangle_hz {}", freq);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Frequency {} should produce output", freq);
        println!("Frequency {} Hz: RMS {:.4}", freq, rms);
    }
}

#[test]
fn test_triangle_hz_level3_pattern_control() {
    let code = r#"
        tempo: 2.0
        out: triangle_hz "110 165 220 330"
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled triangle should work");
    println!("Pattern control - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_soft_bass() {
    // Triangle bass is softer than square/saw
    let code = "out: triangle_hz 55";
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Triangle bass should work");
    println!("Triangle bass - RMS: {:.4}", rms);
}

#[test]
fn test_triangle_hz_level3_vs_oscillators() {
    // Compare triangle to square and saw
    let code_triangle = "out: triangle_hz 220";
    let code_square = "out: square_hz 220";
    let code_saw = "out: saw_hz 220";

    let audio_triangle = render_dsl(code_triangle, 1.0);
    let audio_square = render_dsl(code_square, 1.0);
    let audio_saw = render_dsl(code_saw, 1.0);

    let rms_triangle = calculate_rms(&audio_triangle);
    let rms_square = calculate_rms(&audio_square);
    let rms_saw = calculate_rms(&audio_saw);

    // All should have energy
    assert!(rms_triangle > 0.01);
    assert!(rms_square > 0.01);
    assert!(rms_saw > 0.01);

    println!("Triangle RMS: {:.4}, Square RMS: {:.4}, Saw RMS: {:.4}",
        rms_triangle, rms_square, rms_saw);
}
```

---

## Expected Outcomes

After implementation:
- ✅ triangle_hz works from Phonon DSL
- ✅ Pattern modulation of frequency at audio rate
- ✅ Classic triangle wave sound (soft, mellow)
- ✅ Bandlimited (no aliasing)
- ✅ Perfect for soft bass, flutes, smooth leads

---

## Implementation Checklist

- [ ] Add TriangleHz variant to FundspUnitType
- [ ] Implement new_triangle_hz constructor
- [ ] Update tick() to handle TriangleHz (generator, no input)
- [ ] Add update_triangle_frequency method
- [ ] Update Clone implementation
- [ ] Add TriangleHz case to eval_node()
- [ ] Add compile_triangle_hz function
- [ ] Register "triangle_hz" keyword
- [ ] Create test_fundsp_triangle.rs (direct API)
- [ ] Create test_triangle_hz_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 25-30 minutes (workflow is very fast now!)

---

## Musical Use Cases

```phonon
-- Soft bass (much mellower than square/saw)
~bass: triangle_hz 55

-- Flute-like lead
~flute: triangle_hz 880 # lpf 1500 0.3

-- Smooth pad
~pad: triangle_hz "110 165 220" # reverb_stereo 0.9 0.5

-- Compare waveforms
~tri: triangle_hz 220
~sq: square_hz 220
~saw: saw_hz 220
~mix: (~tri * 0.4 + ~sq * 0.3 + ~saw * 0.3) # lpf 1000 0.7

-- Octave stack (rich but mellow)
~tri1: triangle_hz 110
~tri2: triangle_hz 220
~tri3: triangle_hz 440
~stack: (~tri1 * 0.5 + ~tri2 * 0.3 + ~tri3 * 0.2)
```

---

## Technical Notes

### Harmonic Content Comparison

**Sawtooth**:
- Harmonics: 1, 2, 3, 4, 5, 6...
- Amplitude falloff: 1/n
- Sound: Bright, buzzy

**Square**:
- Harmonics: 1, 3, 5, 7, 9... (odd only)
- Amplitude falloff: 1/n
- Sound: Hollow, woody

**Triangle**:
- Harmonics: 1, 3, 5, 7, 9... (odd only)
- Amplitude falloff: 1/n² (much faster!)
- Sound: Soft, mellow, flute-like

This makes triangle wave perfect for:
- Soft bass sounds (clean fundamental)
- Flute/recorder emulation
- Smooth lead tones
- Mellower alternative to square
- Mixing with brighter waveforms for warmth

---

**Study Complete**: Ready to implement! 🚀
