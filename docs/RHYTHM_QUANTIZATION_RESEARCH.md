# Rhythm Quantization and Swing Analysis

Research document for Phonon's rhythm manipulation capabilities.

## Executive Summary

Phonon already has solid rhythm/groove foundations with `swing`, `shuffle`, `ghost`, and various timing transforms. This research identifies opportunities to align more closely with industry-standard groove implementations (MPC-style) and adds context for potential future enhancements.

---

## Current State in Phonon

### Existing Rhythm/Timing Functions

| Function | Location | Description | Status |
|----------|----------|-------------|--------|
| `swing(amount)` | `pattern_ops_extended.rs:317` | Delays every odd-indexed event | ✅ Working |
| `shuffle(amount)` | `pattern_ops_extended.rs:354` | Random timing jitter | ✅ Working |
| `ghost` / `ghostWith` | `pattern.rs:1217` | Add ghost notes | ✅ Working |
| `nudge` | DSL | Time offset | ✅ Working |
| `late` / `early` | DSL | Shift pattern in time | ✅ Working |
| `rotL` / `rotR` | `pattern.rs` | Rotate pattern | ✅ Working |
| `press` / `pressBy` | DSL | Delay by slot fraction | ✅ Working |
| `discretise(n)` | `pattern_structure.rs:451` | Sample pattern at N points | ✅ Working |
| `quantize(steps)` | `pattern_ops_extended.rs:857` | Quantize numeric values | ✅ Working |

### Phonon's `swing` Implementation

```rust
// pattern_ops_extended.rs:317-351
pub fn swing(self, amount: Pattern<f64>) -> Self {
    Pattern::new(move |state: &State| {
        let swing_amount = /* query amount pattern */;
        let haps = self.query(state);
        haps.into_iter()
            .enumerate()
            .map(|(i, mut hap)| {
                if i % 2 == 1 {  // Delay odd-indexed events
                    let shift = Fraction::from_float(swing_amount);
                    hap.part = TimeSpan::new(hap.part.begin + shift, hap.part.end + shift);
                }
                hap
            })
            .collect()
    })
}
```

**Key Characteristic**: Phonon's swing delays every OTHER event by a fixed cycle fraction.

---

## Industry Standards

### 1. MPC/Linn Swing (The "Classic" Approach)

Roger Linn invented drum machine swing for the LM-1 and later MPC series.

**Algorithm**:
- Delays the **second 16th note within each 8th note pair**
- Expressed as a percentage ratio between the two 16ths

**Swing Percentages**:
| Percentage | Feel | Mathematical Ratio |
|------------|------|-------------------|
| 50% | Straight (no swing) | 1:1 |
| 54% | Slight looseness | ~1.08:0.92 |
| 62% | Groovy feel | ~1.24:0.76 |
| 66% | Perfect triplet swing | 2:1 (triplet) |
| 70% | Heavy swing | ~1.4:0.6 |

**Linn's Quote**: "The fun comes in the in-between settings. For example, a 90 BPM swing groove will feel looser at 62% than at a perfect swing setting of 66%."

**Hardware**: LM-1 had six LEDs labeled 50% to 70% in 4% increments.

### 2. TidalCycles `swingBy`

```haskell
swingBy :: Pattern Time -> Pattern Time -> Pattern a -> Pattern a
```

**Semantics**:
- Breaks each cycle into `n` slices
- Delays events in the **second half of each slice** by amount `x`
- `x` is relative to half-slice size (0 = no effect, 0.5 = half note, 1 = wrap around)

**Example**:
```haskell
d1 $ swingBy (1/3) 4 $ sound "hh*8"
-- Breaks cycle into 4 slices
-- Each slice has 2 hi-hats
-- Second hi-hat in each slice is delayed by 1/3 of half-slice
```

**Alias**: `swing = swingBy (1/3)` (triplet feel shorthand)

### 3. Strudel Implementation

```javascript
s("hh*8").swingBy(1/3, 4)  // Same as Tidal
s("hh*8").swing(4)         // Shorthand for swingBy(1/3, 4)
```

**Parameters**:
- `offset` (number): delay amount relative to half-slice (0-1)
- `subdivision` (number): number of slices per cycle

---

## Gap Analysis: Phonon vs Tidal/MPC

### Phonon's Current Approach

Phonon's `swing(amount)` simply delays every odd-indexed event by `amount` cycles:
- Event 0: no delay
- Event 1: +amount
- Event 2: no delay
- Event 3: +amount
- etc.

**Pros**:
- Simple mental model
- Pattern-controllable amount
- Works with any number of events

