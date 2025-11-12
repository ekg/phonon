/// Combined tests for `degrade` (50% removal) and `degradeBy` (custom % removal)
/// Both use per-event probabilistic removal with deterministic RNG
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Event Removal)
// ============================================================================

#[test]
fn test_degrade_level1_event_removal() {
    // degrade should remove ~50% of events (per-event decision)
    let base_pattern = parse_mini_notation("a b c d e f g h");
    let degrade_pattern = base_pattern.clone().degrade();

    let mut base_total = 0;
    let mut degrade_total = 0;

    for cycle in 0..20 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += base_pattern.query(&state).len();
        degrade_total += degrade_pattern.query(&state).len();
    }

    let removal_ratio = degrade_total as f64 / base_total as f64;
    assert!(
        removal_ratio >= 0.35 && removal_ratio <= 0.65,
        "degrade should keep ~50% of events: kept {}/{} = {:.1}%",
        degrade_total,
        base_total,
        removal_ratio * 100.0
    );

    println!(
        "✅ degrade Level 1: Kept {}/{} events = {:.1}%",
        degrade_total,
        base_total,
        removal_ratio * 100.0
    );
}

#[test]
fn test_degradeBy_level1_custom_probability() {
    // degradeBy 0.25 should remove ~25% of events (keep 75%)
    let base_pattern = parse_mini_notation("a b c d e f g h");
    let degrade_pattern = base_pattern.clone().degrade_by(Pattern::pure(0.25));

    let mut base_total = 0;
    let mut degrade_total = 0;

    for cycle in 0..20 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += base_pattern.query(&state).len();
        degrade_total += degrade_pattern.query(&state).len();
    }

    let keep_ratio = degrade_total as f64 / base_total as f64;
    assert!(
        keep_ratio >= 0.6 && keep_ratio <= 0.9,
        "degradeBy 0.25 should keep ~75% of events: kept {}/{} = {:.1}%",
        degrade_total,
        base_total,
        keep_ratio * 100.0
    );

    println!(
        "✅ degradeBy Level 1: Kept {}/{} events = {:.1}%",
        degrade_total,
        base_total,
        keep_ratio * 100.0
    );
}

#[test]
fn test_degrade_level1_deterministic() {
    // Same event in same position should always get same random value
    let pattern = parse_mini_notation("a b c d");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let degrade_pattern = pattern.clone().degrade();
    let events1 = degrade_pattern.query(&state);
    let events2 = degrade_pattern.query(&state);

    assert_eq!(
        events1.len(),
        events2.len(),
        "degrade should be deterministic"
    );

    println!("✅ degrade Level 1: Deterministic behavior verified");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_degrade_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp"
"#;

    let degrade_code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp" $ degrade
"#;

    let cycles = 20;
    let base_audio = render_dsl(base_code, cycles);
    let degrade_audio = render_dsl(degrade_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let degrade_onsets = detect_audio_events(&degrade_audio, sample_rate, 0.01);

    // degrade removes ~50% of events
    let ratio = degrade_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.3 && ratio < 0.7,
        "degrade should keep ~50% of onsets: base={}, degrade={}, ratio={:.2}",
        base_onsets.len(),
        degrade_onsets.len(),
        ratio
    );

    println!(
        "✅ degrade Level 2: Base onsets = {}, degrade onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        degrade_onsets.len(),
        ratio
    );
}

#[test]
fn test_degradeBy_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp"
"#;

    let degrade_code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp" $ degradeBy 0.75
"#;

    let cycles = 20;
    let base_audio = render_dsl(base_code, cycles);
    let degrade_audio = render_dsl(degrade_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let degrade_onsets = detect_audio_events(&degrade_audio, sample_rate, 0.01);

    // degradeBy 0.75 removes ~75% of events (keeps 25%)
    let ratio = degrade_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.1 && ratio < 0.4,
        "degradeBy 0.75 should keep ~25% of onsets: base={}, degrade={}, ratio={:.2}",
        base_onsets.len(),
        degrade_onsets.len(),
        ratio
    );

    println!(
        "✅ degradeBy Level 2: Base onsets = {}, degrade onsets = {}, ratio = {:.2}",
        base_onsets.len(),
        degrade_onsets.len(),
        ratio
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_degrade_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp" $ degrade
"#;

    let audio = render_dsl(code, 20);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "degrade should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "degrade should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "degrade should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ degrade Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_degrade_level3_energy_reduction() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp"
"#;

    let degrade_code = r#"
tempo: 0.5
out: s "bd sn hh cp bd sn hh cp" $ degrade
"#;

    let base_audio = render_dsl(base_code, 20);
    let degrade_audio = render_dsl(degrade_code, 20);

    let base_rms = calculate_rms(&base_audio);
    let degrade_rms = calculate_rms(&degrade_audio);

    // degrade removes ~50% of events, so energy should be lower
    let ratio = degrade_rms / base_rms;
    assert!(
        ratio > 0.3 && ratio < 0.8,
        "degrade should reduce energy to ~50%: base RMS = {:.4}, degrade RMS = {:.4}, ratio = {:.2}",
        base_rms,
        degrade_rms,
        ratio
    );

    println!(
        "✅ degrade Level 3: Base RMS = {:.4}, degrade RMS = {:.4}, ratio = {:.2}",
        base_rms, degrade_rms, ratio
    );
}

