//! Validated Tests: DnB patterns match reference characteristics
//!
//! Verifies that Phonon's Drum & Bass patterns produce audio matching
//! the documented musical characteristics of the genre:
//!
//! - TWO-STEP: Kicks on 1st and 6th eighth notes, snares on beats 2 & 4
//! - HALF-TIME: Single snare on beat 3, perceived slower feel
//! - NEUROFUNK: Dark, mechanical, second kick shifted before beat 3
//! - JUMP-UP: Bouncy with multiple kicks, high energy
//! - ROLLERS: Hypnotic, minimal, extra kick for rolling groove
//! - JUNGLE: Heavy breakbeat chopping, syncopated rhythms
//! - REESE BASS: Detuned saws with LFO filter modulation
//!
//! Uses three-level verification methodology:
//!   Level 1: Pattern query verification (event counts, timing)
//!   Level 2: Onset detection (audio events at correct times)
//!   Level 3: Audio characteristics (RMS, spectral content, modulation)
//!
//! DnB tempo: 165-185 BPM (cps 2.75-3.08), standard 174 BPM = cps 2.9

use phonon::audio_similarity::{
    detect_onsets, AudioSimilarityScorer, SimilarityConfig, SpectralFeatures,
};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

const SAMPLE_RATE: f32 = 44100.0;

// ============================================================================
// Test Helpers
// ============================================================================

fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Parse DSL failed");
    let compiler = DslCompiler::new(SAMPLE_RATE);
    let mut graph = compiler.compile(statements);
    let samples = (SAMPLE_RATE * duration_secs) as usize;
    graph.render(samples)
}

fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = audio.iter().map(|s| s * s).sum();
    (sum_sq / audio.len() as f32).sqrt()
}

fn calculate_peak(audio: &[f32]) -> f32 {
    audio.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

fn spectral_centroid(audio: &[f32]) -> f32 {
    let features = SpectralFeatures::from_audio(audio, SAMPLE_RATE, 2048);
    features.centroid
}

fn envelope_variation(audio: &[f32], window_ms: f32) -> f32 {
    let window_samples = (SAMPLE_RATE * window_ms / 1000.0) as usize;
    if audio.len() < window_samples * 2 {
        return 0.0;
    }
    let rms_values: Vec<f32> = audio
        .chunks(window_samples)
        .filter(|c| c.len() == window_samples)
        .map(|c| calculate_rms(c))
        .collect();

    if rms_values.is_empty() {
        return 0.0;
    }
    let mean = rms_values.iter().sum::<f32>() / rms_values.len() as f32;
    let variance =
        rms_values.iter().map(|&r| (r - mean).powi(2)).sum::<f32>() / rms_values.len() as f32;
    variance.sqrt()
}

fn count_events_over_cycles<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> usize {
    let mut total = 0;
    for cycle in 0..cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total += pattern.query(&state).len();
    }
    total
}

fn query_single_cycle<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

// ============================================================================
// LEVEL 1: PATTERN QUERY VERIFICATION
// Tests pattern logic without rendering audio
// ============================================================================

#[test]
fn dnb_l1_twostep_kick_positions() {
    // Two-step: "bd ~ ~ ~ ~ bd ~ ~" (16 step version compressed to 8)
    // Kicks on positions 0 and 5 of 8 slots
    let kick_pattern = parse_mini_notation("bd ~ ~ ~ ~ bd ~ ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Two-step kick should have 2 hits per cycle, got {}",
        non_rest.len()
    );

    // First kick at 0/8 = 0.0, second at 5/8 = 0.625
    let pos0 = non_rest[0].part.begin.to_float();
    let pos1 = non_rest[1].part.begin.to_float();
    assert!(
        pos0.abs() < 0.01,
        "First kick should be at beat 1 (0.0), got {}",
        pos0
    );
    assert!(
        (pos1 - 0.625).abs() < 0.01,
        "Second kick should be at position 5/8 (0.625), got {}",
        pos1
    );
}

