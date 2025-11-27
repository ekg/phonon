# Pattern Parameter Verification Plan
**Date:** 2025-11-22
**Goal:** Systematically verify ALL parameters accept patterns across ALL 137 nodes
**Status:** 🎯 Planning Phase

---

## Executive Summary

We claim: "Every parameter in Phonon can be modulated by patterns."

**Prove it.** Build a comprehensive test suite that verifies:
1. ✅ Every node parameter accepts pattern inputs
2. ✅ Pattern modulation produces correct audio output
3. ✅ All four pattern input types work (constant, pattern string, bus, pattern ref)
4. ✅ Audio-rate modulation actually happens

---

## Testing Strategy: The Parameter Matrix

### Test Dimensions

For EACH parameter of EACH node, verify:

| Input Type | Example | Test Coverage |
|------------|---------|---------------|
| **Constant** | `lpf 1000 0.8` | Baseline - must work |
| **Pattern String** | `lpf "500 1000 2000" 0.8` | Pattern evaluation |
| **Bus Reference** | `~lfo: sine 0.5; lpf ~lfo 0.8` | Audio→param bridge |
| **Pattern Reference** | `%cutoffs: "500 1000"; lpf %cutoffs 0.8` | Pattern→pattern |
| **Arithmetic** | `lpf (~lfo * 2000 + 500) 0.8` | Expression eval |

### Verification Levels (Three-Level Testing)

**Level 1: Pattern Query** - Does pattern produce correct events?
**Level 2: Onset Detection** - Does audio match expected timing?
**Level 3: Audio Characteristics** - Does modulation affect sound correctly?

---

## Phase 1: Node Inventory & Categorization

### Task 1.1: Catalog All Nodes by Category

Generate complete inventory with parameter counts:

```bash
# Script to generate node inventory
for node in src/nodes/*.rs; do
    echo "$(basename $node .rs): $(grep -c "NodeId\|Signal" $node) parameters"
done > /tmp/node_inventory.txt
```

**Expected Output:**
```
Oscillators (7 nodes, ~1-3 params each)
Filters (12 nodes, ~2-4 params each)
Envelopes (5 nodes, ~3-5 params each)
Effects (40+ nodes, ~2-8 params each)
Modulation (10 nodes, ~1-3 params each)
Utility (20+ nodes, ~1-4 params each)
Analysis (5 nodes, ~1-2 params each)
Logic (10 nodes, ~2 params each)
```

**Deliverable:** `docs/NODE_PARAMETER_INVENTORY.md`

### Task 1.2: Create Parameter Matrix

For each node, document:
- Node name
- Parameter names
- Parameter types (Signal, NodeId, Pattern<T>)
- Current test coverage
- Missing test coverage

**Deliverable:** `docs/PARAMETER_TEST_MATRIX.csv`

---

## Phase 2: Tier 1 - Representative Sampling (QUICK WIN)

**Goal:** Prove the concept with 20 representative nodes (one from each category)

### Selection Criteria

Pick nodes that represent different parameter patterns:
- **1 param:** Absolute, Not
- **2 params:** Addition, Multiplication, LowPass (simplified)
- **3 params:** LowPass (full), Reverb (simplified)
- **4 params:** BandPass, Compressor
- **5+ params:** ADSR, Reverb (full), Delay (full)

### Test Template: Tier 1 Node Test

