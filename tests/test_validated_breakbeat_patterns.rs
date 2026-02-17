//! Validated Tests: Breakbeat patterns match reference characteristics
//!
//! Verifies that Phonon's breakbeat, jungle, and breakcore patterns produce
//! audio matching the documented musical characteristics of these genres:
//!
//! - AMEN BREAK: Syncopated kick+snare pattern with 8th note hats
//! - JUNGLE: Fast tempo (160-175 BPM), syncopated breaks, deep bass
//! - BREAKCORE: Extreme speed (200+ BPM), rapid-fire subdivisions
//! - CHOPPED BREAKS: Rearranged sample slices (amencutup)
//! - PATTERN TRANSFORMS: Swing, Euclidean, ghost notes, polyrhythms
//!
//! Uses three-level verification methodology:
//!   Level 1: Pattern query verification (event counts, timing, density, syncopation)
//!   Level 2: Onset detection (audio events at correct times)
//!   Level 3: Audio characteristics (RMS, spectral content, no clipping)

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_metrics::PatternMetrics;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::{calculate_rms, compute_spectral_centroid, find_peak};
use pattern_verification_utils::{detect_audio_events, is_clipping, is_silent};

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

fn get_onset_positions<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycle: usize,
) -> Vec<f64> {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    let mut positions: Vec<f64> = events
        .iter()
        .map(|hap| {
            let t = hap.part.begin.to_float();
            t - t.floor()
        })
        .collect();
    positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    positions
}

/// Calculate RMS of a time window within audio
#[allow(dead_code)]
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

// --- Amen Break Patterns ---

#[test]
fn breakbeat_l1_amen_basic_kick_has_syncopated_hits() {
    // Basic Amen kick: "bd ~ bd ~ ~ ~ bd ~"
    // Kicks at positions 0, 2, 6 out of 8 = 0.0, 0.25, 0.75
    let kick_pattern: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let events = query_single_cycle(&kick_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        3,
        "Amen kick should have 3 hits per cycle, got {}",
        non_rest.len()
    );
}

#[test]
fn breakbeat_l1_amen_basic_kick_positions() {
    // Verify the syncopated kick positions
    let kick_pattern: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let positions = get_onset_positions(&kick_pattern, 0);

    // Expected: 0/8=0.0, 2/8=0.25, 6/8=0.75
    assert!(
        (positions[0] - 0.0).abs() < 0.01,
        "First kick at 0.0, got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.25).abs() < 0.01,
        "Second kick at 0.25, got {}",
        positions[1]
    );
    assert!(
        (positions[2] - 0.75).abs() < 0.01,
        "Third kick at 0.75, got {}",
        positions[2]
    );
}

#[test]
fn breakbeat_l1_amen_snare_on_backbeat() {
    // Amen snare: "~ sn ~ sn ~ ~ ~ sn"
    // Snare at positions 1, 3, 7 out of 8
    let snare_pattern: Pattern<String> = parse_mini_notation("~ sn ~ sn ~ ~ ~ sn");
    let events = query_single_cycle(&snare_pattern);

    let non_rest: Vec<_> = events.iter().filter(|h| h.value != "~").collect();
    assert_eq!(
        non_rest.len(),
        3,
        "Amen snare should have 3 hits per cycle, got {}",
        non_rest.len()
    );

    // Verify backbeat positions
    let positions = get_onset_positions(&snare_pattern, 0);
    // 1/8 = 0.125 (beat 1 "and"), 3/8 = 0.375 (beat 2 "and"), 7/8 = 0.875
    assert!(
        (positions[0] - 0.125).abs() < 0.01,
        "First snare at 0.125, got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.375).abs() < 0.01,
        "Second snare at 0.375, got {}",
        positions[1]
    );
}

#[test]
fn breakbeat_l1_amen_hats_eighth_notes() {
    // Amen hats: "hh*8" = 8 evenly spaced hits
    let hh_pattern: Pattern<String> = parse_mini_notation("hh*8");
    let metrics = PatternMetrics::analyze(&hh_pattern, 4);

    assert_eq!(metrics.density, 8.0, "Amen hats should have 8 events/cycle");
    assert!(
        metrics.evenness > 0.9,
        "Amen hats should be evenly spaced, got {}",
        metrics.evenness
    );
}

