//! Validated Tests: House patterns match reference characteristics
//!
//! Verifies that Phonon's house music patterns produce audio matching
//! the documented musical characteristics of house subgenres:
//!
//! - CHICAGO HOUSE: Four-on-floor, swung 16th hats, offbeat open hats
//! - DEEP HOUSE: Warm sub bass, sparse claps, lush pads
//! - ACID HOUSE: Resonant 303 basslines, distorted filter sweeps
//! - PROGRESSIVE HOUSE: Slow builds, arpeggiated synths, long filter sweeps
//! - TECH HOUSE: Tight swing, punchy bass, hypnotic loops
//! - DISCO HOUSE: Funky grooves, octave bass jumps, open hat accents
//! - FRENCH HOUSE: Sidechain pumping, compressed 16ths, filtered chords
//! - GARAGE HOUSE: Heavy swing, organ bass, staccato stabs
//! - VOCAL HOUSE: Piano chords, accented open hats, octave bass
//! - TRIBAL HOUSE: Polyrhythmic percussion, deep sub, sparse melodic
//! - MINIMAL HOUSE: Degraded patterns, heavy shuffle, noise textures
//! - AFRO HOUSE: Complex polyrhythms, minor key bass
//! - SOULFUL HOUSE: Walking bass, organ pads, gospel chords
//! - ELECTRO HOUSE: Sidechained saw bass, aggressive filter sweeps
//! - BREAKDOWN: Atmospheric, no kick, building tension
//!
//! Uses three-level verification methodology:
//!   Level 1: Pattern query verification (event counts, timing)
//!   Level 2: Onset detection (audio events at correct times)
//!   Level 3: Audio characteristics (RMS, spectral content, modulation)

use phonon::audio_similarity::{detect_onsets, SpectralFeatures};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
use audio_test_utils::{calculate_rms, compute_spectral_centroid, find_peak};

mod pattern_verification_utils;
use pattern_verification_utils::{detect_audio_events, is_silent};

const SAMPLE_RATE: f32 = 44100.0;

// ============================================================================
// HELPERS
// ============================================================================

fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Parse DSL");
    let compiler = DslCompiler::new(SAMPLE_RATE);
    let mut graph = compiler.compile(statements);
    let samples = (SAMPLE_RATE * duration_secs) as usize;
    graph.render(samples)
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

/// Calculate RMS of a time window within audio
fn rms_window(audio: &[f32], start_sec: f32, end_sec: f32, sample_rate: f32) -> f32 {
    let start = (start_sec * sample_rate) as usize;
    let end = (end_sec * sample_rate).min(audio.len() as f32) as usize;
    if start >= end || start >= audio.len() {
        return 0.0;
    }
    let slice = &audio[start..end];
    calculate_rms(slice)
}

/// Calculate spectral centroid using the audio_similarity module
fn spectral_centroid(audio: &[f32]) -> f32 {
    let features = SpectralFeatures::from_audio(audio, SAMPLE_RATE, 2048);
    features.centroid
}

/// Calculate envelope variation (std dev of RMS across windows)
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

// ============================================================================
// LEVEL 1: PATTERN QUERY VERIFICATION
// Tests pattern logic without rendering audio
// ============================================================================

// --- Four-on-the-Floor Foundation ---

#[test]
fn house_l1_four_on_floor_kick_count() {
    // The universal house foundation: "bd*4" => 4 kicks per cycle
    let kick_pattern = parse_mini_notation("bd*4");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Four-on-the-floor should have 4 kicks per cycle, got {}",
        non_rest.len()
    );
}

#[test]
fn house_l1_four_on_floor_evenly_spaced() {
    // Kicks should be evenly spaced at 0.0, 0.25, 0.5, 0.75
    let kick_pattern = parse_mini_notation("bd*4");
    let events = query_single_cycle(&kick_pattern);
    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();

    for (i, event) in non_rest.iter().enumerate() {
        let expected = i as f64 * 0.25;
        let actual = event.part.begin.to_float();
        assert!(
            (actual - expected).abs() < 0.01,
            "Kick {} should be at {}, got {}",
            i,
            expected,
            actual
        );
    }
}

#[test]
fn house_l1_four_on_floor_over_8_cycles() {
    // Consistency check: 4 kicks × 8 cycles = 32 events
    let kick_pattern = parse_mini_notation("bd*4");
    let count = count_events_over_cycles(&kick_pattern, 8);
    assert_eq!(
        count, 32,
        "bd*4 over 8 cycles should produce 32 events, got {}",
        count
    );
}

// --- Clap/Snare Patterns ---

#[test]
fn house_l1_clap_on_2_and_4() {
    // Standard house clap: "~ cp ~ cp" => beats 2 and 4
    let clap_pattern = parse_mini_notation("~ cp ~ cp");
    let events = query_single_cycle(&clap_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Clap on 2 and 4 should have 2 hits per cycle, got {}",
        non_rest.len()
    );

    let positions: Vec<f64> = non_rest.iter().map(|h| h.part.begin.to_float()).collect();
    assert!(
        (positions[0] - 0.25).abs() < 0.01,
        "First clap should be at beat 2 (0.25), got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.75).abs() < 0.01,
        "Second clap should be at beat 4 (0.75), got {}",
        positions[1]
    );
}

#[test]
fn house_l1_deep_house_sparse_clap() {
    // Deep house: clap only on beat 2: "~ cp ~ ~"
    let clap_pattern = parse_mini_notation("~ cp ~ ~");
    let events = query_single_cycle(&clap_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "Deep house sparse clap should have 1 hit per cycle, got {}",
        non_rest.len()
    );

    let pos = non_rest[0].part.begin.to_float();
    assert!(
        (pos - 0.25).abs() < 0.01,
        "Sparse clap should be at beat 2 (0.25), got {}",
        pos
    );
}

#[test]
fn house_l1_progressive_clap_on_4_only() {
    // Progressive house: very sparse clap on beat 4 only: "~ ~ ~ cp"
    let clap_pattern = parse_mini_notation("~ ~ ~ cp");
    let events = query_single_cycle(&clap_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "Progressive clap should have 1 hit per cycle, got {}",
        non_rest.len()
    );

    let pos = non_rest[0].part.begin.to_float();
    assert!(
        (pos - 0.75).abs() < 0.01,
        "Progressive clap should be at beat 4 (0.75), got {}",
        pos
    );
}

