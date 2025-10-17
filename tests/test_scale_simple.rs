//! Simple test for scale quantization without FFT

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_scale_parse() {
    // Just test that scale() parses correctly
    let input = r#"
        out: scale("0 1 2", "major", "60")
    "#;

    let result = parse_dsl(input);
    assert!(result.is_ok(), "Scale parsing failed: {:?}", result.err());
}

#[test]
fn test_scale_compile() {
    // Test that scale() compiles without errors
    let input = r#"
        cps: 2.0
        out: scale("0 1 2 3", "major", "c4")
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render a bit to make sure it doesn't crash
    let buffer = graph.render(100);
    assert_eq!(buffer.len(), 100);
}

#[test]
fn test_scale_with_sine() {
    // Test scale() feeding into sine()
    let input = r#"
        cps: 2.0
        out: sine(scale("0 2 4", "major", "60")) * 0.3
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    // Check that audio was produced
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("RMS: {}", rms);

    assert!(rms > 0.05, "Should produce audio, got RMS={}", rms);
}

#[test]
fn test_scale_direct_output() {
    // Test scale() output directly (should output frequencies)
    let input = "cps: 1.0\nout: scale(\"0\", \"major\", \"60\")";

    let (_, statements) = parse_dsl(input).unwrap();
    println!("Parsed statements: {:#?}", statements);

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Get first few samples to debug
    for i in 0..5 {
        let sample = graph.process_sample();
        println!("Sample {}: {}", i, sample);
    }

    // Get sample after a few cycles
    let sample = graph.process_sample();
    println!("Sample after warmup: {}", sample);

    // Scale degree 0 in C major from MIDI 60 should give C4 = 261.63 Hz
    assert!(
        (sample - 261.63).abs() < 1.0,
        "Expected ~261.63, got {}",
        sample
    );
}

#[test]
fn test_scale_changes() {
    // Test that scale values change with the pattern
    // Using <> alternation to get one value per cycle
    let input = r#"
        cps: 4.0
        out: scale("<0 1 2 3>", "major", "60")
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 4.0) as usize;

    // Render all audio first
    let buffer = graph.render(samples_per_cycle * 4);

    // Sample from the middle of each cycle
    let mut values = Vec::new();
    for cycle in 0..4 {
        let sample_idx = cycle * samples_per_cycle + samples_per_cycle / 2;
        let value = buffer[sample_idx];
        values.push(value);
        println!("Cycle {} value: {}", cycle, value);
    }

    // Expected frequencies: C, D, E, F = 261.63, 293.66, 329.63, 349.23 Hz
    let expected = [261.63, 293.66, 329.63, 349.23];

    for (i, (actual, expected)) in values.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - expected).abs() < 5.0,
            "Cycle {}: Expected {}, got {}",
            i,
            expected,
            actual
        );
    }
}
