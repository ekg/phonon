# Tidal Cycles Deep Dive: How Pattern Evaluation Really Works

**Purpose**: Document the canonical Tidal Cycles pattern evaluation model to ensure Phonon implements it correctly.

**Date**: 2025-10-27

---

## Core Concepts

### 1. Patterns Are Functions From Time To Events

**Tidal Definition**:
```haskell
type Query a = State -> [Event a]
data Pattern a = Pattern { query :: Query a }
```

A pattern is NOT a pre-computed sequence of events. It's a **function** that you call with a timespan, and it returns the events that occur during that timespan.

**Key Insight**: Patterns are evaluated **lazily and on-demand** for specific time windows.

### 2. Time is Rational, Not Float

**Tidal uses rational numbers** (ratios of integers) to represent time:
- `1/3` is exactly one-third (not `0.333333...`)
- `2/5` is exactly two-fifths
- This ensures **precise musical subdivisions** without floating-point error

**Why This Matters**:
- No accumulating timing errors
- Polyrhythms work perfectly (e.g., 3 against 5)
- Cycles align exactly across pattern transformations

### 3. The Event Structure: Whole vs Part

```haskell
data Event a = Event {
  whole :: Maybe Arc,  -- The complete timespan of the event
  part :: Arc,         -- The portion relevant to this query
  value :: a           -- The event's value (sample name, frequency, etc.)
}

type Arc = (Time, Time)  -- (begin, end)
type Time = Rational     -- Exact rational time
```

**Critical Distinction**:
- **`whole`**: The ENTIRE duration the event conceptually occupies
- **`part`**: The INTERSECTION of `whole` with the query span

**Example**:
```
Query span: [0.5, 1.5)
Event whole: [0.0, 1.0)
Event part:  [0.5, 1.0)   <-- Only the overlapping portion!
```

This allows events to "bleed" across query boundaries while only returning the relevant portion.

### 4. Cycles Are Numbered from 0

- Cycle 0: [0.0, 1.0)
- Cycle 1: [1.0, 2.0)
- Cycle 2: [2.0, 3.0)
- etc.

Cycles are **half-open intervals**: `[begin, end)` includes begin but excludes end.

---

## Pattern Evaluation Model

### How Queries Work

**Step 1**: Create a State with a timespan (arc) and controls
```haskell
state = State {
  span: Arc (Fraction 0 1) (Fraction 1 1),  -- Query cycle 0
  controls: HashMap.empty
}
```

**Step 2**: Call pattern.query(state)
```haskell
events = pattern.query(state)
-- Returns: [Event { whole, part, value }, ...]
```

**Step 3**: Process returned events
- Each event has `whole` (logical span) and `part` (actual span to render)
- Events may have `part` that's a subset of `whole`
- Events outside the query span are NOT returned

### Example: Querying "bd sn hh cp"

**Pattern**: `"bd sn hh cp"` (4 events per cycle)

**Query Cycle 0** `[0.0, 1.0)`:
```
Event 1: whole=[0.00, 0.25), part=[0.00, 0.25), value="bd"
Event 2: whole=[0.25, 0.50), part=[0.25, 0.50), value="sn"
Event 3: whole=[0.50, 0.75), part=[0.50, 0.75), value="hh"
Event 4: whole=[0.75, 1.00), part=[0.75, 1.00), value="cp"
```

**Query Cycle 1** `[1.0, 2.0)`:
```
Event 1: whole=[1.00, 1.25), part=[1.00, 1.25), value="bd"
Event 2: whole=[1.25, 1.50), part=[1.25, 1.50), value="sn"
Event 3: whole=[1.50, 1.75), part=[1.50, 1.75), value="hh"
Event 4: whole=[1.75, 2.00), part=[1.75, 2.00), value="cp"
```

**Key Point**: Same pattern, different cycle query → different absolute times, same structure.

---

## Transform Semantics

### `fast n` - Speed Up Pattern

**Definition**: Makes pattern repeat `n` times faster (squeezed into less time)

**Example**: `"bd sn" $ fast 2`

Original pattern over 1 cycle:
```
[0.0-0.5): bd
[0.5-1.0): sn
```