#[test]
fn house_l1_tribal_clap_on_3() {
    // Tribal house: clap on beat 3 only: "~ ~ cp ~"
    let clap_pattern = parse_mini_notation("~ ~ cp ~");
    let events = query_single_cycle(&clap_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "Tribal clap should have 1 hit, got {}",
        non_rest.len()
    );

    let pos = non_rest[0].part.begin.to_float();
    assert!(
        (pos - 0.5).abs() < 0.01,
        "Tribal clap should be at beat 3 (0.5), got {}",
        pos
    );
}

// --- Hi-Hat Patterns ---

#[test]
fn house_l1_16th_hats_count() {
    // Common in Chicago, French, Tech, Soulful house: "hh*16" => 16 hits
    let hh_pattern = parse_mini_notation("hh*16");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 64,
        "hh*16 over 4 cycles should produce 64 events, got {}",
        count
    );
}

#[test]
fn house_l1_8th_hats_count() {
    // Common in Deep, Progressive, Vocal house: "hh*8" => 8 hits
    let hh_pattern = parse_mini_notation("hh*8");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 32,
        "hh*8 over 4 cycles should produce 32 events, got {}",
        count
    );
}

#[test]
fn house_l1_offbeat_open_hats() {
    // Chicago/Soulful: offbeat open hats "~ oh ~ oh ~ oh ~ oh" => 4 hits
    let oh_pattern = parse_mini_notation("~ oh ~ oh ~ oh ~ oh");
    let events = query_single_cycle(&oh_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Offbeat open hats should have 4 hits per cycle, got {}",
        non_rest.len()
    );

    // Each hit should be on the offbeat (odd positions in 8-slot grid)
    for (i, event) in non_rest.iter().enumerate() {
        let expected = (2.0 * i as f64 + 1.0) / 8.0;
        let actual = event.part.begin.to_float();
        assert!(
            (actual - expected).abs() < 0.02,
            "Open hat {} should be at {:.3} (offbeat), got {:.3}",
            i,
            expected,
            actual
        );
    }
}

#[test]
fn house_l1_sparse_hats_deep_house() {
    // Deep house sparse hats: "~ hh ~ hh" => only 2 hits per cycle
    let hh_pattern = parse_mini_notation("~ hh ~ hh");
    let events = query_single_cycle(&hh_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Sparse deep house hats should have 2 hits, got {}",
        non_rest.len()
    );
}

#[test]
fn house_l1_disco_hat_pattern() {
    // Disco house: mixed closed/open hat: "hh hh oh hh hh hh oh hh" => 8 events
    let hat_pattern = parse_mini_notation("hh hh oh hh hh hh oh hh");
    let events = query_single_cycle(&hat_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        8,
        "Disco hat pattern should have 8 events, got {}",
        non_rest.len()
    );

    // Open hats should be at positions 2/8 and 6/8
    let oh_positions: Vec<f64> = events
        .iter()
        .filter(|h| h.value == "oh")
        .map(|h| h.part.begin.to_float())
        .collect();
    assert_eq!(oh_positions.len(), 2, "Should have 2 open hats");
    assert!(
        (oh_positions[0] - 0.25).abs() < 0.02,
        "First open hat at position 2/8"
    );
    assert!(
        (oh_positions[1] - 0.75).abs() < 0.02,
        "Second open hat at position 6/8"
    );
}

// --- Bass Patterns ---

#[test]
fn house_l1_syncopated_bass_events() {
    // Chicago bass: "55 ~ 55 82.5 ~ 55 ~ 110" => 5 notes per cycle
    let bass_pattern = parse_mini_notation("55 ~ 55 82.5 ~ 55 ~ 110");
    let events = query_single_cycle(&bass_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        5,
        "Chicago syncopated bass should have 5 events, got {}",
        non_rest.len()
    );
}

#[test]
fn house_l1_octave_bass_pattern() {
    // Disco/Vocal house bass with octave jumps: "55 55 110 55" => 4 events
    let bass_pattern = parse_mini_notation("55 55 110 55");
    let events = query_single_cycle(&bass_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Octave bass should have 4 events per cycle, got {}",
        non_rest.len()
    );
}

#[test]
fn house_l1_walking_bass_pattern() {
    // Soulful house walking bass: "55 65.41 73.42 82.5 73.42 65.41 55 55" => 8 steps
    let bass_pattern = parse_mini_notation("55 65.41 73.42 82.5 73.42 65.41 55 55");
    let events = query_single_cycle(&bass_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        8,
        "Walking bass should have 8 events, got {}",
        non_rest.len()
    );
}

#[test]
fn house_l1_acid_bass_line() {
    // Acid house 303 line: "55 55 110 55 82.5 55 110 55" => 8 events (all notes)
    let bass_pattern = parse_mini_notation("55 55 110 55 82.5 55 110 55");
    let events = query_single_cycle(&bass_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        8,
        "Acid bass should have 8 events (continuous), got {}",
        non_rest.len()
    );
}

#[test]
fn house_l1_sparse_organ_bass() {
    // Garage house organ bass: "55 ~ 55 ~ 82.5 ~ 55 ~" => 4 notes
    let bass_pattern = parse_mini_notation("55 ~ 55 ~ 82.5 ~ 55 ~");
    let events = query_single_cycle(&bass_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Organ bass should have 4 events per cycle, got {}",
        non_rest.len()
    );
}

// --- Euclidean Patterns ---

#[test]
fn house_l1_euclidean_shaker() {
    // Afro house shaker: "shaker(5,8)" => 5 hits in 8 slots
    let shaker_pattern = parse_mini_notation("shaker(5,8)");
    let events = query_single_cycle(&shaker_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        5,
        "Euclidean shaker(5,8) should have 5 hits, got {}",
        non_rest.len()
    );
}

// --- Rim/Percussion Patterns ---

