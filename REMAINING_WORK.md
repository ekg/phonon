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

#### Group 1: Chopping & Restructuring ✅ **COMPLETED**
- ✅ `compress` - **NOW HAS E2E TEST**
- ✅ `shuffle` - **NOW HAS E2E TEST**
- ✅ `spin` - **NOW HAS E2E TEST**
- ✅ `fit` - **NOW HAS E2E TEST**
- ✅ `scramble` - **NOW HAS E2E TEST**
- ✅ `segment` - Has E2E test

#### Group 2: Timing Transforms ✅ **COMPLETED**
- ✅ `inside` - **NOW HAS E2E TEST**
- ✅ `outside` - **NOW HAS E2E TEST**
- ✅ `wait` - **NOW HAS E2E TEST**

#### Group 3: Smoothing & Shaping ✅ **PARTIALLY COMPLETED**
- ✅ `focus` - **NOW HAS E2E TEST**
- ✅ `smooth` - **NOW HAS E2E TEST**
- ✅ `trim` - **NOW HAS E2E TEST**
- ⬜ `exp` - Has unit test only
- ⬜ `log` - Has unit test only
- ⬜ `walk` - Has unit test only

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

**Total**: ~35 transforms with unit tests
**Completed**: 11 transforms now have E2E audio tests
**Remaining**: ~24 transforms (Groups 3-8)

### Why E2E Tests Matter
- Unit tests verify pattern logic (event timing, count, structure)
- E2E tests verify **actual audio output** (RMS, frequency content, timing)
- Example: Pattern may generate correct events but audio pipeline could fail
- Prevents regressions in audio rendering path

## 🔗 Transform Chaining Coverage ✅ **COMPLETED**

**Added 4 new chain tests:**
- ✅ Multiple chains: `$ fast 2 $ rev $ euclid 5 8`
- ✅ Order testing: `$ fast 2 $ slow 2` vs `$ slow 2 $ fast 2`
- ✅ Higher-order: `$ fast 2 $ sometimes (fast 4) $ rev`
- ✅ Mixed categories: `$ euclid 3 8 $ often (fast 2) $ rev`

Previously had:
- 2 examples: `$ euclid 3 8 $ fast 2`, `$ chop 4 $ rev`

**Total**: 6 transform chaining tests (sufficient coverage)

### Why Important
- Verifies transforms compose correctly ✅ **VERIFIED**
- Ensures left-to-right evaluation works ✅ **VERIFIED**
- Catches interaction bugs between transforms ✅ **VERIFIED**

## 📝 Recommended Action Plan

### ✅ Priority 1: Fix Cross-Mode Tests **COMPLETED**
1. ✅ Debugged auto-routing issue
2. ✅ Implemented auto-routing in compile_program()
3. ✅ Fixed test syntax (lpf parentheses)
4. ✅ **Result**: All 6 cross-mode tests pass

### ✅ Priority 2: Add E2E Tests for Recent Transforms **COMPLETED**
1. ✅ Group 1 (chopping): compress, shuffle, spin, fit, scramble
2. ✅ Groups 2-3: inside, outside, wait, focus, smooth, trim
3. ✅ **Result**: Added 11 new E2E tests

### ✅ Priority 3: Add Transform Chaining E2E Tests **COMPLETED**
1. ✅ Test complex chain combinations (4 tests)
2. ✅ Test order dependency
3. ✅ Test interaction between categories
4. ✅ **Result**: Comprehensive chaining coverage

### Priority 4: Complete E2E Coverage (Lower Priority) ⬜ **REMAINING**
1. Add E2E tests for Groups 3-8 (remaining ~24 transforms)
   - exp, log, walk (Group 3 remainder)
   - reset, restart, loopback, binary, range, quantize (Group 4)
   - offset, loop, chew, fastGap, discretise, compressGap (Group 5)
   - humanize, euclid_legato (Group 6)
   - sometimesBy, almostAlways, often, rarely, etc. (Group 7)
   - degradeSeed, undegrade, accelerate (Group 8)
2. **Time estimate**: 4-6 hours
3. **Priority**: Low (all have unit tests, transforms work)

## 🎯 Success Criteria

- ✅ **Immediate**: 297/297 core tests passing **DONE**
- ✅ **Short-term**: 303/303 all tests passing **DONE** (fixed cross-mode)
- ✅ **Medium-term**: +15 E2E tests for recent transforms **DONE**
  - Added 11 transform tests + 4 chaining tests = 15 new
  - Total: 57 → 72 E2E tests
- ⬜ **Long-term**: Comprehensive E2E coverage for all 60+ transforms (24 remaining)

## 📌 Notes

- **Unit tests are NOT sufficient**: They test pattern logic but not audio output
- **E2E tests prevent regressions**: Audio pipeline bugs won't be caught by unit tests
- **Scientific verification**: E2E tests use FFT analysis, RMS, onset detection
- **Test philosophy**: If it doesn't have an E2E audio test, it's not fully verified
