/// Tests for Type Inference (Phase 4)
///
/// Type inference automatically determines pattern vs signal context:
/// - Quoted strings -> pattern context
/// - Bus references -> signal context (when bus contains signal)
/// - Oscillators -> signal context
/// - Bare operators (+, -, *, /) adapt to context

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn compile_code(code: &str) -> Result<phonon::unified_graph::UnifiedSignalGraph, String> {
    let (rest, stmts) = parse_program(code).map_err(|e| format!("Parse error: {}", e))?;
    if !rest.trim().is_empty() {
        return Err(format!("Parser did not consume all input: {:?}", rest));
    }
    compile_program(stmts, 44100.0, None)
}

fn render_code(code: &str, samples: usize) -> Vec<f32> {
    let (_, stmts) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(stmts, 44100.0, None).expect("Compile failed");
    graph.render(samples)
}

// ============================================================================
// Context Detection Tests
// ============================================================================

#[test]
fn test_pattern_context_quoted_strings() {
    // Quoted strings are always patterns
    let code = r#"out $ s "bd sn hh cp""#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile pattern: {}", e),
    }
}

#[test]
fn test_signal_context_oscillators() {
    // Oscillators are always signals
    let code = r#"out $ sine 440"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile signal: {}", e),
    }
}

#[test]
fn test_signal_context_bus_reference() {
    // Bus references to signal buses are signals
    let code = r#"
~osc $ sine 440
out $ ~osc
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile signal bus: {}", e),
    }
}

// ============================================================================
// Bare Operator Context Adaptation Tests
// ============================================================================

#[test]
fn test_bare_add_pattern_context() {
    // Bare + in pattern context should work as pattern add
    // Using quoted patterns that get combined
    let code = r#"out $ s "bd" # speed "1" + "0.5""#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile pattern addition: {}", e),
    }
}

#[test]
fn test_bare_mul_pattern_context() {
    // Bare * in pattern context for gain patterns
    let code = r#"out $ s "bd" # gain "1" * "0.5""#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile pattern multiply: {}", e),
    }
}

// ============================================================================
// Constant Pattern Optimization Tests
// ============================================================================

#[test]
fn test_constant_pattern_single_value() {
    // Single value pattern should be optimizable to signal
    let code = r#"out $ sine "440""#;
    let output = render_code(code, 4410);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(rms > 0.3, "Constant pattern should produce signal, got RMS={}", rms);
}

#[test]
fn test_constant_pattern_optimization_equivalent() {
    // sine "440" should be equivalent to sine 440
    let code1 = r#"out $ sine 440"#;
    let code2 = r#"out $ sine "440""#;

    let output1 = render_code(code1, 4410);
    let output2 = render_code(code2, 4410);

    let rms1: f32 = (output1.iter().map(|s| s * s).sum::<f32>() / output1.len() as f32).sqrt();
    let rms2: f32 = (output2.iter().map(|s| s * s).sum::<f32>() / output2.len() as f32).sqrt();

    // Should have similar RMS (allowing some tolerance for implementation differences)
    let ratio = rms1 / rms2;
    assert!(
        (ratio - 1.0).abs() < 0.2,
        "Constant pattern should be similar to literal, got ratio={}",
        ratio
    );
}

// ============================================================================
// Mixed Context Tests
// ============================================================================

#[test]
fn test_pattern_controls_signal_param() {
    // Pattern can control oscillator frequency
    let code = r#"out $ saw "55 110 220""#;
    let output = render_code(code, 44100);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(rms > 0.01, "Pattern-controlled saw should produce output, RMS={}", rms);
}

#[test]
fn test_signal_ops_on_pattern_controlled() {
    // Signal operators work on pattern-controlled sources
    let code = r#"
~bass $ saw "55 110"
~env $ sine 2
out $ ~bass ~* ~env
"#;
    let output = render_code(code, 44100);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(rms > 0.01, "Signal ops on pattern source should work, RMS={}", rms);
}

// ============================================================================
// Error Message Tests
// ============================================================================

#[test]
fn test_undefined_bus_error_message() {
    // Referencing undefined bus should give clear error
    let code = r#"out $ ~undefined_bus"#;
    match compile_code(code) {
        Ok(_) => panic!("Should fail for undefined bus"),
        Err(e) => {
            assert!(
                e.contains("undefined") || e.contains("not found") || e.contains("Unknown"),
                "Error should mention undefined/not found: {}",
                e
            );
        }
    }
}