#[test]
fn breakbeat_l1_amen_combined_density() {
    // Full Amen: kick(3) + snare(3) + hats(8) = 14 events per cycle
    let kick: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let snare: Pattern<String> = parse_mini_notation("~ sn ~ sn ~ ~ ~ sn");
    let hats: Pattern<String> = parse_mini_notation("hh*8");

    let kick_d = PatternMetrics::analyze(&kick, 1).density;
    let snare_d = PatternMetrics::analyze(&snare, 1).density;
    let hats_d = PatternMetrics::analyze(&hats, 1).density;

    let total = kick_d + snare_d + hats_d;
    assert!(
        total >= 13.0 && total <= 15.0,
        "Combined Amen density should be ~14, got {}",
        total
    );
}

#[test]
fn breakbeat_l1_amen_kick_is_syncopated() {
    // Amen kick should have significant syncopation (hits on weak beats)
    let kick: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let metrics = PatternMetrics::analyze(&kick, 1);

    assert!(
        metrics.syncopation > 0.1,
        "Amen kick should have syncopation > 0.1, got {}",
        metrics.syncopation
    );
}

// --- Jungle Patterns ---

#[test]
fn breakbeat_l1_jungle_basic_kick_density() {
    // Jungle basic kick: "bd ~ bd ~ ~ ~ bd ~" = 3 hits
    let pattern: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 3.0,
        "Jungle kick should have 3 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_ragga_jungle_sparser_than_rolling() {
    // Ragga jungle: more space (reggae influence)
    // Rolling jungle: 16th hats, driving energy
    let ragga_hats: Pattern<String> = parse_mini_notation("hh*4");
    let rolling_hats: Pattern<String> = parse_mini_notation("hh*16");

    let ragga_d = PatternMetrics::analyze(&ragga_hats, 1).density;
    let rolling_d = PatternMetrics::analyze(&rolling_hats, 1).density;

    assert!(
        rolling_d > ragga_d,
        "Rolling jungle hats ({}) should be denser than ragga ({})",
        rolling_d,
        ragga_d
    );
}