After `fast 2` (2 repetitions in 1 cycle):
```
[0.00-0.25): bd  (first rep)
[0.25-0.50): sn
[0.50-0.75): bd  (second rep)
[0.75-1.00): sn
```

**Implementation**: `fast n` queries the original pattern with a timespan **divided by n**, then **multiplies** the resulting event times by **1/n**.

### `slow n` - Slow Down Pattern

**Definition**: Makes pattern repeat `n` times slower (stretched over more time)

**Example**: `"bd sn hh cp" $ slow 2`

Original (1 cycle):
```
[0.00-0.25): bd, [0.25-0.50): sn, [0.50-0.75): hh, [0.75-1.00): cp
```

After `slow 2` (stretched over 2 cycles):
```
Cycle 0:
[0.00-0.50): bd
[0.50-1.00): sn

Cycle 1:
[1.00-1.50): hh
[1.50-2.00): cp
```

**Implementation**: `slow n` queries with timespan **multiplied by n**, then **divides** event times by **1/n**.

### `<a b c>` - Alternation (slowcat)

**Definition**: Cycles through patterns one per cycle

**Example**: `sound "<bd sn hh>"`

```
Cycle 0: bd
Cycle 1: sn
Cycle 2: hh
Cycle 3: bd  (wraps around)
```

**Implementation**:
1. Determine which cycle we're in: `cycle = floor(query_begin)`
2. Select pattern: `pattern_idx = cycle % num_patterns`
3. Query that pattern only
4. Return events from selected pattern

**CRITICAL**: Only ONE pattern is active per cycle, not all of them!

---

## Euclidean Rhythms with Pattern Arguments

### How `bd(<3 7>, <8 9>)` Should Work

**Pattern**: `bd(<3 7>, <8 9>)`
- Pulses alternates: 3, 7, 3, 7, ...
- Steps alternates: 8, 9, 8, 9, ...

**Cycle 0**: `pulses=3, steps=8` → `X..X..X.` (Bjorklund(3,8))
**Cycle 1**: `pulses=7, steps=9` → `X.X.X.X.X.X.X..` (Bjorklund(7,9))
**Cycle 2**: `pulses=3, steps=8` → `X..X..X.` (same as cycle 0)
**Cycle 3**: `pulses=7, steps=9` → `X.X.X.X.X.X.X..` (same as cycle 1)

**Key Point**: The alternation happens PER CYCLE. Within a single cycle, the parameters are constant.

---

## Common Pitfalls & Anti-Patterns

### ❌ DON'T: Query at sample rate

**Wrong**:
```rust
// Every sample (44100 times/second)
let state = State { span: [cycle_pos, cycle_pos + 1/44100), ... };
let events = pattern.query(state);
```

**Why it's wrong**: Patterns aren't designed for microsecond queries. You'll get duplicate events, missing events, and poor performance.

### ✅ DO: Query at cycle boundaries or larger spans

**Right**:
```rust
// Query full cycle at start
let state = State { span: [0.0, 1.0), ... };
let events = pattern.query(state);

// Then process events during rendering
for event in events {
    if event_starts_in_current_sample_window(&event) {
        trigger_sample(&event);
    }
}
```

### ❌ DON'T: Create new patterns on every query

**Wrong**:
```rust
Pattern::new(move |state| {
    let euclid = Pattern::euclid(3, 8, 0);  // NEW pattern every query!
    euclid.query(state)
})
```

**Why it's wrong**: If you query this pattern multiple times for the same cycle, you get different pattern instances → duplicate events.

### ✅ DO: Cache patterns per-cycle or create once

**Right**:
```rust
// Create pattern once
let euclid = Pattern::euclid(3, 8, 0);

Pattern::new(move |state| {
    euclid.query(state)  // Reuse same pattern
})
```

---

## Phonon vs Tidal: Current Gaps

### Gap 1: Sample-Rate Querying ⚠️

**Tidal**: Queries patterns at cycle or larger spans
**Phonon**: Queries patterns at sample rate (44100 Hz)

