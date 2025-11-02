//! Debug noise implementation
//!
//! **WARNING**: All tests in this file hang indefinitely due to fundsp noise() issue.
//! They are marked #[ignore] until the root cause is fixed.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

#[test]
#[ignore = "Hangs indefinitely - fundsp noise() issue"]
fn test_noise_direct_output() {
    // Simplest possible test - direct output
    let code = r#"
tempo: 2.0
out: noise
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 100 samples
    let buffer = graph.render(100);

    eprintln!("Direct noise - first 20 samples: {:?}", &buffer[0..20]);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("Direct noise RMS: {}", rms);

    assert!(rms > 0.01, "Direct noise should produce signal");
}

#[test]
#[ignore = "Hangs indefinitely - fundsp noise() issue"]
fn test_noise_in_bus() {
    // Test noise assigned to bus
    let code = r#"
tempo: 2.0
~n: noise
out: ~n
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 100 samples
    let buffer = graph.render(100);

    eprintln!("Bus noise - first 20 samples: {:?}", &buffer[0..20]);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("Bus noise RMS: {}", rms);

    assert!(rms > 0.01, "Bus noise should produce signal");
}

#[test]
#[ignore = "Hangs indefinitely - fundsp noise() issue"]
fn test_noise_through_lpf() {
    // Test noise through low-pass filter
    let code = r#"
tempo: 2.0
~n: noise
out: ~n # lpf 2000 0.8
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 100 samples
    let buffer = graph.render(100);

    eprintln!(
        "Filtered noise (lpf) - first 20 samples: {:?}",
        &buffer[0..20]
    );

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("Filtered noise (lpf) RMS: {}", rms);

    assert!(rms > 0.001, "Filtered noise should produce signal");
}