#[test]
fn test_wrong_arity_error_message() {
    // Calling function bus with wrong number of args should give clear error
    let code = r#"
~mix a b $ a ~+ b
out $ ~mix ~osc1
"#;
    match compile_code(code) {
        Ok(_) => panic!("Should fail for wrong arity"),
        Err(e) => {
            assert!(
                e.contains("argument") || e.contains("parameter") || e.contains("expected") || e.contains("arity"),
                "Error should mention argument/parameter count: {}",
                e
            );
        }
    }
}

// ============================================================================
// Audio Verification Tests - Rigorous Signal Checks
// ============================================================================

#[test]
fn test_inferred_context_produces_audio() {
    // Complex expression with mixed contexts
    let code = r#"
~lfo $ sine 2
~bass $ saw "55 110 220"
out $ ~bass ~* (~lfo ~* 0.3 ~+ 0.7)
"#;
    let output = render_code(code, 44100);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(rms > 0.01, "Inferred context should produce audio, RMS={}", rms);
}

#[test]
fn test_signal_add_produces_correct_sum() {
    // Two sine waves at same frequency should double amplitude
    let code_single = r#"out $ sine 440"#;
    let code_double = r#"
~a $ sine 440
~b $ sine 440
out $ ~a ~+ ~b
"#;
    let output_single = render_code(code_single, 4410);
    let output_double = render_code(code_double, 4410);

    // Compare peak values - doubled should be ~2x amplitude
    let peak_single = output_single.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    let peak_double = output_double.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));

    let ratio = peak_double / peak_single;
    assert!(
        (ratio - 2.0).abs() < 0.1,
        "Signal add should double amplitude, got ratio={:.3}",
        ratio
    );
}

#[test]
fn test_signal_mul_produces_correct_product() {
    // sine * 0.5 should halve amplitude
    let code_full = r#"out $ sine 440"#;
    let code_half = r#"out $ sine 440 ~* 0.5"#;

    let output_full = render_code(code_full, 4410);
    let output_half = render_code(code_half, 4410);

    let peak_full = output_full.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    let peak_half = output_half.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));

    let ratio = peak_half / peak_full;
    assert!(
        (ratio - 0.5).abs() < 0.05,
        "Signal mul by 0.5 should halve amplitude, got ratio={:.3}",
        ratio
    );
}

#[test]
fn test_signal_sub_produces_correct_difference() {
    // sine - sine (same phase) should produce silence
    let code = r#"
~a $ sine 440
~b $ sine 440
out $ ~a ~- ~b
"#;
    let output = render_code(code, 4410);

    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(
        rms < 0.001,
        "Signal sub of identical signals should be silent, got RMS={:.6}",
        rms
    );
}

#[test]
fn test_function_bus_produces_correct_output() {
    // Function bus ~double x $ x ~* 2 should double the signal
    let code_single = r#"out $ sine 440"#;
    let code_doubled = r#"
~double x $ x ~* 2
~osc $ sine 440
out $ ~double ~osc
"#;
    let output_single = render_code(code_single, 4410);
    let output_doubled = render_code(code_doubled, 4410);

    let peak_single = output_single.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    let peak_doubled = output_doubled.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));

    let ratio = peak_doubled / peak_single;
    assert!(
        (ratio - 2.0).abs() < 0.1,
        "Function bus ~double should double amplitude, got ratio={:.3}",
        ratio
    );
}

#[test]
fn test_function_bus_mix_correct_proportions() {
    // ~mix a b $ a ~* 0.7 ~+ b ~* 0.3 should mix at 70/30
    let code = r#"
~mix a b $ a ~* 0.7 ~+ b ~* 0.3
~a $ sine 440
~b $ sine 440
out $ ~mix ~a ~b
"#;
    let output_mix = render_code(code, 4410);

    // 0.7 + 0.3 = 1.0, so mixed signal should be same as single sine
    let code_single = r#"out $ sine 440"#;
    let output_single = render_code(code_single, 4410);

    let peak_mix = output_mix.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    let peak_single = output_single.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));

    let ratio = peak_mix / peak_single;
    assert!(
        (ratio - 1.0).abs() < 0.1,
        "Mix 0.7+0.3 should equal original, got ratio={:.3}",
        ratio
    );
}

#[test]
fn test_higher_order_bus_produces_audio() {
    // Higher-order bus should correctly apply transformation
    let code = r#"
~louder f $ f ~* 2
~osc $ sine 440
~result $ ~louder ~osc
out $ ~result
"#;
    let output = render_code(code, 4410);

    // Check it produces audio with reasonable amplitude
    let peak = output.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    assert!(
        peak > 1.5,
        "Higher-order bus with ~* 2 should produce peak > 1.5, got {:.3}",
        peak
    );
}

