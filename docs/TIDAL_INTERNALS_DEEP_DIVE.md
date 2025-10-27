# Tidal Cycles Internals: How Pattern Manipulation ACTUALLY Works

**Date**: 2025-10-27
**Source**: Strudel (JavaScript port of Tidal Cycles)

---

## Executive Summary

After studying the **actual source code** of Tidal/Strudel, I now understand HOW pattern manipulation works internally. It's elegant and surprisingly simple once you see it.

**Key Insight**: Patterns don't store events. They're **functions** that transform TIME ITSELF.

---

## The Core Abstraction: Patterns as Query Functions

### Pattern Definition (JavaScript/Strudel)

```javascript
class Pattern {
  constructor(query, steps = undefined) {
    this.query = query;  // THE PATTERN IS THIS FUNCTION
    this._steps = steps;  // Optional: for step-based rhythms
  }
}
```

**The pattern IS the query function.** That's it. No event storage, no pre-computed sequences.

### The Query Function Signature

```javascript
query: (State) => Hap[]
```

Where:
- `State` contains: `{ span: TimeSpan, controls: {...} }`
- `TimeSpan` is: `{ begin: Fraction, end: Fraction }`
- `Hap` (event) is: `{ whole: TimeSpan, part: TimeSpan, value: any }`

**You call `pattern.query(state)` and get back events for that timespan.**

---

## How Transforms Work: Time Transformation

This is the BIG insight - transforms **don't modify events**, they **transform TIME**.

### The Two Time-Transformation Methods

#### 1. `withQueryTime(func)` - Transform BEFORE Querying

```javascript
withQueryTime(func) {
  return new Pattern((state) =>
    this.query(state.withSpan((span) => span.withTime(func)))
  );
}
```

**What this does**:
1. Take the incoming query span
2. Apply `func` to its begin/end times
3. Query the original pattern with the TRANSFORMED span
4. Return those events

**Example**: To query cycle [0, 1) but get events from cycle [0, 2):
```javascript
pattern.withQueryTime(t => t.mul(2))  // Multiply query time by 2
```

#### 2. `withHapTime(func)` - Transform AFTER Querying

```javascript
withHapTime(func) {
  return new Pattern((state) => {
    const haps = this.query(state);  // Query first
    return haps.map(hap => hap.withTime(func));  // Then transform results
  });
}
```

**What this does**:
1. Query the pattern normally
2. Get back events
3. Apply `func` to each event's time values
4. Return transformed events

**Example**: To shift all events forward by 0.5 cycles:
```javascript
pattern.withHapTime(t => t.add(0.5))
```

---

## Implementing `fast` - The Tidal Way

Here's the ACTUAL implementation from Strudel:

```javascript
export const fast = register('fast', function (factor, pat) {
  if (factor === 0) return silence;
  factor = Fraction(factor);

  // Step 1: Speed up the QUERY (compress pattern in time)
  const fastQuery = pat.withQueryTime((t) => t.mul(factor));

  // Step 2: Slow down the EVENTS (stretch them back to original positions)
  return fastQuery.withHapTime((t) => t.div(factor));
});
```

### How `fast 2` Works Internally

Let's trace `"bd sn" $ fast 2` querying cycle [0, 1):

**Original pattern**: `"bd sn"`
- Generates: bd @ [0, 0.5), sn @ [0.5, 1)

**Step 1**: `withQueryTime(t => t * 2)`
- Incoming query: [0, 1)
- Transform query to: [0, 2)
- Query original pattern with [0, 2)
- Get back: bd @ [0, 0.5), sn @ [0.5, 1), bd @ [1, 1.5), sn @ [1.5, 2)

**Step 2**: `withHapTime(t => t / 2)`
- Take those events and divide their times by 2:
  - bd @ [0, 0.25)
  - sn @ [0.25, 0.5)
  - bd @ [0.5, 0.75)
  - sn @ [0.75, 1.0)

**Result**: The pattern now plays twice as fast!

### Why This Works

**The trick**: Query MORE of the original pattern (by multiplying time), then SQUISH the results back (by dividing time).

It's like:
1. "Give me 2 cycles worth of events" (query transformation)
2. "But compress them into 1 cycle" (hap transformation)

---

## Implementing `slow` - The Tidal Way

```javascript
export const slow = register('slow', function (factor, pat) {
  if (factor === 0) return silence;
  return pat._fast(Fraction(1).div(factor));
});
```

**That's it!** `slow` is just `fast` with the reciprocal factor.

`slow 2` = `fast 0.5`

### How `slow 2` Works Internally