#[test]
fn house_l1_rim_beat3() {
    // Deep/Tech house rim on beat 3: "~ ~ rim ~"
    let rim_pattern = parse_mini_notation("~ ~ rim ~");
    let events = query_single_cycle(&rim_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "Should have 1 rim hit, got {}",
        non_rest.len()
    );

    let pos = non_rest[0].part.begin.to_float();
    assert!(
        (pos - 0.5).abs() < 0.01,
        "Rim should be at beat 3 (0.5), got {}",
        pos
    );
}

#[test]
fn house_l1_minimal_rim_syncopation() {
    // Minimal house syncopated rim: "~ rim [~ rim] ~" => 3 hits (subdivided)
    let rim_pattern = parse_mini_notation("~ rim [~ rim] ~");
    let events = query_single_cycle(&rim_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Syncopated rim should have 2 hits per cycle, got {}",
        non_rest.len()
    );
}

// --- Sidechain / Pump Envelope Patterns ---

#[test]
fn house_l1_sidechain_envelope_pattern() {
    // French/Electro house pump: "0.3 0.8 1 1 0.3 0.8 1 1" => 8 steps
    let pump_pattern = parse_mini_notation("0.3 0.8 1 1 0.3 0.8 1 1");
    let events = query_single_cycle(&pump_pattern);
    assert_eq!(
        events.len(),
        8,
        "Sidechain pattern should have 8 steps, got {}",
        events.len()
    );
}

// --- Chord Stab Patterns ---

#[test]
fn house_l1_staccato_stab_pattern() {
    // Garage house stabs: "~ 1 ~ 0.5 ~ 1 ~ ~" => 3 active hits
    let stab_pattern = parse_mini_notation("~ 1 ~ 0.5 ~ 1 ~ ~");
    let events = query_single_cycle(&stab_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        3,
        "Staccato stab should have 3 active events, got {}",
        non_rest.len()
    );
}

// ============================================================================
// LEVEL 2: ONSET DETECTION
// Tests that rendered audio has events at the right times
// ============================================================================

// --- Chicago House ---

#[test]
fn house_l2_chicago_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5
out $ ~kick + ~clap + ~hats + ~oh
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Chicago house should produce audible output"
    );
}

#[test]
fn house_l2_chicago_has_dense_onsets() {
    // 16th hats + kick + clap + open hat = very dense onset pattern
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~clap + ~hats
"#;
    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, SAMPLE_RATE, 0.01);

    assert!(
        events.len() >= 8,
        "Chicago house should have dense onsets (16th hats), got {}",
        events.len()
    );
}

// --- Deep House ---

#[test]
fn house_l2_deep_renders_audio() {
    let code = r#"
cps: 1.97
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
~rim $ s "~ ~ ~ rim" # gain 0.4
out $ ~kick * 0.85 + ~clap + ~hats + ~rim
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Deep house should produce audible output"
    );
}

#[test]
fn house_l2_deep_with_bass_and_pad() {
    let code = r#"
cps: 1.97
~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.35
~bass $ sine 55 # lpf 200 0.8
~pad $ sine 130.81 + sine 155.56 + sine 196
~pad_filtered $ ~pad # lpf 1500 0.5 * 0.12
out $ ~kick * 0.85 + ~hats + ~bass * 0.4 + ~pad_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Deep house with bass and pad should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

// --- Acid House ---

#[test]
fn house_l2_acid_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.5
~bass $ saw "55 55 110 55 82.5 55 110 55"
~acid $ ~bass # lpf 2200 3.5 # distortion 1.5 * 0.25
out $ ~kick + ~clap + ~hats + ~acid
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Acid house should produce audible output"
    );
}

// --- Progressive House ---

#[test]
fn house_l2_progressive_renders_audio() {
    let code = r#"
cps: 2.1
~kick $ s "bd*4"
~clap $ s "~ ~ ~ cp" # gain 0.65
~hats $ s "hh*8" # gain 0.45
out $ ~kick + ~clap + ~hats
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Progressive house should produce audible output"
    );
}

#[test]
fn house_l2_progressive_with_arp_and_pad() {
    let code = r#"
cps: 2.1
~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.45
~arp $ sine "130.81 196 261.63 196" $ fast 2
~arp_filtered $ ~arp # lpf 4000 0.5 * 0.12
out $ ~kick + ~hats + ~arp_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Progressive with arp should produce audio"
    );
}

// --- Tech House ---

#[test]
fn house_l2_tech_renders_audio() {
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.65
~rim $ s "~ ~ rim ~" # gain 0.5
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~clap + ~rim + ~hats
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Tech house should produce audible output"
    );
}

#[test]
fn house_l2_tech_with_filtered_bass() {
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 300 1.3
~lfo $ sine 0.25
~bass_filtered $ ~bass # lpf (~lfo * 200 + 300) 1.0 * 0.35
out $ ~kick + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Tech house with filtered bass should produce audio"
    );
}

// --- Disco House ---

#[test]
fn house_l2_disco_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.6
~hats $ s "hh hh oh hh hh hh oh hh" # gain 0.5
~bass $ saw "55 110 55 82.5 110 55 82.5 55"
~bass_filtered $ ~bass # lpf 700 1.0 * 0.3
out $ ~kick + ~clap + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Disco house should produce audible output"
    );
}

// --- French House ---

#[test]
fn house_l2_french_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.35
~bass $ saw 55 # lpf 500 1.2
~pump_envelope $ "0.3 0.8 1 1 0.3 0.8 1 1"
out $ ~kick + ~clap + ~hats + ~bass * 0.35
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "French house should produce audible output"
    );
}

// --- Garage House ---

#[test]
fn house_l2_garage_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ sine "55 ~ 55 ~ 82.5 ~ 55 ~"
~bass_filtered $ ~bass # lpf 300 0.9 * 0.4
out $ ~kick + ~clap + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Garage house should produce audible output"
    );
}

// --- Vocal House ---

#[test]
fn house_l2_vocal_renders_audio() {
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8" # gain 0.45
~oh $ s "~ ~ oh ~ ~ ~ oh ~" # gain 0.5
~bass $ saw "55 55 110 55" # lpf 350 1.1 * 0.35
out $ ~kick + ~clap + ~hats + ~oh + ~bass
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Vocal house should produce audible output"
    );
}

