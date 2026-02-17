# Test Pattern DSL Design

**Status**: Design Document
**Author**: Claude
**Date**: 2026-01-28

## Executive Summary

This document proposes a domain-specific language (DSL) for expressing test expectations in Phonon. The goal is to eliminate boilerplate, make tests more readable, and enforce the three-level testing methodology consistently.

## Problem Statement

Current Phonon tests suffer from:

1. **Boilerplate**: Same `State { span: TimeSpan::new(...) }` pattern repeated 200+ times
2. **Scattered assertions**: No consistent language for "expect N events with pattern X"
3. **Cycle patterns hard to express**: No concise way to say "cycles should be [8, 4, 8, 4]"
4. **Magic numbers**: Tolerances like `0.01`, `0.005`, `1.5` scattered without explanation
5. **Level mixing**: Tests mix verification levels without clear separation

## Design Goals

1. **Concise**: Reduce typical test from 50 lines to 5-10
2. **Readable**: Tests should read like specifications
3. **Structured**: Enforce three-level methodology
4. **Type-safe**: Leverage Rust's type system for correctness
5. **Expressive**: Support cycle patterns, probabilistic tests, comparisons

---

## The DSL: `phonon_test!`

### Core Syntax

```rust
phonon_test! {
    name: "every_2_fast_2",
    pattern: "a b c d" $ every 2 (fast 2),

    // Level 1: Pattern query verification
    level1: {
        cycles: 8,
        events_per_cycle: [8, 4, 8, 4, 8, 4, 8, 4],
    },

    // Level 2: Audio onset verification
    level2: {
        tempo: 0.5,
        onset_ratio: (1.2, 2.0),  // vs base pattern
    },

    // Level 3: Audio characteristics
    level3: {
        rms: (0.01, 1.0),
        peak: (_, 0.95),  // _ means "don't care about lower bound"
    },
}
```

### Why This Syntax?

1. **Declarative**: Express what to verify, not how
2. **All three levels visible**: At a glance, see what's being tested
3. **Named expectations**: `events_per_cycle` is clearer than raw assertions
4. **Range syntax**: `(min, max)` or `(_, max)` for bounds

---

## Module Structure

```
src/test_dsl/
├── mod.rs           # Main macro definitions
├── level1.rs        # Pattern query verification
├── level2.rs        # Onset detection
├── level3.rs        # Audio characteristics
├── assertions.rs    # Assertion builders
├── matchers.rs      # Flexible matching (ranges, patterns, etc.)
└── render.rs        # DSL rendering utilities
```

---

## Level 1: Pattern Query DSL

### Basic Event Count

```rust
phonon_level1! {
    pattern: "a b c d",
    cycles: 8,
    total_events: 32,
}
```

**Expands to**:
```rust
#[test]
fn test_pattern_level1() {
    let pattern = parse_mini_notation("a b c d");
    let mut total = 0;
    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total += pattern.query(&state).len();
    }
    assert_eq!(total, 32);
}
```

### Per-Cycle Event Pattern

```rust
phonon_level1! {
    pattern: "a b" $ every 2 (fast 2),
    cycles: 8,
    events_per_cycle: [4, 2, 4, 2, 4, 2, 4, 2],
}
```

### Event Values Verification

```rust
phonon_level1! {
    pattern: "a b c d" $ rotL 0.25,
    cycle: 0,
    expect_values: ["b", "c", "d", "a"],
}
```

### Timing Verification

```rust
phonon_level1! {
    pattern: "a b c d",
    cycle: 0,
    event_times: [0.0, 0.25, 0.5, 0.75],
    tolerance: 0.001,
}
```

### Pattern Comparison

```rust
phonon_level1! {
    base: "a b c d",
    modified: "a b c d" $ fast 2,
    cycles: 8,
    event_ratio: 2.0,
}
```

---

## Level 2: Onset Detection DSL

### Basic Onset Count

```rust
phonon_level2! {
    code: r#"
        tempo: 0.5
        out $ s "bd sn hh cp"
    "#,
    cycles: 8,
    onsets: (28, 36),  // Expected range
    threshold: 0.01,
}
```

### Comparative Onset Analysis

```rust
phonon_level2! {
    base: r#"
        tempo: 0.5
        out $ s "bd sn"
    "#,
    modified: r#"
        tempo: 0.5
        out $ s "bd sn" $ fast 2
    "#,
    cycles: 8,
    onset_ratio: (1.8, 2.2),  // ~2x expected
}
```

### Timing Verification

