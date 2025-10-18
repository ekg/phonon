/// End-to-end tests for sample playback DSL syntax
/// Tests s() function, mini-notation, and sample operations

use std::process::Command;
use std::fs;

fn render_and_verify(dsl_code: &str, test_name: &str) -> (bool, String) {
    let ph_path = format!("/tmp/test_sample_{}.ph", test_name);
    let wav_path = format!("/tmp/test_sample_{}.wav", test_name);

    fs::write(&ph_path, dsl_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", &ph_path, &wav_path, "--duration", "2"])
        .output()
        .expect("Failed to run phonon render");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stderr)
}

// ============================================================================
// BASIC SAMPLE PLAYBACK TESTS - Individual sample types
// ============================================================================

#[test]
fn test_kick_drum_only() {
    let dsl = r#"
tempo: 2.0
out: s "bd" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "bd_only");
    assert!(success, "Failed to render kick drum: {}", stderr);
}

#[test]
fn test_snare_drum_only() {
    let dsl = r#"
tempo: 2.0
out: s "sn" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "sn_only");
    assert!(success, "Failed to render snare: {}", stderr);
}

#[test]
fn test_hihat_only() {
    let dsl = r#"
tempo: 2.0
out: s "hh" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "hh_only");
    assert!(success, "Failed to render hihat: {}", stderr);
}

#[test]
fn test_clap_only() {
    let dsl = r#"
tempo: 2.0
out: s "cp" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "cp_only");
    assert!(success, "Failed to render clap: {}", stderr);
}

#[test]
fn test_open_hihat_only() {
    let dsl = r#"
tempo: 2.0
out: s "oh" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "oh_only");
    assert!(success, "Failed to render open hihat: {}", stderr);
}

// ============================================================================
// BASIC DRUM PATTERNS - Simple sequences
// ============================================================================

#[test]
fn test_kick_snare_pattern() {
    let dsl = r#"
tempo: 2.0
out: s "bd sn" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "bd_sn");
    assert!(success, "Failed to render bd sn pattern: {}", stderr);
}

#[test]
fn test_four_on_floor() {
    let dsl = r#"
tempo: 2.0
out: s "bd bd bd bd" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "four_on_floor");
    assert!(success, "Failed to render 4/4 kick pattern: {}", stderr);
}

#[test]
fn test_basic_house_beat() {
    let dsl = r#"
tempo: 2.0
out: s "bd sn bd sn" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "house_beat");
    assert!(success, "Failed to render house beat: {}", stderr);
}

#[test]
fn test_complete_drum_kit() {
    let dsl = r#"
tempo: 2.0
out: s "bd sn hh cp" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "complete_kit");
    assert!(success, "Failed to render complete drum kit: {}", stderr);
}

#[test]
fn test_eight_step_pattern() {
    let dsl = r#"
tempo: 2.0
out: s "bd hh sn hh bd hh sn hh" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "eight_steps");
    assert!(success, "Failed to render 8-step pattern: {}", stderr);
}

// ============================================================================
// SAMPLE PATTERN WITH RESTS
// ============================================================================

#[test]
fn test_samples_with_rests() {
    let dsl = r#"
tempo: 2.0
out: s "bd ~ sn ~" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "with_rests");
    assert!(success, "Failed to render samples with rests: {}", stderr);
}

#[test]
fn test_kick_with_rests() {
    let dsl = r#"
tempo: 2.0
out: s "bd ~ ~ ~" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "kick_sparse");
    assert!(success, "Failed to render sparse kick: {}", stderr);
}

#[test]
fn test_alternating_rest() {
    let dsl = r#"
tempo: 2.0
out: s "bd ~ sn ~ hh ~ cp ~" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "alternating_rest");
    assert!(success, "Failed to render alternating rest pattern: {}", stderr);
}

#[test]
fn test_multiple_consecutive_rests() {
    let dsl = r#"
tempo: 2.0
out: s "bd ~ ~ ~ sn" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "consecutive_rests");
    assert!(success, "Failed to render consecutive rests: {}", stderr);
}

// ============================================================================
// SAMPLE SUBDIVISION TESTS - Using * operator
// ============================================================================

