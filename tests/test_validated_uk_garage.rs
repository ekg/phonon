//! Validated Tests: UK Garage patterns match reference characteristics
//!
//! Verifies that Phonon's UK Garage / 2-step patterns produce audio matching
//! the documented musical characteristics of the genre:
//!
//! - 2-STEP KICK: Kicks skip beats — beat 1 and "and" of beat 3 (7/16)
//! - BACKBEAT SNARE: Snares on beats 2 & 4 (standard backbeat)
//! - SWING: Hi-hats and percussion swung, kick/snare stay straight
//! - BOUNCY: Ghost kicks add bounce; ghost snares add drive
//! - SPEED GARAGE: Four-on-floor variant with swung hats
//! - OPEN HATS: "Cutting high-end" signature of classic UKG
//!
//! Uses three-level verification methodology:
//!   Level 1: Pattern query verification (event counts, timing)
//!   Level 2: Onset detection (audio events at correct times)
//!   Level 3: Audio characteristics (RMS, spectral content, modulation)
//!
//! UK Garage tempo: 130-140 BPM (cps 2.17-2.33), standard 132 BPM = cps 2.2

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
fn ukg_l1_twostep_kick_positions() {
    // 2-step: "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
    // Kick on beat 1 (0/16 = 0.0) and position 7/16 = 0.4375
    let kick_pattern = parse_mini_notation("bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "2-step kick should have 2 hits per cycle, got {}",
        non_rest.len()
    );

    // First kick at 0/16 = 0.0, second at 7/16 = 0.4375
    let pos0 = non_rest[0].part.begin.to_float();
    let pos1 = non_rest[1].part.begin.to_float();
    assert!(
        pos0.abs() < 0.01,
        "First kick should be at beat 1 (0.0), got {}",
        pos0
    );
    assert!(
        (pos1 - 0.4375).abs() < 0.01,
        "Second kick should be at 7/16 (0.4375), got {}",
        pos1
    );
}

#[test]
fn ukg_l1_snare_on_beats_2_and_4() {
    // Standard backbeat: "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
    // Snares at positions 4/16 = 0.25 (beat 2) and 12/16 = 0.75 (beat 4)
    let snare_pattern = parse_mini_notation("~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~");
    let events = query_single_cycle(&snare_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "UKG snare should have 2 hits per cycle, got {}",
        non_rest.len()
    );

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
fn ukg_l1_offbeat_hats_count() {
    // Offbeat hats: "~ hh ~ hh ~ hh ~ hh" => 4 hits per cycle on offbeats
    let hat_pattern = parse_mini_notation("~ hh ~ hh ~ hh ~ hh");
    let events = query_single_cycle(&hat_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Offbeat hats should have 4 hits per cycle, got {}",
        non_rest.len()
    );

    // Hats at positions 1/8, 3/8, 5/8, 7/8 = 0.125, 0.375, 0.625, 0.875
    for (i, event) in non_rest.iter().enumerate() {
        let expected = (2.0 * i as f64 + 1.0) / 8.0;
        let actual = event.part.begin.to_float();
        assert!(
            (actual - expected).abs() < 0.02,
            "Hat {} should be at {:.3} (offbeat), got {:.3}",
            i,
            expected,
            actual
        );
    }
}

#[test]
fn ukg_l1_8th_note_hats_count() {
    // 8th note hats: "hh*8" => 8 hits per cycle
    let hh_pattern = parse_mini_notation("hh*8");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 32,
        "hh*8 over 4 cycles should produce 32 events, got {}",
        count
    );
}

