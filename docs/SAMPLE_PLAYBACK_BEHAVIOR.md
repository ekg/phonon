# Sample Playback Behavior - How It Should Work

This document describes the correct behavior of sample playback parameters based on Tidal Cycles and Strudel implementations.

**Last updated**: 2025-11-10

## Executive Summary

Sample playback in Tidal Cycles has nuanced behavior that differs from synthesis:

1. **Default behavior**: Samples play their full duration unless controlled by parameters
2. **legato**: Controls duration relative to event spacing (inter-onset time)
3. **speed**: Controls playback rate and pitch; negative values play backwards
4. **sustain**: Direct duration control in seconds
5. **loopAt**: Makes a sample fit N cycles by adjusting speed

## Critical Implementation Details

### The Legato Parameter

#### What It Does

`legato` controls the duration of a sound relative to its "space" in the pattern - the time from the beginning of one sound to the beginning of the next (inter-onset time).

- **`legato 1`**: Sound plays for exactly the duration of its space in the pattern
- **`legato > 1`**: Sounds overlap (e.g., `legato 2` = 2x the space = overlap)
- **`legato < 1`**: Gaps between sounds (e.g., `legato 0.5` = half the space = gaps)

#### Default Behavior - CRITICAL DIFFERENCE

**For Samples:**
- When `legato` is **NOT specified**, samples play their **full duration**
- This is the default behavior - samples are not cut off

**For Synthesizers:**
- When `legato` is **NOT specified**, SuperDirt defaults to `legato 1`
- Synths play for exactly their event duration

#### How It Works Internally

In Tidal/SuperDirt:
1. Tidal calculates the "delta" (inter-onset time) for each event
2. If `legato` is specified: `sustain = delta * legato`
3. If `legato` is NOT specified:
   - Samples: no sustain envelope applied (full sample plays)
   - Synths: `sustain = delta * 1` (default legato=1)
4. SuperDirt applies an envelope to cut the sound at the sustain time

#### In Strudel

Strudel renamed `legato` to `clip` to avoid confusion:
- `clip` works the same as `legato` in Tidal
- Setting `clip(1)` tells the sampler to respect the event duration
- Without `clip`, samples play until their buffer ends (like Tidal)

#### Example Behavior

Pattern: `"bd sn hh cp"` (4 events per cycle at tempo 2.0 cps)
- Event spacing: 0.25 cycles = 0.125 seconds each

**No legato specified:**
```
bd: plays full sample (e.g., 0.5 seconds if that's the sample length)
sn: plays full sample
hh: plays full sample
cp: plays full sample
Result: Samples overlap naturally based on their recorded length
```

**With `legato 1`:**
```
bd: plays for 0.125 seconds (cut off by envelope)
sn: plays for 0.125 seconds
hh: plays for 0.125 seconds
cp: plays for 0.125 seconds
Result: Each sample cut to exactly fit its space - no overlap, no gaps
```

**With `legato 2`:**
```
bd: plays for 0.25 seconds (2x the space)
sn: plays for 0.25 seconds
hh: plays for 0.25 seconds
cp: plays for 0.25 seconds
Result: Samples overlap (each occupies 2 event slots)
```

**With `legato 0.5`:**
```
bd: plays for 0.0625 seconds (half the space)
sn: plays for 0.0625 seconds
hh: plays for 0.0625 seconds
cp: plays for 0.0625 seconds
Result: Staccato - gaps between each sound
```

### The Speed Parameter

#### What It Does

`speed` controls the playback rate of samples, which affects both pitch and duration simultaneously.

- **`speed 1`**: Normal playback (default)
- **`speed 2`**: 2x faster (higher pitch, shorter duration)
- **`speed 0.5`**: Half speed (lower pitch, longer duration)
- **`speed -1`**: Reverse playback at normal speed
- **`speed -2`**: Reverse playback at 2x speed (higher pitch)

#### Negative Speed - Reverse Playback

Negative speed values play samples backwards:

```haskell
d1 $ s "bd sn" # speed "-1"     -- play backwards at normal speed
d1 $ s "bd sn" # speed "-0.5"   -- play backwards at half speed
d1 $ s "bd sn" # speed "[1 -1]" -- alternate forward/backward
```

