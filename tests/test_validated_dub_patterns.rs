//! Validated Tests: Dub/Reggae patterns match reference characteristics
//!
//! Verifies that Phonon's dub and reggae patterns produce audio matching
//! the documented musical characteristics of these genres:
//!
//! - ONE DROP: Beat 1 is silent ("dropped"), kick+rimshot on beat 3
//! - ROCKERS: Kick on beats 1 & 3, snare on 2 & 4
//! - STEPPERS: Four-on-floor kick, driving feel
//! - DUB BASS: Deep, melodic, emphasizing root and fifth
//! - FILTER MODULATION: LFO-controlled sweeps (signature dub technique)
//!
//! Uses three-level verification methodology:
//!   Level 1: Pattern query verification (event counts, timing)
//!   Level 2: Onset detection (audio events at correct times)
//!   Level 3: Audio characteristics (RMS, spectral content, modulation)

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
use audio_test_utils::{calculate_rms, compute_spectral_centroid, find_peak};

mod pattern_verification_utils;
use pattern_verification_utils::{detect_audio_events, is_silent};

// ============================================================================
// HELPERS
// ============================================================================

fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let samples = (44100.0 * duration_secs) as usize;
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

// ============================================================================
// LEVEL 1: PATTERN QUERY VERIFICATION
// Tests pattern logic without rendering audio
// ============================================================================

#[test]
fn dub_l1_one_drop_kick_on_beat3_only() {
    // One Drop: "~ ~ bd ~" => kick only on beat 3 (position 2/4)
    let kick_pattern = parse_mini_notation("~ ~ bd ~");
    let events = query_single_cycle(&kick_pattern);

    // Should have exactly 1 event (bd on beat 3)
    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "One drop kick should have exactly 1 hit per cycle, got {}",
        non_rest.len()
    );

    // The hit should be at position 2/4 = 0.5 of the cycle
    let kick_pos = non_rest[0].part.begin.to_float();
    assert!(
        (kick_pos - 0.5).abs() < 0.01,
        "One drop kick should be at beat 3 (0.5 of cycle), got {}",
        kick_pos
    );
}

#[test]
fn dub_l1_one_drop_beat1_is_silent() {
    // One Drop: beat 1 must be "dropped" (silent)
    let kick_pattern = parse_mini_notation("~ ~ bd ~");
    let events = query_single_cycle(&kick_pattern);

    // Check that no non-rest event starts at or near 0.0 (beat 1)
    let beat1_events: Vec<_> = events
        .iter()
        .filter(|h| h.value != "~" && h.part.begin.to_float() < 0.25)
        .collect();
    assert!(
        beat1_events.is_empty(),
        "One drop should have NO hits on beat 1, got {}",
        beat1_events.len()
    );
}

#[test]
fn dub_l1_rockers_kick_on_1_and_3() {
    // Rockers: "bd ~ bd ~" => kick on beats 1 and 3
    let kick_pattern = parse_mini_notation("bd ~ bd ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Rockers kick should have 2 hits per cycle, got {}",
        non_rest.len()
    );

    // First kick at beat 1 (0.0), second at beat 3 (0.5)
    let positions: Vec<f64> = non_rest.iter().map(|h| h.part.begin.to_float()).collect();
    assert!(
        (positions[0] - 0.0).abs() < 0.01,
        "First kick should be at beat 1 (0.0), got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.5).abs() < 0.01,
        "Second kick should be at beat 3 (0.5), got {}",
        positions[1]
    );
}

#[test]
fn dub_l1_rockers_snare_on_2_and_4() {
    // Rockers: "~ sn ~ sn" => snare on beats 2 and 4
    let snare_pattern = parse_mini_notation("~ sn ~ sn");
    let events = query_single_cycle(&snare_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        2,
        "Rockers snare should have 2 hits per cycle, got {}",
        non_rest.len()
    );

    // Snare at beat 2 (0.25) and beat 4 (0.75)
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
fn dub_l1_steppers_four_on_floor() {
    // Steppers: "bd*4" => kick on all 4 beats
    let kick_pattern = parse_mini_notation("bd*4");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Steppers should have 4 kicks per cycle, got {}",
        non_rest.len()
    );

    // Evenly spaced at 0.0, 0.25, 0.5, 0.75
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
fn dub_l1_hihat_8ths_count() {
    // Hi-hat pattern for reggae: "hh*8" => 8 hits per cycle
    let hh_pattern = parse_mini_notation("hh*8");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 32,
        "hh*8 over 4 cycles should produce 32 events, got {}",
        count
    );
}