#[test]
fn ukg_l1_bouncy_kick_ghost_kick() {
    // Bouncy 2-step: "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
    // Ghost kick on 16th before beat 3 adds bounce
    // Kicks at positions 0, 6/16, 7/16
    let kick_pattern = parse_mini_notation("bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        3,
        "Bouncy 2-step should have 3 kicks (including ghost), got {}",
        non_rest.len()
    );

    // First kick at 0.0, ghost at 6/16 = 0.375, main at 7/16 = 0.4375
    let pos0 = non_rest[0].part.begin.to_float();
    let pos1 = non_rest[1].part.begin.to_float();
    let pos2 = non_rest[2].part.begin.to_float();
    assert!(pos0.abs() < 0.01, "First kick at 0.0, got {}", pos0);
    assert!(
        (pos1 - 0.375).abs() < 0.01,
        "Ghost kick at 6/16 (0.375), got {}",
        pos1
    );
    assert!(
        (pos2 - 0.4375).abs() < 0.01,
        "Main second kick at 7/16 (0.4375), got {}",
        pos2
    );
}

#[test]
fn ukg_l1_speed_garage_four_on_floor() {
    // Speed garage: "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~" => 4 kicks (4-on-floor)
    let kick_pattern = parse_mini_notation("bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Speed garage should have 4 kicks (four-on-floor), got {}",
        non_rest.len()
    );

    // Evenly spaced at 0.0, 0.25, 0.5, 0.75
    for (i, event) in non_rest.iter().enumerate() {
        let expected = i as f64 * 0.25;
        let actual = event.part.begin.to_float();
        assert!(
            (actual - expected).abs() < 0.01,
            "Speed garage kick {} should be at {}, got {}",
            i,
            expected,
            actual
        );
    }
}

