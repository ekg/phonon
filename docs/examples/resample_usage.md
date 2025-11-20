# ResampleNode Usage Examples

## Overview

**ResampleNode** provides high-quality sample rate conversion using linear interpolation with fractional delay. Unlike PitchShifterNode which uses dual delay lines with crossfading, ResampleNode implements a simpler, more direct approach suitable for:

- **Pitch shifting** - Change pitch without time stretching
- **Speed changes** - Slow down or speed up audio playback
- **Time stretching preparation** - Foundation for more complex time manipulation
- **Sample rate matching** - Convert between different sample rates

## Algorithm

The ResampleNode uses a straightforward resampling approach:

```rust
// Read with linear interpolation
let read_pos = phase;  // Fractional position
let idx = floor(read_pos);
let frac = fractional_part(read_pos);

let sample = buffer[idx] * (1.0 - frac) + buffer[idx + 1] * frac;

// Advance by ratio
phase += ratio;
if phase >= buffer_length {
    phase -= buffer_length;  // Wrap around
}
```

## Parameters

- **input**: NodeId - Audio signal to resample
- **ratio**: NodeId - Resampling ratio
  - `0.5` = half speed (pitch down one octave)
  - `1.0` = unity (no change, passthrough)
  - `2.0` = double speed (pitch up one octave)
  - `0.1` to `4.0` typical range

## Programmatic Usage Examples

### Example 1: Basic Pitch Shifting (Octave Down)

```rust
use phonon::nodes::{ConstantNode, OscillatorNode, Waveform, ResampleNode};
use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::pattern::Fraction;

let sample_rate = 44100.0;
let block_size = 512;

// Create a 440 Hz sine wave
let mut freq_node = ConstantNode::new(440.0);       // NodeId 0
let mut osc = OscillatorNode::new(0, Waveform::Sine); // NodeId 1

// Resample at half speed (220 Hz effective pitch)
let mut ratio_node = ConstantNode::new(0.5);        // NodeId 2
let mut resampler = ResampleNode::new(1, 2, sample_rate); // NodeId 3

let context = ProcessContext::new(
    Fraction::from_float(0.0),
    0,
    block_size,
    2.0,
    sample_rate,
);

// Process audio
let mut freq_buf = vec![440.0; block_size];
let mut input_buf = vec![0.0; block_size];
let mut ratio_buf = vec![0.5; block_size];
let mut output = vec![0.0; block_size];

freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];
resampler.process_block(&inputs, &mut output, sample_rate, &context);

// output now contains audio pitched down one octave
```

### Example 2: Speed Up (Octave Up)

```rust
// Same setup as above, but with ratio = 2.0
let mut ratio_node = ConstantNode::new(2.0);  // Double speed = octave up

// Process as before...
// Result: 880 Hz effective pitch (octave up from 440 Hz)
```

### Example 3: Pattern-Modulated Resampling

```rust
// Create a varying ratio using an LFO
let mut lfo = OscillatorNode::new(0, Waveform::Sine); // NodeId 0
let mut lfo_freq = ConstantNode::new(0.5);            // 0.5 Hz LFO

// Scale LFO output: sine wave (-1 to 1) -> ratio (0.5 to 1.5)
// ratio = 1.0 + (lfo * 0.5)
// This requires additional nodes for arithmetic...

// Result: Pitch wobbles between half-speed and 1.5x speed
```

### Example 4: Tape Speed Effect

```rust
// Simulate tape slow-down effect
let mut ratio_node = ConstantNode::new(1.0);
let mut resampler = ResampleNode::new(1, 2, sample_rate);

// Over time, gradually reduce ratio from 1.0 to 0.1
for frame in 0..1000 {
    let ratio = 1.0 - (frame as f32 / 1000.0) * 0.9; // 1.0 -> 0.1
    // Update ratio_buf with new values
    // Process block...
}

// Result: Audio gradually slows down (like stopping a tape)
```

## Technical Details

### Buffer Management

- **Buffer Size**: Fixed 100ms (4410 samples @ 44.1kHz)
- **Memory**: VecDeque for efficient circular buffering
- **Latency**: Minimal (~100ms maximum due to buffer)

### Linear Interpolation

The node uses **linear interpolation** for fractional sample reads:

```
sample = sample1 + frac * (sample2 - sample1)
```

This provides:
- **Good quality** for most audio content
- **Low CPU cost** (one multiply, one add per sample)
- **Acceptable aliasing** for typical pitch shifts (±1 octave)

For higher quality (less aliasing), consider:
- Using a larger window with sinc interpolation
- Applying anti-aliasing filters before/after
- Using PitchShifterNode for more complex crossfading

### Phase Wrapping

Phase advances by `ratio` per sample and wraps at buffer length:

```rust
phase += ratio;
if phase >= buffer_length {
    phase = phase % buffer_length;
}
```

This ensures continuous playback without discontinuities.

### Ratio Clamping

Ratio is clamped to minimum 0.01 to prevent:
- Division by zero
- Extremely slow playback (which could cause buffer underruns)
- Negative ratios (reverse playback not supported)

## Comparison with PitchShifterNode

| Feature | ResampleNode | PitchShifterNode |
|---------|--------------|------------------|
| Algorithm | Single buffer + linear interpolation | Dual delay lines + crossfading |
| Complexity | Simpler | More complex |
| CPU Cost | Lower | Higher |
| Quality | Good | Better (smoother) |
| Artifacts | Some aliasing at extreme ratios | Fewer artifacts |
| Use Case | Speed changes, simple pitch shifts | Professional pitch shifting |
| Window Size | Fixed 100ms buffer | Adjustable window (10-100ms) |

