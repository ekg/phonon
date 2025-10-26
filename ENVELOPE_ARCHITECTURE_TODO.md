# Envelope Architecture - What's Needed

## The Core Problem

**Current State**: Each envelope needs a separate `_trig` variant AND separate pattern
```phonon
-- BROKEN: TWO patterns to maintain
~melody: sine "c4 e4 g4"
~env: segments_trig "x x x" "0 1 0" "0.1 0.2"
~shaped: ~melody * ~env
```

**What We Want**: ONE pattern that triggers everything
```phonon
-- IDEAL: ONE pattern, custom envelope
~melody: synth_segments "c4 e4 g4" saw "0 1 0.5 0" "0.1 0.1 0.2"
```

## What Works NOW

### Option 1: Use synth with ADSR (recommended for most cases)
```phonon
~melody: synth "c4 e4 g4" saw 0.05 0.1 0.7 0.2
~bass: synth "c2 ~ e2 g2" saw 0.1 0.3 0.6 0.5
```

ADSR covers 80% of musical use cases!

### Option 2: Use env_trig for rhythmic envelopes
```phonon
~kick_env: env_trig "x(3,8)" 0.01 0.3 0.0 0.1
~kick: sine 60 * ~kick_env

~hat_env: env_trig "~ x ~ x" 0.001 0.05 0.0 0.05
~hat: white_noise * ~hat_env
```

Works great for drums/percussion where the envelope IS the rhythm.

### Option 3: Layer multiple env_trigs for complex shapes
```phonon
~attack: env_trig "x ~ x ~" 0.01 0.0 1.0 0.01
~body: env_trig "x ~ x ~" 0.05 0.2 0.0 0.0
~combined: ~attack * 0.3 + ~body * 0.7
```

Can approximate complex envelopes by combining simpler ones.

## What's Needed (Architectural Work)

### Short Term: synth_segments
```phonon
~melody: synth_segments "c4 e4 g4" saw "0 1 0.5 0" "0.1 0.1 0.2"
```

**Requires**:
1. Extend `SynthVoiceManager` to support different envelope types
2. Create `SegmentEnvelope` that can be triggered per-voice
3. Add voice allocation/polyphony handling

**Estimated Work**: 4-6 hours

### Medium Term: synth_curve, synth_ad variants
```phonon
~melody: synth_curve "c4 e4 g4" saw 0.0 1.0 0.3 3.0
~perc: synth_ad "bd sn" sine 0.01 0.2
```

**Same architecture as synth_segments**, just different envelope implementations.

### Long Term: Composable Envelopes
**Ideal but complex**:
```phonon
-- Define envelope once, use everywhere
~env: segments "0 1 0.5 0" "0.1 0.1 0.2"
~melody: synth "c4 e4 g4" saw ~env
```

**Requires**: First-class functions / envelope as a parameter type. Major architectural change.

## Current Best Practices

1. **For most melodies**: Use `synth` with ADSR
2. **For drums**: Use `env_trig` with different ADSR per voice
3. **For bass**: Use `synth` with longer decay/release
4. **For pads**: Use `synth` with slow attack and high sustain

ADSR is incredibly versatile - adjust parameters to get different feels:
- Percussive: `0.01 0.2 0.0 0.1`
- Plucky: `0.001 0.05 0.0 0.05`
- Pad: `0.5 0.2 0.8 1.0`
- Bell: `0.01 0.5 0.3 0.8`

## Conclusion

The user is RIGHT that the current architecture makes live coding harder than it should be.

**For now**: Use `synth` with ADSR - it's very flexible.

**Next steps**: Implement synth_segments/synth_curve as proper variants with voice management.
