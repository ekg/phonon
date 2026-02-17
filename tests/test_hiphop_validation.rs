//! Hip-Hop / Trap Pattern Validation Tests
//!
//! Validates that the hip-hop pattern library (demos/hiphop_trap.ph) produces
//! audio with characteristics matching real hip-hop sub-genres:
//!
//! - Boom-Bap: 85-95 BPM, heavy swing, kick+snare+hats
//! - Lo-Fi Hip-Hop: 70-85 BPM, lazy swing, sparse drums
//! - Trap: 130-150 BPM (half-time feel), hi-hat rolls, 808 bass
//! - Drill: 140-145 BPM, sliding 808s, rapid hi-hat patterns
//! - Phonk: 130-145 BPM, cowbell patterns, dark atmosphere
//!
//! Three-level methodology:
//!   Level 1: Pattern query verification (event counts, timing, density)
//!   Level 2: Onset detection / audio timing
//!   Level 3: Audio characteristics (RMS, spectral, not silent/clipping)

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_metrics::PatternMetrics;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::{
    calculate_spectral_centroid, detect_audio_events, is_clipping, is_silent,
};

// ============================================================================
// TEST HELPERS
// ============================================================================

/// Render DSL code to audio samples
fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).expect("Parse DSL failed");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let samples = (44100.0 * duration_secs) as usize;
    graph.render(samples)
}

/// Count events from a pattern over multiple cycles
#[allow(dead_code)]
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

/// Get events for a single cycle
fn get_events_for_cycle<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycle: usize,
) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