#[test]
fn dub_l1_hihat_16ths_count() {
    // Steppers hi-hat: "hh*16" => 16 hits per cycle
    let hh_pattern = parse_mini_notation("hh*16");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 64,
        "hh*16 over 4 cycles should produce 64 events, got {}",
        count
    );
}

#[test]
fn dub_l1_skank_offbeat_pattern() {
    // Skank: "[~ rim]*4" => offbeat hits (rest then hit, repeated 4 times)
    let skank_pattern = parse_mini_notation("[~ rim]*4");
    let events = query_single_cycle(&skank_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        4,
        "Skank should have 4 offbeat hits per cycle, got {}",
        non_rest.len()
    );

    // Each hit should be on the "and" (second half of each beat subdivision)
    // [~ rim]*4 gives 8 slots total; rim at positions 1, 3, 5, 7 of 8
    // That's 1/8, 3/8, 5/8, 7/8 of the cycle
    for (i, event) in non_rest.iter().enumerate() {
        let expected = (2.0 * i as f64 + 1.0) / 8.0;
        let actual = event.part.begin.to_float();
        assert!(
            (actual - expected).abs() < 0.02,
            "Skank hit {} should be at {:.3} (offbeat), got {:.3}",
            i,
            expected,
            actual
        );
    }
}

#[test]
fn dub_l1_euclidean_one_drop() {
    // Euclidean one drop: "bd(1,4,2)" => 1 hit in 4 slots, rotated by 2
    let pattern = parse_mini_notation("bd(1,4,2)");
    let events = query_single_cycle(&pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        1,
        "Euclidean bd(1,4,2) should have 1 hit per cycle, got {}",
        non_rest.len()
    );

    // With rotation=2, the hit should be at position 2/4 = 0.5
    let pos = non_rest[0].part.begin.to_float();
    assert!(
        (pos - 0.5).abs() < 0.01,
        "Euclidean bd(1,4,2) should hit at 0.5 (beat 3), got {}",
        pos
    );
}

// ============================================================================
// LEVEL 2: ONSET DETECTION
// Tests that rendered audio has events at the right times
// ============================================================================

#[test]
fn dub_l2_one_drop_renders_audio() {
    // Classic one drop should produce non-silent audio
    let code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~rim $ s "~ ~ rim ~"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~rim * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "One drop pattern should produce audible output"
    );
}

#[test]
fn dub_l2_one_drop_onset_count() {
    // One drop: kick on beat 3 + hi-hats on 8ths
    // Over ~2 cycles at 1.167 cps: expect several onsets
    let code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.01);

    // Should have multiple onsets (hi-hats + kick)
    assert!(
        events.len() >= 4,
        "One drop should have at least 4 onsets over 2 seconds, got {}",
        events.len()
    );
}

#[test]
fn dub_l2_rockers_renders_audio() {
    let code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Rockers pattern should produce audible output"
    );
}

#[test]
fn dub_l2_rockers_has_regular_kick_pattern() {
    // Rockers: kick on 1 and 3 => every half-cycle
    // At cps=1.25, one cycle = 0.8s, so kicks at 0, 0.4, 0.8, 1.2...
    let code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
out $ ~kick * 0.8
"#;
    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.01);

    // Over 2 seconds at cps 1.25 = 2.5 cycles, expect ~5 kicks
    assert!(
        events.len() >= 3,
        "Rockers kick should produce at least 3 onsets over 2 seconds, got {}",
        events.len()
    );
}

#[test]
fn dub_l2_steppers_renders_audio() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Steppers pattern should produce audible output"
    );
}

#[test]
fn dub_l2_steppers_has_more_events_than_one_drop() {
    // Steppers (4-on-floor) should have more onsets than one drop
    let one_drop_code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
out $ ~kick * 0.8
"#;
    let steppers_code = r#"
cps: 2.0
~kick $ s "bd*4"
out $ ~kick * 0.8
"#;

    let one_drop_audio = render_dsl(one_drop_code, 2.0);
    let steppers_audio = render_dsl(steppers_code, 2.0);

    let one_drop_events = detect_audio_events(&one_drop_audio, 44100.0, 0.01);
    let steppers_events = detect_audio_events(&steppers_audio, 44100.0, 0.01);

    assert!(
        steppers_events.len() > one_drop_events.len(),
        "Steppers ({} events) should have more onsets than one drop ({} events)",
        steppers_events.len(),
        one_drop_events.len()
    );
}