Applying `slow 2` to `"bd sn hh cp"`:

**Step 1**: Convert to `fast 0.5`
**Step 2**: Query transformation: `t * 0.5`
- Query [0, 1) becomes [0, 0.5)
- Get back: bd @ [0, 0.25), sn @ [0.25, 0.5)

**Step 3**: Hap transformation: `t / 0.5` (which is `t * 2`)
- bd @ [0, 0.5)
- sn @ [0.5, 1.0)

**Result**: Only the first half of the pattern plays in cycle 0!

For cycle 1:
- Query [1, 2) becomes [0.5, 1.0)
- Get back: hh @ [0.5, 0.75), cp @ [0.75, 1.0)
- Transform: hh @ [1.0, 1.5), cp @ [1.5, 2.0)

**Result**: Second half plays in cycle 1!

---

## Implementing `slowcat` (Alternation `<a b c>`)

Here's the ACTUAL implementation:

```javascript
export function slowcat(...pats) {
  pats = pats.map(reify);  // Convert to patterns
  if (pats.length == 1) return pats[0];

  const query = function (state) {
    const span = state.span;

    // Which pattern to play? Based on the cycle number!
    const pat_n = _mod(span.begin.sam(), pats.length);
    const pat = pats[pat_n];

    // Calculate offset for this pattern
    const offset = span.begin.floor().sub(
      span.begin.div(pats.length).floor()
    );

    // Query the selected pattern with adjusted time
    return pat
      .withHapTime((t) => t.add(offset))
      .query(state.setSpan(span.withTime((t) => t.sub(offset))));
  };

  return new Pattern(query);
}
```

### How `<bd sn hh>` Works Internally

Query cycle 0 `[0, 1)`:
- `pat_n = floor(0) % 3 = 0` → Select "bd"
- `offset = 0 - floor(0/3) = 0`
- Query "bd" pattern with span [0, 1)
- Result: bd @ [0, 1)

Query cycle 1 `[1, 2)`:
- `pat_n = floor(1) % 3 = 1` → Select "sn"
- `offset = 1 - floor(1/3) = 1`
- Query "sn" pattern, then add offset
- Result: sn @ [1, 2)

Query cycle 2 `[2, 3)`:
- `pat_n = floor(2) % 3 = 2` → Select "hh"
- `offset = 2 - floor(2/3) = 2`
- Query "hh" pattern, then add offset
- Result: hh @ [2, 3)

**Key Point**: `slowcat` selects ONE pattern per cycle based on `cycle % len`, not all of them!

---

## Implementing `fastcat`

```javascript
export function fastcat(...pats) {
  let result = slowcat(...pats);
  if (pats.length > 1) {
    result = result._fast(pats.length);
  }
  return result;
}
```

**That's it!** `fastcat` is just `slowcat` sped up by the number of patterns.

So `[bd sn hh]` becomes:
1. `slowcat(bd, sn, hh)` → plays over 3 cycles
2. `fast 3` → compress those 3 cycles into 1

Result: All three play in one cycle!

---

## Implementing `stack`

```javascript
export function stack(...pats) {
  pats = pats.map(reify);

  const query = (state) => {
    // Query ALL patterns and merge results
    return flatten(pats.map((pat) => pat.query(state)));
  };

  return new Pattern(query);
}
```

**Simple!** Query all patterns with the same span and concatenate their events.

---

## Implementing `pure`

```javascript
export function pure(value) {
  function query(state) {
    return state.span.spanCycles.map((subspan) =>
      new Hap(
        Fraction(subspan.begin).wholeCycle(),  // whole: entire cycle
        subspan,                                // part: query span
        value                                   // value: the data
      )
    );
  }
  return new Pattern(query);
}
```

**What it does**: For each cycle in the query span, create one event with the given value.

Example: `pure("bd")` queried with [0, 2):
- Cycle 0: Event with value "bd" @ [0, 1)
- Cycle 1: Event with value "bd" @ [1, 2)

---

## Implementing `silence`

```javascript
export const silence = new Pattern(() => []);
```

**Simplest pattern**: Query function that always returns empty array!

---

## The Brilliance of This Design

### 1. Composability

Since patterns are just query functions, you can compose them infinitely:

```javascript
pattern
  .fast(2)
  .slow(3)
  .rev()
  .every(4, p => p.fast(2))
```

Each transformation wraps the previous pattern in a new query function.

### 2. Laziness

No events are computed until you query! This means:
- Infinite patterns are possible
- No memory overhead
- Real-time live coding works

### 3. Time is Relative

