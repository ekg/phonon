//! Test note parameter with bus-referenced synth voices
//!
//! Reproduces the issue from e.ph where:
//! ~x: saw 220
//! o1: s "~x" # note "[a3 g3]"
//!
//! Expected: Should create polyphonic voices at a3 and g3 frequencies
//! Actual: User reports "voices don't stack or change"

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::{calculate_rms, find_dominant_frequency};

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

#[test]
fn test_note_transposes_bus_synth_single() {
    let code = r#"
~x: saw 220
out: s "~x" # note "a3"
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
~x: saw 220
out: s "~x" # note "[a3, g3]"
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
~x: saw 220
out: s "~x"
"#;

    let code_with_note = r#"
~x: saw 220
out: s "~x" # note "c4"
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
~x: saw 220
out: s "~x" # note "a3"
"#;

    let code_chord = r#"
~x: saw 220
out: s "~x" # note "[a3, e4]"
"#;

    let audio_single = render_dsl(code_single, 2.0);
    let audio_chord = render_dsl(code_chord, 2.0);

    let rms_single = calculate_rms(&audio_single);
    let rms_chord = calculate_rms(&audio_chord);

    println!("\n=== Chord Polyphony Test ===");
    println!("Single note RMS: {:.4}", rms_single);
    println!("Chord RMS: {:.4}", rms_chord);

    // Chord should have higher RMS due to multiple voices
    // Note: Due to phase relationships, two voices might not be exactly 2x louder
    assert!(
        rms_chord > rms_single * 1.1,
        "Chord should be louder than single note! Single: {}, Chord: {}",
        rms_single,
        rms_chord
    );
}