#[test]
fn dub_l2_dub_bass_renders_audio() {
    // Dub bass with filtered saw wave
    let code = r#"
cps: 1.167
~bass $ saw "55 ~ 82.5 55" # lpf 200 0.8
out $ ~bass * 0.5
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Dub bass should produce audible output"
    );
}

#[test]
fn dub_l2_dub_siren_renders_audio() {
    // Dub siren with changing pitch
    let code = r#"
cps: 1.167
~siren $ sine "880 1320 880 660"
out $ ~siren * 0.3
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Dub siren should produce audible output"
    );
}

// ============================================================================
// LEVEL 3: AUDIO CHARACTERISTICS
// Verifies signal properties match dub/reggae reference characteristics
// ============================================================================

#[test]
fn dub_l3_one_drop_beat1_quieter_than_beat3() {
    // Defining characteristic of one-drop: beat 1 is silent, beat 3 has the hit
    // At cps 1.167, cycle = 0.857s, beat 3 starts at ~0.43s
    let code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~rim $ s "~ ~ rim ~"
out $ ~kick * 0.8 + ~rim * 0.6
"#;
    let audio = render_dsl(code, 1.5);
    let sample_rate = 44100.0;
    let cycle_duration = 1.0 / 1.167;

    // Beat 1 region (first quarter of first cycle)
    let beat1_rms = rms_window(&audio, 0.0, cycle_duration as f32 * 0.25, sample_rate);
    // Beat 3 region (third quarter of first cycle)
    let beat3_start = cycle_duration as f32 * 0.5;
    let beat3_end = cycle_duration as f32 * 0.75;
    let beat3_rms = rms_window(&audio, beat3_start, beat3_end, sample_rate);

    assert!(
        beat3_rms > beat1_rms,
        "One drop: beat 3 (RMS {:.4}) should be louder than beat 1 (RMS {:.4})",
        beat3_rms,
        beat1_rms
    );
}

#[test]
fn dub_l3_rockers_has_regular_energy_distribution() {
    // Rockers: kick on 1&3, snare on 2&4 => energy in every quarter
    let code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
~snare $ s "~ sn ~ sn"
out $ ~kick * 0.8 + ~snare * 0.6
"#;
    let audio = render_dsl(code, 1.5);
    let sample_rate = 44100.0;
    let cycle_duration = 1.0 / 1.25;

    // Check RMS in each beat of the first cycle
    let mut beat_rms = Vec::new();
    for beat in 0..4 {
        let start = cycle_duration as f32 * beat as f32 / 4.0;
        let end = cycle_duration as f32 * (beat + 1) as f32 / 4.0;
        beat_rms.push(rms_window(&audio, start, end, sample_rate));
    }

    // All beats should have some energy (kick or snare)
    for (i, &rms) in beat_rms.iter().enumerate() {
        assert!(
            rms > 0.001,
            "Rockers: beat {} should have energy (RMS {:.4}), got near-silence",
            i + 1,
            rms
        );
    }
}

#[test]
fn dub_l3_steppers_more_rms_than_one_drop() {
    // Steppers (4-on-floor at 120 BPM) should have more total energy than
    // one-drop (sparse pattern at 70 BPM)
    let one_drop_code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
out $ ~kick * 0.8
"#;
    let steppers_code = r#"
cps: 2.0
~kick $ s "bd*4"
out $ ~kick * 0.8
"#;

    let one_drop_audio = render_dsl(one_drop_code, 2.0);
    let steppers_audio = render_dsl(steppers_code, 2.0);

    let one_drop_rms = calculate_rms(&one_drop_audio);
    let steppers_rms = calculate_rms(&steppers_audio);

    assert!(
        steppers_rms > one_drop_rms,
        "Steppers (RMS {:.4}) should have more energy than one drop (RMS {:.4})",
        steppers_rms,
        one_drop_rms
    );
}

#[test]
fn dub_l3_bass_saw_has_harmonic_content() {
    // A saw wave at 55 Hz should produce audio with significant harmonic content
    // (saw waves are rich in harmonics, making them ideal for dub bass)
    let code = r#"
cps: 1.167
~bass $ saw 55
out $ ~bass * 0.5
"#;
    let audio = render_dsl(code, 2.0);

    // Should produce non-silent audio
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Saw bass should produce audible output (RMS {:.4})",
        rms
    );

    // Saw wave should have a higher centroid than a pure sine at the same frequency
    // (because saw waves contain all harmonics)
    let sine_code = r#"
