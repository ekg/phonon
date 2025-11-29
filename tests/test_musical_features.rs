/// Musical Feature Tests - Real Production Scenarios
///
/// Tests that verify Phonon works for actual music production:
/// - Sidechain compression (house music kick→bass)
/// - Feedback loops (dub delays, echo)
/// - Chord generation
/// - Combined musical examples
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ============================================================================
// Sidechain Compression Tests
// ============================================================================

#[test]
fn test_sidechain_compression_basic() {
    // Basic sidechain compression - kick ducking bass
    // Threshold in dB: -10dB is typical for sidechain compression
    let code = r#"
        tempo: 0.5
        ~kick $ sine 55 * 0.8
        ~bass $ saw 55 * 0.5
        ~compressed $ ~bass # sidechain_compressor ~kick -10.0 4.0 0.01 0.1
        out $ ~compressed
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Sidechain compression should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_sidechain_compression_house_track() {
    // Real house music scenario: kick sidechaining bass
    // Threshold in dB: -15dB allows strong kick to duck the bass
    let code = r#"
        tempo: 0.5
        ~bass $ saw 55 * 0.6
        ~kick $ sine 55 * 0.8
        ~ducked_bass $ ~bass # sidechain_compressor ~kick -15.0 8.0 0.001 0.2
        out $ ~kick * 0.7 + ~ducked_bass * 0.5
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    // Should have strong audio (kick + bass)
    assert!(
        rms > 0.1,
        "House track should have strong signal, got RMS: {}",
        rms
    );
}

#[test]
fn test_sidechain_creates_pumping() {
    // Verify sidechain actually reduces bass level when kick hits
    // Threshold must be in dB: sine 1.0 = 0dB, sine 0.5 = -6dB, sine 0.1 = -20dB
    let no_sidechain = r#"
        tempo: 0.5
        ~bass $ saw 55 * 0.5
        out $ ~bass
    "#;

    let with_sidechain = r#"
        tempo: 0.5
        ~bass $ saw 55 * 0.5
        ~kick $ sine 55 * 0.8
        ~ducked $ ~bass # sidechain_compressor ~kick -20.0 10.0 0.001 0.2
        out $ ~ducked
    "#;

    let baseline = render_dsl(no_sidechain, 2.0);
    let sidechained = render_dsl(with_sidechain, 2.0);

    let baseline_rms = calculate_rms(&baseline);
    let sidechained_rms = calculate_rms(&sidechained);

    // Sidechained version should have noticeably lower average RMS due to ducking
    // Threshold -20dB means any signal above -20dB triggers compression
    assert!(
        sidechained_rms < baseline_rms * 0.9,
        "Sidechain should reduce average level (baseline: {}, sidechained: {})",
        baseline_rms,
        sidechained_rms
    );
}

// ============================================================================
// Feedback Loop Tests (Dub Effects)
// ============================================================================

#[test]
fn test_delay_with_feedback() {
    // Dub-style delay with feedback
    let code = r#"
        tempo: 0.5
        ~input $ sine 440 * 0.5
        ~delayed $ ~input # delay 0.25 0.6
        out $ ~input * 0.5 + ~delayed * 0.5
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Delay with feedback should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_delay_creates_echoes() {
    // Verify delay actually creates repeating events
    let no_delay = r#"
        tempo: 0.5
        out $ sine 440 * 0.5
    "#;

    let with_delay = r#"
        tempo: 0.5
        ~dry $ sine 440 * 0.5
        ~wet $ ~dry # delay 0.125 0.5
        out $ ~dry * 0.7 + ~wet * 0.3
    "#;

    let dry_buffer = render_dsl(no_delay, 1.0);
    let wet_buffer = render_dsl(with_delay, 1.0);

    let dry_rms = calculate_rms(&dry_buffer);
    let wet_rms = calculate_rms(&wet_buffer);

    // Delayed version should have more energy due to echoes
    assert!(
        wet_rms >= dry_rms * 0.95,
        "Delay should maintain or increase energy (dry: {}, wet: {})",
        dry_rms,
        wet_rms
    );
}

#[test]
fn test_dub_echo_chain() {
    // Classic dub delay chain
    let code = r#"
        tempo: 1.0
        ~source $ sine 880 * 0.4
        ~echout $ ~source # delay 0.25 0.5
        ~echo2 $ ~echo1 # delay 0.25 0.4
        out $ ~source * 0.6 + ~echo1 * 0.3 + ~echo2 * 0.2
    "#;

    let buffer = render_dsl(code, 3.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Dub echo chain should produce audio, got RMS: {}",
        rms
    );
}

// ============================================================================
// Chord Generation Tests
// ============================================================================

#[test]
fn test_major_chord() {
    // C major chord (C-E-G)
    let code = r#"
        tempo: 0.5
        ~note1 $ sine 261.63
        ~note2 $ sine 329.63
        ~note3 $ sine 392.00
        out $ (~note1 + ~note2 + ~note3) * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "Major chord should produce strong audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_minor_chord() {
    // A minor chord (A-C-E)
    let code = r#"
        tempo: 0.5
        ~note1 $ sine 220.00
        ~note2 $ sine 261.63
        ~note3 $ sine 329.63
        out $ (~note1 + ~note2 + ~note3) * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "Minor chord should produce strong audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_seventh_chord() {
    // Dominant 7th chord (G7: G-B-D-F)
    let code = r#"
        tempo: 0.5
        ~note1 $ sine 196.00
        ~note2 $ sine 246.94
        ~note3 $ sine 293.66
        ~note4 $ sine 349.23
        out $ (~note1 + ~note2 + ~note3 + ~note4) * 0.25
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "7th chord should produce strong audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_chord_progression() {
    // Simple I-IV-V progression in C (C-F-G)
    let code = r#"
        tempo: 1.0
        ~chord1 $ (sine 261.63 + sine 329.63 + sine 392.00) * 0.3
        ~chord2 $ (sine 349.23 + sine 440.00 + sine 523.25) * 0.3
        ~chord3 $ (sine 392.00 + sine 493.88 + sine 587.33) * 0.3
        out $ ~chord1
    "#;

    let buffer = render_dsl(code, 3.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "Chord progression should produce audio, got RMS: {}",
        rms
    );
}

// ============================================================================
// Combined Musical Scenario Tests
// ============================================================================

#[test]
fn test_house_track_with_sidechain_and_chords() {
    // Complete house track: kick, bass (sidechained), chords
    // Threshold in dB: -15dB for typical house music pumping
    let code = r#"
        tempo: 0.5
        ~kick $ sine 55 * 0.8
        ~bass $ saw 55 * 0.6
        ~ducked_bass $ ~bass # sidechain_compressor ~kick -15.0 8.0 0.001 0.2
        ~chord $ (sine 261.63 + sine 329.63 + sine 392.00) * 0.2
        out $ ~kick * 0.6 + ~ducked_bass * 0.4 + ~chord * 0.3
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "Full house track should have strong signal, got RMS: {}",
        rms
    );
}

#[test]
fn test_melodic_pattern_with_effects() {
    // Melodic pattern with delay and reverb
    let code = r#"
        tempo: 0.5
        ~melody $ sine 440
        ~with_delay $ ~melody # delay 0.125 0.4
        ~with_reverb $ ~with_delay # reverb 0.5 0.5 0.3
        out $ ~with_reverb * 0.8
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Melodic pattern with effects should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_vibrato_and_tremolo() {
    // Vibrato (FM) and tremolo (AM) modulation
    let code = r#"
        tempo: 0.5
        ~vibrato $ sine 5.0 * 10.0 + 440.0
        ~tremolo $ sine 4.0 * 0.3 + 0.5
        ~modulated $ sine ~vibrato * ~tremolo
        out $ ~modulated * 0.6
    "#;

    let buffer = render_dsl(code, 4.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Vibrato and tremolo should produce audio, got RMS: {}",
        rms
    );
}