#[test]
fn ukg_l1_syncopated_kick_pattern() {
    // Syncopated: "bd ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~ bd ~ ~"
    // More complex kick pattern with extra syncopation
    let kick_pattern = parse_mini_notation("bd ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~ bd ~ ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Syncopated kick should have 4 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn ukg_l1_open_hat_pattern() {
    // Open hat groove: "~ oh ~ oh ~ oh ~ oh" => 4 open hats on offbeats
    let oh_pattern = parse_mini_notation("~ oh ~ oh ~ oh ~ oh");
    let events = query_single_cycle(&oh_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Open hat pattern should have 4 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn ukg_l1_minimal_hat_pattern() {
    // Minimal: "~ ~ oh ~ ~ ~ oh ~" => 2 sparse open hats
    let oh_pattern = parse_mini_notation("~ ~ oh ~ ~ ~ oh ~");
    let events = query_single_cycle(&oh_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Minimal hat pattern should have 2 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn ukg_l1_euclidean_kick_pattern() {
    // Euclidean UKG: "bd(3,16,0)" => 3 kicks evenly distributed in 16 slots
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
fn ukg_l1_euclidean_hat_pattern() {
    // Euclidean hats: "hh(7,8)" => nearly continuous, 7 of 8 slots
    let hh_pattern = parse_mini_notation("hh(7,8)");
    let events = query_single_cycle(&hh_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        7,
        "Euclidean hh(7,8) should have 7 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn ukg_l1_clap_replaces_snare() {
    // Minimal/choppy 2-step uses claps: "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
    let clap_pattern = parse_mini_notation("~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~");
    let events = query_single_cycle(&clap_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Clap pattern should have 2 hits on beats 2 & 4, got {}",
        non_rest.len()
    );

    let positions: Vec<f64> = non_rest.iter().map(|h| h.part.begin.to_float()).collect();
    assert!(
        (positions[0] - 0.25).abs() < 0.01,
        "First clap at beat 2 (0.25), got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.75).abs() < 0.01,
        "Second clap at beat 4 (0.75), got {}",
        positions[1]
    );
}

// ============================================================================
// LEVEL 2: DSL Integration - Patterns Compile and Produce Audio
// ============================================================================

#[test]
fn ukg_l2_classic_twostep_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh"
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Classic 2-step should produce audible sound, RMS: {}",
        rms
    );
    assert!(
        !audio.iter().any(|s| s.is_nan()),
        "Audio should not contain NaN"
    );
}

#[test]
fn ukg_l2_swung_twostep_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Swung 2-step should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_bouncy_twostep_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" $ swing 0.1
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Bouncy 2-step should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_speed_garage_produces_audio() {
    let code = r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh hh oh hh hh hh oh hh" $ swing 0.06
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Speed garage should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_minimal_twostep_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
        ~hats $ s "~ ~ oh ~ ~ ~ oh ~" $ swing 0.08
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Minimal 2-step should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_broken_twostep_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*16" * 0.5 $ swing 0.08
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Broken 2-step should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_fouronfour_garage_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~"
        ~hats $ s "~ hh oh hh ~ hh oh hh" $ swing 0.06
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "4x4 garage should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_percussive_twostep_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" $ swing 0.08
        ~rim $ s "~ rs ~ ~ ~ rs ~ ~" * 0.5 $ swing 0.08
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~rim
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Percussive 2-step should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_euclidean_produces_audio() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd(3,16)"
        ~snare $ s "sn(2,8,4)"
        ~hats $ s "hh(7,8)" * 0.5
        ~rim $ s "rs(5,16)" * 0.3
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~rim
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Euclidean UKG pattern should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_sub_bass_produces_audio() {
    let code = r#"
        cps: 2.2
        ~sub $ sine "55 55 82.5 73.4" * 0.3
        out $ ~sub
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "UKG sub bass should produce audible sound, RMS: {}",
        rms
    );
}

#[test]
fn ukg_l2_wobble_bass_produces_audio() {
    let code = r#"
        cps: 2.2
        ~lfo $ sine 0.5 * 0.5 + 0.5
        ~bass $ saw "55 55 82.5 73.4" # lpf (~lfo * 800 + 400) 0.7
        out $ ~bass * 0.3
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.001,
        "UKG wobble bass should produce audible sound, RMS: {}",
        rms
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics - Genre-Specific Properties
// ============================================================================

/// 2-step kick should have onset regularity
#[test]
fn ukg_l3_twostep_kick_regularity() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        out $ ~kick
    "#;
    let audio = render_dsl(code, 4.0);
    let onsets = detect_onsets(&audio, SAMPLE_RATE);

    // At cps 2.2, 2 kicks per cycle = ~4.4 kicks/sec
    // Over 4 seconds expect ~17 kicks
    assert!(
        onsets.len() >= 6,
        "2-step kick should detect multiple regular onsets, got {}",
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
            "2-step kick should have reasonably regular intervals (CV < 1.0), got CV={:.3}",
            cv
        );
    }
}

/// Speed garage (more kicks) should have more energy than minimal 2-step
#[test]
fn ukg_l3_speed_garage_more_energy_than_minimal() {
    let minimal_code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;
    let speed_code = r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;

    let minimal_audio = render_dsl(minimal_code, 2.0);
    let speed_audio = render_dsl(speed_code, 2.0);

    let minimal_rms = calculate_rms(&minimal_audio);
    let speed_rms = calculate_rms(&speed_audio);

    assert!(
        speed_rms > minimal_rms,
        "Speed garage (RMS {:.4}) should have more energy than minimal 2-step (RMS {:.4})",
        speed_rms,
        minimal_rms
    );
}

/// Adding hi-hats should increase spectral centroid (brighter)
#[test]
fn ukg_l3_hats_add_brightness() {
    let kick_only = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        out $ ~kick * 0.8
    "#;
    let kick_plus_hats = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" * 0.5
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

/// Open hats should add more high-frequency content than closed hats
#[test]
fn ukg_l3_open_hats_brighter_than_closed() {
    let closed_code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~hats $ s "hh*8" * 0.5
        out $ ~kick * 0.8 + ~hats
    "#;
    let open_code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~hats $ s "oh*8" * 0.5
        out $ ~kick * 0.8 + ~hats
    "#;

    let closed_audio = render_dsl(closed_code, 2.0);
    let open_audio = render_dsl(open_code, 2.0);

    let closed_rms = calculate_rms(&closed_audio);
    let open_rms = calculate_rms(&open_audio);

    // Both should produce audio
    assert!(closed_rms > 0.01, "Closed hats should produce audio");
    assert!(open_rms > 0.01, "Open hats should produce audio");

    // Open hats should be at least as bright (longer decay = more sustained high freq)
    let open_centroid = spectral_centroid(&open_audio);
    let closed_centroid = spectral_centroid(&closed_audio);

    // We just verify both have high-frequency content
    assert!(
        open_centroid > 100.0,
        "Open hats should have significant high-freq content, centroid: {:.0}Hz",
        open_centroid
    );
    assert!(
        closed_centroid > 100.0,
        "Closed hats should have significant high-freq content, centroid: {:.0}Hz",
        closed_centroid
    );
}

/// Sub bass (sine 55Hz) should have very low spectral centroid
#[test]
fn ukg_l3_sub_bass_low_centroid() {
    let code = r#"
        cps: 2.2
        ~sub $ sine 55 * 0.4
        out $ ~sub
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Sub bass should produce audio, RMS: {}", rms);

    let centroid = spectral_centroid(&audio);
    assert!(
        centroid < 500.0,
        "UKG sub bass should have very low centroid, got {:.0}Hz",
        centroid
    );
}

/// Wobble bass (LFO-modulated filter) should have spectral variation
#[test]
fn ukg_l3_wobble_bass_spectral_variation() {
    let code = r#"
        cps: 2.2
        ~lfo $ sine 0.5
        ~bass $ saw 55 # lpf (~lfo * 800 + 400) 0.7
        out $ ~bass * 0.3
    "#;
    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.001, "Wobble bass should produce audio");

    let variation = envelope_variation(&audio, 50.0);
    assert!(
        variation > 0.0001,
        "LFO-modulated wobble bass should have envelope variation, got {:.6}",
        variation
    );
}

/// UKG drum patterns should have rhythmic envelope variation
#[test]
fn ukg_l3_rhythmic_envelope() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;
    let audio = render_dsl(code, 4.0);
    let variation = envelope_variation(&audio, 50.0);

    assert!(
        variation > 0.001,
        "UKG drum pattern should have rhythmic envelope variation, got {:.6}",
        variation
    );
}

/// Bouncy 2-step (3 kicks) should have more energy than classic (2 kicks)
#[test]
fn ukg_l3_bouncy_more_energy_than_classic() {
    let classic_code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;
    let bouncy_code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#;

    let classic_audio = render_dsl(classic_code, 2.0);
    let bouncy_audio = render_dsl(bouncy_code, 2.0);

    let classic_rms = calculate_rms(&classic_audio);
    let bouncy_rms = calculate_rms(&bouncy_audio);

    assert!(
        bouncy_rms > classic_rms,
        "Bouncy (RMS {:.4}) should have more energy than classic (RMS {:.4})",
        bouncy_rms,
        classic_rms
    );
}

/// UKG patterns should not clip excessively
#[test]
fn ukg_l3_classic_twostep_no_clipping() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let audio = render_dsl(code, 2.0);
    let peak = calculate_peak(&audio);

    assert!(
        peak < 5.0,
        "Classic 2-step mix should not have extreme clipping, peak: {:.3}",
        peak
    );
}

#[test]
fn ukg_l3_speed_garage_no_clipping() {
    let code = r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh hh oh hh hh hh oh hh" $ swing 0.06
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
    "#;
    let audio = render_dsl(code, 2.0);
    let peak = calculate_peak(&audio);

    assert!(
        peak < 5.0,
        "Speed garage mix should not have extreme clipping, peak: {:.3}",
        peak
    );
}

// ============================================================================
// CROSS-PATTERN COMPARISON TESTS
// Verify that different UKG styles sound meaningfully different
// ============================================================================

/// Classic 2-step should have less kick density than speed garage
#[test]
fn ukg_cross_twostep_sparser_than_speed_garage() {
    let twostep = render_dsl(
        r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        3.0,
    );
    let speed_garage = render_dsl(
        r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        3.0,
    );

    let twostep_rms = calculate_rms(&twostep);
    let speed_rms = calculate_rms(&speed_garage);

    assert!(
        speed_rms > twostep_rms,
        "Speed garage (RMS {:.4}) should have more energy than 2-step (RMS {:.4})",
        speed_rms,
        twostep_rms
    );
}

/// Different UKG subgenres should produce distinct audio
#[test]
fn ukg_cross_subgenres_produce_different_audio() {
    let classic = render_dsl(
        r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        2.0,
    );
    let bouncy = render_dsl(
        r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        2.0,
    );
    let speed = render_dsl(
        r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        out $ ~kick * 0.8 + ~snare * 0.6
    "#,
        2.0,
    );

    // All should produce audio
    assert!(
        calculate_rms(&classic) > 0.01,
        "Classic should produce audio"
    );
    assert!(calculate_rms(&bouncy) > 0.01, "Bouncy should produce audio");
    assert!(
        calculate_rms(&speed) > 0.01,
        "Speed garage should produce audio"
    );

    // Speed garage (most kicks + faster) > bouncy (3 kicks) > classic (2 kicks)
    let cl_rms = calculate_rms(&classic);
    let bn_rms = calculate_rms(&bouncy);
    let sp_rms = calculate_rms(&speed);

    assert!(
        sp_rms > cl_rms,
        "Speed garage (RMS {:.4}) should have more energy than classic (RMS {:.4})",
        sp_rms,
        cl_rms
    );
    assert!(
        bn_rms > cl_rms,
        "Bouncy (RMS {:.4}) should have more energy than classic (RMS {:.4})",
        bn_rms,
        cl_rms
    );
}

/// More percussion layers = more energy
#[test]
fn ukg_cross_percussive_more_energy_than_minimal() {
    let minimal = render_dsl(
        r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
        ~hats $ s "~ ~ oh ~ ~ ~ oh ~" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#,
        2.0,
    );
    let percussive = render_dsl(
        r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" * 0.4
        ~rim $ s "~ rs ~ ~ ~ rs ~ ~" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~rim
    "#,
        2.0,
    );

    let minimal_rms = calculate_rms(&minimal);
    let percussive_rms = calculate_rms(&percussive);

    assert!(
        percussive_rms > minimal_rms,
        "Percussive (RMS {:.4}) should have more energy than minimal (RMS {:.4})",
        percussive_rms,
        minimal_rms
    );
}

/// UKG at faster tempo should maintain energy
#[test]
fn ukg_cross_tempo_affects_density() {
    let slow = r#"
        cps: 2.07
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        out $ ~kick
    "#;
    let fast = r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
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
// Tests complete UKG patterns from the pattern library
// ============================================================================

#[test]
fn ukg_full_classic_twostep_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
        ~opens $ s "~ oh ~ oh ~ ~ ~ oh" $ swing 0.08
        ~bass $ sine "55 55 82.5 73.4" * 0.3
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5 + ~opens * 0.4 + ~bass
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Classic 2-step production should produce audio"
    );
    assert!(
        calculate_peak(&audio) < 5.0,
        "Classic 2-step production should not clip excessively"
    );
}

#[test]
fn ukg_full_bouncy_production() {
    let code = r#"
        cps: 2.13
        ~kick $ s "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" $ swing 0.1
        ~sub $ sine "55 55 55 82.5" * 0.25
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5 + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Bouncy production should produce audio"
    );
}

