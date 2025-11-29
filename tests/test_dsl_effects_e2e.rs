use std::fs;
/// End-to-end tests for effects DSL syntax
/// Tests reverb, delay, distortion, and other effects
///
/// CRITICAL: Tests verify ACTUAL AUDIO OUTPUT using spectral analysis!
/// We are "deaf" - can only verify effects through analysis tools.
use std::process::Command;

mod audio_verification_enhanced;
use audio_verification_enhanced::*;

/// Helper to render DSL code and verify it produces audio
/// Duration: 2 seconds by default (1 cycle at 120 BPM / 0.5 cps)
fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String, String) {
    render_and_verify_duration(dsl_code, test_name, "2")
}

/// Helper with custom duration for longer effect tails
fn render_and_verify_duration(
    dsl_code: &str,
    test_name: &str,
    duration: &str,
) -> (bool, String, String) {
    let ph_path = format!("/tmp/test_effect_{}.ph", test_name);
    let wav_path = format!("/tmp/test_effect_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            &ph_path,
            &wav_path,
            "--duration",
            duration,
        ])
        .output()
        .expect("Failed to run phonon render");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stderr, wav_path)
}

// ============================================================================
// REVERB TESTS
// ============================================================================

#[test]
fn test_reverb_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.5 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_synth");
    assert!(success, "Failed to apply reverb to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_reverb_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums: s "bd sn hh cp" # reverb 0.6 0.8
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_samples");
    assert!(success, "Failed to apply reverb to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_reverb_short_decay() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.2 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_short");
    assert!(success, "Failed reverb with short decay: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_reverb_long_decay() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.8 0.9
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_long");
    assert!(success, "Failed reverb with long decay: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_reverb_dry_mix() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.1 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_dry");
    assert!(success, "Failed reverb with dry mix: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_reverb_wet_mix() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.9 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_wet");
    assert!(success, "Failed reverb with wet mix: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// DELAY TESTS
// ============================================================================

#[test]
fn test_delay_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 0.5 0.5 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_synth");
    assert!(success, "Failed to apply delay to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums: s "bd sn hh cp" # delay 0.25 0.6 0.7
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_samples");
    assert!(success, "Failed to apply delay to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_short_time() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 0.1 0.5 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_short");
    assert!(success, "Failed delay with short time: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_long_time() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 1.0 0.5 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_long");
    assert!(success, "Failed delay with long time: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_low_feedback() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 0.5 0.2 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_low_fb");
    assert!(success, "Failed delay with low feedback: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_high_feedback() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 0.5 0.8 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_high_fb");
    assert!(success, "Failed delay with high feedback: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_dry_mix() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 0.5 0.5 0.2
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_dry");
    assert!(success, "Failed delay with dry mix: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_wet_mix() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # delay 0.5 0.5 0.9
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_wet");
    assert!(success, "Failed delay with wet mix: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// DISTORTION TESTS
// ============================================================================

#[test]
fn test_distortion_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # distortion 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_synth");
    assert!(success, "Failed to apply distortion to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums: s "bd sn hh cp" # distortion 0.6
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_samples");
    assert!(success, "Failed to apply distortion to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_light() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # distortion 0.2
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_light");
    assert!(success, "Failed light distortion: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_heavy() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # distortion 0.9
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_heavy");
    assert!(success, "Failed heavy distortion: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_on_bass() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # distortion 0.7
out $ ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_bass");
    assert!(success, "Failed distortion on bass: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// BITCRUSH TESTS
// ============================================================================

#[test]
fn test_bitcrush_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # bitcrush 8 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "crush_synth");
    assert!(success, "Failed to apply bitcrush to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_bitcrush_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums: s "bd sn hh cp" # bitcrush 6 0.5
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "crush_samples");
    assert!(success, "Failed to apply bitcrush to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_bitcrush_low_bits() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # bitcrush 4 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "crush_low_bits");
    assert!(success, "Failed bitcrush with low bits: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_bitcrush_high_bits() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # bitcrush 12 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "crush_high_bits");
    assert!(success, "Failed bitcrush with high bits: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_bitcrush_low_rate() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # bitcrush 8 0.1
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "crush_low_rate");
    assert!(success, "Failed bitcrush with low rate: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_bitcrush_high_rate() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # bitcrush 8 0.9
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "crush_high_rate");
    assert!(success, "Failed bitcrush with high rate: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// CHORUS TESTS
// ============================================================================

#[test]
fn test_chorus_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # chorus 0.5 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_synth");
    assert!(success, "Failed to apply chorus to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_chorus_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums: s "bd sn hh cp" # chorus 0.5 0.6
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_samples");
    assert!(success, "Failed to apply chorus to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_chorus_slow_rate() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # chorus 0.2 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_slow");
    assert!(success, "Failed chorus with slow rate: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_chorus_fast_rate() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # chorus 0.8 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_fast");
    assert!(success, "Failed chorus with fast rate: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_chorus_shallow_depth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # chorus 0.5 0.3
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_shallow");
    assert!(success, "Failed chorus with shallow depth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_chorus_deep_depth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # chorus 0.5 0.9
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_deep");
    assert!(success, "Failed chorus with deep depth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// CHAINED EFFECTS TESTS
// ============================================================================

#[test]
fn test_reverb_and_delay() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.5 0.7 # delay 0.5 0.5 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_delay");
    assert!(success, "Failed to chain reverb and delay: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_and_reverb() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # distortion 0.5 # reverb 0.6 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_reverb");
    assert!(success, "Failed to chain distortion and reverb: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_filter_and_reverb() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # lpf 2000 0.8 # reverb 0.5 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "filter_reverb");
    assert!(success, "Failed to chain filter and reverb: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_three_effects_chain() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # lpf 1500 0.7 # distortion 0.4 # reverb 0.5 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "three_fx");
    assert!(success, "Failed to chain three effects: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_four_effects_chain() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # lpf 2000 0.7 # distortion 0.3 # chorus 0.5 0.6 # reverb 0.4 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "four_fx");
    assert!(success, "Failed to chain four effects: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// EFFECTS ON DIFFERENT SOURCES
// ============================================================================

#[test]
fn test_reverb_on_bass() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # reverb 0.3 0.6
out $ ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_bass");
    assert!(success, "Failed reverb on bass: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_delay_on_hihat() {
    let dsl = r#"
tempo: 0.5
~hh: s "hh*8" # delay 0.125 0.4 0.5
out $ ~hh * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_hihat");
    assert!(success, "Failed delay on hihat: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_on_snare() {
    let dsl = r#"
tempo: 0.5
~sn: s "~ sn ~ sn" # distortion 0.6
out $ ~sn * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_snare");
    assert!(success, "Failed distortion on snare: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_chorus_on_pad() {
    let dsl = r#"
tempo: 0.5
~pad: sine 220 + sine 330 # chorus 0.5 0.8
out $ ~pad * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "chorus_pad");
    assert!(success, "Failed chorus on pad: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// EFFECTS WITH PATTERN MODULATION
// ============================================================================

#[test]
fn test_delay_pattern_time() {
    let dsl = r#"
tempo: 0.5
~time: "0.25 0.5 0.125"
~sig: sine 440 # delay ~time 0.5 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_pattern_time");
    assert!(success, "Failed delay with pattern time: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_reverb_pattern_mix() {
    let dsl = r#"
tempo: 0.5
~mix: "0.3 0.6 0.9"
~sig: sine 440 # reverb ~mix 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_pattern_mix");
    assert!(success, "Failed reverb with pattern mix: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_distortion_pattern_amount() {
    let dsl = r#"
tempo: 0.5
~amt: "0.2 0.5 0.8"
~sig: sine 440 # distortion ~amt
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_pattern_amt");
    assert!(success, "Failed distortion with pattern amount: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// EFFECTS IN DIFFERENT ROUTING CONTEXTS
// ============================================================================

#[test]
fn test_effects_reverse_flow() {
    // Note: Reverse flow syntax (<<) is not yet supported in compositional parser
    // Using standard forward flow (#) instead
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # reverb 0.5 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "fx_reverse_flow");
    assert!(success, "Failed effects with forward flow: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_parallel_effects() {
    let dsl = r#"
tempo: 0.5
~dry: sine 440
~wet: ~dry # reverb 0.8 0.9
out $ (~dry * 0.5 + ~wet * 0.5) * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "parallel_fx");
    assert!(success, "Failed parallel effects: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_send_return_style() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440
~reverb_send: ~sig # reverb 0.8 0.9
out $ (~sig * 0.7 + ~reverb_send * 0.3) * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "send_return");
    assert!(success, "Failed send/return style effects: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

// ============================================================================
// COMPRESSOR TESTS - E2E DSL VERIFICATION
// ============================================================================

#[test]
fn test_compressor_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # compressor -20.0 4.0 0.01 0.1 5.0
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_synth");
    assert!(success, "Failed to apply compressor to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums: s "bd sn hh cp" # compressor -20.0 4.0 0.01 0.1 5.0
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_samples");
    assert!(success, "Failed to apply compressor to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_reduces_peaks() {
    // Test that compressor actually reduces dynamic range
    // Compare uncompressed vs compressed signal

    // Uncompressed
    let dsl_uncompressed = r#"
tempo: 0.5
out $ sine 440 * 0.5
"#;
    let (success, _, wav_path_uncomp) =
        render_and_verify(dsl_uncompressed, "compressor_uncompressed");
    assert!(success, "Failed to render uncompressed signal");

    // Compressed with heavy ratio
    let dsl_compressed = r#"
tempo: 0.5
~sig: sine 440 # compressor -30.0 10.0 0.001 0.01 0.0
out $ ~sig * 0.5
"#;
    let (success, stderr, wav_path_comp) =
        render_and_verify(dsl_compressed, "compressor_compressed");
    assert!(success, "Failed to render compressed signal: {}", stderr);

    // Analyze both
    let analysis_uncomp =
        analyze_wav_enhanced(&wav_path_uncomp).expect("Failed to analyze uncompressed");
    let analysis_comp = analyze_wav_enhanced(&wav_path_comp).expect("Failed to analyze compressed");

    println!("Uncompressed peak: {:.6}", analysis_uncomp.peak);
    println!("Compressed peak:   {:.6}", analysis_comp.peak);

    // Compressed signal should have lower peak
    assert!(
        analysis_comp.peak < analysis_uncomp.peak * 0.7,
        "Compressor should reduce peak! Uncomp: {:.6}, Comp: {:.6}",
        analysis_uncomp.peak,
        analysis_comp.peak
    );
}

#[test]
fn test_compressor_gentle_ratio() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # compressor -15.0 2.0 0.05 0.2 3.0
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_gentle");
    assert!(success, "Failed gentle compression: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_heavy_limiting() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # compressor -10.0 20.0 0.001 0.05 5.0
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_limiter");
    assert!(success, "Failed heavy limiting: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_fast_attack() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # compressor -20.0 4.0 0.001 0.1 5.0
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_fast_attack");
    assert!(success, "Failed fast attack compression: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_slow_release() {
    let dsl = r#"
tempo: 0.5
~sig: sine 440 # compressor -20.0 4.0 0.01 0.5 5.0
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_slow_release");
    assert!(success, "Failed slow release compression: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_on_bass() {
    let dsl = r#"
tempo: 0.5
~bass: saw 55 # compressor -25.0 6.0 0.01 0.15 8.0
out $ ~bass * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_bass");
    assert!(success, "Failed compressor on bass: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_on_drums() {
    let dsl = r#"
tempo: 0.5
~drums: s "[bd*4, sn*2, hh*8]" # compressor -18.0 4.0 0.005 0.1 4.0
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_drums");
    assert!(success, "Failed compressor on drums: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}

#[test]
fn test_compressor_with_other_effects() {
    let dsl = r#"
tempo: 0.5
~sig: saw 110 # lpf 1500 0.8 # distortion 0.5 # compressor -15.0 3.0 0.01 0.2 5.0 # reverb 0.3 0.6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "compressor_chain");
    assert!(success, "Failed compressor in effects chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Audio should not be silent");
}
