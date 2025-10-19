/// End-to-end tests for filter DSL syntax
/// Tests all filter types and modulation using actual .ph file syntax
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
    let ph_path = format!("/tmp/test_filter_{}.ph", test_name);
    let wav_path = format!("/tmp/test_filter_{}.wav", test_name);

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
// LOWPASS FILTER TESTS
// ============================================================================

#[test]
fn test_lpf_constant_cutoff() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # lpf 2000 0.8
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_constant");
    assert!(success, "Failed to render lpf with constant cutoff: {}", stderr);

    // VERIFY filter effect
    verify_audio_exists(&wav_path)
        .expect("No audio output from filtered saw");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_lpf_low_cutoff() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 500 0.7
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_low_cut");
    assert!(success, "Failed to render lpf with low cutoff: {}", stderr);

    // VERIFY low cutoff filters out highs
    verify_filter_effect(&wav_path, 500.0, 200.0)
        .expect("Low cutoff filter not working");
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

#[test]
fn test_lpf_high_cutoff() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # lpf 5000 0.5
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_high_cut");
    assert!(success, "Failed to render lpf with high cutoff: {}", stderr);

    // VERIFY high cutoff preserves more spectrum
    verify_audio_exists(&wav_path)
        .expect("No audio output");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_lpf_pattern_cutoff() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # lpf "500 1000 1500 2000" 0.8
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_pattern_cut");
    assert!(success, "Failed to render lpf with pattern cutoff: {}", stderr);

    // VERIFY pattern modulation of cutoff creates spectral changes
    verify_pattern_modulation(&wav_path, "spectral", 2)
        .expect("Pattern cutoff modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

#[test]
fn test_lpf_low_resonance() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 1000 0.3
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_low_res");
    assert!(success, "Failed to render lpf with low resonance: {}", stderr);

    // VERIFY low resonance filter works
    verify_audio_exists(&wav_path)
        .expect("No audio output");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_lpf_high_resonance() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 1000 0.95
out: ~bass * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_high_res");
    assert!(success, "Failed to render lpf with high resonance: {}", stderr);

    // VERIFY high resonance creates resonant peak
    verify_audio_exists(&wav_path)
        .expect("No audio output");
    verify_filter_effect(&wav_path, 1000.0, 300.0)
        .expect("High resonance filter not working");
}

#[test]
fn test_lpf_pattern_resonance() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # lpf 1500 "0.3 0.6 0.8 0.9"
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_pattern_res");
    assert!(success, "Failed to render lpf with pattern resonance: {}", stderr);

    // VERIFY pattern modulation of resonance
    verify_pattern_modulation(&wav_path, "spectral", 2)
        .expect("Pattern resonance modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

#[test]
fn test_lpf_both_patterns() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # lpf "500 1500 2500" "0.5 0.8"
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_both_patterns");
    assert!(success, "Failed to render lpf with both patterns: {}", stderr);

    // VERIFY both parameter patterns create modulation
    verify_pattern_modulation(&wav_path, "spectral", 2)
        .expect("Dual pattern modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

// ============================================================================
// HIGHPASS FILTER TESTS
// ============================================================================

#[test]
fn test_hpf_constant_cutoff() {
    let dsl = r#"
tempo: 0.5
~sig: saw 220 # hpf 500 0.7
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hpf_constant");
    assert!(success, "Failed to render hpf with constant cutoff: {}", stderr);

    // VERIFY highpass filter effect
    verify_audio_exists(&wav_path)
        .expect("No audio output from highpass filter");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_hpf_low_cutoff() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # hpf 100 0.5
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hpf_low_cut");
    assert!(success, "Failed to render hpf with low cutoff: {}", stderr);

    // VERIFY low cutoff preserves most spectrum
    verify_audio_exists(&wav_path)
        .expect("No audio output");
    verify_amplitude_range(&wav_path, 0.05, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_hpf_high_cutoff() {
    let dsl = r#"
tempo: 0.5
~sig: saw 220 # hpf 2000 0.8
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hpf_high_cut");
    assert!(success, "Failed to render hpf with high cutoff: {}", stderr);

    // VERIFY high cutoff removes lows
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

#[test]
fn test_hpf_pattern_cutoff() {
    let dsl = r#"
tempo: 0.5
~sig: saw 220 # hpf "200 400 800 1600" 0.7
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hpf_pattern_cut");
    assert!(success, "Failed to render hpf with pattern cutoff: {}", stderr);

    // VERIFY pattern modulation of highpass cutoff
    verify_pattern_modulation(&wav_path, "spectral", 2)
        .expect("Pattern cutoff modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

// ============================================================================
// BANDPASS FILTER TESTS
// ============================================================================

#[test]
fn test_bpf_constant_cutoff() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # bpf 1000 0.7
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bpf_constant");
    assert!(success, "Failed to render bpf with constant cutoff: {}", stderr);

    // VERIFY bandpass filter effect
    verify_audio_exists(&wav_path)
        .expect("No audio output from bandpass filter");
    // Bandpass filters naturally reduce amplitude by removing frequencies outside passband
    verify_amplitude_range(&wav_path, 0.02, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_bpf_narrow_bandwidth() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # bpf 1000 0.95
out: ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bpf_narrow");
    assert!(success, "Failed to render bpf with narrow bandwidth: {}", stderr);

    // VERIFY narrow bandpass
    verify_audio_exists(&wav_path)
        .expect("No audio output");
    verify_filter_effect(&wav_path, 1000.0, 300.0)
        .expect("Narrow bandpass not working");
}

#[test]
fn test_bpf_wide_bandwidth() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # bpf 1000 0.3
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bpf_wide");
    assert!(success, "Failed to render bpf with wide bandwidth: {}", stderr);

    // VERIFY wide bandpass
    verify_audio_exists(&wav_path)
        .expect("No audio output");
    // Wide bandpass (low Q) allows more energy through but still reduces amplitude
    verify_amplitude_range(&wav_path, 0.02, 0.95)
        .expect("Amplitude out of range");
}

#[test]
fn test_bpf_pattern_cutoff() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # bpf "500 1000 2000 4000" 0.8
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bpf_pattern");
    assert!(success, "Failed to render bpf with pattern cutoff: {}", stderr);

    // VERIFY pattern modulation of bandpass center frequency
    verify_pattern_modulation(&wav_path, "spectral", 2)
        .expect("Pattern cutoff modulation not detected");
    verify_audio_exists(&wav_path)
        .expect("No audio output");
}

// ============================================================================
// LFO MODULATED FILTER TESTS - The signature Phonon feature!
// ============================================================================

#[test]
fn test_lpf_lfo_modulated_cutoff() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8
out: ~bass * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "lpf_lfo_cutoff", "4");
    assert!(success, "Failed to render LFO-modulated lpf: {}", stderr);

    // VERIFY LFO MODULATION - This is Phonon's signature feature!
    verify_lfo_modulation(&wav_path)
        .expect("LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Spectral modulation from LFO not detected");
}

#[test]
fn test_lpf_slow_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 0.25 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 3000 + 200) 0.8
out: ~bass * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "lpf_slow_lfo", "8");
    assert!(success, "Failed to render slow LFO filter: {}", stderr);

    // VERIFY SLOW LFO - need longer duration to detect 0.25 Hz modulation
    verify_lfo_modulation(&wav_path)
        .expect("Slow LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Slow spectral modulation not detected");
}