cps: 1.167
~bass $ sine 55
out $ ~bass * 0.5
"#;
    let sine_audio = render_dsl(sine_code, 2.0);

    let saw_centroid = compute_spectral_centroid(&audio, 44100.0);
    let sine_centroid = compute_spectral_centroid(&sine_audio, 44100.0);

    assert!(
        saw_centroid > sine_centroid,
        "Saw wave ({:.1} Hz) should be brighter than sine ({:.1} Hz) at same fundamental",
        saw_centroid,
        sine_centroid
    );
}

#[test]
fn dub_l3_bass_lower_frequency_than_bright_synth() {
    // Dub bass at 55 Hz should have a lower spectral centroid than
    // a bright synth at higher frequencies, verifying the bass sits
    // in the low end of the spectrum as dub/reggae demands
    let bass_code = r#"
cps: 1.167
~bass $ saw 55
out $ ~bass * 0.5
"#;
    let bright_code = r#"
cps: 1.167
~bright $ saw 440
out $ ~bright * 0.5
"#;

    let bass_audio = render_dsl(bass_code, 2.0);
    let bright_audio = render_dsl(bright_code, 2.0);

    let bass_centroid = compute_spectral_centroid(&bass_audio, 44100.0);
    let bright_centroid = compute_spectral_centroid(&bright_audio, 44100.0);

    assert!(
        bass_centroid < bright_centroid,
        "Bass at 55Hz ({:.1} Hz centroid) should be darker than synth at 440Hz ({:.1} Hz centroid)",
        bass_centroid,
        bright_centroid
    );
}

#[test]
fn dub_l3_full_one_drop_no_clipping() {
    // Full one drop mix should not clip
    let code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~rim $ s "~ ~ rim ~"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~rim * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let peak = find_peak(&audio);

    // Peak should be reasonable (under 2.0 to account for summing)
    assert!(
        peak < 2.0,
        "One drop mix should not have extreme peaks: peak = {:.3}",
        peak
    );
}

#[test]
fn dub_l3_full_rockers_no_clipping() {
    let code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let peak = find_peak(&audio);

    assert!(
        peak < 2.0,
        "Rockers mix should not have extreme peaks: peak = {:.3}",
        peak
    );
}

#[test]
fn dub_l3_full_steppers_no_clipping() {
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let peak = find_peak(&audio);

    assert!(
        peak < 2.0,
        "Steppers mix should not have extreme peaks: peak = {:.3}",
        peak
    );
}

#[test]
fn dub_l3_hihat_adds_high_frequency_content() {
    // Adding hi-hats should increase spectral centroid (brighter sound)
    let no_hh_code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
out $ ~kick * 0.8
"#;
    let with_hh_code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~hats * 0.4
"#;

    let no_hh_audio = render_dsl(no_hh_code, 2.0);
    let with_hh_audio = render_dsl(with_hh_code, 2.0);

    let no_hh_centroid = compute_spectral_centroid(&no_hh_audio, 44100.0);
    let with_hh_centroid = compute_spectral_centroid(&with_hh_audio, 44100.0);

    assert!(
        with_hh_centroid > no_hh_centroid,
        "Adding hi-hats should brighten the mix: without={:.1}Hz, with={:.1}Hz",
        no_hh_centroid,
        with_hh_centroid
    );
}

#[test]
fn dub_l3_filter_modulation_varies_spectral_content() {
    // LFO-modulated filter should create varying spectral content over time
    let code = r#"
cps: 2.0
~lfo $ sine 0.5
~bass $ saw 55
out $ ~bass # lpf (~lfo * 1500 + 800) 0.5
"#;
    let audio = render_dsl(code, 4.0);
    let sample_rate = 44100.0;

    // Compute spectral centroid for different time windows
    let window_size = (sample_rate * 0.5) as usize;
    let mut centroids = Vec::new();

    for i in 0..6 {
        let start = i * window_size;
        let end = ((i + 1) * window_size).min(audio.len());
        if end > start && start < audio.len() {
            let centroid = compute_spectral_centroid(&audio[start..end], sample_rate);
            centroids.push(centroid);
        }
    }

    if centroids.len() >= 3 {
        // Compute variation in centroids
        let mean = centroids.iter().sum::<f32>() / centroids.len() as f32;
        let variance =
            centroids.iter().map(|&c| (c - mean).powi(2)).sum::<f32>() / centroids.len() as f32;
        let std_dev = variance.sqrt();

        // LFO modulation should cause measurable spectral variation
        assert!(
            std_dev > 10.0,
            "Filter modulation should create spectral variation (std_dev={:.1}Hz, centroids={:?})",
            std_dev,
            centroids
        );
    }
}

