/// Test bus triggering from mini-notation patterns
///
/// This feature allows referencing continuous synth signals like samples: s "~kick sn hh"
///
/// Bus triggering works by:
/// 1. Defining a continuous synthesis bus: ~kick $ sine 60
/// 2. Referencing it in a sample pattern: ~triggered $ s "~kick*4"
/// 3. Output the triggered bus: out $ ~triggered
///
/// This creates envelope-gated synthesis voices that play the bus content at pattern times.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

fn render_in_chunks(
    graph: &mut phonon::unified_graph::UnifiedSignalGraph,
    total_samples: usize,
    chunk_size: usize,
) -> Vec<f32> {
    let mut result = Vec::with_capacity(total_samples);
    let num_chunks = total_samples / chunk_size;
    for _ in 0..num_chunks {
        let chunk = graph.render(chunk_size);
        result.extend_from_slice(&chunk);
    }
    result
}

#[test]
fn test_bus_trigger_simple() {
    // Test that we can trigger a bus from mini-notation
    // ~kick is a continuous sine wave, s "~kick" should gate it on/off
    let sample_rate = 44100.0;
    let code = r#"
bpm: 120
~kick $ sine 60
~triggered $ s "~kick*4"
out $ ~triggered
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Bus trigger simple: RMS={:.6}, Peak={:.6}", rms, peak);
    assert!(
        rms > 0.01,
        "Bus trigger should produce audio, RMS = {}",
        rms
    );
    assert!(peak > 0.1, "Bus trigger should have peaks, Peak = {}", peak);
}

#[test]
fn test_bus_trigger_pattern() {
    // Test multiple bus triggers in a pattern
    let sample_rate = 44100.0;
    let code = r#"
bpm: 120
~kick $ sine 60
~snare $ sine 200
~triggered $ s "~kick ~snare ~kick ~snare"
out $ ~triggered
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    println!("Bus trigger pattern: RMS={:.6}", rms);
    assert!(
        rms > 0.01,
        "Bus trigger pattern should produce audio, RMS = {}",
        rms
    );
}

#[test]
fn test_bus_trigger_mixed_with_samples() {
    // Test mixing bus triggers with regular samples
    let sample_rate = 44100.0;
    let code = r#"
bpm: 120
~bass $ sine 55
~triggered $ s "~bass bd ~bass sn"
out $ ~triggered
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    println!("Bus trigger mixed: RMS={:.6}", rms);
    assert!(
        rms > 0.01,
        "Bus trigger mixed should produce audio, RMS = {}",
        rms
    );
}

#[test]
fn test_bus_trigger_with_fast_subdivision() {
    // Test bus triggering with fast subdivision
    let sample_rate = 44100.0;
    let code = r#"
bpm: 120
~hat $ sine 8000
~triggered $ s "~hat*8"
out $ ~triggered
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    println!("Bus trigger fast subdivision: RMS={:.6}", rms);
    assert!(
        rms > 0.01,
        "Bus trigger fast should produce audio, RMS = {}",
        rms
    );
}

#[test]
fn test_nonexistent_bus_graceful_failure() {
    // Test that referencing a non-existent bus doesn't crash
    let sample_rate = 44100.0;
    let code = r#"
tempo: 1.0
~triggered $ s "~nonexistent bd"
out $ ~triggered
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    println!("Nonexistent bus + bd: RMS={:.6}", rms);
    // Should complete without crashing and produce some audio from bd sample
    assert!(
        rms > 0.001,
        "Should produce some audio from bd sample, RMS = {}",
        rms
    );
}
