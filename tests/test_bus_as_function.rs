/// Tests for buses as functions (Phase 3)
///
/// Transformer buses: effect chains that can be applied via #
/// Function buses: parameterized buses with explicit parameters
/// Higher-order buses: buses that take other buses as parameters
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
// Transformer Bus Tests
// ============================================================================

#[test]
fn test_transformer_bus_parse() {
    // Effect chain defined as bus
    let code = r#"~fx $ lpf 1000 0.8"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

#[test]
fn test_transformer_bus_apply_to_signal() {
    // Apply effect bus to a signal
    let code = r#"
~fx $ lpf 1000 0.8
out $ saw 110 # ~fx
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_transformer_bus_chain() {
    // Multiple effects in transformer bus
    let code = r#"
~fx $ lpf 2000 0.6 # hpf 100 0.5
out $ saw 110 # ~fx
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_transformer_bus_reuse() {
    // Same effect bus applied to multiple sources
    let code = r#"
~fx $ lpf 1000 0.8
~osc1 $ saw 110 # ~fx
~osc2 $ sine 220 # ~fx
out $ ~osc1 ~+ ~osc2
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_transformer_produces_audio() {
    let code = r#"
~fx $ lpf 2000 0.8
out $ saw 110 # ~fx
"#;
    let output = render_code(code, 4410);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Transformer bus should produce output, got RMS={}",
        rms
    );
}

// ============================================================================
// Function Bus Tests (Parameters before $)
// ============================================================================

#[test]
fn test_function_bus_parse() {
    // Parameterized bus
    let code = r#"~mix a b $ a ~* 0.5 ~+ b ~* 0.5"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

#[test]
fn test_function_bus_call() {
    // Call parameterized bus
    let code = r#"
~mix a b $ a ~* 0.5 ~+ b ~* 0.5
~osc1 $ sine 440
~osc2 $ sine 441
out $ ~mix ~osc1 ~osc2
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_function_bus_single_param() {
    // Single parameter function bus
    let code = r#"
~double x $ x ~* 2
~osc $ sine 440
out $ ~double ~osc
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_function_bus_produces_audio() {
    let code = r#"
~amplify x $ x ~* 2
~osc $ sine 440
out $ ~amplify ~osc
"#;
    let output = render_code(code, 4410);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Function bus should produce output, got RMS={}",
        rms
    );
}

// ============================================================================
// Higher-Order Bus Tests
// ============================================================================

#[test]
fn test_higher_order_bus_parse() {
    // Bus that takes another bus as parameter
    let code = r#"~doubled f $ f ~+ f"#;
    match parse_program(code) {
        Ok((rest, _)) => {
            assert!(rest.trim().is_empty(), "Parser should consume all input");
        }
        Err(e) => panic!("Should parse: {}", e),
    }
}

#[test]
fn test_higher_order_bus_apply() {
    // Apply higher-order bus
    let code = r#"
~doubled f $ f ~+ f
~osc $ sine 440
~fat $ ~doubled ~osc
out $ ~fat
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_higher_order_with_transform() {
    // Higher-order bus that applies a transform
    let code = r#"
~withReverb f $ f # reverb 0.5 0.5
~dry $ sine 440
~wet $ ~withReverb ~dry
out $ ~wet
"#;
    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_higher_order_produces_audio() {
    let code = r#"
~louder f $ f ~* 2
~osc $ sine 440
~loud $ ~louder ~osc
out $ ~loud
"#;
    let output = render_code(code, 4410);
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Higher-order bus should produce output, got RMS={}",
        rms
    );
}