#[test]
fn breakbeat_l1_darkside_high_kick_density() {
    // Darkside: "[bd bd] ~ bd ~ ~ bd [bd ~] ~"
    // More kicks than basic jungle
    let pattern: Pattern<String> = parse_mini_notation("[bd bd] ~ bd ~ ~ bd [bd ~] ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert!(
        metrics.density >= 4.0,
        "Darkside kick should have >= 4 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_rolling_jungle_16th_hats() {
    // Rolling jungle has 16th note hi-hats
    let hh_pattern: Pattern<String> = parse_mini_notation("hh*16");
    let count = count_events_over_cycles(&hh_pattern, 4);
    assert_eq!(
        count, 64,
        "hh*16 over 4 cycles should produce 64 events, got {}",
        count
    );
}

// --- Breakcore Patterns ---

#[test]
fn breakbeat_l1_breakcore_blast_extreme_density() {
    // Breakcore blast: "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"
    // Very high density due to subdivisions
    let pattern: Pattern<String> =
        parse_mini_notation("[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert!(
        metrics.density >= 12.0,
        "Breakcore blast should have >= 12 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_glitch_break_has_subdivisions() {
    // Glitch break: "bd [sn sn sn] ~ [[bd bd] cp] ~ bd [bd sn] ~"
    // Contains nested subdivisions
    let pattern: Pattern<String> =
        parse_mini_notation("bd [sn sn sn] ~ [[bd bd] cp] ~ bd [bd sn] ~");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert!(
        metrics.density >= 8.0,
        "Glitch break should have >= 8 events/cycle (subdivisions), got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_drill_break_snare_rolls() {
    // Drill break: "bd ~ [sn sn sn sn] ~ bd ~ [sn*8] ~"
    // Contains snare rolls with high event count
    let pattern: Pattern<String> = parse_mini_notation("bd ~ [sn sn sn sn] ~ bd ~ [sn*8] ~");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert!(
        metrics.density >= 10.0,
        "Drill break should have >= 10 events/cycle (snare rolls), got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_breakcore_denser_than_jungle() {
    // Breakcore should have significantly more events than basic jungle
    let jungle: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8");
    let breakcore: Pattern<String> =
        parse_mini_notation("[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn");

    let jungle_d = PatternMetrics::analyze(&jungle, 1).density;
    let breakcore_d = PatternMetrics::analyze(&breakcore, 1).density;

    assert!(
        breakcore_d > jungle_d,
        "Breakcore ({}) should be denser than jungle ({})",
        breakcore_d,
        jungle_d
    );
}

// --- Chopped Break Patterns ---

#[test]
fn breakbeat_l1_amencutup_slice_pattern() {
    // Using pre-sliced amen samples
    let pattern: Pattern<String> = parse_mini_notation(
        "amencutup:0 amencutup:1 ~ amencutup:2 ~ ~ amencutup:3 ~ amencutup:4 ~ ~ ~ amencutup:5 ~ ~ ~",
    );
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 6.0,
        "Amencutup pattern should have 6 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_reversed_slice_order() {
    // Reversed slice pattern: all 8 slices
    let pattern: Pattern<String> = parse_mini_notation(
        "amencutup:7 amencutup:6 amencutup:5 amencutup:4 amencutup:3 amencutup:2 amencutup:1 amencutup:0",
    );
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 8.0,
        "Reversed slice should have 8 events/cycle, got {}",
        metrics.density
    );
    assert!(
        metrics.evenness > 0.9,
        "Evenly spaced slices should be even, got {}",
        metrics.evenness
    );
}

#[test]
fn breakbeat_l1_random_cutup_subdivisions() {
    // "amencutup:0 [amencutup:1 amencutup:5] amencutup:2*2 [amencutup:3 amencutup:4]"
    // Has bracketed groups = more events than 4
    let pattern: Pattern<String> = parse_mini_notation(
        "amencutup:0 [amencutup:1 amencutup:5] amencutup:2*2 [amencutup:3 amencutup:4]",
    );
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert!(
        metrics.density >= 6.0,
        "Random cutup should have >= 6 events/cycle (subdivisions), got {}",
        metrics.density
    );
}

// --- Euclidean Jungle ---

#[test]
fn breakbeat_l1_euclidean_jungle_kick() {
    // Euclidean kick: bd(5,16) - 5 hits in 16 slots
    let pattern: Pattern<String> = parse_mini_notation("bd(5,16)");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 5.0,
        "Euclidean bd(5,16) should have 5 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn breakbeat_l1_euclidean_snare_with_rotation() {
    // Euclidean snare: sn(3,8,4) - 3 hits in 8 slots, rotated by 4
    let pattern: Pattern<String> = parse_mini_notation("sn(3,8,4)");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 3.0,
        "Euclidean sn(3,8,4) should have 3 events/cycle, got {}",
        metrics.density
    );
}

// --- Polyrhythmic Break ---

#[test]
fn breakbeat_l1_polyrhythmic_3_against_4() {
    // "[bd bd bd, sn sn sn sn]" = 3 kicks + 4 snares in same cycle
    let pattern: Pattern<String> = parse_mini_notation("[bd bd bd, sn sn sn sn]");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 7.0,
        "Polyrhythmic 3+4 should have 7 events/cycle, got {}",
        metrics.density
    );
}

// --- Pattern Consistency ---

#[test]
fn breakbeat_l1_patterns_consistent_over_cycles() {
    let patterns = vec![
        ("amen kick", "bd ~ bd ~ ~ ~ bd ~"),
        ("amen snare", "~ sn ~ sn ~ ~ ~ sn"),
        ("jungle basic", "bd ~ bd ~ ~ ~ bd ~"),
        ("rolling hats", "hh*16"),
    ];

    for (name, notation) in patterns {
        let pattern: Pattern<String> = parse_mini_notation(notation);
        let metrics = PatternMetrics::analyze(&pattern, 8);

        assert!(
            metrics.density_variance < 0.001,
            "{}: pattern should be consistent across cycles (variance: {})",
            name,
            metrics.density_variance
        );
    }
}

// --- Genre Characteristic Summary ---

#[test]
fn breakbeat_l1_amen_genre_characteristics() {
    // Amen break characteristics:
    // - Kick: 3 per cycle, syncopated
    // - Snare: 3 per cycle, includes off-beat hits
    // - Hats: 8th notes (8 per cycle)
    let kick: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let snare: Pattern<String> = parse_mini_notation("~ sn ~ sn ~ ~ ~ sn");
    let hats: Pattern<String> = parse_mini_notation("hh*8");

    let kick_m = PatternMetrics::analyze(&kick, 4);
    let snare_m = PatternMetrics::analyze(&snare, 4);
    let hats_m = PatternMetrics::analyze(&hats, 4);

    // Kick: 3 per cycle, syncopated
    assert_eq!(kick_m.density, 3.0);
    assert!(kick_m.syncopation > 0.1);

    // Snare: 3 per cycle
    assert_eq!(snare_m.density, 3.0);

    // Hats: 8 per cycle, very even
    assert_eq!(hats_m.density, 8.0);
    assert!(hats_m.evenness > 0.9);
}

#[test]
fn breakbeat_l1_jungle_genre_characteristics() {
    // Jungle characteristics:
    // - Syncopated kick (not straight four-on-floor)
    // - Fast hat patterns (8 or 16 per cycle)
    // - Snare on backbeat positions
    let kick: Pattern<String> = parse_mini_notation("bd ~ bd ~ ~ ~ bd ~");
    let hats: Pattern<String> = parse_mini_notation("hh*16");
    let snare: Pattern<String> = parse_mini_notation("~ sn ~ sn");

    let kick_m = PatternMetrics::analyze(&kick, 4);
    let hats_m = PatternMetrics::analyze(&hats, 4);
    let snare_m = PatternMetrics::analyze(&snare, 4);

    // Kick should be syncopated (not four-on-floor)
    assert!(kick_m.syncopation > 0.0);
    // Hats: 16 per cycle (rolling jungle)
    assert_eq!(hats_m.density, 16.0);
    // Snare: 2 per cycle (backbeat)
    assert_eq!(snare_m.density, 2.0);
}

// ============================================================================
// LEVEL 2: ONSET DETECTION
// Tests that rendered audio has events at correct times
// ============================================================================

#[test]
fn breakbeat_l2_amen_basic_renders_audio() {
    // Basic Amen pattern should produce non-silent audio
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ sn ~ sn ~ ~ ~ sn"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Amen break pattern should produce audible output"
    );
}

