/// Combined tests for `legato` (longer duration) and `staccato` (shorter duration)
/// Both modify event duration by a multiplicative factor (staccato is alias for legato)
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
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Duration Modification)
// ============================================================================

#[test]
fn test_legato_level1_extends_duration() {
    // legato should multiply event duration
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let legato_pattern = base_pattern.clone().legato(Pattern::pure(2.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let legato_haps = legato_pattern.query(&state);

    // Same number of events
    assert_eq!(
        legato_haps.len(),
        base_haps.len(),
        "legato should preserve event count"
    );

    // Each event should have 2x duration
    for i in 0..legato_haps.len() {
        let base_duration = base_haps[i].part.duration().to_float();
        let legato_duration = legato_haps[i].part.duration().to_float();

        assert!(
            (legato_duration - base_duration * 2.0).abs() < 0.001,
            "Event {} should have 2x duration: base={}, legato={}",
            i,
            base_duration,
            legato_duration
        );
    }

    println!("✅ legato Level 1: Duration extended by 2x");
}

#[test]
fn test_staccato_level1_shortens_duration() {
    // staccato should shorten event duration
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let staccato_pattern = base_pattern.clone().staccato(Pattern::pure(0.5));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let staccato_haps = staccato_pattern.query(&state);

    // Same number of events
    assert_eq!(
        staccato_haps.len(),
        base_haps.len(),
        "staccato should preserve event count"
    );

    // Each event should have 0.5x duration
    for i in 0..staccato_haps.len() {
        let base_duration = base_haps[i].part.duration().to_float();
        let staccato_duration = staccato_haps[i].part.duration().to_float();

        assert!(
            (staccato_duration - base_duration * 0.5).abs() < 0.001,
            "Event {} should have 0.5x duration: base={}, staccato={}",
            i,
            base_duration,
            staccato_duration
        );
    }

    println!("✅ staccato Level 1: Duration shortened to 0.5x");
}

#[test]
fn test_legato_level1_event_count() {
    // legato should preserve all events
    let pattern = parse_mini_notation("bd sn hh cp bd sn hh cp");

    let mut base_total = 0;
    let mut legato_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        legato_total += pattern
            .clone()
            .legato(Pattern::pure(1.5))
            .query(&state)
            .len();
    }

    assert_eq!(
        legato_total, base_total,
        "legato should preserve all events"
    );

    println!("✅ legato Level 1: Event count preserved: {}", base_total);
}

#[test]
fn test_legato_level1_preserves_start_time() {
    // legato should only change duration, not start time
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let legato_pattern = base_pattern.clone().legato(Pattern::pure(2.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let legato_haps = legato_pattern.query(&state);

    for i in 0..legato_haps.len() {
        assert_eq!(
            legato_haps[i].part.begin, base_haps[i].part.begin,
            "Event {} start time should not change",
            i
        );
    }

    println!("✅ legato Level 1: Start times preserved");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_legato_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp"
"#;

    let legato_code = r#"
tempo: 0.5
out $ s "bd sn hh cp bd sn hh cp" $ legato 1.5
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let legato_audio = render_dsl(legato_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let legato_onsets = detect_audio_events(&legato_audio, sample_rate, 0.01);

    // Same number of onsets (duration doesn't affect onset detection much)
    let ratio = legato_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.95 && ratio < 1.05,
        "legato should preserve onset count: base={}, legato={}, ratio={:.3}",
        base_onsets.len(),
        legato_onsets.len(),
        ratio
    );

    println!(
        "✅ legato Level 2: Onsets detected: base={}, legato={}",
        base_onsets.len(),
        legato_onsets.len()
    );
}

#[test]
fn test_staccato_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let staccato_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ staccato 0.25
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let staccato_audio = render_dsl(staccato_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let staccato_onsets = detect_audio_events(&staccato_audio, sample_rate, 0.01);

    // Same number of onsets
    let ratio = staccato_onsets.len() as f32 / base_onsets.len() as f32;
    assert!(
        ratio > 0.95 && ratio < 1.05,
        "staccato should preserve onset count: base={}, staccato={}, ratio={:.3}",
        base_onsets.len(),
        staccato_onsets.len(),
        ratio
    );

    println!(
        "✅ staccato Level 2: Onsets detected: base={}, staccato={}",
        base_onsets.len(),
        staccato_onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_legato_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ legato 2.0
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "legato should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "legato should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "legato should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ legato Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_staccato_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ staccato 0.5
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "staccato should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "staccato should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "staccato should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ staccato Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_legato_level3_energy_comparison() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let legato_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ legato 2.0
"#;

    let base_audio = render_dsl(base_code, 8);
    let legato_audio = render_dsl(legato_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let legato_rms = calculate_rms(&legato_audio);

    // legato extends duration, potentially increasing energy if samples overlap
    let ratio = legato_rms / base_rms;
    assert!(
        ratio > 0.8 && ratio < 1.5,
        "legato energy should be reasonable: base RMS = {:.4}, legato RMS = {:.4}, ratio = {:.2}",
        base_rms,
        legato_rms,
        ratio
    );

    println!(
        "✅ legato Level 3: Base RMS = {:.4}, legato RMS = {:.4}, ratio = {:.2}",
        base_rms, legato_rms, ratio
    );
}

#[test]
fn test_staccato_level3_energy_comparison() {
    let base_code = r#"
tempo: 0.5
out $ s "bd sn hh cp"
"#;

    let staccato_code = r#"
tempo: 0.5
out $ s "bd sn hh cp" $ staccato 0.3
"#;

    let base_audio = render_dsl(base_code, 8);
    let staccato_audio = render_dsl(staccato_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let staccato_rms = calculate_rms(&staccato_audio);

    // staccato shortens duration, potentially reducing energy
    let ratio = staccato_rms / base_rms;
    assert!(
        ratio > 0.5 && ratio < 1.2,
        "staccato energy should be reasonable: base RMS = {:.4}, staccato RMS = {:.4}, ratio = {:.2}",
        base_rms,
        staccato_rms,
        ratio
    );

    println!(
        "✅ staccato Level 3: Base RMS = {:.4}, staccato RMS = {:.4}, ratio = {:.2}",
        base_rms, staccato_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_legato_factor_one() {
    // legato(1.0) should have no effect
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let legato_pattern = base_pattern.clone().legato(Pattern::pure(1.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let legato_haps = legato_pattern.query(&state);

    for i in 0..base_haps.len() {
        let base_duration = base_haps[i].part.duration().to_float();
        let legato_duration = legato_haps[i].part.duration().to_float();

        assert!(
            (legato_duration - base_duration).abs() < 0.001,
            "legato(1.0) should preserve duration"
        );
    }

    println!("✅ legato edge case: Factor 1.0 preserves duration");
}

#[test]
fn test_staccato_preserves_values() {
    // staccato should only affect duration, not values
    let base_pattern = parse_mini_notation("bd sn hh cp");
    let staccato_pattern = base_pattern.clone().staccato(Pattern::pure(0.5));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let staccato_haps = staccato_pattern.query(&state);

    for i in 0..base_haps.len() {
        assert_eq!(
            staccato_haps[i].value, base_haps[i].value,
            "staccato should preserve event values"
        );
    }

    println!("✅ staccato edge case: Values preserved");
}

#[test]
fn test_legato_extreme_values() {
    // Test legato with very large and very small factors
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    // Very short
    let short = pattern.clone().legato(Pattern::pure(0.1)).query(&state);
    assert!(
        short.len() > 0 && short[0].part.duration().to_float() < 0.1,
        "legato(0.1) should create very short events"
    );

    // Very long
    let long = pattern.clone().legato(Pattern::pure(5.0)).query(&state);
    assert!(
        long.len() > 0 && long[0].part.duration().to_float() > 1.0,
        "legato(5.0) should create very long events"
    );

    println!("✅ legato edge case: Extreme factors handled");
}