#[test]
fn test_hihat_subdivision_2x() {
    let dsl = r#"
tempo: 2.0
out: s "bd hh*2" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "hh_2x");
    assert!(success, "Failed to render 2x hihat subdivision: {}", stderr);
}

#[test]
fn test_hihat_subdivision_4x() {
    let dsl = r#"
tempo: 2.0
out: s "bd hh*4" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "hh_4x");
    assert!(success, "Failed to render 4x hihat subdivision: {}", stderr);
}

#[test]
fn test_hihat_subdivision_8x() {
    let dsl = r#"
tempo: 2.0
out: s "bd hh*8" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "hh_8x");
    assert!(success, "Failed to render 8x hihat subdivision: {}", stderr);
}

#[test]
fn test_complex_subdivision_pattern() {
    let dsl = r#"
tempo: 2.0
out: s "bd*2 sn hh*4 cp" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "complex_subdiv");
    assert!(success, "Failed to render complex subdivision: {}", stderr);
}

#[test]
fn test_all_subdivided() {
    let dsl = r#"
tempo: 2.0
out: s "bd*2 sn*2 hh*2 cp*2" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "all_subdiv");
    assert!(success, "Failed to render all subdivided: {}", stderr);
}

// ============================================================================
// SAMPLE ALTERNATION TESTS - Using < > brackets
// ============================================================================

#[test]
fn test_kick_alternation() {
    let dsl = r#"
tempo: 2.0
out: s "<bd cp>" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "kick_alt");
    assert!(success, "Failed to render kick alternation: {}", stderr);
}

#[test]
fn test_snare_alternation_three() {
    let dsl = r#"
tempo: 2.0
out: s "bd <sn cp hh>" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "snare_alt_3");
    assert!(success, "Failed to render 3-way alternation: {}", stderr);
}

#[test]
fn test_complex_alternation() {
    let dsl = r#"
tempo: 2.0
out: s "<bd sn> <hh cp>" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "complex_alt");
    assert!(success, "Failed to render complex alternation: {}", stderr);
}

#[test]
fn test_alternation_with_subdivision() {
    let dsl = r#"
tempo: 2.0
out: s "bd <hh*2 cp>" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "alt_with_subdiv");
    assert!(success, "Failed to render alternation with subdivision: {}", stderr);
}

// ============================================================================
// EUCLIDEAN RHYTHM TESTS WITH SAMPLES
// ============================================================================

#[test]
fn test_euclidean_3_8_kick() {
    let dsl = r#"
tempo: 2.0
out: s "(3,8,bd)" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_3_8_bd");
    assert!(success, "Failed to render euclidean (3,8) with bd: {}", stderr);
}

#[test]
fn test_euclidean_5_8_hihat() {
    let dsl = r#"
tempo: 2.0
out: s "(5,8,hh)" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_5_8_hh");
    assert!(success, "Failed to render euclidean (5,8) with hh: {}", stderr);
}

#[test]
fn test_euclidean_3_4_snare() {
    let dsl = r#"
tempo: 2.0
out: s "(3,4,sn)" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_3_4_sn");
    assert!(success, "Failed to render euclidean (3,4) with sn: {}", stderr);
}

#[test]
fn test_euclidean_7_16_kick() {
    let dsl = r#"
tempo: 2.0
out: s "(7,16,bd)" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "euclid_7_16_bd");
    assert!(success, "Failed to render euclidean (7,16) with bd: {}", stderr);
}

// ============================================================================
// SAMPLE TRANSFORMS - Using $ operator
// ============================================================================

#[test]
fn test_samples_fast_transform() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn"
out: (~drums $ fast 2) * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "fast_drums");
    assert!(success, "Failed to apply fast to samples: {}", stderr);
}

#[test]
fn test_samples_slow_transform() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh cp"
out: (~drums $ slow 2) * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "slow_drums");
    assert!(success, "Failed to apply slow to samples: {}", stderr);
}

#[test]
fn test_samples_rev_transform() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh cp"
out: (~drums $ rev) * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "rev_drums");
    assert!(success, "Failed to apply rev to samples: {}", stderr);
}