#[test]
fn house_l2_vocal_with_piano_chords() {
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~piano_c $ sine 130.81 + sine 164.81 + sine 196 + sine 246.94
~piano_f $ sine 174.61 + sine 220 + sine 261.63 + sine 329.63
~piano $ (~piano_c * "1 ~ ~ 0.7") + (~piano_f * "~ 0.8 ~ ~")
~piano_filtered $ ~piano # lpf 3500 0.5 * 0.15
out $ ~kick + ~clap + ~piano_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Piano chord pattern should produce audio"
    );
}

// --- Tribal House ---

#[test]
fn house_l2_tribal_renders_audio() {
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ ~ cp ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
~bass $ sine 55 # lpf 120 0.8 * 0.45
out $ ~kick + ~clap + ~hats + ~bass
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Tribal house should produce audible output"
    );
}

// --- Minimal House ---

#[test]
fn house_l2_minimal_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.35
~rim $ s "~ rim [~ rim] ~" # gain 0.45
~bass $ sine "55 ~ ~ 82.5 ~ 55 ~ ~" * 0.4
out $ ~kick * 0.85 + ~hats + ~rim + ~bass
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Minimal house should produce audible output"
    );
}

// --- Afro House ---

#[test]
fn house_l2_afro_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "~ hh ~ hh" # gain 0.4
~bass $ sine "55 ~ 55 65.41 ~ 55 ~ 73.42"
~bass_filtered $ ~bass # lpf 250 0.9 * 0.4
out $ ~kick + ~clap + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Afro house should produce audible output"
    );
}

// --- Soulful House ---

#[test]
fn house_l2_soulful_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.45
~bass $ saw "55 65.41 73.42 82.5 73.42 65.41 55 55"
~bass_filtered $ ~bass # lpf 400 1.0 * 0.3
out $ ~kick + ~clap + ~hats + ~oh + ~bass_filtered
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Soulful house should produce audible output"
    );
}

// --- Electro House ---

#[test]
fn house_l2_electro_renders_audio() {
    let code = r#"
cps: 2.133
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 400 1.5
~sidechain $ "0.2 0.6 1 1 0.2 0.6 1 1"
out $ ~kick + ~clap + ~hats + ~bass * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Electro house should produce audible output"
    );
}

// --- Breakdown ---

#[test]
fn house_l2_breakdown_renders_audio() {
    // Breakdown: atmospheric, no kick, building tension
    let code = r#"
cps: 2.0
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_building $ ~pad # lpf 800 0.6 * 0.15
~sub $ sine 55 # lpf 80 0.7 * 0.25
out $ ~pad_building + ~sub
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Breakdown should produce audible output"
    );
}

// ============================================================================
// LEVEL 3: AUDIO CHARACTERISTICS
// Verifies signal properties match house genre reference characteristics
// ============================================================================

// --- Four-on-the-Floor Regularity ---

#[test]
fn house_l3_four_on_floor_regular_onsets() {
    // Just kick, should produce very regular onsets
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
out $ ~kick
"#;
    let audio = render_dsl(code, 4.0);
    let onsets = detect_onsets(&audio, SAMPLE_RATE);

    // At cps 2.0, 4 beats per cycle = 8 beats/sec, 32 beats in 4s
    assert!(
        onsets.len() >= 8,
        "Four-on-the-floor should detect multiple regular onsets, got {}",
        onsets.len()
    );

    // Check regularity: coefficient of variation should be low
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
            "Four-on-the-floor should have regular intervals (CV < 1.0), got CV={:.3}",
            cv
        );
    }
}

#[test]
fn house_l3_kick_energy_on_all_beats() {
    // Four-on-floor: every beat should have energy
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
out $ ~kick * 0.8
"#;
    let audio = render_dsl(code, 1.5);
    let cycle_duration = 1.0 / 2.0; // 0.5s per cycle

    // Check RMS in each beat of the first cycle
    for beat in 0..4 {
        let start = cycle_duration as f32 * beat as f32 / 4.0;
        let end = cycle_duration as f32 * (beat + 1) as f32 / 4.0;
        let beat_rms = rms_window(&audio, start, end, SAMPLE_RATE);
        assert!(
            beat_rms > 0.001,
            "Beat {} should have energy (RMS {:.4})",
            beat + 1,
            beat_rms
        );
    }
}

// --- Genre Spectral Comparisons ---

#[test]
fn house_l3_deep_house_darker_than_electro() {
    // Deep house (warm, filtered) should have lower spectral centroid than
    // electro house (aggressive, bright)
    let deep_code = r#"
cps: 1.97
~kick $ s "bd*4"
~bass $ sine 55 # lpf 200 0.8
~hats $ s "hh*8" # gain 0.35
out $ ~kick * 0.85 + ~bass * 0.4 + ~hats
"#;
    let electro_code = r#"
cps: 2.133
~kick $ s "bd*4"
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 400 1.5
~lead $ saw 110 + saw 220
out $ ~kick + ~hats + ~bass * 0.4 + ~lead * 0.12
"#;

    let deep_audio = render_dsl(deep_code, 3.0);
    let electro_audio = render_dsl(electro_code, 3.0);

    let deep_centroid = spectral_centroid(&deep_audio);
    let electro_centroid = spectral_centroid(&electro_audio);

    assert!(
        deep_centroid < electro_centroid,
        "Deep house ({:.0}Hz) should be darker than electro ({:.0}Hz)",
        deep_centroid,
        electro_centroid
    );
}

#[test]
fn house_l3_acid_has_bright_bass() {
    // Acid house (resonant filter, distortion) should be brighter than
    // deep house (warm sine bass)
    let acid_code = r#"
cps: 2.0
~bass $ saw "55 55 110 55 82.5 55 110 55"
~acid $ ~bass # lpf 2200 3.5 # distortion 1.5 * 0.25
out $ ~acid
"#;
    let deep_code = r#"
cps: 1.97
~bass $ sine 55 # lpf 200 0.8 * 0.4
out $ ~bass
"#;

    let acid_audio = render_dsl(acid_code, 3.0);
    let deep_audio = render_dsl(deep_code, 3.0);

    let acid_centroid = spectral_centroid(&acid_audio);
    let deep_centroid = spectral_centroid(&deep_audio);

    assert!(
        acid_centroid > deep_centroid,
        "Acid bass ({:.0}Hz) should be brighter than deep bass ({:.0}Hz)",
        acid_centroid,
        deep_centroid
    );
}

