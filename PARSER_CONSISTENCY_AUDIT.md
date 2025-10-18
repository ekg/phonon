# Parser Consistency Audit - Critical Gaps

## ⚠️ **CRITICAL**: Three Different Parsers Across Modes

You're absolutely right to be concerned. Phonon currently has **three different code paths** that behave differently:

### Current State (INCONSISTENT)

| Mode | Parser Used | Auto-Routing | Status |
|------|------------|--------------|--------|
| **Render** (`phonon render`) | `parse_dsl` + `DslCompiler` | ✅ YES | ✅ Tested (204 tests) |
| **OSC Server** (`/eval`) | `parse_dsl` + `DslCompiler` | ✅ YES | ✅ Tested (16 tests) |
| **Live File Watch** (`phonon live`) | Custom manual parser | ❌ NO | ⚠️ Untested |
| **Modal Editor** (`phonon edit`) | `parse_glicol` (old Glicol) | ❌ NO | ⚠️ Untested |

### The Problem

**File**: `src/live_engine.rs:158`
```rust
match parse_glicol(&clean_code) {  // ❌ OLD PARSER!
```

**File**: `src/main.rs:1535-1800`
```rust
fn parse_expression(...) {  // ❌ CUSTOM PARSER!
    // 265 lines of custom parsing logic
    // Completely different from DslCompiler
}
```

This means:
- ❌ Code that works in `phonon render` may fail in `phonon live`
- ❌ Auto-routing (`~d1`, `~d2`) works in Render/OSC but NOT in Live/Edit modes
- ❌ Syntax differences between modes = user confusion
- ❌ Test coverage only validates one code path

## Missing from Tested Vision

### 1. **Unified Parser Across All Modes** ⚠️ CRITICAL
**Gap**: Live and Edit modes use different parsers
**Impact**: Same code behaves differently in different contexts
**Fix Required**: Refactor Live and Edit modes to use `parse_dsl` + `DslCompiler`

### 2. **Auto-Routing in Live Modes** ⚠️ HIGH PRIORITY
**Gap**: Auto-routing (`~d1`, `~out1` → master) only works in Render/OSC
**Impact**: Users can't use TidalCycles-style `d1, d2, d3` in live mode
**Fix Required**: Live/Edit modes need DslCompiler integration

### 3. **Integration Tests Across Modes** ⚠️ HIGH PRIORITY
**Gap**: No tests verify same code works identically in all modes
**Impact**: Parser divergence can go undetected
**Tests Needed**:
- ✅ Render mode: 204 tests passing
- ✅ OSC mode: 16 tests passing
- ❌ Live file watch mode: 0 tests
- ❌ Edit mode: 0 tests
- ❌ Cross-mode consistency: 0 tests

### 4. **Feature Parity Testing** ⚠️ MEDIUM PRIORITY
**Gap**: Each feature needs testing in ALL modes

| Feature | Render | OSC | Live | Edit |
|---------|--------|-----|------|------|
| Auto-routing (~d1) | ✅ | ✅ | ❌ | ❌ |
| Pattern transforms ($) | ✅ | ✅ | ✅ | ❓ |
| Effects (lpf, reverb) | ✅ | ✅ | ❓ | ❓ |
| Synthesis (sine, saw) | ✅ | ✅ | ✅ | ❓ |
| Sample playback (s()) | ✅ | ✅ | ✅ | ❓ |
| Bus routing (~bus) | ✅ | ✅ | ❓ | ❓ |
| CPS/tempo | ✅ | ✅ | ❓ | ❓ |

### 5. **Live Mode OSC Integration** ⚠️ MEDIUM PRIORITY
**Gap**: OSC server runs separately, not integrated with `phonon live`
**Impact**: Can't use OSC + file watching simultaneously
**Fix**: Add `--osc` flag to `phonon live` to start OSC server alongside file watch

### 6. **Error Consistency** ⚠️ LOW PRIORITY
**Gap**: Different parsers give different error messages for same syntax
**Impact**: Confusing for users debugging
**Fix**: Unified error reporting from DslCompiler

## Recommended Action Plan

### Phase 1: Unify Parsers (CRITICAL)
1. ✅ **DONE**: Implement `parse_dsl` + `DslCompiler` (works in Render/OSC)
2. ❌ **TODO**: Refactor `LiveEngine` to use `DslCompiler` instead of `parse_glicol`
3. ❌ **TODO**: Refactor `phonon live` custom parser to use `DslCompiler`
4. ❌ **TODO**: Update `ModalEditor` to use `DslCompiler`

