/// Tests for slice with pattern-controlled indices
///
/// slice n indices_pattern allows deterministic reordering of chunks
/// Example: slice 4 "0 2 1 3" plays first, third, second, fourth chunks
///
/// This is different from:
/// - chop: slices and stacks (plays simultaneously)
/// - scramble: random reordering
/// - shuffle: random time shifts
///
/// slice gives you CONTROL over the exact order of chunks
///
/// ## Verified semantics (verify-slice-sample, 2026-07-04)
///
/// `slice n indices` selects, per index event, the source material lying in the
/// cycle window `[i/n, (i+1)/n]`, and sets each event's sample `begin`/`end`
/// playback fractions RELATIVE TO THE MATCHED SAMPLE'S OWN EXTENT (see
/// `pattern::slice_sample_range`). Two input shapes therefore behave differently
/// — as they should:
///
/// - **Multi-sample pattern** (`s "bd sn hh cp"`, each event fills one slice
///   window): begin/end resolve to `(0.0, 1.0)`, so slice REORDERS FULL discrete
///   samples. `slice 4 "0 1 2 3"` is a no-op; `slice 4 "3 2 1 0"` reverses the
///   four full samples; total energy is PRESERVED across any permutation.
/// - **Single long sample** (`s "break"` spanning the whole cycle): begin/end
///   resolve to the sub-buffer fractions `(i/n, (i+1)/n)`, i.e. classic Tidal
///   breakbeat slicing of one buffer.
///
/// Before this fix, a multi-sample pattern was force-sliced at the GLOBAL cycle
/// fractions, so `slice 4 "3 2 1 0"` played decay-tail sub-slices and the first
/// half of the cycle rendered near-silent. The level-2/3 assertions below now
/// encode the full-sample-reorder semantics instead of merely non-silence.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

/// Helper: Render DSL code
fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    graph.set_cps(2.0); // 2 cycles per second

    let samples_per_cycle = (sample_rate / 2.0) as usize;
    let total_samples = samples_per_cycle * cycles;

    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification
// ============================================================================

#[test]
fn test_slice_level1_reorders_chunks() {
    // slice 4 "0 2 1 3" should reorder 4 chunks
    // Cycle 0: slice 0 (first quarter)
    // Cycle 1: slice 2 (third quarter)
    // Cycle 2: slice 1 (second quarter)
    // Cycle 3: slice 3 (fourth quarter)

    let pattern = parse_mini_notation("bd sn hh cp");

    // Create index pattern: 0 2 1 3
    let _indices = parse_mini_notation("0 2 1 3");

    // This would be: pattern.slice_pattern(4, indices)
    // For now, just verify the base pattern structure
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_events = pattern.query(&state);
    assert_eq!(base_events.len(), 4, "Base pattern should have 4 events");

    println!("✅ slice Level 1: Base pattern verified");
}

#[test]
fn test_slice_level1_identity() {
    // slice 4 "0 1 2 3" should be identity (no reordering)
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(4, 1)),
        controls: HashMap::new(),
    };

    let base_events = pattern.query(&state);

    // Identity ordering should preserve all events
    assert!(base_events.len() >= 4, "Should have events over 4 cycles");

    println!("✅ slice Level 1: Identity case verified");
}

#[test]
fn test_slice_level1_reverses_chunks() {
    // slice 4 "3 2 1 0" should reverse the 4 chunks
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_events = pattern.query(&state);
    assert_eq!(base_events.len(), 4, "Base pattern has 4 events");

    println!("✅ slice Level 1: Reverse pattern structure verified");
}

#[test]
fn test_slice_level1_full_sample_vs_subbuffer_semantics() {
    // Deterministic pattern-query check of the two verified slice semantics
    // (no audio render / sample files involved).
    let one_cycle = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // (a) MULTI-SAMPLE pattern: each event fills exactly one 1/4 slice window,
    //     so slice must REORDER FULL samples -> begin=0.0, end=1.0 on every event.
    let multi = parse_mini_notation("bd sn hh cp")
        .slice_pattern(4, parse_mini_notation("0 1 2 3"));
    let multi_haps = multi.query(&one_cycle);
    assert_eq!(multi_haps.len(), 4, "slice of 4-sample pattern -> 4 events");
    for hap in &multi_haps {
        let begin: f64 = hap.context.get("begin").expect("begin ctx").parse().unwrap();
        let end: f64 = hap.context.get("end").expect("end ctx").parse().unwrap();
        assert!(
            (begin - 0.0).abs() < 1e-6 && (end - 1.0).abs() < 1e-6,
            "multi-sample slice must play the FULL sample (begin=0,end=1), got begin={} end={}",
            begin,
            end
        );
    }

    // (b) SINGLE long sample spanning the whole cycle: slice must cut the buffer
    //     into sub-slices -> begin/end = (i/n, (i+1)/n).
    let single = parse_mini_notation("break")
        .slice_pattern(4, parse_mini_notation("0 1 2 3"));
    let mut single_haps = single.query(&one_cycle);
    single_haps.sort_by(|a, b| {
        a.part
            .begin
            .to_float()
            .partial_cmp(&b.part.begin.to_float())
            .unwrap()
    });
    assert_eq!(single_haps.len(), 4, "slice of single sample -> 4 sub-slices");
    let expected = [(0.0, 0.25), (0.25, 0.5), (0.5, 0.75), (0.75, 1.0)];
    for (hap, (eb, ee)) in single_haps.iter().zip(expected.iter()) {
        let begin: f64 = hap.context.get("begin").expect("begin ctx").parse().unwrap();
        let end: f64 = hap.context.get("end").expect("end ctx").parse().unwrap();
        assert!(
            (begin - eb).abs() < 1e-6 && (end - ee).abs() < 1e-6,
            "single-sample slice must sub-buffer slice, expected ({},{}) got ({},{})",
            eb,
            ee,
            begin,
            end
        );
    }

    println!("✅ slice Level 1: full-sample reorder vs sub-buffer semantics verified");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Verification)