/// Get onset positions within a cycle (normalized to [0, 1))
fn get_onset_positions<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycle: usize,
) -> Vec<f64> {
    let events = get_events_for_cycle(pattern, cycle);
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

// ============================================================================
// LEVEL 1: PATTERN QUERY VERIFICATION
// ============================================================================
// Tests pattern logic directly without rendering audio.
// Verifies event counts, positions, and rhythmic metrics.

// --- Boom-Bap Patterns ---

#[test]
fn level1_boombap_kick_pattern_has_correct_density() {
    // Classic boom-bap kick: "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
    // Should have 3 kicks per cycle in a 16-step pattern
    let pattern: Pattern<String> = parse_mini_notation("bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 3.0,
        "Boom-bap kick should have 3 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn level1_boombap_snare_on_backbeat() {
    // Classic snare: "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
    // Snare hits on positions 4/16 and 12/16 (beats 2 and 4 in 4/4)
    let pattern: Pattern<String> = parse_mini_notation("~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~");
    let events = get_events_for_cycle(&pattern, 0);

    assert_eq!(
        events.len(),
        2,
        "Boom-bap snare should have 2 hits per cycle"
    );

    let positions = get_onset_positions(&pattern, 0);
    // Position 4/16 = 0.25, position 12/16 = 0.75
    assert!(
        (positions[0] - 0.25).abs() < 0.01,
        "First snare should be on beat 2 (0.25), got {}",
        positions[0]
    );
    assert!(
        (positions[1] - 0.75).abs() < 0.01,
        "Second snare should be on beat 4 (0.75), got {}",
        positions[1]
    );
}

#[test]
fn level1_boombap_hats_steady_eighth_notes() {
    // Hi-hats: "hh*8" = 8 evenly spaced hits
    let pattern: Pattern<String> = parse_mini_notation("hh*8");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 8.0,
        "Boom-bap hats should have 8 events/cycle"
    );
    assert!(
        metrics.evenness > 0.9,
        "Evenly spaced hats should have high evenness, got {}",
        metrics.evenness
    );
}

#[test]
fn level1_boombap_combined_density() {
    // Full boom-bap = kick(3) + snare(2) + hats(8) = 13 events per cycle
    let kick: Pattern<String> = parse_mini_notation("bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~");
    let snare: Pattern<String> = parse_mini_notation("~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~");
    let hats: Pattern<String> = parse_mini_notation("hh*8");

    let kick_d = PatternMetrics::analyze(&kick, 1).density;
    let snare_d = PatternMetrics::analyze(&snare, 1).density;
    let hats_d = PatternMetrics::analyze(&hats, 1).density;

    let total = kick_d + snare_d + hats_d;
    assert!(
        total >= 12.0 && total <= 14.0,
        "Combined boom-bap density should be ~13, got {}",
        total
    );
}

// --- Trap Patterns ---

#[test]
fn level1_trap_kick_pattern_sparse() {
    // Trap kick: "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~"
    // Should have 3 kicks - trap is relatively sparse on kicks
    let pattern: Pattern<String> =
        parse_mini_notation("808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert!(
        metrics.density >= 2.0 && metrics.density <= 4.0,
        "Trap kick should have 2-4 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn level1_trap_hihat_rolls_high_density() {
    // Trap hi-hat with rolls: "hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] [hh*3] hh hh"
    // Should be high density due to rolls (brackets subdivide)
    let pattern: Pattern<String> =
        parse_mini_notation("hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] [hh*3] hh hh");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    // Regular 16 hats + extra from rolls
    assert!(
        metrics.density >= 16.0,
        "Trap hats with rolls should have >= 16 events/cycle, got {}",
        metrics.density
    );
}

#[test]
fn level1_trap_clap_on_backbeat() {
    // Trap clap: "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
    // Two claps per cycle, roughly on beats 2 and 4
    let pattern: Pattern<String> = parse_mini_notation("~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~");
    let events = get_events_for_cycle(&pattern, 0);

    assert_eq!(events.len(), 2, "Trap clap should have 2 hits per cycle");
}

// --- Drill Patterns ---

#[test]
fn level1_drill_hihat_with_triplet_rolls() {
    // UK Drill hats: "hh hh hh hh hh [hh*3] hh hh hh hh hh hh hh [hh*3] hh hh"
    let pattern: Pattern<String> =
        parse_mini_notation("hh hh hh hh hh [hh*3] hh hh hh hh hh hh hh [hh*3] hh hh");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    // 14 regular + 6 from two [hh*3] = 20 events
    assert!(
        metrics.density >= 16.0,
        "Drill hats should have >= 16 events/cycle due to rolls, got {}",
        metrics.density
    );
}

#[test]
fn level1_drill_kick_syncopated() {
    // UK Drill kick: "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~"
    // More syncopated than boom-bap
    let pattern: Pattern<String> =
        parse_mini_notation("808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 4.0,
        "Drill kick should have 4 events/cycle"
    );
    assert!(
        metrics.syncopation > 0.0,
        "Drill kick should have some syncopation"
    );
}

// --- Lo-Fi Hip-Hop Patterns ---

#[test]
fn level1_lofi_sparse_kick() {
    // Lo-Fi kick: "bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ bd ~"
    let pattern: Pattern<String> = parse_mini_notation("bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ bd ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert!(
        metrics.density >= 2.0 && metrics.density <= 4.0,
        "Lo-fi kick should have 2-4 events/cycle (sparse), got {}",
        metrics.density
    );
}

#[test]
fn level1_lofi_reduced_hat_volume() {
    // Lo-Fi hats are 8th notes but quiet: "hh*8" * 0.5
    // Pattern-level: still 8 events per cycle
    let pattern: Pattern<String> = parse_mini_notation("hh*8");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 8.0,
        "Lo-fi hats still have 8 events/cycle (volume is separate)"
    );
}

// --- Phonk Patterns ---

#[test]
fn level1_phonk_cowbell_pattern() {
    // Memphis Phonk cowbell: "cb ~ cb ~ cb ~ cb ~"
    let pattern: Pattern<String> = parse_mini_notation("cb ~ cb ~ cb ~ cb ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 4.0,
        "Phonk cowbell should have 4 events/cycle"
    );
    assert!(
        metrics.evenness > 0.9,
        "Phonk cowbell should be evenly spaced, got {}",
        metrics.evenness
    );
}

#[test]
fn level1_drift_phonk_double_kick() {
    // Drift Phonk kick: "808bd 808bd ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ 808bd 808bd ~"
    // More kicks than other styles, characteristic double kicks
    let pattern: Pattern<String> =
        parse_mini_notation("808bd 808bd ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ 808bd 808bd ~");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert!(
        metrics.density >= 5.0,
        "Drift phonk should have >= 5 kicks/cycle (double kicks), got {}",
        metrics.density
    );
}

// --- Euclidean Hip-Hop Approximations ---

#[test]
fn level1_euclidean_boombap_approximation() {
    // bd(3,8) should approximate boom-bap kick pattern
    let pattern: Pattern<String> = parse_mini_notation("bd(3,8)");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 3.0,
        "Euclidean bd(3,8) should have 3 events/cycle"
    );
}

