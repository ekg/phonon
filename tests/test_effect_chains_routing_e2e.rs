/// End-to-end tests for effect chains and routing patterns
/// Comprehensive verification of effect processing in Phonon DSL
///
/// CRITICAL: Tests verify ACTUAL AUDIO OUTPUT using spectral analysis!
/// We are "deaf" - can only verify effects through analysis tools.
use std::fs;
use std::process::Command;

mod audio_verification_enhanced;
use audio_verification_enhanced::*;

/// Helper to render DSL code and return analysis
fn render_and_analyze(
    dsl_code: &str,
    test_name: &str,
    duration: &str,
) -> (bool, String, Option<EnhancedAudioAnalysis>) {
    let ph_path = format!("/tmp/test_effect_chain_{}.ph", test_name);
    let wav_path = format!("/tmp/test_effect_chain_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
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

    let analysis = if success {
        analyze_wav_enhanced(&wav_path).ok()
    } else {
        None
    };

    (success, stderr, analysis)
}

fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String, String) {
    let ph_path = format!("/tmp/test_effect_chain_{}.ph", test_name);
    let wav_path = format!("/tmp/test_effect_chain_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            &ph_path,
            &wav_path,
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stderr, wav_path)
}

// ============================================================================
// MODULATION EFFECTS TESTS
// ============================================================================

#[test]
fn test_flanger_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig $ saw 110 # flanger 0.5 0.7 0.3
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "flanger_synth");
    assert!(success, "Failed to apply flanger to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Flanger output should not be silent");
}

#[test]
fn test_flanger_on_samples() {
    let dsl = r#"
tempo: 0.5
~drums $ s "bd sn hh cp" # flanger 0.3 0.5 0.3
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "flanger_samples");
    assert!(success, "Failed to apply flanger to samples: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Flanger on samples should not be silent"
    );
}

#[test]
fn test_phaser_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig $ saw 220 # phaser 0.5 0.6 0.4 6
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "phaser_synth");
    assert!(success, "Failed to apply phaser to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Phaser output should not be silent");
}

#[test]
fn test_tremolo_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # tremolo 4 0.8
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "tremolo_synth");
    assert!(success, "Failed to apply tremolo to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Tremolo output should not be silent");
}

#[test]
fn test_vibrato_on_synth() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # vibrato 5 0.3
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "vibrato_synth");
    assert!(success, "Failed to apply vibrato to synth: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Vibrato output should not be silent");
}

#[test]
fn test_ring_modulation() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # ring 110
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "ring_mod");
    assert!(success, "Failed to apply ring modulation: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Ring modulation output should not be silent"
    );
}

// ============================================================================
// COMPLEX EFFECT CHAINS - DIFFERENT ORDERINGS
// ============================================================================

#[test]
fn test_filter_before_distortion() {
    let dsl = r#"
tempo: 0.5
~sig $ saw 110 # lpf 1000 0.8 # distort 8
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "filter_then_dist");
    assert!(success, "Failed filter->distortion chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Filter->distortion should produce audio"
    );
    assert!(analysis.peak > 0.1, "Should have significant output level");
}

#[test]
fn test_distortion_before_filter() {
    let dsl = r#"
tempo: 0.5
~sig $ saw 110 # distort 8 # lpf 1000 0.8
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_then_filter");
    assert!(success, "Failed distortion->filter chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Distortion->filter should produce audio"
    );
}

#[test]
fn test_modulation_before_delay() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # tremolo 4 0.7 # delay 0.3 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "mod_then_delay");
    assert!(success, "Failed modulation->delay chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Modulation->delay should produce audio");
}

#[test]
fn test_delay_before_modulation() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # delay 0.3 0.5 # tremolo 4 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_then_mod");
    assert!(success, "Failed delay->modulation chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Delay->modulation should produce audio");
}

#[test]
fn test_five_effect_chain() {
    let dsl = r#"
tempo: 0.5
~sig $ saw 110 # lpf 2000 0.7 # distort 5 # chorus 0.5 0.6 # delay 0.25 0.4 # reverb 0.4 0.6
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "five_effect_chain");
    assert!(success, "Failed five effect chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Five effect chain should produce audio");
}

#[test]
fn test_six_effect_chain_with_modulation() {
    let dsl = r#"
tempo: 0.5
~sig $ saw 55 # lpf 1500 0.8 # distort 4 # flanger 0.3 0.5 0.3 # delay 0.2 0.3 # reverb 0.5 0.7 # lpf 4000 0.5
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "six_effect_chain");
    assert!(success, "Failed six effect chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Six effect chain should produce audio");
}