```rust
phonon_level2! {
    code: r#"
        tempo: 0.5
        out $ s "bd"
    "#,
    cycles: 4,
    expected_onset_times: [0.0, 2.0, 4.0, 6.0],  // in seconds
    timing_tolerance: 0.1,
}
```

---

## Level 3: Audio Characteristics DSL

### Basic Signal Analysis

```rust
phonon_level3! {
    code: r#"
        tempo: 0.5
        out $ s "bd sn"
    "#,
    cycles: 4,
    rms: (0.01, 0.5),
    peak: (_, 0.95),  // only upper bound
}
```

### Spectral Analysis

```rust
phonon_level3! {
    code: r#"
        tempo: 0.5
        out $ saw 440 # lpf 500 0.8
    "#,
    cycles: 1,
    spectral_centroid: (200.0, 800.0),  // Hz
    dominant_frequency: (430.0, 450.0),  // Hz
}
```

### Comparative Spectral Analysis

```rust
phonon_level3! {
    low_filter: r#"
        out $ saw 440 # lpf 500 0.8
    "#,
    high_filter: r#"
        out $ saw 440 # lpf 5000 0.8
    "#,
    assert: spectral_centroid(high_filter) > spectral_centroid(low_filter),
}
```

---

## Combined Three-Level Tests

The `phonon_test!` macro combines all three levels:

```rust
phonon_test! {
    name: "fast_transform",

    // The pattern being tested
    pattern: "a b c d" $ fast 2,

    // Level 1: Pattern query verification
    level1: {
        cycles: 8,
        events_per_cycle: repeating![8],  // All cycles have 8 events
        total_events: 64,
    },

    // Level 2: Audio onset verification
    level2: {
        code: r#"
            tempo: 0.5
            out $ s "bd sn hh cp" $ fast 2
        "#,
        cycles: 8,
        onset_ratio_vs_base: (1.8, 2.2),
        base_code: r#"
            tempo: 0.5
            out $ s "bd sn hh cp"
        "#,
    },

    // Level 3: Audio characteristics
    level3: {
        rms: (0.01, _),  // At least 0.01
        peak: (_, 0.95),  // At most 0.95
    },
}
```

---

## Specialized Test Types

### Probabilistic Tests (for `sometimes`, `degrade`, etc.)

```rust
phonon_probabilistic! {
    name: "sometimes_50_percent",
    pattern: "a b c d" $ sometimes (fast 2),
    cycles: 100,

    // Over 100 cycles, expect roughly 50% to have fast applied
    expect: {
        fast_cycles: proportion(0.4, 0.6),  // 40-60%
        total_ratio_vs_base: (1.3, 1.7),    // ~1.5x events
    },

    // Seed for reproducibility
    seed: 12345,
}
```

### Cycle Pattern Tests (for `every`, `whenmod`, etc.)

```rust
phonon_cycle_pattern! {
    name: "every_3_rev",
    pattern: "a b c d" $ every 3 rev,
    cycles: 12,

    // Express the expected pattern
    expect_pattern: {
        // Every 3rd cycle (0, 3, 6, 9) has reversed order
        cycle_values: {
            0: ["d", "c", "b", "a"],
            1: ["a", "b", "c", "d"],
            2: ["a", "b", "c", "d"],
            // Pattern repeats...
        },
        period: 3,
    },
}
```

### Time Transform Tests (for `rotL`, `rotR`, `zoom`, etc.)

```rust
phonon_time_transform! {
    name: "rotL_quarter",
    base: "a b c d",
    transform: rotL 0.25,

    expect: {
        // First event of transformed should be second event of base
        first_value: "b",
        // Values should cycle
        values: ["b", "c", "d", "a"],
        // Timing should be preserved (just shifted)
        event_spacing: uniform(0.25),
    },
}
```

### Modulation Tests (for audio-rate patterns)

```rust
phonon_modulation! {
    name: "lfo_filter_sweep",
    code: r#"
        ~lfo $ sine 2
        out $ saw 110 # lpf (~lfo * 2000 + 500) 0.8
    "#,
    duration: 2.0,  // seconds

    expect: {
        // Spectral centroid should vary over time
        spectral_centroid: varies_over_time(0.5),  // period in seconds
        // Range of variation
        centroid_range: (300.0, 2500.0),
    },
}
```

---

## Matcher Types

### Range Matchers

```rust
// Exact value
exact(42)           // == 42

// Range (inclusive)
range(1.0, 2.0)     // >= 1.0 && <= 2.0

// Unbounded (one-sided)
at_least(0.01)      // >= 0.01
at_most(0.95)       // <= 0.95

// Tuple syntax (min, max) - _ for unbounded
(0.01, 0.95)        // >= 0.01 && <= 0.95
(0.01, _)           // >= 0.01
(_, 0.95)           // <= 0.95
```

