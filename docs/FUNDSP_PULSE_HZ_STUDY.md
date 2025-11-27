# fundsp pulse Study Notes

**Date**: 2025-10-30
**UGen**: pulse (Pulse wave oscillator with variable pulse width)
**Status**: DEFERRED - Requires multi-input architecture

## Why Deferred

fundsp's `pulse()` function takes **2 audio inputs** (frequency and pulse width), not constructor parameters like `saw_hz()`. This requires architectural changes to support multi-input audio processing, which is beyond the scope of simple UGen wrapping.

Current FundspState architecture supports:
- ✅ Generators with parameters: `saw_hz(freq)` - tick with no input
- ✅ Processors with audio input: `moog_hz(cutoff, resonance)` - tick with 1 audio input
- ❌ Generators with audio inputs: `pulse()` - tick with 2 audio inputs (NOT YET SUPPORTED)

**Decision**: Skip `pulse` for now, revisit after implementing multi-input support.

**Alternative**: Use `square_hz` for 50% duty cycle pulse waves

---

## fundsp API Research

### Function Signature
```rust
pub fn pulse_hz(frequency: f32, pulse_width: f32) -> impl AudioUnit
```

### Parameters
- **frequency**: Frequency in Hz (e.g., 110.0, 220.0, 440.0)
- **pulse_width**: Duty cycle from 0.0 to 1.0 (0.5 = square wave)

### Input/Output
- **Inputs**: 0 (generator, no audio input)
- **Outputs**: 1 (mono pulse wave)

### Characteristics
- Pulse wave with variable duty cycle (pulse width)
- pulse_width = 0.5 → square wave (odd harmonics only)
- pulse_width ≠ 0.5 → both odd and even harmonics
- Bandlimited (anti-aliased) to prevent digital artifacts
- Essential for analog synth sounds, bass, leads, pads
- **PWM (Pulse Width Modulation)**: Modulating pulse width creates rich, evolving timbres

---

## Design Decisions for Phonon Integration

### Pulse Width Modulation (PWM)

**What is PWM?**
- Continuously varying the pulse width parameter
- Creates chorusing, phasing, evolving timbres
- Classic analog synth technique

**Phonon's Killer Feature**:
```phonon
-- PWM using LFO (IMPOSSIBLE in Tidal!)
~lfo: sine 0.3
~width: ~lfo * 0.4 + 0.5  -- Varies from 0.1 to 0.9
out: pulse_hz 110 ~width
```

### Pulse Width Values

- **0.5**: Square wave (50% duty cycle)
- **0.25**: Narrow pulse (25% duty cycle) - brighter, more harmonics
- **0.75**: Wide pulse (75% duty cycle) - equivalent to 0.25 inverted
- **0.1**: Very narrow - thin, buzzy sound
- **0.9**: Very wide - also thin, buzzy sound
- **Modulating width**: Creates evolving, rich timbres

### Phonon DSL Syntax

```phonon
-- Basic pulse wave
~pulse: pulse_hz 110 0.5  -- Square wave

-- Narrow pulse (bright)
~narrow: pulse_hz 220 0.25

-- PWM with LFO (classic analog synth)
~lfo: sine 0.5
~width: ~lfo * 0.3 + 0.5  -- Varies 0.2 to 0.8
~pwm: pulse_hz 110 ~width

-- Pattern-controlled frequency
~melody: pulse_hz "110 165 220 330" 0.3

-- Pattern-controlled width
~varying: pulse_hz 110 "0.2 0.5 0.8 0.2"

-- Detuned PWM pad
~pulse1: pulse_hz 110 0.3
~pulse2: pulse_hz 110.5 0.7
~pulse3: pulse_hz 220 0.4
~pad: (~pulse1 + ~pulse2 + ~pulse3) * 0.3

-- Output
out: ~pwm * 0.5
```

### Naming