#[test]
fn test_samples_every_transform() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn"
out: (~drums $ every 2 rev) * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "every_drums");
    assert!(success, "Failed to apply every to samples: {}", stderr);
}

#[test]
fn test_samples_chained_transforms() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh"
out: (~drums $ fast 2 $ rev) * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "chained_drums");
    assert!(success, "Failed to chain transforms on samples: {}", stderr);
}

// ============================================================================
// SAMPLES THROUGH FILTERS
// ============================================================================

#[test]
fn test_samples_through_lpf() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh*4 cp" # lpf 2000 0.8
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "lpf_drums");
    assert!(success, "Failed to filter samples with lpf: {}", stderr);
}

#[test]
fn test_samples_through_hpf() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh*4 cp" # hpf 500 0.7
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "hpf_drums");
    assert!(success, "Failed to filter samples with hpf: {}", stderr);
}

#[test]
fn test_samples_through_bpf() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh*4 cp" # bpf 1000 0.8
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "bpf_drums");
    assert!(success, "Failed to filter samples with bpf: {}", stderr);
}

#[test]
fn test_samples_lfo_filter() {
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.5 * 0.5 + 0.5
~drums: s "bd sn hh*4" # lpf (~lfo * 2000 + 500) 0.8
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "lfo_filter_drums");
    assert!(success, "Failed LFO-filtered samples: {}", stderr);
}

#[test]
fn test_samples_pattern_filter() {
    let dsl = r#"
tempo: 2.0
~cutoff: "1000 2000 3000"
~drums: s "bd sn hh" # lpf ~cutoff 0.7
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_filter_drums");
    assert!(success, "Failed pattern-filtered samples: {}", stderr);
}

// ============================================================================
// MULTIPLE SAMPLE PATTERNS MIXED
// ============================================================================

#[test]
fn test_two_sample_patterns_mixed() {
    let dsl = r#"
tempo: 2.0
~kicks: s "bd ~ bd ~"
~snares: s "~ sn ~ sn"
out: (~kicks + ~snares) * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "two_patterns");
    assert!(success, "Failed to mix two sample patterns: {}", stderr);
}

#[test]
fn test_three_sample_patterns_mixed() {
    let dsl = r#"
tempo: 2.0
~kicks: s "bd ~ bd ~"
~snares: s "~ sn ~ sn"
~hats: s "hh*8"
out: (~kicks + ~snares + ~hats) * 0.6
"#;
    let (success, stderr) = render_and_verify(dsl, "three_patterns");
    assert!(success, "Failed to mix three sample patterns: {}", stderr);
}

#[test]
fn test_layered_drums() {
    let dsl = r#"
tempo: 2.0
~layer1: s "bd sn"
~layer2: s "hh*4"
~layer3: s "~ cp"
out: (~layer1 + ~layer2 * 0.7 + ~layer3 * 0.8) * 0.7
"#;
    let (success, stderr) = render_and_verify(dsl, "layered_drums");
    assert!(success, "Failed to create layered drums: {}", stderr);
}

// ============================================================================
// SAMPLES MIXED WITH SYNTHESIS
// ============================================================================

#[test]
fn test_samples_with_bass() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 * 0.3
~drums: s "bd sn hh*4 cp"
out: ~bass + ~drums * 0.6
"#;
    let (success, stderr) = render_and_verify(dsl, "drums_with_bass");
    assert!(success, "Failed to mix samples with bass: {}", stderr);
}

#[test]
fn test_samples_with_melody() {
    let dsl = r#"
tempo: 2.0
~melody: sine "220 330 440 550" * 0.2
~drums: s "bd sn hh cp"
out: ~melody + ~drums * 0.6
"#;
    let (success, stderr) = render_and_verify(dsl, "drums_with_melody");
    assert!(success, "Failed to mix samples with melody: {}", stderr);
}

#[test]
fn test_complete_track_with_samples() {
    let dsl = r#"
tempo: 2.0
~bass: saw 55 * 0.3
~melody: sine "220 440" * 0.1
~drums: s "bd sn hh*4 cp"
out: ~bass + ~melody + ~drums * 0.6
"#;
    let (success, stderr) = render_and_verify(dsl, "complete_track");
    assert!(success, "Failed to create complete track: {}", stderr);
}

