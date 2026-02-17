# Triage: Ignored Tests Report

**Date**: 2026-02-17
**Actual count**: ~300 ignored (not 363 as estimated)

## Summary

| Category | Count | Action |
|----------|-------|--------|
| Doc-test `/// ```ignore` (src/nodes/) | ~160 | Leave as-is |
| Doc-test `/// ```phonon` (src/nodes/) | ~155 | Leave as-is |
| Doc-test `/// ```ignore` (other src/) | ~24 | Leave as-is |
| Doc-test `/// ```phonon` (other src/) | ~7 | Leave as-is |
| Doc-tests with ````rust` that FAIL | 22 | **Fix or convert to `ignore`** |
| Unit tests with `#[ignore]` | 4 | Leave as-is (well-categorized) |
| Integration tests with `#[ignore]` | 29 | Categorized below |

**Verdict: The 267 ignored doc-tests are fine. The 33 ignored unit/integration tests are well-labeled and intentional. The 22 FAILING doc-tests are a separate issue worth fixing.**

---

## Category 1: Doc-test ````ignore` in src/nodes/ (~160)

These are **API usage examples** on node structs showing how to instantiate them with `NodeId` references. They can't run as doc-tests because:
- They reference types like `OscillatorNode`, `ConstantNode` without imports
- They use `NodeId` values that assume a graph context
- They're pseudocode showing the mental model, not runnable Rust

**Example** (from `src/nodes/gain.rs`):
```rust
/// ```ignore
/// let osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let gain_amount = ConstantNode::new(0.5);          // NodeId 1
/// let gain = GainNode::new(0, 1);                    // NodeId 2
/// ```
```

**Recommendation**: **Leave as-is.** These serve their purpose as documentation. Converting them to runnable tests would require heavy boilerplate (imports, graph setup) that would obscure the example's intent. The `ignore` tag is the correct Rust convention for illustrative-only code blocks.

---

## Category 2: Doc-test ````phonon` in src/nodes/ (~155)

These show the **Phonon DSL syntax** for using each node. Rustdoc treats unknown language tags as ignored.

**Example** (from `src/nodes/oscillator.rs`):
```phonon
~osc: oscillator 440 sine
```

**Recommendation**: **Leave as-is.** These are DSL examples, not Rust code. The `phonon` language tag correctly communicates intent and gets syntax highlighting if configured.

---

## Category 3: Unit tests with `#[ignore]` (4 in lib)

All are well-categorized with descriptive reasons:

| Test | Reason | Action |
|------|--------|--------|
| `test_dataflow_graph_pipeline` | FLAKY: thread startup race | Fix the flakiness or leave |
| `test_d_ph_realtime_simulation` | HARDWARE: requires d.ph file | Leave (hardware-dependent) |
| `test_convolution_performance_under_1ms` | BENCHMARK: 4s in debug mode | Leave (benchmark) |
| `test_waveguide_frequency_controls_pitch` | BUG: high frequency handling | Fix the bug or leave |

**Recommendation**: **Leave as-is.** These are intentionally skipped with clear reasons.

---

## Category 4: Integration tests with `#[ignore]` (29)

Broken down by category:

### UNIMPLEMENTED (14 tests) - future features
| Test | Feature needed |
|------|---------------|
| `test_elongate_operator` | Elongate operator `_` in mini-notation |
| `test_division_edge_cases` | Mini-notation edge cases |
| `test_elongate_with_silence` | Mini-notation edge cases |
| `test_euclidean_edge_cases` | Mini-notation edge cases |
| `test_random_choice_consistency` | Mini-notation edge cases |
| `test_additive_pattern_amplitudes` | Pattern-modulated harmonic amplitudes |
| `test_backwards_compatibility_out_bus` | Auto-routing in compositional compiler |
| `test_d_pattern_auto_routing` | Auto-routing in compositional compiler |
| `test_explicit_master_overrides_auto_routing` | Auto-routing in compositional compiler |
| `test_mixed_d_and_out_pattern` | Auto-routing in compositional compiler |
| `test_non_matching_buses_dont_auto_route` | Auto-routing in compositional compiler |
| `test_out_pattern_auto_routing` | Auto-routing in compositional compiler |
| `test_bus_reference_nested` | Nested bus triggering |
| `test_forward_reference` | Forward references in compiler |

### BUG (8 tests) - known issues
| Test | Bug |
|------|-----|
| `test_complex_patch` | Multiline DSL parsing fails |
| `test_alternation_in_sequence` | Alternation cycle counter not advancing |
| `test_complex_pattern_with_alternation` | Complex alternation counter bug |
| `test_multi_stage_feedback_3_stages` | 3-stage feedback produces silence |
| `test_architectural_limitation_drum_synths_continuous` | Superkick synth producing no audio |
| `test_supersaw_freq_pattern_actually_cycles` | Supersaw freq pattern params not working |
| `test_user_case_no_clicking` | Bus triggering via s pattern has clicking |
| `test_many_buses` | Stack overflow with 8 buses |
| `test_reverb_increases_overall_amplitude` | Reverb reduces RMS instead of increasing |

### KNOWN_LIMITATION (1 test)
| Test | Issue |
|------|-------|
| `test_euclidean_default_steps` | Single-arg euclidean not standard syntax |

### UNIMPLEMENTED (other, 3 tests)
| Test | Feature needed |
|------|---------------|
| `test_jux_transform` | Jux requires stereo pattern support |
| `test_jux_with_chained_transforms` | Jux requires stereo pattern support |
| `test_pattern_assignment_from_bus` | Audio signal to pattern conversion |
| `test_gate_reduces_quiet_signals` | `gate()` in compositional compiler |

### BENCHMARK (1 test)
| Test | Reason |
|------|--------|
| `test_feedback_performance_multiple_loops` | Performance test, slow in debug mode |

**Recommendation**: **Leave as-is.** All are well-categorized. The BUG ones serve as regression tests for when those bugs get fixed. The UNIMPLEMENTED ones are effectively a feature backlog encoded as tests.

---

## Category 5: Failing doc-tests (22) - SEPARATE ISSUE

These are `/// ```rust` blocks (or bare `/// ````) that actually attempt to compile and run but fail. They are NOT ignored - they show as failures.

**Files with failing doc-tests**:
- `src/lib.rs` (4 failures)
- `src/unified_graph.rs` (7 failures)
- `src/dataflow_graph.rs` (4 failures)
- `src/voice_manager.rs` (3 failures)
- `src/groove.rs` (1 failure)
- `src/onset_timing.rs` (1 failure)
- `src/compositional_parser.rs` (1 failure)
- `src/nodes/fdn_reverb.rs` (1 failure - compile error)

**Recommendation**: **Create a separate task to fix these.** Options:
1. Convert to `/// ```ignore` if they're just illustrative
2. Fix the imports/setup to make them actually compile
3. Convert to `/// ```no_run` if they compile but can't run in test context

---

## Overall Recommendation

**No action needed for the 267 ignored doc-tests or 33 ignored unit/integration tests.** They are all correctly categorized and serve their documentation/backlog purpose.

The only actionable finding is the **22 failing doc-tests** (a separate issue from the "ignored" triage).
