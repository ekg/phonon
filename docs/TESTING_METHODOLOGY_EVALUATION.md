# Testing Methodology Evaluation

## Executive Summary

**Question:** Are we systematically testing end-to-end audio production with proper signal analysis?

**Answer:** Partially - we have basic RMS testing, but we need more sophisticated signal analysis to verify patterns, timing, and effects are working correctly. This document evaluates current testing and proposes comprehensive improvements.

## Current State Analysis

### What We Test Now

#### 1. Basic Audio Production (✅ Implemented)
**Files:** Most test files in `tests/`
**Method:** RMS (Root Mean Square) level checking
```rust
let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
assert!(rms > 0.0003, "Should produce audio");
```

**What this tells us:**
- ✅ Audio is being produced (not silent)
- ✅ Rough amplitude level
- ❌ **Does NOT verify** pattern timing, rhythm, frequency content, or effects

**Problem:** This is like testing a speech recognition system by checking if sound comes out - it doesn't verify *what* is being produced.

#### 2. Pattern Query Testing (✅ Good Foundation)
**Files:** `tests/test_pattern_query_debug.rs`, pattern-related tests
**Method:** Query patterns at specific times, verify events
```rust
let events = pattern.query(Arc::new(TimeSpan::new(0.0, 1.0)));
assert_eq!(events.len(), 4, "Should have 4 events");
```

**What this tells us:**
- ✅ Pattern generates correct number of events
- ✅ Events occur at expected times
- ✅ Events have correct values
- ❌ **Does NOT verify** these events are rendered correctly to audio

**Problem:** We test pattern logic separately from audio rendering - no connection verification.

#### 3. WAV Analysis Tool (✅ Implemented but Underused)
**File:** `src/bin/wav_analyze.rs`
**Capabilities:**
- RMS level
- Peak level
- DC offset
- Spectral analysis (FFT)
- Onset detection
- Zero-crossing rate

**Problem:** Tool exists but most tests don't use it! Tests only check basic RMS.

### Critical Gap: Pattern Events → Audio Verification

**The user is absolutely right** - we test patterns work, we test audio is produced, but we don't verify that **pattern events correlate with actual audio output**.

#### Example of the Gap:
```rust
// ✅ We test pattern produces events:
let pattern = parse_mini_notation("bd*4");  // 4 kicks per cycle
let events = pattern.query(TimeSpan::new(0.0, 1.0));
assert_eq!(events.len(), 4);  // PASS

// ✅ We test audio is produced:
let audio = graph.render(44100);
let rms = calculate_rms(&audio);
assert!(rms > 0.001);  // PASS

// ❌ We DON'T test that audio has 4 kick drum transients!
// We don't verify the events actually triggered samples at the right times!
```

## What We Should Test

### Level 1: Transient Detection ✅✅✅ CRITICAL
**Goal:** Verify pattern events create audio transients at correct times

**Method:** Onset detection with timing correlation
```rust
#[test]
fn test_pattern_events_match_audio_transients() {
    let input = r#"
        bpm 120
        out: s("bd sn bd sn") * 0.5
    "#;

    // Parse and compile
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Get pattern events
    let pattern = parse_mini_notation("bd sn bd sn");
    let events = pattern.query(TimeSpan::new(0.0, 1.0));

    // Expected event times (at 120 BPM = 2 CPS, 1 cycle = 0.5 seconds)
    let expected_times = vec![0.0, 0.125, 0.25, 0.375]; // 4 events in 0.5s cycle

    // Render audio
    let audio = graph.render(44100);

    // Detect onsets/transients
    let onsets = detect_onsets(&audio, 44100.0, threshold: 0.1);

    // Verify onset count matches event count
    assert_eq!(onsets.len(), events.len(),
        "Number of audio transients should match pattern events");

    // Verify onset times match expected times (within 5ms tolerance)
    for (i, onset_time) in onsets.iter().enumerate() {
        let expected = expected_times[i];
        let diff = (onset_time - expected).abs();
        assert!(diff < 0.005, // 5ms tolerance
            "Onset {} at {:.3}s should be near {:.3}s (diff: {:.3}ms)",
            i, onset_time, expected, diff * 1000.0);
    }
}
```

**Why this is critical:** This is the FUNDAMENTAL test - does the pattern actually control audio timing?

