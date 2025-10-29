# Pattern Transform Implementation & Testing Checklist

**Purpose**: Systematic verification and implementation of all Tidal Cycles transforms
**Last Updated**: 2025-10-28
**Current Status**: 4 verified, ~50 implemented but untested, ~15 missing

---

## Methodology

For each transform, we verify 4 stages:
1. **✅ Parsed** - DSL syntax works (`Transform::Name` exists in parser)
2. **✅ Compiled** - Compiler handles it (case in `compile_transform`)
3. **✅ Implemented** - Pattern method exists and works
4. **✅ Tested** - Has 3-level verification tests (pattern query, onset detection, audio)

**Testing Requirements**:
- Level 1: Pattern query verification (exact event counts over 4-8 cycles)
- Level 2: Onset detection (audio events match expectations)
- Level 3: Audio characteristics (RMS, peak, frequency analysis)

---

## TIER 1: Core Transforms (Musical Essentials)

### Time Manipulation
- [x] **fast** - Speed up pattern ✅ FULLY VERIFIED
- [x] **slow** - Slow down pattern ✅ FULLY VERIFIED
- [x] **rev** - Reverse pattern ✅ FULLY VERIFIED
- [x] **iter** - Progressive iteration ✅ FULLY VERIFIED
- [x] **palindrome** - Forward then backward ✅ FULLY VERIFIED

### Repetition
- [x] **stutter** - Repeat each event ✅ FULLY VERIFIED
- [x] **dup** - Duplicate pattern ✅ FULLY VERIFIED
- [x] **ply** - Repeat events (like stutter) ✅ FULLY VERIFIED

### Conditional
- [x] **every** - Apply every n cycles ✅ FULLY VERIFIED
- [x] **sometimes** - 50% probability ✅ FULLY VERIFIED
- [x] **often** - 75% probability ✅ FULLY VERIFIED
- [x] **rarely** - 10% probability ✅ FULLY VERIFIED

### Probability
- [x] **degrade** - Random removal (50%) ✅ FULLY VERIFIED
- [x] **degradeBy** - Random removal (custom %) ✅ FULLY VERIFIED

### Structural
- [x] **euclid** - Euclidean rhythms ✅ FULLY VERIFIED
- [x] **chop** - Slice into pieces ✅ FULLY VERIFIED
- [x] **striate** - Alias for chop ✅ FULLY VERIFIED

**Priority**: CRITICAL - These are the most-used transforms in live coding
**Estimated Time**: 3-4 days (1-2 hours per transform with tests)

---

## TIER 2: Enhanced Expression (Common Usage)

### Rhythmic Feel
- [x] **swing** - Add swing timing ✅ FULLY VERIFIED
- [x] **shuffle** - Random time shifts ✅ FULLY VERIFIED
- [x] **legato** - Longer duration ✅ FULLY VERIFIED
- [x] **staccato** - Shorter duration ✅ FULLY VERIFIED

### Time Shifting
- [x] **early** - Shift earlier ✅ FULLY VERIFIED
- [x] **late** - Shift later ✅ FULLY VERIFIED
- [x] **offset** - Time offset (alias) ✅ FULLY VERIFIED

### Pattern Effects
- [x] **echo** - Echo with decay ✅ FULLY VERIFIED
- [x] **segment** - Sample n times per cycle ✅ FULLY VERIFIED

### Structure
- [x] **zoom** - Focus on time range ✅ FULLY VERIFIED
- [x] **compress** - Compress to range ✅ FULLY VERIFIED
- [x] **spin** - Rotate versions ✅ FULLY VERIFIED
- [x] **scramble** - Shuffle events ✅ FULLY VERIFIED

**Priority**: HIGH - Used frequently, adds expressiveness
**Estimated Time**: 2-3 days

---

## TIER 3: Advanced Transforms (Power Users)

### Rotation & Iteration
- [x] **rotL** - Rotate left ✅ FULLY VERIFIED
- [x] **rotR** - Rotate right ✅ FULLY VERIFIED
- [x] **iterBack** - Iterate backwards ✅ FULLY VERIFIED

### Meta-transforms
- [x] **chunk** - Divide and transform ✅ FULLY VERIFIED
- [x] **superimpose** - Layer with transform ✅ FULLY VERIFIED
- [x] **within** - Apply within time ✅ FULLY VERIFIED
- [x] **inside** - Apply inside range ✅ FULLY VERIFIED
- [x] **outside** - Apply outside range ✅ FULLY VERIFIED

### Conditional Variants
- [ ] **almostAlways** - 90% probability ❌ NOT IMPLEMENTED (would be alias for sometimesBy 0.9)
- [ ] **almostNever** - 10% probability ❌ NOT IMPLEMENTED (same as rarely)
- [x] **sometimesBy** - Custom probability ✅ FULLY VERIFIED
- [x] **whenmod** - Cycle-based condition ✅ FULLY VERIFIED

### Advanced Time
- [x] **gap** - Insert silence ✅ FULLY VERIFIED
- [x] **fit** - Fit to cycles ✅ FULLY VERIFIED
- [x] **stretch** - Sustain notes ✅ FULLY VERIFIED
- [x] **linger** - Linger on values ✅ FULLY VERIFIED
- [x] **loop** - Loop within cycle ✅ FULLY VERIFIED
- [x] **chew** - Chew through pattern ✅ FULLY VERIFIED