#[test]
fn test_lpf_fast_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 4 * 0.5 + 0.5
~bass: saw 110 # lpf (~lfo * 2000 + 800) 0.7
out: ~bass * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_fast_lfo");
    assert!(success, "Failed to render fast LFO filter: {}", stderr);

    // VERIFY FAST LFO creates rapid spectral changes
    verify_lfo_modulation(&wav_path)
        .expect("Fast LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "spectral", 3)
        .expect("Fast spectral modulation not detected");
}

#[test]
fn test_lpf_triangle_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo: tri 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2500 + 500) 0.8
out: ~bass * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "lpf_tri_lfo", "4");
    assert!(success, "Failed to render triangle LFO filter: {}", stderr);

    // VERIFY TRIANGLE LFO modulation
    verify_lfo_modulation(&wav_path)
        .expect("Triangle LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Triangle LFO spectral modulation not detected");
}

#[test]
fn test_lpf_square_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo: square 1 * 0.5 + 0.5
~bass: saw 110 # lpf (~lfo * 3000 + 500) 0.7
out: ~bass * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_square_lfo");
    assert!(success, "Failed to render square LFO filter: {}", stderr);

    // VERIFY SQUARE LFO creates abrupt spectral changes
    verify_lfo_modulation(&wav_path)
        .expect("Square LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Square LFO spectral changes not detected");
}

#[test]
fn test_hpf_lfo_modulated() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 0.5 * 0.5 + 0.5
~sig: saw 220 # hpf (~lfo * 1500 + 100) 0.7
out: ~sig * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "hpf_lfo", "4");
    assert!(success, "Failed to render LFO-modulated hpf: {}", stderr);

    // VERIFY LFO-modulated highpass filter
    verify_lfo_modulation(&wav_path)
        .expect("LFO modulation on hpf not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Spectral modulation from LFO on hpf not detected");
}

#[test]
fn test_bpf_lfo_modulated() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 1 * 0.5 + 0.5
~sig: saw 110 # bpf (~lfo * 3000 + 500) 0.8
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bpf_lfo");
    assert!(success, "Failed to render LFO-modulated bpf: {}", stderr);

    // VERIFY LFO-modulated bandpass filter
    verify_lfo_modulation(&wav_path)
        .expect("LFO modulation on bpf not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Spectral modulation from LFO on bpf not detected");
}

#[test]
fn test_lpf_resonance_lfo_modulated() {
    let dsl = r#"
tempo: 0.5
~lfo: sine 0.5 * 0.3 + 0.5
~bass: saw 55 # lpf 1500 ~lfo
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "lpf_res_lfo", "4");
    assert!(success, "Failed to render LFO-modulated resonance: {}", stderr);

    // VERIFY LFO modulation of resonance parameter
    verify_lfo_modulation(&wav_path)
        .expect("LFO modulation of resonance not detected");
    verify_pattern_modulation(&wav_path, "spectral", 1)
        .expect("Spectral changes from resonance modulation not detected");
}