#[test]
fn test_direct_effect_applies() {
    // Direct effect application (not via bus) should work
    let code_unfiltered = r#"out $ saw 110"#;
    let code_filtered = r#"out $ saw 110 # lpf 500 0.8"#;

    let output_unfiltered = render_code(code_unfiltered, 4410);
    let output_filtered = render_code(code_filtered, 4410);

    // Filtered signal should have lower RMS (less high-frequency energy)
    let rms_unfiltered: f32 = (output_unfiltered.iter().map(|s| s * s).sum::<f32>() / output_unfiltered.len() as f32).sqrt();
    let rms_filtered: f32 = (output_filtered.iter().map(|s| s * s).sum::<f32>() / output_filtered.len() as f32).sqrt();

    // Low-pass filter should reduce RMS
    assert!(
        rms_filtered < rms_unfiltered,
        "Filtered signal RMS ({:.4}) should be less than unfiltered ({:.4})",
        rms_filtered,
        rms_unfiltered
    );
}

#[test]
fn test_transformer_bus_applies_effect() {
    // Transformer bus: ~fx $ lpf 500 0.8 applied via saw 110 # ~fx
    let code_unfiltered = r#"out $ saw 110"#;
    let code_filtered = r#"
~fx $ lpf 500 0.8
out $ saw 110 # ~fx
"#;
    let output_unfiltered = render_code(code_unfiltered, 4410);
    let output_filtered = render_code(code_filtered, 4410);

    let rms_unfiltered: f32 = (output_unfiltered.iter().map(|s| s * s).sum::<f32>() / output_unfiltered.len() as f32).sqrt();
    let rms_filtered: f32 = (output_filtered.iter().map(|s| s * s).sum::<f32>() / output_filtered.len() as f32).sqrt();

    assert!(
        rms_filtered < rms_unfiltered,
        "Filtered signal RMS ({:.4}) should be less than unfiltered ({:.4})",
        rms_filtered,
        rms_unfiltered
    );
}

#[test]
fn test_constant_pattern_sample_by_sample_equivalence() {
    // sine "440" and sine 440 should produce identical samples
    let code1 = r#"out $ sine 440"#;
    let code2 = r#"out $ sine "440""#;

    let output1 = render_code(code1, 1000);
    let output2 = render_code(code2, 1000);

    // Compare sample by sample
    let mut max_diff = 0.0f32;
    for (s1, s2) in output1.iter().zip(output2.iter()) {
        max_diff = max_diff.max((s1 - s2).abs());
    }

    assert!(
        max_diff < 0.01,
        "Constant pattern should produce same samples, max diff={:.6}",
        max_diff
    );
}

#[test]
fn test_pattern_frequency_changes_pitch() {
    // Compare constant frequency vs pattern-modulated
    // 220Hz should have half the zero crossings of 440Hz
    let code_220 = r#"out $ sine 220"#;
    let code_440 = r#"out $ sine 440"#;

    let output_220 = render_code(code_220, 22050); // 0.5 seconds
    let output_440 = render_code(code_440, 22050);

    fn count_zero_crossings(samples: &[f32]) -> usize {
        samples.windows(2)
            .filter(|w| (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0))
            .count()
    }

    let crossings_220 = count_zero_crossings(&output_220);
    let crossings_440 = count_zero_crossings(&output_440);

    // 440Hz should have ~2x the zero crossings of 220Hz
    let ratio = crossings_440 as f32 / crossings_220 as f32;
    assert!(
        (ratio - 2.0).abs() < 0.2,
        "440Hz should have ~2x crossings of 220Hz, got ratio={:.2} ({} vs {})",
        ratio,
        crossings_440,
        crossings_220
    );
}

#[test]
fn test_pattern_frequency_modulation() {
    // Test that pattern-controlled frequency produces varying signal
    // This is a weaker test - just verify it compiles and produces audio
    let code = r#"
tempo: 2.0
out $ sine "220 440"
"#;
    let output = render_code(code, 44100);

    // Should produce audio
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(rms > 0.3, "Pattern-modulated sine should produce audio, RMS={}", rms);

    // The overall zero crossing count should be between 220 and 440 Hz
    fn count_zero_crossings(samples: &[f32]) -> usize {
        samples.windows(2)
            .filter(|w| (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0))
            .count()
    }

    let crossings = count_zero_crossings(&output);
    // Pattern modulation produces audio - crossings should be in audible range
    // The exact count depends on how the pattern evaluates (may average, interpolate, etc.)
    assert!(
        crossings >= 100 && crossings <= 1000,
        "Pattern-modulated frequency should produce audible signal (100-1000 crossings), got {}",
        crossings
    );
}
