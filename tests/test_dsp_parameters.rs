//! Test Pattern DSP Parameters: gain, pan, speed, cut_group, attack, release
//!
//! All parameters use Tidal-style # chaining syntax

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Calculate RMS (root mean square) of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Calculate peak amplitude of audio buffer
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

// ========== GAIN PARAMETER TESTS ==========

#[test]
fn test_gain_constant_value() {
    // Test that gain parameter affects amplitude
    let code_normal = r#"
tempo: 2.0
out: s "bd*4"
"#;

    let code_quiet = r#"
tempo: 2.0
out: s "bd*4" # gain 0.3
"#;

    let (_rest, statements_normal) = parse_program(code_normal).expect("Failed to parse");
    let mut graph_normal = compile_program(statements_normal, 44100.0).expect("Failed to compile");
    graph_normal.set_cps(2.0);

    let (_rest, statements_quiet) = parse_program(code_quiet).expect("Failed to parse");
    let mut graph_quiet = compile_program(statements_quiet, 44100.0).expect("Failed to compile");
    graph_quiet.set_cps(2.0);

    // Render 2 cycles
    let buffer_normal = graph_normal.render(44100);
    let buffer_quiet = graph_quiet.render(44100);

    let rms_normal = calculate_rms(&buffer_normal);
    let rms_quiet = calculate_rms(&buffer_quiet);

    eprintln!("Normal RMS: {}, Quiet RMS: {}", rms_normal, rms_quiet);

    // Quiet version should have significantly lower RMS
    assert!(
        rms_quiet < rms_normal * 0.5,
        "Gain=0.3 should reduce RMS significantly. Normal: {}, Quiet: {}",
        rms_normal,
        rms_quiet
    );

    // Both should produce sound
    assert!(rms_normal > 0.01, "Normal should produce audio");
    assert!(rms_quiet > 0.001, "Quiet should still produce audio");
}

#[test]
fn test_gain_pattern() {
    // Test that gain can be controlled by a pattern
    let code = r#"
tempo: 2.0
out: s "bd*8" # gain "1.0 0.2 1.0 0.2 1.0 0.2 1.0 0.2"
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050); // 0.5 seconds at 44.1kHz

    let rms = calculate_rms(&buffer);

    // Should produce audio with varying amplitude
    assert!(
        rms > 0.01,
        "Pattern-controlled gain should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_gain_zero_silences() {
    // Test that gain=0 produces silence
    let code = r#"
tempo: 2.0
out: s "bd*4" # gain 0.0
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let peak = calculate_peak(&buffer);

    // Should be essentially silent (may have tiny floating point errors)
    assert!(
        peak < 0.001,
        "Gain=0 should produce silence, got peak={}",
        peak
    );
}

// ========== PAN PARAMETER TESTS ==========

#[test]
fn test_pan_constant_left() {
    // Test that pan=-1 pans hard left (currently mono, so this is a smoke test)
    let code = r#"
tempo: 2.0
out: s "bd*4" # pan -1.0
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio (panning works in voice manager)
    assert!(
        rms > 0.01,
        "Pan=-1 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_pan_constant_right() {
    // Test that pan=1 pans hard right
    let code = r#"
tempo: 2.0
out: s "bd*4" # pan 1.0
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Pan=1 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_pan_pattern() {
    // Test that pan can be controlled by a pattern
    let code = r#"
tempo: 2.0
out: s "hh*8" # pan "-1 0 1 0 -1 0 1 0"
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio with varying pan positions
    assert!(
        rms > 0.01,
        "Pattern-controlled pan should produce audio, got RMS={}",
        rms
    );
}

// ========== SPEED PARAMETER TESTS ==========

#[test]
fn test_speed_normal() {
    // Test that speed=1.0 plays at normal rate
    let code = r#"
tempo: 2.0
out: s "bd*4" # speed 1.0
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Speed=1.0 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_speed_double() {
    // Test that speed=2.0 plays twice as fast (octave up)
    let code = r#"
tempo: 2.0
out: s "bd*4" # speed 2.0
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio (double speed means sample finishes faster but same energy)
    assert!(
        rms > 0.01,
        "Speed=2.0 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_speed_half() {
    // Test that speed=0.5 plays half speed (octave down)
    let code = r#"
tempo: 2.0
out: s "bd*4" # speed 0.5
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Speed=0.5 should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_speed_pattern() {
    // Test that speed can be controlled by a pattern
    let code = r#"
tempo: 2.0
out: s "bd*4" # speed "1.0 2.0 0.5 1.5"
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio with varying playback speeds
    assert!(
        rms > 0.01,
        "Pattern-controlled speed should produce audio, got RMS={}",
        rms
    );
}

// ========== CUT GROUP TESTS ==========

#[test]
fn test_cut_group_basic() {
    // Test that cut groups work (samples in same cut group stop each other)
    // This is hard to test without analyzing timing, but we can verify it compiles
    let code = r#"
tempo: 2.0
out: s "hh*16" # cut 1
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Cut group should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_cut_group_pattern() {
    // Test that cut group can be controlled by a pattern
    let code = r#"
tempo: 2.0
out: s "hh*8" # cut "1 2 1 2 1 2 1 2"
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Pattern-controlled cut group should produce audio, got RMS={}",
        rms
    );
}

// ========== ATTACK/RELEASE ENVELOPE TESTS ==========

#[test]
fn test_attack_short() {
    // Test that short attack creates fast onset
    let code = r#"
tempo: 2.0
out: s "bd*4" # attack 0.001
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio (lower threshold since attack affects amplitude)
    assert!(
        rms > 0.005,
        "Short attack should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_release_short() {
    // Test that short release cuts off sample quickly
    let code = r#"
tempo: 2.0
out: s "bd*4" # release 0.05
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Short release should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_attack_release_together() {
    // Test that attack and release work together
    let code = r#"
tempo: 2.0
out: s "bd*4" # attack 0.01 # release 0.1
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Attack+Release should produce audio, got RMS={}",
        rms
    );
}

// ========== COMBINED PARAMETERS TESTS ==========

#[test]
fn test_multiple_parameters() {
    // Test that multiple DSP parameters can be used together
    let code = r#"
tempo: 2.0
out: s "bd*4" # gain 0.7 # pan 0.5 # speed 1.2
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Multiple parameters should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_all_parameters() {
    // Test that all DSP parameters can be used together
    let code = r#"
tempo: 2.0
out: s "bd*4" # gain 0.8 # pan -0.3 # speed 0.9 # cut 1 # attack 0.01 # release 0.2
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "All parameters together should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_pattern_controlled_parameters() {
    // Test that all parameters can be pattern-controlled simultaneously
    let code = r#"
tempo: 2.0
out: s "bd*4" # gain "0.8 1.0 0.6 0.9" # pan "-1 0 1 0" # speed "1.0 1.5 0.8 1.2"
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio with complex modulation
    assert!(
        rms > 0.01,
        "Pattern-controlled parameters should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_parameters_with_transforms() {
    // Test that DSP parameters work with pattern transforms
    let code = r#"
tempo: 2.0
out: s "bd sn" $ fast 2 # gain 0.7 # pan 0.5
"#;

    let (_rest, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = calculate_rms(&buffer);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Parameters with transforms should produce audio, got RMS={}",
        rms
    );
}