#[test]
fn breakbeat_l2_amen_onset_count() {
    // Amen break at jungle tempo should have many onsets
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ sn ~ sn ~ ~ ~ sn"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.005);

    // At 2.8 CPS over 2 seconds = 5.6 cycles
    // Each cycle has kick+snare+hats = many onsets
    assert!(
        events.len() >= 10,
        "Amen break should have at least 10 onsets over 2 seconds, got {}",
        events.len()
    );
}

#[test]
fn breakbeat_l2_jungle_basic_renders_audio() {
    let code = r#"
tempo: 2.8
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"
out $ ~drums * 0.7
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Jungle basic pattern should produce audible output"
    );
}

#[test]
fn breakbeat_l2_ragga_jungle_renders_audio() {
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~snare $ s "~ ~ sn ~ ~ sn ~ ~"
~hats $ s "hh*4"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Ragga jungle should produce audible output"
    );
}

#[test]
fn breakbeat_l2_rolling_jungle_renders_audio() {
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Rolling jungle should produce audible output"
    );
}

#[test]
fn breakbeat_l2_darkside_renders_audio() {
    let code = r#"
tempo: 2.8
~kick $ s "[bd bd] ~ bd ~ ~ bd [bd ~] ~"
~snare $ s "~ sn ~ sn ~ ~ sn sn"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Darkside jungle should produce audible output"
    );
}

#[test]
fn breakbeat_l2_breakcore_blast_renders_audio() {
    // Breakcore at high tempo
    let code = r#"
tempo: 3.3
~drums $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"
out $ ~drums * 0.7
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Breakcore blast should produce audible output"
    );
}

#[test]
fn breakbeat_l2_breakcore_more_events_than_jungle() {
    // Breakcore at higher tempo should have more detected events
    let jungle_code = r#"
tempo: 2.8
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"
out $ ~drums * 0.7
"#;
    let breakcore_code = r#"
tempo: 3.3
~drums $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"
out $ ~drums * 0.7
"#;

    let jungle_audio = render_dsl(jungle_code, 2.0);
    let breakcore_audio = render_dsl(breakcore_code, 2.0);

    let jungle_events = detect_audio_events(&jungle_audio, 44100.0, 0.005);
    let breakcore_events = detect_audio_events(&breakcore_audio, 44100.0, 0.005);

    assert!(
        breakcore_events.len() > jungle_events.len(),
        "Breakcore ({} events) should have more onsets than jungle ({} events)",
        breakcore_events.len(),
        jungle_events.len()
    );
}

