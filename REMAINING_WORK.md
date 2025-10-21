# Phonon: Remaining Work & Coverage Gaps

**Date**: 2025-10-20
**Status**: 297/303 tests passing (6 cross-mode tests failing)

## ✅ What Was Fixed

### Parser Issues (Just Fixed)
- **Problem**: Tests were using `unified_graph_parser::parse_dsl` instead of `compositional_parser::parse_program`
- **Impact**: 6 tests failing with "RMS = 0" (chop, gap, segment, chunk transforms)
- **Solution**: Updated tests to use the correct parser (same as phonon binary)
- **Result**: All 297 core library tests now pass ✅

## ⚠️  Remaining Issues

### 1. Cross-Mode Consistency Tests (6 failures)
**File**: `tests/test_cross_mode_consistency.rs`

**Failing Tests**:
- `test_auto_routing_cross_mode`
- `test_synthesis_cross_mode`
- `test_effects_cross_mode`
- `test_pattern_params_cross_mode`
- `test_bus_routing_cross_mode`
- `test_unified_vision_same_file_all_modes`

**Issue**: Tests run `phonon render` command and expect audio output, but get silence.

**Why Failing**:
- These are integration tests that test the phonon binary itself
- Tests write `.ph` files and invoke `cargo run --bin phonon -- render`
- The binary should produce audio but tests report "Render mode produced no audio"
- Likely related to auto-routing logic or test configuration

**Next Steps**:
1. Investigate why phonon binary produces no audio for test files
2. Check if auto-routing is working in render mode
3. Verify tempo/cps handling in cross-mode scenarios
4. May need to update test assertions or fix auto-routing

## 📊 E2E Test Coverage Gaps

### Missing E2E Audio Rendering Tests
Based on git history, these transforms have **unit tests** but **NO E2E audio tests**:

#### Group 1: Chopping & Restructuring (Recently Implemented)
- ✅ `compress` - Has unit test
- ✅ `shuffle` - Has unit test
- ✅ `spin` - Has unit test
- ✅ `fit` - Has unit test
- ✅ `scramble` - Has unit test
- ✅ `segment` - Now has E2E test (just added)

#### Group 2: Timing Transforms
- ✅ `inside` - Has unit test
- ✅ `outside` - Has unit test
- ✅ `wait` - Has unit test

#### Group 3: Smoothing & Shaping
- ✅ `focus` - Has unit test
- ✅ `smooth` - Has unit test
- ✅ `trim` - Has unit test
- ✅ `exp` - Has unit test
- ✅ `log` - Has unit test
- ✅ `walk` - Has unit test

#### Group 4: Advanced Pattern Ops
- ✅ `reset` - Has unit test
- ✅ `restart` - Has unit test
- ✅ `loopback` - Has unit test
- ✅ `binary` - Has unit test
- ✅ `range` - Has unit test
- ✅ `quantize` - Has unit test

#### Group 5: Timing & Gaps
- ✅ `offset` - Has unit test
- ✅ `loop` - Has unit test
- ✅ `chew` - Has unit test
- ✅ `fastGap` - Has unit test
- ✅ `discretise` - Has unit test
- ✅ `compressGap` - Has unit test

#### Group 6: Pattern Variations
- ✅ `humanize` - Has unit test
- ✅ `euclid_legato` - Has unit test

#### Group 7: Probability Transforms
- ✅ `sometimesBy` - Has unit test
- ✅ `almostAlways` - Has unit test
- ✅ `almostNever` - Has unit test
- ✅ `always` - Has unit test
- ✅ `whenmod` - Has unit test
- ✅ `often` - Has unit test
- ✅ `rarely` - Has unit test

#### Group 8: Sample Modulation
- ✅ `degradeSeed` - Has unit test
- ✅ `undegrade` - Has unit test
- ✅ `accelerate` - Has unit test

**Total**: ~35 transforms with unit tests but no E2E audio verification

### Why E2E Tests Matter
- Unit tests verify pattern logic (event timing, count, structure)
- E2E tests verify **actual audio output** (RMS, frequency content, timing)
- Example: Pattern may generate correct events but audio pipeline could fail
- Prevents regressions in audio rendering path

## 🔗 Transform Chaining Coverage

Currently E2E tests have:
- 2 examples of chained transforms: `$ euclid 3 8 $ fast 2`
- 1 test for `$ chop 4 $ rev`

### Missing Chain Tests
- Multiple chains: `$ fast 2 $ rev $ slow 2 $ sometimes (fast 4)`
- Order dependency: Does `$ fast 2 $ rev` differ from `$ rev $ fast 2`?
- Complex nested chains with higher-order transforms
- Chains with different transform categories (timing + restructuring + probability)

### Why Important
- Verifies transforms compose correctly
- Ensures left-to-right evaluation works
- Catches interaction bugs between transforms

## 📝 Recommended Action Plan

### Priority 1: Fix Cross-Mode Tests (High Impact)
1. Debug why phonon binary produces no audio for test files
2. Check auto-routing in render mode
3. Update tests or fix binary issue
4. **Time estimate**: 2-4 hours

### Priority 2: Add E2E Tests for Recent Transforms (Medium Impact)
Focus on transforms implemented in last month:
1. Group 1 (chopping): compress, shuffle, spin, fit, scramble ✅ segment done
2. Groups 2-3: inside, outside, wait, focus, smooth, trim
3. **Time estimate**: 4-6 hours (15-20 tests)

### Priority 3: Add Transform Chaining E2E Tests (Medium Impact)
1. Test 5-10 complex chain combinations
2. Test order dependency
3. Test interaction between transform categories
4. **Time estimate**: 2-3 hours

### Priority 4: Complete E2E Coverage (Lower Priority)
1. Add E2E tests for Groups 4-8 (remaining 20+ transforms)
2. **Time estimate**: 6-8 hours

## 🎯 Success Criteria

- ✅ **Immediate**: 297/297 core tests passing (DONE!)
- ⬜ **Short-term**: 303/303 all tests passing (fix cross-mode)
- ⬜ **Medium-term**: +35 E2E tests for recent transforms
- ⬜ **Long-term**: Comprehensive E2E coverage for all 60+ transforms

## 📌 Notes

- **Unit tests are NOT sufficient**: They test pattern logic but not audio output
- **E2E tests prevent regressions**: Audio pipeline bugs won't be caught by unit tests
- **Scientific verification**: E2E tests use FFT analysis, RMS, onset detection
- **Test philosophy**: If it doesn't have an E2E audio test, it's not fully verified