### Phase 2: Test Coverage (HIGH PRIORITY)
1. ❌ **TODO**: Create cross-mode consistency test suite
2. ❌ **TODO**: Test same .ph file in all 4 modes
3. ❌ **TODO**: Verify auto-routing works in all modes
4. ❌ **TODO**: Feature parity matrix testing

### Phase 3: Integration (MEDIUM PRIORITY)
1. ❌ **TODO**: Add OSC server to `phonon live` mode
2. ❌ **TODO**: Add OSC server to `phonon edit` mode
3. ❌ **TODO**: Unified command-line interface across modes

### Phase 4: Documentation (LOW PRIORITY)
1. ❌ **TODO**: Document that all modes use identical parser
2. ❌ **TODO**: Example .ph files tested in all modes
3. ❌ **TODO**: Migration guide from old parser

## Specific Code Changes Needed

### 1. Fix LiveEngine (src/live_engine.rs:143-180)
**Replace**:
```rust
match parse_glicol(&clean_code) {  // OLD
    Ok(env) => {
        match executor.render(&env, cycle_duration) {
```

**With**:
```rust
use crate::unified_graph_parser::{parse_dsl, DslCompiler};

match parse_dsl(&clean_code) {  // NEW
    Ok((_, statements)) => {
        let compiler = DslCompiler::new(sample_rate);
        let mut graph = compiler.compile(statements);
        let buffer = graph.render((cycle_duration * sample_rate) as usize);
```

### 2. Fix Live Mode (src/main.rs:1439-2400)
**Replace**: 265 lines of custom `parse_expression` logic

**With**:
```rust
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

match parse_dsl(&file_content) {
    Ok((_, statements)) => {
        let compiler = DslCompiler::new(sample_rate);
        let mut graph = compiler.compile(statements);
        // Use graph...
    }
}
```

### 3. Test Harness for Cross-Mode Consistency
**New file**: `tests/test_cross_mode_consistency.rs`

```rust
#[test]
fn test_same_code_all_modes() {
    let code = r#"
cps: 2.0
~d1: saw 110
~d2: saw 220
"#;

    // Test 1: Render mode
    let render_output = test_render_mode(code);
    assert!(has_audio(&render_output));

    // Test 2: OSC mode
    let osc_output = test_osc_mode(code);
    assert!(has_audio(&osc_output));

    // Test 3: Live mode
    let live_output = test_live_mode(code);
    assert!(has_audio(&live_output));

    // Test 4: Verify outputs match
    assert_eq!(render_output, live_output,
        "Render and Live modes must produce identical output");
}
```

## Summary: What's Missing

1. **Parser Unification**: Live and Edit modes need to use `DslCompiler`
2. **Auto-Routing Everywhere**: All modes need auto-routing support
3. **Comprehensive Testing**: Cross-mode test suite with parity matrix
4. **Integration**: OSC server in all modes, not just standalone
5. **Documentation**: Clear statement that all modes are identical

## Current Test Coverage

```
✅ Render mode: 204/204 tests (100%)
✅ OSC server: 16/18 tests (89%)
❌ Live file watch: 0 tests
❌ Edit mode: 0 tests
❌ Cross-mode: 0 tests

Total coverage: ~40% (only 2 of 4 modes tested)
```

## Risk Assessment

**WITHOUT PARSER UNIFICATION**:
- Users will encounter syntax that works in one mode but not another
- Bug reports will be confusing ("works in render but not live")
- Auto-routing feature is half-implemented (only in 2/4 modes)
- Technical debt will compound as features diverge further

**WITH PARSER UNIFICATION**:
- One codebase, one parser, one test suite
- Features automatically work in all modes
- Easy to maintain and extend
- Clear mental model for users

## Recommendation

**Priority 1**: Unify parsers NOW before adding any new features. Every new feature added to `DslCompiler` will need to be duplicated in the custom parsers, increasing technical debt.

**Next Steps**:
1. Refactor `LiveEngine` to use `DslCompiler` (1-2 hours)
2. Refactor `phonon live` to use `DslCompiler` (2-3 hours)
3. Create cross-mode test suite (1 hour)
4. Verify auto-routing works in all modes (30 min)
5. Document unified behavior (30 min)

**Total effort**: ~5-7 hours to achieve full parser consistency