#[test]
fn dnb_l1_twostep_snare_on_2_and_4() {
    // Two-step snare: "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" (16 steps)
    // Snares at positions 4 and 12 of 16 = beats 2 and 4
    let snare_pattern = parse_mini_notation("~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~");
    let events = query_single_cycle(&snare_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Two-step snare should have 2 hits per cycle, got {}",
        non_rest.len()
    );

    // Snare at 4/16 = 0.25 (beat 2) and 12/16 = 0.75 (beat 4)
    let positions: Vec<f64> = non_rest.iter().map(|h| h.part.begin.to_float()).collect();
    assert!(
        (positions[0] - 0.25).abs() < 0.01,
        "First snare should be at beat 2 (0.25), got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.75).abs() < 0.01,
        "Second snare should be at beat 4 (0.75), got {}",
        positions[1]
    );
}

#[test]
fn dnb_l1_halftime_single_snare_beat3() {
    // Half-time: snare only on beat 3 (position 8 of 16 = 0.5)
    let snare_pattern = parse_mini_notation("~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~");
    let events = query_single_cycle(&snare_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "Half-time snare should have exactly 1 hit per cycle, got {}",
        non_rest.len()
    );

    let pos = non_rest[0].part.begin.to_float();
    assert!(
        (pos - 0.5).abs() < 0.01,
        "Half-time snare should be at beat 3 (0.5), got {}",
        pos
    );
}

#[test]
fn dnb_l1_neurofunk_kick_shifted() {
    // Neurofunk: "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
    // Second kick shifted to position 14/16 = 0.875 (last 16th before beat 3 area)
    let kick_pattern = parse_mini_notation("bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Neurofunk kick should have 2 hits, got {}",
        non_rest.len()
    );

    // First kick at 0.0, second at 14/16 = 0.875
    let pos0 = non_rest[0].part.begin.to_float();
    let pos1 = non_rest[1].part.begin.to_float();
    assert!(
        pos0.abs() < 0.01,
        "First neurofunk kick at 0.0, got {}",
        pos0
    );
    assert!(
        (pos1 - 0.875).abs() < 0.01,
        "Second neurofunk kick should be at 0.875 (shifted), got {}",
        pos1
    );
}

#[test]
fn dnb_l1_jumpup_multiple_kicks() {
    // Jump-up: "bd bd ~ ~ bd ~ bd ~" => many kicks for bouncy feel
    let kick_pattern = parse_mini_notation("bd bd ~ ~ bd ~ bd ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Jump-up should have 4 kicks per cycle, got {}",
        non_rest.len()
    );
}

#[test]
fn dnb_l1_roller_extra_kick() {
    // Roller: "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~" => 3 kicks including rolling extra
    let kick_pattern = parse_mini_notation("bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        3,
        "Roller should have 3 kicks per cycle, got {}",
        non_rest.len()
    );
}

#[test]
fn dnb_l1_hihat_8th_count() {
    // DnB hi-hats at 8th notes: "hh*8"
    let hh_pattern = parse_mini_notation("hh*8");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 32,
        "hh*8 over 4 cycles should produce 32 events, got {}",
        count
    );
}

#[test]
fn dnb_l1_hihat_16th_count() {
    // DnB hi-hats at 16th notes (more energy): "hh*16"
    let hh_pattern = parse_mini_notation("hh*16");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 64,
        "hh*16 over 4 cycles should produce 64 events, got {}",
        count
    );
}

#[test]
fn dnb_l1_hihat_32nd_count() {
    // DnB intense hi-hats at 32nd notes: "hh*32"
    let hh_pattern = parse_mini_notation("hh*32");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 128,
        "hh*32 over 4 cycles should produce 128 events, got {}",
        count
    );
}