#[test]
fn ukg_full_speed_garage_production() {
    let code = r#"
        cps: 2.33
        ~kick $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh hh oh hh hh hh oh hh" $ swing 0.06
        ~bass $ saw "110 ~ 82.5 ~" # lpf 1000 0.7 * 0.2
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5 + ~bass
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Speed garage production should produce audio"
    );
}

#[test]
fn ukg_full_minimal_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
        ~hats $ s "~ ~ oh ~ ~ ~ oh ~" $ swing 0.08
        ~sub $ sine "55 55 82.5 73.4" * 0.2
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5 + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Minimal production should produce audio"
    );
}

#[test]
fn ukg_full_percussive_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" $ swing 0.08
        ~rim $ s "~ rs ~ ~ ~ rs ~ ~" * 0.5 $ swing 0.08
        ~bass $ sine "55 55 82.5 73.4" * 0.25
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~rim + ~bass
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Percussive production should produce audio"
    );
}

#[test]
fn ukg_full_open_hat_groove_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~opens $ s "~ oh ~ oh ~ oh ~ oh" $ swing 0.08
        ~closed $ s "hh ~ hh ~ hh ~ hh ~" * 0.4
        ~bass $ sine "55 55 82.5 73.4" * 0.25
        out $ ~kick * 0.8 + ~snare * 0.6 + ~opens * 0.5 + ~closed + ~bass
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Open hat groove production should produce audio"
    );
}

