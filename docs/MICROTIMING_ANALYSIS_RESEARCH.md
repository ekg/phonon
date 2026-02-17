# Microtiming Analysis in Electronic Music

## Research Summary for Phonon

This document synthesizes academic research and industry practices on microtiming in electronic music production, with specific recommendations for the Phonon live-coding language.

---

## 1. What is Microtiming?

**Definition**: Minute timing deviations from strict metronomic regularity, typically on the order of tens of milliseconds (10-50ms).

Microtiming arises from two sources:
1. **Human motor limitations** - natural timing imprecision
2. **Intentional stylistic expression** - deliberate "feel" adjustments

Key insight: Even untrained listeners are highly sensitive to timing deviations as small as 2.5% of beat length (approximately 12.5ms at 120 BPM).

---

## 2. Scientific Findings

### 2.1 Perception Thresholds

| Tempo (BPM) | Beat Duration | JND (2.5%) | Subliminal (~1.5ms) |
|-------------|---------------|------------|---------------------|
| 60          | 1000ms        | 25ms       | detectable          |
| 90          | 667ms         | 16.7ms     | detectable          |
| 120         | 500ms         | 12.5ms     | detectable          |
| 140         | 428ms         | 10.7ms     | detectable          |

**Just Noticeable Difference (JND)**: 2.5% for trained musicians, 4.4% for non-musicians.

### 2.2 The Groove Paradox

Research reveals a surprising finding:
- **Quantized patterns often rated more "groovy"** than patterns with microtiming deviations
- Increasing microtiming deviations generally **decreases** perceived groove quality
- Early shifts rated **more negatively** than late shifts
- Deviations on **snare drum** rated worse than on **bass drum**

However:
- **Very small deviations (1-2%)** can improve groove
- Expert listeners show **increased body movement** with subtle microtiming (not fully quantized)
- The effect is genre-dependent

### 2.3 Optimal Conditions

- **Optimal tempo for groove**: 100-120 BPM
- **Typical microtiming range**: 0-50ms displacement
- **"Laid-back" snare**: Average delay of 17.4ms at 96 BPM

---

## 3. Roger Linn's Swing Implementation (MPC Standard)

Roger Linn, creator of the LM-1, LinnDrum, and Akai MPC, defined the industry standard for swing:

### 3.1 The Algorithm

```
Swing delays every EVEN-numbered 16th note within each 8th note.
```

Swing percentage represents the ratio of time between paired 16th notes:

| Swing % | Meaning | Feel |
|---------|---------|------|
| 50%     | Equal timing (1:1) | Straight, no swing |
| 54%     | Slight push | Loosens feel without sounding swung |
| 58%     | Light swing | Subtle bounce |
| 62%     | Medium swing | Danceable groove |
| 66%     | Perfect triplet (2:1) | Classic shuffle |
| 70%     | Heavy swing | Very loose feel |

### 3.2 Key Insight

> "Between 50% and around 70% are lots of wonderful little settings that, for a particular beat and tempo, can change a rigid beat into something that makes people move."
> — Roger Linn

The "magic" is in the in-between values, not the exact triplet ratio.

---

## 4. Genre-Specific Approaches

### 4.1 Electronic Dance Music (EDM)

EDM challenges traditional microtiming theory:
- Dominates dance floors despite grid-based, quantized rhythms
- Groove achieved through **sonic features** rather than timing:
  - Attack/release shaping
  - Timbre manipulation
  - Frequency content
  - Dynamic envelope

**Key finding**: Altering a sound's microstructure changes its **perceived temporal location**, even without changing actual onset time.

### 4.2 Hip-Hop (J Dilla Style)

J Dilla's signature "tipsy" feel comes from:
- Using hardware samplers (MPC 3000, SP1200)
- Leaving "mistakes" in beats
- Combining mechanical loop repetition with intentional swing
- Off-grid playing until it "swings the right way"

### 4.3 Jazz/Funk

- Active use of Participatory Discrepancies (PDs)
- Microtiming creates ensemble "conversation"
- Expert listeners respond with increased body movement

---

## 5. Ghost Notes

### 5.1 Definition
Subtle drum hits between main beats that drive the groove forward, felt more than heard.

### 5.2 Velocity Guidelines

| Level | MIDI Velocity | Use |
|-------|---------------|-----|
| Accent | 110-127 | Hard hits |
| Loud | 100-110 | Main backbeats |
| Normal | 90-100 | Standard hits |
| Quiet | 80-90 | Softer hits |
| Ghost | 10-70 | Ghost notes |

**Typical ghost note velocity**: 20-50 (approximately 15-40% of main hit)

### 5.3 Tonal Consideration

Ghost notes should have **different tonality**, not just lower volume:
- Darker timbre
- Different articulation
- Less attack transient

---

## 6. Current Phonon Implementation

### 6.1 Existing Features

**Swing** (`pattern.rs:317`):
```rust
pub fn swing(self, amount: Pattern<f64>) -> Self
// Delays odd-indexed events by the specified amount
```

Note: Phonon's swing delays **odd-indexed** events (1, 3, 5...), while MPC-style swing delays **even-numbered 16th notes** within each 8th note pair. Both achieve similar results for standard 16th-note patterns.

**Ghost Notes** (`pattern.rs:1209`):
```rust
pub fn ghost(self) -> Self
// Adds copies at 1/8 and 1/16 cycle offsets
pub fn ghost_with(self, offset1: f64, offset2: f64) -> Self
```

**Humanize** (`test_humanize_within_euclid.rs`):
```phonon
out $ "bd sn hh cp" $ humanize 0.1 0.2
// Parameters: timing variation, velocity variation
```

### 6.2 Gaps and Recommendations

