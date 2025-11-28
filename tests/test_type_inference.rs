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
// Audio Verification Tests
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
