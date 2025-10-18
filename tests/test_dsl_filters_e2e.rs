/// End-to-end tests for filter DSL syntax
/// Tests all filter types and modulation using actual .ph file syntax

use std::process::Command;
use std::fs;

fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String) {
    let ph_path = format!("/tmp/test_filter_{}.ph", test_name);
    let wav_path = format!("/tmp/test_filter_{}.wav", test_name);

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
// LOWPASS FILTER TESTS
// ============================================================================

#[test]
fn test_lpf_constant_cutoff() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 # lpf 2000 0.8
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_constant");
    assert!(success, "Failed to render lpf with constant cutoff: {}", stderr);
}

#[test]
fn test_lpf_low_cutoff() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 500 0.7
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_low_cut");
    assert!(success, "Failed to render lpf with low cutoff: {}", stderr);
}

#[test]
fn test_lpf_high_cutoff() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 # lpf 5000 0.5
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_high_cut");
    assert!(success, "Failed to render lpf with high cutoff: {}", stderr);
}

#[test]
fn test_lpf_pattern_cutoff() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 # lpf "500 1000 1500 2000" 0.8
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_pattern_cut");
    assert!(success, "Failed to render lpf with pattern cutoff: {}", stderr);
}

#[test]
fn test_lpf_low_resonance() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 1000 0.3
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_low_res");
    assert!(success, "Failed to render lpf with low resonance: {}", stderr);
}

#[test]
fn test_lpf_high_resonance() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 1000 0.95
out: ~bass * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_high_res");
    assert!(success, "Failed to render lpf with high resonance: {}", stderr);
}

#[test]
fn test_lpf_pattern_resonance() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 # lpf 1500 "0.3 0.6 0.8 0.9"
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_pattern_res");
    assert!(success, "Failed to render lpf with pattern resonance: {}", stderr);
}

#[test]
fn test_lpf_both_patterns() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 # lpf "500 1500 2500" "0.5 0.8"
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_both_patterns");
    assert!(success, "Failed to render lpf with both patterns: {}", stderr);
}

// ============================================================================
// HIGHPASS FILTER TESTS
// ============================================================================

#[test]
fn test_hpf_constant_cutoff() {
    let dsl = r#"
tempo: 2.0
~sig: saw 220 # hpf 500 0.7
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_constant");
    assert!(success, "Failed to render hpf with constant cutoff: {}", stderr);
}

#[test]
fn test_hpf_low_cutoff() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # hpf 100 0.5
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_low_cut");
    assert!(success, "Failed to render hpf with low cutoff: {}", stderr);
}

#[test]
fn test_hpf_high_cutoff() {
    let dsl = r#"
tempo: 2.0
~sig: saw 220 # hpf 2000 0.8
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_high_cut");
    assert!(success, "Failed to render hpf with high cutoff: {}", stderr);
}

#[test]
fn test_hpf_pattern_cutoff() {
    let dsl = r#"
tempo: 2.0
~sig: saw 220 # hpf "200 400 800 1600" 0.7
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_pattern_cut");
    assert!(success, "Failed to render hpf with pattern cutoff: {}", stderr);
}

// ============================================================================
// BANDPASS FILTER TESTS
// ============================================================================

#[test]
fn test_bpf_constant_cutoff() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # bpf 1000 0.7
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_constant");
    assert!(success, "Failed to render bpf with constant cutoff: {}", stderr);
}

#[test]
fn test_bpf_narrow_bandwidth() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # bpf 1000 0.95
out: ~sig * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_narrow");
    assert!(success, "Failed to render bpf with narrow bandwidth: {}", stderr);
}

#[test]
fn test_bpf_wide_bandwidth() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # bpf 1000 0.3
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_wide");
    assert!(success, "Failed to render bpf with wide bandwidth: {}", stderr);
}

#[test]
fn test_bpf_pattern_cutoff() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # bpf "500 1000 2000 4000" 0.8
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_pattern");
    assert!(success, "Failed to render bpf with pattern cutoff: {}", stderr);
}

// ============================================================================
// LFO MODULATED FILTER TESTS - The signature Phonon feature!
// ============================================================================

#[test]
fn test_lpf_lfo_modulated_cutoff() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8
out: ~bass * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_lfo_cutoff");
    assert!(success, "Failed to render LFO-modulated lpf: {}", stderr);
}

#[test]
fn test_lpf_slow_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.25 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 3000 + 200) 0.8
out: ~bass * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_slow_lfo");
    assert!(success, "Failed to render slow LFO filter: {}", stderr);
}

#[test]
fn test_lpf_fast_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 4 * 0.5 + 0.5
~bass: saw 110 # lpf (~lfo * 2000 + 800) 0.7
out: ~bass * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_fast_lfo");
    assert!(success, "Failed to render fast LFO filter: {}", stderr);
}

#[test]
fn test_lpf_triangle_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: tri 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2500 + 500) 0.8
out: ~bass * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_tri_lfo");
    assert!(success, "Failed to render triangle LFO filter: {}", stderr);
}

#[test]
fn test_lpf_square_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo: square 1 * 0.5 + 0.5
~bass: saw 110 # lpf (~lfo * 3000 + 500) 0.7
out: ~bass * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_square_lfo");
    assert!(success, "Failed to render square LFO filter: {}", stderr);
}