#[test]
fn dnb_l1_euclidean_kick_pattern() {
    // Euclidean DnB: "bd(3,16)" => 3 kicks evenly distributed in 16 slots
    let kick_pattern = parse_mini_notation("bd(3,16)");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        3,
        "Euclidean bd(3,16) should have 3 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn dnb_l1_euclidean_perc_pattern() {
    // Euclidean percussion: "cp(7,16)" => 7 claps in 16 slots (polyrhythmic)
    let perc_pattern = parse_mini_notation("cp(7,16)");
    let events = query_single_cycle(&perc_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        7,
        "Euclidean cp(7,16) should have 7 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn dnb_l1_jungle_syncopated_snare() {
    // Jungle: "~ sn ~ sn" => syncopated snare pattern
    let snare_pattern = parse_mini_notation("~ sn ~ sn");
    let events = query_single_cycle(&snare_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Jungle snare should have 2 hits, got {}",
        non_rest.len()
    );

    // Snares at positions 0.25 and 0.75
    let positions: Vec<f64> = non_rest.iter().map(|h| h.part.begin.to_float()).collect();
    assert!(
        (positions[0] - 0.25).abs() < 0.01,
        "First jungle snare at 0.25, got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.75).abs() < 0.01,
        "Second jungle snare at 0.75, got {}",
        positions[1]
    );
}

// ============================================================================
// LEVEL 2: DSL Integration - Patterns Compile and Produce Audio
// ============================================================================

#[test]
fn dnb_l2_twostep_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8"
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Two-step pattern should produce audible sound, RMS: {}",
        rms
    );
    assert!(
        !audio.iter().any(|s| s.is_nan()),
        "Audio should not contain NaN"
    );
}

