//! Tests for template functionality (@name: expression)
//!
//! Templates allow defining reusable transforms and effect chains once and applying them multiple times.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

#[test]
fn test_template_simple_constant() {
    // Define a template that's just a constant, use it in output
    let code = r#"
        @gain: 0.5
        out $ sine 440 * @gain
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let graph = compile_program(statements, 44100.0, None);

    // Should compile successfully without errors
    assert!(graph.is_ok());
}

#[test]
fn test_template_transform() {
    // Define a transform template and apply it
    let code = r#"
        tempo: 0.5
        @swing: swing 0.6
        out $ s "bd sn" $ @swing
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_effect_chain() {
    // Define an effect chain template and apply it
    let code = r#"
        @heavy: lpf 800 0.9 # distortion 0.4
        out $ s "bd" # @heavy
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_multiple_uses() {
    // Define a template and use it multiple times
    let code = r#"
        @filt: lpf 1000 0.8
        ~bass $ saw 55 # @filt
        ~lead $ sine 440 # @filt
        out $ ~bass + ~lead
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    // Should compile successfully
    assert!(result.is_ok());
}

#[test]
fn test_template_undefined_error() {
    // Try to use an undefined template
    let code = r#"
        out $ sine 440 * @undefined
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    // Should fail with undefined template error
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.contains("Undefined template: @undefined"));
    }
}

#[test]
fn test_template_chained_transforms() {
    // Template with chained transforms
    let code = r#"
        @crazy: fast 2 $ rev
        out $ s "bd sn hh cp" $ @crazy
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    // Should compile successfully
    assert!(result.is_ok());
}

/// Render helper: compile + render DSL to a mono audio buffer.
fn render(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    graph.render((duration * sample_rate) as usize)
}

#[test]
fn test_template_chained_transforms_semantics() {
    // A chained-transform template must expand to the SAME result as writing
    // the transform chain inline (macro expansion is faithful), and must
    // actually alter the pattern versus the untransformed source.
    let tpl = render(
        r#"
        tempo: 1.0
        @crazy: fast 2 $ rev
        out $ s "bd sn hh cp" $ @crazy
        "#,
        2.0,
    );
    let inline = render(
        r#"
        tempo: 1.0
        out $ s "bd sn hh cp" $ fast 2 $ rev
        "#,
        2.0,
    );
    let plain = render(
        r#"
        tempo: 1.0
        out $ s "bd sn hh cp"
        "#,
        2.0,
    );

    assert_eq!(tpl.len(), inline.len());

    // Faithful expansion: template render == inline-chain render, sample-for-sample.
    let max_diff = tpl
        .iter()
        .zip(inline.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1e-6,
        "Template @crazy must render identically to inline 'fast 2 $ rev', max sample diff {}",
        max_diff
    );

    // If samples produced audio, the transform must have changed the output.
    let tpl_energy: f32 = tpl.iter().map(|s| s.abs()).sum();
    if tpl_energy > 0.0 {
        let diff_energy: f32 = tpl
            .iter()
            .zip(plain.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(
            diff_energy > 0.0,
            "Chained template transform should change the rendered output vs the plain pattern"
        );
    }
}

#[test]
fn test_template_in_bus() {
    // Use template in bus definition
    let code = r#"
        @verb: reverb 0.3 0.5
        ~wet $ s "cp" # @verb
        out $ ~wet
    "#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let result = compile_program(statements, 44100.0, None);

    // Should compile successfully
    assert!(result.is_ok());
}
