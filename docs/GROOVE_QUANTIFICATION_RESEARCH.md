# Groove and Feel Quantification Research

This document summarizes research on groove quantification methods for potential implementation in Phonon.

---

## Table of Contents

1. [What is Groove?](#what-is-groove)
2. [Academic Research Findings](#academic-research-findings)
3. [DAW Groove Template Algorithms](#daw-groove-template-algorithms)
4. [Live Coding Implementations (Tidal/Strudel/SuperCollider)](#live-coding-implementations)
5. [Genre-Specific Patterns](#genre-specific-patterns)
6. [Phonon's Current Implementation](#phonons-current-implementation)
7. [Recommendations for Phonon](#recommendations-for-phonon)

---

## What is Groove?

Groove is defined as **wanting to move the body to music** - the invisible thread that makes a beat come alive. It consists of:

1. **Microtiming deviations (MTDs)**: Small, intentional timing variations from a strict grid
2. **Velocity patterns**: Dynamic variations in loudness/intensity
3. **Swing/shuffle**: Systematic delay of metrically-weaker notes
4. **Push/pull feel**: Playing slightly ahead or behind the beat

### Key Insight

Human performers introduce temporal variability consisting of both long-range tempo changes and note-to-note level deviations from nominal beat time. These micro-timing variations are important for achieving preferred characteristics like "hang," "drive," or "groove."

---

## Academic Research Findings

### Microtiming Measurement Methods

Researchers quantify microtiming using three components:

1. **Systematic Variability (SV)**: Recurrent temporal patterns (genre-specific feel)
2. **Residual Variability (RV)**: Unexplained deviations (humanization/randomness)
3. **Root Mean Squared Error (RMSE)**: Summary deviation measure

**Typical ranges**: 5ms to 50ms in groove music, with 20ms being typical for funk/samba and up to 40ms for jazz.

### Swing Ratio Research

The swing ratio varies significantly with tempo:
- **Slow tempi (< 100 BPM)**: Swing ratio as high as 3.5:1
- **Medium tempi (~130 BPM)**: True triplet feel around 2:1 (67%)
- **Fast tempi (> 200 BPM)**: Approaches 1:1 (straight)

The "triplet feel" (2:1) only occurs at certain tempos - it's not universal.

### Controversial Findings

Research shows **conflicting results** about whether microtiming enhances groove:
- Early studies claimed MTDs enhance groove (participatory discrepancies theory)
- Later research found that **quantized** patterns often rate higher for groove
- Resolution: The effect depends on **genre familiarity** and listener expertise

Key finding: **Slightly delayed downbeats and synchronized offbeats of a soloist with respect to a rhythm section enhance swing** - small push/pull relationships matter more than random deviations.

### Sources

- [Microtiming Deviations and Swing Feel in Jazz (Nature Scientific Reports)](https://www.nature.com/articles/s41598-019-55981-3)
- [The Effect of Microtiming Deviations on the Perception of Groove](https://www.researchgate.net/publication/236893338_The_Effect_of_Microtiming_Deviations_on_the_Perception_of_Groove_in_Short_Rhythms)
- [Swingogram representation for tracking micro-rhythmic variation](https://www.tandfonline.com/doi/full/10.1080/09298215.2017.1367405)

---

## DAW Groove Template Algorithms

### How Groove Templates Work

Groove templates are "quantization maps" derived from real performances. They capture:

1. **Timing deviations**: When each hit lands relative to the grid
2. **Velocity patterns**: How loud each hit is
3. **Duration information**: How long notes sustain

### Ableton Live Groove Pool

Ableton's algorithm uses **position-based timing**:

```
For each note in the groove template at position P:
  deviation = actual_time(P) - grid_time(P)

When applying to a clip:
  For each note in clip at position P:
    note.time += deviation(P) * (Timing_parameter / 100)
    note.velocity = lerp(note.velocity, groove_velocity(P), Velocity_parameter / 100)
```

**Key Parameters**:
- **Base**: Rhythmic resolution (1/4, 1/8, 1/16)
- **Timing**: 0-100% strength of timing deviation application
- **Velocity**: -100 to +100% velocity deviation application
- **Random**: Amount of additional random humanization
- **Quantize**: How much to quantize to the groove template

### MPC Swing Algorithm

MPC swing is simpler - it only delays **even-numbered 16th notes**:

```
swing_percentage = 50 to 75  (50 = no swing, 67 = triplet, 75 = max)

For each 16th note at position i:
  if i is even (2, 4, 6, 8...):
    delay = (swing_percentage - 50) / 100 * sixteenth_note_duration
    note.time += delay
```

At 50%: No delay (straight)
At 67%: Triplet feel (2:1 ratio)
At 75%: Maximum swing (3:1 ratio, the limit before it sounds wrong)

### Sources

- [Ableton Manual: Using Grooves](https://www.ableton.com/en/manual/using-grooves/)
- [Quantisation & Groove Functions In Logic](https://www.soundonsound.com/techniques/quantisation-groove-functions-logic)
- [MPC Groove Template Tutorial (peff.com)](https://www.peff.com/journal/2006/01/28/mpc-groove-template-tutorial/)

---

## Live Coding Implementations

### TidalCycles

**nudge**: Shifts pattern timing by a pattern of values

```haskell
d1 $ s "bd*4" # nudge "[0 0.04]*4"  -- Alternating tight/loose
```

**swingBy**: Breaks cycle into n slices, delays events in second half of each slice

```haskell
swingBy :: Pattern Time -> Pattern Time -> Pattern a -> Pattern a
swingBy x n pat = ...

-- x = 0: no swing
-- x = 0.5: delay by half the note duration
-- x = 1: wraps to no effect

d1 $ swingBy 0.1 4 $ s "bd sn hh cp"  -- Swing on 4 slices
```

### Strudel (JavaScript)

Similar to Tidal:

```javascript
// nudge shifts pattern by cycles
s("bd*4").nudge("[0 0.05]*4")

// swingBy same as Tidal
s("bd sn hh cp").swingBy(0.1, 4)
```

### SuperCollider Swing

The SuperCollider implementation is more sophisticated:

```supercollider
// Key parameters
swingBase = 0.25    // Base subdivision (16ths = 0.25)
swingAmount = 0.1   // How much to delay (fraction of swingBase)
swingThreshold = 0.05 // Tolerance for recognizing grid positions

// Algorithm:
// 1. Check if note is near the swing grid
isNearGrid = abs(now - now.round(swingBase)) <= swingThreshold

// 2. Check if it's an off-beat position
isOffBeat = (now / swingBase).round.asInteger.odd

// 3. Apply delay only to off-beats near the grid
if (isNearGrid && isOffBeat) {
  timingOffset = swingBase * swingAmount
}

// 4. Compensate durations
// - Shorten swung notes if next note is not swung
// - Lengthen non-swung notes if next note is swung
```

The threshold prevents applying swing to triplets or other non-duple subdivisions.

### Sources

- [TidalCycles nudge documentation](https://userbase.tidalcycles.org/nudge.html)
- [Strudel Time Modifiers](https://strudel.cc/learn/time-modifiers/)
- [SuperCollider Pattern Guide Cookbook 08: Swing](https://doc.sccode.org/Tutorials/A-Practical-Guide/PG_Cookbook08_Swing.html)

---

## Genre-Specific Patterns

### Jazz Swing

- Swing ratio varies from 1:1 (fast) to 3.5:1 (slow)
- Soloists typically swing less than expected (ratio < 1.5)
- Key: **slight downbeat delays** enhance swing feel
- MTD magnitude larger and more variable than rock

### Funk

- Studied pattern: James Brown's "Funky Drummer"
- Ghost notes crucial for feel
- Typical MTD: around 20ms
- Strong emphasis on velocity accents

### Samba

- Third and fourth 16th notes played **early** (not late like swing)
- Ghost notes create "suingue"
- Velocity patterns essential

### Hip-Hop (J Dilla style)

- Intentional "flawed timing"
- Some layers quantized, others deliberately off
- Mix of machine precision and human feel
- Uses MPC-style quantization artifacts as feature

### General Velocity Patterns

Ghost notes (velocity 15-30 on 0-127 scale) between main hits create groove:
- Hi-hats: Every other 16th at 30-40% velocity
- Snare ghost notes: 30-40% velocity before/after backbeat
- Kick accents: First of a double-kick lighter than second

---

## Phonon's Current Implementation

### Existing Functions

**`swing(amount)`** (src/pattern_ops_extended.rs:317-351):
- Delays every **odd-indexed event** by `amount` cycles
- Simple index-based: `if i % 2 == 1 { delay(amount) }`
- **Limitation**: Doesn't consider metric position, just event order

**`shuffle(amount)`** (src/pattern_ops_extended.rs:354-395):
- Adds random timing variation to all events
- Seeded per-cycle for determinism
- Range: ±amount

**`humanize(time_var, velocity_var)`** (src/pattern_ops_extended.rs:398-400):
- Currently just calls `shuffle(time_var)`
- **Does not actually use velocity_var** (placeholder)

**`late(amount)` / `early(amount)`** (src/pattern_ops.rs:28-101):
- Shifts entire pattern in time
- Pattern-controlled: amount can vary per cycle
- Foundation for other timing operations

**`nudge(amount)`** (src/unified_graph.rs:7977):
- Graph-level timing adjustment
- Works in both live and render modes

### Current Limitations

1. **swing** is index-based, not metric-position-based
2. **humanize** doesn't affect velocity (parameter ignored)
3. No **swingBy** function (Tidal's subdivision-aware swing)
4. No **groove template** support (cannot import/export grooves)
5. No **velocity pattern** transforms
6. No genre-specific presets

---

## Recommendations for Phonon

### Priority 1: Fix/Enhance Existing Functions

**1.1 Implement proper `swingBy(amount, subdivisions)`**

```rust
/// Subdivision-aware swing
/// swingBy 0.1 4  -- swing on quarter-note grid
/// swingBy 0.2 8  -- swing on 8th-note grid
pub fn swing_by(self, amount: Pattern<f64>, subdivisions: Pattern<f64>) -> Self {
    Pattern::new(move |state| {
        let cycle_start = state.span.begin.to_float().floor();

        // Get parameters
        let amt = query_first_value(&amount, cycle_start);
        let subs = query_first_value(&subdivisions, cycle_start) as i32;

        let haps = self.query(state);
        haps.into_iter().map(|mut hap| {
            let pos = hap.part.begin.to_float();
            let grid_pos = (pos * subs as f64).round() as i32;

            // Only delay odd grid positions (off-beats)
            if grid_pos % 2 == 1 {
                let delay = amt / subs as f64;
                hap.part = shift_timespan(hap.part, delay);
            }
            hap
        }).collect()
    })
}
```

**1.2 Implement velocity in `humanize`**

```rust
pub fn humanize(self, time_var: Pattern<f64>, velocity_var: Pattern<f64>) -> Self {
    self.shuffle(time_var)
        .map_context(|ctx, rng| {
            let vel_var = query_first_value(&velocity_var, ctx.cycle);
            let current_vel = ctx.get("velocity").unwrap_or(1.0);
            let variation = rng.gen_range(-vel_var..vel_var);
            ctx.insert("velocity", (current_vel + variation).clamp(0.0, 1.0));
        })
}
```

### Priority 2: New Groove Functions

**2.1 `groove(template)` - Apply a groove template**

```rust
pub struct GrooveTemplate {
    name: String,
    base: f64,              // 0.25 for 16ths, 0.5 for 8ths
    timing: Vec<f64>,       // Deviation per position
    velocity: Vec<f64>,     // Velocity per position
}

pub fn groove(self, template: GrooveTemplate, strength: f64) -> Self {
    // Apply template deviations scaled by strength
}
```

**2.2 Built-in groove presets**

```phonon
-- Usage
~drums $ s "bd sn hh cp" $ groove "mpc60" 0.7
~drums $ s "bd sn hh cp" $ groove "jdilla" 1.0
~drums $ s "bd sn hh cp" $ groove "samba" 0.8
```

**2.3 `pushPull(amount)` - Ahead/behind beat feel**

```rust
/// Shift entire pattern to feel "ahead" (negative) or "behind" (positive) the beat
/// pushPull -0.02  -- slightly ahead, urgent feel
/// pushPull 0.03   -- slightly behind, laid-back feel
pub fn push_pull(self, amount: Pattern<f64>) -> Self {
    self.late(amount)
}
```

### Priority 3: Velocity/Accent Patterns

**3.1 `accent(positions, amount)`**

```phonon
-- Accent on 1 and 3
~drums $ s "bd sn hh cp" $ accent "1 0 1 0" 0.3
```

**3.2 `ghost(positions, velocity)`**

```phonon
-- Add ghost notes at 20% velocity
~drums $ s "bd sn hh cp" $ ghost "0 1 0 1" 0.2
```

### Priority 4: Groove Extraction

**4.1 Extract groove from pattern**

```rust
pub fn extract_groove(pattern: &Pattern<String>, subdivisions: usize) -> GrooveTemplate {
    // Query pattern over one cycle
    // Build timing/velocity arrays from event positions
}
```

This would allow:
```phonon
-- Record a performance, extract its groove, apply to other patterns
~live $ midi_record ...
~groove # extract_groove ~live 16
~synth $ saw 55 $ groove ~groove 0.8
```

### Implementation Priority Order

1. **swingBy** - Most commonly needed, Tidal-compatible
2. **Velocity in humanize** - Currently broken
3. **pushPull** - Simple alias but expressive
4. **accent/ghost** - High value for expressiveness
5. **Groove templates** - Larger feature, high impact
6. **Groove extraction** - Advanced feature

### Example DSL Usage (Target)

```phonon
-- Swing with subdivision control
~drums $ s "bd sn hh*4 cp" $ swingBy 0.12 4

-- Push/pull feel (laid back)
~bass $ saw 55 $ pushPull 0.02

-- Velocity accents
~hats $ s "hh*8" $ accent "1 0 0.5 0" 0.4

-- Full humanization with velocity
~perc $ s "cp*4" $ humanize 0.02 0.15

-- Genre preset
~drums $ s "bd sn hh cp" $ groove "mpc60" 0.8

-- Combine techniques
~beat $ s "bd sn hh*4 cp"
    $ swingBy 0.1 4
    $ humanize 0.01 0.1
    $ accent "1 0 0.7 0" 0.2
```

---

## Summary

Groove quantification involves:

1. **Timing**: Systematic deviations (swing) + random deviations (humanize)
2. **Velocity**: Accent patterns + ghost notes
3. **Feel**: Push/pull relationship to the beat

The research shows that:
- Groove is genre-specific (jazz ≠ funk ≠ samba)
- Too much randomness hurts groove (quantized often better)
- Subtle velocity patterns matter as much as timing
- Swing ratio varies with tempo (not fixed at triplet)

Phonon already has the foundation with `swing`, `shuffle`, `late/early`. The main gaps are:
- Subdivision-aware swing (`swingBy`)
- Working velocity modulation
- Groove templates
- Genre presets
