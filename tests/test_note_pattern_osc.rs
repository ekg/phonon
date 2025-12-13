/// Tests for automatic note pattern detection in oscillators
///
/// When an oscillator receives a pattern string containing note names like "c4 e4 g4",
/// it should automatically trigger per-note (like saw_trig) rather than playing continuously.
///
/// Expected behavior:
/// - `saw "c4 e4 g4"` should trigger on each note (like saw_trig)
/// - `saw 220` should play continuous (unchanged)
/// - `saw ~midi` should use MIDI polyphony (unchanged)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Test that `saw "c4 e4 g4"` triggers individual notes (not continuous)
#[test]
fn test_saw_note_pattern_triggers_per_note() {
    // Using note names should trigger per note
    let code = r#"
bpm: 120
out $ saw "c4 e4 g4"
"#;
    let audio = render_dsl(code, 2.0);

    // Should detect discrete onsets for each note
    let onsets = detect_audio_events(&audio, 44100.0, 0.02);

    // At 120 BPM, 1 cycle = 0.5 seconds, so in 2 seconds we have 4 cycles
    // Each cycle has 3 notes, so we expect ~12 note events (maybe fewer due to detection)
    assert!(
        onsets.len() >= 3,
        "saw \"c4 e4 g4\" should trigger individual notes, got {} onsets (expected >= 3)",
        onsets.len()
    );

    println!("Detected {} note onsets for saw \"c4 e4 g4\"", onsets.len());
}

/// Test that note pattern triggering works the same as saw_trig
#[test]
fn test_saw_note_pattern_same_as_saw_trig() {
    // Both should produce similar onset patterns
    let code_trig = r#"
bpm: 120
out $ saw_trig "c4 e4 g4"
"#;
    let code_auto = r#"
bpm: 120
out $ saw "c4 e4 g4"
"#;

    let audio_trig = render_dsl(code_trig, 2.0);
    let audio_auto = render_dsl(code_auto, 2.0);

    let onsets_trig = detect_audio_events(&audio_trig, 44100.0, 0.02);
    let onsets_auto = detect_audio_events(&audio_auto, 44100.0, 0.02);

    // They should have similar number of onsets
    let diff = (onsets_trig.len() as i32 - onsets_auto.len() as i32).abs();
    assert!(
        diff <= 2,
        "saw \"c4 e4 g4\" should behave like saw_trig: trig={} onsets, auto={} onsets",
        onsets_trig.len(),
        onsets_auto.len()
    );

    println!(
        "Onset comparison: saw_trig={}, saw auto={}",
        onsets_trig.len(),
        onsets_auto.len()
    );
}

/// Test that numeric frequencies still work continuously (unchanged behavior)
#[test]
fn test_saw_numeric_freq_still_continuous() {
    let code = r#"
out $ saw 440
"#;
    let audio = render_dsl(code, 0.5);

    // Continuous oscillator should have sustained energy
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "saw 440 should produce continuous sound with RMS > 0.1, got {}",
        rms
    );

    // For continuous oscillator, energy should be consistent throughout
    // Compare RMS of first half vs second half
    let mid = audio.len() / 2;
    let rms_first = calculate_rms(&audio[..mid]);
    let rms_second = calculate_rms(&audio[mid..]);

    // Both halves should have similar energy (within 50%)
    let ratio = if rms_first > rms_second {
        rms_second / rms_first
    } else {
        rms_first / rms_second
    };
    assert!(
        ratio > 0.5,
        "Continuous saw should have consistent energy: first={}, second={}, ratio={}",
        rms_first, rms_second, ratio
    );

    println!("Continuous saw 440: RMS={}, ratio={}", rms, ratio);
}

/// Test that sine with note pattern also triggers
#[test]
fn test_sine_note_pattern_triggers() {
    let code = r#"
bpm: 120
out $ sine "a4 c5 e5"
"#;
    let audio = render_dsl(code, 2.0);

    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    assert!(
        onsets.len() >= 3,
        "sine \"a4 c5 e5\" should trigger individual notes, got {} onsets",
        onsets.len()
    );

    println!("Detected {} note onsets for sine \"a4 c5 e5\"", onsets.len());
}

/// Test square wave with note pattern
#[test]
fn test_square_note_pattern_triggers() {
    let code = r#"
bpm: 120
out $ square "c3 g3 c4"
"#;
    let audio = render_dsl(code, 2.0);

    let onsets = detect_audio_events(&audio, 44100.0, 0.02);
    assert!(
        onsets.len() >= 3,
        "square \"c3 g3 c4\" should trigger individual notes, got {} onsets",
        onsets.len()
    );

    println!("Detected {} note onsets for square \"c3 g3 c4\"", onsets.len());
}

/// Test that chords work with note pattern (polyphonic stack)
#[test]
fn test_saw_chord_pattern() {
    let code = r#"
bpm: 120
out $ saw "[c4, e4, g4]"
"#;
    let audio = render_dsl(code, 1.0);

    // Chord should have sound
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "saw chord should produce sound, got RMS {}",
        rms
    );

    println!("Chord RMS: {}", rms);
}

/// Test that sample patterns (bd, sn) don't accidentally trigger note detection
#[test]
fn test_sample_names_not_detected_as_notes() {
    // "bd" should NOT be detected as note b followed by d
    assert!(
        !is_sample_name_incorrectly_detected("bd"),
        "bd should not trigger note detection"
    );
    // "bd sn hh" should not trigger note detection
    assert!(
        !is_sample_name_incorrectly_detected("bd sn hh"),
        "bd sn hh should not trigger note detection"
    );
    // "808bd" should not trigger
    assert!(
        !is_sample_name_incorrectly_detected("808bd"),
        "808bd should not trigger note detection"
    );
    // But "c4" should
    assert!(
        is_sample_name_incorrectly_detected("c4"),
        "c4 should trigger note detection"
    );
    // And "c4 e4 g4" should
    assert!(
        is_sample_name_incorrectly_detected("c4 e4 g4"),
        "c4 e4 g4 should trigger note detection"
    );
    // "f#3 Bb4" should
    assert!(
        is_sample_name_incorrectly_detected("f#3 Bb4"),
        "f#3 Bb4 should trigger note detection"
    );
}

// Helper to test the note detection logic (mirrors the actual implementation)
fn is_sample_name_incorrectly_detected(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i].to_ascii_lowercase();
        if c >= 'a' && c <= 'g' {
            let mut next_idx = i + 1;
            if next_idx < len && (chars[next_idx] == '#' || chars[next_idx] == 'b') {
                if chars[next_idx] == 'b' {
                    if next_idx + 1 < len && chars[next_idx + 1].is_ascii_digit() {
                        next_idx += 1;
                    }
                } else {
                    next_idx += 1;
                }
            }
            if next_idx < len && chars[next_idx].is_ascii_digit() {
                let has_boundary_before = i == 0 || !chars[i - 1].is_ascii_alphanumeric();
                let note_end = next_idx + 1;
                let has_boundary_after = note_end >= len || !chars[note_end].is_ascii_alphanumeric();
                if has_boundary_before && has_boundary_after {
                    return true;
                }
            }
        }
    }
    false
}
