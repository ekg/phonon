# Test Failures Analysis
Date: 2025-10-16

## Summary

**Lib tests**: 208/208 passing ✅
**Integration tests**: Multiple failures identified

## Critical Findings

### Pattern Transform Tests (6 tests failing)
**Status**: All produce RMS=0 despite CLI working correctly

**Evidence**:
- CLI render: `phonon render test.ph` → RMS: 0.141 ✅
- Test using DslCompiler → RMS: 0.000 ❌

**Tests failing**:
1. test_fast_transform_produces_audio
2. test_slow_transform_syntax
3. test_rev_transform_debug
4. test_every_transform_produces_audio
5. test_chained_transforms
6. test_bus_reference_with_transform

**Root cause hypothesis**:
Either:
1. Pattern transforms not being compiled to graph nodes correctly
2. Tempo (`SetCps`) statements not being processed
3. Sample paths not configured in test environment

**Comparison with working tests** (test_sample_integration.rs):
- Working tests explicitly call `graph.set_cps(1.0)`
- Failing tests rely on `tempo 2.0` in DSL

**Next steps**:
1. Verify `tempo` statement is parsed to `SetCps` and compiled correctly
2. Check if `PatternTransform` expressions are compiled to appropriate nodes
3. Try explicit `graph.set_cps()` calls in failing tests as workaround

### test_effects_comprehensive.rs (Won't compile)
**Status**: 22 compilation errors

**Errors**:
- 12× `SignalNode::Sine` doesn't exist (use `Oscillator` instead)
- 8× Missing `state` field in effect nodes
- 1× `Delay` missing `buffer` and `write_idx` fields
- 1× `Bitcrush` should be `BitCrush` (capitalization)

**Fix**: Update test to use current SignalNode API or mark as ignored

### test_sample_integration.rs (1 test failing)
**Status**: 11/12 tests passing

**Need to investigate**: Which specific test is failing

### test_pattern_transforms.rs (8 tests failing)
**Status**: 5/13 tests passing

**Need to investigate**: Which transforms are broken

## Operator Syntax Changes

### ✅ Completed
- Changed `#` → `#` for signal chains
- Changed `$` → `$` for pattern transforms
- All parser tests pass (15/15)
- All lib tests pass (208/208)
- Examples updated with new syntax

### ⚠️ Semantic Issue
**Phonon's `$` is REVERSED from Tidal:**
- Tidal: `fast 2 $ sound "bd"` (function-first)
- Phonon: `s("bd") $ fast 2` (data-first, like Unix pipe)

This is intentional but needs documentation.

## Action Plan

### Priority 1: Fix Pattern Transform Tests (HIGH)
**Estimated**: 2-4 hours

1. Add debug output to see if `tempo` statements are compiled
2. Check if `PatternTransform` expressions compile to nodes
3. Try explicit `set_cps()` workaround
4. If workaround works, investigate why DSL tempo isn't working

### Priority 2: Investigate Other Test Failures
**Estimated**: 2-3 hours

1. Identify which test in test_sample_integration fails
2. Identify which 8 tests in test_pattern_transforms fail
3. Categorize by root cause

### Priority 3: Fix or Ignore test_effects_comprehensive
**Estimated**: 1-2 hours

Options:
1. Update to current API (20+ changes)
2. Mark as `#[ignore]` with TODO comment
3. Delete if redundant with other effect tests

Recommendation: Mark as ignored for now, revisit later

### Priority 4: Document Everything
**Estimated**: 2-3 hours

1. Update ROADMAP.md with operator changes
2. Document `$` semantic difference from Tidal
3. Update all markdown docs to use `#` and `$`
4. Create troubleshooting guide for common issues

### Priority 5: Comprehensive Test Audit
**Estimated**: 4-6 hours

1. Run all 116 test files systematically
2. Categorize: pass / fail / compile-error
3. Create test status matrix
4. Identify patterns in failures

### Priority 6: Implement Missing Features (with TDD)
**Estimated**: 1-2 weeks

1. Pattern DSP Parameters (gain, pan, speed, cut) - 2-3 days
2. Additional effects (reverb, delay, distortion) - 2-3 days
3. Documentation updates - 1-2 days

## Test Environment Issues

**Hypothesis**: Tests may need:
- Sample path configuration
- Explicit CPS setup
- Voice manager initialization

**Evidence from working tests**:
```rust
let mut graph = UnifiedSignalGraph::new(44100.0);
graph.set_cps(1.0);  // ← Explicit CPS setting
```

**Failing tests**:
```rust
let compiler = DslCompiler::new(44100.0);
let mut graph = compiler.compile(statements);  // ← Relies on DSL tempo
```

## Recommendations

1. **Fix pattern transform tests FIRST** - they're blocking confidence in core feature
2. **Don't spend time on test_effects_comprehensive** - old API, mark as ignored
3. **Document operator semantics clearly** - prevent user confusion
4. **Complete test audit** - understand full scope before new features
5. **Use TDD for all new features** - write tests first, implement second

## Questions to Answer

1. Does `tempo` in DSL actually call `graph.set_cps()`?
2. Are `PatternTransform` expressions compiled to any graph nodes?
3. Why does CLI work but DslCompiler tests don't?
4. Should we keep `$` reversed from Tidal or flip it?

## Files to Check

- `src/unified_graph_parser.rs` - DslCompiler implementation
  - Line ~1220: `fn compile_statement` - check `SetCps` handling
  - Line ~1300: `fn compile_expression` - check `PatternTransform` handling
- `tests/test_pattern_transform_integration.rs` - failing tests
- `tests/test_sample_integration.rs` - working test examples