**Priority**: MEDIUM - Used for advanced patterns
**Estimated Time**: 3-4 days

---

## TIER 4: Numeric & Special (Lower Priority)

### Numeric Transforms
- [x] **discretise** - Quantize time ✅ FULLY VERIFIED
- [x] **range** - Scale to range ✅ FULLY VERIFIED
- [x] **quantize** - Quantize values ✅ FULLY VERIFIED
- [x] **smooth** - Smooth values ✅ FULLY VERIFIED
- [x] **exp** - Exponential ✅ FULLY VERIFIED
- [x] **log** - Logarithmic ✅ FULLY VERIFIED
- [x] **walk** - Random walk ✅ FULLY VERIFIED

### Special Purpose
- [ ] **compressGap** - Compress with gaps 🟨 IMPLEMENTED, needs tests
- [ ] **reset** - Restart cycles 🟨 IMPLEMENTED, needs tests
- [ ] **restart** - Restart alias 🟨 IMPLEMENTED, needs tests
- [ ] **loopback** - Back then forward 🟨 IMPLEMENTED, needs tests
- [ ] **binary** - Bit mask 🟨 IMPLEMENTED, needs tests
- [ ] **focus** - Focus on cycles 🟨 IMPLEMENTED, needs tests
- [ ] **trim** - Trim time range 🟨 IMPLEMENTED, needs tests
- [ ] **wait** - Delay by cycles 🟨 IMPLEMENTED, needs tests
- [ ] **mask** - Boolean mask 🟨 IMPLEMENTED, needs tests
- [ ] **weave** - Weave pattern 🟨 IMPLEMENTED, needs tests
- [ ] **degradeSeed** - Seeded degrade 🟨 IMPLEMENTED, needs tests
- [ ] **undegrade** - Identity 🟨 IMPLEMENTED, needs tests
- [ ] **accelerate** - Speed up over time 🟨 IMPLEMENTED, needs tests
- [ ] **humanize** - Timing variation 🟨 IMPLEMENTED, needs tests
- [ ] **mirror** - Palindrome alias 🟨 IMPLEMENTED, needs tests
- [ ] **always** - Identity (100%) 🟨 IMPLEMENTED, needs tests
- [ ] **fastGap** - Fast with gaps 🟨 IMPLEMENTED, needs tests

**Priority**: LOW - Niche use cases
**Estimated Time**: 4-5 days

---

## TIER 5: Not Yet Implemented (Future Work)

### High-Impact Missing
- [x] **jux** - Stereo channel manipulation 🟨 IMPLEMENTED (pattern_ops.rs), needs tests
- [x] **bite** - Extract slices 🟨 IMPLEMENTED (pattern_structure.rs), needs tests
- [x] **slice** - Select from slices 🟨 IMPLEMENTED, needs tests
- [x] **hurry** - Speed up with playback 🟨 IMPLEMENTED, needs tests

### Medium-Impact Missing
- [x] **fastcat** - Fast concatenation 🟨 IMPLEMENTED (mini_notation_v3.rs), needs tests
- [x] **slowcat** - Slow concatenation 🟨 IMPLEMENTED (pattern.rs), needs tests
- [x] **randcat** - Random concatenation 🟨 IMPLEMENTED, needs tests
- [x] **timeCat** - Time-weighted cat 🟨 IMPLEMENTED, needs tests
- [x] **splice** - Splice with pattern 🟨 IMPLEMENTED (pattern_ops_extended.rs), needs tests
- [x] **loopAt** - Loop at cycles 🟨 IMPLEMENTED, needs tests

### Advanced Missing
- [x] **weaveWith** - Weave with function 🟨 IMPLEMENTED, needs tests
- [x] **layer** - Layer transforms 🟨 IMPLEMENTED (pattern_structure.rs), needs tests
- [x] **chooseWith** - Weighted choice 🟨 IMPLEMENTED, needs tests
- [x] **scale** / **scaleList** - Musical scales 🟨 IMPLEMENTED, needs tests
- [x] **chordList** - Chord transforms 🟨 IMPLEMENTED, needs tests
- [x] **steps** - Step sequencer 🟨 IMPLEMENTED (pattern_structure.rs), needs tests
- [x] **run** - Run up/down 🟨 IMPLEMENTED (pattern_signal.rs), needs tests
- [x] **scan** - Cumulative fold 🟨 IMPLEMENTED (pattern_signal.rs), needs tests

**Priority**: FUTURE - Implement after verifying existing transforms
**Estimated Time**: 5-7 days

---

## Implementation Workflow (TDD)

For EACH transform:

### 1. Write Failing Tests (30-45 min)
```bash
tests/test_transform_NAME.rs
```