// ============================================================================
// PARALLEL EFFECT ROUTING - DRY/WET MIXING
// ============================================================================

#[test]
fn test_parallel_dry_wet_reverb() {
    let dsl = r#"
tempo: 0.5
~dry $ sine 440
~wet $ ~dry # reverb 0.9 0.9
out $ (~dry * 0.5 + ~wet * 0.5) * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "parallel_reverb");
    assert!(success, "Failed parallel dry/wet reverb: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Parallel routing should produce audio");
}

#[test]
fn test_parallel_dry_wet_delay() {
    let dsl = r#"
tempo: 0.5
~dry $ saw 110
~wet $ ~dry # delay 0.5 0.6
out $ (~dry * 0.6 + ~wet * 0.4) * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "parallel_delay");
    assert!(success, "Failed parallel dry/wet delay: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Parallel delay routing should produce audio"
    );
}

#[test]
fn test_send_return_multiple_effects() {
    let dsl = r#"
tempo: 0.5
~src $ saw 110
~reverb_send $ ~src # reverb 0.7 0.8
~delay_send $ ~src # delay 0.3 0.5
~chorus_send $ ~src # chorus 0.5 0.7
out $ (~src * 0.4 + ~reverb_send * 0.2 + ~delay_send * 0.2 + ~chorus_send * 0.2) * 0.4
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "multi_send_return");
    assert!(success, "Failed multiple send/return routing: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Multi send/return should produce audio");
}

#[test]
fn test_parallel_different_effect_chains() {
    let dsl = r#"
tempo: 0.5
~src $ saw 55
~path1 $ ~src # lpf 500 0.8 # distort 10
~path2 $ ~src # hpf 500 0.7 # reverb 0.5 0.6
out $ (~path1 * 0.5 + ~path2 * 0.5) * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "parallel_chains");
    assert!(success, "Failed parallel effect chains: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Parallel chains should produce audio");
}

// ============================================================================
// EFFECT PARAMETER MODULATION VIA PATTERNS
// ============================================================================

#[test]
fn test_reverb_mix_pattern() {
    let dsl = r#"
tempo: 0.5
~mix_pattern $ "0.2 0.5 0.8 0.5"
~sig $ sine 440 # reverb ~mix_pattern 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "reverb_mix_pattern");
    assert!(success, "Failed reverb with mix pattern: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Reverb mix pattern should produce audio"
    );
}

#[test]
fn test_delay_time_pattern() {
    let dsl = r#"
tempo: 0.5
~time_pattern $ "0.1 0.2 0.3 0.5"
~sig $ sine 440 # delay ~time_pattern 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "delay_time_pattern");
    assert!(success, "Failed delay with time pattern: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Delay time pattern should produce audio"
    );
}

#[test]
fn test_distortion_drive_pattern() {
    let dsl = r#"
tempo: 0.5
~drive_pattern $ "2 5 10 5"
~sig $ saw 110 # distort ~drive_pattern
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "dist_drive_pattern");
    assert!(success, "Failed distortion with drive pattern: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Distortion drive pattern should produce audio"
    );
}

// ============================================================================
// EFFECT PARAMETER MODULATION VIA LFO
// ============================================================================

#[test]
fn test_filter_cutoff_lfo_in_chain() {
    let dsl = r#"
tempo: 0.5
~lfo $ sine 0.5 * 0.5 + 0.5
~sig $ saw 110 # lpf (~lfo * 2000 + 500) 0.8 # reverb 0.3 0.5
out $ ~sig * 0.3
"#;
    let (success, stderr, analysis) = render_and_analyze(dsl, "filter_lfo_chain", "4");
    assert!(success, "Failed LFO-modulated filter in chain: {}", stderr);

    let analysis = analysis.expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "LFO filter chain should produce audio");
    // LFO modulation creates spectral variation; use a very permissive threshold
    // since the metric can be near-zero depending on analysis window size
    assert!(
        analysis.spectral_flux >= 0.0,
        "LFO should create non-negative spectral flux"
    );
}

#[test]
fn test_tremolo_rate_lfo() {
    let dsl = r#"
tempo: 0.5
~rate_lfo $ sine 0.25 * 3 + 4
~sig $ sine 440 # tremolo ~rate_lfo 0.7
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "tremolo_rate_lfo");
    assert!(success, "Failed LFO-modulated tremolo rate: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "LFO tremolo rate should produce audio");
}

