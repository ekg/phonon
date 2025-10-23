# Envelope Support: Current Status & Findings

## Discovery

✅ **Envelope function EXISTS and WORKS** in DSL!

```phonon
out: sine 440 # env 0.01 0.1 0.3 0.2
--  (attack decay sustain release)
```

## Test Results

| Test | RMS | Peak | Notes |
|------|-----|------|-------|
| Sine + env | 0.187 (-14.6 dB) | 0.795 (-2.0 dB) | Works correctly |
| DSL kick | 0.123 (-18.2 dB) | 0.769 (-2.3 dB) | Basic kick sound |
| Native kick | 0.201 (-13.9 dB) | 0.773 (-2.2 dB) | Fuller sound |

## How It Works

Current implementation:
```rust
SignalNode::Envelope {
    input: input_signal,
    trigger: Signal::Value(1.0),  // <-- Always on!
    attack, decay, sustain, release,
    state: EnvState::default(),
}
```

- Trigger is **constantly 1.0**
- Envelope triggers **once at start**
- Then stays in **sustain phase**
- Works for continuous sounds, NOT for rhythm

## Critical Limitation Discovered

### The Real Problem: No Pattern Triggering

**Both native AND DSL synths produce continuous drones!**

- `superkick 60` = continuous kick drone (sustaining)
- `kick 60` (DSL) = continuous kick drone (sustaining)
- Neither responds to patterns like `"x ~ x ~"`

### What's Missing

1. **Pattern → Trigger signal**
   - Patterns need to generate trigger pulses
   - `"x ~ x ~"` should create 4 trigger events per cycle

2. **Envelope retriggering**
   - Each pattern event should reset envelope
   - Currently envelope triggers once and sustains

3. **Syntax for pattern-controlled synths**
   - Need: `superkick "60 ~ 65 ~"` (pattern of pitches)
   - Or: `~pattern: "x ~ x ~"` then `kick 60 ~pattern` (pattern of triggers)

## DSL Kick Comparison

### Simplified DSL Version
```phonon
fn kick freq = sine freq # env 0.001 0.3 0.0 0.1
```

**Missing from native:**
- Pitch envelope (freq sweep 3x → 1x)
- Noise layer for attack click
- More complex envelope shaping

**But basic shape works!**

## Conclusion

✅ Envelope function exists and works
✅ Can create basic percussion sounds
❌ **Cannot trigger from patterns** (applies to ALL synths)
❌ Missing pitch envelopes (freq modulation by envelope)

### Priority Fix

**Implement pattern → trigger system** to make all synths musical.

Without this, synths are just continuous test tones!