```rust
#[test]
fn test_NAME_level1_pattern_query() {
    // Test pattern events over 4-8 cycles
    let pattern = parse_mini_notation("bd sn").TRANSFORM();
    let events = count_events_over_cycles(pattern, 8);
    assert_eq!(events, EXPECTED);
}

#[test]
fn test_NAME_level2_onset_detection() {
    // Render audio and detect onsets
    let audio = render_dsl("s \"bd sn\" $ NAME", 8);
    let onsets = detect_audio_events(&audio);
    assert_eq!(onsets.len(), EXPECTED);
}

#[test]
fn test_NAME_level3_audio_quality() {
    // Check RMS, peak, DC offset
    let audio = render_dsl("s \"bd sn\" $ NAME", 8);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01);
}
```

### 2. Run Test - Confirm Failure (2 min)
```bash
cargo test test_NAME
# Should fail if not implemented, or pass if buggy
```

### 3. Implement/Fix (30-60 min)
- Check if pattern method exists in `src/pattern_ops.rs` or `src/pattern_ops_extended.rs`
- If missing, implement it
- If exists, verify correctness
- Ensure compiler case exists in `src/compositional_compiler.rs`

### 4. Run Test - Confirm Pass (2 min)
```bash
cargo test test_NAME
# All 3 levels should pass
```

### 5. Manual Test (10 min)
```bash
# Create example file
cat > /tmp/test_NAME.ph << 'EOF'
tempo: 0.5
~test: s "bd sn hh cp" $ NAME ARGS
out: ~test
EOF

cargo run --release --bin phonon -- render --cycles 4 /tmp/test_NAME.ph /tmp/test_NAME.wav
cargo run --release --bin wav_analyze -- /tmp/test_NAME.wav
```

### 6. Commit (2 min)
```bash
git add tests/test_transform_NAME.rs src/pattern_ops*.rs src/compositional_compiler.rs
git commit -m "Verify/implement TRANSFORM with 3-level tests

- Level 1: Pattern query verification
- Level 2: Onset detection
- Level 3: Audio characteristics
- Test case: DESCRIPTION
"
```

### 7. Update This Checklist
- Mark transform as ✅ FULLY VERIFIED
- Update status in this document

---

## Progress Tracking

### Overall Status
- ✅ FULLY VERIFIED: 54 / ~68 (79.4%) 🎉 ALMOST 80%!
- 🟨 IMPLEMENTED, needs tests: ~12 / ~68 (17.6%)
- ❌ NOT IMPLEMENTED: 2 / ~68 (2.9%) - almostAlways, almostNever (would be aliases)

### Tier Progress
- **Tier 1** (Core): 18/18 verified (100%) ✅ COMPLETE
- **Tier 2** (Enhanced): 12/12 verified (100%) ✅ COMPLETE
- **Tier 3** (Advanced): 22/22 implemented, 16/22 verified (72.7%) - 6 remaining
- **Tier 4** (Numeric): 24/24 implemented, 7/24 verified (29.2%) - IN PROGRESS 🔥
- **Tier 5** (All transforms): 16/16 implemented (100%) ✅ ALL IMPLEMENTED

### Estimated Timeline
- **Tier 1**: 3-4 days (12 transforms remaining)
- **Tier 2**: 2-3 days (12 transforms)
- **Tier 3**: 3-4 days (24 transforms)
- **Tier 4**: 4-5 days (22 transforms)
- **Total**: ~12-16 days for full verification

**At 3-4 transforms per day, expect 2-3 weeks of focused work**

---

## Success Criteria

### Technical
- [ ] All Tier 1 transforms have 3-level tests (18 transforms)
- [ ] All Tier 2 transforms have 3-level tests (12 transforms)
- [ ] All Tier 3 transforms have 3-level tests (24 transforms)
- [ ] All Tier 4 transforms have 3-level tests (22 transforms)
- [ ] 500+ total tests passing
- [ ] No compilation warnings for transforms
- [ ] All examples in docs/ render correctly

### Musical
- [ ] Can recreate any Tidal Cycles tutorial pattern
- [ ] Can perform live coding without bugs
- [ ] Transform combinations work correctly
- [ ] Edge cases handled (primes, fractions, large numbers)

### Documentation
- [ ] Every transform has usage example
- [ ] Every transform has expected behavior documented
- [ ] TIDAL_CYCLES_DEEP_DIVE.md updated
- [ ] This checklist kept current

---

## Next Session Start Here

**Current Focus**: TIER 3 - Advanced Transforms (Power Users)

**TIER 1 & 2 COMPLETE! 🎉** All 30 core and enhanced transforms verified with 3-level tests.

**Next Transform**: `rotL` (rotate left)

**Command to run**:
```bash
# 1. Create test file for rotation & iteration transforms
touch tests/test_transform_rotation.rs

# 2. Follow TDD workflow above
# 3. Test rotL, rotR, iterBack together
# 4. Commit when done
# 5. Move to next category: meta-transforms
```

**Quick Status Check**:
```bash
# Count verified transforms
rg "✅ FULLY VERIFIED" docs/TRANSFORM_IMPLEMENTATION_CHECKLIST.md | wc -l

# Count implemented but untested
rg "🟨 IMPLEMENTED" docs/TRANSFORM_IMPLEMENTATION_CHECKLIST.md | wc -l

# Run all transform tests
cargo test transform
```