#[test]
fn test_degradeBy_level3_energy_reduction() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn hh cp"
"#;

    let degrade_code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ degradeBy 0.9
"#;

    let base_audio = render_dsl(base_code, 20);
    let degrade_audio = render_dsl(degrade_code, 20);

    let base_rms = calculate_rms(&base_audio);
    let degrade_rms = calculate_rms(&degrade_audio);

    // degradeBy 0.9 removes ~90% of events (keeps 10%)
    // Energy doesn't scale linearly - fewer events = less overlap
    let ratio = degrade_rms / base_rms;
    assert!(
        ratio > 0.05 && ratio < 0.5,
        "degradeBy 0.9 should reduce energy significantly: base RMS = {:.4}, degrade RMS = {:.4}, ratio = {:.2}",
        base_rms,
        degrade_rms,
        ratio
    );

    println!(
        "✅ degradeBy Level 3: Base RMS = {:.4}, degrade RMS = {:.4}, ratio = {:.2}",
        base_rms, degrade_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_degrade_preserves_timing() {
    // Remaining events should have original timing
    let base_pattern = parse_mini_notation("a b c d");
    let degrade_pattern = base_pattern.clone().degrade();

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let degrade_haps = degrade_pattern.query(&state);

    // Each remaining event should match original timing
    for degrade_hap in &degrade_haps {
        let matching_base = base_haps
            .iter()
            .find(|h| h.value == degrade_hap.value && h.part.begin == degrade_hap.part.begin);
        assert!(
            matching_base.is_some(),
            "Degraded event should have original timing"
        );
    }

    println!("✅ degrade edge case: Timing preserved for remaining events");
}

#[test]
fn test_degradeBy_zero_removes_nothing() {
    // degradeBy 0.0 should keep all events
    let base_pattern = parse_mini_notation("a b c d");
    let degrade_pattern = base_pattern.clone().degrade_by(Pattern::pure(0.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let degrade_haps = degrade_pattern.query(&state);

    assert_eq!(
        degrade_haps.len(),
        base_haps.len(),
        "degradeBy 0.0 should keep all events"
    );

    println!("✅ degradeBy edge case: 0.0 probability keeps all events");
}

#[test]
fn test_degradeBy_one_removes_all() {
    // degradeBy 1.0 should remove all events
    let base_pattern = parse_mini_notation("a b c d");
    let degrade_pattern = base_pattern.clone().degrade_by(Pattern::pure(1.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let degrade_haps = degrade_pattern.query(&state);

    assert_eq!(
        degrade_haps.len(),
        0,
        "degradeBy 1.0 should remove all events"
    );

    println!("✅ degradeBy edge case: 1.0 probability removes all events");
}

#[test]
fn test_degrade_long_term_probability() {
    // Verify long-term probability approaches 50% removal
    let pattern = parse_mini_notation("a b c d e f g h");

    let mut base_total = 0;
    let mut degrade_total = 0;
    let total_cycles = 100;

    for cycle in 0..total_cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let degrade_pattern = pattern.clone().degrade();
        base_total += pattern.query(&state).len();
        degrade_total += degrade_pattern.query(&state).len();
    }

    let keep_ratio = degrade_total as f64 / base_total as f64;

    // With 100 cycles × 8 events = 800 events, should be very close to 50%
    assert!(
        keep_ratio >= 0.45 && keep_ratio <= 0.55,
        "Long-term degrade should keep ~50%: {}/{} = {:.1}%",
        degrade_total,
        base_total,
        keep_ratio * 100.0
    );

    println!(
        "✅ degrade edge case: Long-term ratio = {:.1}% ({}/{})",
        keep_ratio * 100.0,
        degrade_total,
        base_total
    );
}
