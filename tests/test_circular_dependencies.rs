/// Circular Bus Dependency Tests
///
/// These tests verify that circular bus dependencies work in the DSL:
/// - Self-referential buses (~a $ ~a + ...) - WORKING
/// - Two-bus cycles (~a $ ~b, ~b $ ~a) - WORKING
/// - Three-bus cycles (~a $ ~b, ~b $ ~c, ~c $ ~a) - WORKING
/// - Complex cross-feedback networks (4 taps, FM synthesis) - WORKING
///
/// The placeholder-based two-pass compilation allows forward and circular references:
/// 1. Pass 1: Pre-register all bus names with placeholder nodes (Constant 0.0)
/// 2. Pass 2: Compile expressions with forward references resolved to real nodes
///
/// Note: The `mix` function with bus references as parameters is not yet supported
/// (separate from the circular dependency issue).

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ============================================================================
// Self-Referential Buses
// ============================================================================

#[test]
fn test_self_referential_feedback_basic() {
    // Basic self-feedback: signal mixes with delayed version of itself
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~feedback $ ~input * 0.5 + ~feedback * 0.3
        out $ ~feedback
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Self-referential feedback should produce audio, got RMS: {}", rms);
}

#[test]
fn test_self_referential_with_processing() {
    // Self-feedback with filtering
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.5
        ~feedback $ (~input * 0.6 + ~feedback * 0.4) # lpf 2000 0.8
        out $ ~feedback * 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Self-referential with processing should produce audio, got RMS: {}", rms);
}

#[test]
fn test_self_referential_reverb_injection() {
    // The exact pattern from the original question:
    // "we have a reverb or delay in a hard self loop and then
    //  in the self loop we have a mix with another input"
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.5
        ~feedback $ (~feedback * 0.7 + ~input * 0.3) # reverb 0.95 0.3 0.8
        out $ ~feedback * 0.5
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Reverb with self-feedback should produce audio, got RMS: {}", rms);
}

// ============================================================================
// Two-Bus Circular Dependencies (a -> b -> a)
// ============================================================================

#[test]
fn test_two_bus_cycle_basic() {
    // The exact pattern from the user's question: "a -> b -> a"
    let code = r#"
        tempo: 0.5
        ~a $ ~b # lpf 1000 0.8
        ~b $ ~a # delay 0.1 0.5
        out $ ~a * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let _rms = calculate_rms(&buffer);

    // This creates a feedback loop with no external input, so it will
    // eventually decay to silence, but should still compile and run
    assert!(buffer.len() > 0, "Two-bus cycle should compile and render");
}

#[test]
fn test_two_bus_cycle_with_input() {
    // Two-bus cycle with external input injection
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~a $ ~input * 0.5 + ~b * 0.3
        ~b $ ~a # delay 0.125 0.6
        out $ ~a * 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Two-bus cycle with input should produce audio, got RMS: {}", rms);
}

#[test]
fn test_two_bus_cross_feedback_delay() {
    // Stereo ping-pong delay (cross-feedback)
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.5
        ~left $ (~left * 0.4 + ~right * 0.2 + ~input * 0.4) # delay 0.25 0.6
        ~right $ (~right * 0.4 + ~left * 0.2 + ~input * 0.4) # delay 0.33 0.5
        out $ ~left + ~right
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Cross-feedback delay should produce audio, got RMS: {}", rms);
}

// ============================================================================
// Three-Bus Circular Dependencies (a -> b -> c -> a)
// ============================================================================

#[test]
fn test_three_bus_cycle() {
    // Three buses in circular dependency
    let code = r#"
        tempo: 0.5
        ~input $ sine 220 * 0.4
        ~a $ ~input * 0.4 + ~c * 0.2
        ~b $ ~a # lpf 2000 0.7
        ~c $ ~b # delay 0.1 0.5
        out $ ~a + ~b + ~c
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Three-bus cycle should produce audio, got RMS: {}", rms);
}

#[test]
fn test_three_bus_cycle_different_effects() {
    // Three buses with different processing
    let code = r#"
        tempo: 0.5
        ~input $ saw 110 * 0.5
        ~a $ (~input * 0.5 + ~c * 0.2) # lpf 1500 0.8
        ~b $ ~a # delay 0.125 0.6
        ~c $ ~b # hpf 500 0.7
        out $ (~a + ~b + ~c) * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Three-bus cycle with effects should produce audio, got RMS: {}", rms);
}

// ============================================================================
// Complex Circular Patterns
// ============================================================================

#[test]
fn test_fm_in_self_feedback_loop() {
    // FM synthesis where modulator is in self-feedback loop
    // "or a fm synth or something" from the user's question
    let code = r#"
        tempo: 0.5
        ~input $ sine 2.0 * 0.5
        ~modulator $ (~modulator * 0.5 + ~input * 0.5) # lpf 2000 0.8
        ~fm $ sine (~modulator * 100 + 440)
        out $ ~fm * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "FM in self-feedback loop should produce audio, got RMS: {}", rms);
}

#[test]
fn test_four_tap_cross_feedback_network() {
    // Four delay taps with cross-feedback (reverb-like diffusion)
    let code = r#"
        tempo: 1.0
        ~input $ sine 880 * 0.3
        ~tap1 $ (~input * 0.25 + ~tap2 * 0.15 + ~tap4 * 0.1) # delay 0.037 0.7
        ~tap2 $ (~input * 0.25 + ~tap1 * 0.15 + ~tap3 * 0.1) # delay 0.043 0.7
        ~tap3 $ (~input * 0.25 + ~tap2 * 0.15 + ~tap4 * 0.1) # delay 0.051 0.7
        ~tap4 $ (~input * 0.25 + ~tap3 * 0.15 + ~tap1 * 0.1) # delay 0.061 0.7
        out $ (~tap1 + ~tap2 + ~tap3 + ~tap4) * 0.3
    "#;

    let buffer = render_dsl(code, 3.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Four-tap cross-feedback should produce audio, got RMS: {}", rms);
}

#[test]
fn test_karplus_strong_feedback() {
    // Karplus-Strong plucked string (self-referential delay)
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.8
        ~string $ (~string * 0.98 + ~input * 0.02) # delay 0.00227 0.995
        out $ ~string * 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Karplus-Strong should produce audio, got RMS: {}", rms);
}

#[test]
#[ignore = "mix function with bus reference as param not yet supported"]
fn test_mix_function_in_circular_feedback() {
    // Mix function used in circular feedback
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~feedback $ mix ~feedback ~input
        out $ ~feedback * 0.7
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "Mix function in circular feedback should produce audio, got RMS: {}", rms);
}
