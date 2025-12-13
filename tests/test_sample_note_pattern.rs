/// Test note modifier with samples: s "bd" # note "c4 e4 g4"

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() { return 0.0; }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Test: s "bd" # note "c4 e4 g4"
/// Sample gets pitched to different notes
#[test]
fn test_sample_with_note_pattern() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4 g4"
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Sample with note pattern should produce sound, got RMS {}", rms);
    println!("s \"bd\" # note \"c4 e4 g4\": RMS = {}", rms);
}

/// Test: s "bd sn hh" # note "c4 e4 g4"
/// Pattern alignment: each sample gets a note
#[test]
fn test_sample_pattern_with_note_pattern() {
    let code = r#"
bpm: 120
out $ s "bd sn hh" # note "c4 e4 g4"
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Sample pattern with notes should produce sound, got RMS {}", rms);
    println!("s \"bd sn hh\" # note \"c4 e4 g4\": RMS = {}", rms);
}

/// Test: s "bd" # note "0 7 12"
/// Semitone offsets (relative pitch)
#[test]
fn test_sample_with_semitone_offsets() {
    let code = r#"
bpm: 120
out $ s "bd" # note "0 7 12"
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Sample with semitone offsets should produce sound, got RMS {}", rms);
    println!("s \"bd\" # note \"0 7 12\": RMS = {}", rms);
}

/// Test both modes together
#[test]
fn test_note_with_oscillator_vs_sample() {
    // Oscillator with note pattern (continuous)
    let code_osc = r#"
bpm: 120
out $ saw $ note "c4 e4 g4"
"#;

    // Sample with note pattern (triggers sample at different pitches)
    let code_sample = r#"
bpm: 120
out $ s "bd" # note "c4 e4 g4"
"#;

    let audio_osc = render_dsl(code_osc, 2.0);
    let audio_sample = render_dsl(code_sample, 2.0);

    let rms_osc = calculate_rms(&audio_osc);
    let rms_sample = calculate_rms(&audio_sample);

    assert!(rms_osc > 0.1, "Oscillator should produce sound");
    assert!(rms_sample > 0.01, "Sample should produce sound");

    println!("Oscillator RMS: {}, Sample RMS: {}", rms_osc, rms_sample);
}