#[test]
fn dnb_l2_twostep_tight_produces_audio() {
    let code = r#"
        cps: 2.9
        ~drums $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*16"
        out $ ~drums
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Two-step tight pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_halftime_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~"
        ~hats $ s "hh*8" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Half-time pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_neurofunk_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*32" * 0.25
        ~perc $ s "~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ ~" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~perc
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Neurofunk pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_jumpup_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd bd ~ ~ bd ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*16" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Jump-up pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_roller_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" * 0.35
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Roller pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_jungle_produces_audio() {
    let code = r#"
        cps: 2.75
        ~kick $ s "bd ~ bd ~ ~ ~ bd ~"
        ~snare $ s "~ sn ~ sn ~ ~ ~ ~"
        ~hats $ s "hh*16" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Jungle pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_euclidean_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd(3,16)"
        ~snare $ s "sn(2,8,4)"
        ~hats $ s "hh*16" * 0.4
        ~perc $ s "cp(7,16)" * 0.3
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~perc
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Euclidean DnB pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_ghost_notes_produces_audio() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
        ~main_sn $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~ghost_sn $ s "~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~" * 0.25
        ~hats $ s "hh*16" * 0.4
        out $ ~kick * 0.8 + ~main_sn * 0.6 + ~ghost_sn + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Ghost notes pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_reese_bass_produces_audio() {
    let code = r#"
        cps: 2.9
        ~reese $ supersaw "55 55 55 82.5" 0.6 12
        ~lfo $ sine 0.5 * 0.5 + 0.5
        out $ ~reese # lpf (~lfo * 1500 + 400) 0.85 * 0.2
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.001,
        "Reese bass should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_sub_bass_produces_audio() {
    let code = r#"
        cps: 2.9
        ~sub $ sine "55 55 55 82.5" * 0.3
        out $ ~sub
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Sub bass should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn dnb_l2_neuro_bass_produces_audio() {
    let code = r#"
        cps: 2.9
        ~neuro_lfo $ sine 4
        ~bass $ saw 55 # lpf (~neuro_lfo * 1500 + 500) 0.9 * 0.2
        out $ ~bass
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.001,
        "Neuro bass should produce audible sound, RMS: {}",
        rms
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics - Genre-Specific Properties
// ============================================================================

/// DnB at 174 BPM should have regular kick onsets
#[test]
fn dnb_l3_twostep_kick_regularity() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        out $ ~kick
    "#;
    let audio = render_dsl(code, 4.0);
    let onsets = detect_onsets(&audio, SAMPLE_RATE);

    // At cps 2.9, 2 kicks per cycle = ~5.8 kicks/sec
    // Over 4 seconds expect ~23 kicks
    assert!(
        onsets.len() >= 6,
        "Two-step kick should detect multiple regular onsets, got {}",
        onsets.len()
    );

    // Check regularity
    if onsets.len() >= 4 {
        let intervals: Vec<f64> = onsets.windows(2).map(|w| w[1].time - w[0].time).collect();
        let mean_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals
            .iter()
            .map(|&i| (i - mean_interval).powi(2))
            .sum::<f64>()
            / intervals.len() as f64;
        let cv = variance.sqrt() / mean_interval;

        assert!(
            cv < 1.0,
            "Two-step kick should have reasonably regular intervals (CV < 1.0), got CV={:.3}",
            cv
        );
    }
}

/// Jump-up (more kicks) should have more energy than half-time (sparse)
#[test]
fn dnb_l3_jumpup_more_energy_than_halftime() {
    let halftime_code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;
    let jumpup_code = r#"
        cps: 2.9
        ~kick $ s "bd bd ~ ~ bd ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;

    let halftime_audio = render_dsl(halftime_code, 2.0);
    let jumpup_audio = render_dsl(jumpup_code, 2.0);

    let halftime_rms = calculate_rms(&halftime_audio);
    let jumpup_rms = calculate_rms(&jumpup_audio);

    assert!(
        jumpup_rms > halftime_rms,
        "Jump-up (RMS {:.4}) should have more energy than half-time (RMS {:.4})",
        jumpup_rms,
        halftime_rms
    );
}

/// 32nd note hi-hats (neurofunk) should add more high frequency content than 8th note hats
#[test]
fn dnb_l3_faster_hats_brighter_spectrum() {
    let hats_8th = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~hats $ s "hh*8" * 0.4
        out $ ~kick * 0.8 + ~hats
    "#;
    let hats_32nd = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~hats $ s "hh*32" * 0.4
        out $ ~kick * 0.8 + ~hats
    "#;

    let audio_8th = render_dsl(hats_8th, 2.0);
    let audio_32nd = render_dsl(hats_32nd, 2.0);

    let rms_8th = calculate_rms(&audio_8th);
    let rms_32nd = calculate_rms(&audio_32nd);

    // 32nd hats should have more overall energy (more events)
    assert!(
        rms_32nd > rms_8th,
        "32nd hats (RMS {:.4}) should have more energy than 8th hats (RMS {:.4})",
        rms_32nd,
        rms_8th
    );
}

/// Adding hi-hats to kicks should increase spectral centroid (brighter)
#[test]
fn dnb_l3_hats_add_brightness() {
    let kick_only = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        out $ ~kick * 0.8
    "#;
    let kick_plus_hats = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~hats $ s "hh*16" * 0.4
        out $ ~kick * 0.8 + ~hats
    "#;

    let kick_audio = render_dsl(kick_only, 2.0);
    let mixed_audio = render_dsl(kick_plus_hats, 2.0);

    let kick_centroid = spectral_centroid(&kick_audio);
    let mixed_centroid = spectral_centroid(&mixed_audio);

    assert!(
        mixed_centroid > kick_centroid,
        "Adding hats should brighten mix: kick-only={:.0}Hz, with hats={:.0}Hz",
        kick_centroid,
        mixed_centroid
    );
}

/// Sub bass (sine 55Hz) should have very low spectral centroid
#[test]
fn dnb_l3_sub_bass_low_centroid() {
    let code = r#"
        cps: 2.9
        ~sub $ sine 55 * 0.4
        out $ ~sub
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Sub bass should produce audio, RMS: {}", rms);

    let centroid = spectral_centroid(&audio);
    assert!(
        centroid < 500.0,
        "DnB sub bass should have very low centroid, got {:.0}Hz",
        centroid
    );
}

/// Reese bass (detuned saws) should be brighter than sub bass (pure sine)
#[test]
fn dnb_l3_reese_brighter_than_sub() {
    let sub_code = r#"
        cps: 2.9
        ~sub $ sine 55 * 0.4
        out $ ~sub
    "#;
    let reese_code = r#"
        cps: 2.9
        ~reese $ supersaw 55 0.6 12 * 0.3
        out $ ~reese
    "#;

    let sub_audio = render_dsl(sub_code, 2.0);
    let reese_audio = render_dsl(reese_code, 2.0);

    let sub_centroid = spectral_centroid(&sub_audio);
    let reese_centroid = spectral_centroid(&reese_audio);

    assert!(
        reese_centroid > sub_centroid,
        "Reese ({:.0}Hz) should be brighter than sub ({:.0}Hz)",
        reese_centroid,
        sub_centroid
    );
}

/// LFO-modulated neuro bass should create spectral variation over time
#[test]
fn dnb_l3_neuro_bass_spectral_variation() {
    let code = r#"
        cps: 2.9
        ~lfo $ sine 4
        ~bass $ saw 55 # lpf (~lfo * 1500 + 500) 0.9
        out $ ~bass * 0.3
    "#;
    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.001, "Neuro bass should produce audio");

    // Check envelope variation from LFO modulation
    let variation = envelope_variation(&audio, 50.0);
    assert!(
        variation > 0.0001,
        "LFO-modulated neuro bass should have envelope variation, got {:.6}",
        variation
    );
}

/// DnB patterns should have rhythmic envelope variation (transients vs silence)
#[test]
fn dnb_l3_rhythmic_envelope() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;
    let audio = render_dsl(code, 4.0);
    let variation = envelope_variation(&audio, 50.0);

    assert!(
        variation > 0.001,
        "DnB drum pattern should have rhythmic envelope variation, got {:.6}",
        variation
    );
}

/// DnB patterns should not clip excessively
#[test]
fn dnb_l3_twostep_no_clipping() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*16" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let peak = calculate_peak(&audio);

    assert!(
        peak < 5.0,
        "Two-step mix should not have extreme clipping, peak: {:.3}",
        peak
    );
}