Use `pulse_hz` to match fundsp naming convention (explicit frequency parameter).

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
    Noise,
    PulseHz,  // NEW
}
```

### 2. Constructor

```rust
pub fn new_pulse_hz(frequency: f32, pulse_width: f32, sample_rate: f64) -> Self {
    use fundsp::prelude::AudioUnit;

    let mut unit = fundsp::prelude::pulse_hz(frequency, pulse_width);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |_input: f32| -> f32 {
        // pulse_hz: 0 inputs -> 1 output (generator)
        let output_frame = unit.tick(&Default::default());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::PulseHz,
        params: vec![frequency, pulse_width],
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
pub fn update_pulse_params(&mut self, new_freq: f32, new_width: f32, sample_rate: f64) {
    // Recreate if either parameter changed significantly
    let freq_changed = (self.params[0] - new_freq).abs() > 0.1;
    let width_changed = (self.params[1] - new_width).abs() > 0.01;

    if freq_changed || width_changed {
        *self = Self::new_pulse_hz(new_freq, new_width, sample_rate);
    }
}
```

### 4. Compiler Function

```rust
fn compile_pulse_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "pulse_hz requires 2 parameters (frequency, pulse_width), got {}",
            args.len()
        ));
    }

    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let freq_node = compile_expr(ctx, args[0].clone())?;
    let width_node = compile_expr(ctx, args[1].clone())?;

    let state = FundspState::new_pulse_hz(440.0, 0.5, ctx.graph.sample_rate() as f64);

    // Create constant node for "no input" (pulse_hz is a generator)
    let no_input = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::PulseHz,
        input: Signal::Node(no_input),
        params: vec![Signal::Node(freq_node), Signal::Node(width_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

### 5. eval_node() Update

```rust
FundspUnitType::PulseHz => {
    if param_values.len() >= 2 {
        let frequency = param_values[0];
        let pulse_width = param_values[1].clamp(0.0, 1.0);  // Ensure 0-1 range
        state_guard.update_pulse_params(frequency, pulse_width, self.sample_rate as f64);
    }
}
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_pulse_basic() {
    // Test that fundsp pulse generates audio
    let mut unit = pulse_hz(220.0, 0.5);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    assert!(sum > 0.0, "Pulse should produce output: {}", sum);
    println!("Pulse 220 Hz @ 50% - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_pulse_width_sweep() {
    // Test different pulse widths
    let widths = vec![0.1, 0.25, 0.5, 0.75, 0.9];

    for width in &widths {
        let mut unit = pulse_hz(220.0, *width);
        unit.reset();
        unit.set_sample_rate(44100.0);

        let mut sum = 0.0;
        for _ in 0..4410 {
            let frame = unit.tick(&Default::default());
            sum += frame[0].abs();
        }

        assert!(sum > 0.0, "Pulse width {} should produce output", width);
        println!("Pulse width {:.2} - sum: {:.2}", width, sum);
    }
}

#[test]
fn test_fundsp_pulse_square_equivalence() {
    // Pulse at 50% width should equal square wave
    let mut pulse = pulse_hz(220.0, 0.5);
    let mut square = square_hz(220.0);

    pulse.reset();
    square.reset();
    pulse.set_sample_rate(44100.0);
    square.set_sample_rate(44100.0);

    // Compare a few samples (should be very close)
    for _ in 0..100 {
        let pulse_frame = pulse.tick(&Default::default());
        let square_frame = square.tick(&Default::default());

        let diff = (pulse_frame[0] - square_frame[0]).abs();
        assert!(diff < 0.01, "Pulse@50% should match square wave");
    }

    println!("Pulse@50% matches square wave ✓");
}

#[test]
fn test_fundsp_pulse_dc_centered() {
    // Test that average is close to 0 (DC-centered)
    let mut unit = pulse_hz(220.0, 0.5);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100;  // 1 second

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // DC offset should be very small
    assert!(
        average.abs() < 0.05,
        "Pulse should be roughly DC-centered, average: {}",
        average
    );

    println!("Pulse DC offset: {:.6}", average);
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_pulse_hz_level3_basic() {
    let code = "out: pulse_hz 220 0.5";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic pulse_hz 220 Hz @ 50% - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_pulse_hz_level3_width_sweep() {
    // Test different pulse widths
    let widths = vec![0.1, 0.25, 0.5, 0.75, 0.9];

    for width in &widths {
        let code = format!("out: pulse_hz 220 {}", width);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Width {} should produce output", width);
        println!("Pulse width {:.2}: RMS {:.4}", width, rms);
    }
}

#[test]
fn test_pulse_hz_level3_pwm() {
    // Test PWM (pulse width modulation) with LFO
    let code = r#"
        tempo: 0.5
        ~lfo: sine 0.5
        ~width: ~lfo * 0.3 + 0.5
        out: pulse_hz 110 ~width
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "PWM should work: {}", rms);

    println!("PWM (LFO modulated width) - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_pattern_frequency() {
    // Test pattern-controlled frequency
    let code = r#"
        tempo: 0.5
        out: pulse_hz "110 165 220 330" 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled pulse should work: {}", rms);

    println!("Pattern control (frequency) - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_pattern_width() {
    // Test pattern-controlled pulse width
    let code = r#"
        tempo: 0.5
        out: pulse_hz 220 "0.2 0.5 0.8 0.2"
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pattern-controlled width should work: {}", rms);

    println!("Pattern control (width) - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_narrow_pulse() {
    // Narrow pulse (bright sound)
    let code = "out: pulse_hz 110 0.1";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Narrow pulse should work");

    println!("Narrow pulse (10%) - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_bass() {
    // Pulse bass with moderate width
    let code = "out: pulse_hz 55 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pulse bass should work");

    println!("Pulse bass (55 Hz @ 30%) - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_detuned_pad() {
    // Detuned pulse pad
    let code = r#"
        ~pulse1: pulse_hz 110 0.3
        ~pulse2: pulse_hz 110.5 0.7
        ~pulse3: pulse_hz 220 0.4
        out: (~pulse1 + ~pulse2 + ~pulse3) * 0.3
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Detuned pulse pad should work");

    println!("Detuned pulse pad - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_with_envelope() {
    // Pulse with amplitude envelope
    let code = r#"
        tempo: 0.5
        ~lfo: sine 0.5
        ~env: ~lfo * 0.4 + 0.6
        out: pulse_hz 220 0.3 * ~env
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Pulse with envelope should work");

    println!("Pulse with envelope - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_vs_square() {
    // Compare pulse@50% to square (should be similar)
    let code_pulse = "out: pulse_hz 220 0.5";
    let code_square = "out: square_hz 220";

    let audio_pulse = render_dsl(code_pulse, 1.0);
    let audio_square = render_dsl(code_square, 1.0);

    let rms_pulse = calculate_rms(&audio_pulse);
    let rms_square = calculate_rms(&audio_square);

    // Should have similar energy
    let ratio = rms_pulse / rms_square;
    assert!(ratio > 0.9 && ratio < 1.1, "Pulse@50% should match square");

    println!("Pulse@50% RMS: {:.4}, Square RMS: {:.4}, Ratio: {:.2}",
        rms_pulse, rms_square, ratio);
}

#[test]
fn test_pulse_hz_level3_with_filter() {
    // Pulse through filter
    let code = "out: pulse_hz 220 0.25 # lpf 1500 0.5";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Filtered pulse should work");

    println!("Pulse through LPF - RMS: {:.4}", rms);
}

#[test]
fn test_pulse_hz_level3_dual_pwm() {
    // Two PWM oscillators with different rates
    let code = r#"
        tempo: 0.5
        ~lfo1: sine 0.3
        ~lfo2: sine 0.7
        ~width1: ~lfo1 * 0.2 + 0.5
        ~width2: ~lfo2 * 0.3 + 0.4
        ~pwm1: pulse_hz 110 ~width1
        ~pwm2: pulse_hz 220 ~width2
        out: (~pwm1 + ~pwm2) * 0.4
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Dual PWM should work");

    println!("Dual PWM - RMS: {:.4}", rms);
}
```

---

## Expected Outcomes

After implementation:
- ✅ pulse_hz works from Phonon DSL
- ✅ Variable pulse width (0.0 to 1.0)
- ✅ PWM (pulse width modulation) enabled
- ✅ Pattern-controllable frequency and width
- ✅ pulse_hz 220 0.5 ≈ square_hz 220
- ✅ Essential for analog synth sounds, bass, leads, pads

---

## Implementation Checklist

- [ ] Add PulseHz variant to FundspUnitType
- [ ] Implement new_pulse_hz constructor
- [ ] Implement update_pulse_params method
- [ ] Update Clone implementation
- [ ] Add PulseHz case to eval_node()
- [ ] Add compile_pulse_hz function
- [ ] Register "pulse_hz" keyword
- [ ] Create test_fundsp_pulse.rs (direct API)
- [ ] Create test_pulse_hz_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 20-25 minutes

---

## Musical Use Cases

```phonon
-- Classic PWM bass
~lfo: sine 0.5
~width: ~lfo * 0.3 + 0.4
~bass: pulse_hz 55 ~width # lpf 800 0.8

-- Detuned PWM pad (classic analog)
~pw1: sine 0.3
~pw2: sine 0.7
~width1: ~pw1 * 0.2 + 0.5
~width2: ~pw2 * 0.25 + 0.45
~pulse1: pulse_hz 110 ~width1
~pulse2: pulse_hz 110.5 ~width2
~pulse3: pulse_hz 220 ~width1
~pad: (~pulse1 + ~pulse2 + ~pulse3) * 0.3 # lpf 2000 0.5

-- Narrow pulse lead (bright)
~lead: pulse_hz 440 0.15 # lpf 3000 0.7

-- Square bass (50% duty cycle)
~square_bass: pulse_hz 55 0.5

-- Output
out: ~bass * 0.4 + ~pad * 0.3 + ~lead * 0.2
```

---

## Technical Notes

### Pulse Width and Harmonics

- **0.5 (square)**: Odd harmonics only (1/n falloff)
- **Other widths**: Both odd and even harmonics
- **Narrower pulse** (0.1-0.3): More harmonics, brighter
- **Wider pulse** (0.7-0.9): Also more harmonics (symmetrical)
- **50% (square)**: Minimum harmonic content for pulse

### PWM (Pulse Width Modulation)

**What happens**:
- Sweeping pulse width creates chorusing/phasing effect
- Continuously changing harmonic content
- Rich, evolving timbre

**Classic analog synth technique**:
- Moog, ARP, Sequential used PWM extensively
- Creates movement without using filters
- Essential for warm, evolving pads

**Phonon implementation**:
```phonon
~lfo: sine 0.5
~width: ~lfo * 0.3 + 0.5  -- Modulates from 0.2 to 0.8
out: pulse_hz 110 ~width
```

### Comparison to Square Wave

- `square_hz 220` = `pulse_hz 220 0.5` (identical)
- Square is a special case of pulse (50% duty cycle)
- Pulse offers continuous variation of harmonic content

---

**Study Complete**: Ready to implement! 🚀
