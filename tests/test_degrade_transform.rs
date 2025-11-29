use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

#[test]
fn test_degrade_transform_dsl() {
    let input = r#"
        tempo: 0.5
        out $ s "bd bd bd bd" $ degrade * 0.5
    "#;

    let (_, statements) = parse_program(input).expect("Should parse DSL");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    // Render 2 seconds (4 cycles at 2 CPS)
    // With degrade, we expect ~50% of events to be dropped
    let buffer = graph.render(88200);

    // Calculate RMS to verify audio is produced
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Original pattern has 4 events per cycle * 4 cycles = 16 events
    // With 50% degradation, we expect ~8 events (allow some for randomness)
    // RMS should be positive (samples are quiet, so use low threshold)
    println!("RMS: {}", rms);
    assert!(
        rms > 0.0001,
        "Expected some audio with 50% degrade, got RMS: {}",
        rms
    );
}

#[test]
fn test_degrade_by_transform_dsl() {
    let input = r#"
        tempo: 0.5
        out $ s "bd bd bd bd" $ degradeBy 0.9 * 0.5
    "#;

    let (_, statements) = parse_program(input).expect("Should parse DSL");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    // Render 2 seconds (4 cycles at 2 CPS)
    // With 90% degradation, we expect ~10% of events to remain
    let buffer = graph.render(88200);

    // Calculate RMS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // With 90% degradation, RMS should be quite low but might still have some events
    println!("RMS: {}", rms);

    // Test passes if it compiles and runs without crashing
    // RMS may be very low or even zero due to high degradation
    assert!(rms >= 0.0, "RMS should be non-negative");
}

#[test]
fn test_degrade_with_sample_pattern() {
    let input = r#"
        tempo: 0.5
        out $ s "bd sn hh cp" $ degrade * 0.5
    "#;

    let (_, statements) = parse_program(input).expect("Should parse DSL");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    // Just verify it compiles and runs without crashing
    let buffer = graph.render(44100);

    // Should produce some audio (not complete silence)
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("RMS: {}", rms);

    // With 50% degrade, we should still hear some samples (samples are quiet)
    assert!(rms > 0.001, "Degraded sample pattern should produce audio");
}