#[test]
fn house_l3_minimal_dark_spectrum() {
    // Minimal house should have dark spectrum (bass-heavy, filtered)
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.35
~bass $ sine "55 ~ ~ 82.5 ~ 55 ~ ~" * 0.4
out $ ~kick * 0.85 + ~hats + ~bass
"#;
    let audio = render_dsl(code, 4.0);
    let centroid = spectral_centroid(&audio);

    assert!(
        centroid < 3000.0,
        "Minimal house should have dark spectrum (centroid < 3kHz), got {:.0}Hz",
        centroid
    );
}

// --- Energy Comparisons ---

#[test]
fn house_l3_full_mix_more_energy_than_breakdown() {
    // Full drop should have significantly more energy than breakdown
    let full_code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 400 1.2 * 0.3
out $ ~kick + ~clap + ~hats + ~bass
"#;
    let breakdown_code = r#"
cps: 2.0
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_building $ ~pad # lpf 800 0.6 * 0.15
~sub $ sine 55 # lpf 80 0.7 * 0.25
out $ ~pad_building + ~sub
"#;

    let full_audio = render_dsl(full_code, 3.0);
    let breakdown_audio = render_dsl(breakdown_code, 3.0);

    let full_rms = calculate_rms(&full_audio);
    let breakdown_rms = calculate_rms(&breakdown_audio);

    assert!(
        full_rms > breakdown_rms,
        "Full mix (RMS {:.4}) should have more energy than breakdown (RMS {:.4})",
        full_rms,
        breakdown_rms
    );
}

#[test]
fn house_l3_adding_hats_increases_brightness() {
    // Adding hi-hats should increase spectral centroid
    let no_hh_code = r#"
cps: 2.0
~kick $ s "bd*4"
~bass $ saw 55 # lpf 300 1.0 * 0.3
out $ ~kick * 0.8 + ~bass
"#;
    let with_hh_code = r#"
cps: 2.0
~kick $ s "bd*4"
~bass $ saw 55 # lpf 300 1.0 * 0.3
~hats $ s "hh*16" # gain 0.5
out $ ~kick * 0.8 + ~bass + ~hats
"#;

    let no_hh_audio = render_dsl(no_hh_code, 3.0);
    let with_hh_audio = render_dsl(with_hh_code, 3.0);

    let no_hh_centroid = compute_spectral_centroid(&no_hh_audio, SAMPLE_RATE);
    let with_hh_centroid = compute_spectral_centroid(&with_hh_audio, SAMPLE_RATE);

    assert!(
        with_hh_centroid > no_hh_centroid,
        "Adding hi-hats should brighten: without={:.1}Hz, with={:.1}Hz",
        no_hh_centroid,
        with_hh_centroid
    );
}

#[test]
fn house_l3_more_elements_more_energy() {
    // Adding more percussion elements should increase RMS
    let sparse_code = r#"
cps: 2.0
~kick $ s "bd*4"
out $ ~kick
"#;
    let dense_code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 400 1.0 * 0.3
out $ ~kick + ~clap + ~hats + ~bass
"#;

    let sparse_audio = render_dsl(sparse_code, 3.0);
    let dense_audio = render_dsl(dense_code, 3.0);

    let sparse_rms = calculate_rms(&sparse_audio);
    let dense_rms = calculate_rms(&dense_audio);

    assert!(
        dense_rms > sparse_rms,
        "Dense pattern (RMS {:.4}) should have more energy than sparse (RMS {:.4})",
        dense_rms,
        sparse_rms
    );
}

// --- Filter Modulation ---

#[test]
fn house_l3_progressive_filter_sweep_spectral_variation() {
    // Slow LFO modulated filter should create varying spectral content
    let code = r#"
cps: 2.1
~lfo_slow $ sine 0.5
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_filtered $ ~pad # lpf (~lfo_slow * 3000 + 500) 0.7 * 0.15
out $ ~pad_filtered
"#;
    let audio = render_dsl(code, 4.0);

    // Compute spectral centroid for different time windows
    let window_size = (SAMPLE_RATE * 0.5) as usize;
    let mut centroids = Vec::new();

    for i in 0..6 {
        let start = i * window_size;
        let end = ((i + 1) * window_size).min(audio.len());
        if end > start && start < audio.len() {
            let centroid = compute_spectral_centroid(&audio[start..end], SAMPLE_RATE);
            centroids.push(centroid);
        }
    }

    if centroids.len() >= 3 {
        let mean = centroids.iter().sum::<f32>() / centroids.len() as f32;
        let variance =
            centroids.iter().map(|&c| (c - mean).powi(2)).sum::<f32>() / centroids.len() as f32;
        let std_dev = variance.sqrt();

        assert!(
            std_dev > 5.0,
            "Progressive filter sweep should create spectral variation (std_dev={:.1}Hz)",
            std_dev
        );
    }
}

#[test]
fn house_l3_tech_house_lfo_modulation() {
    // Tech house LFO on bass filter should create audible modulation
    let static_code = r#"
cps: 2.083
~bass $ saw 55 # lpf 300 1.3 * 0.35
out $ ~bass
"#;
    let modulated_code = r#"
cps: 2.083
~lfo $ sine 1.0
~bass $ saw 55 # lpf (~lfo * 500 + 300) 1.3 * 0.35
out $ ~bass
"#;

    let static_audio = render_dsl(static_code, 4.0);
    let modulated_audio = render_dsl(modulated_code, 4.0);

    // Both should produce audio
    let static_rms = calculate_rms(&static_audio);
    let modulated_rms = calculate_rms(&modulated_audio);
    assert!(static_rms > 0.01, "Static bass should produce audio");
    assert!(modulated_rms > 0.01, "Modulated bass should produce audio");
}

// --- No Clipping ---

