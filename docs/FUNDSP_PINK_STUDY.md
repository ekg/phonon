# fundsp pink Study Notes

**Date**: 2025-10-30
**UGen**: pink (Pink noise generator)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn pink<F: Float>() -> An<Pipe<Noise, Pinkpass<F>>>
```

### Parameters
- **None** - Generates random pink noise (1/f spectrum)
- Generic F: Float type (use f32 for Phonon)

### Input/Output
- **Inputs**: 0 (generator, no audio input)
- **Outputs**: 1 (mono pink noise)

### Characteristics
- Pink noise generator (1/f power spectrum)
- 3dB/octave rolloff (more bass than white noise)
- Equal energy per octave (vs white noise = equal energy per Hz)
- Random samples between approximately -1.0 and 1.0
- Non-deterministic (different each render)
- Essential for drums, percussion, natural sounds
- Sounds "warmer" and more "natural" than white noise

### Pink vs White Noise

**White Noise**:
- Equal energy across all frequencies (flat spectrum)
- Sounds harsh, hissy
- More high-frequency content

**Pink Noise**:
- 1/f spectrum (3dB/octave rolloff)
- Equal energy per octave
- Sounds warmer, more natural
- Better for realistic drums (snares, hi-hats)
- Found in nature (rain, waterfalls)

---

## Design Decisions for Phonon Integration

### Pink Noise Use Cases

**Drums & Percussion**:
- Snare drums (more natural than white noise)
- Realistic hi-hats
- Cymbals with body
- Toms (filtered pink noise)

**Natural Sounds**:
- Rain (pink noise + envelope)
- Waterfalls (filtered pink noise)
- Wind (low-pass filtered pink noise)
- Ocean waves

**Sound Design**:
- Warm texture layers
- Analog-style noise
- Breath component for instruments
- Background ambience

### Phonon DSL Syntax

```phonon
-- Basic pink noise
~pink: pink

-- Snare with pink noise (more natural)
~snare_body: sine 180
~snare_noise: pink # hpf 2000 0.5
~snare: (~snare_body + ~snare_noise) * 0.5

-- Realistic hi-hat
~hihat: pink # hpf 6000 0.3

-- Rain sound
~rain_lfo: sine 0.1
~rain_env: ~rain_lfo * 0.5 + 0.5
~rain: pink # lpf 2000 0.3 * ~rain_env

-- Waterfall
~waterfall: pink # bpf 800 0.4

-- Output
out: ~pink * 0.3
```

### Naming

Use `pink` to match fundsp naming convention (simple and clear).

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
    Noise,      // White noise
    Pink,       // NEW - Pink noise
}
```

### 2. Constructor

```rust
pub fn new_pink(sample_rate: f64) -> Self {
    use fundsp::prelude::AudioUnit;

    // pink::<f32>() requires type annotation
    let mut unit = fundsp::prelude::pink::<f32>();
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |_input: f32| -> f32 {
        // pink: 0 inputs -> 1 output (generator)
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::Pink,
        params: vec![],  // No parameters!
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
// No update function needed - pink noise has no parameters!
```

### 4. Compiler Function