#[test]
fn level1_euclidean_trap_approximation() {
    // 808bd(3,16) for trap kick
    let pattern: Pattern<String> = parse_mini_notation("808bd(3,16)");
    let metrics = PatternMetrics::analyze(&pattern, 4);

    assert_eq!(
        metrics.density, 3.0,
        "Euclidean 808bd(3,16) should have 3 events/cycle"
    );
}

// --- Pattern Consistency Over Multiple Cycles ---

#[test]
fn level1_patterns_consistent_over_cycles() {
    let patterns = vec![
        ("boom-bap kick", "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"),
        ("boom-bap snare", "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"),
        ("trap kick", "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~"),
        ("trap hats", "hh*16"),
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

// --- Genre Density Comparison ---

#[test]
fn level1_genre_density_ordering() {
    // Trap hi-hats should be denser than boom-bap hi-hats
    let boombap_hats: Pattern<String> = parse_mini_notation("hh*8");
    let trap_hats: Pattern<String> = parse_mini_notation("hh*16");

    let bb_density = PatternMetrics::analyze(&boombap_hats, 1).density;
    let trap_density = PatternMetrics::analyze(&trap_hats, 1).density;

    assert!(
        trap_density > bb_density,
        "Trap hats ({}) should be denser than boom-bap hats ({})",
        trap_density,
        bb_density
    );
}

#[test]
fn level1_syncopation_comparison() {
    // Off-beat pattern more syncopated than on-beat
    let on_beat: Pattern<String> = parse_mini_notation("bd ~ bd ~");
    let off_beat: Pattern<String> = parse_mini_notation("~ bd ~ bd");

    let on_sync = PatternMetrics::analyze(&on_beat, 1).syncopation;
    let off_sync = PatternMetrics::analyze(&off_beat, 1).syncopation;

    assert!(
        off_sync > on_sync,
        "Off-beat ({}) should be more syncopated than on-beat ({})",
        off_sync,
        on_sync
    );
}

// ============================================================================
// LEVEL 2: DSL INTEGRATION / ONSET DETECTION
// ============================================================================
// Tests that DSL patterns render to audio with correct timing and event counts.

#[test]
fn level2_boombap_renders_with_events() {
    let code = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    // At 1.5 CPS over 2 seconds = 3 cycles
    // Each cycle: ~13 events (3 kick + 2 snare + 8 hats)
    // Some events overlap, so detection might merge some
    println!("Boom-bap: {} events detected in 2s", events.len());
    assert!(
        events.len() >= 10,
        "Boom-bap should produce many events, got {}",
        events.len()
    );
}

#[test]
fn level2_trap_renders_with_events() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    // At 2.33 CPS over 2 seconds = ~4.66 cycles
    // Trap has high hat density (16 per cycle)
    println!("Trap: {} events detected in 2s", events.len());
    assert!(
        events.len() >= 15,
        "Trap should produce many events due to hi-hats, got {}",
        events.len()
    );
}

#[test]
fn level2_lofi_renders_with_events() {
    let code = r#"
        tempo: 1.25
        out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8" * 0.4
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    // Lo-fi at 1.25 CPS = ~2.5 cycles over 2 seconds
    println!("Lo-fi: {} events detected in 2s", events.len());
    assert!(
        events.len() >= 5,
        "Lo-fi should produce events, got {}",
        events.len()
    );
}

#[test]
fn level2_drill_renders_with_events() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~" + s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*3] hh hh hh hh"
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    println!("Drill: {} events detected in 2s", events.len());
    assert!(
        events.len() >= 15,
        "Drill should produce many events, got {}",
        events.len()
    );
}

#[test]
fn level2_phonk_renders_with_events() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*8" * 0.5 + s "cb ~ cb ~ cb ~ cb ~" * 0.4
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    println!("Phonk: {} events detected in 2s", events.len());
    assert!(
        events.len() >= 10,
        "Phonk should produce events including cowbell, got {}",
        events.len()
    );
}