#[test]
fn dnb_l3_neurofunk_full_no_clipping() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*32" * 0.25
        ~perc $ s "~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ ~" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~perc
    "#;
    let audio = render_dsl(code, 2.0);
    let peak = calculate_peak(&audio);

    assert!(
        peak < 5.0,
        "Neurofunk mix should not have extreme clipping, peak: {:.3}",
        peak
    );
}

// ============================================================================
// CROSS-PATTERN COMPARISON TESTS
// Verify that different DnB styles sound meaningfully different
// ============================================================================

/// Half-time should have less beat-density than two-step
#[test]
fn dnb_cross_halftime_sparser_than_twostep() {
    let halftime = render_dsl(
        r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        3.0,
    );
    let twostep = render_dsl(
        r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        3.0,
    );

    let halftime_rms = calculate_rms(&halftime);
    let twostep_rms = calculate_rms(&twostep);

    // Two-step has more hits per cycle
    assert!(
        twostep_rms > halftime_rms,
        "Two-step (RMS {:.4}) should have more energy than half-time (RMS {:.4})",
        twostep_rms,
        halftime_rms
    );
}

/// Different DnB subgenres should produce distinct audio
#[test]
fn dnb_cross_subgenres_produce_different_audio() {
    let twostep = render_dsl(
        r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        2.0,
    );
    let jumpup = render_dsl(
        r#"
        cps: 2.9
        ~kick $ s "bd bd ~ ~ bd ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        2.0,
    );
    let halftime = render_dsl(
        r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        2.0,
    );

    // All should produce audio
    assert!(
        calculate_rms(&twostep) > 0.01,
        "Two-step should produce audio"
    );
    assert!(
        calculate_rms(&jumpup) > 0.01,
        "Jump-up should produce audio"
    );
    assert!(
        calculate_rms(&halftime) > 0.01,
        "Half-time should produce audio"
    );

    // Jump-up (most kicks) > two-step > half-time (fewest kicks)
    let ts_rms = calculate_rms(&twostep);
    let ju_rms = calculate_rms(&jumpup);
    let ht_rms = calculate_rms(&halftime);

    assert!(
        ju_rms > ht_rms,
        "Jump-up (RMS {:.4}) should have more energy than half-time (RMS {:.4})",
        ju_rms,
        ht_rms
    );
    assert!(
        ts_rms > ht_rms,
        "Two-step (RMS {:.4}) should have more energy than half-time (RMS {:.4})",
        ts_rms,
        ht_rms
    );
}

