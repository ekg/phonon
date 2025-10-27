# Pattern Evaluation Diagnosis: What's Wrong and How to Fix It

**Date**: 2025-10-27
**Status**: CRITICAL ISSUES IDENTIFIED

---

## Executive Summary

Phonon's pattern evaluation **fundamentally deviates** from Tidal Cycles in ways that cause subtle but pervasive bugs. The core issues are:

1. **Sample-rate querying**: Patterns queried 44,100 times/second instead of once per cycle
2. **Missing cycle-based caching**: Same query repeated thousands of times
3. **Onset detection issues**: wav_analyze fails to detect all events

---

## Test Results: What's Actually Happening

### Test 1: Simple Sequence `s "bd sn hh"`

**Expected** (Tidal semantics):
- 3 cycles × 3 events/cycle = **9 total events**
- Cycle 0: bd @ 0.00, sn @ 0.33, hh @ 0.67
- Cycle 1: bd @ 1.00, sn @ 1.33, hh @ 1.67
- Cycle 2: bd @ 2.00, sn @ 2.33, hh @ 2.67

**Actual** (Phonon):
- Onset detector: **2 events detected** (should be 9)
- RMS level: Normal (suggests audio is being generated)

**Conclusion**: Events may be triggering correctly, but onset detector is broken OR samples overlap so much they merge.

### Test 2: Single Event `s "bd"`

**Expected**: 3 cycles × 1 event/cycle = **3 total events**

**Actual**: **2 onset events detected** (should be 3)

**Conclusion**: Consistent undercounting - likely onset detection threshold/timing issue.

### Test 3: Alternation `s "<bd sn hh>"`

**Expected** (Tidal semantics):
- Cycle 0: bd only
- Cycle 1: sn only
- Cycle 2: hh only
- Total: **3 events**

**Actual**: **1 onset event detected** (should be 3)

**Conclusion**: Either slowcat is broken OR onset detector is severely undercounting.

---

## Root Cause Analysis

### Issue 1: Sample-Rate Pattern Queries (PERFORMANCE + CORRECTNESS)

**Current Implementation** (`unified_graph.rs:2728-2739`):

```rust
SignalNode::Sample { pattern, ... } => {
    // THIS RUNS 44,100 TIMES PER SECOND!
    let current_cycle_start = self.cycle_position.floor();
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(current_cycle_start),
            Fraction::from_float(current_cycle_start + 1.0),
        ),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);  // SAME QUERY REPEATED!
    // ...
}
```

**Problems**:

1. **Performance**: For a 3-cycle render:
   - 132,300 pattern queries (44100 Hz × 3 cycles)
   - 99.999% are identical queries returning same events
   - Massive CPU waste

2. **Correctness**: Every sample-rate query creates potential for:
   - Race conditions in stateful patterns
   - Inconsistent event ordering
   - Duplicate event generation (Euclidean bug we just fixed!)

3. **Not Idiomatic**: Tidal patterns are designed for coarse-grained queries (cycle or larger), not microsecond precision.

**How Tidal Does It**:
```haskell
-- Query ONCE at start of each cycle
events = pattern.query(State { span = (0, 1), ... })

-- Then during render loop, check event timing:
for sample in 0..44100:
    for event in events:
        if event.part.begin <= sample_time < event.part.end:
            render_event(event)
```

### Issue 2: Missing Event Cache

**Current**: No caching - every sample does a full pattern query
**Needed**: Cache events per cycle, invalidate when cycle changes

```rust
struct SampleNode {
    pattern: Pattern<String>,
    // ADD THESE:
    event_cache: Vec<Hap<String>>,
    cached_cycle: i32,
}

fn eval_sample_node(&mut self) -> f32 {
    let current_cycle = self.cycle_position.floor() as i32;

    // Refresh cache only when cycle changes
    if current_cycle != self.cached_cycle {
        self.event_cache = self.pattern.query(/* full cycle span */);
        self.cached_cycle = current_cycle;
    }

    // Use cached events for triggering
    // ...
}
```

### Issue 3: Onset Detection Unreliable

**Current**: `wav_analyze` misses many events

**Possible Causes**:
1. Threshold too high (quiet samples not detected)
2. Minimum spacing too large (rapid events merged)
3. Samples overlap in time (long decay tails merge)

**Fix Needed**: Either improve onset detector OR use alternative verification (check Sample node debug output directly).

---

## The Alternation Problem: Is Slowcat Broken?

### Slowcat Implementation (`pattern.rs:486-501`)

```rust
pub fn slowcat(patterns: Vec<Pattern<T>>) -> Pattern<T> {
    let len = patterns.len();
    Pattern::new(move |state| {
        let cycle = state.span.begin.to_float().floor() as usize;
        let pattern_idx = cycle % len;
        let pattern = &patterns[pattern_idx];
        pattern.query(state)
    })
}
```