### Sequence Matchers

```rust
// Exact sequence
[8, 4, 8, 4]        // Exact match

// Repeating pattern
repeating![8, 4]    // [8, 4, 8, 4, 8, 4, ...]

// All same value
repeating![8]       // [8, 8, 8, 8, ...]

// Alternating
alternating![8, 4]  // Same as repeating![8, 4]
```

### Probabilistic Matchers

```rust
// Proportion (for probabilistic patterns)
proportion(0.4, 0.6)    // 40-60% of the time

// Distribution
normal(mean: 4.0, stddev: 1.0)  // Approximately normal distribution
```

### Comparative Matchers

```rust
// Compare two values
greater_than(other_value)
less_than(other_value)
approximately(value, tolerance)

// Ratio comparison
ratio(expected: 2.0, tolerance: 0.2)  // 1.8 to 2.2
```

---

## Implementation Strategy

### Phase 1: Core Infrastructure

1. **Assertion builders**: Type-safe assertion generation
2. **Pattern query helpers**: Simplify cycle iteration
3. **Audio rendering utilities**: Standard `render_dsl` with options

```rust
// src/test_dsl/assertions.rs
pub struct Assertion<T> {
    actual: T,
    context: String,
}

impl<T: PartialOrd + Display> Assertion<T> {
    pub fn is_between(self, min: T, max: T) -> Result<(), String> { ... }
    pub fn is_at_least(self, min: T) -> Result<(), String> { ... }
    pub fn equals(self, expected: T) -> Result<(), String> { ... }
}

// Usage
assert_that!(rms).is_between(0.01, 1.0)?;
assert_that!(events.len()).equals(32)?;
```

### Phase 2: Level-Specific Macros

```rust
// src/test_dsl/level1.rs
pub fn query_cycles<T: Clone>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> Vec<Vec<Hap<T>>> {
    (0..cycles)
        .map(|c| query_cycle(pattern, c))
        .collect()
}

pub fn query_cycle<T: Clone>(
    pattern: &Pattern<T>,
    cycle: usize,
) -> Vec<Hap<T>> {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

// Macro for pattern assertions
#[macro_export]
macro_rules! assert_cycle_events {
    ($pattern:expr, $expected:expr) => {
        let cycles = $expected.len();
        for (cycle, expected_count) in $expected.iter().enumerate() {
            let events = query_cycle(&$pattern, cycle);
            assert_eq!(
                events.len(),
                *expected_count,
                "Cycle {} expected {} events, got {}",
                cycle,
                expected_count,
                events.len()
            );
        }
    };
}
```

### Phase 3: Full DSL Macros

```rust
// src/test_dsl/mod.rs
#[macro_export]
macro_rules! phonon_level1 {
    (
        pattern: $pattern:expr,
        cycles: $cycles:expr,
        events_per_cycle: [$($count:expr),+ $(,)?],
    ) => {
        let pattern = parse_mini_notation($pattern);
        let expected = vec![$($count),+];
        assert_cycle_events!(pattern, expected);
    };

    (
        pattern: $pattern:expr,
        cycles: $cycles:expr,
        total_events: $total:expr,
    ) => {
        let pattern = parse_mini_notation($pattern);
        let total: usize = (0..$cycles)
            .map(|c| query_cycle(&pattern, c).len())
            .sum();
        assert_eq!(total, $total);
    };
}
```

### Phase 4: Integration & Documentation

1. Convert existing tests to new DSL (one file at a time)
2. Add examples and documentation
3. Create test template generator

---

## Migration Path

### Before (current style):

```rust
#[test]
fn test_every_level1_cycle_pattern() {
    let base_pattern = "a b c d";
    let pattern = parse_mini_notation(base_pattern);

    let mut total_events_per_cycle = Vec::new();

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        if cycle % 2 == 0 {
            let fast_pattern = pattern.clone().fast(Pattern::pure(2.0));
            let events = fast_pattern.query(&state);
            total_events_per_cycle.push(events.len());
        } else {
            let events = pattern.query(&state);
            total_events_per_cycle.push(events.len());
        }
    }

    assert_eq!(total_events_per_cycle[0], 8);
    assert_eq!(total_events_per_cycle[1], 4);
    assert_eq!(total_events_per_cycle[2], 8);
    // ... more assertions
}
```

### After (with DSL):

