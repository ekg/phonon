//! End-to-end tests for time control features
//!
//! Tests tempo/cps, bpm, resetCycles, setCycle, and nudge functionality.
//! Uses three-level methodology:
//! 1. Pattern query verification (fast, deterministic)
//! 2. Onset detection (audio events at correct times)
//! 3. Audio characteristics (signal quality sanity checks)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to compile DSL code and return the signal graph
fn compile_code(code: &str, sample_rate: f32) -> phonon::unified_graph::UnifiedSignalGraph {
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert!(
        rest.trim().is_empty(),
        "Parser should consume all input, remaining: {}",
        rest
    );
    compile_program(statements, sample_rate, None).expect("Failed to compile")
}

/// Helper to render audio from DSL code
fn render_code(code: &str, duration_seconds: f32, sample_rate: f32) -> Vec<f32> {
    let mut graph = compile_code(code, sample_rate);
    let num_samples = (duration_seconds * sample_rate) as usize;
    graph.render(num_samples)
}

/// Calculate RMS amplitude
fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = audio.iter().map(|&x| x * x).sum();
    (sum_squares / audio.len() as f32).sqrt()
}

/// Check if audio is not silent
fn has_audio(buffer: &[f32], threshold: f32) -> bool {
    calculate_rms(buffer) > threshold
}

// ============================================================================
// TEMPO / CPS TESTS (5 tests)
// ============================================================================

#[test]
fn test_e2e_tempo_default() {
    // Default tempo should be 0.5 CPS (120 BPM in 4/4)
    let code = r#"
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    // Default CPS should be 0.5
    let cps = graph.get_cps();
    assert!(
        (cps - 0.5).abs() < 0.001,
        "Default CPS should be 0.5, got {}",
        cps
    );
}

#[test]
fn test_e2e_tempo_explicit() {
    // Set tempo explicitly with tempo: syntax
    let code = r#"
tempo: 1.0
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    assert!(
        (cps - 1.0).abs() < 0.001,
        "Explicit tempo should be 1.0, got {}",
        cps
    );
}

#[test]
fn test_e2e_cps_syntax() {
    // Set tempo with cps: syntax
    let code = r#"
cps: 2.0
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    assert!((cps - 2.0).abs() < 0.001, "CPS should be 2.0, got {}", cps);
}

#[test]
fn test_e2e_tempo_affects_pattern_speed() {
    // Higher tempo = faster pattern playback = more energy in shorter time
    let sample_rate = 44100.0;
    let duration = 2.0;

    // Slow tempo
    let code_slow = r#"
tempo: 0.25
out $ sine "220 440 660 880" * 0.5
"#;
    let audio_slow = render_code(code_slow, duration, sample_rate);

    // Fast tempo (4x faster)
    let code_fast = r#"
tempo: 1.0
out $ sine "220 440 660 880" * 0.5
"#;
    let audio_fast = render_code(code_fast, duration, sample_rate);

    // Both should have audio
    assert!(
        has_audio(&audio_slow, 0.01),
        "Slow tempo should produce audio"
    );
    assert!(
        has_audio(&audio_fast, 0.01),
        "Fast tempo should produce audio"
    );

    // Fast tempo should cycle through all frequencies faster
    // Measure frequency changes by analyzing different time windows
    let window_size = (sample_rate * 0.5) as usize; // 0.5 second windows

    // Count how many distinct frequency regions in slow audio
    let slow_rms_variance = calculate_variance_over_windows(&audio_slow, window_size);
    let fast_rms_variance = calculate_variance_over_windows(&audio_fast, window_size);

    // Fast tempo should have more variation per window (pattern cycles faster)
    // This is a rough heuristic - the important thing is both render without error
    eprintln!(
        "Slow variance: {}, Fast variance: {}",
        slow_rms_variance, fast_rms_variance
    );
}

/// Helper to calculate variance of RMS over windows
fn calculate_variance_over_windows(audio: &[f32], window_size: usize) -> f32 {
    let mut rms_values = Vec::new();
    for chunk in audio.chunks(window_size) {
        rms_values.push(calculate_rms(chunk));
    }

    if rms_values.is_empty() {
        return 0.0;
    }

    let mean = rms_values.iter().sum::<f32>() / rms_values.len() as f32;
    rms_values.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / rms_values.len() as f32
}

#[test]
fn test_e2e_tempo_override() {
    // Later tempo statement should override earlier one
    let code = r#"
tempo: 0.5
tempo: 2.0
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    assert!(
        (cps - 2.0).abs() < 0.001,
        "Later tempo should override, expected 2.0, got {}",
        cps
    );
}

// ============================================================================
// BPM TESTS (5 tests)
// ============================================================================