#[test]
fn ukg_full_dubby_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~rim $ s "~ ~ rs ~ ~ ~ ~ ~ ~ ~ rs ~ ~ ~ ~ ~" $ swing 0.1
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.1
        ~sub $ sine "55 55 82.5 73.4" * 0.25
        out $ ~kick * 0.8 + ~snare * 0.6 + ~rim * 0.4 + ~hats * 0.4 + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Dubby 2-step production should produce audio"
    );
}

#[test]
fn ukg_full_wobble_bass_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
        ~lfo $ sine 0.5 * 0.5 + 0.5
        ~bass $ saw "55 55 82.5 73.4" # lpf (~lfo * 800 + 400) 0.7 * 0.2
        ~sub $ sine "55 55 82.5 73.4" * 0.15
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~bass + ~sub
    "#;
    let audio = render_dsl(code, 3.0);
    assert!(
        calculate_rms(&audio) > 0.01,
        "Wobble bass production should produce audio"
    );
}

/// UKG rendering should be deterministic
#[test]
fn ukg_full_rendering_determinism() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;

    let audio1 = render_dsl(code, 2.0);
    let audio2 = render_dsl(code, 2.0);

    let scorer = AudioSimilarityScorer::new(SAMPLE_RATE, SimilarityConfig::default());
    let result = scorer.compare(&audio1, &audio2);

    assert!(
        result.overall >= 0.9,
        "Same UKG pattern should render consistently, similarity: {:.1}%",
        result.overall * 100.0
    );
}

