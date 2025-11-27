# fundsp chorus Study Notes

**Date**: 2025-10-30
**UGen**: chorus (Mono chorus effect, 5 voices)
**Status**: STUDYING

---

## fundsp API Research

### Function Signature
```rust
pub fn chorus(
    seed: u64,
    separation: f32,
    variation: f32,
    mod_frequency: f32
) -> impl AudioUnit
```

### Parameters
- **seed**: LFO seed (u64) - Different seeds create different LFO patterns
- **separation**: Base voice separation in seconds (typical: 0.015)
- **variation**: Delay variation in seconds (typical: 0.005)
- **mod_frequency**: Delay modulation frequency in Hz (typical: 0.2)

### Input/Output
- **Inputs**: 1 (mono audio signal to process)
- **Outputs**: 1 (mono chorus effect)

### Characteristics
- Mono chorus effect with 5 voices
- Creates thick, detuned sound by mixing delayed copies
- LFOs modulate delay times for movement
- For stereo: stack two with different seeds

### Example Usage (from docs)
```rust
// Add 20% chorus to mono signal
pass() & 0.2 * chorus(0, 0.0, 0.01, 0.3)
```

---

## Design Decisions for Phonon Integration

### Parameter Mapping

fundsp has 4 parameters, but seed is typically static. Let's expose the 3 musical parameters:

1. **separation** - Controls voice spread (0.005 - 0.030 seconds typical)
2. **variation** - Controls depth of modulation (0.001 - 0.010 seconds typical)
3. **mod_frequency** - Controls LFO speed (0.1 - 2.0 Hz typical)

**Seed handling**: Use a fixed seed (0) for now, or make it configurable per instance

### Phonon DSL Syntax

**Note**: Renamed to `fchorus` to avoid conflict with existing custom chorus implementation.

```phonon
-- Basic chorus with static parameters
~chorused: saw 110 # fchorus 0.015 0.005 0.3

-- Pattern modulation (Phonon's killer feature!)
~lfo: sine 0.1
~mod_freq: ~lfo * 0.5 + 0.5  -- Modulate chorus speed
out: saw 110 # fchorus 0.02 0.005 ~mod_freq
```

### Type Considerations

- **seed**: u64 (not f32!) - Must convert from Phonon's f32 parameters
  - Solution: Cast to u64: `seed_param as u64`
- **separation, variation, mod_frequency**: f32 - Direct pass-through

---

## Implementation Plan

### 1. FundspState Variant

```rust
pub enum FundspUnitType {
    OrganHz,
    MoogHz,
    ReverbStereo,
    Chorus,  // NEW
}
```

### 2. Constructor

```rust
pub fn new_chorus(
    seed: u64,
    separation: f32,
    variation: f32,
    mod_frequency: f32,
    sample_rate: f64
) -> Self {
    let mut unit = fundsp::prelude::chorus(seed, separation, variation, mod_frequency);
    unit.reset();
    unit.set_sample_rate(sample_rate);

    let tick_fn = Box::new(move |input: f32| -> f32 {
        // chorus: 1 input -> 1 output
        let output_frame = unit.tick(&[input].into());
        output_frame[0]
    });

    Self {
        tick_fn,
        unit_type: FundspUnitType::Chorus,
        params: vec![seed as f32, separation, variation, mod_frequency],
        sample_rate,
    }
}
```

### 3. Update Parameters

```rust
pub fn update_chorus_params(
    &mut self,
    new_seed: u64,
    new_separation: f32,
    new_variation: f32,
    new_mod_frequency: f32,
    sample_rate: f64
) {
    let seed_changed = (self.params[0] as u64) != new_seed;
    let separation_changed = (self.params[1] - new_separation).abs() > 0.001;
    let variation_changed = (self.params[2] - new_variation).abs() > 0.0001;
    let mod_freq_changed = (self.params[3] - new_mod_frequency).abs() > 0.01;

    if seed_changed || separation_changed || variation_changed || mod_freq_changed {
        *self = Self::new_chorus(new_seed, new_separation, new_variation, new_mod_frequency, sample_rate);
    }
}
```

