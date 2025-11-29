/// Test bus-triggered synthesis with various patterns
/// Isolating the issue with Euclidean patterns + parameters
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
fn test_simple_bus_trigger() {
    let sample_rate = 44100.0;
    // Use bpm: 120 (2 CPS) so events trigger within 1 second
    let code = r#"
bpm: 120
~s $ sine 440
~c $ s "~s*4"
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds in 128-sample chunks

    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Simple bus trigger failed: RMS = {}", rms);
    println!("✓ Simple bus trigger: RMS = {:.6}", rms);
}

#[test]
fn test_bus_trigger_with_pattern() {
    let sample_rate = 44100.0;
    let code = r#"
tempo: 0.5
~s $ sine 440
~c $ s "~s ~s ~s ~s"
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 44100, 128);

    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bus with pattern failed: RMS = {}", rms);
    println!("✓ Bus trigger with pattern: RMS = {:.6}", rms);
}

#[test]
fn test_bus_trigger_with_gain() {
    let sample_rate = 44100.0;
    // Use bpm: 120 (2 CPS) so events trigger within render time
    let code = r#"
bpm: 120
~s $ sine 440
~c $ s "~s*4" # gain 1
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bus with gain failed: RMS = {}", rms);
    println!("✓ Bus trigger with gain: RMS = {:.6}", rms);
}

#[test]
fn test_bus_trigger_with_note() {
    let sample_rate = 44100.0;
    // Use bpm: 120 (2 CPS) so events trigger within render time
    let code = r#"
bpm: 120
~s $ sine 440
~c $ s "~s*4" # note "c3"
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 88200, 128); // 2 seconds

    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bus with note failed: RMS = {}", rms);
    println!("✓ Bus trigger with note: RMS = {:.6}", rms);
}

#[test]
fn test_bus_trigger_with_euclidean() {
    let sample_rate = 44100.0;
    let code = r#"
tempo: 0.5
~s $ sine 440
~c $ s "~s(3,8)"
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 44100, 128);

    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "Bus with Euclidean failed: RMS = {}", rms);
    println!("✓ Bus trigger with Euclidean: RMS = {:.6}", rms);
}

#[test]
fn test_bus_trigger_euclidean_plus_note_plus_gain() {
    let sample_rate = 44100.0;
    let code = r#"
tempo: 0.5
~s $ sine 440
~c $ s "~s(3,8)" # note "c3" # gain 1
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");
    let buffer = render_in_chunks(&mut graph, 44100, 128);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Bus with Euclidean+note+gain failed: RMS = {}",
        rms
    );
    println!("✓ Bus trigger with Euclidean+note+gain: RMS = {:.6}", rms);
}
