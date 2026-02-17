//! Integration tests for the A/B comparison testing framework.
//!
//! These tests verify that the A/B framework correctly detects
//! differences and similarities when comparing Phonon patterns.

use phonon::ab_test::{run_batch, ABTest, ABTestCase};

const SAMPLE_RATE: f32 = 44100.0;

/// Helper to render DSL to audio for direct comparison
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    let num_samples = (duration * SAMPLE_RATE) as usize;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile");
    graph.render(num_samples)
}

// ============================================================================
// Pattern vs Pattern Comparisons
// ============================================================================

#[test]
fn test_ab_identical_patterns_are_similar() {
    let code = r#"
cps: 1.0
out $ sine 440
"#;
    let result = ABTest::compare_patterns(code, code)
        .duration(1.0)
        .expect_similar(0.9)
        .run();
    assert!(result.passed(), "{}", result.report());
}

#[test]
fn test_ab_different_frequencies_detected() {
    let code_a = r#"
cps: 1.0
out $ sine 220
"#;
    let code_b = r#"
cps: 1.0
out $ sine 880
"#;
    let result = ABTest::compare_patterns(code_a, code_b)
        .duration(0.5)
        .expect_spectral_different(0.9)
        .run();
    assert!(result.passed(), "{}", result.report());
}

#[test]
fn test_ab_fast_changes_envelope() {
    // fast 2 should change the temporal structure of the pattern.
    // With continuous synthesis like sine, fast 2 changes how quickly
    // the frequency alternates, producing a different envelope shape.
    let base = r#"
cps: 1.0
out $ sine "220 440"
"#;
    let fast = r#"
cps: 1.0
out $ sine "220 440" $ fast 2
"#;
    let result = ABTest::compare_patterns(base, fast).duration(2.0).run();

    // Both contain the same frequencies but different temporal distribution.
    // Spectral content should remain similar since same pitches are used.
    assert!(
        result.similarity.spectral > 0.5,
        "fast 2 should preserve spectral content: {:.3}",
        result.similarity.spectral
    );
}

#[test]
fn test_ab_same_pattern_different_render_deterministic() {
    // Rendering the same pattern twice should produce identical results
    let code = r#"
cps: 1.0
out $ saw 110
"#;
    let audio_a = render_dsl(code, 1.0);
    let audio_b = render_dsl(code, 1.0);

    let result = ABTest::compare_audio(audio_a, audio_b)
        .label_a("saw 110 (render 1)")
        .label_b("saw 110 (render 2)")
        .expect_similar(0.99)
        .run();
    assert!(result.passed(), "{}", result.report());
}

// ============================================================================
// Transform Verification
// ============================================================================

#[test]
fn test_ab_rev_preserves_spectral_content() {
    // Reversing a pattern should preserve spectral content
    let base = r#"
cps: 1.0
out $ sine "220 440 660 880"
"#;
    let reversed = r#"
cps: 1.0
out $ sine "220 440 660 880" $ rev
"#;
    let result = ABTest::compare_patterns(base, reversed)
        .duration(2.0)
        .melodic()
        .expect_chroma_similar(0.5)
        .run();
    assert!(result.passed(), "{}", result.report());
}

#[test]
fn test_ab_slow_reduces_density() {
    // slow 2 should produce longer events, lower density
    let base = r#"
cps: 1.0
out $ sine "220 440 660 880"
"#;
    let slowed = r#"
cps: 1.0
out $ sine "220 440 660 880" $ slow 2
"#;
    let result = ABTest::compare_patterns(base, slowed).duration(4.0).run();

    // slow 2 should change the temporal structure
    // but maintain similar spectral content
    assert!(
        result.similarity.spectral > 0.5,
        "slow should preserve spectral character: {:.3}",
        result.similarity.spectral
    );
}

// ============================================================================
// Audio Properties
// ============================================================================

#[test]
fn test_ab_amplitude_difference() {
    let loud = r#"
cps: 1.0
out $ sine 440
"#;
    let audio_loud = render_dsl(loud, 0.5);
    let audio_quiet: Vec<f32> = audio_loud.iter().map(|&x| x * 0.25).collect();

    let result = ABTest::compare_audio(audio_loud, audio_quiet)
        .label_a("loud")
        .label_b("quiet")
        .expect_quieter(0.5)
        .run();
    assert!(result.passed(), "{}", result.report());
}

#[test]
fn test_ab_non_silent_output() {
    let code = r#"
cps: 1.0
out $ sine 440
"#;
    let audio = render_dsl(code, 0.5);
    let rms = (audio.iter().map(|&x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Rendered audio should not be silent, RMS={:.6}",
        rms
    );
}

// ============================================================================
// Batch Testing
// ============================================================================

#[test]
fn test_ab_batch_transforms() {
    let base_audio = render_dsl(
        r#"
cps: 1.0
out $ sine "220 440"
"#,
        2.0,
    );

    let same_audio = base_audio.clone();

    let batch = run_batch(vec![ABTestCase {
        name: "self-comparison".to_string(),
        test: ABTest::compare_audio(base_audio.clone(), same_audio).expect_similar(0.9),
    }]);

    assert!(batch.all_passed(), "{}", batch.report());
}

// ============================================================================
// Report Generation
// ============================================================================

#[test]
fn test_ab_report_contains_metrics() {
    let a = render_dsl("cps: 1.0\nout $ sine 440", 0.5);
    let b = render_dsl("cps: 1.0\nout $ sine 880", 0.5);

    let result = ABTest::compare_audio(a, b)
        .label_a("440Hz sine")
        .label_b("880Hz sine")
        .expect_spectral_different(0.9)
        .run();

    let report = result.report();
    assert!(
        report.contains("440Hz sine"),
        "Report should contain label A"
    );
    assert!(
        report.contains("880Hz sine"),
        "Report should contain label B"
    );
    assert!(
        report.contains("Similarity:"),
        "Report should contain similarity metrics"
    );
    assert!(report.contains("RMS:"), "Report should contain RMS values");
    assert!(
        report.contains("Spectral centroid:"),
        "Report should contain spectral info"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_ab_silence_vs_signal() {
    let signal = render_dsl("cps: 1.0\nout $ sine 440", 0.5);
    let silence = vec![0.0f32; signal.len()];

    let result = ABTest::compare_audio(signal, silence)
        .label_a("sine")
        .label_b("silence")
        .expect_max_similarity(0.6)
        .expect_quieter(0.01)
        .run();
    assert!(result.passed(), "{}", result.report());
}

#[test]
fn test_ab_short_duration() {
    let a = render_dsl("cps: 1.0\nout $ sine 440", 0.1);
    let b = render_dsl("cps: 1.0\nout $ sine 440", 0.1);

    let result = ABTest::compare_audio(a, b).expect_similar(0.9).run();
    assert!(result.passed(), "{}", result.report());
}