**Impact**:
- Performance overhead
- Potential duplicate events
- Not idiomatic to pattern evaluation model

**Fix Needed**: Query patterns once per cycle, cache events, trigger during render loop.

### Gap 2: Float vs Rational Time ⚠️

**Tidal**: Uses `Rational` (exact fractions)
**Phonon**: Uses `f64` (floating point)

**Impact**:
- Accumulating timing errors over long sessions
- Polyrhythms may drift
- 1/3 is `0.333333...` not exact

**Fix Needed**: Consider implementing Fraction-based time throughout (Phonon already has `Fraction` type!).

### Gap 3: Event Whole/Part Semantics ⚠️

**Tidal**: Events have `whole` (logical span) and `part` (query intersection)
**Phonon**: Events have `part` only (Hap structure)

**Impact**:
- Can't properly represent events that span query boundaries
- Legato/sustain behaviors may be incorrect

**Fix Needed**: Add `whole` field to `Hap` structure, populate correctly.

### Gap 4: Euclidean Pattern Caching ✅ (FIXED)

**Issue**: New pattern created on every query → duplicates
**Status**: Fixed with per-cycle caching (commit 38b198c)

---

## Recommended Fixes

### Priority 1: Query at Cycle Boundaries, Not Sample Rate

**Current**: Sample node queries pattern 44100 times/second
**Should be**: Query once per cycle, cache events, reference during render

```rust
// Pseudocode
struct SampleNode {
    pattern: Pattern<String>,
    cached_events: Vec<Event>,
    cached_cycle: i32,
}

fn eval_sample_node(&mut self) -> f32 {
    let current_cycle = self.cycle_position.floor() as i32;

    // Cache events if cycle changed
    if current_cycle != self.cached_cycle {
        self.cached_events = self.pattern.query(
            State { span: [current_cycle, current_cycle + 1), ... }
        );
        self.cached_cycle = current_cycle;
    }

    // Find events that should trigger in this sample window
    for event in &self.cached_events {
        if event_should_trigger_now(event, self.cycle_position) {
            trigger_sample(event);
        }
    }
}
```

### Priority 2: Use Fraction for Time Throughout

Phonon already has `Fraction` type. Use it everywhere instead of `f64`:

```rust
// Instead of:
let cycle_position: f64 = ...;

// Use:
let cycle_position: Fraction = Fraction::from_float(pos);
```

### Priority 3: Add `whole` to Hap

```rust
pub struct Hap<T> {
    pub whole: Option<TimeSpan>,  // Add this!
    pub part: TimeSpan,
    pub value: T,
}
```

---

## Testing Against Tidal

To verify Phonon matches Tidal, test these patterns:

```haskell
-- Basic sequence
sound "bd sn hh cp"
-- Should trigger 4 events evenly spaced

-- Fast
sound "bd sn" # fast 2
-- Should trigger 4 events (bd sn bd sn)

-- Slow
sound "bd sn hh cp" # slow 2
-- Cycle 0: bd sn (first half)
-- Cycle 1: hh cp (second half)

-- Alternation
sound "<bd sn hh>"
-- Cycle 0: bd, Cycle 1: sn, Cycle 2: hh

-- Euclidean with alternation
sound "bd(<3 7>, <8 9>)"
-- Cycle 0: 3 hits in 8 steps
-- Cycle 1: 7 hits in 9 steps
```

Compare:
1. Number of events per cycle
2. Timing of each event (exact fractions)
3. Event values
4. Behavior across cycle boundaries

---

## References

- [What is a Pattern?](https://tidalcycles.org/docs/innards/what_is_a_pattern/)
- [Mini Notation](https://tidalcycles.org/docs/reference/mini_notation/)
- [TidalCycles Source](https://github.com/tidalcycles/Tidal)
- [Strudel (JS implementation)](https://github.com/tidalcycles/strudel)

---

## Action Items

- [ ] Refactor Sample node to query once per cycle
- [ ] Switch time representation from f64 to Fraction
- [ ] Add `whole` field to Hap structure
- [ ] Write tests comparing Phonon output to Tidal for standard patterns
- [ ] Profile performance improvement from cycle-based querying