// ============================================================================
// CROSS-PATTERN COMPARISON TESTS
// Verify that different dub styles sound meaningfully different
// ============================================================================

#[test]
fn dub_cross_one_drop_vs_rockers_different_energy_profile() {
    // One drop has sparse beat 1, rockers has kick on beat 1
    let one_drop = render_dsl(
        r#"
cps: 1.25
~kick $ s "~ ~ bd ~"
out $ ~kick * 0.8
"#,
        1.5,
    );
    let rockers = render_dsl(
        r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
out $ ~kick * 0.8
"#,
        1.5,
    );

    let cycle_dur = 1.0 / 1.25;

    // Check beat 1 region
    let one_drop_beat1 = rms_window(&one_drop, 0.0, cycle_dur as f32 * 0.2, 44100.0);
    let rockers_beat1 = rms_window(&rockers, 0.0, cycle_dur as f32 * 0.2, 44100.0);

    // Rockers should have energy on beat 1, one drop should not
    assert!(
        rockers_beat1 > one_drop_beat1,
        "Rockers (RMS {:.4}) should have more beat-1 energy than one drop (RMS {:.4})",
        rockers_beat1,
        one_drop_beat1
    );
}

#[test]
fn dub_cross_steppers_vs_one_drop_event_density() {
    // Steppers at 120 BPM with 4-on-floor should have much higher onset density
    // than one-drop at 70 BPM with kick only on beat 3
    let one_drop = render_dsl(
        r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~hats * 0.4
"#,
        3.0,
    );
    let steppers = render_dsl(
        r#"
cps: 2.0
~kick $ s "bd*4"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~hats * 0.4
"#,
        3.0,
    );

    let one_drop_events = detect_audio_events(&one_drop, 44100.0, 0.005);
    let steppers_events = detect_audio_events(&steppers, 44100.0, 0.005);

    assert!(
        steppers_events.len() > one_drop_events.len(),
        "Steppers ({} events) should have higher onset density than one drop ({} events)",
        steppers_events.len(),
        one_drop_events.len()
    );
}

#[test]
fn dub_cross_three_patterns_produce_different_audio() {
    // The three core dub patterns should produce distinctly different audio
    let one_drop = render_dsl(
        r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~snare $ s "~ ~ rim ~"
out $ ~kick * 0.8 + ~snare * 0.6
"#,
        2.0,
    );
    let rockers = render_dsl(
        r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
~snare $ s "~ sn ~ sn"
out $ ~kick * 0.8 + ~snare * 0.6
"#,
        2.0,
    );
    let steppers = render_dsl(
        r#"
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn"
out $ ~kick * 0.8 + ~snare * 0.6
"#,
        2.0,
    );

    // Each should produce audible audio
    assert!(
        !is_silent(&one_drop, 0.001),
        "One drop should not be silent"
    );
    assert!(!is_silent(&rockers, 0.001), "Rockers should not be silent");
    assert!(
        !is_silent(&steppers, 0.001),
        "Steppers should not be silent"
    );

    // Steppers should have more total energy due to faster tempo and more hits
    let od_rms = calculate_rms(&one_drop);
    let rk_rms = calculate_rms(&rockers);
    let st_rms = calculate_rms(&steppers);

    assert!(
        st_rms > od_rms,
        "Steppers (RMS {:.4}) should have more energy than one drop (RMS {:.4})",
        st_rms,
        od_rms
    );
    assert!(
        rk_rms > od_rms * 0.5,
        "Rockers (RMS {:.4}) should have meaningful energy compared to one drop (RMS {:.4})",
        rk_rms,
        od_rms
    );
}

// ============================================================================
// FULL MIX INTEGRATION TESTS
// Tests complete dub patterns from the pattern library
// ============================================================================

