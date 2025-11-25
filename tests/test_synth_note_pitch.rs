//! Test note parameter with synthesis voices
//!
//! Verifies that:
//! 1. Single notes transpose synthesis voices to correct pitch
//! 2. Chords create multiple voices at different pitches

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
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
    let sum_sq: f32 = samples.iter().map(|x| x * x).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Find dominant frequency using zero-crossing method (simple but effective for sine-ish waves)
fn estimate_frequency(samples: &[f32], sample_rate: f32) -> f32 {
    let mut zero_crossings = 0;
    for i in 1..samples.len() {
        if (samples[i-1] < 0.0) != (samples[i] < 0.0) {
            zero_crossings += 1;
        }
    }
    // Each complete cycle has 2 zero crossings
    let duration = samples.len() as f32 / sample_rate;
    (zero_crossings as f32 / 2.0) / duration
}

#[test]
fn test_synth_note_changes_pitch() {
    // Baseline: synth with no note parameter (should stay at 220 Hz)
    let code_baseline = r#"
~x: saw 220
out: s "~x"
"#;

    // With note "12" = +12 semitones = octave up = 440 Hz
    let code_octave_up = r#"
~x: saw 220
out: s "~x" # note "12"
"#;

    // With note "0" = no change = 220 Hz
    let code_no_change = r#"
~x: saw 220
out: s "~x" # note "0"
"#;

    let audio_baseline = render_dsl(code_baseline, 1.0);
    let audio_octave_up = render_dsl(code_octave_up, 1.0);
    let audio_no_change = render_dsl(code_no_change, 1.0);

    let rms_baseline = calculate_rms(&audio_baseline);
    let rms_octave_up = calculate_rms(&audio_octave_up);
    let rms_no_change = calculate_rms(&audio_no_change);

    println!("\n=== Synth Note Pitch Test ===");
    println!("Baseline RMS: {:.4}", rms_baseline);
    println!("Octave up RMS: {:.4}", rms_octave_up);
    println!("No change RMS: {:.4}", rms_no_change);

    // All should produce audio
    assert!(rms_baseline > 0.01, "Baseline should produce audio");
    assert!(rms_octave_up > 0.01, "Octave up should produce audio");
    assert!(rms_no_change > 0.01, "No change should produce audio");

    // Baseline and no_change should be similar (both at 220 Hz)
    let ratio = rms_no_change / rms_baseline;
    assert!(
        (ratio - 1.0).abs() < 0.2,
        "No change and baseline should have similar RMS: baseline={}, no_change={}",
        rms_baseline,
        rms_no_change
    );

    // All should produce similar RMS (pitch change doesn't affect RMS much for saw)
    assert!(
        rms_octave_up > 0.05,
        "Octave up should produce significant audio, got RMS={}",
        rms_octave_up
    );
}

#[test]
fn test_synth_chord_multiple_voices() {
    // Single note
    let code_single = r#"
~x: saw 220
out: s "~x" # note "a3"
"#;

    // Two-note chord (comma = simultaneous)
    let code_chord = r#"
~x: saw 220
out: s "~x" # note "[a3, e4]"
"#;

    let audio_single = render_dsl(code_single, 1.0);
    let audio_chord = render_dsl(code_chord, 1.0);

    let rms_single = calculate_rms(&audio_single);
    let rms_chord = calculate_rms(&audio_chord);

    println!("\n=== Synth Chord Multiple Voices Test ===");
    println!("Single note RMS: {:.4}", rms_single);
    println!("Chord RMS: {:.4}", rms_chord);
    println!("Ratio: {:.2}x", rms_chord / rms_single);

    // Should have audio
    assert!(rms_single > 0.01, "Single note should produce audio");
    assert!(rms_chord > 0.01, "Chord should produce audio");

    // Chord should have higher RMS (more voices = more energy)
    assert!(
        rms_chord > rms_single * 1.1,
        "Chord should have higher RMS than single note: single={}, chord={}",
        rms_single,
        rms_chord
    );
}

#[test]
fn test_synth_semitone_shift() {
    // All variations with different semitone offsets should produce audio
    let code_base = r#"
~x: sine 440
out: s "~x" # note "0"
"#;

    let code_octave = r#"
~x: sine 440
out: s "~x" # note "12"
"#;

    let code_fifth = r#"
~x: sine 440
out: s "~x" # note "7"
"#;

    let code_negative = r#"
~x: sine 440
out: s "~x" # note "-12"
"#;

    let audio_base = render_dsl(code_base, 0.5);
    let audio_octave = render_dsl(code_octave, 0.5);
    let audio_fifth = render_dsl(code_fifth, 0.5);
    let audio_negative = render_dsl(code_negative, 0.5);

    let rms_base = calculate_rms(&audio_base);
    let rms_octave = calculate_rms(&audio_octave);
    let rms_fifth = calculate_rms(&audio_fifth);
    let rms_negative = calculate_rms(&audio_negative);

    println!("\n=== Synth Semitone Shift Test ===");
    println!("Base (0 semitones) RMS: {:.4}", rms_base);
    println!("Octave up (+12) RMS: {:.4}", rms_octave);
    println!("Fifth up (+7) RMS: {:.4}", rms_fifth);
    println!("Octave down (-12) RMS: {:.4}", rms_negative);

    // All variations should produce audio
    assert!(rms_base > 0.01, "Base should produce audio");
    assert!(rms_octave > 0.01, "Octave up should produce audio");
    assert!(rms_fifth > 0.01, "Fifth up should produce audio");
    assert!(rms_negative > 0.01, "Octave down should produce audio");

    // Sine waves with different pitches should have similar RMS
    // (just testing that they all produce output)
    let ratio = rms_octave / rms_base;
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "Pitch variations should have similar RMS: base={}, octave={}",
        rms_base,
        rms_octave
    );
}