#[test]
fn breakbeat_l2_rolling_more_events_than_ragga() {
    // Rolling jungle (16th hats) should have more events than ragga (4th hats)
    let ragga_code = r#"
tempo: 2.8
~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~hats $ s "hh*4"
out $ ~kick * 0.8 + ~hats * 0.4
"#;
    let rolling_code = r#"
tempo: 2.8
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~hats * 0.5
"#;

    let ragga_audio = render_dsl(ragga_code, 2.0);
    let rolling_audio = render_dsl(rolling_code, 2.0);

    let ragga_events = detect_audio_events(&ragga_audio, 44100.0, 0.005);
    let rolling_events = detect_audio_events(&rolling_audio, 44100.0, 0.005);

    assert!(
        rolling_events.len() > ragga_events.len(),
        "Rolling jungle ({} events) should have more onsets than ragga ({} events)",
        rolling_events.len(),
        ragga_events.len()
    );
}

#[test]
fn breakbeat_l2_amencutup_slices_render() {
    // Pre-sliced amen samples should render
    let code = r#"
tempo: 2.8
~chop $ s "amencutup:0 amencutup:1 amencutup:2 amencutup:3 amencutup:4 amencutup:5 amencutup:6 amencutup:7"
out $ ~chop * 0.7
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Amencutup slices should produce audible output"
    );
}

#[test]
fn breakbeat_l2_jungle_sub_bass_renders() {
    // Jungle sub bass (sine) should produce audio
    let code = r#"
tempo: 2.8
~sub $ sine "55 ~ 55 ~ ~ ~ 55 ~" * 0.3
out $ ~sub
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Jungle sub bass should produce audible output"
    );
}

#[test]
fn breakbeat_l2_swing_produces_audio() {
    // Verify swing transform doesn't break audio rendering
    let code = r#"
tempo: 2.8
out $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8" $ swing 0.15
"#;
    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Swung breakbeat should produce audio"
    );

    let events = detect_audio_events(&audio, 44100.0, 0.005);
    assert!(
        events.len() >= 4,
        "Swung breakbeat should still produce events, got {}",
        events.len()
    );
}

// ============================================================================
// LEVEL 3: AUDIO CHARACTERISTICS
// Verifies signal properties match breakbeat/jungle reference characteristics
// ============================================================================

#[test]
fn breakbeat_l3_amen_not_silent() {
    let code = r#"
tempo: 2.8
out $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn ~ ~ ~ sn, hh*8"
"#;
    let audio = render_dsl(code, 2.0);
    assert!(!is_silent(&audio, 0.001), "Amen break should not be silent");
}

#[test]
fn breakbeat_l3_amen_not_clipping() {
    let code = r#"
tempo: 2.8
out $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn ~ ~ ~ sn, hh*8"
"#;
    let audio = render_dsl(code, 2.0);
    assert!(!is_clipping(&audio, 1.0), "Amen break should not clip");
}

#[test]
fn breakbeat_l3_jungle_reasonable_rms() {
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ sn ~ sn ~ ~ ~ sn"
~hats $ s "hh*8"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.001,
        "Jungle should have audible RMS (> 0.001), got {}",
        rms
    );
    assert!(
        rms < 0.8,
        "Jungle should not be overly loud (< 0.8), got {}",
        rms
    );
}

#[test]
fn breakbeat_l3_breakcore_reasonable_rms() {
    let code = r#"
tempo: 3.3
~drums $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"
out $ ~drums * 0.6
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.001,
        "Breakcore should have audible RMS (> 0.001), got {}",
        rms
    );
    assert!(
        rms < 0.8,
        "Breakcore should not be overly loud (< 0.8), got {}",
        rms
    );
}

