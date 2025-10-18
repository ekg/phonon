/// End-to-end tests for oscillator DSL syntax
/// Tests all oscillator types and variations using actual .ph file syntax

use std::process::Command;
use std::fs;

/// Helper to render DSL code and verify it produces audio
fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String) {
    let ph_path = format!("/tmp/test_{}.ph", test_name);
    let wav_path = format!("/tmp/test_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", &ph_path, &wav_path, "--duration", "1"])
        .output()
        .expect("Failed to run phonon render");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stderr)
}

// ============================================================================
// BASIC OSCILLATOR TESTS - Each oscillator type with constant frequency
// ============================================================================

#[test]
fn test_sine_constant_frequency() {
    let dsl = r#"
tempo: 2.0
out: sine 440 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "sine_constant");
    assert!(success, "Failed to render sine oscillator: {}", stderr);
}

#[test]
fn test_saw_constant_frequency() {
    let dsl = r#"
tempo: 2.0
out: saw 110 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "saw_constant");
    assert!(success, "Failed to render saw oscillator: {}", stderr);
}

#[test]
fn test_square_constant_frequency() {
    let dsl = r#"
tempo: 2.0
out: square 220 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "square_constant");
    assert!(success, "Failed to render square oscillator: {}", stderr);
}

#[test]
fn test_tri_constant_frequency() {
    let dsl = r#"
tempo: 2.0
out: tri 330 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "tri_constant");
    assert!(success, "Failed to render tri oscillator: {}", stderr);
}

// ============================================================================
// PATTERN-CONTROLLED FREQUENCY TESTS - Each oscillator with pattern
// ============================================================================

#[test]
fn test_sine_pattern_frequency_2_values() {
    let dsl = r#"
tempo: 2.0
out: sine "220 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "sine_pattern_2");
    assert!(success, "Failed to render sine with 2-value pattern: {}", stderr);
}

#[test]
fn test_sine_pattern_frequency_4_values() {
    let dsl = r#"
tempo: 2.0
out: sine "110 220 330 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "sine_pattern_4");
    assert!(success, "Failed to render sine with 4-value pattern: {}", stderr);
}

#[test]
fn test_saw_pattern_frequency() {
    let dsl = r#"
tempo: 2.0
out: saw "55 82.5 110 165" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "saw_pattern");
    assert!(success, "Failed to render saw with pattern: {}", stderr);
}

#[test]
fn test_square_pattern_frequency() {
    let dsl = r#"
tempo: 2.0
out: square "110 165 220" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "square_pattern");
    assert!(success, "Failed to render square with pattern: {}", stderr);
}

#[test]
fn test_tri_pattern_frequency() {
    let dsl = r#"
tempo: 2.0
out: tri "220 330 440 550" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "tri_pattern");
    assert!(success, "Failed to render tri with pattern: {}", stderr);
}

// ============================================================================
// MULTIPLE OSCILLATOR MIXING TESTS
// ============================================================================

#[test]
fn test_two_sines_mixed() {
    let dsl = r#"
tempo: 2.0
~osc1: sine 440 * 0.1
~osc2: sine 880 * 0.1
out: ~osc1 + ~osc2
"#;
    let (success, stderr) = render_and_verify(dsl, "two_sines");
    assert!(success, "Failed to mix two sines: {}", stderr);
}

#[test]
fn test_all_oscillator_types_mixed() {
    let dsl = r#"
tempo: 2.0
~s: sine 440 * 0.05
~saw: saw 220 * 0.05
~sq: square 110 * 0.05
~t: tri 880 * 0.05
out: ~s + ~saw + ~sq + ~t
"#;
    let (success, stderr) = render_and_verify(dsl, "all_oscs_mixed");
    assert!(success, "Failed to mix all oscillator types: {}", stderr);
}