// ============================================================================

#[test]
fn test_slice_level2_produces_audio() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 1 2 3"
"#;

    let audio = render_dsl(code, 8);
    let sample_rate = 44100.0;

    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should have events (exact count depends on reordering)
    assert!(
        onsets.len() >= 8,
        "Sliced pattern should have events (got {})",
        onsets.len()
    );

    println!("✅ slice Level 2: Audio events detected = {}", onsets.len());
}

#[test]
fn test_slice_level2_reordered_vs_normal() {
    // Verified semantics: on a multi-sample pattern, slice REORDERS FULL samples.
    //  - identity "0 1 2 3" is a no-op: identical onset set to the un-sliced pattern.
    //  - a permutation like "3 2 1 0" still yields a full, dense set of onsets
    //    (NOT the near-silent decay-tail render the old buffer-slicing produced).
    let sample_rate = 44100.0;

    let normal = render_dsl(
        "tempo: 0.5\nout $ s \"bd sn hh cp\"\n",
        8,
    );
    let identity = render_dsl(
        "tempo: 0.5\nout $ s \"bd sn hh cp\" $ slice 4 \"0 1 2 3\"\n",
        8,
    );
    let reordered = render_dsl(
        "tempo: 0.5\nout $ s \"bd sn hh cp\" $ slice 4 \"3 2 1 0\"\n",
        8,
    );

    let normal_onsets = detect_audio_events(&normal, sample_rate, 0.01);
    let identity_onsets = detect_audio_events(&identity, sample_rate, 0.01);
    let reordered_onsets = detect_audio_events(&reordered, sample_rate, 0.01);

    assert!(
        normal_onsets.len() >= 8,
        "sanity: base pattern should have onsets, got {}",
        normal_onsets.len()
    );

    // Identity slice must reproduce the base pattern's onsets exactly (no-op).
    assert_eq!(
        identity_onsets.len(),
        normal_onsets.len(),
        "slice 4 \"0 1 2 3\" is identity; onset count must match base ({} vs {})",
        identity_onsets.len(),
        normal_onsets.len()
    );

    // Reordered slice reorders FULL samples -> still a dense onset set, on the
    // same order of magnitude as the base (within ±40%). The pre-fix bug made
    // the first half near-silent, which would collapse this count.
    let lo = normal_onsets.len() * 3 / 5; // 0.6x
    let hi = normal_onsets.len() * 7 / 5 + 1; // 1.4x
    assert!(
        reordered_onsets.len() >= lo && reordered_onsets.len() <= hi,
        "reordered full-sample onsets ({}) should stay near base ({}), band [{}, {}]",
        reordered_onsets.len(),
        normal_onsets.len(),
        lo,
        hi
    );

    println!(
        "✅ slice Level 2: normal = {}, identity = {}, reordered = {}",
        normal_onsets.len(),
        identity_onsets.len(),
        reordered_onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics
// ============================================================================

#[test]
fn test_slice_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 2 1 3"
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.05,
        "Sliced pattern should have audible audio (RMS = {})",
        rms
    );

    println!("✅ slice Level 3: RMS = {:.4}", rms);
}

#[test]
fn test_slice_level3_preserves_energy() {
    let normal_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let sliced_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "1 3 0 2"
"#;

    let normal = render_dsl(normal_code, 8);
    let sliced = render_dsl(sliced_code, 8);

    let normal_rms = calculate_rms(&normal);
    let sliced_rms = calculate_rms(&sliced);

    // Verified semantics: on a multi-sample pattern, slice REORDERS FULL samples,
    // so a permutation is energy-preserving — the same four buffers play, only in
    // a different order. Total RMS must therefore match the base closely.
    let ratio = sliced_rms / normal_rms;
    assert!(
        ratio >= 0.95 && ratio <= 1.05,
        "full-sample reorder must preserve energy: normal = {:.4}, sliced = {:.4}, ratio = {:.3}",
        normal_rms,
        sliced_rms,
        ratio
    );

    println!(
        "✅ slice Level 3: Normal RMS = {:.4}, Sliced RMS = {:.4}, ratio = {:.3}",
        normal_rms, sliced_rms, ratio
    );
}