### 4. Compiler Function

**Option A: Expose all 4 parameters (including seed)**
```phonon
out: saw 110 # chorus 0 0.015 0.005 0.3
--                     ^ seed (0-255 typical)
```

**Option B: Fix seed, expose 3 musical parameters (RECOMMENDED)**
```phonon
out: saw 110 # chorus 0.015 0.005 0.3
--                    separation variation mod_freq
```

Let's go with **Option B** for simpler syntax:

```rust
fn compile_chorus(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 3 {
        return Err(format!(
            "chorus requires 3 parameters (separation, variation, mod_frequency), got {}",
            params.len()
        ));
    }

    let separation_node = compile_expr(ctx, params[0].clone())?;
    let variation_node = compile_expr(ctx, params[1].clone())?;
    let mod_freq_node = compile_expr(ctx, params[2].clone())?;

    // Create fundsp chorus with fixed seed=0
    let state = FundspState::new_chorus(0, 0.015, 0.005, 0.3, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Chorus,
        input: input_signal,
        params: vec![
            Signal::Constant(0.0),  // Fixed seed
            Signal::Node(separation_node),
            Signal::Node(variation_node),
            Signal::Node(mod_freq_node)
        ],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}
```

**Note**: We include seed as `Signal::Constant(0.0)` to match the 4-parameter structure in update_chorus_params.

### 5. eval_node() Update

```rust
FundspUnitType::Chorus => {
    if param_values.len() >= 4 {
        let seed = param_values[0] as u64;  // Convert f32 to u64
        let separation = param_values[1];
        let variation = param_values[2];
        let mod_frequency = param_values[3];
        state_guard.update_chorus_params(seed, separation, variation, mod_frequency, self.sample_rate as f64);
    }
}
```

---

## Test Plan

### Level 1: Direct fundsp API Tests

```rust
#[test]
fn test_fundsp_chorus_basic() {
    // Test that fundsp chorus processes audio
    let mut unit = chorus(0, 0.015, 0.005, 0.3);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Send continuous signal
    let mut sum = 0.0;
    for _ in 0..4410 {  // 0.1 seconds
        let frame = unit.tick(&[1.0].into());
        sum += frame[0].abs();
    }

    // Should have output
    assert!(sum > 0.0, "Chorus should produce output");
    println!("Chorus sum: {:.2}", sum);
}

#[test]
fn test_fundsp_chorus_separation() {
    // Test that separation parameter affects output
    let mut unit_narrow = chorus(0, 0.005, 0.002, 0.3);  // Narrow spread
    let mut unit_wide = chorus(0, 0.030, 0.002, 0.3);    // Wide spread

    unit_narrow.reset();
    unit_narrow.set_sample_rate(44100.0);
    unit_wide.reset();
    unit_wide.set_sample_rate(44100.0);

    let mut narrow_sum = 0.0;
    let mut wide_sum = 0.0;

    for _ in 0..4410 {
        let narrow_frame = unit_narrow.tick(&[1.0].into());
        let wide_frame = unit_wide.tick(&[1.0].into());

        narrow_sum += narrow_frame[0].abs();
        wide_sum += wide_frame[0].abs();
    }

    // Both should produce output
    assert!(narrow_sum > 0.0);
    assert!(wide_sum > 0.0);

    println!("Narrow: {:.2}, Wide: {:.2}", narrow_sum, wide_sum);
}

#[test]
fn test_fundsp_chorus_mod_frequency() {
    // Test that mod frequency affects modulation speed
    let mut unit_slow = chorus(0, 0.015, 0.005, 0.1);  // Slow LFO
    let mut unit_fast = chorus(0, 0.015, 0.005, 2.0);  // Fast LFO

    unit_slow.reset();
    unit_slow.set_sample_rate(44100.0);
    unit_fast.reset();
    unit_fast.set_sample_rate(44100.0);

    let mut slow_sum = 0.0;
    let mut fast_sum = 0.0;

    for _ in 0..44100 {  // 1 second
        let slow_frame = unit_slow.tick(&[1.0].into());
        let fast_frame = unit_fast.tick(&[1.0].into());

        slow_sum += slow_frame[0].abs();
        fast_sum += fast_frame[0].abs();
    }

    // Both should produce output
    assert!(slow_sum > 0.0);
    assert!(fast_sum > 0.0);

    println!("Slow LFO: {:.2}, Fast LFO: {:.2}", slow_sum, fast_sum);
}
```