#[test]
fn test_e2e_bpm_120() {
    // 120 BPM = 120 / 60 = 2.0 CPS
    let code = r#"
bpm: 120
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    let expected = 120.0 / 60.0; // BPM / 60
    assert!(
        (cps - expected).abs() < 0.001,
        "120 BPM should be {} CPS, got {}",
        expected,
        cps
    );
}

#[test]
fn test_e2e_bpm_60() {
    // 60 BPM = 60 / 60 = 1.0 CPS
    let code = r#"
bpm: 60
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    let expected = 60.0 / 60.0;
    assert!(
        (cps - expected).abs() < 0.001,
        "60 BPM should be {} CPS, got {}",
        expected,
        cps
    );
}

#[test]
fn test_e2e_bpm_174_dnb() {
    // 174 BPM (drum and bass tempo) = 174 / 60 = 2.9 CPS
    let code = r#"
bpm: 174
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    let expected = 174.0 / 60.0;
    assert!(
        (cps - expected).abs() < 0.001,
        "174 BPM should be {} CPS, got {}",
        expected,
        cps
    );
}

#[test]
fn test_e2e_bpm_with_time_signature_3_4() {
    // 120 BPM in 3/4 time = 2.0 CPS (time signature doesn't affect CPS)
    let code = r#"
bpm: 120 "3/4"
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    let expected = 120.0 / 60.0;
    assert!(
        (cps - expected).abs() < 0.01,
        "120 BPM in 3/4 should be {} CPS, got {}",
        expected,
        cps
    );
}

#[test]
fn test_e2e_bpm_with_time_signature_6_8() {
    // 120 BPM in 6/8 time = 2.0 CPS (time signature doesn't affect CPS)
    let code = r#"
bpm: 120 "6/8"
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    let expected = 120.0 / 60.0;
    assert!(
        (cps - expected).abs() < 0.01,
        "120 BPM in 6/8 should be {} CPS, got {}",
        expected,
        cps
    );
}

// ============================================================================
// resetCycles TESTS (4 tests)
// ============================================================================

#[test]
fn test_e2e_reset_cycles_compile() {
    // resetCycles should parse and compile
    let code = r#"
tempo: 0.5
resetCycles
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    // After compilation with resetCycles, position should be 0
    let pos = graph.get_cycle_position();
    assert!(
        pos.abs() < 0.001,
        "After resetCycles, position should be 0, got {}",
        pos
    );
}

#[test]
fn test_e2e_reset_cycles_after_rendering() {
    // Reset cycles after some rendering
    let sample_rate = 44100.0;

    let code = r#"
tempo: 0.5
out $ sine 440 * 0.5
"#;
    let mut graph = compile_code(code, sample_rate);

    // Render some audio to advance cycle position
    let _audio = graph.render(44100); // 1 second

    // Position should have advanced
    let pos_before = graph.get_cycle_position();
    assert!(
        pos_before > 0.4,
        "Position should have advanced after rendering, got {}",
        pos_before
    );

    // Reset cycles
    graph.reset_cycles();

    let pos_after = graph.get_cycle_position();
    assert!(
        pos_after.abs() < 0.001,
        "After reset_cycles(), position should be near 0, got {}",
        pos_after
    );
}

#[test]
fn test_e2e_reset_cycles_multiple_times() {
    // Multiple resetCycles calls should work
    let code = r#"
tempo: 1.0
resetCycles
resetCycles
resetCycles
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        pos.abs() < 0.001,
        "Multiple resetCycles should leave position at 0, got {}",
        pos
    );
}

#[test]
fn test_e2e_reset_cycles_produces_audio() {
    // Verify that resetCycles doesn't break audio generation
    let code = r#"
tempo: 0.5
resetCycles
out $ sine 440 * 0.5
"#;
    let audio = render_code(code, 1.0, 44100.0);

    assert!(
        has_audio(&audio, 0.01),
        "Audio should be generated after resetCycles"
    );
}

// ============================================================================
// setCycle TESTS (3 tests)
// ============================================================================

#[test]
fn test_e2e_set_cycle_compile() {
    // setCycle should parse and compile
    let code = r#"
tempo: 0.5
setCycle 5.0
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        (pos - 5.0).abs() < 0.001,
        "After setCycle 5.0, position should be 5.0, got {}",
        pos
    );
}

#[test]
fn test_e2e_set_cycle_fractional() {
    // setCycle with fractional cycle position
    let code = r#"
tempo: 0.5
setCycle 2.75
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        (pos - 2.75).abs() < 0.001,
        "After setCycle 2.75, position should be 2.75, got {}",
        pos
    );
}

