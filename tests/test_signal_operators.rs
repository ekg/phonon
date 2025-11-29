/// Tests for signal operators (~+, ~-, ~*, ~/)
///
/// Signal operators perform sample-by-sample audio-rate arithmetic.
/// They use the ~ prefix to distinguish from pattern operators.
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
// Parser Tests - Infix Signal Operators
// ============================================================================

#[test]
fn test_parse_signal_add() {
    let code = r#"out $ sine 440 ~+ sine 441"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

#[test]
fn test_parse_signal_sub() {
    let code = r#"out $ sine 440 ~- sine 220"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

#[test]
fn test_parse_signal_mul() {
    let code = r#"out $ sine 440 ~* 0.5"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

#[test]
fn test_parse_signal_div() {
    let code = r#"out $ sine 440 ~/ 2"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

// ============================================================================
// Compilation Tests
// ============================================================================

#[test]
fn test_compile_signal_add_two_oscillators() {
    let code = r#"
~osc1 $ sine 440
~osc2 $ sine 441
out $ ~osc1 ~+ ~osc2
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_compile_signal_mul_by_constant() {
    let code = r#"out $ sine 440 ~* 0.5"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_compile_ring_modulation() {
    // AM/ring mod synthesis using ~*
    let code = r#"
~carrier $ sine 440
~modulator $ sine 5
out $ ~carrier ~* ~modulator
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_compile_complex_signal_chain() {
    let code = r#"
~osc1 $ sine 110
~osc2 $ saw 220
~lfo $ sine 0.5
out $ (~osc1 ~+ ~osc2) ~* (~lfo ~* 0.5 ~+ 0.5)
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

// ============================================================================
// Function Form Tests (~add, ~sub, ~mul, ~div)
// ============================================================================

#[test]
fn test_compile_signal_add_function() {
    let code = r#"out $ ~add (sine 440) (sine 441)"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_compile_signal_mul_function() {
    let code = r#"out $ ~mul (sine 440) 0.5"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

// ============================================================================
// Audio Verification Tests
// ============================================================================

#[test]
fn test_signal_add_produces_output() {
    let code = r#"out $ sine 440 ~+ sine 441"#;
    let output = render_code(code, 4410); // 0.1 second

    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Signal add should produce output, got RMS={}",
        rms
    );
}

#[test]
fn test_signal_mul_scales_amplitude() {
    // sine 440 should have RMS ~0.707
    // sine 440 ~* 0.5 should have RMS ~0.354
    let code_full = r#"out $ sine 440"#;
    let code_half = r#"out $ sine 440 ~* 0.5"#;

    let output_full = render_code(code_full, 4410);
    let output_half = render_code(code_half, 4410);

    let rms_full: f32 =
        (output_full.iter().map(|s| s * s).sum::<f32>() / output_full.len() as f32).sqrt();
    let rms_half: f32 =
        (output_half.iter().map(|s| s * s).sum::<f32>() / output_half.len() as f32).sqrt();

    let ratio = rms_half / rms_full;
    assert!(
        (ratio - 0.5).abs() < 0.1,
        "~* 0.5 should halve amplitude, got ratio={}",
        ratio
    );
}

// ============================================================================
// Reserved Name Tests
// ============================================================================

#[test]
fn test_reserved_name_add_rejected() {
    // Users should not be able to create a bus named ~add
    let code = r#"~add $ sine 440"#;
    match compile_code(code) {
        Ok(_) => panic!("Should reject reserved name ~add"),
        Err(e) => {
            assert!(
                e.contains("reserved") || e.contains("~add"),
                "Error should mention reserved name: {}",
                e
            );
        }
    }
}

#[test]
fn test_reserved_name_mul_rejected() {
    let code = r#"~mul $ sine 440"#;
    match compile_code(code) {
        Ok(_) => panic!("Should reject reserved name ~mul"),
        Err(e) => {
            assert!(
                e.contains("reserved") || e.contains("~mul"),
                "Error should mention reserved name: {}",
                e
            );
        }
    }
}
