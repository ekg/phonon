/// Tests for struct transform - apply structure/rhythm from one pattern to values from another
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Structure Logic)
// ============================================================================

#[test]
fn test_struct_level1_simple_pattern() {
    // struct "t ~ t ~" $ "bd sn hh cp"
    // Structure: triggers at 0, 0.5 (2 per cycle)
    // Values: bd, sn, hh, cp (cycling through)
    // Expected: bd at 0, sn at 0.5

    let value_pattern = parse_mini_notation("bd sn hh cp");
    let struct_str = parse_mini_notation("t ~ t ~");

    // Convert to boolean pattern
    let struct_pattern = Pattern::new(move |state: &State| {
        struct_str
            .query(state)
            .into_iter()
            .map(|hap| phonon::pattern::Hap {
                whole: hap.whole,
                part: hap.part,
                value: hap.value == "t",
                context: hap.context,
            })
            .collect()
    });

    let structured = value_pattern.struct_pattern(struct_pattern);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = structured.query(&state);

    // Should have 2 events per cycle (matching structure)
    assert_eq!(
        events.len(),
        2,
        "struct should produce 2 events per cycle"
    );

    // First event should be "bd" (first value)
    assert_eq!(events[0].value, "bd", "First event should be bd");

    // Second event should be "sn" (second value)
    assert_eq!(events[1].value, "sn", "Second event should be sn");

    // Check timing - should be at 0 and 0.5
    assert!(
        (events[0].part.begin.to_float() - 0.0).abs() < 0.01,
        "First event should be at 0"
    );
    assert!(
        (events[1].part.begin.to_float() - 0.5).abs() < 0.01,
        "Second event should be at 0.5"
    );
}

#[test]
fn test_struct_level1_euclidean_pattern() {
    // struct "t(3,8)" $ "bd sn hh"
    // Structure: Euclidean 3 pulses in 8 steps = triggers at 0, 0.375, 0.625
    // Values: bd, sn, hh

    let value_pattern = parse_mini_notation("bd sn hh");
    let struct_str = parse_mini_notation("t(3,8)");

    let struct_pattern = Pattern::new(move |state: &State| {
        struct_str
            .query(state)
            .into_iter()
            .map(|hap| phonon::pattern::Hap {
                whole: hap.whole,
                part: hap.part,
                value: hap.value == "t",
                context: hap.context,
            })
            .collect()
    });

    let structured = value_pattern.struct_pattern(struct_pattern);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = structured.query(&state);

    // Should have 3 events (matching euclidean structure)
    assert_eq!(
        events.len(),
        3,
        "Euclidean struct should produce 3 events"
    );

    // Values should cycle: bd, sn, hh
    assert_eq!(events[0].value, "bd");
    assert_eq!(events[1].value, "sn");
    assert_eq!(events[2].value, "hh");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_struct_level2_simple_produces_events() {
    let code = r#"
tempo: 1.0
out $ s "bd sn hh cp" $ struct "t ~ t ~"
"#;
    let audio = render_dsl(code, 4);
    let onsets = detect_audio_events(&audio, 44100.0, 0.05);

    // Should detect 2 events per cycle × 4 cycles = 8 events
    assert!(
        onsets.len() >= 7 && onsets.len() <= 9,
        "Should detect ~8 events over 4 cycles, got {}",
        onsets.len()
    );
}

#[test]
fn test_struct_level2_euclidean_timing() {
    let code = r#"
tempo: 1.0
out $ s "bd" $ struct "t(3,8)"
"#;
    let audio = render_dsl(code, 4);
    let onsets = detect_audio_events(&audio, 44100.0, 0.05);

    // Euclidean (3,8) produces 3 events per cycle
    // Over 4 cycles: 3 × 4 = 12 events
    assert!(
        onsets.len() >= 11 && onsets.len() <= 13,
        "Euclidean struct should produce ~12 events over 4 cycles, got {}",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_struct_level3_produces_audio() {
    let code = r#"
tempo: 1.0
out $ s "bd sn hh cp" $ struct "t ~ t ~"
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.05,
        "struct should produce audible signal, got RMS {}",
        rms
    );
}

#[test]
fn test_struct_level3_fewer_events_than_original() {
    // struct with sparse structure should produce less energy than original
    let code_original = r#"
tempo: 1.0
out $ s "bd*8"
"#;
    let code_struct = r#"
tempo: 1.0
out $ s "bd*8" $ struct "t ~ t ~"
"#;

    let audio_original = render_dsl(code_original, 4);
    let audio_struct = render_dsl(code_struct, 4);

    let rms_original = calculate_rms(&audio_original);
    let rms_struct = calculate_rms(&audio_struct);

    // struct with "t ~ t ~" (2 events/cycle) should have less energy than "bd*8" (8 events/cycle)
    assert!(
        rms_struct < rms_original,
        "struct should reduce event count/energy: original={}, struct={}",
        rms_original,
        rms_struct
    );
}

// ============================================================================
// Integration Tests (Real Livecode Patterns)
// ============================================================================

#[test]
fn test_struct_integration_livecode_example() {
    // From livecode: struct "t(3,7)" $ s "808mt"
    let code = r#"
tempo: 1.0
out $ s "bd sn hh" $ struct "t(3,7)"
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.05,
        "livecode struct pattern should produce audio, got RMS {}",
        rms
    );
}

#[test]
fn test_struct_integration_simple_rhythm() {
    // Simple four-on-the-floor with struct
    let code = r#"
tempo: 1.0
out $ s "bd" $ struct "t t t t"
"#;
    let audio = render_dsl(code, 4);
    let onsets = detect_audio_events(&audio, 44100.0, 0.05);

    // Should have 4 events per cycle
    assert!(
        onsets.len() >= 15 && onsets.len() <= 17,
        "Should have ~16 events over 4 cycles, got {}",
        onsets.len()
    );
}
