//! Test note parameter with bus-referenced synth voices
//!
//! Reproduces the issue from e.ph where:
//! ~x $ saw 220
//! out $ s "~x" # note "[a3 g3]"
//!
//! Expected: Should create polyphonic voices at a3 and g3 frequencies
//! Actual: User reports "voices don't stack or change"

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::{band_energy, calculate_rms, find_dominant_frequency};

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

#[test]
fn test_note_transposes_bus_synth_single() {
    let code = r#"
~x $ saw 220
out $ s "~x" # note "a3"
"#;

    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    let freq = find_dominant_frequency(&audio, 44100.0);

    println!("\n=== Note Transpose Bus Synth (Single) ===");
    println!("RMS: {:.4}", rms);
    println!("Dominant frequency: {:.1} Hz", freq);
    println!("Expected: ~220 Hz (a3)");

    // Should produce audio
    assert!(rms > 0.01, "Should produce audio, got RMS: {}", rms);

    // Should be around 220 Hz (a3)
    assert!(
        (freq - 220.0).abs() < 20.0,
        "Expected ~220 Hz (a3), got {} Hz",
        freq
    );
}

#[test]
fn test_note_transposes_bus_synth_chord() {
    let code = r#"
~x $ saw 220
out $ s "~x" # note "[a3, g3]"
"#;
    // Note: Using comma [a3, g3] for simultaneous notes (chord)
    // Square brackets without comma [a3 g3] is subdivision (sequential)

    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    let freq = find_dominant_frequency(&audio, 44100.0);

    println!("\n=== Note Transpose Bus Synth (Chord) ===");
    println!("RMS: {:.4}", rms);
    println!("Dominant frequency: {:.1} Hz", freq);
    println!("Expected: Multiple frequencies (a3=220Hz, g3=196Hz)");

    // Should produce audio
    assert!(rms > 0.01, "Should produce audio, got RMS: {}", rms);

    // Should have higher RMS due to polyphonic voices
    // (This test may need adjustment based on expected behavior)
}

#[test]
fn test_note_vs_baseline_bus_synth() {
    // Baseline: bus synth without note parameter
    let code_baseline = r#"
~x $ saw 220
out $ s "~x"
"#;

    let code_with_note = r#"
~x $ saw 220
out $ s "~x" # note "c4"
"#;

    let audio_baseline = render_dsl(code_baseline, 2.0);
    let audio_with_note = render_dsl(code_with_note, 2.0);

    let freq_baseline = find_dominant_frequency(&audio_baseline, 44100.0);
    let freq_with_note = find_dominant_frequency(&audio_with_note, 44100.0);

    println!("\n=== Note Parameter Effect ===");
    println!("Baseline frequency: {:.1} Hz", freq_baseline);
    println!("With note c4: {:.1} Hz", freq_with_note);
    println!("Expected note c4: ~262 Hz");

    // Frequencies should be different
    assert!(
        (freq_baseline - freq_with_note).abs() > 10.0,
        "Note parameter should change frequency! Baseline: {}, With note: {}",
        freq_baseline,
        freq_with_note
    );

    // With note should be around 262 Hz (c4)
    assert!(
        (freq_with_note - 262.0).abs() < 20.0,
        "Expected ~262 Hz (c4), got {} Hz",
        freq_with_note
    );
}

// Note: Direct synth + note modifier (saw 220 # note "c4") is not supported.
// The note modifier only works with Sample nodes (s "pattern" # note "...").
// For direct synth pitch control, use: saw (note_to_freq "c4") or similar.

#[test]
fn test_chord_creates_polyphonic_voices() {
    // Test that comma chord notation creates multiple synthesis voices
    // Using comma [a3, e4] for simultaneous notes in the note pattern
    let code_single = r#"
~x $ saw 220
out $ s "~x" # note "a3"
"#;

    let code_chord = r#"
~x $ saw 220
out $ s "~x" # note "[a3, e4]"
"#;

    let audio_single = render_dsl(code_single, 2.0);
    let audio_chord = render_dsl(code_chord, 2.0);

    // Polyphony is verified by spectral content, NOT by RMS. The voice manager applies a
    // 1/sqrt(N) normalization to prevent clipping, which cancels the energy gain from
    // stacking voices -- a 2-note chord therefore has ~the same RMS as a single note.
    // The meaningful test of polyphony is that BOTH notes' frequencies are present in the
    // chord, while only the single note's frequency is present when played alone.
    // a3 = 220 Hz, e4 = 329.63 Hz.
    let a3 = 220.0;
    let e4 = 329.63;

    let single_a3 = band_energy(&audio_single, 44100.0, a3, 10.0);
    let single_e4 = band_energy(&audio_single, 44100.0, e4, 10.0);
    let chord_a3 = band_energy(&audio_chord, 44100.0, a3, 10.0);
    let chord_e4 = band_energy(&audio_chord, 44100.0, e4, 10.0);

    println!("\n=== Chord Polyphony Test ===");
    println!("Single: a3 band={:.1}, e4 band={:.1}", single_a3, single_e4);
    println!("Chord:  a3 band={:.1}, e4 band={:.1}", chord_a3, chord_e4);

    // Single note "a3" contains a3 but essentially no e4.
    assert!(single_a3 > 0.0, "Single note should contain a3 energy");
    assert!(
        single_e4 < single_a3 * 0.25,
        "Single note should NOT contain significant e4 energy: a3={}, e4={}",
        single_a3,
        single_e4
    );

    // Chord "[a3, e4]" must contain BOTH notes -> genuine polyphony.
    assert!(
        chord_a3 > single_a3 * 0.3,
        "Chord should retain the a3 voice: single_a3={}, chord_a3={}",
        single_a3,
        chord_a3
    );
    assert!(
        chord_e4 > chord_a3 * 0.4,
        "Chord should contain a strong e4 voice (polyphony): a3={}, e4={}",
        chord_a3,
        chord_e4
    );
}