```rust
// tests/test_pattern_params_tier1.rs

#[test]
fn test_lowpass_all_parameters_accept_patterns() {
    use phonon::nodes::LowPassFilterNode;

    // Test 1: Constant parameters (baseline)
    let constant_audio = render_dsl(r#"
        tempo: 0.5
        ~saw: saw 110
        out: ~saw # lpf 1000 0.8
    "#, 4);
    assert!(constant_audio.rms > 0.01, "Baseline must produce audio");

    // Test 2: Pattern string on cutoff
    let pattern_cutoff = render_dsl(r#"
        tempo: 0.5
        ~saw: saw 110
        out: ~saw # lpf "500 1000 2000" 0.8
    "#, 4);

    // Verify modulation happened (spectral changes across cycles)
    let cycle1 = &pattern_cutoff.audio[0..11025];
    let cycle2 = &pattern_cutoff.audio[22050..33075];
    assert_spectral_difference(cycle1, cycle2, 0.1);

    // Test 3: Bus reference on cutoff (audio-rate modulation)
    let bus_cutoff = render_dsl(r#"
        tempo: 0.5
        ~lfo: sine 0.5
        ~saw: saw 110
        out: ~saw # lpf (~lfo * 1000 + 500) 0.8
    "#, 8);

    // Verify continuous modulation (not just stepped)
    assert_continuous_modulation(&bus_cutoff.audio, 44100.0);

    // Test 4: Pattern string on Q
    let pattern_q = render_dsl(r#"
        tempo: 0.5
        ~saw: saw 110
        out: ~saw # lpf 1000 "0.5 2.0 5.0"
    "#, 4);

    // Q changes should affect resonance peak
    assert_spectral_difference(&pattern_q.audio[0..11025],
                               &pattern_q.audio[22050..33075], 0.15);

    // Test 5: Both parameters modulated simultaneously
    let both_modulated = render_dsl(r#"
        tempo: 0.5
        ~lfo1: sine 0.25
        ~lfo2: sine 0.33
        ~saw: saw 110
        out: ~saw # lpf (~lfo1 * 1000 + 500) (~lfo2 * 3 + 1)
    "#, 8);

    assert!(both_modulated.rms > 0.01, "Dual modulation must work");
}
```

### Tier 1 Test Matrix (20 nodes)

| Category | Node | Params | Test File | Status |
|----------|------|--------|-----------|--------|
| **Source** | Sine | 1 (freq) | test_pattern_params_sources.rs | ⏳ |
| **Source** | Saw | 1 (freq) | test_pattern_params_sources.rs | ⏳ |
| **Source** | WhiteNoise | 0 | test_pattern_params_sources.rs | ⏳ |
| **Filter** | LowPass | 2 (cutoff, q) | test_pattern_params_filters.rs | ⏳ |
| **Filter** | HighPass | 2 (cutoff, q) | test_pattern_params_filters.rs | ⏳ |
| **Filter** | BandPass | 3 (freq, q, gain) | test_pattern_params_filters.rs | ⏳ |
| **Envelope** | ADSR | 4 (a, d, s, r) | test_pattern_params_envelopes.rs | ⏳ |
| **Envelope** | AR | 2 (a, r) | test_pattern_params_envelopes.rs | ⏳ |
| **Effect** | Reverb | 3 (room, damp, mix) | test_pattern_params_effects.rs | ⏳ |
| **Effect** | Delay | 3 (time, feedback, mix) | test_pattern_params_effects.rs | ⏳ |
| **Effect** | Compressor | 5 (thr, ratio, atk, rel, makeup) | test_pattern_params_effects.rs | ⏳ |
| **Effect** | Distortion | 2 (drive, mix) | test_pattern_params_effects.rs | ⏳ |
| **Modulation** | LFO | 2 (rate, depth) | test_pattern_params_modulation.rs | ⏳ |
| **Utility** | Gain | 1 (amount) | test_pattern_params_utility.rs | ⏳ |
| **Utility** | Pan | 1 (position) | test_pattern_params_utility.rs | ⏳ |
| **Utility** | Mix | 2 (a, b) | test_pattern_params_utility.rs | ⏳ |
| **Math** | Addition | 2 (a, b) | test_pattern_params_math.rs | ⏳ |
| **Math** | Multiply | 2 (a, b) | test_pattern_params_math.rs | ⏳ |
| **Logic** | Comparator | 2 (a, b) | test_pattern_params_logic.rs | ⏳ |
| **Analysis** | RMS | 1 (window) | test_pattern_params_analysis.rs | ⏳ |

