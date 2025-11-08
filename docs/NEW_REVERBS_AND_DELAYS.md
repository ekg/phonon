# New Reverbs and Delays Implementation Guide

## Overview

We're adding professional-grade reverbs and creative delays to Phonon:

**Reverbs:**
- **Dattorro Plate** - Rich, dense, professional (Lexicon/Valhalla quality)
- **Hall** - Spacious, long tail (can use Dattorro with different params)

**Delays:**
- **Tape Delay** - Vintage tape simulation (wow, flutter, saturation)
- **Multi-Tap Delay** - Rhythmic multiple echoes
- **Ping-Pong Delay** - Stereo bouncing delay

## Implementation Status

✅ SignalNode variants added (unified_graph.rs:904-943, 1065-1078)
✅ State structs added (unified_graph.rs:1810-1956)
⏳ DSP evaluation (needs to be added to eval_node())
⏳ Compiler functions (needs to be added to compositional_compiler.rs)
⏳ Function table registration

## Current vs New Reverb Comparison

### Freeverb (Current)
- **Algorithm**: 8 parallel combs + 4 series allpasses
- **Character**: Soft, subtle, neutral, smooth
- **Pros**: Fast, low CPU, invisible/transparent
- **Cons**: Static (no modulation), loses detail, generic sound
- **Use case**: Background ambience, mixing "glue"

### Dattorro Plate (New)
- **Algorithm**: Figure-8 delay network with modulated allpasses
- **Character**: Rich, dense, lush, evolving
- **Pros**: Professional sound, stereo width, adjustable character
- **Cons**: Higher CPU, more complex
- **Use case**: Lead vocals, drums, creative effects
- **Based on**: Jon Dattorro's 1997 AES paper (industry standard)

## DSP Algorithms Explained

### Tape Delay

**Characteristics:**
- Wow: Slow pitch modulation (0.1-2 Hz) from tape speed variation
- Flutter: Fast pitch modulation (5-10 Hz) from motor vibration
- Saturation: Tape compression/warmth
- Head filtering: Rolled-off highs (~5kHz)

**Implementation:**
```rust
// Modulate delay time with wow and flutter LFOs
let wow_mod = sin(wow_phase) * wow_depth * 0.001;  // ±1ms
let flutter_mod = sin(flutter_phase) * flutter_depth * 0.0001;  // ±0.1ms
let delay_time = base_time + wow_mod + flutter_mod;

// Fractional delay (linear interpolation)
let delayed = lerp(buffer[i], buffer[i+1], frac);

// Tape saturation
let saturated = tanh(delayed * drive) / drive;

// Tape head lowpass
let filtered = lpf_state * 0.7 + saturated * 0.3;
```

### Dattorro Reverb

**Architecture:**
1. Pre-delay (early reflections)
2. Input diffusion (4 allpass filters)
3. Figure-8 tank (two cross-coupled delay networks)
4. Each tank has:
   - Modulated allpass filter
   - Delay line
   - Another modulated allpass
   - Another delay line
   - Lowpass filter (damping)
5. Cross-feed between left/right tanks
6. Multiple tap points for output

**Key Parameters:**
- `pre_delay`: 0-500ms (room size perception)
- `decay`: 0.1-10.0 (RT60 time)
- `diffusion`: 0.0-1.0 (echo density)
- `damping`: 0.0-1.0 (high-frequency absorption)
- `mod_depth`: 0.0-1.0 (chorusing/shimmer)

**Why it sounds better:**
- Modulated delays = evolving, lush sound
- Figure-8 topology = realistic stereo spread
- Allpass diffusers = dense early reflections
- Tuned delay lengths = no metallic ringing

## Usage Examples

### Tape Delay
```phonon
-- Classic dub delay
~dub: tapedelay 0.375 0.7 0.5 0.02 6.0 0.05 0.3 0.5
~drums: s "bd sn" # ~dub
out: ~dub

-- Parameters: time feedback wow_rate wow_depth flutter_rate flutter_depth saturation mix
```

### Dattorro Plate
```phonon
-- Bright plate (vocals, snare)
~plate: plate 20 2.0 0.7 0.3 0.3 0.5
~drums: s "bd sn hh cp" # ~plate
out: ~plate

-- Parameters: pre_delay(ms) decay diffusion damping mod_depth mix
```

### Multi-Tap Delay
```phonon
-- Rhythmic delays
~taps: multitap 0.25 4 0.5 0.6
~synth: saw 220 # ~taps
out: ~taps

-- Parameters: time taps feedback mix
```

## Next Steps

To complete the implementation:

1. Add DSP evaluation code to `src/unified_graph.rs` in `eval_node()` function
   - Insert after existing `Delay` case (around line 7138)
   - Use implementations from `/tmp/new_effects_dsp.rs`

2. Add compiler functions to `src/compositional_compiler.rs`:
   ```rust
   fn compile_tapedelay(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String>
   fn compile_plate(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String>
   fn compile_multitap(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String>
   fn compile_pingpong(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String>
   ```

3. Register in function table (src/compositional_compiler.rs around line 829):
   ```rust
   "tapedelay" | "tape" => compile_tapedelay(ctx, args),
   "plate" => compile_plate(ctx, args),
   "multitap" => compile_multitap(ctx, args),
   "pingpong" => compile_pingpong(ctx, args),
   ```

4. Update effect bus detection (src/compositional_compiler.rs around line 71):
   - Add new effect names to `is_effect_function()` match

5. Test with example files

## Performance Notes

- **Tape Delay**: ~2x CPU of basic delay (LFOs + filtering)
- **Dattorro**: ~3-4x CPU of Freeverb (more delay lines, modulation)
- **Multi-Tap**: Linear with tap count (use 2-4 taps for efficiency)
- **Ping-Pong**: Similar to basic delay

All are real-time capable on modern CPUs.

## References

- Dattorro, J. (1997). "Effect Design, Part 1: Reverberator and Other Filters." AES Journal
- Schroeder, M.R. (1962). "Natural Sounding Artificial Reverberation." JAES
- Tape Delay: Based on Echoplex EP-3, Roland Space Echo characteristics