#[test]
fn test_weighted_oscillator_mix() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 * 0.3
~mid: square 220 * 0.2
~high: sine 880 * 0.1
out: ~bass + ~mid + ~high
"#;
    let (success, stderr) = render_and_verify(dsl, "weighted_mix");
    assert!(success, "Failed to create weighted mix: {}", stderr);
}

// ============================================================================
// LFO MODULATION TESTS - Low frequency oscillators modulating audio
// ============================================================================

#[test]
fn test_lfo_amplitude_modulation() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 2 * 0.5 + 0.5
~carrier: sine 440
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "lfo_am");
    assert!(success, "Failed to create LFO amplitude modulation: {}", stderr);
}

#[test]
fn test_slow_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.25 * 0.5 + 0.5
~carrier: saw 110
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "slow_lfo");
    assert!(success, "Failed to create slow LFO: {}", stderr);
}

#[test]
fn test_fast_lfo_vibrato() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 6 * 10
~carrier: sine (440 + ~lfo)
out: ~carrier * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "fast_lfo");
    assert!(success, "Failed to create vibrato: {}", stderr);
}

#[test]
fn test_triangle_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: tri 1 * 0.5 + 0.5
~carrier: sine 440
out: ~carrier * ~lfo * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "tri_lfo");
    assert!(success, "Failed to create triangle LFO: {}", stderr);
}

// ============================================================================
// FREQUENCY RANGE TESTS - Low bass to high frequencies
// ============================================================================

#[test]
fn test_sub_bass_frequency() {
    let dsl = r#"
tempo: 2.0
out: sine 40 * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "sub_bass");
    assert!(success, "Failed to render sub-bass: {}", stderr);
}

#[test]
fn test_bass_frequency() {
    let dsl = r#"
tempo: 2.0
out: saw 55 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "bass");
    assert!(success, "Failed to render bass: {}", stderr);
}

#[test]
fn test_mid_frequency() {
    let dsl = r#"
tempo: 2.0
out: square 440 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "mid");
    assert!(success, "Failed to render mid frequency: {}", stderr);
}

#[test]
fn test_high_frequency() {
    let dsl = r#"
tempo: 2.0
out: sine 3520 * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "high");
    assert!(success, "Failed to render high frequency: {}", stderr);
}

// ============================================================================
// PATTERN MODULATION WITH COMPLEX PATTERNS
// ============================================================================

#[test]
fn test_8_step_frequency_pattern() {
    let dsl = r#"
tempo: 2.0
out: sine "110 165 220 275 330 385 440 495" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "8_step_pattern");
    assert!(success, "Failed to render 8-step pattern: {}", stderr);
}

#[test]
fn test_octave_pattern() {
    let dsl = r#"
tempo: 2.0
out: sine "110 220 440 880" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "octave_pattern");
    assert!(success, "Failed to render octave pattern: {}", stderr);
}

#[test]
fn test_pentatonic_pattern() {
    let dsl = r#"
tempo: 2.0
out: sine "220 247.5 275 330 370" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "pentatonic");
    assert!(success, "Failed to render pentatonic pattern: {}", stderr);
}

// ============================================================================
// AUDIO RATE MODULATION TESTS - FM synthesis
// ============================================================================

#[test]
fn test_simple_fm_synthesis() {
    let dsl = r#"
tempo: 2.0
~modulator: sine 55 * 100
~carrier: sine (440 + ~modulator)
out: ~carrier * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "simple_fm");
    assert!(success, "Failed to create FM synthesis: {}", stderr);
}

#[test]
fn test_deep_fm_modulation() {
    let dsl = r#"
tempo: 2.0
~mod: sine 110 * 500
~car: sine (220 + ~mod)
out: ~car * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "deep_fm");
    assert!(success, "Failed to create deep FM: {}", stderr);
}

#[test]
fn test_pattern_controlled_fm() {
    let dsl = r#"
tempo: 2.0
~mod: sine "55 82.5" * 200
~car: sine (440 + ~mod)
out: ~car * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_fm");
    assert!(success, "Failed to create pattern-controlled FM: {}", stderr);
}

