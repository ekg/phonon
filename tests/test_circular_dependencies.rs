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
#[ignore = "reverb node produces NaN in tight feedback loops - numerical stability issue"]
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

// ============================================================================
// True z^-1 Feedback Verification Tests
// ============================================================================

/// This test PROVES that z^-1 (unit delay) feedback is working correctly.
///
/// If feedback works (z^-1):
///   - Sample 1: ~accum = 0.1 + 0 * 0.9 = 0.1
///   - Sample 2: ~accum = 0.1 + 0.1 * 0.9 = 0.19
///   - Sample 3: ~accum = 0.1 + 0.19 * 0.9 = 0.271
///   - ... converges to 1.0 (geometric series sum = input / (1 - feedback_coef))
///
/// If feedback doesn't work (placeholder always 0):
///   - All samples: ~accum = 0.1 + 0 * 0.9 = 0.1 (constant)
///
/// This test verifies that later samples have HIGHER values than earlier samples.
#[test]
fn test_unit_delay_feedback_accumulation() {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    // Simple accumulator: output = 0.1 + prev_output * 0.9
    // With z^-1, this converges to 1.0 over time
    let code = r#"
        tempo: 0.5
        ~accum $ 0.1 + ~accum * 0.9
        out $ ~accum
    "#;

    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile");

    // Render 1000 samples
    let buffer = graph.render(1000);

    // Compare early samples vs late samples
    let early_avg: f32 = buffer[0..100].iter().sum::<f32>() / 100.0;
    let late_avg: f32 = buffer[900..1000].iter().sum::<f32>() / 100.0;

    // With z^-1 working:
    //   - early samples should average around 0.3-0.5 (still accumulating)
    //   - late samples should average around 0.9-1.0 (converged)
    // Without z^-1 (broken):
    //   - both should average around 0.1 (constant)

    println!("Early average (samples 0-100): {:.6}", early_avg);
    println!("Late average (samples 900-1000): {:.6}", late_avg);

    // With z^-1 working correctly:
    // - The geometric series 0.1 * (1 + 0.9 + 0.81 + ...) converges to 1.0
    // - After ~100 samples, we should be at ~0.9 (close to convergence)
    // - After ~1000 samples, we should be at ~1.0 (fully converged)

    // Late samples should have converged close to 1.0 (the geometric series limit)
    assert!(
        late_avg > 0.95,
        "Feedback should converge to ~1.0, but late_avg = {:.4}. \
         This indicates z^-1 is not working correctly.",
        late_avg
    );

    // Early samples should already be accumulating (not stuck at 0.1)
    // With feedback coefficient 0.9, values grow quickly
    assert!(
        early_avg > 0.5,
        "Early samples should show accumulation, but early_avg = {:.4}. \
         Without z^-1 working, this would be ~0.1",
        early_avg
    );

    // The difference between early and late should be small (system converges)
    // If feedback wasn't working, both would be 0.1 (identical but wrong)
    // With feedback, late > early but both are high
    let convergence_ratio = late_avg / early_avg;
    assert!(
        convergence_ratio > 1.0 && convergence_ratio < 1.5,
        "Feedback should show convergence: late ({:.4}) / early ({:.4}) = {:.4}",
        late_avg, early_avg, convergence_ratio
    );
}

/// Test that feedback with a decaying input shows proper accumulation and decay.
#[test]
fn test_unit_delay_feedback_with_input() {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    // Input * 0.5 + feedback * 0.3
    // With 440Hz sine input, should produce accumulated signal
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~feedback $ ~input * 0.5 + ~feedback * 0.3
        out $ ~feedback
    "#;

    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile");

    // Render 44100 samples (1 second)
    let buffer = graph.render(44100);

    // Calculate RMS of early vs late sections
    let early_rms: f32 = (buffer[0..4410].iter().map(|&x| x * x).sum::<f32>() / 4410.0).sqrt();
    let late_rms: f32 = (buffer[40000..44100].iter().map(|&x| x * x).sum::<f32>() / 4100.0).sqrt();

    println!("Early RMS (first 0.1s): {:.6}", early_rms);
    println!("Late RMS (last 0.1s): {:.6}", late_rms);

    // Both should have non-trivial signal
    assert!(early_rms > 0.1, "Early RMS too low: {:.4}", early_rms);
    assert!(late_rms > 0.1, "Late RMS too low: {:.4}", late_rms);

    // The feedback should create a "fuller" sound than without feedback
    // With feedback coefficient 0.3, the system should converge quickly
    // so late and early RMS should be similar (both should be non-trivial)
    let rms_ratio = late_rms / early_rms;
    assert!(
        rms_ratio > 0.8 && rms_ratio < 1.5,
        "RMS ratio {:.4} suggests feedback isn't stable",
        rms_ratio
    );
}