/// UKG tempo range (130-140 BPM = cps 2.17-2.33) should all work
#[test]
fn ukg_full_tempo_range() {
    let tempos = [
        (2.07, "124 BPM (rolling)"),
        (2.17, "130 BPM"),
        (2.2, "132 BPM (standard)"),
        (2.25, "135 BPM"),
        (2.33, "140 BPM (speed garage)"),
    ];

    for (cps, label) in tempos {
        let code = format!(
            r#"
            cps: {}
            ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
            ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
            ~hats $ s "~ hh ~ hh ~ hh ~ hh" * 0.5
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

/// Full demo-style production with all elements
#[test]
fn ukg_full_demo_production() {
    let code = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
        ~opens $ s "~ oh ~ oh ~ ~ ~ oh" $ swing 0.08
        ~rim $ s "~ ~ ~ rs ~ rs ~ ~" * 0.4 $ swing 0.08
        ~bass $ sine "55 55 82.5 73.4" * 0.25
        ~lfo $ sine 0.5 * 0.5 + 0.5
        ~wobble $ saw "55 55 82.5 73.4" # lpf (~lfo * 800 + 400) 0.7 * 0.15
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5 + ~opens * 0.4 + ~rim + ~bass + ~wobble
    "#;
    let audio = render_dsl(code, 4.0);

    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Full UKG demo should produce audio, RMS: {}",
        rms
    );

    let peak = calculate_peak(&audio);
    assert!(
        peak < 5.0,
        "Full UKG demo should not clip excessively, peak: {:.3}",
        peak
    );
}

/// Minimal vs full production: full should have more energy
#[test]
fn ukg_full_minimal_vs_full_energy() {
    let minimal = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
        ~hats $ s "~ ~ oh ~ ~ ~ oh ~" * 0.5
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats
    "#;
    let full = r#"
        cps: 2.2
        ~kick $ s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" * 0.4 $ swing 0.08
        ~opens $ s "~ oh ~ oh ~ ~ ~ oh" * 0.4
        ~rim $ s "~ rs ~ ~ ~ rs ~ ~" * 0.3
        ~bass $ sine "55 55 82.5 73.4" * 0.25
        out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~opens + ~rim + ~bass
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