```rust
fn compile_pink(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!(
            "pink takes no parameters, got {}",
            args.len()
        ));
    }

    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_pink(ctx.graph.sample_rate() as f64);

    // Create constant node for "no input" (pink is a generator)
    let no_input = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Pink,
        input: Signal::Node(no_input),
        params: vec![],  // No parameters!
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. eval_node() Update

```rust
FundspUnitType::Pink => {
    // No parameters to update!
}
```

### 6. Clone Implementation

```rust
FundspUnitType::Pink => Self::new_pink(self.sample_rate),
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_pink_basic() {
    // Test that fundsp pink generates audio
    let mut unit = pink::<f32>();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    assert!(sum > 0.0, "Pink noise should produce output: {}", sum);
    println!("Pink noise - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_pink_range() {
    // Test that pink noise values are in range [-1, 1]
    let mut unit = pink::<f32>();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut min_val = f32::INFINITY;
    let mut max_val = f32::NEG_INFINITY;

    for _ in 0..44100 {  // 1 second
        let frame = unit.tick(&Default::default());
        let sample = frame[0];

        min_val = min_val.min(sample);
        max_val = max_val.max(sample);
    }

    // Should be roughly in [-1, 1] range
    assert!(
        min_val >= -2.0 && min_val < 0.0,
        "Pink noise should have negative values: {}",
        min_val
    );
    assert!(
        max_val > 0.0 && max_val <= 2.0,
        "Pink noise should have positive values: {}",
        max_val
    );

    println!("Pink noise range: {:.3} to {:.3}", min_val, max_val);
}

#[test]
fn test_fundsp_pink_distribution() {
    // Test that pink noise has roughly equal positive/negative samples
    let mut unit = pink::<f32>();
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

    // Should be roughly 50/50 (allow 40-60% range for randomness)
    assert!(
        positive_ratio > 0.4 && positive_ratio < 0.6,
        "Pink noise should be roughly balanced: {:.2}% positive",
        positive_ratio * 100.0
    );

    println!(
        "Pink noise distribution: {:.1}% positive, {:.1}% negative",
        positive_ratio * 100.0,
        (1.0 - positive_ratio) * 100.0
    );
}

#[test]
fn test_fundsp_pink_dc_centered() {
    // Test that average is close to 0 (DC-centered)
    let mut unit = pink::<f32>();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100;  // 1 second

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // DC offset should be very small (noise is random, but should average to ~0)
    assert!(
        average.abs() < 0.1,
        "Pink noise should be roughly DC-centered, average: {}",
        average
    );

    println!("Pink noise DC offset: {:.6}", average);
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_pink_level3_basic() {
    let code = "out: pink";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    assert!(rms > 0.1, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic pink noise - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_pink_level3_amplitude_control() {
    let code = "out: pink * 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Amplitude-scaled pink noise should work");

    println!("Amplitude scaled (0.5x) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_snare() {
    // Snare: pink noise + sine body (more natural than white noise)
    let code = r#"
        ~snare_body: sine 180
        ~snare_noise: pink # hpf 2000 0.5
        out: (~snare_body + ~snare_noise) * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Snare (pink noise + sine) should work");

    println!("Snare (pink noise + sine) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_hihat() {
    // Hi-hat: high-pass filtered pink noise
    let code = "out: pink # hpf 5000 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Hi-hat (filtered pink noise) should work");

    println!("Hi-hat (HPF pink noise) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_rain() {
    // Rain: pink noise with envelope
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.1
        ~env: ~lfo * 0.5 + 0.5
        out: pink # lpf 2000 0.3 * ~env * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Rain (pink noise with envelope) should work");

    println!("Rain (pink noise) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_waterfall() {
    // Waterfall: band-pass filtered pink noise
    let code = "out: pink # bpf 800 0.4";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Waterfall (BPF pink noise) should work");

    println!("Waterfall (BPF pink noise) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_with_envelope() {
    // Pink noise burst with envelope
    let code = r#"
        tempo: 2.0
        ~lfo: sine 0.5
        ~env: ~lfo * 0.4 + 0.6
        out: pink * ~env * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pink noise with envelope should work");

    println!("Pink noise with envelope - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_vs_white_noise() {
    // Compare pink to white noise (different spectrum)
    let code_pink = "out: pink * 0.3";
    let code_white = "out: noise * 0.3";

    let audio_pink = render_dsl(code_pink, 1.0);
    let audio_white = render_dsl(code_white, 1.0);

    let rms_pink = calculate_rms(&audio_pink);
    let rms_white = calculate_rms(&audio_white);

    // Both should have energy
    assert!(rms_pink > 0.01);
    assert!(rms_white > 0.01);

    println!("Pink RMS: {:.4}, White RMS: {:.4}", rms_pink, rms_white);
}

#[test]
fn test_pink_level3_textured_pad() {
    // Textured pad (oscillator + filtered pink noise)
    let code = r#"
        ~pad: saw_hz 110
        ~texture: pink # lpf 500 0.2
        out: (~pad * 0.8 + ~texture * 0.2) * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Textured pad should work");

    println!("Textured pad (osc + pink) - RMS: {:.4}", rms);
}

#[test]
fn test_pink_level3_rhythmic() {
    // Rhythmic pink noise (amplitude modulation)
    let code = r#"
        tempo: 4.0
        ~lfo: sine 1.0
        ~env: ~lfo * 0.5 + 0.5
        out: pink * ~env * 0.3
    "#;
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Rhythmic pink noise should work");

    println!("Rhythmic pink noise - RMS: {:.4}", rms);
}
```

---

## Expected Outcomes

After implementation:
- ✅ pink works from Phonon DSL
- ✅ Warmer, more natural sound than white noise
- ✅ Better for realistic drums and percussion
- ✅ Can be filtered for different textures
- ✅ Can be mixed with oscillators
- ✅ Can be modulated with patterns

---

## Implementation Checklist

- [ ] Add Pink variant to FundspUnitType
- [ ] Implement new_pink constructor
- [ ] Update Clone implementation
- [ ] Add Pink case to eval_node()
- [ ] Add compile_pink function
- [ ] Register "pink" keyword (zero-argument function)
- [ ] Create test_fundsp_pink.rs (direct API)
- [ ] Create test_pink_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 15-20 minutes (parameterless, same as noise!)

---

## Musical Use Cases

```phonon
-- Natural snare
~snare_body: sine 180
~snare_noise: pink # hpf 2000 0.5
~snare: (~snare_body + ~snare_noise) * 0.5

-- Realistic hi-hat
~hihat: pink # hpf 6000 0.3

-- Rain ambience
~rain_lfo: sine 0.1
~rain_env: ~rain_lfo * 0.5 + 0.5
~rain: pink # lpf 2000 0.3 * ~rain_env

-- Waterfall
~waterfall: pink # bpf 800 0.4

-- Warm pad texture
~pad: saw_hz 110
~texture: pink # lpf 500 0.2
~warm_pad: (~pad * 0.8 + ~texture * 0.2)

-- Output
out: ~snare * 0.4 + ~hihat * 0.3 + ~rain * 0.2
```

---

## Technical Notes

### Pink Noise Spectrum

- **Power spectrum**: 1/f (3dB/octave rolloff)
- **Equal energy per octave** (vs white noise = equal energy per Hz)
- Found in nature (rain, waterfalls, wind)
- Perceptually "warmer" and more "natural" than white noise
- Better for drums (more low-frequency content)

### Comparison to White Noise

| Property | White Noise | Pink Noise |
|----------|-------------|------------|
| Spectrum | Flat (equal energy/Hz) | 1/f (-3dB/oct) |
| Sound | Harsh, hissy | Warm, natural |
| Use case | Hi-hats, static | Snares, natural sounds |
| Frequency balance | Too bright | Balanced |

### Usage in Drums

- **Snare**: Pink noise sounds more natural than white noise
- **Hi-hat**: Pink noise + HPF creates realistic metallic sound
- **Cymbals**: Pink noise + BPF for body and shimmer
- **Toms**: Pink noise + LPF for punch

### Modulation

- Amplitude modulation creates rhythmic textures
- Filter modulation creates evolving ambiences
- Mixing with oscillators creates rich, warm timbres

---

**Study Complete**: Ready to implement! 🚀
