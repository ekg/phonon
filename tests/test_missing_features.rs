/// Tests to verify what features are actually missing
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
#[ignore] // This SHOULD work but let's verify
fn test_pattern_freq_on_supersaw() {
    let input = r#"out $ supersaw("110 220 330", 0.5, 5) * 0.2"#;

    let result = parse_dsl(input);
    println!("Parse result: {:?}", result);

    if let Ok((_, statements)) = result {
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render audio
        let buffer = graph.render(44100);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        println!("Pattern freq supersaw RMS: {}", rms);
        assert!(rms > 0.01, "Should produce audio with pattern freq");
    } else {
        panic!("Failed to parse");
    }
}

#[test]
#[ignore] // This probably DOESN'T work
fn test_pattern_detune_on_supersaw() {
    let input = r#"out $ supersaw(110, "0.3 0.5 0.7", 5) * 0.2"#;

    let result = parse_dsl(input);

    if let Ok((_, statements)) = result {
        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        let buffer = graph.render(44100);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        println!("Pattern detune supersaw RMS: {}", rms);
        // This will probably be 0 or fail
        assert!(rms > 0.01, "Should work but probably doesn't");
    }
}

#[test]
#[ignore] // This definitely doesn't exist
fn test_sample_pattern_from_language() {
    let input = r#"
        cps: 2.0
        out $ s "bd sn hh cp"
    "#;

    let result = parse_dsl(input);

    // This will fail to parse because s() doesn't exist
    assert!(result.is_err(), "s() function doesn't exist in parser");
}

#[test]
#[ignore] // Verify synths are continuous, not triggered
fn test_synth_is_continuous_not_triggered() {
    let input = "out $ superkick(60, 0.5, 0.3, 0.1) * 0.3";

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds
    let buffer = graph.render(88200);

    // Check first and second half
    let first_half_rms: f32 = (buffer[..44100].iter().map(|x| x * x).sum::<f32>() / 44100.0).sqrt();
    let second_half_rms: f32 =
        (buffer[44100..].iter().map(|x| x * x).sum::<f32>() / 44100.0).sqrt();

    println!(
        "First half RMS: {}, Second half RMS: {}",
        first_half_rms, second_half_rms
    );

    // Kick should decay - second half should be quieter
    // But since it's continuous without triggering, it just keeps going
    assert!(
        second_half_rms > 0.01,
        "Synth is continuous, not triggered (this is the problem!)"
    );
}
