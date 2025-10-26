# The Right Architecture: Per-Event Effects

## What You Want (and it's RIGHT!)

```phonon
-- ONE pattern, envelope applies to each hit
s "bd sn bd cp" # segments "0 1 0" "0.1 0.2"

-- Mix samples and synth in ONE pattern
s "bd synth:c4 bd synth:e4" # segments "0 1 0" "0.1 0.2"

-- Effects apply per-event
s "bd sn" # lpf "400 2000" 0.8 # segments "0 1 0" "0.1 0.2"
```

Each hit (whether sample or synth) gets:
- Its own envelope instance
- Its own filter state
- Its own effects

This is how Tidal works! One pattern, effects apply per-event.

## Why It Doesn't Work Now

Current architecture:
```
Pattern → Trigger Voices → Voice Manager → Audio Signal → Effects Chain
```

Effects apply to the MIXED audio signal, not per-event.

```phonon
-- CURRENT (doesn't work how you want):
~drums: s "bd sn bd cp"
~env: segments "0 1 0" "0.1 0.2"  # This is CONTINUOUS, not per-event!
out: ~drums * ~env  # Envelope applies to mixed signal, not each hit

-- Result: All drums share ONE envelope, not individual envelopes per hit
```

## What Needs to Change

**Architecture shift**: Per-event parameters

```rust
// Current: Voices just play audio
struct Voice {
    sample: Sample,
    position: usize,
    // ...
}

// Needed: Voices carry per-event parameters
struct Voice {
    sample: Sample,
    position: usize,
    envelope: EnvelopeInstance,    // Each voice has its own!
    filter: FilterState,           // Each voice has its own!
    effects: Vec<EffectInstance>,  // Each voice has its own!
    // ...
}
```

**Pattern syntax**: Effects as modifiers

```phonon
-- The `#` operator passes parameters to each event
s "bd sn bd cp"
  # segments "0 1 0" "0.1 0.2"    # envelope per-event
  # lpf "400 2000" 0.8            # filter per-event
  # pan "-1 0 1 0"                # pan per-event
```

## How Tidal Does It

In Tidal/SuperDirt:
```haskell
-- Effects are per-event parameters
s "bd sn"
  # vowel "a e"        -- Each hit gets different vowel
  # cutoff "400 2000"  -- Each hit gets different filter
  # gain "0.8 1.0"     -- Each hit gets different volume
```

SuperDirt receives OSC messages with ALL parameters per-event:
```
/dirt/play {
    sound: "bd",
    cutoff: 400,
    resonance: 0.8,
    gain: 0.8,
    pan: 0.0,
    // ... all parameters bundled together
}
```

## Implementation Plan

### Phase 1: Extend Voice with Per-Event Envelopes (4-6 hours)

```rust
// In voice_manager.rs
pub struct Voice {
    // ... existing fields ...

    // Add envelope support
    envelope_type: EnvelopeType,
    envelope_state: EnvelopeState,
}

pub enum EnvelopeType {
    ADSR { attack, decay, sustain, release },
    Segments { levels: Vec<f32>, times: Vec<f32> },
    Curve { start, end, duration, curve },
}
```

### Phase 2: Chain Syntax for Per-Event Parameters (2-4 hours)

```rust
// In compositional_compiler.rs

// Recognize chain syntax: s "..." # effect1 # effect2
fn compile_s_with_effects(pattern: Pattern, effects: Vec<Effect>) -> NodeId {
    // Parse pattern
    // For each event, attach effect parameters
    // Pass to voice manager as bundle
}
```

### Phase 3: Per-Event Filters & Effects (6-8 hours)

```rust
// Each voice needs its own filter state
pub struct Voice {
    // ...
    filter: Option<FilterInstance>,
    effects: Vec<EffectInstance>,
}
```

## Total Estimated Work: 12-18 hours

But this would make Phonon MUCH better for live coding!

## Short-Term Workaround

Until this is implemented, use:

```phonon
-- Option 1: synth with ADSR (works great for most cases)
~melody: synth "c4 e4 g4" saw 0.05 0.1 0.7 0.2

-- Option 2: env_trig for rhythmic patterns
~kick: env_trig "x ~ x ~" 0.01 0.3 0.0 0.1
~kick_sound: sine 60 * ~kick

-- Option 3: Individual samples with different envelopes
~kick: s "bd" * env_trig "x ~ x ~" 0.01 0.3 0.0 0.1
~snare: s "sn" * env_trig "~ x ~ x" 0.001 0.1 0.0 0.05
~mix: ~kick + ~snare
```

## Conclusion

You're absolutely right about the architecture. The current system makes live coding harder than it should be.

**The RIGHT way**: Per-event effects/envelopes that automatically apply to each pattern event.

**The workaround**: Use `synth` with ADSR for most cases, `env_trig` for drums.

**The future**: Implement per-event parameters so `s "..." # effect` just works.