### Level 2: Frequency Content Verification ✅✅ HIGH PRIORITY
**Goal:** Verify synthesizers and filters produce expected frequency content

**Method:** FFT analysis with frequency assertions
```rust
#[test]
fn test_lowpass_filter_removes_high_frequencies() {
    let input = r#"
        bpm 120
        out: saw(440) >> lpf(880, 0.7) * 0.5
    "#;

    let audio = compile_and_render(input, 44100, 44100);

    // Analyze spectrum
    let spectrum = fft_analyze(&audio, 44100.0);

    // Get energy in different frequency bands
    let low_band = spectrum.energy_in_band(0.0, 880.0);    // Pass band
    let high_band = spectrum.energy_in_band(880.0, 5000.0); // Stop band

    // Low-pass should have most energy below cutoff
    let ratio = low_band / (low_band + high_band);
    assert!(ratio > 0.9,
        "Low-pass filter should have >90% energy below cutoff, got {:.1}%",
        ratio * 100.0);
}

#[test]
fn test_saw_wave_has_expected_harmonics() {
    let input = r#"
        bpm 120
        out: saw(110) * 0.3  # 110 Hz = A2
    "#;

    let audio = compile_and_render(input, 44100, 44100);
    let spectrum = fft_analyze(&audio, 44100.0);

    // Saw wave should have strong fundamental and harmonics
    assert!(spectrum.peak_at_frequency(110.0, tolerance: 5.0) > 0.1,
        "Saw should have strong fundamental at 110 Hz");
    assert!(spectrum.peak_at_frequency(220.0, tolerance: 5.0) > 0.05,
        "Saw should have 2nd harmonic at 220 Hz");
    assert!(spectrum.peak_at_frequency(330.0, tolerance: 5.0) > 0.03,
        "Saw should have 3rd harmonic at 330 Hz");
}
```

**Why this matters:** Verifies synthesis and filtering actually work, not just produce noise.

### Level 3: Pattern Transform Verification ✅ MEDIUM PRIORITY
**Goal:** Verify transforms change audio in expected ways

**Method:** Comparative analysis with correlation
```rust
#[test]
fn test_fast_transform_doubles_event_rate() {
    let normal = r#"bpm 120; out: s("bd sn") * 0.5"#;
    let fast = r#"bpm 120; out: s("bd sn" $ fast 2) * 0.5"#;

    let audio_normal = compile_and_render(normal, 44100, 44100);
    let audio_fast = compile_and_render(fast, 44100, 44100);

    // Detect onsets
    let onsets_normal = detect_onsets(&audio_normal, 44100.0);
    let onsets_fast = detect_onsets(&audio_fast, 44100.0);

    // fast 2 should double the event count
    assert_eq!(onsets_fast.len(), onsets_normal.len() * 2,
        "fast 2 should double the number of events");

    // Verify timing: fast events should be at half the intervals
    let interval_normal = onsets_normal[1] - onsets_normal[0];
    let interval_fast = onsets_fast[1] - onsets_fast[0];

    assert!((interval_fast - interval_normal / 2.0).abs() < 0.005,
        "fast 2 should halve the event interval");
}

#[test]
fn test_rev_transform_reverses_timing() {
    let normal = r#"bpm 120; out: s("bd sn hh cp") * 0.5"#;
    let reversed = r#"bpm 120; out: s("bd sn hh cp" $ rev) * 0.5"#;

    let audio_normal = compile_and_render(normal, 44100, 44100);
    let audio_reversed = compile_and_render(reversed, 44100, 44100);

    let onsets_normal = detect_onsets(&audio_normal, 44100.0);
    let onsets_reversed = detect_onsets(&audio_reversed, 44100.0);

    assert_eq!(onsets_normal.len(), onsets_reversed.len());

    // Verify reversed timing: first event should be near last, etc.
    let cycle_duration = 0.5; // 120 BPM = 2 CPS
    for i in 0..onsets_normal.len() {
        let expected_reversed = cycle_duration - onsets_normal[onsets_normal.len() - 1 - i];
        let actual_reversed = onsets_reversed[i];
        assert!((actual_reversed - expected_reversed).abs() < 0.005,
            "Reversed event {} timing mismatch", i);
    }
}
```

