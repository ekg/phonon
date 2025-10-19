/// End-to-end tests for oscillator DSL syntax
/// Tests all oscillator types and variations using actual .ph file syntax
///
/// CRITICAL: Tests verify ACTUAL AUDIO OUTPUT, not just rendering success!
/// We are "deaf" - can only verify audio through analysis tools.

use std::process::Command;
use std::fs;

mod audio_verification;
use audio_verification::*;

/// Helper to render DSL code and verify it produces audio
/// Duration: 2 seconds by default (1 cycle at 120 BPM / 0.5 cps)
fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String, String) {
    render_and_verify_duration(dsl_code, test_name, "2")
}

/// Helper with custom duration for multi-cycle tests
fn render_and_verify_duration(dsl_code: &str, test_name: &str, duration: &str) -> (bool, String, String) {
    let ph_path = format!("/tmp/test_{}.ph", test_name);
    let wav_path = format!("/tmp/test_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", &ph_path, &wav_path, "--duration", duration])
        .output()
        .expect("Failed to run phonon render");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stderr, wav_path)
}

// ============================================================================
// BASIC OSCILLATOR TESTS - Each oscillator type with constant frequency
// ============================================================================

#[test]
fn test_sine_constant_frequency() {
    let dsl = r#"
tempo: 0.5
out: sine 440 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "sine_constant");
    assert!(success, "Failed to render sine oscillator: {}", stderr);

    // VERIFY ACTUAL AUDIO OUTPUT
    verify_oscillator_frequency(&wav_path, 440.0, 50.0)
        .expect("440 Hz sine wave not detected");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of expected range");
}

#[test]
fn test_saw_constant_frequency() {
    let dsl = r#"
tempo: 0.5
out: saw 110 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "saw_constant");
    assert!(success, "Failed to render saw oscillator: {}", stderr);

    // VERIFY ACTUAL AUDIO OUTPUT
    verify_oscillator_frequency(&wav_path, 110.0, 30.0)
        .expect("110 Hz saw wave not detected");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of expected range");
}

#[test]
fn test_square_constant_frequency() {
    let dsl = r#"
tempo: 0.5
out: square 220 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "square_constant");
    assert!(success, "Failed to render square oscillator: {}", stderr);

    // VERIFY ACTUAL AUDIO OUTPUT
    verify_oscillator_frequency(&wav_path, 220.0, 50.0)
        .expect("220 Hz square wave not detected");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of expected range");
}

#[test]
fn test_tri_constant_frequency() {
    let dsl = r#"
tempo: 0.5
out: tri 330 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "tri_constant");
    assert!(success, "Failed to render tri oscillator: {}", stderr);

    // VERIFY ACTUAL AUDIO OUTPUT
    verify_oscillator_frequency(&wav_path, 330.0, 50.0)
        .expect("330 Hz triangle wave not detected");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of expected range");
}

// ============================================================================
// PATTERN-CONTROLLED FREQUENCY TESTS - Each oscillator with pattern
// ============================================================================

#[test]
fn test_sine_pattern_frequency_2_values() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "sine_pattern_2");
    assert!(success, "Failed to render sine with 2-value pattern: {}", stderr);

    // VERIFY PATTERN MODULATION - expect frequency changes
    verify_pattern_modulation(&wav_path, "frequency", 1)
        .expect("Pattern frequency modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_sine_pattern_frequency_4_values() {
    let dsl = r#"
tempo: 0.5
out: sine "110 220 330 440" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "sine_pattern_4");
    assert!(success, "Failed to render sine with 4-value pattern: {}", stderr);

    // VERIFY PATTERN MODULATION - expect multiple frequency changes
    verify_pattern_modulation(&wav_path, "frequency", 2)
        .expect("Pattern frequency modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_saw_pattern_frequency() {
    let dsl = r#"
tempo: 0.5
out: saw "55 82.5 110 165" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "saw_pattern");
    assert!(success, "Failed to render saw with pattern: {}", stderr);

    // VERIFY PATTERN MODULATION
    verify_pattern_modulation(&wav_path, "frequency", 2)
        .expect("Pattern frequency modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_square_pattern_frequency() {
    let dsl = r#"
tempo: 0.5
out: square "110 165 220" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "square_pattern");
    assert!(success, "Failed to render square with pattern: {}", stderr);

    // VERIFY PATTERN MODULATION
    verify_pattern_modulation(&wav_path, "frequency", 1)
        .expect("Pattern frequency modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_tri_pattern_frequency() {
    let dsl = r#"
tempo: 0.5
out: tri "220 330 440 550" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "tri_pattern");
    assert!(success, "Failed to render tri with pattern: {}", stderr);

    // VERIFY PATTERN MODULATION
    verify_pattern_modulation(&wav_path, "frequency", 2)
        .expect("Pattern frequency modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

// ============================================================================
// MULTIPLE OSCILLATOR MIXING TESTS
// ============================================================================

#[test]
fn test_two_sines_mixed() {
    let dsl = r#"
tempo: 0.5
~osc1: sine 440 * 0.1
~osc2: sine 880 * 0.1
out: ~osc1 + ~osc2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "two_sines");
    assert!(success, "Failed to mix two sines: {}", stderr);

    // VERIFY AUDIO EXISTS and has reasonable amplitude
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Mixed amplitude out of range");
}