## Integration Status

**Status**: ✅ Implemented, ⚠️ Not yet integrated into DSL compiler

The ResampleNode is fully implemented with:
- ✅ Complete implementation in `src/nodes/resample.rs`
- ✅ 14 comprehensive tests (all passing)
- ✅ Registered in `src/nodes/mod.rs`
- ⚠️ **Not yet available in Phonon DSL syntax**

### Future DSL Integration

Once integrated into the compiler, the DSL syntax would be:

```phonon
-- FUTURE SYNTAX (not yet implemented)
tempo: 2.0

-- Octave down
~bass: saw 110
~pitched_down: ~bass # resample 0.5
out1: ~pitched_down

-- Octave up
~lead: saw 440
~pitched_up: ~lead # resample 2.0
out2: ~pitched_up

-- Wobble effect (pattern-modulated ratio)
~lfo: sine 0.5 * 0.5 + 1.0  -- Oscillate between 0.5 and 1.5
~pad: saw "110 165 220"
~wobbling: ~pad # resample ~lfo
out3: ~wobbling
```

To enable DSL syntax, add to `src/compositional_compiler.rs`:

```rust
"resample" => compile_resample(ctx, args),

fn compile_resample(
    ctx: &mut CompilerContext,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "resample requires 2 arguments (input, ratio), got {}",
            args.len()
        ));
    }

    let input = compile_expr(ctx, &args[0])?;
    let ratio = compile_signal(ctx, &args[1])?;

    let node_id = ctx.graph.create_node(
        Box::new(ResampleNode::new(input, ratio, ctx.graph.sample_rate()))
    );

    Ok(node_id)
}
```

## Test Coverage

The implementation includes 14 comprehensive tests:

1. ✅ Ratio 0.5 = half speed (octave down)
2. ✅ Ratio 2.0 = double speed (octave up)
3. ✅ Ratio 1.0 = unity (passthrough)
4. ✅ Linear interpolation accuracy
5. ✅ Pattern modulation of ratio
6. ✅ Stability over long duration (1000 blocks)
7. ✅ Phase wrapping behavior
8. ✅ clear_buffer() reset functionality
9. ✅ Correct dependency reporting
10. ✅ Very slow ratio (0.1)
11. ✅ Very fast ratio (4.0)
12. ✅ Smooth transitions with fractional ratios
13. ✅ Minimum ratio clamping (prevents division errors)
14. ✅ Buffer fill behavior

All tests verify:
- Output energy (RMS)
- Stability (no NaN/inf)
- Smooth output (no huge discontinuities)
- Correct state management

## Performance Characteristics

### CPU Usage

- **Per-sample cost**: ~5 operations
  - 1 buffer write
  - 2 buffer reads (with bounds check)
  - 1 linear interpolation (1 multiply, 1 add)
  - 1 phase advance

- **Memory**: ~17 KB per instance (4410 f32 samples)

### Real-Time Safety

- ✅ No allocations in process_block
- ✅ Bounded execution time
- ✅ Lock-free operation
- ✅ SIMD-friendly (linear memory access)

## Applications

### Music Production

- **Pitch correction** (subtle ratio adjustments around 1.0)
- **Harmony generation** (0.5 = octave down, 1.5 = fifth up)
- **Tape stop effects** (gradually reduce ratio to 0.1)
- **Tape speed up** (gradually increase ratio from 0.1 to 1.0)

### Sound Design

- **Formant preservation** (unlike time-stretching, maintains timbre)
- **Granular texture** (rapid ratio modulation)
- **Glitch effects** (sudden ratio jumps)
- **Doppler simulation** (LFO-modulated ratio)

### Game Audio

- **Dynamic pitch based on game state** (speed, danger level, etc.)
- **Character voice variations** (0.9-1.1 for variety)
- **Slow-motion audio** (0.3-0.5 ratio during slow-mo)

## Limitations

1. **No anti-aliasing** - Can produce aliasing artifacts at extreme ratios
   - Mitigate: Keep ratios within ±1 octave (0.5 to 2.0)
   - Future: Add optional anti-aliasing filter

2. **Fixed buffer size** - 100ms may not suit all use cases
   - Future: Make buffer size configurable

3. **Forward playback only** - Negative ratios not supported
   - Future: Add reverse playback (ratio < 0)

4. **Time-domain only** - Not suitable for polyphonic material preservation
   - Alternative: Use frequency-domain time stretching for that

## Future Enhancements

1. **Configurable interpolation**
   - Linear (current)
   - Cubic (better quality)
   - Sinc (highest quality)

2. **Anti-aliasing filter**
   - Lowpass filter before resampling
   - Adjustable based on ratio

3. **Reverse playback**
   - Support negative ratios
   - Read buffer backwards

4. **Buffer size control**
   - Expose buffer size parameter
   - Trade-off between latency and range

5. **Quality modes**
   - Fast: Linear interpolation (current)
   - Good: Cubic interpolation
   - Best: Sinc interpolation with anti-aliasing

## References

- **Linear Interpolation**: Standard DSP technique, see Julius O. Smith's "Physical Audio Signal Processing"
- **Resampling Theory**: "Digital Audio Resampling" by Jon Dattorro
- **Phase Vocoder**: For higher-quality pitch/time manipulation (different approach)

## License

MIT (same as Phonon project)

---

**Implementation Date**: 2025-11-20
**Author**: Claude (Anthropic AI)
**Status**: Production-ready (pending DSL integration)