**Why this matters:** Verifies transforms actually modify audio correctly.

### Level 4: Effects Verification ✅ MEDIUM PRIORITY
**Goal:** Verify effects process audio correctly

**Method:** Signal characteristic analysis
```rust
#[test]
fn test_reverb_increases_signal_duration() {
    let dry = r#"bpm 120; out: s("bd ~ ~ ~") * 0.5"#;  // Single kick
    let wet = r#"bpm 120; out: reverb(s("bd ~ ~ ~") * 0.5, 0.8, 0.7, 0.5)"#;

    let audio_dry = compile_and_render(dry, 44100, 44100);
    let audio_wet = compile_and_render(wet, 44100, 44100);

    // Measure decay time (time to drop below noise floor)
    let decay_dry = measure_decay_time(&audio_dry, threshold: 0.001);
    let decay_wet = measure_decay_time(&audio_wet, threshold: 0.001);

    // Reverb should significantly increase decay time
    assert!(decay_wet > decay_dry * 2.0,
        "Reverb should at least double decay time");
}

#[test]
fn test_delay_creates_echoes() {
    let input = r#"
        bpm 120
        out: delay(s("bd ~ ~ ~") * 0.5, 0.25, 0.5, 0.8)  # 250ms delay, 50% feedback
    "#;

    let audio = compile_and_render(input, 44100, 44100);
    let onsets = detect_onsets(&audio, 44100.0);

    // Should have original + at least 2 echoes
    assert!(onsets.len() >= 3, "Delay should create multiple echoes");

    // Verify echo spacing (should be ~0.25s apart)
    let echo_interval = onsets[1] - onsets[0];
    assert!((echo_interval - 0.25).abs() < 0.01,
        "Echo interval should be ~250ms");
}
```

**Why this matters:** Verifies effects actually process audio, not just pass through.

### Level 5: Sample Playback Verification ✅ HIGH PRIORITY
**Goal:** Verify samples trigger at correct times with correct parameters

**Method:** Correlation analysis with reference samples
```rust
#[test]
fn test_sample_triggers_at_pattern_times() {
    let input = r#"
        bpm 120
        out: s("bd ~ sn ~") * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Load reference samples
    let bd_sample = load_sample("bd");
    let sn_sample = load_sample("sn");

    // Find sample triggers by correlation
    let bd_triggers = find_sample_triggers(&audio, &bd_sample, threshold: 0.7);
    let sn_triggers = find_sample_triggers(&audio, &sn_sample, threshold: 0.7);

    // Expected times: bd at 0.0, sn at 0.25 (in 0.5s cycle at 120 BPM)
    assert_eq!(bd_triggers.len(), 1, "Should trigger bd once");
    assert_eq!(sn_triggers.len(), 1, "Should trigger sn once");

    assert!((bd_triggers[0] - 0.0).abs() < 0.005, "bd should trigger at start");
    assert!((sn_triggers[0] - 0.125).abs() < 0.005, "sn should trigger at 1/4");
}

#[test]
fn test_sample_gain_parameter_works() {
    let quiet = r#"bpm 120; out: s("bd", 0.2)"#;  // gain 0.2
    let loud = r#"bpm 120; out: s("bd", 1.0)"#;   // gain 1.0

    let audio_quiet = compile_and_render(quiet, 44100, 44100);
    let audio_loud = compile_and_render(loud, 44100, 44100);

    let rms_quiet = calculate_rms(&audio_quiet);
    let rms_loud = calculate_rms(&audio_loud);

    // Loud should be ~5x louder (1.0 / 0.2)
    let ratio = rms_loud / rms_quiet;
    assert!(ratio > 4.0 && ratio < 6.0,
        "Gain parameter should scale amplitude correctly, got ratio {:.2}", ratio);
}
```

**Why this matters:** Sample playback is core functionality - must verify it works correctly.

## Implementation Plan

### Phase 1: Essential Signal Analysis Utilities (Week 1)
**File:** `tests/audio_test_utils.rs`