#[test]
fn breakbeat_l3_faster_tempo_more_energy() {
    // Higher tempo = more events per second = more total energy
    let slow_code = r#"
tempo: 2.28
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"
out $ ~drums * 0.7
"#;
    let fast_code = r#"
tempo: 3.3
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"
out $ ~drums * 0.7
"#;

    let slow_audio = render_dsl(slow_code, 2.0);
    let fast_audio = render_dsl(fast_code, 2.0);

    let slow_rms = calculate_rms(&slow_audio);
    let fast_rms = calculate_rms(&fast_audio);

    assert!(
        fast_rms > slow_rms,
        "Faster tempo (RMS {:.4}) should have more energy than slower (RMS {:.4})",
        fast_rms,
        slow_rms
    );
}

#[test]
fn breakbeat_l3_hats_add_brightness() {
    // Adding hi-hats to kick-only pattern should increase spectral centroid
    // Use kick-only base to maximize the timbral contrast
    let no_hh_code = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
out $ ~kick * 0.8
"#;
    let with_hh_code = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~hats * 0.5
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
fn breakbeat_l3_sub_bass_low_centroid() {
    // Jungle sub bass at 55 Hz should have a low spectral centroid
    let bass_code = r#"
tempo: 2.8
~sub $ sine 55 * 0.5
out $ ~sub
"#;
    let bright_code = r#"
tempo: 2.8
~bright $ sine 880 * 0.5
out $ ~bright
"#;

    let bass_audio = render_dsl(bass_code, 2.0);
    let bright_audio = render_dsl(bright_code, 2.0);

    let bass_centroid = compute_spectral_centroid(&bass_audio, 44100.0);
    let bright_centroid = compute_spectral_centroid(&bright_audio, 44100.0);

    assert!(
        bass_centroid < bright_centroid,
        "Sub bass ({:.1}Hz) should be darker than bright synth ({:.1}Hz)",
        bass_centroid,
        bright_centroid
    );
}

#[test]
fn breakbeat_l3_lpf_darkens_sound() {
    // Low-pass filtering should reduce spectral centroid
    let unfiltered_code = r#"
tempo: 2.8
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*16"
out $ ~drums * 0.7
"#;
    let filtered_code = r#"
tempo: 2.8
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*16"
out $ ~drums * 0.7 # lpf 4000 0.7
"#;

    let unfiltered = render_dsl(unfiltered_code, 2.0);
    let filtered = render_dsl(filtered_code, 2.0);

    let unfilt_centroid = compute_spectral_centroid(&unfiltered, 44100.0);
    let filt_centroid = compute_spectral_centroid(&filtered, 44100.0);

    // Filtered should be darker (lower centroid)
    assert!(
        filt_centroid < unfilt_centroid,
        "LPF should darken sound: unfiltered={:.1}Hz, filtered={:.1}Hz",
        unfilt_centroid,
        filt_centroid
    );
}

// ============================================================================
// CROSS-PATTERN COMPARISON TESTS
// Verify that different breakbeat styles sound meaningfully different
// ============================================================================

#[test]
fn breakbeat_cross_ragga_vs_rolling_event_density() {
    // Ragga jungle (sparse hats) vs rolling jungle (16th hats)
    let ragga = render_dsl(
        r#"
tempo: 2.8
~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~hats $ s "hh*4"
out $ ~kick * 0.8 + ~hats * 0.4
"#,
        2.0,
    );
    let rolling = render_dsl(
        r#"
tempo: 2.8
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~hats * 0.5
"#,
        2.0,
    );

    let ragga_events = detect_audio_events(&ragga, 44100.0, 0.005);
    let rolling_events = detect_audio_events(&rolling, 44100.0, 0.005);

    assert!(
        rolling_events.len() > ragga_events.len(),
        "Rolling ({} events) should have higher density than ragga ({} events)",
        rolling_events.len(),
        ragga_events.len()
    );
}

#[test]
fn breakbeat_cross_jungle_vs_breakcore_energy() {
    // Breakcore should have more energy than basic jungle (more events)
    let jungle = render_dsl(
        r#"
tempo: 2.8
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"
out $ ~drums * 0.7
"#,
        2.0,
    );
    let breakcore = render_dsl(
        r#"
tempo: 3.3
~drums $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"
out $ ~drums * 0.7
"#,
        2.0,
    );

    let jungle_rms = calculate_rms(&jungle);
    let breakcore_rms = calculate_rms(&breakcore);

    assert!(
        breakcore_rms > jungle_rms,
        "Breakcore (RMS {:.4}) should have more energy than jungle (RMS {:.4})",
        breakcore_rms,
        jungle_rms
    );
}

#[test]
fn breakbeat_cross_three_styles_produce_different_audio() {
    // Ragga, rolling jungle, and breakcore should all produce distinct audio
    let ragga = render_dsl(
        r#"
tempo: 2.8
~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~snare $ s "~ ~ sn ~ ~ sn ~ ~"
~hats $ s "hh*4"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#,
        2.0,
    );
    let rolling = render_dsl(
        r#"
tempo: 2.8
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.5
"#,
        2.0,
    );
    let breakcore = render_dsl(
        r#"
tempo: 3.3
~drums $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"
out $ ~drums * 0.7
"#,
        2.0,
    );

    // All should produce audio
    assert!(!is_silent(&ragga, 0.001), "Ragga should not be silent");
    assert!(!is_silent(&rolling, 0.001), "Rolling should not be silent");
    assert!(
        !is_silent(&breakcore, 0.001),
        "Breakcore should not be silent"
    );

    // Breakcore should be the densest (highest RMS)
    let ragga_rms = calculate_rms(&ragga);
    let rolling_rms = calculate_rms(&rolling);
    let breakcore_rms = calculate_rms(&breakcore);

    assert!(
        breakcore_rms > ragga_rms,
        "Breakcore (RMS {:.4}) should have more energy than ragga (RMS {:.4})",
        breakcore_rms,
        ragga_rms
    );

    // Rolling jungle (16th hats, more kicks) should have more total energy than
    // ragga jungle (quarter note hats, sparse kicks)
    assert!(
        rolling_rms > ragga_rms,
        "Rolling (RMS {:.4}) should have more energy than ragga (RMS {:.4})",
        rolling_rms,
        ragga_rms
    );
}

// ============================================================================
// FULL MIX INTEGRATION TESTS
// Tests complete breakbeat patterns from the pattern library
// ============================================================================

#[test]
fn breakbeat_full_classic_jungle_track() {
    // Full jungle track with drums + sub bass
    let code = r#"
tempo: 2.8
~kicks $ s "bd ~ bd ~ ~ ~ bd ~"
~snares $ s "~ sn ~ sn ~ ~ ~ sn"
~hats $ s "hh*16" * 0.5
~sub $ sine "55 ~ 55 ~ ~ ~ 55 ~" * 0.3
out $ ~kicks * 0.8 + ~snares * 0.6 + ~hats + ~sub
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Classic jungle track should produce audio"
    );

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
fn breakbeat_full_ragga_jungle() {
    // Ragga jungle: more space, reggae influence
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ ~ ~ bd ~ ~ ~"
~snare $ s "~ ~ sn ~ ~ sn ~ ~"
~hats $ s "hh*4"
~sub $ sine "55 ~ 82.5 55" * 0.3
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~sub
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Ragga jungle should produce audio"
    );
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn breakbeat_full_darkside_jungle() {
    // Darkside: harder, more aggressive
    let code = r#"
tempo: 2.8
~kick $ s "[bd bd] ~ bd ~ ~ bd [bd ~] ~"
~snare $ s "~ sn ~ sn ~ ~ sn sn"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Darkside jungle should produce audio"
    );

    let events = detect_audio_events(&audio, 44100.0, 0.005);
    assert!(
        events.len() >= 10,
        "Darkside should have high onset density, got {}",
        events.len()
    );
}