Transforms modify how time flows through the pattern:
- `fast` compresses time
- `slow` stretches time
- `rev` reverses time
- `early/late` shifts time

**All without modifying events!**

---

## How Phonon Compares

### ✅ What Phonon Gets Right

1. **Pattern as Function**: `Pattern::new(move |state| { ... })` ✅
2. **Query-based**: Patterns return events via `query(state)` ✅
3. **Transforms compose**: `pattern.fast(2).slow(3)` works ✅
4. **Fraction-based time**: `Fraction` type exists ✅

### ⚠️ Where Phonon Differs

#### 1. Query Frequency

**Tidal/Strudel**: Query once per cycle (or less)
**Phonon**: Queries 44,100 times per second (every sample!)

**Impact**: Performance overhead, but no correctness issues (since queries are pure functions).

#### 2. Time Representation

**Tidal/Strudel**: Uses `Fraction` everywhere
**Phonon**: Uses `f64` for `cycle_position`, `Fraction` for pattern internals

**Impact**: Potential floating-point drift, but minor for typical usage.

#### 3. Event Caching

**Tidal/Strudel**: No caching needed (queries are cheap, done infrequently)
**Phonon**: Would benefit from per-cycle event cache (reduce 44100 queries to 1)

---

## Recommended Changes for Phonon

### Priority 1: Cache Events Per Cycle (Performance)

Instead of:
```rust
// CURRENT: Query 44,100 times per cycle!
fn eval_sample_node(&mut self) -> f32 {
    let events = self.pattern.query(state);  // EVERY SAMPLE!
}
```

Do this:
```rust
// BETTER: Query once per cycle
struct SampleNode {
    pattern: Pattern<String>,
    event_cache: Vec<Hap<String>>,
    cached_cycle: i32,
}

fn eval_sample_node(&mut self) -> f32 {
    let current_cycle = self.cycle_position.floor() as i32;

    if current_cycle != self.cached_cycle {
        // Only query when cycle changes!
        self.event_cache = self.pattern.query(/* full cycle span */);
        self.cached_cycle = current_cycle;
    }

    // Use cached events to determine triggers
}
```

**Benefit**: 99.9% reduction in pattern queries!

### Priority 2: Use Fraction Consistently (Correctness)

Replace:
```rust
let cycle_position: f64 = ...;
```

With:
```rust
let cycle_position: Fraction = Fraction::from_float(pos);
```

**Benefit**: Perfect rhythmic alignment, no drift.

### Priority 3: Implement `withQueryTime` and `withHapTime` (Clarity)

Add these helper methods to `Pattern`:

```rust
impl<T> Pattern<T> {
    pub fn with_query_time<F>(self, f: F) -> Pattern<T>
    where
        F: Fn(Fraction) -> Fraction + Send + Sync + 'static,
    {
        Pattern::new(move |state| {
            let new_span = TimeSpan::new(
                f(state.span.begin),
                f(state.span.end),
            );
            self.query(&State { span: new_span, ..state })
        })
    }

    pub fn with_hap_time<F>(self, f: F) -> Pattern<T>
    where
        F: Fn(Fraction) -> Fraction + Send + Sync + 'static,
    {
        Pattern::new(move |state| {
            self.query(state)
                .into_iter()
                .map(|hap| hap.with_time(&f))
                .collect()
        })
    }
}
```

Then reimplement `fast`:
```rust
pub fn fast(self, factor: f64) -> Pattern<T> {
    let factor_frac = Fraction::from_float(factor);
    self.with_query_time(|t| t * factor_frac)
        .with_hap_time(|t| t / factor_frac)
}
```

**Benefit**: Code clarity, matches Tidal's model exactly.

---

## Conclusion

**Tidal's pattern system is beautifully simple**:
1. Patterns are query functions
2. Transforms modify time, not events
3. Composition is just function wrapping
4. Everything is lazy and pure

**Phonon's implementation is fundamentally correct** - it follows this model! The main differences are:
- Query frequency (optimization opportunity)
- Time representation (minor drift risk)
- Code style (could be more "functional")

All the bugs we fixed this session were EDGE CASES, not fundamental architecture problems. The core pattern engine is solid!

---

## References

- [Strudel Source Code](https://codeberg.org/uzu/strudel/src/branch/main/packages/core/pattern.mjs)
- [Tidal Cycles Pattern Documentation](https://tidalcycles.org/docs/innards/what_is_a_pattern/)
- [Strudel Technical Manual](https://github.com/tidalcycles/strudel/wiki/Technical-Manual)
