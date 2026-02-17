# Audit Cycle 2 Report (2026-02-17)

## Test Suite Summary

| Metric | Count |
|--------|-------|
| **Passed** | 4,614 |
| **Failed** | 1 (flaky) |
| **Ignored** | 73 |
| **Total** | 4,688 |
| **Doc-tests** | 34 passed, 269 ignored |
| **Growth** | 2.55x since baseline (1,837) |

### Single Failure: Flaky Test
- `test_gain_pattern_based` (test_gain_parameter.rs) — threshold assertion on peak amplitude
  - Sometimes passes, sometimes fails depending on timing
  - Other flaky tests observed across runs: `test_brownnoise_mean_near_zero`, `test_very_efficient_cpu`, `render_tests::test_euclidean_audio_onset_count`

### Ignored Test Breakdown (73 total)

| Category | Count | Description |
|----------|-------|-------------|
| UNIMPLEMENTED | 33 | Features not yet built (envelopes, wedge, jux, auto-routing, gate, etc.) |
| BUG | 13 | Known bugs tagged for fixing (stack overflow, silence, clicking) |
| No reason tag | 21 | Formant buffer, ADSR, dattorro, FM, etc. — need triage |
| BENCHMARK | 3 | Performance tests too slow for debug mode |
| FLAKY | 1 | Thread race condition in dataflow_graph |
| KNOWN_LIMITATION | 1 | Single-arg euclidean syntax |
| HARDWARE | 1 | Requires d.ph file |

---

## Failed Workgraph Tasks (73 failed tasks)

### Categorization of Failures

**Feature Implementation Failures (should retry):**
- `implement-loopat-pattern-2` — loopAt function
- `implement-striate-pattern-2` — striate (since fixed in a later task)
- `implement-hurry-time-2` — hurry (since fixed in implement-hurry-time-3)
- `implement-multi-output` — multi-output system (done by implement-multi-output-2)
- `implement-swing-and` — swing/groove (done by integrate-groove-quantizer)
- `implement-limiter-ugen` — limiter (done by implement-limiter-ugen-2)

**Test Fix Failures (partially addressed):**
- `fix-22-known` — 22 known-bug ignored tests (ChainInput compiler bugs)
- `fix-8-failing` — error_messages tests (fixed by fix-8-failing-2)
- `fix-9-failing` — transient_shaper_dsl tests
- `fix-11-failing` — groove_dsl tests (fixed by fix-11-failing-2)
- `fix-3-failing` — live_coding_e2e tests (fixed by fix-3-failing-2)
- `fix-test-e2e` — sample_playback tests
- `fix-flaky-test` — one_pole_filter benchmark

**Cleanup Failures:**
- `remove-14-dead` — dead tracked modules (partially done)
- `fix-103-compiler` — compiler warnings (partially done by apply-clippy-auto)
- `apply-508-clippy` — clippy auto-fixes
- `fix-unreachable-code` — unreachable code in unified_graph.rs

**Test Addition Failures:**
- `add-unit-tests-2` — compositional_compiler.rs tests (done by test_compositional_compiler_unit.rs)
- `add-unit-tests-4` — sample_loader.rs tests
- `add-dedicated-tests` — dataflow_graph.rs tests

**Other Failures:**
- `audit-and-triage` — 168 ignored tests
- `fix-targeted-improvements` — targeted improvements from audit
- `fix-slice-function` / `fix-slice-function-2` — slice produces silence

---

## Code Health

### Orphaned Files: None
All 78 modules in src/ are properly declared in lib.rs. No orphaned .rs files found.

### Untracked Files: 33 new test files
All untracked files are new test files in tests/ that need to be staged and committed.

### Dead Code
1. **Unreachable code** in `unified_graph.rs:17916-17928` — debug logging after early return
2. **Unused function** `parse_file_to_graph()` in `main.rs:353`
3. **269 ignored doc-tests** — many are stubs or examples that don't compile

### Test Coverage Gaps (Top Priority)
1. `unified_graph.rs` — 22,464 LOC, 0 inline tests (core audio engine)
2. `main.rs` — 2,424 LOC, 0 inline tests
3. `simple_dsp_executor.rs` — 612 LOC, 0 tests
4. `simple_dsp_executor_v2.rs` — 420 LOC, 0 tests
5. `engine.rs` — 468 LOC, 0 tests
6. `live.rs` — 405 LOC, 0 tests

---

## Recommended Next Tasks (Priority Order)

### P0 — Flaky Tests (quick wins)
1. Fix `test_gain_pattern_based` threshold (widen tolerance or increase sample count)
2. Fix `test_brownnoise_mean_near_zero` tolerance (already tracked in fix-flaky-test-2)
3. Fix or ignore `test_very_efficient_cpu` benchmark

### P1 — Commit Pending Work
4. Stage and commit 33 new test files + deleted files

### P2 — Bug Fixes (13 ignored BUG tests)
5. Fix recursive signal evaluation stack overflow (bitcrush pattern-controlled params)
6. Fix 3-stage feedback silence bug
7. Fix supersaw freq pattern params
8. Fix bus triggering clicking
9. Fix pattern pan low RMS
10. Fix reverb RMS reduction
11. Fix BPF Q-factor test

### P3 — Unimplemented Features (33 ignored UNIMPLEMENTED tests)
12. Implement envelope modifiers for oscillators (12 tests waiting)
13. Implement auto-routing in compiler (6 tests waiting)
14. Implement wedge function (3 tests waiting)
15. Implement jux stereo support (2 tests waiting)
16. Implement gate() in compiler
17. Implement forward references
18. Implement nested bus triggering
19. Implement audio signal to pattern conversion

### P4 — Cleanup
20. Remove unreachable code in unified_graph.rs
21. Clean up 173 dead test backup files
22. Remove unused parse_file_to_graph function
23. Triage 21 ignored tests without reason tags

### P5 — Coverage
24. Add inline tests to unified_graph.rs
25. Improve compositional_compiler.rs test coverage