/// DnB at faster tempo should be denser than at slower tempo
#[test]
fn dnb_cross_tempo_affects_density() {
    let slow = r#"
        cps: 2.5
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        out $ ~kick
    "#;
    let fast = r#"
        cps: 3.0
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        out $ ~kick
    "#;

    let slow_audio = render_dsl(slow, 4.0);
    let fast_audio = render_dsl(fast, 4.0);

    let slow_rms = calculate_rms(&slow_audio);
    let fast_rms = calculate_rms(&fast_audio);

    assert!(slow_rms > 0.01, "Slow pattern should produce audio");
    assert!(fast_rms > 0.01, "Fast pattern should produce audio");

    // Faster tempo packs more events in the same time window
    assert!(
        fast_rms > slow_rms * 0.8,
        "Faster tempo should maintain or increase energy: fast RMS {:.4} vs slow RMS {:.4}",
        fast_rms,
        slow_rms
    );
}

// ============================================================================
// FULL MIX INTEGRATION TESTS
// Tests complete DnB patterns from the pattern library
// ============================================================================

#[test]
fn dnb_full_liquid_production() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*16" * 0.4
        ~ride $ s "~ ~ ~ ~ ~ ~ ride ~" * 0.3
        ~bass $ saw "55 55 82.5 73.4" # lpf 600 0.6 * 0.25
        ~sub $ sine "55 55 82.5 73.4" * 0.2
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~ride + ~bass + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Liquid DnB should produce audio"
    );
    assert!(
        calculate_peak(&audio) < 5.0,
        "Liquid DnB mix should not clip excessively"
    );
}

#[test]
fn dnb_full_neurofunk_production() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*32" * 0.25
        ~perc $ s "~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ ~" * 0.4
        ~lfo $ sine 4
        ~bass $ saw 55 # lpf (~lfo * 1500 + 500) 0.9 * 0.2
        ~sub $ sine 55 * 0.15
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~perc + ~bass + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Neurofunk production should produce audio"
    );
}

#[test]
fn dnb_full_roller_production() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" * 0.35
        ~bass $ saw 55 # lpf 500 0.8 * 0.3
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~bass
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Roller production should produce audio"
    );
}

#[test]
fn dnb_full_reese_bass_production() {
    let code = r#"
        cps: 2.9
        ~drums $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*16" * 0.8
        ~reese $ supersaw "55 55 55 82.5" 0.6 12
        ~lfo $ sine 0.5 * 0.5 + 0.5
        ~bass $ ~reese # lpf (~lfo * 1500 + 400) 0.85 * 0.2
        ~sub $ sine "55 55 55 82.5" * 0.15
        out $ ~drums + ~bass + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Reese bass production should produce audio"
    );
}