#### Implementation Details

In Strudel's sampler.mjs:
```javascript
// Check for negative speed and reverse buffer
if (hapValue.speed < 0) {
  buffer = reverseBuffer(buffer);  // Reverse the audio buffer
}

// Then play at absolute value of speed
let playbackRate = Math.abs(speed) * Math.pow(2, transpose / 12);
```

**Key points:**
1. Buffer is reversed when speed < 0
2. Playback rate uses absolute value of speed
3. Speed of 0 = no playback (early return)

#### Pitch Relationship

Speed affects pitch logarithmically:
- `speed 2` = +12 semitones (one octave up)
- `speed 0.5` = -12 semitones (one octave down)
- `speed 4` = +24 semitones (two octaves up)

### The LoopAt Function

#### What It Does

`loopAt n` makes a sample fit exactly `n` cycles by automatically adjusting playback speed.

```haskell
d1 $ loopAt 4 $ sound "break:8"   -- "break:8" sample fits exactly 4 cycles
d1 $ loopAt 1 $ sound "amen"      -- "amen" sample fits exactly 1 cycle
```

#### How It Works

Internally, `loopAt` sets two parameters:
1. `unit "c"` - changes speed unit to "cycles"
2. `speed` - calculated to make the sample fit

The speed is calculated as: `speed = sampleDurationInCycles / targetCycles`

#### With Tempo

If tempo is 2.0 cps (cycles per second):
- `loopAt 1` on a 2-second sample: plays at 1x speed (sample = 2 seconds = 1 cycle)
- `loopAt 2` on a 2-second sample: plays at 0.5x speed (stretched to 4 seconds = 2 cycles)
- `loopAt 0.5` on a 2-second sample: plays at 2x speed (compressed to 1 second = 0.5 cycles)

#### Interaction with Speed Unit

The `unit` parameter changes how `speed` is interpreted:
- `unit "r"` (default): speed is relative playback rate (1 = normal)
- `unit "c"`: speed is in cycles (sample plays for `1/speed` cycles)
- `unit "s"`: speed is in seconds (sample plays for `1/speed` seconds)

### The Sustain Parameter

#### What It Does

`sustain` directly sets the duration in seconds, independent of pattern timing.

```haskell
d1 $ s "bd sn" # sustain 0.5      -- each sample plays for 0.5 seconds
d1 $ s "bd sn" # sustain "0.1 2"  -- bd plays 0.1s, sn plays 2s
```

#### Relationship to Legato

In SuperDirt, both `legato` and `sustain` control the same thing (sustain envelope), but:
- `legato` is relative to event spacing (multiplicative)
- `sustain` is absolute time in seconds (additive)

If both are specified: `final_sustain = sustain + (delta * legato)`

#### For Synths

For synthesizers, `sustain` is critical because it controls the envelope duration:
```haskell
d1 $ n "c e g" # s "superpiano" # sustain 2   -- each note lasts 2 seconds
```

### Other Sample Control Parameters

#### Begin and End

Control which portion of the sample plays:
- `begin 0` - start from beginning (default)
- `begin 0.5` - start from halfway through
- `end 1` - play until the end (default)
- `end 0.5` - stop at halfway point

```haskell
d1 $ s "amen" # begin 0.25 # end 0.75   -- play middle 50% of sample
d1 $ s "amen" # begin 0.75              -- play only last quarter
```

#### Cut (Voice Choking)

`cut` creates groups where only one voice can play at a time:

```haskell
d1 $ s "bd*4" # cut 1   -- each new bd cuts off the previous one
d1 $ s "hh*16" # cut 2  -- hi-hats choke each other (typical for closed hats)
```

Cut groups are global across all patterns.

**Not the same as duration control!** Cut is for voice stealing, not for controlling how long a sample plays.

## Common Misconceptions

### ❌ "legato 1 should not change sample duration"

**Wrong.** `legato 1` means "play for exactly the event duration". If your events are 0.125 seconds, samples will be cut off at 0.125 seconds.

**Correct behavior**: If you want samples to play their full length, **don't specify legato at all**.

### ❌ "speed -1 doesn't work"

**Wrong.** `speed -1` should absolutely work and play samples backwards. This is a fundamental feature in Tidal Cycles.