// ============================================================================
// AMPLITUDE VARIATION TESTS
// ============================================================================

#[test]
fn test_samples_quiet() {
    let dsl = r#"
tempo: 2.0
out: s "bd sn hh cp" * 0.3
"#;
    let (success, stderr) = render_and_verify(dsl, "quiet_samples");
    assert!(success, "Failed to render quiet samples: {}", stderr);
}

#[test]
fn test_samples_loud() {
    let dsl = r#"
tempo: 2.0
out: s "bd sn hh cp" * 1.0
"#;
    let (success, stderr) = render_and_verify(dsl, "loud_samples");
    assert!(success, "Failed to render loud samples: {}", stderr);
}

#[test]
fn test_samples_pattern_amplitude() {
    let dsl = r#"
tempo: 2.0
~amp: "0.5 1.0 0.7 0.9"
out: s "bd sn hh cp" * ~amp
"#;
    let (success, stderr) = render_and_verify(dsl, "pattern_amp_samples");
    assert!(success, "Failed to apply pattern amplitude to samples: {}", stderr);
}

// ============================================================================
// TEMPO VARIATION TESTS
// ============================================================================

#[test]
fn test_samples_slow_tempo() {
    let dsl = r#"
tempo: 0.5
out: s "bd sn hh cp" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "slow_tempo");
    assert!(success, "Failed samples at slow tempo: {}", stderr);
}

#[test]
fn test_samples_fast_tempo() {
    let dsl = r#"
tempo: 4.0
out: s "bd sn hh cp" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "fast_tempo");
    assert!(success, "Failed samples at fast tempo: {}", stderr);
}

#[test]
fn test_samples_very_slow_tempo() {
    let dsl = r#"
tempo: 0.25
out: s "bd sn" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "very_slow_tempo");
    assert!(success, "Failed samples at very slow tempo: {}", stderr);
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_very_long_sample_pattern() {
    let dsl = r#"
tempo: 2.0
out: s "bd sn hh cp bd sn hh oh bd sn hh cp bd sn hh oh" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "very_long");
    assert!(success, "Failed with very long sample pattern: {}", stderr);
}

#[test]
fn test_all_same_sample() {
    let dsl = r#"
tempo: 2.0
out: s "bd bd bd bd" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "all_same");
    assert!(success, "Failed with all same sample: {}", stderr);
}

#[test]
fn test_extreme_subdivision() {
    let dsl = r#"
tempo: 2.0
out: s "bd hh*16" * 0.6
"#;
    let (success, stderr) = render_and_verify(dsl, "extreme_subdiv");
    assert!(success, "Failed with extreme subdivision: {}", stderr);
}

#[test]
fn test_mostly_rests() {
    let dsl = r#"
tempo: 2.0
out: s "bd ~ ~ ~ ~ ~ ~ ~" * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "mostly_rests");
    assert!(success, "Failed with mostly rests: {}", stderr);
}

// ============================================================================
// BUS ROUTING TESTS
// ============================================================================

#[test]
fn test_samples_through_bus() {
    let dsl = r#"
tempo: 2.0
~drums: s "bd sn hh cp"
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "bus_routing");
    assert!(success, "Failed to route samples through bus: {}", stderr);
}

#[test]
fn test_multiple_sample_buses() {
    let dsl = r#"
tempo: 2.0
~bus1: s "bd ~"
~bus2: s "~ sn"
~bus3: s "hh*8"
out: ~bus1 + ~bus2 + ~bus3 * 0.7
"#;
    let (success, stderr) = render_and_verify(dsl, "multi_bus_samples");
    assert!(success, "Failed multiple sample buses: {}", stderr);
}

#[test]
fn test_nested_sample_bus() {
    let dsl = r#"
tempo: 2.0
~kicks: s "bd ~ bd ~"
~snares: s "~ sn ~ sn"
~drums: ~kicks + ~snares
out: ~drums * 0.8
"#;
    let (success, stderr) = render_and_verify(dsl, "nested_bus_samples");
    assert!(success, "Failed nested sample bus: {}", stderr);
}