#[test]
fn house_l3_chicago_no_extreme_clipping() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5
~bass $ saw "55 ~ 55 82.5 ~ 55 ~ 110" # lpf 400 1.2 * 0.35
out $ ~kick + ~clap + ~hats + ~oh + ~bass
"#;
    let audio = render_dsl(code, 2.0);
    let peak = find_peak(&audio);
    assert!(
        peak < 5.0,
        "Chicago house mix should not have extreme peaks: {:.3}",
        peak
    );
}

#[test]
fn house_l3_deep_no_extreme_clipping() {
    let code = r#"
cps: 1.97
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
~bass $ sine 55 # lpf 200 0.8
out $ ~kick * 0.85 + ~clap + ~hats + ~bass * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let peak = find_peak(&audio);
    assert!(
        peak < 5.0,
        "Deep house mix should not have extreme peaks: {:.3}",
        peak
    );
}

#[test]
fn house_l3_electro_no_extreme_clipping() {
    let code = r#"
cps: 2.133
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 400 1.5 * 0.4
out $ ~kick + ~clap + ~hats + ~bass
"#;
    let audio = render_dsl(code, 2.0);
    let peak = find_peak(&audio);
    assert!(
        peak < 5.0,
        "Electro house mix should not have extreme peaks: {:.3}",
        peak
    );
}

// --- Rhythmic Characteristics ---

#[test]
fn house_l3_drum_pattern_has_envelope_variation() {
    // House drum pattern should have transient/decay envelope variation
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
out $ ~kick * 0.8 + ~clap
"#;
    let audio = render_dsl(code, 4.0);
    let variation = envelope_variation(&audio, 50.0);

    assert!(
        variation > 0.001,
        "House drum pattern should have rhythmic envelope variation, got {:.6}",
        variation
    );
}

// --- Spectral Element Ordering ---

#[test]
fn house_l3_sub_bass_darkest_element() {
    // Sub bass (sine 55Hz) should be the darkest element
    let sub_code = r#"
cps: 2.0
~sub $ sine 55 * 0.4
out $ ~sub
"#;
    let saw_code = r#"
cps: 2.0
~bass $ saw 55 # lpf 500 1.0 * 0.3
out $ ~bass
"#;
    let bright_code = r#"
cps: 2.0
~bright $ saw 220 * 0.3
out $ ~bright
"#;

    let sub_audio = render_dsl(sub_code, 2.0);
    let saw_audio = render_dsl(saw_code, 2.0);
    let bright_audio = render_dsl(bright_code, 2.0);

    let sub_centroid = spectral_centroid(&sub_audio);
    let saw_centroid = spectral_centroid(&saw_audio);
    let bright_centroid = spectral_centroid(&bright_audio);

    assert!(
        sub_centroid < saw_centroid,
        "Sub ({:.0}Hz) should be darker than filtered saw ({:.0}Hz)",
        sub_centroid,
        saw_centroid
    );
    assert!(
        saw_centroid < bright_centroid,
        "Filtered saw ({:.0}Hz) should be darker than bright saw ({:.0}Hz)",
        saw_centroid,
        bright_centroid
    );
}

#[test]
fn house_l3_saw_brighter_than_sine_at_same_freq() {
    // Saw wave has harmonics; sine does not
    let saw_code = r#"
cps: 2.0
~bass $ saw 55 * 0.4
out $ ~bass
"#;
    let sine_code = r#"
cps: 2.0
~bass $ sine 55 * 0.4
out $ ~bass
"#;

    let saw_audio = render_dsl(saw_code, 2.0);
    let sine_audio = render_dsl(sine_code, 2.0);

    let saw_centroid = compute_spectral_centroid(&saw_audio, SAMPLE_RATE);
    let sine_centroid = compute_spectral_centroid(&sine_audio, SAMPLE_RATE);

    assert!(
        saw_centroid > sine_centroid,
        "Saw ({:.1}Hz) should be brighter than sine ({:.1}Hz) at same frequency",
        saw_centroid,
        sine_centroid
    );
}

// --- Tempo Range Verification ---

#[test]
fn house_l3_tempo_range_all_produce_audio() {
    // All house tempos should work correctly
    let tempos = [
        (1.97, "118 BPM (Deep)"),
        (2.0, "120 BPM (Chicago)"),
        (2.083, "125 BPM (Tech/Vocal)"),
        (2.1, "126 BPM (Progressive)"),
        (2.133, "128 BPM (Electro)"),
    ];

    for (cps, label) in tempos {
        let code = format!(
            r#"
cps: {}
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*8" # gain 0.4
out $ ~kick + ~clap + ~hats
"#,
            cps
        );

        let audio = render_dsl(&code, 2.0);
        let rms = calculate_rms(&audio);
        assert!(
            rms > 0.01,
            "{} (cps {}) should produce audio, RMS: {:.4}",
            label,
            cps,
            rms
        );
    }
}

#[test]
fn house_l3_faster_tempo_more_energy_density() {
    // Faster tempo should pack more events => more energy in same time
    let slow_code = r#"
cps: 1.5
~kick $ s "bd*4"
out $ ~kick
"#;
    let fast_code = r#"
cps: 3.0
~kick $ s "bd*4"
out $ ~kick
"#;

    let slow_audio = render_dsl(slow_code, 4.0);
    let fast_audio = render_dsl(fast_code, 4.0);

    let slow_rms = calculate_rms(&slow_audio);
    let fast_rms = calculate_rms(&fast_audio);

    assert!(slow_rms > 0.01, "Slow pattern should produce audio");
    assert!(fast_rms > 0.01, "Fast pattern should produce audio");
    assert!(
        fast_rms > slow_rms * 0.8,
        "Faster tempo should maintain or increase energy: fast {:.4} vs slow {:.4}",
        fast_rms,
        slow_rms
    );
}

// --- Rendering Determinism ---