### Level 3: Phonon Integration Tests

```rust
#[test]
fn test_chorus_level3_basic() {
    // Test basic chorus application
    let code = "out: saw 220 # chorus 0.015 0.005 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic chorus - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_chorus_level3_separation_sweep() {
    // Test different separation values
    let separations = vec![0.005, 0.010, 0.015, 0.020, 0.030];

    for sep in &separations {
        let code = format!("out: saw 220 # chorus {} 0.005 0.3", sep);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Separation {} should produce output", sep);
        println!("Separation {}: RMS {:.4}", sep, rms);
    }
}

#[test]
fn test_chorus_level3_pattern_modulation() {
    // Test Phonon's killer feature: pattern modulation at audio rate!
    let code = "
        tempo: 0.5
        ~lfo: sine 0.5
        ~mod_freq: ~lfo * 0.8 + 0.4
        out: saw 110 # chorus 0.015 0.005 ~mod_freq
    ";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    // Modulated signal should have energy
    assert!(rms > 0.01, "Pattern modulated chorus should work: {}", rms);

    println!("Pattern modulation - RMS: {:.4}", rms);
}

#[test]
fn test_chorus_level3_vs_dry() {
    // Compare chorus to dry signal
    let code_chorus = "out: saw 220 # chorus 0.020 0.007 0.4";
    let code_dry = "out: saw 220";

    let audio_chorus = render_dsl(code_chorus, 2.0);
    let audio_dry = render_dsl(code_dry, 2.0);

    let rms_chorus = calculate_rms(&audio_chorus);
    let rms_dry = calculate_rms(&audio_dry);

    // Both should have energy
    assert!(rms_chorus > 0.01, "Chorus should have energy");
    assert!(rms_dry > 0.01, "Dry should have energy");

    println!("Chorus RMS: {:.4}, Dry RMS: {:.4}", rms_chorus, rms_dry);
}

#[test]
fn test_chorus_level3_on_drums() {
    // Test chorus on percussive sample
    let code = "out: s \"bd sn\" # chorus 0.012 0.004 0.25";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Chorus on drums should work");

    println!("Drums with chorus - RMS: {:.4}", rms);
}
```

---

## Expected Outcomes

After implementation:
- ✅ chorus works from Phonon DSL
- ✅ Pattern modulation of chorus parameters at audio rate
- ✅ Classic chorus effect (thickening, detuning)
- ✅ Works on both synthesis and samples

---

## Implementation Checklist

- [ ] Add Chorus variant to FundspUnitType
- [ ] Implement new_chorus constructor (handle u64 seed)
- [ ] Update tick() to handle Chorus (returns mono)
- [ ] Add update_chorus_params method
- [ ] Update Clone implementation
- [ ] Update Debug implementation
- [ ] Add Chorus case to eval_node() (cast seed to u64)
- [ ] Add compile_chorus function (3 params + fixed seed)
- [ ] Register "chorus" keyword
- [ ] Create test_fundsp_chorus.rs (direct API)
- [ ] Create test_chorus_integration.rs (Phonon DSL)
- [ ] Run all tests
- [ ] Commit

**Estimated time**: 45 minutes

---

## Notes on Seed Parameter

The seed parameter creates different LFO patterns:
- seed=0: Standard pattern
- seed=1,2,3...: Different random LFO variations

For stereo chorus, use two chorus units with different seeds:
```phonon
-- Pseudo-stereo (when Phonon supports stereo)
~left: saw 110 # chorus_with_seed 0 0.015 0.005 0.3
~right: saw 110 # chorus_with_seed 1 0.015 0.005 0.3
```

For now, we fix seed=0 for simplicity.

---

**Study Complete**: Ready to implement! 🚀