```rust
phonon_level1! {
    pattern: "a b c d" $ every 2 (fast 2),
    cycles: 8,
    events_per_cycle: [8, 4, 8, 4, 8, 4, 8, 4],
}
```

**Reduction**: 50 lines → 5 lines (90% reduction)

---

## Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Test macro | `phonon_<level>!` | `phonon_level1!`, `phonon_test!` |
| Assertion macro | `assert_<thing>!` | `assert_cycle_events!` |
| Helper function | `snake_case` | `query_cycle`, `detect_onsets` |
| Matcher | `snake_case` | `at_least`, `proportion` |
| Type | `PascalCase` | `EventComparison`, `Assertion` |

---

## Error Messages

Good error messages are crucial for debugging. The DSL should produce:

```
test_every_2_fast_2::level1 FAILED

Pattern: "a b c d" $ every 2 (fast 2)
Expected events per cycle: [8, 4, 8, 4, 8, 4, 8, 4]
Actual events per cycle:   [8, 4, 8, 4, 6, 4, 8, 4]
                                        ^ Cycle 4: expected 8, got 6

Context:
  - Cycle 0-3: ✓ matched
  - Cycle 4: ✗ expected 8 events, got 6
  - Cycle 5-7: ✓ matched
```

---

## Future Extensions

### 1. Property-Based Testing Integration

```rust
phonon_property! {
    // For any pattern P and any n > 0:
    // fast(n).slow(n) == identity
    forall pattern: Pattern<String>,
    forall n: f64 in (0.1, 10.0),

    assert: pattern.fast(n).slow(n) == pattern
}
```

### 2. Visual Diff for Audio

```rust
phonon_level2! {
    // ... test config ...
    on_failure: save_waveform_diff("test_output/"),
}
```

### 3. Regression Test Generation

```rust
// Capture current behavior as test
phonon_snapshot! {
    name: "every_behavior",
    pattern: "a b c d" $ every 2 (fast 2),
    cycles: 8,
    // Automatically records current behavior as expected
}
```

---

## Summary

The proposed test pattern DSL:

1. **Reduces boilerplate by 80-90%** in typical tests
2. **Enforces three-level methodology** structurally
3. **Makes tests readable as specifications**
4. **Provides rich matchers** for complex assertions
5. **Generates helpful error messages**

Implementation can proceed incrementally:
1. Start with `query_cycle` helper and `assert_cycle_events!` macro
2. Add `phonon_level1!` macro
3. Add `phonon_level2!` and `phonon_level3!` macros
4. Combine into `phonon_test!` macro
5. Migrate existing tests

---

## Appendix A: Quick Reference

```rust
// Level 1: Pattern events
phonon_level1! {
    pattern: "a b c d",
    cycles: 8,
    total_events: 32,
}

phonon_level1! {
    pattern: "a b c d" $ fast 2,
    cycles: 8,
    events_per_cycle: repeating![8],
}

// Level 2: Audio onsets
phonon_level2! {
    code: r#"tempo: 0.5; out $ s "bd sn""#,
    cycles: 8,
    onsets: (14, 18),
}

// Level 3: Audio characteristics
phonon_level3! {
    code: r#"out $ saw 440"#,
    cycles: 1,
    rms: (0.1, _),
    peak: (_, 0.95),
}

// Combined
phonon_test! {
    name: "my_test",
    pattern: "a b c d",
    level1: { cycles: 8, total_events: 32 },
    level2: { onsets: (28, 36) },
    level3: { rms: (0.01, _) },
}
```

## Appendix B: DSL Grammar (EBNF-like)

```ebnf
phonon_test     = "phonon_test!" "{" test_body "}"
test_body       = "name:" string ","
                  ["pattern:" pattern_expr ","]
                  ["level1:" level1_block ","]
                  ["level2:" level2_block ","]
                  ["level3:" level3_block ","]

level1_block    = "{" level1_field ("," level1_field)* "}"
level1_field    = "cycles:" number
                | "total_events:" number
                | "events_per_cycle:" sequence
                | "expect_values:" string_sequence

level2_block    = "{" level2_field ("," level2_field)* "}"
level2_field    = "code:" raw_string
                | "cycles:" number
                | "onsets:" range
                | "onset_ratio:" range
                | "threshold:" number

level3_block    = "{" level3_field ("," level3_field)* "}"
level3_field    = "rms:" range
                | "peak:" range
                | "spectral_centroid:" range
                | "dominant_frequency:" range

range           = "(" (number | "_") "," (number | "_") ")"
sequence        = "[" number ("," number)* "]"
                | "repeating!" "[" number ("," number)* "]"
string_sequence = "[" string ("," string)* "]"
```
