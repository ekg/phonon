# fundsp noise Study Notes

**Date**: 2025-10-30
**UGen**: noise (White noise generator)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn noise() -> impl AudioUnit
```

### Parameters
- **None** - Generates random white noise

### Input/Output
- **Inputs**: 0 (generator, no audio input)
- **Outputs**: 1 (mono white noise)

### Characteristics
- White noise generator (equal energy across all frequencies)
- Random samples between -1.0 and 1.0
- Non-deterministic (different each render)
- Essential for drums, percussion, hi-hats, wind, FX
- Can be filtered to create various noise colors

---

## Design Decisions for Phonon Integration

### White Noise Use Cases

**Drums & Percussion**:
- Hi-hats (noise through high-pass filter)
- Snare (noise + sine for body)
- Claps (short bursts of noise)
- Cymbals (filtered noise)

**Sound Effects**:
- Wind (low-pass filtered noise)
- Ocean waves (band-pass filtered noise)
- Rain (filtered noise with envelope)
- Static/interference

**Synthesis**:
- Noise source for subtractive synthesis
- Breath component for wind instruments
- Texture layer in pads

### Phonon DSL Syntax

```phonon
-- Basic white noise
~noise: noise

-- Hi-hat (high-pass filtered noise)
~hihat: noise # hpf 8000 0.3

-- Snare (noise + sine)
~snare_body: sine 180
~snare_noise: noise # hpf 2000 0.5
~snare: (~snare_body + ~snare_noise) * 0.5

-- Wind sound (low-pass filtered noise)
~wind: noise # lpf 800 0.3

-- Burst of noise with envelope
~lfo: sine 2.0
~env: ~lfo * 0.4 + 0.6
~burst: noise * ~env

-- Output
out: ~noise * 0.3
```

### Naming

Use `noise` to match fundsp naming convention (simple and clear).

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
    TriangleHz,
    Noise,  // NEW
}
```

### 2. Constructor

```rust
pub fn new_noise(sample_rate: f64) -> Self {
    use fundsp::prelude::AudioUnit;

    let mut unit = fundsp::prelude::noise();
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |_input: f32| -> f32 {
        // noise: 0 inputs -> 1 output (generator)
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::Noise,
        params: vec![],  // No parameters!
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
// No update function needed - noise has no parameters!
```

### 4. Compiler Function

