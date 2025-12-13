//! Test modifier bus expansion
//!
//! Modifier buses use # syntax: ~name # speed "0.5 0.66"
//! When used in a chain, the stored expression is expanded:
//! s "bd" # ~name  ->  s "bd" # speed "0.5 0.66"

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::{parse_program, BusType, Statement};

/// Render DSL code using chunk-based processing
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;

    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

fn calculate_rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
}

#[test]
fn test_parse_modifier_bus() {
    let input = r#"~waver # speed "0.5 0.66"
out $ s "bd" # ~waver
"#;

    let result = parse_program(input);
    assert!(result.is_ok(), "Should parse modifier bus");

    let (_, statements) = result.unwrap();
    assert_eq!(statements.len(), 2);

    // Check first statement is a modifier bus
    if let Statement::BusAssignment {
        name,
        bus_type,
        expr: _,
        params: _,
    } = &statements[0]
    {
        assert_eq!(name, "waver");
        assert_eq!(*bus_type, BusType::Modifier);
    } else {
        panic!("Expected BusAssignment");
    }
}

#[test]
fn test_parse_signal_bus() {
    let input = r#"~drums $ s "bd sn"
out $ ~drums
"#;

    let result = parse_program(input);
    assert!(result.is_ok(), "Should parse signal bus");

    let (_, statements) = result.unwrap();
    assert_eq!(statements.len(), 2);

    // Check first statement is a signal bus
    if let Statement::BusAssignment {
        name,
        bus_type,
        expr: _,
        params: _,
    } = &statements[0]
    {
        assert_eq!(name, "drums");
        assert_eq!(*bus_type, BusType::Signal);
    } else {
        panic!("Expected BusAssignment");
    }
}

#[test]
fn test_compile_modifier_bus() {
    let input = r#"~waver # speed "0.5 0.66"
out $ s "bd(<3 5>,8)" # ~waver
"#;

    let (_, statements) = parse_program(input).expect("Parse should succeed");
    let result = compile_program(statements, 44100.0, None);

    assert!(
        result.is_ok(),
        "Should compile modifier bus: {}",
        result.err().unwrap_or_default()
    );
}

#[test]
fn test_compile_modifier_bus_with_filter() {
    // Modifier bus with filter effect
    let input = r#"~filt # lpf 800 0.7
out $ s "bd(<3 5>,8)" # ~filt
"#;

    let (_, statements) = parse_program(input).expect("Parse should succeed");
    let result = compile_program(statements, 44100.0, None);

    assert!(
        result.is_ok(),
        "Should compile filter modifier bus: {}",
        result.err().unwrap_or_default()
    );
}

#[test]
fn test_render_modifier_bus_produces_audio() {
    // Test that modifier bus expansion produces audio
    let input = r#"~waver # speed "0.5 1.0"
out $ s "bd*4" # ~waver
"#;

    let audio = render_dsl(input, 2.0);
    let rms = calculate_rms(&audio);

    println!("Modifier bus RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Modifier bus should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_modifier_bus_equivalent_to_inline() {
    // Compare modifier bus to inline modifier
    let modifier_bus_code = r#"~spd # speed 0.5
out $ s "bd*4" # ~spd
"#;

    let inline_code = r#"out $ s "bd*4" # speed 0.5
"#;

    let audio_modifier = render_dsl(modifier_bus_code, 2.0);
    let audio_inline = render_dsl(inline_code, 2.0);

    let rms_modifier = calculate_rms(&audio_modifier);
    let rms_inline = calculate_rms(&audio_inline);

    println!(
        "Modifier bus RMS: {:.4}, Inline RMS: {:.4}",
        rms_modifier, rms_inline
    );

    // Both should produce similar audio
    assert!(
        rms_modifier > 0.01,
        "Modifier bus should produce audio: {}",
        rms_modifier
    );
    assert!(
        rms_inline > 0.01,
        "Inline should produce audio: {}",
        rms_inline
    );

    // RMS should be similar (within 20%)
    let ratio = rms_modifier / rms_inline;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Modifier bus and inline should produce similar audio: modifier={}, inline={}, ratio={}",
        rms_modifier,
        rms_inline,
        ratio
    );
}

#[test]
fn test_multiple_modifier_buses() {
    // Test using multiple modifier buses
    let input = r#"~spd # speed 0.75
~filt # lpf 2000 0.5
out $ s "bd*4" # ~spd # ~filt
"#;

    let audio = render_dsl(input, 2.0);
    let rms = calculate_rms(&audio);

    println!("Multiple modifier buses RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Multiple modifier buses should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_modifier_bus_with_pattern_transform() {
    // Modifier bus with pattern transform (fast)
    let input = r#"~waver # speed ("0.5 0.66" $ fast 4)
out $ s "bd*4" # ~waver
"#;

    let audio = render_dsl(input, 2.0);
    let rms = calculate_rms(&audio);

    println!("Modifier bus with transform RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Modifier bus with transform should produce audio, got RMS: {}",
        rms
    );
}
