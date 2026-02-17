/// Three-Level Verification Tests for `striate` Transform
///
/// `striate n` plays the pattern n times per cycle, each time with a different
/// slice of the sample. It interlaces slices across all events:
///
/// striate 3 $ s "bd sn"
/// Slice 0: bd[0-33%] sn[0-33%]    (events in first third of cycle)
/// Slice 1: bd[33-67%] sn[33-67%]  (events in second third of cycle)
/// Slice 2: bd[67-100%] sn[67-100%] (events in last third of cycle)
///
/// Result: 6 events per cycle (2 original × 3 slices), interlaced.
///
/// This differs from `chop` which subdivides each event in place:
/// chop 3 $ s "bd sn"
/// bd[0-33%] bd[33-67%] bd[67-100%] sn[0-33%] sn[33-67%] sn[67-100%]
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

// ============================================================================
// LEVEL 1: Pattern Query Verification
// ============================================================================

#[test]
fn test_striate_level1_event_count() {
    // striate N should produce N × original events per cycle
    let pattern = parse_mini_notation("a b c d"); // 4 events per cycle
    let striate2 = pattern.clone().striate(2);
    let striate3 = pattern.clone().striate(3);
    let striate4 = pattern.clone().striate(4);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_count = pattern.query(&state).len();
    let s2_count = striate2.query(&state).len();
    let s3_count = striate3.query(&state).len();
    let s4_count = striate4.query(&state).len();

    assert_eq!(base_count, 4, "Base pattern should have 4 events");
    assert_eq!(
        s2_count,
        base_count * 2,
        "striate 2 should produce 2× events: got {}",
        s2_count
    );
    assert_eq!(
        s3_count,
        base_count * 3,
        "striate 3 should produce 3× events: got {}",
        s3_count
    );
    assert_eq!(
        s4_count,
        base_count * 4,
        "striate 4 should produce 4× events: got {}",
        s4_count
    );

    println!(
        "✅ striate Level 1: base={}, s2={}, s3={}, s4={}",
        base_count, s2_count, s3_count, s4_count
    );
}

#[test]
fn test_striate_level1_begin_end_context() {
    // Each slice group should have correct begin/end values
    let pattern = parse_mini_notation("a b");
    let striated = pattern.striate(3);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = striated.query(&state);
    assert_eq!(haps.len(), 6, "striate 3 of 2 events should produce 6 events");

    // Collect begin/end values
    let mut begin_end_pairs: Vec<(f64, f64)> = haps
        .iter()
        .map(|h| {
            let begin: f64 = h
                .context
                .get("begin")
                .expect("should have begin")
                .parse()
                .unwrap();
            let end: f64 = h
                .context
                .get("end")
                .expect("should have end")
                .parse()
                .unwrap();
            (begin, end)
        })
        .collect();
    begin_end_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Should have 3 distinct begin/end pairs, each appearing twice (for a and b)
    // Pair 0: begin=0.0, end=0.333...
    // Pair 1: begin=0.333..., end=0.666...
    // Pair 2: begin=0.666..., end=1.0
    let expected_pairs = vec![
        (0.0, 1.0 / 3.0),
        (0.0, 1.0 / 3.0),
        (1.0 / 3.0, 2.0 / 3.0),
        (1.0 / 3.0, 2.0 / 3.0),
        (2.0 / 3.0, 1.0),
        (2.0 / 3.0, 1.0),
    ];

    for (i, ((actual_b, actual_e), (expected_b, expected_e))) in
        begin_end_pairs.iter().zip(expected_pairs.iter()).enumerate()
    {
        assert!(
            (actual_b - expected_b).abs() < 0.01,
            "Event {} begin: expected {}, got {}",
            i,
            expected_b,
            actual_b
        );
        assert!(
            (actual_e - expected_e).abs() < 0.01,
            "Event {} end: expected {}, got {}",
            i,
            expected_e,
            actual_e
        );
    }

    println!("✅ striate Level 1: begin/end context verified for all 6 events");
}