// ============================================================================
// AMPLITUDE TESTS - Different gain levels
// ============================================================================

#[test]
fn test_very_quiet_oscillator() {
    let dsl = r#"
tempo: 2.0
out: sine 440 * 0.01
"#;
    let (success, stderr) = render_and_verify(dsl, "very_quiet");
    assert!(success, "Failed to render very quiet oscillator: {}", stderr);
}

#[test]
fn test_moderate_amplitude() {
    let dsl = r#"
tempo: 2.0
out: sine 440 * 0.5
"#;
    let (success, stderr) = render_and_verify(dsl, "moderate_amp");
    assert!(success, "Failed to render moderate amplitude: {}", stderr);
}

#[test]
fn test_pattern_amplitude_modulation() {
    let dsl = r#"
tempo: 2.0
out: sine 440 * "0.1 0.3 0.2 0.4"
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_amp");
    assert!(success, "Failed to render pattern amplitude: {}", stderr);
}

// ============================================================================
// ARITHMETIC OPERATION TESTS
// ============================================================================

#[test]
fn test_oscillator_addition() {
    let dsl = r#"
tempo: 2.0
out: (sine 440 + sine 880) * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "osc_add");
    assert!(success, "Failed to add oscillators: {}", stderr);
}

#[test]
fn test_oscillator_multiplication() {
    let dsl = r#"
tempo: 2.0
out: sine 440 * sine 2 * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "osc_mult");
    assert!(success, "Failed to multiply oscillators: {}", stderr);
}

#[test]
fn test_complex_arithmetic() {
    let dsl = r#"
tempo: 2.0
~a: sine 440
~b: sine 880
~c: saw 220
out: (~a + ~b * 0.5) * ~c * 0.05
"#;
    let (success, stderr) = render_and_verify(dsl, "complex_math");
    assert!(success, "Failed to evaluate complex arithmetic: {}", stderr);
}

// ============================================================================
// BUS ROUTING TESTS
// ============================================================================

#[test]
fn test_oscillator_through_bus() {
    let dsl = r#"
tempo: 2.0
~osc: sine 440
out: ~osc * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "bus_routing");
    assert!(success, "Failed to route through bus: {}", stderr);
}

#[test]
fn test_multiple_buses_to_output() {
    let dsl = r#"
tempo: 2.0
~bus1: sine 220
~bus2: saw 110
~bus3: square 440
out: ~bus1 * 0.1 + ~bus2 * 0.1 + ~bus3 * 0.1
"#;
    let (success, stderr) = render_and_verify(dsl, "multi_bus");
    assert!(success, "Failed to route multiple buses: {}", stderr);
}

#[test]
fn test_nested_bus_routing() {
    let dsl = r#"
tempo: 2.0
~osc1: sine 440
~osc2: saw 220
~mix: ~osc1 + ~osc2
out: ~mix * 0.15
"#;
    let (success, stderr) = render_and_verify(dsl, "nested_bus");
    assert!(success, "Failed to route nested buses: {}", stderr);
}

// ============================================================================
// TEMPO VARIATION TESTS
// ============================================================================

#[test]
fn test_slow_tempo() {
    let dsl = r#"
tempo: 0.5
out: sine "220 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "slow_tempo");
    assert!(success, "Failed to render slow tempo: {}", stderr);
}

#[test]
fn test_fast_tempo() {
    let dsl = r#"
tempo: 4.0
out: sine "110 220 330 440" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "fast_tempo");
    assert!(success, "Failed to render fast tempo: {}", stderr);
}

#[test]
fn test_very_slow_tempo() {
    let dsl = r#"
tempo: 0.1
out: sine "440 880" * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "very_slow_tempo");
    assert!(success, "Failed to render very slow tempo: {}", stderr);
}