**Success Criteria:** All 20 nodes, all parameters, all 5 input types tested → **100 test functions**

---

## Phase 3: Tier 2 - Comprehensive Coverage (ALL 137 Nodes)

### Auto-Generation Strategy

**Don't write tests manually.** Generate them from metadata.

#### Step 3.1: Extract Node Metadata

Create build script to parse all node files:

```rust
// build.rs additions

struct NodeMetadata {
    name: String,
    params: Vec<ParamInfo>,
    category: String,
}

struct ParamInfo {
    name: String,
    param_type: String, // "Signal", "NodeId", "Pattern<f64>"
}

fn extract_all_node_metadata() -> Vec<NodeMetadata> {
    // Parse src/nodes/*.rs
    // Extract struct definitions
    // Parse parameter types
    // Generate metadata JSON
}
```

**Output:** `target/node_metadata.json`

#### Step 3.2: Generate Test Files

```rust
// scripts/generate_param_tests.rs

fn generate_tests_for_node(node: &NodeMetadata) -> String {
    format!(r#"
#[test]
fn test_{}_all_params_accept_patterns() {{
    {}
}}
"#, node.name.to_lowercase(),
    node.params.iter().map(|p| generate_param_test(p)).collect::<Vec<_>>().join("\n    "))
}

fn generate_param_test(param: &ParamInfo) -> String {
    // Generate 5 test cases per parameter (constant, pattern, bus, patref, arithmetic)
    todo!()
}
```

**Generated Output:**
```
tests/generated/test_pattern_params_all_oscillators.rs    (35 nodes × 2 params × 5 types = 350 tests)
tests/generated/test_pattern_params_all_filters.rs        (40 nodes × 3 params × 5 types = 600 tests)
tests/generated/test_pattern_params_all_envelopes.rs      (10 nodes × 4 params × 5 types = 200 tests)
tests/generated/test_pattern_params_all_effects.rs        (50+ nodes × 4 params × 5 types = 1000+ tests)
... etc
```

**Total:** ~3,000+ auto-generated tests

---

## Phase 4: Deep Verification - Audio-Rate Modulation

### The Hard Question: Is Modulation Actually Happening?

Test that parameters change **continuously**, not just at cycle boundaries.

#### Test 4.1: Audio-Rate Cutoff Sweep

```rust
#[test]
fn test_audio_rate_modulation_is_continuous() {
    // LFO at 10 Hz modulating filter cutoff
    // Should see smooth spectral changes, not stepped

    let audio = render_dsl(r#"
        tempo: 1.0
        ~lfo: sine 10  # 10 Hz LFO
        ~carrier: saw 110
        out: ~carrier # lpf (~lfo * 4000 + 1000) 1.0
    "#, 1);  // 1 second = 10 LFO cycles

    // Analyze spectrum over 10ms windows (441 samples)
    let num_windows = audio.len() / 441;
    let mut spectral_centroids = vec![];

    for i in 0..num_windows {
        let window = &audio[i*441..(i+1)*441];
        let centroid = calculate_spectral_centroid(window, 44100.0);
        spectral_centroids.push(centroid);
    }

    // Verify smooth modulation (should vary sinusoidally)
    // NOT stepped (which would show discrete jumps)
    assert_smooth_modulation(&spectral_centroids, 10.0);  // 10 Hz expected
}
```

#### Test 4.2: Per-Sample Verification

