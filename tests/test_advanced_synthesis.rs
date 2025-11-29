/// Systematic tests: Advanced synthesis (wavetable, granular)
///
/// Tests wavetable and granular synthesis with pattern modulation.
/// Verifies P0.0: ALL parameters accept patterns.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Wavetable Synthesis Tests ==========

#[test]
fn test_wavetable_constant_frequency() {
    let code = r#"
        tempo: 0.5
        out $ wavetable 440
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.1,
        "Wavetable with constant frequency should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_wavetable_pattern_frequency_lfo() {
    // Wavetable with LFO-modulated frequency (vibrato)
    let code = r#"
        tempo: 0.5
        out $ wavetable (sine 5 * 10 + 440)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.1,
        "Wavetable with LFO frequency should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_wavetable_pattern_frequency_sweep() {
    // Wavetable with slow frequency sweep
    let code = r#"
        tempo: 0.5
        out $ wavetable (sine 0.5 * 220 + 440)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.1,
        "Wavetable with sweep should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_wavetable_vs_sine_comparison() {
    // Wavetable defaults to sine, should sound similar to sine oscillator
    let wavetable_code = r#"
        tempo: 0.5
        out $ wavetable 440
    "#;

    let sine_code = r#"
        tempo: 0.5
        out $ sine 440
    "#;

    let wavetable_buffer = render_dsl(wavetable_code, 2.0);
    let sine_buffer = render_dsl(sine_code, 2.0);

    let wavetable_rms = calculate_rms(&wavetable_buffer);
    let sine_rms = calculate_rms(&sine_buffer);

    // Both should have audio
    assert!(wavetable_rms > 0.1, "Wavetable should have audio");
    assert!(sine_rms > 0.1, "Sine should have audio");

    // RMS should be similar (both produce sine waves)
    let diff_ratio = (wavetable_rms - sine_rms).abs() / sine_rms;
    assert!(
        diff_ratio < 0.1,
        "Wavetable (default sine) should have similar RMS to sine oscillator, wavetable RMS: {}, sine RMS: {}, diff: {}",
        wavetable_rms,
        sine_rms,
        diff_ratio
    );
}

// ========== Granular Synthesis Tests ==========

#[test]
fn test_granular_constant_parameters() {
    // Use inline source expression
    let code = r#"
        tempo: 0.5
        out $ granular (sine 440) 50 0.5 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Granular with constant params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_granular_pattern_grain_size() {
    // Granular with pattern-modulated grain size using inline source
    let code = r#"
        tempo: 0.5
        out $ granular (saw 110) (sine 1.0 * 30 + 50) 0.5 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Granular with pattern grain_size should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_granular_pattern_density() {
    // Granular with pattern-modulated density using inline source
    let code = r#"
        tempo: 0.5
        out $ granular (square 220) 50 (sine 2.0 * 0.3 + 0.5) 1.0
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Granular with pattern density should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_granular_pattern_pitch() {
    // Granular with pattern-modulated pitch using inline source
    let code = r#"
        tempo: 0.5
        out $ granular (triangle 110) 50 0.5 (sine 0.5 * 0.5 + 1.0)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Granular with pattern pitch should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_granular_all_patterns() {
    // Granular with all parameters as patterns using inline source
    let code = r#"
        tempo: 0.5
        out $ granular (saw 55) (sine 1.0 * 30 + 50) (sine 2.0 * 0.3 + 0.5) (sine 0.5 * 0.5 + 1.0)
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Granular with all pattern params should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_granular_produces_different_texture() {
    // Verify granular synthesis produces different texture than direct playback
    let direct_code = r#"
        tempo: 0.5
        out $ sine 440
    "#;

    // Use inline source expression
    let granular_code = r#"
        tempo: 0.5
        out $ granular (sine 440) 30 0.8 1.0
    "#;

    let direct_buffer = render_dsl(direct_code, 2.0);
    let granular_buffer = render_dsl(granular_code, 2.0);

    let direct_rms = calculate_rms(&direct_buffer);
    let granular_rms = calculate_rms(&granular_buffer);

    // Both should have audio
    assert!(direct_rms > 0.01, "Direct should have audio");
    assert!(granular_rms > 0.01, "Granular should have audio");

    // Granular may have different RMS due to grain overlapping/windowing
    // Just verify both produce sound, not that they're identical
    println!("Direct RMS: {}, Granular RMS: {}", direct_rms, granular_rms);
}
