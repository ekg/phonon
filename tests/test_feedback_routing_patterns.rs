/// Feedback Routing Pattern Tests
///
/// Tests complex feedback routing scenarios that work within DSL limitations:
/// - Delay feedback (using effect feedback parameters)
/// - Effect chain feedback (reverb, delay with internal feedback)
/// - FM synthesis with modulation
/// - Mix operators for signal blending
/// - Complex routing through buses
///
/// NOTE: Circular bus dependencies (~a $ ~b, ~b $ ~a) are NOT supported by the DSL compiler.
/// Feedback must be achieved through effect parameters (delay feedback, reverb feedback, etc.)
/// or through the audio graph structure itself.
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

// ============================================================================
// Delay Feedback Tests
// ============================================================================

#[test]
fn test_delay_with_high_feedback() {
    // Delay with high feedback parameter (internal feedback loop)
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.4
        ~delayed $ ~input # delay 0.25 0.8
        out $ ~input * 0.3 + ~delayed * 0.7
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Delay with feedback should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_cascaded_delays() {
    // Cascade multiple delays (each has internal feedback)
    let code = r#"
        tempo: 1.0
        ~source $ sine 880 * 0.4
        ~echo1 $ ~source # delay 0.25 0.6
        ~echo2 $ ~echo1 # delay 0.25 0.5
        ~echo3 $ ~echo2 # delay 0.25 0.4
        out $ ~source * 0.4 + ~echo1 * 0.3 + ~echo2 * 0.2 + ~echo3 * 0.1
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Cascaded delays should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_delay_feedback_builds_density() {
    // Verify high feedback builds up echo density
    let low_feedback = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~delayed $ ~input # delay 0.125 0.2
        out $ ~delayed
    "#;

    let high_feedback = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~delayed $ ~input # delay 0.125 0.8
        out $ ~delayed
    "#;

    let low_buffer = render_dsl(low_feedback, 2.0);
    let high_buffer = render_dsl(high_feedback, 2.0);

    let low_rms = calculate_rms(&low_buffer);
    let high_rms = calculate_rms(&high_buffer);

    // High feedback should have more sustained energy
    assert!(
        high_rms > low_rms * 1.3,
        "High feedback should have more energy (low: {}, high: {})",
        low_rms,
        high_rms
    );
}

// ============================================================================
// Reverb Feedback Tests
// ============================================================================

#[test]
fn test_reverb_with_large_room() {
    // Large room size creates long feedback loops
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.4
        ~reverb $ ~input # reverb 0.95 0.4 0.7
        out $ ~input * 0.2 + ~reverb * 0.8
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Large reverb should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_cascaded_reverbs() {
    // Multiple reverbs in series (compound feedback)
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.3
        ~verb1 $ ~input # reverb 0.7 0.5 0.6
        ~verb2 $ ~verb1 # reverb 0.6 0.4 0.5
        out $ ~verb2 * 0.7
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.01,
        "Cascaded reverbs should produce audio, got RMS: {}",
        rms
    );
}

// ============================================================================
// Cross-Feedback Through Explicit Routing
// ============================================================================

#[test]
fn test_two_path_mixing() {
    // Two signal paths mixed together (not circular, just mixing)
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.5
        ~path_a $ ~input # lpf 1500 0.8 # delay 0.25 0.6
        ~path_b $ ~input # hpf 500 0.8 # delay 0.33 0.5
        out $ ~path_a * 0.5 + ~path_b * 0.5
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Two-path mixing should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_parallel_effect_chains() {
    // Parallel processing with different effects
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.4
        ~wet $ ~input # delay 0.125 0.5 # reverb 0.7 0.4 0.6
        ~dry $ ~input
        out $ ~dry * 0.4 + ~wet * 0.6
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Parallel effects should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_multi_tap_delay() {
    // Multiple delay taps from same source
    let code = r#"
        tempo: 1.0
        ~input $ sine 880 * 0.3
        ~tap1 $ ~input # delay 0.037 0.7
        ~tap2 $ ~input # delay 0.043 0.7
        ~tap3 $ ~input # delay 0.051 0.7
        ~tap4 $ ~input # delay 0.061 0.7
        out $ (~input + ~tap1 + ~tap2 + ~tap3 + ~tap4) * 0.2
    "#;

    let buffer = render_dsl(code, 3.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Multi-tap delay should produce audio, got RMS: {}",
        rms
    );
}

// ============================================================================
// FM Synthesis Tests
// ============================================================================

#[test]
fn test_fm_synthesis() {
    // FM synthesis with slow modulator
    let code = r#"
        tempo: 0.5
        ~modulator $ sine 5.0
        ~fm $ sine (~modulator * 100 + 440)
        out $ ~fm * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "FM synthesis should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_complex_fm_with_multiple_modulators() {
    // Multiple modulators affecting carrier
    let code = r#"
        tempo: 0.5
        ~lfo1 $ sine 3.0 * 50
        ~lfo2 $ sine 7.0 * 30
        ~carrier $ sine (~lfo1 + ~lfo2 + 440)
        out $ ~carrier * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Complex FM should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_fm_with_audio_rate_modulation() {
    // Fast modulation (creates sidebands)
    let code = r#"
        tempo: 0.5
        ~modulator $ sine 220
        ~fm $ sine (~modulator * 2.0 + 440)
        out $ ~fm * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Audio-rate FM should produce audio, got RMS: {}",
        rms
    );
}

// ============================================================================
// Mix Operator Tests
// ============================================================================

#[test]
fn test_mix_function_basic() {
    // Mix function with multiple signals
    let code = r#"
        tempo: 0.5
        ~sig1 $ sine 220 * 0.5
        ~sig2 $ saw 330 * 0.4
        ~sig3 $ square 440 * 0.3
        ~mixed $ mix ~sig1 ~sig2 ~sig3
        out $ ~mixed * 0.8
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Mix function should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_bus_arithmetic_mixing() {
    // Manual bus arithmetic for mixing
    let code = r#"
        tempo: 0.5
        ~a $ sine 440 * 0.5
        ~b $ saw 220 * 0.4
        ~mixed $ ~a * 0.6 + ~b * 0.4
        out $ ~mixed
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "Bus arithmetic mixing should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_complex_bus_arithmetic() {
    // Complex arithmetic expressions with buses
    let code = r#"
        tempo: 0.5
        ~lfo $ sine 0.5
        ~carrier $ sine 440
        ~modulated $ ~carrier * (~lfo * 0.5 + 0.5)
        out $ ~modulated * 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Complex bus arithmetic should produce audio, got RMS: {}",
        rms
    );
}

// ============================================================================
// Production Scenario Tests
// ============================================================================

#[test]
fn test_dub_delay_chain() {
    // Real dub delay scenario with HPF in feedback
    let code = r#"
        tempo: 0.5
        ~input $ sine 55 * 0.6
        ~dub $ ~input # delay 0.375 0.75 # hpf 800 0.7
        out $ ~input * 0.5 + ~dub * 0.5
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Dub delay chain should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_reverb_with_modulation() {
    // Reverb with LFO-modulated wet mix
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.4
        ~lfo $ sine 0.5 * 0.3 + 0.5
        ~reverb $ ~input # reverb 0.9 0.4 ~lfo
        out $ ~reverb * 0.6
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Reverb with modulation should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_multi_stage_delay_reverb() {
    // Delay followed by reverb (common production technique)
    let code = r#"
        tempo: 1.0
        ~input $ sine 880 * 0.3
        ~delayed $ ~input # delay 0.125 0.6
        ~reverb $ ~delayed # reverb 0.8 0.4 0.7
        out $ ~input * 0.2 + ~reverb * 0.8
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Multi-stage processing should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_parallel_compression_mixing() {
    // Parallel compression (New York style)
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.6
        ~compressed $ ~input # compressor -20.0 4.0 0.01 0.1 1.0
        ~dry $ ~input
        out $ ~dry * 0.5 + ~compressed * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Parallel compression should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_send_return_style_reverb() {
    // Send/return style effect routing
    let code = r#"
        tempo: 0.5
        ~dry1 $ sine 440 * 0.3
        ~dry2 $ saw 220 * 0.3
        ~send $ (~dry1 + ~dry2) * 0.5
        ~return $ ~send # reverb 0.9 0.5 0.9
        out $ ~dry1 * 0.4 + ~dry2 * 0.4 + ~return * 0.3
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Send/return style should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_filter_feedback_sweep() {
    // Filter cutoff swept by LFO, with delay feedback
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.5
        ~lfo $ sine 0.25 * 2000 + 1000
        ~filtered $ ~input # lpf ~lfo 0.8 # delay 0.25 0.6
        out $ ~filtered * 0.7
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Filter sweep with delay should produce audio, got RMS: {}",
        rms
    );
}