#[test]
fn breakbeat_full_breakcore_chaos() {
    // Breakcore at extreme tempo
    let code = r#"
tempo: 3.3
~drums $ s "[bd bd] sn [bd bd] sn [bd bd bd] sn bd sn"
out $ ~drums * 0.6
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Breakcore chaos should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.005, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn breakbeat_full_filtered_jungle() {
    // Filtered jungle with lpf
    let code = r#"
tempo: 2.8
~drums $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*16"
out $ ~drums * 0.7 # lpf 4000 0.7
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Filtered jungle should produce audio"
    );
    assert!(!is_clipping(&audio, 1.0), "Filtered jungle should not clip");
}

#[test]
fn breakbeat_full_amencutup_arrangement() {
    // Chopped amen using pre-sliced samples
    let code = r#"
tempo: 2.8
~chop $ s "amencutup:0 amencutup:1 ~ amencutup:2 ~ ~ amencutup:3 ~ amencutup:4 ~ ~ ~ amencutup:5 ~ ~ ~"
out $ ~chop * 0.7
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Amencutup arrangement should produce audio"
    );
}

#[test]
fn breakbeat_full_euclidean_jungle() {
    // Euclidean rhythm jungle
    let code = r#"
tempo: 2.8
~kick $ s "bd(5,16)"
~snare $ s "sn(3,8,4)"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Euclidean jungle should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.005, "Should have meaningful RMS: {:.4}", rms);
}