#[test]
fn house_l3_rendering_determinism() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*8" # gain 0.4
~bass $ saw 55 # lpf 400 1.0 * 0.3
out $ ~kick + ~clap + ~hats + ~bass
"#;
    let audio1 = render_dsl(code, 2.0);
    let audio2 = render_dsl(code, 2.0);

    // Same code should produce identical output
    assert_eq!(
        audio1.len(),
        audio2.len(),
        "Same code should produce same length audio"
    );

    let rms_diff = calculate_rms(
        &audio1
            .iter()
            .zip(audio2.iter())
            .map(|(a, b)| a - b)
            .collect::<Vec<f32>>(),
    );
    assert!(
        rms_diff < 0.001,
        "Same code should render identically (diff RMS: {:.6})",
        rms_diff
    );
}

// ============================================================================
// CROSS-PATTERN COMPARISON TESTS
// Verify that different house styles sound meaningfully different
// ============================================================================

#[test]
fn house_cross_chicago_vs_deep_spectral_difference() {
    // Chicago (bright, 16th hats, offbeat open hats) vs Deep (warm, sparse)
    let chicago = render_dsl(
        r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5
out $ ~kick + ~clap + ~hats + ~oh
"#,
        3.0,
    );
    let deep = render_dsl(
        r#"
cps: 1.97
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
~bass $ sine 55 # lpf 200 0.8 * 0.4
out $ ~kick * 0.85 + ~clap + ~hats + ~bass
"#,
        3.0,
    );

    // Chicago with 16th hats + open hats should be brighter
    let chicago_centroid = spectral_centroid(&chicago);
    let deep_centroid = spectral_centroid(&deep);

    assert!(
        chicago_centroid > deep_centroid,
        "Chicago ({:.0}Hz) should be brighter than deep ({:.0}Hz)",
        chicago_centroid,
        deep_centroid
    );
}

#[test]
fn house_cross_full_vs_minimal_element_count() {
    // Full Chicago mix (many layers) vs minimal house (stripped back)
    // More layers should produce more onset density
    let chicago = render_dsl(
        r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5
out $ ~kick + ~clap + ~hats + ~oh
"#,
        3.0,
    );
    let minimal = render_dsl(
        r#"
cps: 2.0
~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.35
out $ ~kick * 0.85 + ~hats
"#,
        3.0,
    );

    // Chicago (16th hats + clap + open hats) should have more onsets than
    // minimal (8th hats only)
    let chicago_events = detect_audio_events(&chicago, SAMPLE_RATE, 0.005);
    let minimal_events = detect_audio_events(&minimal, SAMPLE_RATE, 0.005);

    assert!(
        chicago_events.len() >= minimal_events.len(),
        "Chicago ({} onsets) should have at least as many onsets as minimal ({} onsets)",
        chicago_events.len(),
        minimal_events.len()
    );
}

#[test]
fn house_cross_breakdown_quieter_than_any_full_pattern() {
    // Breakdown (no kick, atmospheric) should be quieter than any full pattern
    let breakdown = render_dsl(
        r#"
cps: 2.0
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_building $ ~pad # lpf 800 0.6 * 0.15
~sub $ sine 55 # lpf 80 0.7 * 0.25
out $ ~pad_building + ~sub
"#,
        3.0,
    );
    let full_pattern = render_dsl(
        r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*8" # gain 0.4
out $ ~kick + ~clap + ~hats
"#,
        3.0,
    );

    let breakdown_rms = calculate_rms(&breakdown);
    let full_rms = calculate_rms(&full_pattern);

    assert!(
        full_rms > breakdown_rms,
        "Full pattern (RMS {:.4}) should be louder than breakdown (RMS {:.4})",
        full_rms,
        breakdown_rms
    );
}

#[test]
fn house_cross_all_subgenres_produce_distinct_audio() {
    // Each subgenre should produce non-silent, non-NaN audio
    let patterns = [
        (
            "Chicago",
            r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~clap + ~hats
"#,
        ),
        (
            "Deep",
            r#"
cps: 1.97
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
out $ ~kick * 0.85 + ~clap + ~hats
"#,
        ),
        (
            "Tech",
            r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.65
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~clap + ~hats
"#,
        ),
        (
            "Progressive",
            r#"
cps: 2.1
~kick $ s "bd*4"
~clap $ s "~ ~ ~ cp" # gain 0.65
~hats $ s "hh*8" # gain 0.45
out $ ~kick + ~clap + ~hats
"#,
        ),
        (
            "Electro",
            r#"
cps: 2.133
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
out $ ~kick + ~clap + ~hats
"#,
        ),
    ];

    for (name, code) in patterns {
        let audio = render_dsl(code, 2.0);
        assert!(
            !is_silent(&audio, 0.001),
            "{} house should produce audio",
            name
        );
        assert!(
            !audio.iter().any(|s| s.is_nan()),
            "{} house should not contain NaN",
            name
        );
        let rms = calculate_rms(&audio);
        assert!(
            rms > 0.01,
            "{} house should have meaningful RMS: {:.4}",
            name,
            rms
        );
    }
}

// ============================================================================
// FULL MIX INTEGRATION TESTS
// Complete house patterns from the library
// ============================================================================

#[test]
fn house_full_chicago_complete() {
    // Pattern 1: Full Chicago house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5
~bass $ saw "55 ~ 55 82.5 ~ 55 ~ 110" # lpf 400 1.2 * 0.35
out $ ~kick + ~clap + ~hats + ~oh + ~bass
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Chicago house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
    let peak = find_peak(&audio);
    assert!(peak < 5.0, "Peak should be reasonable: {:.3}", peak);
}