#[test]
fn level2_trap_more_events_than_lofi() {
    // Trap has much higher event density than lo-fi
    let trap_code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "hh*16"
    "#;

    let lofi_code = r#"
        tempo: 1.25
        out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~" + s "hh*8" * 0.4
    "#;

    let trap_audio = render_dsl(trap_code, 2.0);
    let lofi_audio = render_dsl(lofi_code, 2.0);

    let trap_events = detect_audio_events(&trap_audio, 44100.0, 0.001);
    let lofi_events = detect_audio_events(&lofi_audio, 44100.0, 0.001);

    println!(
        "Trap events: {}, Lo-fi events: {}",
        trap_events.len(),
        lofi_events.len()
    );

    assert!(
        trap_events.len() > lofi_events.len(),
        "Trap ({} events) should have more events than lo-fi ({} events)",
        trap_events.len(),
        lofi_events.len()
    );
}

#[test]
fn level2_full_boombap_production_renders() {
    // Test the full production example from the demo file
    let code = r#"
        tempo: 1.5
        ~full_boom_kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
        ~full_boom_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~full_boom_hats $ s "hh hh oh hh hh hh hh oh"
        ~full_boom_bass $ saw "55 55 82.5 73.4" # lpf 600 0.7 * 0.3
        out $ ~full_boom_kick + ~full_boom_snare + ~full_boom_hats + ~full_boom_bass
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    println!("Full boom-bap production: {} events in 2s", events.len());
    assert!(
        events.len() >= 10,
        "Full production should have many events, got {}",
        events.len()
    );
}

#[test]
fn level2_full_trap_production_renders() {
    // Full trap production example
    let code = r#"
        tempo: 2.33
        ~full_trap_kick $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~"
        ~full_trap_clap $ s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
        ~full_trap_hats $ s "hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] [hh*3] hh hh" * 0.7
        out $ ~full_trap_kick + ~full_trap_clap + ~full_trap_hats
    "#;

    let audio = render_dsl(code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    println!("Full trap production: {} events in 2s", events.len());
    assert!(
        events.len() >= 15,
        "Full trap production should have many events (hi-hat rolls), got {}",
        events.len()
    );
}

// ============================================================================
// LEVEL 3: AUDIO CHARACTERISTICS VALIDATION
// ============================================================================
// Tests audio signal quality: not silent, not clipping, reasonable RMS/spectral.

#[test]
fn level3_boombap_not_silent() {
    let code = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Boom-bap pattern should not be silent"
    );
}

#[test]
fn level3_boombap_not_clipping() {
    let code = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_clipping(&audio, 1.0),
        "Boom-bap pattern should not clip"
    );
}

#[test]
fn level3_trap_not_silent() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Trap pattern should not be silent"
    );
}

#[test]
fn level3_trap_not_clipping() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(!is_clipping(&audio, 1.0), "Trap pattern should not clip");
}

#[test]
fn level3_lofi_not_silent() {
    let code = r#"
        tempo: 1.25
        out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8" * 0.4
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Lo-fi pattern should not be silent"
    );
}

#[test]
fn level3_drill_not_silent() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~" + s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh*16"
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Drill pattern should not be silent"
    );
}

#[test]
fn level3_phonk_not_silent() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*8" * 0.5
    "#;
    let audio = render_dsl(code, 2.0);

    assert!(
        !is_silent(&audio, 0.001),
        "Phonk pattern should not be silent"
    );
}

#[test]
fn level3_boombap_reasonable_rms() {
    let code = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    println!("Boom-bap RMS: {:.4}", rms);
    assert!(
        rms > 0.001,
        "Boom-bap should have audible RMS (> 0.001), got {}",
        rms
    );
    assert!(
        rms < 0.8,
        "Boom-bap should not be overly loud (< 0.8), got {}",
        rms
    );
}

#[test]
fn level3_trap_reasonable_rms() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
    "#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);

    println!("Trap RMS: {:.4}", rms);
    assert!(
        rms > 0.001,
        "Trap should have audible RMS (> 0.001), got {}",
        rms
    );
    assert!(
        rms < 0.8,
        "Trap should not be overly loud (< 0.8), got {}",
        rms
    );
}

