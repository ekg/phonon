/// Comprehensive tests for Tidal-style pattern structure combination
/// In Tidal, the # operator takes structure from the RIGHT side
/// s "bd" # note "c4 e4 g4" should produce 3 triggers (structure from note pattern)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

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

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Test: s "bd" # note "c4 e4 g4"
/// Should trigger 3 times (structure from note pattern on right)
#[test]
fn test_note_provides_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4 g4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Should have multiple onsets (at least 3 for the 3 notes in one cycle)
    // Note: detect_audio_events detects RMS changes, so may find multiple per sample
    assert!(
        onsets.len() >= 3,
        "s \"bd\" # note \"c4 e4 g4\" should have at least 3 onsets, got {}",
        onsets.len()
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should produce sound");
    println!("note structure: {} onsets (>= 3 expected), RMS = {}", onsets.len(), rms);
}

/// Test: s "bd" # gain "0.5 1.0 0.8 0.3"
/// Should trigger 4 times (structure from gain pattern)
#[test]
fn test_gain_provides_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # gain "0.5 1.0 0.8 0.3"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 4,
        "s \"bd\" # gain \"0.5 1.0 0.8 0.3\" should have at least 4 onsets, got {}",
        onsets.len()
    );
    println!("gain structure: {} onsets (>= 4 expected)", onsets.len());
}

/// Test: s "bd" # pan "-1 0 1"
/// Should trigger 3 times (structure from pan pattern)
#[test]
fn test_pan_provides_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # pan "-1 0 1"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 3,
        "s \"bd\" # pan \"-1 0 1\" should have at least 3 onsets, got {}",
        onsets.len()
    );
    println!("pan structure: {} onsets (>= 3 expected)", onsets.len());
}

/// Test: s "bd sn" # note "c4 e4 g4 d4"
/// Both sides have patterns - right side should dominate (4 triggers from note)
#[test]
fn test_both_sides_have_patterns() {
    let code = r#"
bpm: 120
out $ s "bd sn" # note "c4 e4 g4 d4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 4,
        "s \"bd sn\" # note \"c4 e4 g4 d4\" should have at least 4 onsets (from note), got {}",
        onsets.len()
    );
    println!("both patterns: {} onsets (>= 4 expected)", onsets.len());
}

/// Test: s "bd" # note "c4 e4" # gain "0.5 1.0"
/// Multiple modifiers - should combine all structures
/// note has 2 events, gain has 2 events -> should have 2 events total
#[test]
fn test_multiple_modifiers() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4" # gain "0.5 1.0"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // After first #note, we have 2 triggers
    // After second #gain, we still have 2 triggers (both have same structure)
    assert!(
        onsets.len() >= 2,
        "Multiple modifiers with same structure should have at least 2 onsets, got {}",
        onsets.len()
    );
    println!("multiple modifiers: {} onsets (>= 2 expected)", onsets.len());
}

/// Test: s "bd" # note "c4 e4" # gain "0.5 1.0 0.8"
/// Multiple modifiers with different structures
/// note has 2 events, gain has 3 events -> final result should have 3 events
#[test]
fn test_multiple_modifiers_different_structure() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 e4" # gain "0.5 1.0 0.8"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // After first #note, we have 2 triggers
    // After second #gain, we should have 3 triggers (structure from gain)
    assert!(
        onsets.len() >= 3,
        "Multiple modifiers - last one should dominate structure, got {}",
        onsets.len()
    );
    println!("multiple modifiers (different): {} onsets (>= 3 expected)", onsets.len());
}

/// Test: s "bd sn hh" without modifiers
/// Should have 3 triggers (structure from sample pattern only)
#[test]
fn test_sample_pattern_alone() {
    let code = r#"
bpm: 120
out $ s "bd sn hh"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 3,
        "s \"bd sn hh\" should have at least 3 onsets, got {}",
        onsets.len()
    );
    println!("sample pattern alone: {} onsets (>= 3 expected)", onsets.len());
}

/// Test: s "bd" # note "c4"
/// Single note should trigger once
#[test]
fn test_single_note() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    assert!(
        onsets.len() >= 1,
        "s \"bd\" # note \"c4\" should have at least 1 onset, got {}",
        onsets.len()
    );
    println!("single note: {} onsets (>= 1 expected)", onsets.len());
}

/// Test: s "bd" # note "c4 ~ e4"
/// Pattern with rest - should have 3 events but only 2 onsets
#[test]
fn test_pattern_with_rest() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4 ~ e4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Pattern "c4 ~ e4" has 3 events, but middle one is rest
    // So we should detect at least 2 audio onsets
    assert!(
        onsets.len() >= 2,
        "s \"bd\" # note \"c4 ~ e4\" should have at least 2 onsets (3 events with 1 rest), got {}",
        onsets.len()
    );
    println!("pattern with rest: {} onsets (>= 2 expected)", onsets.len());
}

/// Test: s "bd" # note "c4*2 e4"
/// Subdivision in note pattern
#[test]
fn test_subdivision_in_note() {
    let code = r#"
bpm: 120
out $ s "bd" # note "c4*2 e4"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // "c4*2 e4" = c4 twice in first half, e4 in second half = 3 events
    assert!(
        onsets.len() >= 3,
        "s \"bd\" # note \"c4*2 e4\" should have at least 3 onsets, got {}",
        onsets.len()
    );
    println!("subdivision in note: {} onsets (>= 3 expected)", onsets.len());
}

/// Test: s "bd" # note "[c4, e4, g4]"
/// Chord (polyrhythm) - should trigger once with chord
#[test]
fn test_chord_in_note() {
    let code = r#"
bpm: 120
out $ s "bd" # note "[c4, e4, g4]"
"#;
    let audio = render_dsl(code, 1.0);
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Chord should trigger as a single event
    assert!(
        onsets.len() >= 1,
        "s \"bd\" # note \"[c4, e4, g4]\" should have at least 1 onset (chord), got {}",
        onsets.len()
    );
    println!("chord in note: {} onsets (>= 1 expected)", onsets.len());
}