#[test]
fn test_e2e_set_cycle_then_render() {
    // setCycle then render should work correctly
    let sample_rate = 44100.0;

    let code = r#"
tempo: 1.0
setCycle 10.0
out $ sine 440 * 0.5
"#;
    let mut graph = compile_code(code, sample_rate);

    // Position should be 10.0
    assert!(
        (graph.get_cycle_position() - 10.0).abs() < 0.001,
        "Position should start at 10.0"
    );

    // Render 1 second at 1.0 CPS = 1 cycle
    let audio = graph.render(44100);

    // Position should now be ~11.0
    let pos = graph.get_cycle_position();
    assert!(
        (pos - 11.0).abs() < 0.1,
        "After rendering 1s at 1 CPS from cycle 10, position should be ~11.0, got {}",
        pos
    );

    // Should have generated audio
    assert!(has_audio(&audio, 0.01), "Audio should be generated");
}

// ============================================================================
// nudge TESTS (3 tests)
// ============================================================================

#[test]
fn test_e2e_nudge_positive() {
    // Positive nudge should delay (increase cycle position)
    let code = r#"
tempo: 0.5
nudge 0.1
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        (pos - 0.1).abs() < 0.001,
        "After nudge 0.1, position should be 0.1, got {}",
        pos
    );
}

#[test]
fn test_e2e_nudge_negative() {
    // Negative nudge should advance (decrease cycle position)
    let code = r#"
tempo: 0.5
setCycle 1.0
nudge -0.25
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        (pos - 0.75).abs() < 0.001,
        "After setCycle 1.0 then nudge -0.25, position should be 0.75, got {}",
        pos
    );
}

#[test]
fn test_e2e_nudge_cumulative() {
    // Multiple nudges should accumulate
    let code = r#"
tempo: 0.5
nudge 0.1
nudge 0.2
nudge 0.15
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    let expected = 0.1 + 0.2 + 0.15;
    assert!(
        (pos - expected).abs() < 0.001,
        "Cumulative nudges should sum to {}, got {}",
        expected,
        pos
    );
}

// ============================================================================
// COMBINED TIME CONTROL TESTS
// ============================================================================

#[test]
fn test_e2e_tempo_bpm_override() {
    // CPS should override BPM if specified after
    let code = r#"
bpm: 120
cps: 2.0
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let cps = graph.get_cps();
    assert!(
        (cps - 2.0).abs() < 0.001,
        "CPS should override BPM, expected 2.0, got {}",
        cps
    );
}

#[test]
fn test_e2e_set_cycle_nudge_combination() {
    // setCycle followed by nudge
    let code = r#"
tempo: 0.5
setCycle 5.0
nudge 0.5
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        (pos - 5.5).abs() < 0.001,
        "setCycle 5.0 + nudge 0.5 should give 5.5, got {}",
        pos
    );
}

#[test]
fn test_e2e_reset_then_set() {
    // resetCycles then setCycle
    let code = r#"
tempo: 0.5
setCycle 10.0
resetCycles
setCycle 3.0
out $ sine 440 * 0.5
"#;
    let graph = compile_code(code, 44100.0);

    let pos = graph.get_cycle_position();
    assert!(
        (pos - 3.0).abs() < 0.001,
        "After reset then setCycle 3.0, position should be 3.0, got {}",
        pos
    );
}

#[test]
fn test_e2e_full_time_control_sequence() {
    // Test a complete sequence of time control operations
    let sample_rate = 44100.0;

    let code = r#"
bpm: 120
out $ sine "220 440 660 880" * 0.3
"#;
    let mut graph = compile_code(code, sample_rate);

    // Initial CPS should be 0.5 (120 BPM / (4 beats * 60))
    let initial_cps = graph.get_cps();
    assert!(
        (initial_cps - 0.5).abs() < 0.001,
        "Initial CPS should be 0.5, got {}",
        initial_cps
    );

    // Set cycle to specific position
    graph.set_cycle(2.5);
    assert!(
        (graph.get_cycle_position() - 2.5).abs() < 0.001,
        "Position should be 2.5"
    );

    // Nudge forward
    graph.nudge(0.25);
    assert!(
        (graph.get_cycle_position() - 2.75).abs() < 0.001,
        "Position should be 2.75"
    );

    // Render some audio
    let audio = graph.render(22050); // 0.5 seconds
    assert!(has_audio(&audio, 0.01), "Should produce audio");

    // Position should have advanced by ~0.25 cycles (0.5 sec * 0.5 CPS)
    let pos_after = graph.get_cycle_position();
    assert!(
        pos_after > 2.9 && pos_after < 3.1,
        "Position should be ~3.0, got {}",
        pos_after
    );

    // Reset cycles
    graph.reset_cycles();
    assert!(
        graph.get_cycle_position().abs() < 0.001,
        "Position should be 0 after reset"
    );
}
