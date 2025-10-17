# Sample Overlap Behavior in Phonon

## What You're Hearing

When using alternating patterns like `"bd(<3 5>,8)"`, you may notice a "layered" or "distorted" sound where multiple samples seem to play on top of each other. This is **expected behavior** in Tidal Cycles-style pattern systems.

## How It Works

### Pattern Alternation
```phonon
tempo: 0.5
out: s("bd(<3 5>,8)")
```

This pattern alternates between:
- **Cycle 0**: 3 bass drum hits distributed across 8 steps (Euclidean rhythm)
- **Cycle 1**: 5 bass drum hits distributed across 8 steps
- **Cycle 2**: Back to 3 hits
- **Cycle 3**: Back to 5 hits
- etc.

### Sample Playback Behavior

1. Each hit **triggers a new sample** that plays to completion
2. Samples **do not auto-stop** when new events occur
3. When the pattern changes from 3 hits to 5 hits, the new samples trigger **while old samples are still decaying**
4. This creates **natural overlap** and layering

## Why This Happens

From the code in `/home/erik/phonon/src/unified_graph.rs:1380-1386`:

```rust
// Get sample from bank and trigger a new voice
if let Some(sample_data) = self.sample_bank.borrow_mut().get_sample(sample_name) {
    self.voice_manager.borrow_mut().trigger_sample_with_cut_group(
        sample_data,
        gain_val,
        pan_val,
        speed_val,
        cut_group_opt,  // ← This controls whether samples cut each other
    );
}
```

The `cut_group_opt` parameter determines whether voices cut each other off:
- **`None` (cut_group = 0)**: Samples play to completion, overlapping naturally
- **`Some(n)` (cut_group = n)**: Voices in the same cut group stop each other when triggered

## Current State: No Cut Groups in DSL

Currently, all samples use `cut_group: Signal::Value(0.0)` which means **no cut group** (see `unified_graph_parser.rs:1213`).

The cut group infrastructure exists in the voice manager (`voice_manager.rs:104-106` and `voice_manager.rs:264-272`), but it's **not exposed through the DSL syntax** yet.

## Example Files

### Natural Overlap (Current Behavior)
```phonon
# File: examples/sample_overlap_demo.ph
tempo: 0.5
out: s("bd(<3 5>,8)")
```

**Result**: Bass drums overlap when pattern alternates, creating a thicker, layered sound.

### More Complex Alternation
```phonon
tempo: 1.0
out: s("bd(<3 4 5>,8) sn(5,8,2)")
```

**Result**: Multiple rhythmic patterns with overlapping samples across different instruments.

## Musical Applications

This overlap behavior can be used creatively:

1. **Thick Textures**: Layering creates natural density
2. **Polyrhythmic Complexity**: Different patterns create emergent rhythms
3. **Evolving Timbres**: Overlapping samples with different start times create shifting tone colors
4. **Controlled Chaos**: Alternating patterns create structured but unpredictable layering

## Future: Cut Group Support

To implement cut group control in the DSL, we would need to:

1. Add `cut_group` field to `DslExpression::SamplePattern`:
   ```rust
   SamplePattern {
       pattern: String,
       gain: Option<Box<DslExpression>>,
       pan: Option<Box<DslExpression>>,
       speed: Option<Box<DslExpression>>,
       cut_group: Option<Box<DslExpression>>,  // ← Add this
   }
   ```

2. Update the parser to accept a 5th argument:
   ```phonon
   # Syntax: s("pattern", gain, pan, speed, cut_group)
   out: s("bd(<3 5>,8)", 1.0, 0.0, 1.0, 1.0)  # cut_group = 1
   ```

3. Update the compiler to use the cut_group parameter:
   ```rust
   let cut_group_signal = cut_group
       .map(|e| self.compile_expression_to_signal(*e))
       .unwrap_or(Signal::Value(0.0));
   ```

## Testing Overlap Behavior

Render the demo file to hear the effect:
```bash
cargo run --release --bin phonon -- render examples/sample_overlap_demo.ph test.wav --cycles 4
```

**Expected Output**:
- RMS: ~0.14 (-16.9 dB)
- Peak: ~0.80 (-1.9 dB)
- Natural layering of bass drum samples as pattern alternates

## Comparison to Other Systems

- **Tidal Cycles**: Same behavior - samples overlap by default
- **SuperCollider**: Requires explicit voice management (Pbind with \instrument)
- **Ableton Live**: Samples can be set to "Cut" or "Play Through" mode
- **Phonon**: Currently "Play Through" only, but "Cut" mode exists internally

## Summary

The sample overlap you're experiencing is:
1. ✅ **Expected behavior** - not a bug
2. ✅ **Consistent with Tidal Cycles** - standard pattern system design
3. ✅ **Musically useful** - creates natural layering and texture
4. ⚠️ **Not user-controllable yet** - cut groups aren't exposed in DSL

If you want to prevent overlap, cut group support would need to be added to the DSL parser.