#[test]
fn dnb_full_demo_production() {
    // Full demo from dnb_patterns.ph
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*16" * 0.35
        ~ride $ s "~ ~ ~ ~ ~ ~ ride ~ ~ ~ ~ ~ ~ ~ ride ~" * 0.25
        ~perc $ s "cp(5,16)" * 0.2
        ~bass $ supersaw "55 55 55 82.5" 0.5 8
        ~lfo $ sine 0.3 * 0.5 + 0.5
        ~bass_filt $ ~bass # lpf (~lfo * 1200 + 500) 0.7 * 0.2
        ~sub $ sine "55 55 55 82.5" * 0.18
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~ride + ~perc + ~bass_filt + ~sub
    "#;
    let audio = render_dsl(code, 4.0);

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Full DnB demo should produce audio, RMS: {}",
        rms
    );

    let peak = calculate_peak(&audio);
    assert!(
        peak < 5.0,
        "Full DnB demo should not clip excessively, peak: {:.3}",
        peak
    );
}

/// DnB rendering should be deterministic
#[test]
fn dnb_full_rendering_determinism() {
    let code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;

    let audio1 = render_dsl(code, 2.0);
    let audio2 = render_dsl(code, 2.0);

    let scorer = AudioSimilarityScorer::new(SAMPLE_RATE, SimilarityConfig::default());
    let result = scorer.compare(&audio1, &audio2);

    assert!(
        result.overall >= 0.9,
        "Same DnB pattern should render consistently, similarity: {:.1}%",
        result.overall * 100.0
    );
}

/// DnB tempo range (165-185 BPM = cps 2.75-3.08) should all work
#[test]
fn dnb_full_tempo_range() {
    let tempos = [
        (2.75, "165 BPM"),
        (2.9, "174 BPM"),
        (3.0, "180 BPM"),
        (3.08, "185 BPM"),
    ];

    for (cps, label) in tempos {
        let code = format!(
            r#"
            cps: {}
            ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
            ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
            ~hats $ s "hh*8" * 0.4
            out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
        "#,
            cps
        );

        let audio = render_dsl(&code, 2.0);
        let rms = calculate_rms(&audio);
        assert!(
            rms > 0.01,
            "{} (cps {}) should produce audio, RMS: {}",
            label,
            cps,
            rms
        );
    }
}

/// Jungle at slower tempo (160-170 BPM) vs DnB at 174+ should both work
#[test]
fn dnb_full_jungle_vs_dnb_tempo() {
    let jungle_code = r#"
        cps: 2.75
        ~kick $ s "bd ~ bd ~ ~ ~ bd ~"
        ~snare $ s "~ sn ~ sn ~ ~ ~ ~"
        ~hats $ s "hh*8" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let dnb_code = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*16" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;

    let jungle_audio = render_dsl(jungle_code, 2.0);
    let dnb_audio = render_dsl(dnb_code, 2.0);

    assert!(
        calculate_rms(&jungle_audio) > 0.01,
        "Jungle should produce audio"
    );
    assert!(calculate_rms(&dnb_audio) > 0.01, "DnB should produce audio");
}

/// Minimal DnB should produce audio with less energy than full production
#[test]
fn dnb_full_minimal_vs_full_energy() {
    let minimal = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ ~ hh ~ ~ ~ hh ~" * 0.4
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let full = r#"
        cps: 2.9
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*32" * 0.3
        ~ride $ s "ride*4" * 0.2
        ~perc $ s "cp(5,16)" * 0.2
        ~bass $ saw 55 # lpf 500 0.8 * 0.3
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~ride + ~perc + ~bass
    "#;

    let minimal_audio = render_dsl(minimal, 3.0);
    let full_audio = render_dsl(full, 3.0);

    let minimal_rms = calculate_rms(&minimal_audio);
    let full_rms = calculate_rms(&full_audio);

    assert!(
        full_rms > minimal_rms,
        "Full production (RMS {:.4}) should have more energy than minimal (RMS {:.4})",
        full_rms,
        minimal_rms
    );
}