1. **Ghost note velocity/gain control**
   - Current ghost notes don't reduce velocity
   - Recommendation: Add `ghostGain` parameter or integrate with ValueMap

2. **MPC-style swing percentage**
   - Convert Phonon's cycle-fraction swing to MPC percentage
   - Add `swingMPC 58` syntax for familiar workflow

3. **Groove templates**
   - Pre-defined swing/timing patterns extracted from classic grooves
   - `groove "mpc60"` or `groove "dilla"`

4. **Nudge per-instrument**
   - Apply different timing offsets to kick vs snare vs hi-hat
   - `nudgeSnare 0.02` for laid-back snare feel

---

## 7. Analysis Tools for Phonon

### 7.1 Potential Integrations

**Aubio** (real-time capable):
- Onset detection
- Beat tracking
- Tempo estimation
- MIDI note extraction

**Librosa** (Python, batch processing):
- Onset strength envelope
- Beat tracking
- Tempo estimation
- Spectral analysis

### 7.2 Microtiming Analysis Features

Suggested analysis tool outputs:
1. **Timing deviation histogram** - Distribution of note timings vs grid
2. **Swing ratio calculator** - Measured swing percentage
3. **Per-instrument timing** - Separate analysis for each drum element
4. **Velocity profile** - Dynamic contour visualization

---

## 8. Implementation Recommendations

### 8.1 High Priority

1. **Add velocity/gain to ghost notes**
   ```phonon
   -- Current: ghost adds copies at fixed offsets
   -- Proposed: ghostWith offset1 offset2 gain1 gain2
   out $ s "sn" $ ghostWith 0.125 0.0625 0.3 0.5
   ```

2. **MPC-style swing percentage syntax**
   ```phonon
   -- Convert percentage to cycle fraction
   -- 66% = triplet swing = delay by 1/6 of 8th note
   out $ s "bd sn hh cp" $ swingMPC 62
   ```

3. **Per-event timing offset (nudge)**
   ```phonon
   -- Delay snare hits for laid-back feel
   out $ s "bd sn hh cp" # nudge "0 0.02 0 0"
   ```

### 8.2 Medium Priority

4. **Groove templates**
   ```phonon
   -- Apply extracted groove from classic recordings
   out $ s "bd sn hh*4 cp" $ groove "mpc3000_58"
   ```

5. **Humanize with per-instrument control**
   ```phonon
   -- Different humanization for each element
   ~drums $ s "bd sn hh" # humanize "0.05 0.1 0.02"
   ```

### 8.3 Lower Priority

6. **Sonic microtiming (attack shaping)**
   - Shape attack envelopes to affect perceived timing
   - Longer attacks = perceived later onset

7. **Analysis tools**
   - Extract groove from audio files
   - Visualize microtiming deviations

---

## 9. References

### Academic Sources

- Frühauf, J., Kopiez, R., & Platz, F. (2013). [Music on the timing grid: The influence of microtiming on perceived groove quality](https://www.researchgate.net/publication/237423294_Music_on_the_timing_grid_The_influence_of_microtiming_on_the_perceived_groove_quality_of_a_simple_drum_pattern_performance)
- [Microtiming in Swing and Funk affects body movement behavior](https://pmc.ncbi.nlm.nih.gov/articles/PMC4542135/)
- [Shaping rhythm: timing and sound in five groove-based genres](https://www.cambridge.org/core/journals/popular-music/article/shaping-rhythm-timing-and-sound-in-five-groovebased-genres/BBC410F9849DB982AEBFACEA14D38F32)
- [A Grid in Flux: Sound and Timing in Electronic Dance Music](https://www.researchgate.net/publication/363269048_A_Grid_in_Flux_Sound_and_Timing_in_Electronic_Dance_Music)
- [21st Century Funk: A Microtiming Analysis of J Dilla's Beats](https://www.academia.edu/24528600/21st_Century_Funk_A_Microtiming_Analysis_of_the_Beats_of_Hip_Hop_Producer_J_Dilla)
- [Microtiming Deviations and Swing Feel in Jazz](https://www.nature.com/articles/s41598-019-55981-3)
- [TIME Project: Timing and Sound in Musical Microrhythm](https://www.uio.no/ritmo/english/projects/time/)

### Industry Sources

- [Roger Linn on Swing, Groove & The Magic of the MPC's Timing](https://www.attackmagazine.com/features/interview/roger-linn-swing-groove-magic-mpc-timing)
- [How to Use Swing Rhythms in Music Production](https://blog.faderpro.com/techniques/how-to-use-swing-rhythms-in-music-production/)
- [Using Ghost Notes to Add Groove and Feel](https://blog.faderpro.com/arrangement/using-ghost-notes-to-add-groove-and-feel-to-your-drums/)
- [7 Drum Programming Tips to Improve Any Groove](https://www.loopcloud.com/cloud/blog/5254-7-Drum-Programming-Tips-to-Improve-Any-Groove)

### Tools

- [Aubio: Audio and Music Analysis Library](https://aubio.org/)
- [Librosa: Audio and Music Signal Analysis in Python](https://librosa.org/)
- [Tidal Cycles Documentation](https://tidalcycles.org/docs/)

---

## 10. Conclusion

Microtiming in electronic music is a nuanced domain where:

1. **Less is often more** - Quantized patterns can be groovier than heavily swung ones
2. **Context matters** - Optimal swing varies with tempo and genre
3. **Sound shapes time** - Sonic characteristics affect perceived timing
4. **In-between values are magic** - The sweet spots are between 50% and 66% swing

For Phonon, the priority should be:
- Making ghost notes properly quiet (velocity control)
- Adding MPC-style swing percentage for familiar workflow
- Enabling per-instrument timing offsets for professional groove control

The goal is not to replicate human imperfection, but to provide the **expressive control** that allows musicians to shape time intentionally.