#[test]
fn dub_full_one_drop_with_bass() {
    // Pattern 2 from library: one drop with bass
    let code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~rim $ s "~ ~ rim ~"
~hats $ s "hh*8"
~bass $ saw "55 ~ 82.5 55" # lpf 180 0.9
out $ ~kick * 0.8 + ~rim * 0.5 + ~hats * 0.4 + ~bass * 0.5
"#;
    let audio = render_dsl(code, 3.0);

    // Should produce audible output
    assert!(
        !is_silent(&audio, 0.001),
        "Full one drop with bass should produce audio"
    );

    // Should have reasonable levels
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Full mix should have meaningful RMS: {:.4}",
        rms
    );

    let peak = find_peak(&audio);
    assert!(
        peak < 3.0,
        "Full mix peak should be reasonable: {:.3}",
        peak
    );
}

#[test]
fn dub_full_rockers_with_skank() {
    // Pattern 4 from library: rockers with offbeat skank
    let code = r#"
cps: 1.25
~kick $ s "bd ~ bd ~"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*8"
~skank $ s "[~ rim]*4"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~skank * 0.4
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Rockers with skank should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn dub_full_steppers_dub() {
    // Pattern 6 from library: steppers with filter modulation
    let code = r#"
cps: 2.0
~kick $ s "bd*4"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*16"
~bass $ saw 55 # lpf 150 0.9
~lfo $ sine 0.5
~mix $ (~kick + ~snare + ~hats) * 0.6 + ~bass * 0.4
out $ ~mix # lpf (~lfo * 1500 + 800) 0.5
"#;
    let audio = render_dsl(code, 4.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Steppers dub should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.005, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn dub_full_heavy_dub_with_filter_lfo() {
    // Pattern 10: heavy dub with filter modulation
    let code = r#"
cps: 1.083
~kick $ s "~ ~ bd ~"
~rim $ s "~ ~ rim ~"
~hats $ s "hh*8"
~bass $ saw "55 ~ 82.5 55" # lpf 120 0.95
~lfo $ sine 0.25
~drums $ ~kick * 0.8 + ~rim * 0.5 + ~hats * 0.4
out $ ~drums # lpf (~lfo * 2000 + 500) 0.6 + ~bass * 0.6
"#;
    let audio = render_dsl(code, 4.0);

    assert!(!is_silent(&audio, 0.001), "Heavy dub should produce audio");

    let rms = calculate_rms(&audio);
    assert!(rms > 0.005, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn dub_full_dub_echo_simulation() {
    // Pattern 8: simulated delay echo using pattern offsets
    let code = r#"
cps: 1.167
~kick $ s "~ ~ bd ~"
~rim $ s "rim ~ ~ ~"
~echo1 $ s "~ rim ~ ~"
~echo2 $ s "~ ~ rim ~"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~rim * 0.6 + ~echo1 * 0.4 + ~echo2 * 0.2 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 3.0);

    assert!(!is_silent(&audio, 0.001), "Dub echo should produce audio");

    let events = detect_audio_events(&audio, 44100.0, 0.005);
    assert!(
        events.len() >= 6,
        "Echo pattern should have multiple onsets (rim + echoes + hats), got {}",
        events.len()
    );
}

#[test]
fn dub_full_lovers_rock() {
    // Pattern 11: smoother lovers rock
    let code = r#"
cps: 1.2
~kick $ s "bd ~ bd ~"
~rim $ s "~ rim ~ rim"
~hats $ s "hh*8"
out $ ~kick * 0.7 + ~rim * 0.5 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Lovers rock should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn dub_full_militant_steppers() {
    // Pattern 12: faster, more aggressive steppers
    let code = r#"
cps: 2.167
~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~clap * 0.7 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Militant steppers should produce audio"
    );

    // Higher tempo should result in good event density
    let events = detect_audio_events(&audio, 44100.0, 0.005);
    assert!(
        events.len() >= 8,
        "Militant steppers should have high onset density, got {}",
        events.len()
    );
}

#[test]
fn dub_full_dub_techno_hybrid() {
    // Pattern 15: dub techno hybrid with filter sweep
    let code = r#"
cps: 1.833
~kick $ s "bd*4"
~rim $ s "~ rim ~ rim"
~hats $ s "hh*16"
~bass $ saw "55 55 82.5 110" # lpf 200 0.8
~lfo $ sine 0.125
~mix $ ~kick * 0.7 + ~rim * 0.5 + ~hats * 0.4 + ~bass * 0.5
out $ ~mix # lpf (~lfo * 3000 + 500) 0.4
"#;
    let audio = render_dsl(code, 4.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Dub techno hybrid should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.005, "Should have meaningful RMS: {:.4}", rms);
}