**Implementation requirement**: The audio engine must:
1. Detect negative speed values
2. Reverse the audio buffer
3. Play at the absolute value of speed

### ❌ "loopAt stretches the sample without changing pitch"

**Wrong.** `loopAt` uses speed adjustment, which DOES change pitch. If you want time-stretching without pitch change, use a different parameter or algorithm (like Paulstretch).

**Correct behavior**: `loopAt 2` on a 1-second sample at 1 cps will play at 0.5x speed (pitched down one octave).

### ❌ "cut controls sample duration"

**Wrong.** `cut` is for voice stealing/choking, not duration control.

**Correct**: Use `legato`, `sustain`, or envelope parameters for duration control.

## Implementation Checklist

To correctly implement Tidal Cycles sample behavior:

- [ ] **Legato default behavior**
  - [ ] When legato is NOT specified: samples play full duration
  - [ ] When legato IS specified: calculate `sustain = delta * legato`
  - [ ] Apply envelope to cut sample at sustain time

- [ ] **Speed parameter**
  - [ ] Positive values: adjust playback rate normally
  - [ ] Negative values: reverse buffer + play at abs(speed)
  - [ ] Zero speed: no playback
  - [ ] Pitch changes logarithmically with speed

- [ ] **LoopAt function**
  - [ ] Calculate: `speed = sampleDuration / (targetCycles / tempo)`
  - [ ] Set `unit "c"` internally
  - [ ] This WILL change pitch (it's not time-stretching)

- [ ] **Sustain parameter**
  - [ ] Direct control in seconds
  - [ ] Independent of pattern timing
  - [ ] Applies envelope to cut at specified time

- [ ] **Cut parameter**
  - [ ] Global cut groups (across all patterns)
  - [ ] New voice in same group stops previous voice
  - [ ] NOT for duration control

## Testing Requirements

To verify correct implementation:

### Test 1: Legato Default Behavior
```
Pattern: s "bd sn hh cp"
Expected: All samples play their full length (overlapping if samples are long)
```

### Test 2: Legato 1
```
Pattern: s "bd sn hh cp" $ legato 1
Expected: Each sample plays for exactly 0.25 cycles (cuts off at event boundary)
```

### Test 3: Legato > 1
```
Pattern: s "bd sn" $ legato 4
Expected: Each sample plays for 2 cycles (massive overlap)
```

### Test 4: Legato < 1
```
Pattern: s "bd sn hh cp" $ legato 0.1
Expected: Very short staccato notes with silence between
```

### Test 5: Negative Speed
```
Pattern: s "amen" # speed "-1"
Expected: "amen" sample plays backwards at normal pitch
```

### Test 6: Negative Speed with Rate Change
```
Pattern: s "amen" # speed "-0.5"
Expected: "amen" plays backwards at half speed (lower pitch)
```

### Test 7: LoopAt
```
Pattern: loopAt 2 $ s "amen"
Expected: "amen" stretched/compressed to fit exactly 2 cycles (pitch adjusted)
```

### Test 8: Speed Zero
```
Pattern: s "bd" # speed 0
Expected: No sound (or error/warning)
```

## References

- [Tidal Cycles Legato Documentation](https://github.com/tidalcycles/tidalcycles.github.io/blob/master/_functions/synth_parameters/legato.md)
- [Tidal Cycles Sampling Reference](https://tidalcycles.org/docs/reference/sampling/)
- [Strudel Issue #111: Fix legato and duration](https://github.com/tidalcycles/strudel/issues/111)
- [Strudel sampler.mjs implementation](https://codeberg.org/uzu/strudel/src/branch/main/packages/superdough/sampler.mjs)
- [SuperDirt legato behavior discussion](https://github.com/musikinformatik/SuperDirt/issues/89)

## Conclusion

The key insight is that **Tidal Cycles prioritizes musical flexibility**:

- Samples default to playing their full length (natural behavior)
- `legato` provides rhythmic control when needed
- `speed` provides pitch/duration control together
- Negative speed enables reverse playback (essential for live coding)
- Parameters compose predictably

Phonon must match this behavior to be compatible with Tidal Cycles patterns and user expectations.