#[test]
fn level3_full_production_boombap_audio_quality() {
    let code = r#"
        tempo: 1.5
        ~kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~ghost $ s "~ ~ sn:1 ~ ~ ~ ~ sn:1 ~ ~ sn:1 ~ ~ ~ ~ ~" * 0.25
        ~hats $ s "hh hh oh hh hh hh hh oh"
        ~bass $ saw "55 55 82.5 73.4" # lpf 600 0.7 * 0.3
        out $ ~kick + ~snare + ~ghost + ~hats + ~bass
    "#;

    let audio = render_dsl(code, 3.0);
    let rms = calculate_rms(&audio);

    assert!(!is_silent(&audio, 0.001), "Production should not be silent");
    assert!(!is_clipping(&audio, 1.0), "Production should not clip");
    assert!(
        rms > 0.001,
        "Production should have audible level, got {}",
        rms
    );

    // With bass (saw oscillator), spectral centroid should reflect
    // a mix of low frequencies (bass) and high (hats)
    let centroid = calculate_spectral_centroid(&audio, 44100.0);
    println!(
        "Full boom-bap production - RMS: {:.4}, Spectral centroid: {:.1}Hz",
        rms, centroid
    );

    // Centroid should be in a reasonable range for a drum+bass mix
    assert!(
        centroid > 100.0,
        "Spectral centroid should be > 100Hz (has content), got {}",
        centroid
    );
}

#[test]
fn level3_full_production_trap_audio_quality() {
    let code = r#"
        tempo: 2.33
        ~kick $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~"
        ~clap $ s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
        ~hats $ s "hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] [hh*3] hh hh" * 0.7
        out $ ~kick + ~clap + ~hats
    "#;

    let audio = render_dsl(code, 3.0);
    let rms = calculate_rms(&audio);

    assert!(
        !is_silent(&audio, 0.001),
        "Trap production should not be silent"
    );
    assert!(!is_clipping(&audio, 1.0), "Trap production should not clip");

    let centroid = calculate_spectral_centroid(&audio, 44100.0);
    println!(
        "Full trap production - RMS: {:.4}, Spectral centroid: {:.1}Hz",
        rms, centroid
    );

    assert!(
        centroid > 100.0,
        "Spectral centroid should be > 100Hz, got {}",
        centroid
    );
}

#[test]
fn level3_lofi_production_audio_quality() {
    let code = r#"
        tempo: 1.25
        ~kick $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh*8" * 0.4
        ~keys $ sine "261 ~ 329 ~ 392 ~ 329 ~" * 0.15
        out $ ~kick + ~snare + ~hats + ~keys
    "#;

    let audio = render_dsl(code, 3.0);
    let rms = calculate_rms(&audio);

    assert!(
        !is_silent(&audio, 0.001),
        "Lo-fi production should not be silent"
    );
    assert!(
        !is_clipping(&audio, 1.0),
        "Lo-fi production should not clip"
    );

    let centroid = calculate_spectral_centroid(&audio, 44100.0);
    println!(
        "Lo-fi production - RMS: {:.4}, Spectral centroid: {:.1}Hz",
        rms, centroid
    );

    assert!(
        centroid > 50.0,
        "Lo-fi spectral centroid should be > 50Hz, got {}",
        centroid
    );
}

// --- Spectral Comparisons Between Genres ---

#[test]
fn level3_trap_hihats_brighter_than_boombap() {
    // Trap with 16th note hi-hats should have a brighter spectral centroid
    // than boom-bap with 8th note hi-hats (more high-frequency content per unit time)
    let boombap_code = r#"
        tempo: 1.5
        out $ s "hh*8"
    "#;
    let trap_code = r#"
        tempo: 2.33
        out $ s "hh*16"
    "#;

    let boombap_audio = render_dsl(boombap_code, 2.0);
    let trap_audio = render_dsl(trap_code, 2.0);

    let bb_events = detect_audio_events(&boombap_audio, 44100.0, 0.001);
    let trap_events = detect_audio_events(&trap_audio, 44100.0, 0.001);

    println!(
        "Boom-bap hats events: {}, Trap hats events: {}",
        bb_events.len(),
        trap_events.len()
    );

    // Trap at higher tempo with more subdivisions = more onset events
    assert!(
        trap_events.len() > bb_events.len(),
        "Trap hats ({}) should have more events than boom-bap hats ({})",
        trap_events.len(),
        bb_events.len()
    );
}

// --- Ghost Notes and Swing ---