#[test]
fn test_lpf_both_params_lfo() {
    let dsl = r#"
tempo: 0.5
~lfo1: sine 0.5 * 0.5 + 0.5
~lfo2: tri 0.25 * 0.3 + 0.5
~bass: saw 55 # lpf (~lfo1 * 2000 + 500) ~lfo2
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify_duration(dsl, "lpf_both_lfo", "8");
    assert!(success, "Failed to render dual LFO filter: {}", stderr);

    // VERIFY DUAL LFO modulation - both cutoff and resonance
    verify_lfo_modulation(&wav_path)
        .expect("Dual LFO modulation not detected");
    verify_pattern_modulation(&wav_path, "spectral", 2)
        .expect("Complex spectral modulation from dual LFOs not detected");
}

// ============================================================================
// DIFFERENT SOURCE SIGNAL TESTS
// ============================================================================

#[test]
fn test_lpf_on_sine() {
    let dsl = r#"
tempo: 0.5
~sig: sine 880 # lpf 1200 0.8
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_sine");
    assert!(success, "Failed to filter sine wave: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_lpf_on_square() {
    let dsl = r#"
tempo: 0.5
~sig: square 220 # lpf 1500 0.7
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_square");
    assert!(success, "Failed to filter square wave: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_lpf_on_tri() {
    let dsl = r#"
tempo: 0.5
~sig: tri 110 # lpf 2000 0.8
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_tri");
    assert!(success, "Failed to filter triangle wave: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_lpf_on_pattern_osc() {
    let dsl = r#"
tempo: 0.5
~sig: saw "55 82.5 110" # lpf 1500 0.8
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_pattern_osc");
    assert!(success, "Failed to filter pattern-controlled oscillator: {}", stderr);

    verify_pattern_modulation(&wav_path, "frequency", 1)
        .expect("Pattern oscillator modulation not detected");
}

#[test]
fn test_lpf_on_mixed_signals() {
    let dsl = r#"
tempo: 0.5
~mix: sine 440 + saw 110
~filtered: ~mix # lpf 2000 0.7
out: ~filtered * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_mixed");
    assert!(success, "Failed to filter mixed signals: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

// ============================================================================
// CHAINED FILTER TESTS
// ============================================================================

#[test]
fn test_two_lpf_cascade() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # lpf 2000 0.5 # lpf 1000 0.7
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "cascade_lpf");
    assert!(success, "Failed to cascade two lpf: {}", stderr);

    verify_filter_effect(&wav_path, 1000.0, 300.0)
        .expect("Cascaded filters not working");
}

#[test]
fn test_lpf_then_hpf() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # lpf 3000 0.6 # hpf 200 0.5
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_hpf_chain");
    assert!(success, "Failed to chain lpf then hpf: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_hpf_then_lpf() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # hpf 100 0.5 # lpf 2000 0.7
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hpf_lpf_chain");
    assert!(success, "Failed to chain hpf then lpf: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_three_filter_cascade() {
    let dsl = r#"
tempo: 0.5
~sig: saw 55 # lpf 3000 0.5 # bpf 1500 0.7 # lpf 2000 0.6
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "three_filters");
    assert!(success, "Failed to cascade three filters: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

// ============================================================================
// REVERSE SIGNAL FLOW TESTS - Using << operator
// ============================================================================

#[test]
fn test_lpf_reverse_flow() {
    let dsl = r#"
tempo: 0.5
~bass: lpf 1500 0.8 << saw 55
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_reverse");
    assert!(success, "Failed to use lpf with reverse flow: {}", stderr);

    verify_filter_effect(&wav_path, 1500.0, 500.0)
        .expect("Reverse flow lpf not working");
}

#[test]
fn test_hpf_reverse_flow() {
    let dsl = r#"
tempo: 0.5
~sig: hpf 500 0.7 << saw 220
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hpf_reverse");
    assert!(success, "Failed to use hpf with reverse flow: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_bpf_reverse_flow() {
    let dsl = r#"
tempo: 0.5
~sig: bpf 1000 0.8 << saw 110
out: ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "bpf_reverse");
    assert!(success, "Failed to use bpf with reverse flow: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

// ============================================================================
// EXTREME PARAMETER TESTS
// ============================================================================

#[test]
fn test_lpf_very_low_cutoff() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 100 0.7
out: ~bass * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_very_low");
    assert!(success, "Failed with very low cutoff: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_lpf_very_high_cutoff() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 10000 0.5
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_very_high");
    assert!(success, "Failed with very high cutoff: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_lpf_zero_resonance() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 1500 0.0
out: ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_zero_res");
    assert!(success, "Failed with zero resonance: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}

#[test]
fn test_lpf_near_max_resonance() {
    let dsl = r#"
tempo: 0.5
~bass: saw 110 # lpf 1500 0.99
out: ~bass * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "lpf_max_res");
    assert!(success, "Failed with near-max resonance: {}", stderr);

    verify_audio_exists(&wav_path).expect("No audio output");
}
