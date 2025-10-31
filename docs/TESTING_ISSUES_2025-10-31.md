# Testing Issues and Prevention Measures
## Date: 2025-10-31

## Issue 1: Integration Test Compilation Errors

### Problem
Pre-commit hook only ran `cargo test --lib`, which masked 268 integration test compilation errors. Issues accumulated as `SignalNode::Sample` and `SignalNode::Oscillator` structs evolved.

### Root Causes
1. **Incomplete test coverage in pre-commit hook**: Only validated library tests
2. **Duplicate field specifications**: Multiple instances of `pending_freq`, `last_sample`, `envelope_type`
3. **Missing required fields**: Oscillator nodes missing `pending_freq` and `last_sample`
4. **Struct evolution**: Sample nodes missing `envelope_type` field

### Fix Applied
✅ **Commit 87caa12**: Fixed all compilation errors across 18 files (10 test files, 4 example files, 2 fix scripts, 2 source files)

### Prevention Measures

#### 1. Update Pre-Commit Hook
**REQUIRED**: Update `.git/hooks/pre-commit` to run ALL tests:

```bash
#!/bin/sh
# Pre-commit hook: Run all tests before allowing commit

echo "Running cargo tests before commit..."
cargo test

if [ $? -ne 0 ]; then
    echo "❌ Tests failed! Commit aborted."
    echo "Please fix the failing tests before committing."
    exit 1
fi

echo "✅ All tests passed!"
exit 0
```

**Change from**:
```bash
cargo test --lib  # Only 300 library tests
```

**Change to**:
```bash
cargo test        # ALL tests (library + 268 integration files)
```

**Note**: This hook is local to each clone (not tracked by git). All developers must update their local `.git/hooks/pre-commit` file.

#### 2. CI/CD Validation
Add GitHub Actions workflow (if not already present) to validate all tests on PR:

```yaml
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --all-features
```

#### 3. Fix Scripts Available
Two Python scripts are now available for fixing similar issues:

- `fix_integration_tests.py`: Removes duplicate field specifications
- `fix_missing_oscillator_fields.py`: Adds missing oscillator fields

These can be run again if similar issues arise during struct evolution.

#### 4. Code Review Checklist
When modifying `SignalNode` variants:
- [ ] Update ALL usages (library + integration tests + examples)
- [ ] Run `cargo test` (not just `cargo test --lib`)
- [ ] Check all example files compile: `cargo build --examples`
- [ ] Consider running fix scripts preventatively

---

## Issue 2: Pre-Existing Onset Detection Failures

### Problem
11 tests in `test_dsl_samples_e2e.rs` fail with onset detection issues:

```
test_eight_step_pattern: Expected 8 onsets, got 7
test_hihat_subdivision_4x: Expected 5 onsets, got 3
test_hihat_subdivision_8x: Expected 8 onsets, got 3
test_extreme_subdivision: Expected 15 onsets, got 3
test_samples_chained_transforms: Expected 6 onsets, got 4
test_layered_drums: Expected 7 onsets, got 2
test_multiple_sample_buses: Expected 9 onsets, got 2
test_samples_through_bpf: Expected 7 onsets, got 4
test_samples_through_hpf: Expected 7 onsets, got 3
test_samples_through_lpf: Expected 7 onsets, got 5
test_three_sample_patterns_mixed: Expected 12 onsets, got 4
```

### Status
⚠️ **UNRESOLVED** - Pre-existing issue (not caused by compilation fixes)

### Possible Causes
1. **Onset detection threshold**: Threshold may be too high, missing quieter events
2. **Sample rendering duration**: Events may be cut off at end of render
3. **Event timing**: Events may be too close together and merged by onset detector
4. **Filter effects**: Some tests apply filters (lpf, hpf, bpf) which may reduce transients

### Investigation Needed
1. Render failing tests manually: `cargo run --bin phonon -- render <file> output.wav`
2. Analyze with wav_analyze: `cargo run --bin wav_analyze -- output.wav`
3. Visual inspection: Open in Audacity to count actual events
4. Compare onset detection parameters with working tests

### Potential Fixes
- Adjust onset detection threshold in `audio_verification_enhanced`
- Increase render duration for multi-event patterns
- Improve onset detection algorithm (spectral flux + energy)
- Add padding at end of renders to catch final events

### Temporary Workaround
These tests can be temporarily marked `#[ignore]` until fixed:

```rust
#[test]
#[ignore] // TODO: Fix onset detection - expecting 8, getting 7
fn test_eight_step_pattern() { ... }
```

---

## Results

### Fixed (Issue 1)
- ✅ All library tests pass (300/300)
- ✅ All fixed integration tests compile
- ✅ All examples compile
- ✅ Fix scripts created for future use

### Remaining (Issue 2)
- ⚠️ 11 onset detection tests fail (pre-existing, unrelated to compilation)
- 📋 Investigation needed (see section above)

---

## Lessons Learned

1. **Always run full test suite**: `cargo test`, not `cargo test --lib`
2. **Pre-commit hooks are critical**: Catch issues before they accumulate
3. **Struct evolution needs systematic updates**: Use fix scripts when possible
4. **Separate compilation from runtime issues**: Different problems need different solutions
5. **Document pre-existing issues**: Don't mix fixes for unrelated problems

---

## Action Items

- [ ] All developers: Update local pre-commit hook to run `cargo test`
- [ ] Add CI/CD workflow for PR validation (if not present)
- [ ] Investigate and fix 11 onset detection failures (Issue 2)
- [ ] Consider adding `cargo build --examples` to pre-commit hook
- [ ] Document struct evolution process for future reference