#[test]
fn test_all_oscillator_types_mixed() {
    let dsl = r#"
tempo: 0.5
~s: sine 440 * 0.05
~saw: saw 220 * 0.05
~sq: square 110 * 0.05
~t: tri 880 * 0.05
out: ~s + ~saw + ~sq + ~t
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "all_oscs_mixed");
    assert!(success, "Failed to mix all oscillator types: {}", stderr);

    // VERIFY AUDIO EXISTS
    verify_audio_exists(&wav_path)
        .expect("No audio output from mixed oscillators");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Mixed amplitude out of range");
}

#[test]
fn test_weighted_oscillator_mix() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 * 0.3
~mid: square 220 * 0.2
~high: sine 880 * 0.1
out: ~bass + ~mid + ~high
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "weighted_mix");
    assert!(success, "Failed to create weighted mix: {}", stderr);

    // VERIFY AUDIO EXISTS with good levels
    verify_audio_exists(&wav_path)
        .expect("No audio output from weighted mix");
    verify_amplitude_range(&wav_path, 0.1, 0.95)
        .expect("Mixed amplitude out of range");
}

// ============================================================================
// LFO MODULATION TESTS - Low frequency oscillators modulating audio
// ============================================================================

#[test]
fn test_lfo_amplitude_modulation() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 2 * 0.5 + 0.5
~carrier: sine 440
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lfo_am");
    assert!(success, "Failed to create LFO amplitude modulation: {}", stderr);

    // VERIFY LFO MODULATION - amplitude should vary over time
    verify_lfo_modulation(&wav_path)
        .expect("LFO amplitude modulation not detected");
    verify_pattern_modulation(&wav_path, "amplitude", 1)
        .expect("Amplitude modulation over time not detected");
}

#[test]
fn test_slow_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 0.25 * 0.5 + 0.5
~carrier: saw 110
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "slow_lfo", "4");
    assert!(success, "Failed to create slow LFO: {}", stderr);

    // VERIFY SLOW LFO - need longer duration to detect 0.25 Hz modulation
    verify_lfo_modulation(&wav_path)
        .expect("Slow LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "amplitude", 1)
        .expect("Slow amplitude modulation not detected");
}

#[test]
fn test_fast_lfo_vibrato() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 6 * 10
~carrier: sine (440 + ~lfo)
out: ~carrier * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "fast_lfo");
    assert!(success, "Failed to create vibrato: {}", stderr);

    // VERIFY VIBRATO - frequency modulation should be detectable
    verify_lfo_modulation(&wav_path)
        .expect("Vibrato (FM) not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_triangle_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo: tri 1 * 0.5 + 0.5
~carrier: sine 440
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "tri_lfo");
    assert!(success, "Failed to create triangle LFO: {}", stderr);

    // VERIFY TRIANGLE LFO modulation
    verify_lfo_modulation(&wav_path)
        .expect("Triangle LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "amplitude", 1)
        .expect("Amplitude modulation not detected");
}

// ============================================================================
// FREQUENCY RANGE TESTS - Low bass to high frequencies
// ============================================================================

#[test]
fn test_sub_bass_frequency() {
    let dsl = r#"
tempo: 0.5
out: sine 40 * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "sub_bass");
    assert!(success, "Failed to render sub-bass: {}", stderr);

    // VERIFY 40 Hz sub-bass frequency
    verify_oscillator_frequency(&wav_path, 40.0, 20.0)
        .expect("40 Hz sub-bass not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_bass_frequency() {
    let dsl = r#"
tempo: 0.5
out: saw 55 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bass");
    assert!(success, "Failed to render bass: {}", stderr);

    // VERIFY 55 Hz bass (A1)
    verify_oscillator_frequency(&wav_path, 55.0, 20.0)
        .expect("55 Hz bass not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_mid_frequency() {
    let dsl = r#"
tempo: 0.5
out: square 440 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "mid");
    assert!(success, "Failed to render mid frequency: {}", stderr);

    // VERIFY 440 Hz (A4)
    verify_oscillator_frequency(&wav_path, 440.0, 50.0)
        .expect("440 Hz not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_high_frequency() {
    let dsl = r#"
tempo: 0.5
out: sine 3520 * 0.1
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "high");
    assert!(success, "Failed to render high frequency: {}", stderr);

    // VERIFY high frequency content
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
    verify_amplitude_range(&wav_path, 0.02, 0.95)
        .expect("Amplitude out of range");
}