```rust
// Onset/transient detection
pub fn detect_onsets(audio: &[f32], sample_rate: f32, threshold: f32) -> Vec<f32>;

// FFT analysis
pub struct SpectrumAnalysis {
    pub fn energy_in_band(&self, low_hz: f32, high_hz: f32) -> f32;
    pub fn peak_at_frequency(&self, freq_hz: f32, tolerance: f32) -> f32;
    pub fn spectral_centroid(&self) -> f32;
}
pub fn fft_analyze(audio: &[f32], sample_rate: f32) -> SpectrumAnalysis;

// Timing analysis
pub fn measure_decay_time(audio: &[f32], threshold: f32) -> f32;

// Correlation analysis
pub fn find_sample_triggers(audio: &[f32], reference: &[f32], threshold: f32) -> Vec<f32>;
pub fn calculate_correlation(audio: &[f32], reference: &[f32], offset: usize) -> f32;
```

**Note:** Some of these exist in `wav_analyze.rs` - need to extract to reusable test utilities.

### Phase 2: Pattern→Audio Verification Tests (Week 1-2)
**New test files:**
- `tests/test_pattern_audio_correlation.rs` - Verify pattern events create transients
- `tests/test_sample_timing_verification.rs` - Verify samples trigger at correct times
- `tests/test_pattern_transform_timing.rs` - Verify transforms affect timing correctly

### Phase 3: Frequency Content Tests (Week 2)
**New test files:**
- `tests/test_synthesis_spectrum.rs` - Verify synthesizers produce expected harmonics
- `tests/test_filter_spectrum.rs` - Verify filters modify frequency content correctly

### Phase 4: Effects Verification Tests (Week 2-3)
**New test files:**
- `tests/test_effects_characteristics.rs` - Verify reverb, delay, etc. work correctly

## Testing Methodology Best Practices

### DO ✅
1. **Test pattern events correlate with audio transients** - this is fundamental
2. **Use onset detection for rhythm verification**
3. **Use FFT for frequency content verification**
4. **Use correlation for sample trigger detection**
5. **Compare before/after for transform verification**
6. **Use reasonable tolerances** (timing: ±5ms, frequency: ±5Hz, amplitude: ±10%)
7. **Test edge cases** (empty patterns, extreme parameters, etc.)

### DON'T ❌
1. **Don't just check RMS > 0** - this tells you almost nothing
2. **Don't trust compilation success** - tests must verify actual audio output
3. **Don't test patterns and audio separately** - must verify connection
4. **Don't use exact equality for floats** - use tolerances
5. **Don't skip negative tests** - test that wrong things fail correctly

## Current Test Quality Score

| Category | Score | Status |
|----------|-------|--------|
| Basic audio production | 8/10 | ✅ Good - RMS testing works |
| Pattern event generation | 9/10 | ✅ Excellent - comprehensive pattern tests |
| **Pattern→Audio correlation** | **2/10** | ❌ **Critical gap** - no verification |
| Frequency content | 1/10 | ❌ Minimal - only one FFT test |
| Timing verification | 3/10 | ⚠️ Basic - some onset tests exist |
| Effects verification | 2/10 | ❌ Minimal - basic RMS only |
| Sample playback verification | 4/10 | ⚠️ Basic - no correlation testing |

**Overall:** 4/10 - **Needs significant improvement**

## Recommended Actions (Priority Order)

1. **CRITICAL:** Implement `tests/audio_test_utils.rs` with onset detection ✅✅✅
2. **CRITICAL:** Create `test_pattern_audio_correlation.rs` - verify pattern events → transients ✅✅✅
3. **HIGH:** Add sample trigger verification with correlation analysis ✅✅
4. **HIGH:** Add frequency content testing for synthesis and filters ✅✅
5. **MEDIUM:** Add transform timing verification tests ✅
6. **MEDIUM:** Add effects characteristic tests ✅
7. **LOW:** Add more edge case tests

## Conclusion

**The user is absolutely correct** - we need much more sophisticated testing. Currently, we test that:
- ✅ Patterns generate events correctly (good!)
- ✅ Audio is produced (good!)
- ❌ **Events don't correlate with audio** (critical gap!)

We're essentially "deaf" as the user said - we're checking if sound comes out but not *what* sound or *when*.

**The fix:** Implement onset detection and correlation analysis to verify pattern events actually control audio timing. This is the fundamental test that connects pattern logic to audio output.

Once we have this foundation, we can systematically verify every feature actually works in the rendered audio, not just in theory.