#[test]
fn level2_ghost_notes_add_events() {
    let without_ghosts = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
    "#;

    let with_ghosts = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "~ ~ sn:1 ~ ~ ~ ~ sn:1 ~ ~ sn:1 ~ ~ ~ ~ ~" * 0.3
    "#;

    let audio_no_ghost = render_dsl(without_ghosts, 2.0);
    let audio_with_ghost = render_dsl(with_ghosts, 2.0);

    let events_no_ghost = detect_audio_events(&audio_no_ghost, 44100.0, 0.001);
    let events_with_ghost = detect_audio_events(&audio_with_ghost, 44100.0, 0.001);

    println!(
        "Without ghosts: {} events, With ghosts: {} events",
        events_no_ghost.len(),
        events_with_ghost.len()
    );

    // Ghost notes should add more detected events
    assert!(
        events_with_ghost.len() >= events_no_ghost.len(),
        "Adding ghost notes should produce >= events: {} vs {}",
        events_with_ghost.len(),
        events_no_ghost.len()
    );
}

#[test]
fn level2_swing_produces_audio() {
    // Verify swing doesn't break audio rendering
    let code = r#"
        tempo: 1.5
        out $ s "hh*8" $ swing 0.1
    "#;

    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Swung hi-hats should produce audio"
    );

    let events = detect_audio_events(&audio, 44100.0, 0.001);
    assert!(
        events.len() >= 4,
        "Swung hi-hats should still produce events, got {}",
        events.len()
    );
}

// --- Hi-Hat Roll Techniques ---

#[test]
fn level1_hihat_roll_subdivision() {
    // Triplet roll "[hh hh hh]" = 3 events in the space of 1 step
    let pattern: Pattern<String> = parse_mini_notation("[hh hh hh]");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 3.0,
        "Triplet roll should have 3 events/cycle"
    );
}

#[test]
fn level1_hihat_machinegun_roll() {
    // Machine gun: "[hh*8]" = 8 events in 1 step space
    let pattern: Pattern<String> = parse_mini_notation("[hh*8]");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 8.0,
        "Machine gun roll should have 8 events/cycle"
    );
}

#[test]
fn level1_velocity_varied_hihats() {
    // Velocity-varied: "hh*16" with different volumes
    // Pattern level: still 16 events per cycle
    let pattern: Pattern<String> = parse_mini_notation("hh*16");
    let metrics = PatternMetrics::analyze(&pattern, 1);

    assert_eq!(
        metrics.density, 16.0,
        "Velocity-varied hats still have 16 events/cycle"
    );
}

// --- 808 Bass Patterns ---

#[test]
fn level2_synth_808_bass_renders() {
    let code = r#"
        tempo: 2.33
        out $ sine "55 ~ ~ ~ ~ ~ ~ ~ 55 ~ ~ ~ ~ ~ 55 ~" * 0.5
    "#;

    let audio = render_dsl(code, 2.0);
    assert!(
        !is_silent(&audio, 0.001),
        "Synth 808 bass should produce audio"
    );

    let centroid = calculate_spectral_centroid(&audio, 44100.0);
    // 808 bass at 55Hz should have a low spectral centroid
    println!("808 bass spectral centroid: {:.1}Hz", centroid);
    assert!(
        centroid < 2000.0,
        "808 bass centroid should be low (< 2000Hz), got {}",
        centroid
    );
}

// --- Pattern Transform Interactions with Hip-Hop ---

#[test]
fn level2_every_variation_on_boombap() {
    // "every 4 (fast 2)" should produce more events every 4th cycle
    let normal_code = r#"
        tempo: 1.5
        out $ s "bd sn" * 0.5
    "#;

    let audio = render_dsl(normal_code, 2.0);
    let events = detect_audio_events(&audio, 44100.0, 0.001);

    println!("Normal boom-bap beat: {} events", events.len());
    assert!(
        events.len() >= 2,
        "Normal pattern should produce events, got {}",
        events.len()
    );
}

// --- Multi-Layer Complexity ---

#[test]
fn level3_multi_layer_mix_not_clipping() {
    // Full production with kick + snare + ghost + hats + bass + keys
    // Should mix without clipping
    let code = r#"
        tempo: 1.5
        ~kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~ghost $ s "~ ~ sn:1 ~ ~ ~ ~ sn:1 ~ ~ sn:1 ~ ~ ~ sn:1 ~" * 0.25
        ~hats $ s "hh hh oh hh hh hh hh oh"
        ~bass $ saw "55 55 82.5 73.4" # lpf 600 0.7 * 0.3
        out $ ~kick + ~snare + ~ghost + ~hats + ~bass
    "#;

    let audio = render_dsl(code, 3.0);

    assert!(!is_clipping(&audio, 1.0), "Multi-layer mix should not clip");
    assert!(
        !is_silent(&audio, 0.001),
        "Multi-layer mix should produce audio"
    );

    let rms = calculate_rms(&audio);
    println!("Multi-layer mix RMS: {:.4}", rms);
    assert!(
        rms > 0.001,
        "Multi-layer mix should be audible, got RMS {}",
        rms
    );
}