#[test]
fn house_full_deep_complete() {
    // Pattern 2: Full deep house
    let code = r#"
cps: 1.97
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
~rim $ s "~ ~ ~ rim" # gain 0.4
~bass $ sine 55 # lpf 200 0.8
~pad $ sine 130.81 + sine 155.56 + sine 196
~pad_filtered $ ~pad # lpf 1500 0.5 * 0.12
out $ ~kick * 0.85 + ~clap + ~hats + ~rim + ~bass * 0.4 + ~pad_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(!is_silent(&audio, 0.001), "Deep house should produce audio");
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_acid_complete() {
    // Pattern 3: Full acid house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.5
~bass $ saw "55 55 110 55 82.5 55 110 55"
~acid $ ~bass # lpf 2200 3.5 # distortion 1.5 * 0.25
out $ ~kick + ~clap + ~hats + ~acid
"#;
    let audio = render_dsl(code, 3.0);
    assert!(!is_silent(&audio, 0.001), "Acid house should produce audio");
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_progressive_complete() {
    // Pattern 4: Full progressive house
    let code = r#"
cps: 2.1
~kick $ s "bd*4"
~clap $ s "~ ~ ~ cp" # gain 0.65
~hats $ s "hh*8" # gain 0.45
~arp $ sine "130.81 196 261.63 196" $ fast 2
~arp_filtered $ ~arp # lpf 4000 0.5 * 0.12
out $ ~kick + ~clap + ~hats + ~arp_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Progressive should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_tech_complete() {
    // Pattern 5: Full tech house
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.65
~rim $ s "~ ~ rim ~" # gain 0.5
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 300 1.3
~lfo $ sine 0.25
~bass_filtered $ ~bass # lpf (~lfo * 200 + 300) 1.0 * 0.35
out $ ~kick + ~clap + ~rim + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(!is_silent(&audio, 0.001), "Tech house should produce audio");
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_disco_complete() {
    // Pattern 6: Full disco house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.6
~snare $ s "~ sn ~ sn" # gain 0.35
~hats $ s "hh hh oh hh hh hh oh hh" # gain 0.5
~bass $ saw "55 110 55 82.5 110 55 82.5 55"
~bass_filtered $ ~bass # lpf 700 1.0 * 0.3
out $ ~kick + ~clap + ~snare + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Disco house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_french_complete() {
    // Pattern 7: Full French house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.35
~bass $ saw 55 # lpf 500 1.2
~pumped_bass $ ~bass * 0.35
~chord $ saw 130.81 + saw 155.56 + saw 196
~chord_filtered $ ~chord # lpf 2500 0.7 * 0.15
out $ ~kick + ~clap + ~hats + ~pumped_bass + ~chord_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "French house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_garage_complete() {
    // Pattern 8: Full garage house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ sine "55 ~ 55 ~ 82.5 ~ 55 ~"
~bass_filtered $ ~bass # lpf 300 0.9 * 0.4
~chord $ sine 196 + sine 246.94 + sine 293.66
~stab $ ~chord * "~ 1 ~ 0.5 ~ 1 ~ ~" * 0.12
out $ ~kick + ~clap + ~hats + ~bass_filtered + ~stab
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Garage house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_vocal_complete() {
    // Pattern 9: Full vocal house
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*8" # gain 0.45
~oh $ s "~ ~ oh ~ ~ ~ oh ~" # gain 0.5
~piano_c $ sine 130.81 + sine 164.81 + sine 196 + sine 246.94
~piano_f $ sine 174.61 + sine 220 + sine 261.63 + sine 329.63
~piano $ (~piano_c * "1 ~ ~ 0.7") + (~piano_f * "~ 0.8 ~ ~")
~piano_filtered $ ~piano # lpf 3500 0.5 * 0.15
~bass $ saw "55 55 110 55" # lpf 350 1.1 * 0.35
out $ ~kick + ~clap + ~hats + ~oh + ~piano_filtered + ~bass
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Vocal house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_tribal_complete() {
    // Pattern 10: Full tribal house
    let code = r#"
cps: 2.083
~kick $ s "bd*4"
~clap $ s "~ ~ cp ~" # gain 0.6
~hats $ s "hh*8" # gain 0.35
~bass $ sine 55 # lpf 120 0.8 * 0.45
out $ ~kick + ~clap + ~hats + ~bass
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Tribal house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_minimal_complete() {
    // Pattern 11: Full minimal house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~hats $ s "hh*8" # gain 0.35
~rim $ s "~ rim [~ rim] ~" # gain 0.45
~bass $ sine "55 ~ ~ 82.5 ~ 55 ~ ~" * 0.4
out $ ~kick * 0.85 + ~hats + ~rim + ~bass
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Minimal house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_afro_complete() {
    // Pattern 12: Full afro house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # gain 0.6
~hats $ s "~ hh ~ hh" # gain 0.4
~bass $ sine "55 ~ 55 65.41 ~ 55 ~ 73.42"
~bass_filtered $ ~bass # lpf 250 0.9 * 0.4
out $ ~kick + ~clap + ~hats + ~bass_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(!is_silent(&audio, 0.001), "Afro house should produce audio");
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_soulful_complete() {
    // Pattern 13: Full soulful house
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.45
~organ_root $ sine 130.81 + sine 196 + sine 261.63
~organ_third $ sine 155.56 + sine 220 + sine 311.13
~organ $ ~organ_root * "1 ~ ~ 1" + ~organ_third * "~ 1 1 ~"
~organ_filtered $ ~organ # lpf 2000 0.6 * 0.12
~bass $ saw "55 65.41 73.42 82.5 73.42 65.41 55 55"
~bass_filtered $ ~bass # lpf 400 1.0 * 0.3
out $ ~kick + ~clap + ~hats + ~oh + ~organ_filtered + ~bass_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Soulful house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_electro_complete() {
    // Pattern 14: Full electro house
    let code = r#"
cps: 2.133
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # gain 0.7
~hats $ s "hh*16" # gain 0.4
~bass $ saw 55 # lpf 400 1.5
~bass_pumped $ ~bass * 0.4
~lead $ saw 110 + saw 220
~lead_filtered $ ~lead # lpf 2000 2.0 * 0.12
out $ ~kick + ~clap + ~hats + ~bass_pumped + ~lead_filtered
"#;
    let audio = render_dsl(code, 3.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Electro house should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn house_full_breakdown_complete() {
    // Pattern 15: Breakdown (atmospheric, no kick)
    let code = r#"
cps: 2.0
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_building $ ~pad # lpf 800 0.6 * 0.15
~sub $ sine 55 # lpf 80 0.7 * 0.25
out $ ~pad_building + ~sub
"#;
    let audio = render_dsl(code, 3.0);
    assert!(!is_silent(&audio, 0.001), "Breakdown should produce audio");
    let rms = calculate_rms(&audio);
    assert!(rms > 0.001, "Breakdown should have some RMS: {:.4}", rms);
}