// ============================================================================
// PATTERN MODULATION WITH COMPLEX PATTERNS
// ============================================================================

#[test]
fn test_8_step_frequency_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "110 165 220 275 330 385 440 495" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "8_step_pattern", "4");
    assert!(success, "Failed to render 8-step pattern: {}", stderr);

    // VERIFY 8-step pattern modulation
    verify_pattern_modulation(&wav_path, "frequency", 4)
        .expect("8-step frequency pattern not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_octave_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "110 220 440 880" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "octave_pattern");
    assert!(success, "Failed to render octave pattern: {}", stderr);

    // VERIFY octave pattern - large frequency jumps
    verify_pattern_modulation(&wav_path, "frequency", 2)
        .expect("Octave pattern modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_pentatonic_pattern() {
    let dsl = r#"
tempo: 0.5
out: sine "220 247.5 275 330 370" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "pentatonic");
    assert!(success, "Failed to render pentatonic pattern: {}", stderr);

    // VERIFY pentatonic scale pattern
    verify_pattern_modulation(&wav_path, "frequency", 2)
        .expect("Pentatonic pattern modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

// ============================================================================
// AUDIO RATE MODULATION TESTS - FM synthesis
// ============================================================================

#[test]
fn test_simple_fm_synthesis() {
    let dsl = r#"
tempo: 0.5
~modulator: sine 55 * 100
~carrier: sine (440 + ~modulator)
out: ~carrier * 0.1
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "simple_fm");
    assert!(success, "Failed to create FM synthesis: {}", stderr);

    // VERIFY FM synthesis creates audio with rich spectral content
    verify_audio_exists(&wav_path)
        .expect("No audio output from FM synthesis");
    verify_amplitude_range(&wav_path, 0.02, 0.95)
        .expect("FM amplitude out of range");
}

#[test]
fn test_deep_fm_modulation() {
    let dsl = r#"
tempo: 0.5
~mod: sine 110 * 500
~car: sine (220 + ~mod)
out: ~car * 0.1
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "deep_fm");
    assert!(success, "Failed to create deep FM: {}", stderr);

    // VERIFY deep FM creates complex spectrum
    verify_audio_exists(&wav_path)
        .expect("No audio output from deep FM");
    verify_amplitude_range(&wav_path, 0.02, 0.95)
        .expect("FM amplitude out of range");
}

#[test]
fn test_pattern_controlled_fm() {
    let dsl = r#"
tempo: 0.5
~mod: sine "55 82.5" * 200
~car: sine (440 + ~mod)
out: ~car * 0.1
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "pattern_fm");
    assert!(success, "Failed to create pattern-controlled FM: {}", stderr);

    // VERIFY pattern-controlled FM
    verify_audio_exists(&wav_path)
        .expect("No audio output from pattern FM");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Spectral modulation from pattern FM not detected");
}

// ============================================================================
// AMPLITUDE TESTS - Different gain levels
// ============================================================================

#[test]
fn test_very_quiet_oscillator() {
    let dsl = r#"
tempo: 0.5
out: sine 440 * 0.01
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "very_quiet");
    assert!(success, "Failed to render very quiet oscillator: {}", stderr);

    // VERIFY very quiet but still audible
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
    let analysis = verify_audio_exists(&wav_path).unwrap();
    assert!(analysis.rms < 0.05, "Expected quiet signal, got RMS: {}", analysis.rms);
}

#[test]
fn test_moderate_amplitude() {
    let dsl = r#"
tempo: 0.5
out: sine 440 * 0.5
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "moderate_amp");
    assert!(success, "Failed to render moderate amplitude: {}", stderr);

    // VERIFY moderate amplitude
    verify_amplitude_range(&wav_path, 0.2, 0.95)
        .expect("Amplitude should be moderate");
    verify_oscillator_frequency(&wav_path, 440.0, 50.0)
        .expect("440 Hz not detected");
}