**Analysis**: This looks CORRECT! It:
1. Gets cycle number from query span begin
2. Selects pattern via modulo
3. Queries selected pattern

**But Wait**: If Sample node queries with span `[0.0, 1.0)` for ALL samples in cycle 0:
- `cycle = floor(0.0) = 0` ✅
- `pattern_idx = 0 % 3 = 0` → selects "bd" ✅

For cycle 1, queries with span `[1.0, 2.0)`:
- `cycle = floor(1.0) = 1` ✅
- `pattern_idx = 1 % 3 = 1` → selects "sn" ✅

For cycle 2, queries with span `[2.0, 3.0)`:
- `cycle = floor(2.0) = 2` ✅
- `pattern_idx = 2 % 3 = 2` → selects "hh" ✅

**Slowcat is CORRECT!** The problem is likely onset detection, not pattern logic.

---

## Verification Strategy

### Method 1: Debug Output (Most Reliable)

Add comprehensive logging to Sample node:

```rust
if std::env::var("DEBUG_SAMPLE_TRIGGERS").is_ok() {
    eprintln!("CYCLE {}: Triggering '{}' at cycle_pos={:.6}",
        current_cycle, sample_name, self.cycle_position);
}
```

Then test:
```bash
DEBUG_SAMPLE_TRIGGERS=1 cargo run --release --bin phonon -- \
    render --cycles 3 /tmp/test_alternation.phon /tmp/out.wav \
    2>&1 | grep "CYCLE"
```

**Expected Output**:
```
CYCLE 0: Triggering 'bd' at cycle_pos=0.000000
CYCLE 1: Triggering 'sn' at cycle_pos=1.000000
CYCLE 2: Triggering 'hh' at cycle_pos=2.000000
```

### Method 2: Pattern Query Test

Directly test pattern evaluation without audio:

```rust
#[test]
fn test_slowcat_alternation() {
    let bd = Pattern::pure("bd".to_string());
    let sn = Pattern::pure("sn".to_string());
    let hh = Pattern::pure("hh".to_string());

    let pattern = Pattern::slowcat(vec![bd, sn, hh]);

    // Cycle 0
    let events0 = pattern.query(&State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    });
    assert_eq!(events0.len(), 1);
    assert_eq!(events0[0].value, "bd");

    // Cycle 1
    let events1 = pattern.query(&State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    });
    assert_eq!(events1.len(), 1);
    assert_eq!(events1[0].value, "sn");

    // Cycle 2
    let events2 = pattern.query(&State {
        span: TimeSpan::new(Fraction::new(2, 1), Fraction::new(3, 1)),
        controls: HashMap::new(),
    });
    assert_eq!(events2.len(), 1);
    assert_eq!(events2[0].value, "hh");
}
```

---

## Recommended Fixes (Priority Order)

### Priority 1: Add Cycle-Based Event Caching ⚠️ CRITICAL

**Impact**: 99.9% reduction in pattern queries, major performance boost

**Implementation**:
1. Add `event_cache` and `cached_cycle` to Sample node
2. Query pattern only when cycle changes
3. Reuse cached events within cycle

**Effort**: 2-3 hours

### Priority 2: Add Debug Logging for Triggers ⚠️ DIAGNOSTIC

**Impact**: Visibility into actual trigger behavior

**Implementation**:
1. Add `DEBUG_SAMPLE_TRIGGERS` environment variable
2. Log every sample trigger with cycle number and position
3. Use for verification instead of relying on onset detection

**Effort**: 30 minutes

### Priority 3: Improve Onset Detection 🔧 NICE-TO-HAVE

**Impact**: Better audio analysis tools

**Implementation**:
1. Lower detection threshold
2. Reduce minimum spacing between onsets
3. Add configurable parameters

**Effort**: 1-2 hours

### Priority 4: Switch to Fraction Time 📐 LONG-TERM

**Impact**: Exact timing, no accumulating errors

**Implementation**:
1. Use `Fraction` instead of `f64` for cycle_position
2. Update all time arithmetic
3. Major refactor across codebase

**Effort**: 1-2 days

---

## Next Steps

1. **Verify Pattern Logic**: Run Method 2 unit test to confirm slowcat works
2. **Add Debug Logging**: Implement Method 1 to see actual triggers
3. **Implement Event Caching**: Priority 1 fix for performance + correctness
4. **Document Findings**: Update CLAUDE.md with pattern evaluation best practices

---

## Questions for User

1. What specific behavior are you seeing that seems wrong?
2. Is it that patterns sound wrong, or that the onset count is wrong?
3. Can you describe what you EXPECT to hear vs. what you ACTUALLY hear?

Understanding the subjective audio experience will help pinpoint whether this is:
- A pattern evaluation bug (wrong events generated)
- A trigger timing bug (right events, wrong times)
- An onset detection bug (right audio, wrong analysis)
