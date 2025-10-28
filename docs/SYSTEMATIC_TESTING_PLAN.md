# Systematic Testing Plan for Phonon

**Goal**: Achieve full Tidal Cycles feature parity with verified correctness

**Motivation**: Recent bugs (Euclidean (3,7) timing, duplicate events) revealed our tests only verify compilation, not correctness. We need three-level verification for every feature.

## The Three-Level Testing Methodology

Every pattern/audio feature MUST pass all three levels:

### Level 1: Pattern Query Verification (Exact)
- Test pattern logic without rendering audio
- Verify event counts over 4-8 cycles
- Check event timing/structure
- Fast, deterministic, exact

### Level 2: Onset Detection (Audio Timing)
- Render audio and detect transients/peaks
- Verify events actually occur in audio
- Check timing intervals match expectations
- Catches silent output, wrong timing, doubled events

### Level 3: Audio Characteristics (Sanity Check)
- RMS level verification
- Peak level checks
- DC offset monitoring
- Frequency content analysis

## Phase 1: Transform Verification (IN PROGRESS)

Test all 40+ transforms systematically:

### Basic Time Transforms
- [x] `fast` - Verified (40+ tests exist)
- [x] `slow` - Verified
- [x] `rev` - Verified
- [ ] `iter` - Needs three-level tests
- [ ] `palindrome` - Needs three-level tests
- [ ] `every` - Has tests, need audio verification

### Structural Transforms
- [ ] `chunk` - Need to implement/verify
- [ ] `bite` - Stub exists, needs implementation
- [ ] `chop` - Not yet implemented
- [ ] `striate` - Not yet implemented
- [ ] `slice` - Not yet implemented

### Conditional Transforms
- [x] `every` - Basic tests exist
- [ ] `whenmod` - Not yet implemented
- [ ] `every'` - Not yet implemented
- [ ] `someCycles` - Not yet implemented

### Probabilistic Transforms
- [x] `degrade` - Exists with deterministic tests
- [ ] `sometimes` - Not yet implemented
- [ ] `rarely` - Not yet implemented
- [ ] `often` - Not yet implemented

### Rhythmic Transforms
- [ ] `hurry` - Not yet implemented
- [ ] `swing` - Exists, needs verification
- [ ] `shuffle` - Not yet implemented

## Phase 2: Missing Tidal Features

### High Priority
- [ ] Sample bank selection: `s "bd:0 bd:1 bd:2"`
- [ ] Pattern DSP params: `gain "1 0.8"`, `pan "0 1"`, `speed "1 2"`
- [ ] More mini-notation: `?` (chance), `!` (replicate), `@` (elongate)
- [ ] Slicing: `chop 4`, `slice 8 "0 2 4"`
- [ ] Stacking: `stack [p1, p2]`, `superimpose`

### Medium Priority
- [ ] Scale transforms: `scale "major"`
- [ ] Mode transforms: `mode "dorian"`
- [ ] More conditionals: `whenmod 4 2`
- [ ] Pattern arithmetic: `(+ 12)`, `(* 2)`

### Lower Priority
- [ ] Sample manipulation: `loopAt`, `striate`
- [ ] Control busses between channels
- [ ] Advanced MIDI features

## Phase 3: Edge Case Matrix

### Euclidean Patterns
Test all combinations:
- [x] Even denominators: (3,8), (5,8)
- [x] Odd denominators: (3,7), (5,9)
- [ ] Prime denominators: (3,11), (5,13), (7,17)
- [ ] Large numbers: (17,32), (31,64)
- [ ] Edge cases: (0,8), (8,8), (1,7)

### Fractional Tempos
- [ ] 0.333 (3 seconds per cycle)
- [ ] 0.666 (1.5 seconds per cycle)
- [ ] 1.414 (√2, irrational)
- [ ] 0.1 (very slow)
- [ ] 10.0 (very fast)

### Complex Combinations
- [ ] `fast 3 $ slow 7` (coprime factors)
- [ ] `rev $ every 5` (reverse + conditional)
- [ ] `struct "t(3,7)" $ fast 2` (Euclidean + time)
- [ ] Nested: `fast 2 $ fast 3 $ fast 5`

## Phase 4: Property-Based Testing

Use `proptest` to auto-generate edge cases:

```rust
proptest! {
    #[test]
    fn euclid_event_count(pulses in 1..16, steps in 1..32, cycles in 1..8) {
        let events = count_events(&format!("bd({},{})", pulses, steps), cycles);
        assert_eq!(events, pulses * cycles);
    }

    #[test]
    fn fast_multiplies_events(factor in 1..10, cycles in 1..8) {
        let normal = count_events("bd sn", cycles);
        let fast = count_events(&format!("bd sn $ fast {}", factor), cycles);
        assert_eq!(fast, normal * factor);
    }
}
```

## Phase 5: Tidal Reference Comparison

If Tidal Cycles is available, render same patterns and compare:

1. Install Tidal Cycles
2. Create test harness to render patterns
3. Compare onset timings (allow small tolerance)
4. Compare audio characteristics

## Implementation Strategy

### For Each Transform:

1. **Write failing test** (TDD):
   ```rust
   #[test]
   fn test_transform_three_levels() {
       // Level 1: Pattern events
       verify_pattern_events("bd sn $ transform", expected_count);

       // Level 2: Audio onsets
       verify_audio_onsets("bd sn $ transform", expected_timings);

       // Level 3: Audio quality
       verify_audio_quality("bd sn $ transform");
   }
   ```

2. **Implement/fix transform**

3. **Verify test passes**

4. **Add to verified list**

## Success Metrics

**Technical**:
- All 40+ transforms have three-level tests
- Property-based tests generate 1000+ edge cases
- Tidal comparison tests pass (if available)
- 500+ total tests passing

**Musical**:
- Can recreate any Tidal Cycles pattern
- Can perform live coding sessions without bugs
- Audio quality matches Tidal output

## Current Status (2025-10-28)

- ✅ Three-level testing methodology defined
- ✅ Fixed Euclidean (3,7) timing bug (epsilon tolerance)
- ✅ 303 tests passing (mostly compilation tests)
- 🚧 Starting systematic transform verification
- 📋 Documented in this plan

## Next Session TODO

1. Create systematic transform test framework
2. Verify all basic time transforms (fast, slow, rev, iter, palindrome)
3. Add edge case matrix for Euclidean patterns
4. Implement missing high-priority features (sample banks, pattern params)
5. Set up property-based testing