```rust
#[test]
fn test_filter_cutoff_changes_every_sample() {
    // Very slow LFO (0.1 Hz) so we can see gradual changes
    let audio = render_dsl(r#"
        ~lfo: sine 0.1
        ~saw: saw 110
        out: ~saw # lpf (~lfo * 3000 + 1000) 0.8
    "#, 10);  // 10 seconds

    // Extract instantaneous frequency content every 1000 samples
    let mut cutoff_estimates = vec![];
    for i in (0..audio.len()).step_by(1000) {
        let window = &audio[i..i.min(audio.len()).saturating_sub(i).min(4410)];
        let estimated_cutoff = estimate_lpf_cutoff(window);
        cutoff_estimates.push(estimated_cutoff);
    }

    // Should see monotonic increase then decrease (sine wave)
    // Verify it's not constant!
    let variance = calculate_variance(&cutoff_estimates);
    assert!(variance > 100000.0, "Cutoff must vary significantly");
}
```

---

## Phase 5: Regression Testing - The Matrix

### Create Parameter Test Matrix Dashboard

**Goal:** Visual dashboard showing test coverage for all nodes × all parameters × all input types

```markdown
# Parameter Test Coverage Matrix

| Node | Param1 | Param2 | Param3 | Param4 | Param5 | Total Coverage |
|------|--------|--------|--------|--------|--------|----------------|
| LowPass | ✅ 5/5 | ✅ 5/5 | ✅ 5/5 | - | - | 100% (15/15) |
| Reverb | ✅ 5/5 | ✅ 5/5 | ✅ 5/5 | - | - | 100% (15/15) |
| ADSR | ✅ 5/5 | ✅ 5/5 | ✅ 5/5 | ✅ 5/5 | - | 100% (20/20) |
| Sine | ✅ 5/5 | - | - | - | - | 100% (5/5) |
| ... | ... | ... | ... | ... | ... | ... |

**Overall:** 2,847 / 3,000 tests passing (94.9%)
```

### Auto-Update on CI

```yaml
# .github/workflows/pattern-params.yml

name: Pattern Parameter Coverage
on: [push, pull_request]

jobs:
  test-pattern-params:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test test_pattern_params --all
      - run: ./scripts/generate_coverage_matrix.sh
      - uses: actions/upload-artifact@v3
        with:
          name: param-coverage-matrix
          path: target/param_coverage.md
```

---

## Implementation Plan: Week-by-Week

### Week 1: Foundation (Phase 1)
- **Day 1-2:** Node inventory script, generate NODE_PARAMETER_INVENTORY.md
- **Day 3-4:** Create PARAMETER_TEST_MATRIX.csv
- **Day 5:** Set up test infrastructure, helper functions

**Deliverables:**
- Complete node/parameter inventory
- Test helper library (assert_spectral_difference, etc.)

### Week 2: Tier 1 Quick Win (Phase 2)
- **Day 1-3:** Implement 20 representative node tests (100 test functions)
- **Day 4:** Run tests, fix any failures
- **Day 5:** Document findings, create first coverage report

**Deliverables:**
- 100 passing tests covering 20 nodes
- First coverage report showing proof of concept

### Week 3: Auto-Generation (Phase 3)
- **Day 1-2:** Build metadata extraction in build.rs
- **Day 3-4:** Create test generator script
- **Day 5:** Generate and run all 3,000+ tests

**Deliverables:**
- Auto-generated test suite for all 137 nodes
- Full coverage report

### Week 4: Deep Verification (Phase 4)
- **Day 1-2:** Implement audio-rate modulation tests
- **Day 3:** Implement spectral analysis tests
- **Day 4-5:** Run deep verification, document any issues

**Deliverables:**
- Audio-rate modulation verification
- Spectral analysis test suite

### Week 5: Polish & CI (Phase 5)
- **Day 1-2:** Create coverage matrix dashboard
- **Day 3:** Set up CI integration
- **Day 4-5:** Documentation, final report

**Deliverables:**
- Live coverage dashboard
- CI integration
- Final verification report

---

## Success Metrics

### Quantitative
- ✅ 3,000+ tests passing (all nodes × all params × all types)
- ✅ 100% node coverage (137/137 nodes tested)
- ✅ 100% parameter coverage (all params of all nodes tested)
- ✅ Audio-rate modulation verified for 20+ critical nodes