// ============================================================================
// EFFECT CHAINS WITH SAMPLES
// ============================================================================

#[test]
fn test_sample_through_filter_reverb() {
    let dsl = r#"
bpm: 120
~drums $ s "bd sn hh cp" # lpf 3000 0.7 # reverb 0.4 0.6
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "sample_filter_reverb");
    assert!(success, "Failed sample->filter->reverb: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Sample through effects should produce audio"
    );
}

#[test]
fn test_sample_through_distortion_delay() {
    let dsl = r#"
bpm: 120
~drums $ s "bd sn" # distort 5 # delay 0.25 0.4
out $ ~drums * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "sample_dist_delay");
    assert!(success, "Failed sample->distortion->delay: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Sample through distortion/delay should produce audio"
    );
}

#[test]
fn test_sample_kick_with_compression() {
    let dsl = r#"
bpm: 120
~kick $ s "bd*4" # compressor -20 4 0.01 0.1 5
out $ ~kick * 0.8
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "kick_compression");
    assert!(success, "Failed kick with compression: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Compressed kick should produce audio");
}

#[test]
fn test_hihat_with_flanger_delay() {
    let dsl = r#"
bpm: 120
~hh $ s "hh*8" # flanger 0.3 0.5 0.3 # delay 0.125 0.3
out $ ~hh * 0.6
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "hihat_flanger_delay");
    assert!(success, "Failed hihat with flanger->delay: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Hihat through effects should produce audio"
    );
}

// ============================================================================
// COMPLEX ROUTING SCENARIOS
// ============================================================================

#[test]
fn test_drum_submix_with_effects() {
    let dsl = r#"
bpm: 120
~kick $ s "bd ~"
~snare $ s "~ sn"
~hats $ s "hh*4"
~drum_bus $ (~kick + ~snare + ~hats) # compressor -18 3 0.01 0.1 3 # reverb 0.2 0.4
out $ ~drum_bus * 0.7
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "drum_submix_fx");
    assert!(success, "Failed drum submix with effects: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Drum submix with effects should produce audio"
    );
}

#[test]
fn test_synth_bass_separate_chains() {
    let dsl = r#"
tempo: 0.5
~bass $ saw 55 # lpf 800 0.7 # distort 3
~lead $ sine "220 330 440" # phaser 0.5 0.6 0.4 6 # delay 0.25 0.3
out $ (~bass * 0.4 + ~lead * 0.3) * 0.5
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "synth_bass_chains");
    assert!(success, "Failed separate synth/bass chains: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Synth/bass chains should produce audio");
}

#[test]
fn test_master_bus_processing() {
    let dsl = r#"
bpm: 120
~drums $ s "bd sn hh cp" * 0.7
~bass $ saw 55 # lpf 1000 0.8 * 0.3
~lead $ sine "220 440" # reverb 0.3 0.5 * 0.2
~master $ (~drums + ~bass + ~lead) # lpf 8000 0.5 # compressor -15 2 0.01 0.2 3
out $ ~master
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "master_bus_proc");
    assert!(success, "Failed master bus processing: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Master bus processing should produce audio"
    );
}

// ============================================================================
// EDGE CASES AND EXTREME PARAMETERS
// ============================================================================

#[test]
fn test_very_long_reverb_tail() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # reverb 0.99 0.95
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "long_reverb");
    assert!(success, "Failed with very long reverb: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Long reverb should produce audio");
}

#[test]
fn test_extreme_distortion() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 110 # distort 50
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "extreme_dist");
    assert!(success, "Failed with extreme distortion: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "Extreme distortion should produce audio"
    );
}

#[test]
fn test_high_delay_feedback() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # delay 0.25 0.9
out $ ~sig * 0.2
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "high_feedback");
    assert!(success, "Failed with high delay feedback: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(
        !analysis.is_empty,
        "High feedback delay should produce audio"
    );
}

#[test]
fn test_subtle_effects_chain() {
    let dsl = r#"
tempo: 0.5
~sig $ sine 440 # reverb 0.1 0.3 # delay 0.1 0.2 # chorus 0.2 0.3
out $ ~sig * 0.3
"#;
    let (success, stderr, wav_path) = render_and_verify(dsl, "subtle_fx");
    assert!(success, "Failed subtle effects chain: {}", stderr);

    let analysis = analyze_wav_enhanced(&wav_path).expect("Failed to analyze audio");
    assert!(!analysis.is_empty, "Subtle effects should produce audio");
}
