# Live Coding Envelope Solution

## The Problem

**BROKEN APPROACH** (what I initially built):
```phonon
-- TWO patterns to maintain - NOT LIVE CODING FRIENDLY!
~melody: sine "c4 e4 g4"
~env: segments_trig "x x x" "0 1 0" "0.1 0.2"
~shaped: ~melody * ~env
```

Problem: You have to duplicate the pattern structure. If you change the melody to `"c4 ~ e4 g4"`, you also need to change the trigger pattern to `"x ~ x x"`. This is tedious and error-prone for live coding.

## The Solution

**CORRECT APPROACH** (single pattern):
```phonon
-- ONE pattern - the notes implicitly trigger the envelope!
~melody: synth_segments "c4 e4 g4" saw "0 1 0.5 0" "0.1 0.1 0.2"
```

## Available Synth Variants

### Standard ADSR (already exists)
```phonon
~melody: synth "c4 e4 g4 c5" saw 0.05 0.1 0.7 0.2
-- Parameters: attack decay sustain release
```

### Custom Segments (NEW - to be implemented)
```phonon
~melody: synth_segments "c4 e4 g4" saw "0 1 0.5 0" "0.1 0.1 0.2"
-- Parameters: levels times
-- Arbitrary breakpoint envelope
```

### Curved Envelopes (NEW - to be implemented)
```phonon
~melody: synth_curve "c4 e4 g4" saw 0.0 1.0 0.3 3.0
-- Parameters: start end duration curve_shape
-- Exponential/logarithmic shapes
```

### Simple AD (NEW - to be implemented)
```phonon
~melody: synth_ad "c4 e4 g4" saw 0.05 0.2
-- Parameters: attack decay
-- Perfect for percussion
```

## Why This Works

Each note in the pattern `"c4 e4 g4"` automatically:
1. Triggers a new envelope
2. Sets the oscillator frequency
3. Manages voice allocation

You edit ONE pattern, everything stays in sync. This is how live coding should work.

## Implementation Status

- [x] `synth` (ADSR) - WORKING
- [ ] `synth_segments` - TO IMPLEMENT
- [ ] `synth_curve` - TO IMPLEMENT
- [ ] `synth_ad` - TO IMPLEMENT

The key insight: **Envelopes should be properties of synths, not separate signal chains that need manual pattern duplication.**