```rust
fn compile_noise(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!(
            "noise takes no parameters, got {}",
            args.len()
        ));
    }

    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_noise(ctx.graph.sample_rate() as f64);

    // Create constant node for "no input" (noise is a generator)
    let no_input = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Noise,
        input: Signal::Node(no_input),
        params: vec![],  // No parameters!
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. eval_node() Update

```rust
FundspUnitType::Noise => {
    // No parameters to update!
}
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_noise_basic() {
    // Test that fundsp noise generates audio
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    assert!(sum > 0.0, "Noise should produce output");
    println!("Noise - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_noise_range() {
    // Test that noise values are in range [-1, 1]
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut min_val = f32::INFINITY;
    let mut max_val = f32::NEG_INFINITY;

    for _ in 0..44100 {
        let frame = unit.tick(&Default::default());
        let sample = frame[0];

        min_val = min_val.min(sample);
        max_val = max_val.max(sample);
    }

    // Should be roughly in [-1, 1] range
    assert!(min_val >= -1.0 && min_val < 0.0, "Noise should have negative values");
    assert!(max_val > 0.0 && max_val <= 1.0, "Noise should have positive values");

    println!("Noise range: {:.3} to {:.3}", min_val, max_val);
}

#[test]
fn test_fundsp_noise_distribution() {
    // Test that noise has roughly equal positive/negative samples
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut positive_count = 0;
    let mut negative_count = 0;

    for _ in 0..44100 {
        let frame = unit.tick(&Default::default());
        let sample = frame[0];

        if sample > 0.0 {
            positive_count += 1;
        } else if sample < 0.0 {
            negative_count += 1;
        }
    }

    let total = positive_count + negative_count;
    let positive_ratio = positive_count as f32 / total as f32;

    // Should be roughly 50/50
    assert!(positive_ratio > 0.4 && positive_ratio < 0.6,
        "Noise should be roughly balanced: {:.2}% positive",
        positive_ratio * 100.0);

    println!("Noise distribution: {:.1}% positive, {:.1}% negative",
        positive_ratio * 100.0, (1.0 - positive_ratio) * 100.0);
}

#[test]
fn test_fundsp_noise_dc_centered() {
    // Test that average is close to 0 (DC-centered)
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100;

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // DC offset should be very small (noise is random)
    assert!(average.abs() < 0.1, "Noise should be roughly DC-centered");
    println!("Noise DC offset: {:.6}", average);
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_noise_level3_basic() {
    let code = "out: noise";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    assert!(rms > 0.1, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic noise - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_noise_level3_amplitude_control() {
    let code = "out: noise * 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Scaled noise should work");
    println!("Amplitude scaled (0.5x) - RMS: {:.4}", rms);
}

#[test]
fn test_noise_level3_hihat() {
    // Hi-hat: high-pass filtered noise
    let code = "out: noise # hpf 8000 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Hi-hat (filtered noise) should work");
    println!("Hi-hat (HPF noise) - RMS: {:.4}", rms);
}

#[test]
fn test_noise_level3_snare() {
    // Snare: noise + sine body
    let code = r#"
        ~snare_body: sine 180
        ~snare_noise: noise # hpf 2000 0.5
        out: (~snare_body + ~snare_noise) * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Snare (noise + sine) should work");
    println!("Snare (noise + sine) - RMS: {:.4}", rms);
}

#[test]
fn test_noise_level3_wind() {
    // Wind: low-pass filtered noise
    let code = "out: noise # lpf 800 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Wind (LPF noise) should work");
    println!("Wind (LPF noise) - RMS: {:.4}", rms);
}

#[test]
fn test_noise_level3_with_envelope() {
    // Noise burst with envelope
    let code = r#"
        tempo: 0.5
        ~lfo: sine 0.5
        ~env: ~lfo * 0.4 + 0.6
        out: noise * ~env * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Noise with envelope should work");
    println!("Noise with envelope - RMS: {:.4}", rms);
}

#[test]
fn test_noise_level3_vs_oscillators() {
    // Compare noise to oscillators (different spectrum)
    let code_noise = "out: noise * 0.3";
    let code_saw = "out: saw_hz 220";

    let audio_noise = render_dsl(code_noise, 1.0);
    let audio_saw = render_dsl(code_saw, 1.0);

    let rms_noise = calculate_rms(&audio_noise);
    let rms_saw = calculate_rms(&audio_saw);

    // Both should have energy
    assert!(rms_noise > 0.01);
    assert!(rms_saw > 0.01);

    println!("Noise RMS: {:.4}, Saw RMS: {:.4}", rms_noise, rms_saw);
}
```

---

## Expected Outcomes

After implementation:
- ✅ noise works from Phonon DSL
- ✅ Can be filtered for different noise colors
- ✅ Can be mixed with oscillators
- ✅ Can be modulated with patterns
- ✅ Essential for drums, percussion, FX

---

## Implementation Checklist

- [ ] Add Noise variant to FundspUnitType
- [ ] Implement new_noise constructor
- [ ] Update Clone implementation
- [ ] Add Noise case to eval_node()
- [ ] Add compile_noise function
- [ ] Register "noise" keyword
- [ ] Create test_fundsp_noise.rs (direct API)
- [ ] Create test_noise_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 20-25 minutes (no parameters = simpler!)

---

## Musical Use Cases

```phonon
-- Hi-hat
~hihat: noise # hpf 8000 0.3

-- Snare (noise + body)
~snare_body: sine 180
~snare_noise: noise # hpf 2000 0.5
~snare: (~snare_body + ~snare_noise) * 0.5

-- Clap (short noise bursts)
~clap: noise # hpf 1000 0.5

-- Wind
~wind: noise # lpf 800 0.3

-- Ocean waves (band-pass filtered)
~ocean: noise # bpf 400 0.5

-- Rain (noise with envelope)
tempo: 8.0
~lfo: sine 0.2
~env: ~lfo * 0.5 + 0.5
~rain: noise # lpf 2000 0.3 * ~env

-- Textured pad (noise + oscillators)
~pad: saw_hz 110
~texture: noise # lpf 500 0.2
~rich: (~pad * 0.8 + ~texture * 0.2)
```

---

## Technical Notes

### White Noise Spectrum
- Equal energy across all frequencies
- Flat power spectrum (no frequency preference)
- RMS should be roughly constant
- Can be filtered to create other noise colors:
  - Pink noise: 1/f spectrum (3dB/octave rolloff)
  - Brown noise: 1/f² spectrum (6dB/octave rolloff)
  - Blue noise: increasing with frequency
  - Violet noise: f² spectrum

### Usage in Drums
- **Hi-hat**: High-pass filtered noise (8kHz+)
- **Snare**: Noise (2kHz+) + sine body (150-200 Hz)
- **Clap**: Short burst of filtered noise
- **Cymbal**: Band-pass filtered noise

### Modulation
- Amplitude modulation creates rhythmic noise
- Filter modulation creates sweeping textures
- Mixing with oscillators creates rich timbres

---

**Study Complete**: Ready to implement! 🚀
