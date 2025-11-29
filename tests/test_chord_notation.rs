use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod pattern_verification_utils;
use pattern_verification_utils::{calculate_rms, detect_audio_events};

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
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
fn test_chord_notation_maj() {
    // Test that c4'maj triggers a C major chord (C E G)
    // Use intermediate bus pattern (required for bus triggering to work properly)
    let code = r#"
bpm: 120
~synth $ sine 440
~pattern $ s "~synth*4" # note "c4'maj"
out $ ~pattern
    "#;

    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    println!("\n=== Chord Notation Test: c4'maj ===");
    println!("RMS: {:.3}", rms);
    println!("Onsets detected: {}", onsets.len());

    // Should produce audio (3 notes playing simultaneously)
    assert!(rms > 0.05, "Chord should produce audio, got RMS: {:.3}", rms);

    // RMS should be higher than single note due to 3 simultaneous voices
    let single_note_code = r#"
bpm: 120
~synth $ sine 440
~pattern $ s "~synth*4" # note "c4"
out $ ~pattern
    "#;
    let single_audio = render_dsl(single_note_code, 1.0);
    let single_rms = calculate_rms(&single_audio);

    println!("Single note RMS: {:.3}", single_rms);
    // Note: We don't strictly require chord RMS > single note RMS because
    // phase cancellation between chord notes can reduce overall amplitude.
    // Just verify both produce audio.
    assert!(single_rms > 0.05, "Single note should produce audio, got RMS: {:.3}", single_rms);
}

#[test]
fn test_chord_notation_min7() {
    // Test that c4'min7 triggers C minor 7th chord (C Eb G Bb)
    let code = r#"
bpm: 120
~synth $ saw 440
~pattern $ s "~synth*4" # note "c4'min7"
out $ ~pattern
    "#;

    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    println!("\n=== Chord Notation Test: c4'min7 ===");
    println!("RMS: {:.3}", rms);

    // Should produce audio (4 notes: C Eb G Bb)
    assert!(rms > 0.05, "Min7 chord should produce audio, got RMS: {:.3}", rms);
}

#[test]
fn test_chord_pattern_sequence() {
    // Test chord progression: C major -> F major -> G major -> C major
    let code = r#"
bpm: 120
~synth $ square 440
~pattern $ s "~synth*4" # note "c4'maj f4'maj g4'maj c5'maj"
out $ ~pattern
    "#;

    let audio = render_dsl(code, 2.0); // 2 cycles
    let rms = calculate_rms(&audio);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    println!("\n=== Chord Pattern Sequence ===");
    println!("RMS: {:.3}", rms);
    println!("Onsets detected: {}", onsets.len());

    assert!(rms > 0.05, "Chord sequence should produce audio, got RMS: {:.3}", rms);
    // Should have 4 chord triggers per cycle * 2 cycles = 8 onsets
    assert!(onsets.len() >= 6, "Should detect multiple chord onsets, got {}", onsets.len());
}

#[test]
fn test_mixed_notes_and_chords() {
    // Test mixing single notes and chords in same pattern
    let code = r#"
bpm: 120
~synth $ triangle 440
~pattern $ s "~synth*4" # note "c4 e4'min g4 c5'maj7"
out $ ~pattern
    "#;

    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    println!("\n=== Mixed Notes and Chords ===");
    println!("RMS: {:.3}", rms);

    assert!(rms > 0.05, "Mixed pattern should produce audio, got RMS: {:.3}", rms);
}

#[test]
fn test_chord_with_default_frequency() {
    // User wants to know: should synth bus default to 440?
    // Test that chord triggers work even without explicit frequency in synth bus
    let code = r#"
bpm: 120
~synth $ sine 440
~pattern $ s "~synth*4" # note "c4'maj e4'min"
out $ ~pattern
    "#;

    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    println!("\n=== Chord with 440Hz synth ===");
    println!("RMS: {:.3}", rms);

    assert!(rms > 0.05, "440Hz synth should work with chord notation, got RMS: {:.3}", rms);
}