**Cons**:
- Not subdivision-aware (doesn't know about 16ths vs 8ths)
- Different from MPC percentage system
- Different from Tidal's slice-based approach

### Tidal/Strudel's `swingBy(amount, subdivision)`

- Subdivision-aware: knows how to break cycles
- Amount is relative to slice size
- More musically intuitive for traditional swing

### MPC Percentage System

- Industry standard for groove
- Intuitive for musicians ("67% swing")
- Maps directly to triplet/shuffle feel

---

## Recommendations

### Option 1: Add `swingBy` (Tidal Compatibility)

Add a subdivision-aware swing to complement existing `swing`:

```rust
/// Tidal-compatible swingBy
/// Breaks cycle into n slices, delays events in second half of each slice
pub fn swing_by(self, amount: Pattern<f64>, n: Pattern<f64>) -> Self {
    Pattern::new(move |state: &State| {
        let amt = /* query amount */;
        let slices = /* query n */;

        let haps = self.query(state);
        haps.into_iter()
            .map(|mut hap| {
                let pos_in_cycle = hap.part.begin.to_float().fract();
                let slice_size = 1.0 / slices;
                let pos_in_slice = (pos_in_cycle % slice_size) / slice_size;

                // If in second half of slice, apply swing
                if pos_in_slice >= 0.5 {
                    let shift = amt * slice_size * 0.5;
                    hap.part = TimeSpan::new(
                        hap.part.begin + Fraction::from_float(shift),
                        hap.part.end + Fraction::from_float(shift)
                    );
                }
                hap
            })
            .collect()
    })
}
```

**DSL Syntax**:
```phonon
~drums $ s "hh*8" $ swingBy 0.33 4
```

### Option 2: Add MPC-Style Groove Percentage

Add `groove` or `mpcSwing` with percentage semantics:

```rust
/// MPC-style groove with percentage (50-75)
/// 50% = straight, 66% = triplet, 75% = heavy swing
pub fn groove(self, percent: Pattern<f64>, subdivision: Pattern<f64>) -> Self {
    // Convert percentage to timing ratio
    // 50% = 0.5:0.5, 66% = 0.66:0.34
    let ratio = percent / 100.0;
    let delay = (ratio - 0.5) * 2.0;  // 0.0 at 50%, 1.0 at 100%
    // Apply to second note of each pair...
}
```

**DSL Syntax**:
```phonon
~drums $ s "hh*8" $ groove 62 16  -- 62% swing on 16ths
~drums $ s "hh*4" $ groove 66 8   -- triplet swing on 8ths
```

### Option 3: Groove Templates

Pre-defined groove patterns extracted from classic machines:

```rust
// Predefined grooves
pub const MPC_60: GrooveTemplate = ...;
pub const MPC_3000: GrooveTemplate = ...;
pub const LM1: GrooveTemplate = ...;
pub const TR808: GrooveTemplate = ...;
```

**DSL Syntax**:
```phonon
~drums $ s "bd sn hh*4 cp" $ applyGroove "mpc60" 0.8
```

### Option 4: Quantize Strength (DAW-Style)

Add partial quantization for humanized input:

```rust
/// Quantize with adjustable strength
/// strength 1.0 = full quantize, 0.0 = no change
pub fn quantize_timing(self, grid: f64, strength: Pattern<f64>) -> Self {
    Pattern::new(move |state: &State| {
        let str_val = /* query strength */;
        let haps = self.query(state);
        haps.into_iter()
            .map(|mut hap| {
                let original = hap.part.begin.to_float();
                let quantized = (original / grid).round() * grid;
                let new_pos = original + (quantized - original) * str_val;
                hap.part = TimeSpan::new(
                    Fraction::from_float(new_pos),
                    hap.part.end + Fraction::from_float(new_pos - original)
                );
                hap
            })
            .collect()
    })
}
```

**Use Case**: When recording MIDI input, allow partial quantization.

---

## Priority Recommendations

### High Priority

1. **Add `swingBy` for Tidal compatibility**
   - Musicians familiar with Tidal expect this
   - More musically meaningful than current `swing`
   - Relatively easy to implement

### Medium Priority

2. **Add MPC-style percentage groove**
   - Industry standard
   - Intuitive for producers
   - Could be syntactic sugar over `swingBy`

3. **Partial timing quantization**
   - Useful for MIDI recording workflows
   - Already have `quantize` for values, extend to timing

### Lower Priority

4. **Groove templates**
   - Nice-to-have for authentic vintage feel
   - Requires research to capture authentic grooves
   - Could be community-contributed

---

## Technical Notes

### Pattern vs Audio-Rate

Phonon's strength is that patterns are evaluated at sample rate. For groove:
- **Pattern-level swing** (current): Affects event timing, simple
- **Audio-rate swing** (possible): Could modulate playback speed in real-time

Current pattern-level approach is correct for groove/swing.

### Interaction with `fast`/`slow`

Consider how swing interacts with time scaling:
```phonon
s "hh*8" $ swing 0.1 $ fast 2  -- Should swing happen before or after fast?
```

Current implementation: swing operates on query results, so `fast` affects timing first.

### Fractional Precision

Phonon uses `Fraction` for timing precision. Swing calculations should preserve fraction accuracy where possible to avoid drift.

---

## References

- [TidalCycles Time Reference](https://tidalcycles.org/docs/reference/time/)
- [Strudel Time Modifiers](https://strudel.cc/learn/time-modifiers/)
- [Roger Linn on Swing & Groove](https://melodiefabriek.com/blog/roger-linns-shuffle/)
- [MPC Groove Template Tutorial](https://www.peff.com/journal/2006/01/28/mpc-groove-template-tutorial/)
- [MIDI Quantization Explained](https://blog.landr.com/quantization-in-music/)
- [TidalCycles Club - Swing Discussion](https://club.tidalcycles.org/t/generalizing-swing-and-rotating-uneven-rhythms-by-mapping-integers-from-a-latent-space-to-time/4991)

---

## Appendix: Phonon Test Coverage

Existing tests in `tests/test_transform_swing.rs`:
- ✅ Level 1: Pattern query verification (timing shifts)
- ✅ Level 2: Onset detection (audio events)
- ✅ Level 3: Audio characteristics (signal quality)
- ✅ Edge cases (zero swing, single event, value preservation)

The test infrastructure is excellent and should be replicated for any new groove functions.