#[test]
fn breakbeat_full_polyrhythmic_break() {
    // 3-against-4 polyrhythm with 12th-note hats
    let code = r#"
tempo: 2.8
~poly $ s "[bd bd bd, sn sn sn sn]"
~hats $ s "hh*12"
out $ ~poly * 0.7 + ~hats * 0.4
"#;
    let audio = render_dsl(code, 3.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Polyrhythmic break should produce audio"
    );

    let events = detect_audio_events(&audio, 44100.0, 0.005);
    assert!(
        events.len() >= 10,
        "Polyrhythmic break should have many onsets, got {}",
        events.len()
    );
}

#[test]
fn breakbeat_full_jungle_with_bass_and_hats() {
    // Full production: breaks + sub bass + filtered pads
    let code = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ sn ~ sn ~ ~ ~ sn"
~hats $ s "hh*16" * 0.5
~sub $ sine "55 ~ 55 ~ ~ ~ 55 ~" * 0.3
~pad $ sine "110 165 220" * 0.08
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats + ~sub + ~pad
"#;
    let audio = render_dsl(code, 4.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Full production should produce audio"
    );

    let rms = calculate_rms(&audio);
    assert!(rms > 0.005, "Production should be audible: RMS {:.4}", rms);

    let peak = find_peak(&audio);
    assert!(
        peak < 3.0,
        "Production peak should be reasonable: {:.3}",
        peak
    );
}

#[test]
fn breakbeat_full_ghost_notes_add_detail() {
    // Ghost notes add subtle extra energy to the mix
    let without_ghosts = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ sn ~ sn"
out $ ~kick * 0.8 + ~snare * 0.6
"#;
    let with_ghosts = r#"
tempo: 2.8
~kick $ s "bd ~ bd ~ ~ ~ bd ~"
~snare $ s "~ sn ~ sn"
~ghost $ s "~ ~ ~ bd ~ ~ ~ ~, sn ~ ~ ~ ~ sn ~ ~" * 0.3
out $ ~kick * 0.8 + ~snare * 0.6 + ~ghost
"#;

    let audio_no_ghost = render_dsl(without_ghosts, 2.0);
    let audio_with_ghost = render_dsl(with_ghosts, 2.0);

    // Ghost notes add energy - RMS should increase
    let rms_no_ghost = calculate_rms(&audio_no_ghost);
    let rms_with_ghost = calculate_rms(&audio_with_ghost);

    assert!(
        rms_with_ghost >= rms_no_ghost,
        "Adding ghost notes should increase or maintain energy: without={:.4}, with={:.4}",
        rms_no_ghost,
        rms_with_ghost
    );

    // Both should produce audio
    assert!(
        !is_silent(&audio_no_ghost, 0.001),
        "Without ghosts should produce audio"
    );
    assert!(
        !is_silent(&audio_with_ghost, 0.001),
        "With ghosts should produce audio"
    );
}

#[test]
fn breakbeat_full_no_clipping_across_styles() {
    // None of the breakbeat styles should clip
    let patterns = vec![
        (
            "Amen",
            r#"
tempo: 2.8
out $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn ~ ~ ~ sn, hh*8"
"#,
        ),
        (
            "Rolling Jungle",
            r#"
tempo: 2.8
~kick $ s "bd ~ ~ bd ~ bd ~ ~"
~snare $ s "~ ~ sn ~ ~ ~ sn ~"
~hats $ s "hh*16"
out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4
"#,
        ),
        (
            "Breakcore",
            r#"
tempo: 3.3
out $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn" * 0.6
"#,
        ),
    ];

    for (name, code) in patterns {
        let audio = render_dsl(code, 2.0);
        assert!(!is_clipping(&audio, 1.0), "{} should not clip", name);
    }
}