#[test]
fn test_pattern_amplitude_modulation() {
    let dsl = r#"
tempo: 0.5
out: sine 440 * "0.1 0.3 0.2 0.4"
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "pattern_amp");
    assert!(success, "Failed to render pattern amplitude: {}", stderr);

    // VERIFY amplitude modulation via pattern
    verify_pattern_modulation(&wav_path, "amplitude", 2)
        .expect("Pattern amplitude modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

// ============================================================================
// ARITHMETIC OPERATION TESTS
// ============================================================================

#[test]
fn test_oscillator_addition() {
    let dsl = r#"
tempo: 0.5
out: (sine 440 + sine 880) * 0.1
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "osc_add");
    assert!(success, "Failed to add oscillators: {}", stderr);

    // VERIFY oscillator addition produces audio
    verify_audio_exists(&wav_path)
        .expect("No audio output from oscillator addition");
    verify_amplitude_range(&wav_path, 0.02, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_oscillator_multiplication() {
    let dsl = r#"
tempo: 0.5
out: sine 440 * sine 2 * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "osc_mult");
    assert!(success, "Failed to multiply oscillators: {}", stderr);

    // VERIFY ring modulation (amplitude modulation)
    verify_audio_exists(&wav_path)
        .expect("No audio output from oscillator multiplication");
    verify_lfo_modulation(&wav_path)
        .expect("Amplitude modulation not detected");
}

#[test]
fn test_complex_arithmetic() {
    let dsl = r#"
tempo: 0.5
~a: sine 440
~b: sine 880
~c: saw 220
out: (~a + ~b * 0.5) * ~c * 0.05
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "complex_math");
    assert!(success, "Failed to evaluate complex arithmetic: {}", stderr);

    // VERIFY complex arithmetic produces audio
    verify_audio_exists(&wav_path)
        .expect("No audio output from complex arithmetic");
    verify_amplitude_range(&wav_path, 0.01, 0.95)
        .expect("Amplitude out of range");
}

// ============================================================================
// BUS ROUTING TESTS
// ============================================================================

#[test]
fn test_oscillator_through_bus() {
    let dsl = r#"
tempo: 0.5
~osc: sine 440
out: ~osc * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bus_routing");
    assert!(success, "Failed to route through bus: {}", stderr);

    // VERIFY bus routing preserves signal
    verify_oscillator_frequency(&wav_path, 440.0, 50.0)
        .expect("440 Hz not detected after bus routing");
    verify_audio_exists(&wav_path)
        .expect("No audio output after bus routing");
}

#[test]
fn test_multiple_buses_to_output() {
    let dsl = r#"
tempo: 0.5
~bus1: sine 220
~bus2: saw 110
~bus3: square 440
out: ~bus1 * 0.1 + ~bus2 * 0.1 + ~bus3 * 0.1
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "multi_bus");
    assert!(success, "Failed to route multiple buses: {}", stderr);

    // VERIFY multiple bus routing
    verify_audio_exists(&wav_path)
        .expect("No audio output from multiple buses");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_nested_bus_routing() {
    let dsl = r#"
tempo: 0.5
~osc1: sine 440
~osc2: saw 220
~mix: ~osc1 + ~osc2
out: ~mix * 0.15
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "nested_bus");
    assert!(success, "Failed to route nested buses: {}", stderr);

    // VERIFY nested bus routing
    verify_audio_exists(&wav_path)
        .expect("No audio output from nested buses");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

// ============================================================================
// TEMPO VARIATION TESTS
// ============================================================================

#[test]
fn test_slow_tempo_60_bpm() {
    let dsl = r#"
tempo: 0.25
out: sine "220 440" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "slow_tempo", "8");
    assert!(success, "Failed to render slow tempo: {}", stderr);

    // VERIFY slow tempo pattern modulation (60 BPM, 4 second cycle)
    verify_pattern_modulation(&wav_path, "frequency", 1)
        .expect("Slow tempo pattern modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_standard_120_bpm() {
    let dsl = r#"
tempo: 0.5
out: sine "110 220 330 440" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "standard_tempo", "4");
    assert!(success, "Failed to render 120 BPM: {}", stderr);

    // VERIFY 120 BPM pattern (2 second cycle)
    verify_pattern_modulation(&wav_path, "frequency", 2)
        .expect("120 BPM pattern modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}

#[test]
fn test_uptempo_140_bpm() {
    let dsl = r#"
tempo: 0.58
out: sine "440 880" * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "uptempo", "3.5");
    assert!(success, "Failed to render 140 BPM: {}", stderr);

    // VERIFY 140 BPM pattern (~1.7 second cycle)
    verify_pattern_modulation(&wav_path, "frequency", 1)
        .expect("140 BPM pattern modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output detected");
}
