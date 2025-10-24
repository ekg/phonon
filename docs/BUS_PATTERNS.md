# Bus References in Patterns

## Overview

**Bus references in patterns** is a powerful feature that lets you sequence any bus (custom synths, continuous signals, filtered sounds) using Tidal Cycles mini-notation, mixing them seamlessly with samples!

## Syntax

```phonon
s "~busname sample ~busname sample"
```

Any pattern event starting with `~` is interpreted as a bus reference instead of a sample name.

## How It Works

When a pattern encounters `~busname`:

1. **Looks up the bus** by name
2. **Calculates event duration** from the pattern timing
3. **Samples the bus signal** for that duration (creates a synthetic audio buffer)
4. **Triggers it as a voice** through the voice manager with envelope support
5. **Mixes with other pattern events** (samples or other bus refs)

This means your custom synths/sounds get:
- ✅ **Pattern timing** - triggered at the right moments
- ✅ **Envelope control** - attack/release from pattern
- ✅ **Voice management** - polyphonic playback
- ✅ **Seamless mixing** with samples

## Examples

### Basic Bus Triggering

```phonon
tempo: 2.0

-- Define a custom kick on a bus
~mykick: sine 55 + sine 54.8

-- Trigger it in a pattern
out: s "~mykick ~ ~mykick ~"
```

### Mixing Buses and Samples

```phonon
tempo: 2.0

-- Custom synths
~kick: sine 55
~snare: square 200 + noise 0 * 0.3

-- Mix custom sounds with samples
out: s "~kick sd ~kick ~snare"
```

### Melodic Bus Patterns

```phonon
tempo: 2.0

-- Bass synth
~bass: saw 55 + saw 55.5

-- Trigger with rhythm variations
out: s "~bass ~bass ~bass*2 ~bass"
```

### Complex Multi-Layer

```phonon
tempo: 2.0

-- Custom instruments
~kick: sine 55 + sine 54.8
~snare: square 200 + noise 0 * 0.3
~bass: saw 55 + saw 55.5 + saw 110

-- Pattern layers
~drums1: s "~kick sd ~kick sd"
~drums2: s "bd ~snare bd ~snare"
~bassline: s "~bass ~bass ~bass*2 ~bass"

-- Mix everything
out: ~drums1 * 0.5 + ~drums2 * 0.4 + ~bassline * 0.6
```

### With Effects Chains

```phonon
tempo: 2.0

-- Synth with effects
~acid: saw 110 # lpf 800 0.9 # delay 0.25 0.4 0.5

-- Sequence the effected sound
out: s "~acid ~ ~acid*3 ~acid"
```

### With Env_trig

```phonon
tempo: 2.0

-- Continuous drone
~drone: saw 55 + saw 55.5

-- Gate it rhythmically
~gated: ~drone # env_trig "x(5,8)" 0.005 0.1 0.2 0.05

-- Then sequence the gated version!
out: s "~gated ~gated*2 ~gated ~gated"
```

## Use Cases

### 1. **Custom Percussion**

Build your own drum sounds from synths:

```phonon
~kick: sine 60 + sine 59.8
~snare: square 220 + noise 0 * 0.4
~hat: square 8000 * 0.3 + noise 0 * 0.2

out: s "~kick ~snare ~hat*2 ~snare"
```

### 2. **Melodic Sequences**

Sequence pitched synths like samples:

```phonon
~lead: square 440 + saw 440 * 0.5

out: s "~lead*4"  -- Rapid-fire melody
```

### 3. **Hybrid Drum Patterns**

Mix custom synths with classic samples:

```phonon
~custom_kick: sine 55
~custom_snare: noise 0 # lpf 3000 0.5

out: s "~custom_kick sd ~custom_kick ~custom_snare"
```

### 4. **Generative Layering**

Build complex textures:

```phonon
~texture1: saw 110 # lpf 400 0.8
~texture2: sine 220 # hpf 800 0.5

~layer1: s "~texture1(3,8)"
~layer2: s "~texture2(5,8)"

out: ~layer1 + ~layer2
```

## Technical Details

### Event Duration

The duration of the synthesized buffer is calculated from the pattern event's timespan:

```rust
let event_duration = event.whole.end - event.whole.begin  // in cycles
let duration_samples = event_duration * sample_rate * cps
```

This means:
- Quarter notes get longer buffers than eighth notes
- Subdivision (`*2`, `*4`) creates shorter buffers
- Pattern transformations (`fast`, `slow`) affect buffer length

### Envelope Application

Bus references get the same envelope support as samples:
- Default: Very short attack (0.001s), long release (10s)
- Can be overridden with `attack` and `release` pattern parameters (TODO: document)

### Voice Management

Bus-triggered sounds use the same voice manager as samples, so:
- They're polyphonic (multiple instances can play simultaneously)
- They respect cut groups if implemented
- They mix automatically with sample playback

## Limitations

1. **Computational Cost**: Each bus reference creates a synthetic buffer by evaluating the bus signal sample-by-sample. Complex synthesis chains may be expensive.

2. **Static Duration**: The bus is sampled for the event duration, so time-varying modulation (LFOs, etc.) will be "frozen" into the buffer.

3. **No Live Modulation**: Once triggered, the bus reference plays back a static buffer - it doesn't continue to track the live bus output.

## Comparison with Other Approaches

| Approach | Use Case | Triggering | Modulation |
|----------|----------|------------|------------|
| `s "~bus"` | Sequence synths like samples | Pattern events | Frozen at trigger time |
| `bus # env_trig "x"` | Gate continuous signal | Pattern rhythm | Live, continuous |
| `sine_trig "pattern"` | Pattern-controlled synth | Pattern notes | Per-note envelopes |
| Continuous bus | Drone/pad | Always on | Live, continuous |

## Implementation

Location: `src/unified_graph.rs:1594-1739`

The feature is implemented in the `SignalNode::Sample` evaluation code, which detects `~` prefix and switches from sample loading to bus sampling.

## Status

**FULLY IMPLEMENTED AND WORKING**

- ✅ Bus reference detection
- ✅ Synthetic buffer generation
- ✅ Voice triggering with envelopes
- ✅ Mixing with samples
- ✅ Pattern timing/duration calculation
- ✅ All pattern transformations work