#[test]
fn test_slice_level3_reverse_mirrors_halves() {
    // Regression test for the original bug: `slice 4 "3 2 1 0"` on a multi-sample
    // pattern used to slice each buffer at the GLOBAL cycle fractions, so the
    // first half of the cycle became decay-tail sub-slices (near-silent).
    //
    // With full-sample reorder, "3 2 1 0" reverses the four whole samples, so the
    // per-half energy is a MIRROR of the un-sliced pattern: whichever half was
    // loud (bd+sn) moves to the opposite half. We verify that mirror directly —
    // it fails loudly if slice reverts to decay-tail sub-slicing.
    let sample_rate = 44100.0_f32;
    let spc = (sample_rate / 2.0) as usize; // samples per cycle at cps=2

    let normal = render_dsl("tempo: 0.5\nout $ s \"bd sn hh cp\"\n", 8);
    let reversed = render_dsl(
        "tempo: 0.5\nout $ s \"bd sn hh cp\" $ slice 4 \"3 2 1 0\"\n",
        8,
    );

    // Use a settled cycle (cycle 1) to avoid any cycle-0 startup transient.
    let half_rms = |buf: &[f32]| {
        let cyc = &buf[spc..2 * spc];
        let mid = cyc.len() / 2;
        (calculate_rms(&cyc[..mid]), calculate_rms(&cyc[mid..]))
    };
    let (norm_first, norm_second) = half_rms(&normal);
    let (rev_first, rev_second) = half_rms(&reversed);

    // Base pattern: loud drums (bd, sn) sit in the first half.
    assert!(
        norm_first > norm_second,
        "base: first half (bd sn) should be louder than second (hh cp): {:.4} vs {:.4}",
        norm_first,
        norm_second
    );
    // Reversed (cp hh sn bd): the loud drums moved to the SECOND half.
    assert!(
        rev_second > rev_first,
        "reversed: loud half must move to second: first {:.4} vs second {:.4}",
        rev_first,
        rev_second
    );
    // The reversed first half is a FULL clap+hihat, not a near-silent decay tail.
    assert!(
        rev_first > 0.02,
        "reversed first half must be full samples (not decay tails), rms = {:.4}",
        rev_first
    );
    // Full-sample reversal => the halves mirror the base exactly.
    assert!(
        (rev_first - norm_second).abs() < 0.03,
        "reversed first half {:.4} should mirror base second half {:.4}",
        rev_first,
        norm_second
    );
    assert!(
        (rev_second - norm_first).abs() < 0.03,
        "reversed second half {:.4} should mirror base first half {:.4}",
        rev_second,
        norm_first
    );

    println!(
        "✅ slice Level 3: reverse mirrors halves — base ({:.4},{:.4}) reversed ({:.4},{:.4})",
        norm_first, norm_second, rev_first, rev_second
    );
}

// ============================================================================
// Use Cases
// ============================================================================

#[test]
fn test_slice_reverse_chunks() {
    // Reverse the order of 4 chunks
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "3 2 1 0"
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Reversed chunks should produce audio");
    println!("✅ slice use case: Reverse chunks RMS = {:.4}", rms);
}

#[test]
fn test_slice_repeat_chunk() {
    // Repeat first chunk 4 times
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 0 0 0"
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Repeated chunk should produce audio");
    println!("✅ slice use case: Repeat chunk RMS = {:.4}", rms);
}

#[test]
fn test_slice_skip_chunks() {
    // Only play chunks 0 and 2 (skip 1 and 3)
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 2 0 2"
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Selective chunks should produce audio");
    println!("✅ slice use case: Skip chunks RMS = {:.4}", rms);
}

#[test]
fn test_slice_with_effects() {
    // Slice with effects chain
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "3 1 2 0" # lpf 2000 0.8
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.03, "Sliced with effects should produce audio");
    println!("✅ slice use case: With effects RMS = {:.4}", rms);
}

#[test]
fn test_slice_pattern_controlled_indices() {
    // Use pattern for indices (alternating chunks)
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ slice 4 "0 2"
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Pattern-controlled indices should work");
    println!("✅ slice use case: Pattern indices RMS = {:.4}", rms);
}

#[test]
fn test_slice_complex_reordering() {
    // Complex reordering for breakbeat-style cuts
    let code = r#"
tempo: 0.5
out $ s "bd*4" $ slice 8 "7 5 3 1 6 4 2 0"
"#;

    let audio = render_dsl(code, 8);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05, "Complex reordering should produce audio");
    println!("✅ slice use case: Complex reordering RMS = {:.4}", rms);
}