### Qualitative
- ✅ Can confidently say: "Every parameter accepts patterns"
- ✅ Auto-generated tests make regression testing trivial
- ✅ Coverage matrix shows gaps instantly
- ✅ CI prevents regressions

---

## Testing Helper Library

### Core Functions Needed

```rust
// tests/pattern_verification_utils.rs

/// Verify two audio signals have different spectral content
pub fn assert_spectral_difference(audio1: &[f32], audio2: &[f32], min_diff: f32) {
    let centroid1 = calculate_spectral_centroid(audio1, 44100.0);
    let centroid2 = calculate_spectral_centroid(audio2, 44100.0);
    let diff = (centroid1 - centroid2).abs();
    assert!(diff > min_diff,
        "Spectral difference too small: {} (expected > {})", diff, min_diff);
}

/// Verify modulation is continuous (not stepped)
pub fn assert_continuous_modulation(audio: &[f32], sample_rate: f32) {
    // Extract instantaneous frequency/amplitude over time
    // Verify changes are smooth (high correlation with sine/LFO)
    // Not stepped (which would show low correlation)
    todo!()
}

/// Estimate filter cutoff from audio
pub fn estimate_lpf_cutoff(audio: &[f32]) -> f32 {
    // FFT, find -3dB point
    todo!()
}

/// Verify array shows smooth variation (not stepped)
pub fn assert_smooth_modulation(values: &[f32], expected_freq: f32) {
    // Fit sine wave, check R²
    todo!()
}

/// Calculate spectral centroid
pub fn calculate_spectral_centroid(audio: &[f32], sample_rate: f32) -> f32 {
    // FFT → weighted average of frequencies
    todo!()
}
```

---

## Expected Findings & Contingencies

### Likely Discoveries

1. **Some parameters might not actually work** → Document, fix, or remove
2. **Pattern evaluation might be buggy** → Create isolated reproduction, fix
3. **Audio-rate modulation might be stepped** → Investigate SignalAsPattern sample-and-hold
4. **Some nodes might crash with pattern inputs** → Fix or document limitations

### Handling Failures

**Document everything:**
- What parameter failed
- What input type failed
- Error message
- Reproduction case
- Fix priority (critical/nice-to-have)

**Create issues:**
```markdown
# Issue Template: Parameter Pattern Support Bug

**Node:** LowPass
**Parameter:** cutoff
**Input Type:** Pattern string
**Expected:** Cutoff varies across cycles
**Actual:** Constant cutoff, no modulation
**Reproduction:** `saw 110 # lpf "500 1000 2000" 0.8`
**Priority:** Critical (violates core architecture rule)
```

---

## The Grand Vision: Pattern Parameter Dashboard

Imagine opening a web page:

```
╔══════════════════════════════════════════════════════════╗
║     PHONON PATTERN PARAMETER VERIFICATION DASHBOARD      ║
╚══════════════════════════════════════════════════════════╝

Overall Coverage: ████████████████████░░ 94.9% (2,847 / 3,000)

Category Breakdown:
  Oscillators:  ████████████████████ 100% (175/175)
  Filters:      ████████████████████ 100% (600/600)
  Envelopes:    ███████████████████░  95% (190/200)
  Effects:      ██████████████░░░░░░  70% (700/1000)

Recent Failures:
  ❌ Reverb.room_size (pattern string) - Issue #142
  ❌ Compressor.ratio (bus reference) - Issue #143

Audio-Rate Modulation:
  ✅ 18/20 critical nodes verified
  ⏳ 2/20 pending (Convolution, FFT)
```

---

## Next Steps

**Ready to start?** Say the word and we'll:

1. **Quick win:** Implement 5 representative tests RIGHT NOW (20 min)
2. **Node inventory:** Generate complete parameter list (30 min)
3. **Phase 1:** Complete foundation this session
4. **Schedule:** Set timeline for full implementation

**Or we can adjust the plan first.** What sounds good?
