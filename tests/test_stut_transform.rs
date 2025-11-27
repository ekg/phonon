//! Tests for stut transform - Tidal's classic stutter/echo with decay
//!
//! stut creates echoing/stuttering repetitions of events with decay:
//! - stut n time decay: repeat pattern n times, each delayed by time cycles, with volume decay
//!
//! Example: stut 3 0.125 0.7
//! - Original event at time 0, volume 1.0
//! - Echo 1 at time +0.125, volume 0.7
//! - Echo 2 at time +0.25, volume 0.49
//!
//! This is different from "stutter" which just subdivides events.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== LEVEL 1: Pattern Query Tests ==========

#[test]
fn test_stut_compiles() {
    // Basic compilation test
    let code = r#"
        tempo: 1.0
        o1: s "bd" $ stut 3 0.125 0.7
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "stut should compile: {:?}", result.err());
}

// ========== LEVEL 2: Onset Detection Tests ==========

#[test]
fn test_stut_creates_echoes() {
    // stut 3 0.125 0.7 should create 3 layers
    // Using sine wave triggered by Pattern
    let code = r#"
        tempo: 1.0
        ~trigger: "x"
        ~tone: ~trigger * sine 440
        o1: ~tone $ stut 3 0.125 0.7
    "#;

    let buffer = render_dsl(code, 1.0); // 1 cycle
    let rms = calculate_rms(&buffer);

    // Should have audio output from stut layers
    assert!(
        rms > 0.01,
        "stut should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_stut_with_multiple_events() {
    // Test stut on pattern with multiple events
    let code = r#"
        tempo: 2.0
        o1: s "bd sn" $ stut 2 0.125 0.8
    "#;

    let buffer = render_dsl(code, 1.0); // 1 cycle at 2 CPS
    let onsets = detect_audio_events(&buffer, 44100.0, 0.01);

    // Should have 4 events: (bd + echo, sn + echo)
    assert!(
        onsets.len() >= 3 && onsets.len() <= 6,
        "stut 2 on 'bd sn' should create 4 events, got {}",
        onsets.len()
    );
}

#[test]
fn test_stut_no_echo() {
    // stut 1 should just return original (no echoes)
    let code_normal = r#"
        tempo: 2.0
        o1: s "bd*4"
    "#;

    let code_stut1 = r#"
        tempo: 2.0
        o1: s "bd*4" $ stut 1 0.125 0.7
    "#;

    let buffer_normal = render_dsl(code_normal, 1.0);
    let buffer_stut1 = render_dsl(code_stut1, 1.0);

    let rms_normal = calculate_rms(&buffer_normal);
    let rms_stut1 = calculate_rms(&buffer_stut1);

    // Should be very similar (stut 1 = no echoes)
    let diff_ratio = (rms_normal - rms_stut1).abs() / rms_normal;
    assert!(
        diff_ratio < 0.1,
        "stut 1 should be same as original, RMS normal: {}, stut1: {}, diff: {}",
        rms_normal,
        rms_stut1,
        diff_ratio
    );
}

// ========== LEVEL 3: Audio Characteristics Tests ==========

#[test]
fn test_stut_increases_density() {
    // More echoes = higher RMS (more energy)
    let code_normal = r#"
        tempo: 2.0
        o1: s "bd*4"
    "#;

    let code_stut = r#"
        tempo: 2.0
        o1: s "bd*4" $ stut 4 0.0625 0.8
    "#;

    let buffer_normal = render_dsl(code_normal, 2.0);
    let buffer_stut = render_dsl(code_stut, 2.0);

    let rms_normal = calculate_rms(&buffer_normal);
    let rms_stut = calculate_rms(&buffer_stut);

    // stut should have more energy
    assert!(
        rms_stut > rms_normal,
        "stut with echoes should have higher RMS: normal={}, stut={}",
        rms_normal,
        rms_stut
    );

    println!("RMS - normal: {}, stut: {}, ratio: {}", rms_normal, rms_stut, rms_stut / rms_normal);
}

#[test]
fn test_stut_decay_reduces_energy() {
    // Lower decay = less energy added
    let code_high_decay = r#"
        tempo: 2.0
        o1: s "bd*4" $ stut 4 0.0625 0.9
    "#;

    let code_low_decay = r#"
        tempo: 2.0
        o1: s "bd*4" $ stut 4 0.0625 0.3
    "#;

    let buffer_high = render_dsl(code_high_decay, 2.0);
    let buffer_low = render_dsl(code_low_decay, 2.0);

    let rms_high = calculate_rms(&buffer_high);
    let rms_low = calculate_rms(&buffer_low);

    // High decay should have more energy
    assert!(
        rms_high > rms_low,
        "Higher decay should give more energy: high={}, low={}",
        rms_high,
        rms_low
    );

    println!("RMS - high decay (0.9): {}, low decay (0.3): {}", rms_high, rms_low);
}

#[test]
fn test_stut_timing_variations() {
    // Different timing values should work
    let code_short = r#"
        tempo: 2.0
        o1: s "bd" $ stut 3 0.05 0.8
    "#;

    let code_long = r#"
        tempo: 2.0
        o1: s "bd" $ stut 3 0.25 0.8
    "#;

    let buffer_short = render_dsl(code_short, 1.0);
    let buffer_long = render_dsl(code_long, 1.0);

    let onsets_short = detect_audio_events(&buffer_short, 44100.0, 0.02);
    let onsets_long = detect_audio_events(&buffer_long, 44100.0, 0.02);

    // Both should create echoes
    assert!(
        onsets_short.len() >= 2,
        "Short timing should create echoes, got {} events",
        onsets_short.len()
    );
    assert!(
        onsets_long.len() >= 2,
        "Long timing should create echoes, got {} events",
        onsets_long.len()
    );

    // Long timing should have wider spacing
    if onsets_short.len() >= 2 && onsets_long.len() >= 2 {
        let spacing_short = onsets_short[1].time - onsets_short[0].time;
        let spacing_long = onsets_long[1].time - onsets_long[0].time;

        assert!(
            spacing_long > spacing_short,
            "Long timing should have wider spacing: short={}s, long={}s",
            spacing_short,
            spacing_long
        );
    }
}

#[test]
fn test_stut_with_pattern_parameters() {
    // stut should work with pattern-modulated parameters
    let code = r#"
        tempo: 2.0
        o1: s "bd*4" $ stut 3 "0.125 0.25" 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "stut with pattern time should produce audio, got RMS: {}", rms);
    println!("stut with pattern timing RMS: {}", rms);
}

// ========== Integration Tests ==========

#[test]
fn test_stut_with_other_transforms() {
    // stut should compose with other transforms
    let code = r#"
        tempo: 2.0
        o1: s "bd sn" $ fast 2 $ stut 2 0.125 0.8
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "stut + fast should produce audio, got RMS: {}", rms);
}

#[test]
fn test_stut_classic_tidal_example() {
    // Classic Tidal pattern: delayed echo feel
    let code = r#"
        tempo: 2.0
        o1: s "bd sn hh cp" $ stut 4 0.125 0.6
    "#;

    let buffer = render_dsl(code, 4.0); // 4 cycles
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Classic stut pattern should produce rich audio, got RMS: {}", rms);
    println!("Classic stut example RMS: {}", rms);
}
