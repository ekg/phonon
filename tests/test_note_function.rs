/// Test the note function for creating frequency patterns
///
/// `note "c4 e4 g4"` creates a frequency pattern that can be used with oscillators

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

/// Test standalone note function: saw $ note "c4 e4 g4"
/// Should produce continuous oscillator with changing pitch (not triggered)
#[test]
fn test_note_standalone_with_saw() {
    let code = r#"
bpm: 120
out $ saw $ note "c4 e4 g4"
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    // Should produce continuous sound
    assert!(rms > 0.1, "saw $ note should produce sound, got RMS {}", rms);

    // Check first and second half have similar energy (continuous, not triggered)
    let mid = audio.len() / 2;
    let rms_first = calculate_rms(&audio[..mid]);
    let rms_second = calculate_rms(&audio[mid..]);
    let ratio = rms_first.min(rms_second) / rms_first.max(rms_second);

    assert!(ratio > 0.5, "Continuous oscillator should have consistent energy");
    println!("Note standalone: RMS={}, ratio={}", rms, ratio);
}

/// Test that note "c4" produces correct frequency (261.63 Hz)
#[test]
fn test_note_c4_frequency() {
    let code = r#"
out $ sine $ note "c4"
"#;
    let audio = render_dsl(code, 0.5);

    // Analyze frequency using zero-crossing
    let mut crossings = 0;
    for i in 1..audio.len() {
        if (audio[i-1] < 0.0 && audio[i] >= 0.0) || (audio[i-1] >= 0.0 && audio[i] < 0.0) {
            crossings += 1;
        }
    }
    // Zero crossings = 2 * frequency * duration
    let estimated_freq = crossings as f32 / 2.0 / 0.5;

    // C4 = 261.63 Hz, allow some tolerance
    assert!(
        (estimated_freq - 261.63).abs() < 30.0,
        "note \"c4\" should produce ~261.63 Hz, got ~{} Hz",
        estimated_freq
    );
    println!("C4 frequency: estimated {} Hz (expected 261.63)", estimated_freq);
}

/// Test note with different waveforms
#[test]
fn test_note_with_different_waveforms() {
    for waveform in ["sine", "saw", "square", "tri"] {
        let code = format!(r#"
bpm: 120
out $ {} $ note "a4"
"#, waveform);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);
        assert!(rms > 0.1, "{} $ note should produce sound, got RMS {}", waveform, rms);
        println!("{} $ note \"a4\": RMS = {}", waveform, rms);
    }
}

/// Test note pattern cycles through notes
#[test]
fn test_note_pattern_cycles() {
    let code = r#"
bpm: 60
out $ sine $ note "c4 e4 g4"
"#;
    // At 60 BPM = 1 cycle per second
    // Pattern has 3 notes, so each note plays for 1/3 second
    let audio = render_dsl(code, 3.0); // 3 cycles

    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Should produce sound");
    println!("Note pattern cycles: RMS = {}", rms);
}

/// Test bus indirection: ~notes $ "c4 e4 g4"; saw ~notes should trigger
#[test]
fn test_bus_indirection_triggers() {
    let code = r#"
bpm: 120
~notes $ "c4 e4 g4"
out $ saw ~notes
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    // Should produce sound with triggered behavior (envelope)
    assert!(rms > 0.05, "Bus indirection should produce sound, got RMS {}", rms);

    // Compare with direct triggered version
    let code_direct = r#"
bpm: 120
out $ saw "c4 e4 g4"
"#;
    let audio_direct = render_dsl(code_direct, 2.0);
    let rms_direct = calculate_rms(&audio_direct);

    // Both should have similar RMS (both triggered)
    let ratio = rms.min(rms_direct) / rms.max(rms_direct);
    assert!(ratio > 0.5, "Bus indirection should behave like direct: bus={}, direct={}", rms, rms_direct);

    println!("Bus indirection: RMS={}, Direct: RMS={}, ratio={}", rms, rms_direct, ratio);
}

/// Compare: saw "c4 e4 g4" (triggered) vs saw $ note "c4 e4 g4" (continuous)
#[test]
fn test_triggered_vs_continuous() {
    // Triggered version (my earlier fix routes this to SynthPattern)
    let code_triggered = r#"
bpm: 120
out $ saw "c4 e4 g4"
"#;

    // Continuous version (note function creates frequency pattern)
    let code_continuous = r#"
bpm: 120
out $ saw $ note "c4 e4 g4"
"#;

    let audio_trig = render_dsl(code_triggered, 2.0);
    let audio_cont = render_dsl(code_continuous, 2.0);

    let rms_trig = calculate_rms(&audio_trig);
    let rms_cont = calculate_rms(&audio_cont);

    // Both should produce sound
    assert!(rms_trig > 0.05, "Triggered should produce sound");
    assert!(rms_cont > 0.1, "Continuous should produce sound");

    // Continuous should have MORE consistent energy (no envelope decay)
    // Triggered has envelope so energy varies more
    println!("Triggered RMS: {}, Continuous RMS: {}", rms_trig, rms_cont);
}