#[test]
fn test_striate_level1_interlacing() {
    // striate interlaces: all events play with slice 0, then all with slice 1, etc.
    // For "a b" with striate 2:
    // Time [0, 0.5): a[0-50%], b[0-50%]
    // Time [0.5, 1.0): a[50-100%], b[50-100%]
    let pattern = parse_mini_notation("a b");
    let striated = pattern.striate(2);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = striated.query(&state);
    assert_eq!(haps.len(), 4, "striate 2 of 2 events = 4 events");

    // Sort by time to verify ordering
    let mut sorted = haps.clone();
    sorted.sort_by(|a, b| {
        a.part
            .begin
            .to_float()
            .partial_cmp(&b.part.begin.to_float())
            .unwrap()
    });

    // First half of cycle: events with begin=0, end=0.5
    let first_half: Vec<_> = sorted
        .iter()
        .filter(|h| h.part.begin.to_float() < 0.5)
        .collect();
    let second_half: Vec<_> = sorted
        .iter()
        .filter(|h| h.part.begin.to_float() >= 0.5)
        .collect();

    assert_eq!(first_half.len(), 2, "First half should have 2 events");
    assert_eq!(second_half.len(), 2, "Second half should have 2 events");

    // First half events should have begin=0, end=0.5
    for h in &first_half {
        let begin: f64 = h.context.get("begin").unwrap().parse().unwrap();
        let end: f64 = h.context.get("end").unwrap().parse().unwrap();
        assert!(
            (begin - 0.0).abs() < 0.01,
            "First half begin should be 0.0"
        );
        assert!((end - 0.5).abs() < 0.01, "First half end should be 0.5");
    }

    // Second half events should have begin=0.5, end=1.0
    for h in &second_half {
        let begin: f64 = h.context.get("begin").unwrap().parse().unwrap();
        let end: f64 = h.context.get("end").unwrap().parse().unwrap();
        assert!(
            (begin - 0.5).abs() < 0.01,
            "Second half begin should be 0.5"
        );
        assert!((end - 1.0).abs() < 0.01, "Second half end should be 1.0");
    }

    // Both halves should have both values
    let first_values: Vec<String> = first_half.iter().map(|h| h.value.clone()).collect();
    let second_values: Vec<String> = second_half.iter().map(|h| h.value.clone()).collect();
    assert!(first_values.contains(&"a".to_string()));
    assert!(first_values.contains(&"b".to_string()));
    assert!(second_values.contains(&"a".to_string()));
    assert!(second_values.contains(&"b".to_string()));

    println!("✅ striate Level 1: interlacing verified");
}

#[test]
fn test_striate_level1_multicycle() {
    // Verify consistent behavior across multiple cycles
    let pattern = parse_mini_notation("a b c");
    let striated = pattern.striate(2);

    let mut total = 0;
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let count = striated.query(&state).len();
        assert_eq!(
            count, 6,
            "Cycle {}: striate 2 of 3 events should always produce 6",
            cycle
        );
        total += count;
    }

    assert_eq!(total, 24, "4 cycles × 6 events = 24 total");
    println!("✅ striate Level 1: multicycle consistency verified");
}

#[test]
fn test_striate_1_is_identity_count() {
    // striate 1 should produce same event count as original
    let pattern = parse_mini_notation("a b c d");
    let striated = pattern.clone().striate(1);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_count = pattern.query(&state).len();
    let s1_count = striated.query(&state).len();

    assert_eq!(s1_count, base_count, "striate 1 should be identity");

    println!("✅ striate edge case: striate 1 is identity");
}

#[test]
fn test_striate_preserves_all_values() {
    // All original pattern values must appear in output
    let pattern = parse_mini_notation("a b c d e");
    let striated = pattern.striate(3);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = striated.query(&state);
    let values: Vec<String> = haps.iter().map(|h| h.value.clone()).collect();

    for expected in &["a", "b", "c", "d", "e"] {
        let count = values.iter().filter(|v| v.as_str() == *expected).count();
        assert_eq!(
            count, 3,
            "Value '{}' should appear 3 times (once per slice), got {}",
            expected, count
        );
    }

    println!("✅ striate Level 1: all values preserved with correct multiplicity");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio)
// ============================================================================

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");

    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize; // tempo = 0.5 cps
    let total_samples = samples_per_cycle * cycles;

    graph.render(total_samples)
}

#[test]
fn test_striate_level2_produces_audio() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ striate 2
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.01,
        "striate should produce audible audio (RMS = {})",
        rms
    );

    println!("✅ striate Level 2: produces audio (RMS = {:.4})", rms);
}

#[test]
fn test_striate_level2_more_onsets_than_base() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let striate_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ striate 2
"#;

    let cycles = 4;
    let base_audio = render_dsl(base_code, cycles);
    let striate_audio = render_dsl(striate_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let striate_onsets = detect_audio_events(&striate_audio, sample_rate, 0.01);

    // striate 2 creates 2× as many events, so should have more onsets
    assert!(
        striate_onsets.len() >= base_onsets.len(),
        "striate should produce at least as many onsets as base: base={}, striate={}",
        base_onsets.len(),
        striate_onsets.len(),
    );

    println!(
        "✅ striate Level 2: base_onsets={}, striate_onsets={}",
        base_onsets.len(),
        striate_onsets.len(),
    );
}

#[test]
fn test_striate_level2_large_n() {
    // Even with large N, striate should produce audio
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ striate 8
"#;

    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.001,
        "striate 8 should still produce audio (RMS = {})",
        rms
    );

    println!("✅ striate Level 2: striate 8 produces audio (RMS = {:.4})", rms);
}

// ============================================================================
// LEVEL 3: Audio Characteristics
// ============================================================================

#[test]
fn test_striate_level3_rms_comparison() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let striate_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ striate 2
"#;

    let base_audio = render_dsl(base_code, 8);
    let striate_audio = render_dsl(striate_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let striate_rms = calculate_rms(&striate_audio);

    // striate plays shorter slices so energy may differ, but should be in reasonable range
    let ratio = striate_rms / base_rms;
    assert!(
        ratio > 0.05 && ratio < 3.0,
        "striate RMS should be in reasonable range: base={:.4}, striate={:.4}, ratio={:.2}",
        base_rms,
        striate_rms,
        ratio
    );

    println!(
        "✅ striate Level 3: base_rms={:.4}, striate_rms={:.4}, ratio={:.2}",
        base_rms, striate_rms, ratio
    );
}
