# Rhythm Pattern Research: SuperCollider & Tidal Cycles

**Task**: research-supercollider-rhythm
**Date**: 2026-01-28

## Executive Summary

This research examines rhythm pattern systems in SuperCollider and Tidal Cycles to identify patterns Phonon should adopt. **Key finding**: Phonon already has excellent coverage (~75% of Tidal's rhythm features), but gaps exist in harmonic/scale support, polymeter, and sample-level manipulation.

---

## 1. SuperCollider Pattern System

### 1.1 Core Pattern Classes

SuperCollider's pattern library has **120+ classes**. The most relevant for rhythm:

| Pattern | Description | Phonon Equivalent |
|---------|-------------|-------------------|
| **Pseq** | Sequential list playback | `cat`, mini-notation sequences |
| **Prand** | Random choice from list | `choose`, `randcat` |
| **Pxrand** | Random without repeats | Not implemented |
| **Pshuf** | Shuffle once, repeat | `scramble` (per-query) |
| **Pwrand** | Weighted random | `wchoose`, `wrand_cat` |
| **Pstutter/Pdup** | Repeat each value n times | `stutter`, `ply` |
| **Pseries** | Arithmetic sequence | `run` (partial) |
| **Pgeom** | Geometric sequence | Not implemented |
| **Pwhite** | Uniform random | `rand()` |
| **Pbrown** | Brownian motion | `walk` |

### 1.2 Time-Based Patterns

| Pattern | Description | Phonon Equivalent |
|---------|-------------|-------------------|
| **Ptime** | Elapsed time since start | Not exposed |
| **Pstep** | Sample-and-hold for duration | Not implemented |
| **Pseg** | Interpolate values over time | Continuous patterns (sine, etc.) |

**Key Insight**: SuperCollider distinguishes between *list-based* patterns (iterate through values) and *time-based* patterns (calculate from elapsed time). Phonon's query model naturally supports time-based behavior through its `TimeSpan` system.

### 1.3 Parallel Patterns (Polyrhythm)

| Pattern | Description | Phonon Equivalent |
|---------|-------------|-------------------|
| **Ppar** | Start all patterns simultaneously | `stack`, `,` mini-notation |
| **Ptpar** | Parallel with time offsets | `late` + `stack` |
| **Pgpar** | Parallel with group allocation | Bus system |
| **Pspawner** | Dynamic pattern spawning | Not implemented |

### 1.4 Euclidean Rhythms (Bjorklund)

SuperCollider's **Pbjorklund** quark provides:
- `Bjorklund(k, n)` - Returns binary array of k pulses in n steps
- `Pbjorklund(k, n, length, offset)` - Pattern version
- `Pbjorklund2(k, n)` - Returns duration ratios instead of binary

**Phonon Status**: ✅ Full support via `euclid(k, n, rotation)` function and `(k,n)` mini-notation.

### 1.5 Swing Implementation

SuperCollider implements swing through **Pchain** filter pattern:

```supercollider
// Core parameters:
swingBase: 0.25,      // Base subdivision (16ths)
swingAmount: 1/3,     // Delay fraction
swingThreshold: 0.05  // Ignore non-duple divisions

// Logic: Delay weak beats (odd divisions) by swingAmount
```

**Phonon Status**: ✅ `swing` function implemented with similar semantics.

---

## 2. Tidal Cycles Pattern System

### 2.1 Mini-Notation (Inherited from Bol Processor)

| Symbol | Meaning | Phonon Status |
|--------|---------|---------------|
| `~` | Rest/silence | ✅ |
| `[ ]` | Grouping/subdivision | ✅ |
| `*` | Repeat/multiply | ✅ |
| `/` | Slow down | ✅ |
| `_` | Elongate | ❌ Not implemented |
| `@` | Elongate (alt) | ❌ Not implemented |
| `.` | Top-level separator | ❌ Not implemented |
| `,` | Simultaneous (polyrhythm) | ✅ |
| `< >` | Alternation per cycle | ✅ |
| `{ }` | Polymeter | ❌ Not implemented |
| `(k,n)` | Euclidean | ✅ |
| `(k,n,o)` | Euclidean + offset | ✅ |
| `\|` | Random choice | ❌ Not implemented |
| `!` | Replicate | ❌ Not implemented |
| `?` | Random removal | ❌ (use `degrade`) |
| `:` | Sample variant | ✅ `bd:0 bd:1` |

### 2.2 Sample Manipulation Functions

| Function | Description | Phonon Status |
|----------|-------------|---------------|
| **chop n** | Cut sample into n parts | ⚠️ Pattern-level only |
| **striate n** | Interlace n parts | ⚠️ Pattern-level only |
| **striateBy n len** | Striate with part length | ❌ Not implemented |
| **slice n pat** | Rearrange slices by pattern | ✅ `slice_pattern` |
| **splice n pat** | Slice with pitch correction | ❌ Not implemented |
| **loopAt n** | Fit sample to n cycles | ✅ `loop_at` |
| **begin/end** | Sample boundaries | ⚠️ DSP-level only |
| **speed** | Playback rate | ✅ Pattern-controlled |

**Key Gap**: Phonon's `chop`/`striate` work at pattern level but don't integrate with sample playback timing the way Tidal's do.

### 2.3 Time Transformation Functions

| Function | Description | Phonon Status |
|----------|-------------|---------------|
| **fast n** | Speed up by n | ✅ |
| **slow n** | Slow down by n | ✅ |
| **hurry n** | Fast + pitch | ✅ |
| **fastGap n** | Fast with gap | ✅ `fast_gap` |
| **compress s e** | Fit to time range | ✅ |
| **zoom s e** | Extract and stretch | ✅ |
| **within s e f** | Apply f in range | ✅ |
| **inside n f** | Scale time, apply f | ✅ |
| **outside n f** | Inverse of inside | ✅ |
| **rotL n** | Rotate left in time | ✅ `rotate_left` |
| **rotR n** | Rotate right in time | ✅ `rotate_right` |
| **press** | Delay by half slot | ✅ |
| **pressBy n** | Delay by n fraction | ✅ |

### 2.4 Structural Functions

| Function | Description | Phonon Status |
|----------|-------------|---------------|
| **every n f** | Apply f every n cycles | ✅ |
| **sometimes f** | 50% chance | ✅ |
| **rarely/often/etc** | Probability variants | ✅ |
| **mask pat** | Apply boolean mask | ✅ |
| **struct pat** | Apply structure | ✅ `struct_pattern` |
| **ply n** | Repeat each event | ✅ |
| **linger n** | Hold first n events | ✅ |
| **bite n pat** | Select from n slices | ✅ |
| **chew n pat** | Offset through pattern | ✅ |
| **iter n** | Iterate with shifts | ✅ |
| **rev** | Reverse | ✅ |
| **palindrome** | Forward then backward | ✅ |
| **jux f** | Stereo juxtaposition | ✅ |
| **superimpose f** | Layer with transform | ✅ |
| **layer fs** | Multiple transforms | ✅ |
| **ghost** | Add ghost notes | ✅ |

---

## 3. Gap Analysis: What Phonon Should Add

### 3.1 High Priority (Rhythm Core)

#### A. Mini-Notation Enhancements
```
| Symbol | Priority | Complexity |
|--------|----------|------------|
| `_`/@  | High     | Medium     | Elongation
| `{}`   | High     | High       | Polymeter
| `|`    | Medium   | Low        | Random choice
| `!`    | Medium   | Low        | Replicate
| `.`    | Low      | Low        | Separator
```

#### B. Polymeter Support
Tidal's `{a b c, d e}` plays 3-step pattern against 2-step pattern. This requires:
- New AST node for polymeter
- Modified query logic to handle different "meters"
- Synchronization tracking

#### C. Sample-Integrated Chopping
Current `chop`/`striate` modify pattern events but don't coordinate with sample `begin`/`end`:

```rust
// Current: Pattern-level only
fn chop(n: usize) -> Pattern<T>

// Needed: Coordinate with sample playback
fn chop_sample(n: usize) -> Pattern<SampleEvent> {
    // Set begin/end for each slice
    // Maintain timing across slices
}
```

### 3.2 Medium Priority (Advanced Features)

#### A. Pstep-style Time Patterns
SuperCollider's `Pstep` holds values for durations:
```supercollider
Pstep([1, 2, 3], [0.5, 0.25, 0.25])  // 1 for 0.5 beats, 2 for 0.25, etc.
```

**Phonon Implementation**:
```rust
fn step<T>(levels: Vec<T>, durs: Vec<Fraction>) -> Pattern<T> {
    // Query returns level based on accumulated duration
}
```

#### B. Pattern-Controlled Time Parameters
Most Phonon time functions accept constants:
```rust
fn fast(self, factor: f64) -> Self  // ❌ Current
fn fast(self, factor: Pattern<f64>) -> Self  // ✅ Should be
```

This is noted in CLAUDE.md as a core principle but not fully implemented for all time functions.

#### C. Pxrand (No Consecutive Repeats)
```rust
fn xrand<T: PartialEq>(options: Vec<T>) -> Pattern<T> {
    // Track previous value, exclude from choices
}
```

### 3.3 Lower Priority (Nice to Have)

- **Pseries/Pgeom**: Arithmetic/geometric sequences
- **Pspawner**: Dynamic pattern spawning
- **Pattern introspection**: Count events, trace values
- **Caching/memoization**: Performance optimization

---

## 4. Recommendations

### 4.1 Immediate Actions

1. **Add mini-notation elongation (`_` and `@`)** - Users expect this for sustained notes
2. **Document pattern-controlled time params** - Clarify which functions accept patterns vs constants
3. **Create sample-chopping integration tests** - Verify chop/striate work correctly with samples

### 4.2 Short-Term (Next Sprint)

1. **Implement polymeter `{}`** - Critical for complex polyrhythmic music
2. **Add `|` (random choice)** - Very common in live coding
3. **Integrate chop/striate with sample begin/end** - Full Tidal parity

### 4.3 Medium-Term

1. **Pattern-controlled time functions** - Make all time params accept `Pattern<f64>`
2. **Pstep-style held patterns** - Useful for step sequencing
3. **Harmonic/scale support** - See separate research task

---

## 5. Sources

### SuperCollider
- [Pattern Guide 02: Basic Vocabulary](https://doc.sccode.org/Tutorials/A-Practical-Guide/PG_02_Basic_Vocabulary.html)
- [Pattern Guide 03: What Is Pbind](https://doc.sccode.org/Tutorials/A-Practical-Guide/PG_03_What_Is_Pbind.html)
- [Pattern Guide 06d: Parallel Patterns](https://doc.sccode.org/Tutorials/A-Practical-Guide/PG_06d_Parallel_Patterns.html)
- [Pattern Guide 06b: Time Based Patterns](https://doc.sccode.org/Tutorials/A-Practical-Guide/PG_06b_Time_Based_Patterns.html)
- [Pattern Guide Cookbook 08: Swing](https://doc.sccode.org/Tutorials/A-Practical-Guide/PG_Cookbook08_Swing.html)
- [Pbjorklund Class Reference](http://doc.sccode.org/Classes/Pbjorklund.html)

### Tidal Cycles
- [Mini Notation Reference](https://tidalcycles.org/docs/reference/mini_notation/)
- [Sampling Functions](https://tidalcycles.org/docs/reference/sampling/)
- [Course I Tutorial](https://tidalcycles.org/docs/patternlib/tutorials/course1/)
- [Tidal Club Forum - Chop/Striate](https://club.tidalcycles.org/t/week-3-lesson-4-chop-and-striate/534)

### Academic
- Toussaint, G. "The Euclidean Algorithm Generates Traditional Musical Rhythms" (2005)
- Roberts, C. "Bringing the TidalCycles Mini-Notation to the Browser" (WAC 2019)

---

## 6. Appendix: Phonon Implementation Files

| Feature | Primary File | Lines |
|---------|--------------|-------|
| Core patterns | `src/pattern.rs` | ~1200 |
| Pattern ops | `src/pattern_ops.rs` | 822 |
| Extended ops | `src/pattern_ops_extended.rs` | 1638 |
| Structure ops | `src/pattern_structure.rs` | 686 |
| Mini-notation | `src/mini_notation_v3.rs` | ~800 |
| Tonal (minimal) | `src/pattern_tonal.rs` | ~200 |