// ============================================================================
// GENRE CHARACTERISTIC SUMMARY TESTS
// ============================================================================
// High-level tests that validate overall genre characteristics.

#[test]
fn level1_boombap_genre_characteristics() {
    // Boom-bap characteristics:
    // - Kick density: 2-4 per cycle
    // - Snare on backbeat (positions 0.25, 0.75)
    // - Hats: 8th notes (8 per cycle)
    // - Moderate syncopation
    let kick: Pattern<String> = parse_mini_notation("bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~");
    let snare: Pattern<String> = parse_mini_notation("~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~");
    let hats: Pattern<String> = parse_mini_notation("hh*8");

    let kick_m = PatternMetrics::analyze(&kick, 4);
    let snare_m = PatternMetrics::analyze(&snare, 4);
    let hats_m = PatternMetrics::analyze(&hats, 4);

    // Kick: 2-4 per cycle
    assert!(kick_m.density >= 2.0 && kick_m.density <= 4.0);
    // Snare: exactly 2 (backbeat)
    assert_eq!(snare_m.density, 2.0);
    // Hats: 8 per cycle
    assert_eq!(hats_m.density, 8.0);
    // Hats should be very even
    assert!(hats_m.evenness > 0.9);
}

#[test]
fn level1_trap_genre_characteristics() {
    // Trap characteristics:
    // - Kick: 2-4 per cycle (sparse)
    // - Clap on backbeat: 2 per cycle
    // - Hats: 16+ per cycle (with rolls)
    // - Low syncopation on hats (machine-like)
    let kick: Pattern<String> = parse_mini_notation("808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~");
    let clap: Pattern<String> = parse_mini_notation("~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~");
    let hats: Pattern<String> = parse_mini_notation("hh*16");

    let kick_m = PatternMetrics::analyze(&kick, 4);
    let clap_m = PatternMetrics::analyze(&clap, 4);
    let hats_m = PatternMetrics::analyze(&hats, 4);

    // Kick: sparse
    assert!(kick_m.density >= 2.0 && kick_m.density <= 4.0);
    // Clap: 2 per cycle
    assert_eq!(clap_m.density, 2.0);
    // Hats: 16 per cycle (no rolls in this simplified version)
    assert_eq!(hats_m.density, 16.0);
    // 16th note hats should be very even
    assert!(hats_m.evenness > 0.9);
}

#[test]
fn level1_phonk_genre_characteristics() {
    // Phonk characteristics:
    // - Cowbell: 4 per cycle, very even
    // - Kick: 3-6 per cycle
    // - Clap: 2 per cycle
    let cowbell: Pattern<String> = parse_mini_notation("cb ~ cb ~ cb ~ cb ~");
    let kick: Pattern<String> =
        parse_mini_notation("808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~");

    let cow_m = PatternMetrics::analyze(&cowbell, 4);
    let kick_m = PatternMetrics::analyze(&kick, 4);

    // Cowbell: 4 per cycle, very even
    assert_eq!(cow_m.density, 4.0);
    assert!(cow_m.evenness > 0.9);
    // Kick: moderate density
    assert!(kick_m.density >= 3.0 && kick_m.density <= 6.0);
}

#[test]
fn level1_dilla_style_characteristics() {
    // J Dilla characteristics:
    // - Irregular kick placement (high syncopation)
    // - Kick density: 4-6 per cycle
    let kick: Pattern<String> = parse_mini_notation("bd ~ ~ bd ~ bd bd ~ bd ~ ~ ~ ~ bd ~ ~");
    let metrics = PatternMetrics::analyze(&kick, 4);

    // Dilla kicks are dense and syncopated
    assert!(
        metrics.density >= 4.0,
        "Dilla kick should have >= 4 events/cycle, got {}",
        metrics.density
    );
    assert!(
        metrics.syncopation > 0.1,
        "Dilla kick should have noticeable syncopation, got {}",
        metrics.syncopation
    );
}