#[test]
fn test_hpf_lfo_modulated() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.5 * 0.5 + 0.5
~sig: saw 220 # hpf (~lfo * 1500 + 100) 0.7
out: ~sig * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_lfo");
    assert!(success, "Failed to render LFO-modulated hpf: {}", stderr);
}

#[test]
fn test_bpf_lfo_modulated() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 1 * 0.5 + 0.5
~sig: saw 110 # bpf (~lfo * 3000 + 500) 0.8
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_lfo");
    assert!(success, "Failed to render LFO-modulated bpf: {}", stderr);
}

#[test]
fn test_lpf_resonance_lfo_modulated() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.5 * 0.3 + 0.5
~bass: saw 55 # lpf 1500 ~lfo
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_res_lfo");
    assert!(success, "Failed to render LFO-modulated resonance: {}", stderr);
}

#[test]
fn test_lpf_both_params_lfo() {
    let dsl = r#"
tempo: 2.0
~lfo1: sine 0.5 * 0.5 + 0.5
~lfo2: tri 0.25 * 0.3 + 0.5
~bass: saw 55 # lpf (~lfo1 * 2000 + 500) ~lfo2
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_both_lfo");
    assert!(success, "Failed to render dual LFO filter: {}", stderr);
}

// ============================================================================
// DIFFERENT SOURCE SIGNAL TESTS
// ============================================================================

#[test]
fn test_lpf_on_sine() {
    let dsl = r#"
tempo: 2.0
~sig: sine 880 # lpf 1200 0.8
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_sine");
    assert!(success, "Failed to filter sine wave: {}", stderr);
}

#[test]
fn test_lpf_on_square() {
    let dsl = r#"
tempo: 2.0
~sig: square 220 # lpf 1500 0.7
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_square");
    assert!(success, "Failed to filter square wave: {}", stderr);
}

#[test]
fn test_lpf_on_tri() {
    let dsl = r#"
tempo: 2.0
~sig: tri 110 # lpf 2000 0.8
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_tri");
    assert!(success, "Failed to filter triangle wave: {}", stderr);
}

#[test]
fn test_lpf_on_pattern_osc() {
    let dsl = r#"
tempo: 2.0
~sig: saw "55 82.5 110" # lpf 1500 0.8
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_pattern_osc");
    assert!(success, "Failed to filter pattern-controlled oscillator: {}", stderr);
}

#[test]
fn test_lpf_on_mixed_signals() {
    let dsl = r#"
tempo: 2.0
~mix: sine 440 + saw 110
~filtered: ~mix # lpf 2000 0.7
out: ~filtered * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_mixed");
    assert!(success, "Failed to filter mixed signals: {}", stderr);
}

// ============================================================================
// CHAINED FILTER TESTS
// ============================================================================

#[test]
fn test_two_lpf_cascade() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # lpf 2000 0.5 # lpf 1000 0.7
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "cascade_lpf");
    assert!(success, "Failed to cascade two lpf: {}", stderr);
}

#[test]
fn test_lpf_then_hpf() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # lpf 3000 0.6 # hpf 200 0.5
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_hpf_chain");
    assert!(success, "Failed to chain lpf then hpf: {}", stderr);
}

#[test]
fn test_hpf_then_lpf() {
    let dsl = r#"
tempo: 2.0
~sig: saw 110 # hpf 100 0.5 # lpf 2000 0.7
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_lpf_chain");
    assert!(success, "Failed to chain hpf then lpf: {}", stderr);
}

#[test]
fn test_three_filter_cascade() {
    let dsl = r#"
tempo: 2.0
~sig: saw 55 # lpf 3000 0.5 # bpf 1500 0.7 # lpf 2000 0.6
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "three_filters");
    assert!(success, "Failed to cascade three filters: {}", stderr);
}

// ============================================================================
// REVERSE SIGNAL FLOW TESTS - Using << operator
// ============================================================================

#[test]
fn test_lpf_reverse_flow() {
    let dsl = r#"
tempo: 2.0
~bass: lpf 1500 0.8 << saw 55
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_reverse");
    assert!(success, "Failed to use lpf with reverse flow: {}", stderr);
}

#[test]
fn test_hpf_reverse_flow() {
    let dsl = r#"
tempo: 2.0
~sig: hpf 500 0.7 << saw 220
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_reverse");
    assert!(success, "Failed to use hpf with reverse flow: {}", stderr);
}

#[test]
fn test_bpf_reverse_flow() {
    let dsl = r#"
tempo: 2.0
~sig: bpf 1000 0.8 << saw 110
out: ~sig * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_reverse");
    assert!(success, "Failed to use bpf with reverse flow: {}", stderr);
}

// ============================================================================
// EXTREME PARAMETER TESTS
// ============================================================================

#[test]
fn test_lpf_very_low_cutoff() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 100 0.7
out: ~bass * 0.4
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_very_low");
    assert!(success, "Failed with very low cutoff: {}", stderr);
}

#[test]
fn test_lpf_very_high_cutoff() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 10000 0.5
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_very_high");
    assert!(success, "Failed with very high cutoff: {}", stderr);
}

#[test]
fn test_lpf_zero_resonance() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 1500 0.0
out: ~bass * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_zero_res");
    assert!(success, "Failed with zero resonance: {}", stderr);
}

#[test]
fn test_lpf_near_max_resonance() {
    let dsl = r#"
tempo: 2.0
~bass: saw 110 # lpf 1500 0.99
out: ~bass * 0.2
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_max_res");
    assert!(success, "Failed with near-max resonance: {}", stderr);
}
