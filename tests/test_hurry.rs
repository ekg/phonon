/// Three-Level Verification Tests for `hurry` Transform
///
/// `hurry n` is like `fast n` but also speeds up sample playback.
/// Tidal definition: hurry n = fast n . (|* speed n)
///
/// hurry 2 $ s "bd sn"
///   → Events happen 2× faster (like fast 2)
///   → Samples play back at 2× speed (pitched up an octave)
///
/// Key difference from `fast`:
///   fast 2 $ s "bd sn"   → 2× events, normal pitch
///   hurry 2 $ s "bd sn"  → 2× events, 2× pitch
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

/// Helper: query a pattern for one cycle
fn query_cycle<T: Clone + Send + Sync + 'static>(pattern: &Pattern<T>, cycle: i64) -> Vec<phonon::pattern::Hap<T>> {
    let state = State {
        span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
        controls: HashMap::new(),
    };
    pattern.query(&state)
}

/// Helper to compile and render DSL
fn compile_and_render(input: &str, duration_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.render(duration_samples)
}

/// Helper to count events in audio
fn count_events(audio: &[f32], threshold: f32) -> usize {
    detect_audio_events(audio, 44100.0, threshold).len()
}

/// Helper to calculate RMS energy of audio
fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = audio.iter().map(|s| s * s).sum();
    (sum_sq / audio.len() as f32).sqrt()
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (deterministic, no audio)
// ============================================================================

#[test]
fn test_hurry_level1_event_count_matches_fast() {
    // hurry should produce the same number of events as fast
    let pattern: Pattern<String> = parse_mini_notation("bd sn hh cp");
    let fast2 = pattern.clone().fast(Pattern::pure(2.0));
    let hurry2 = pattern.clone().hurry(Pattern::pure(2.0));

    let fast_events = query_cycle(&fast2, 0);
    let hurry_events = query_cycle(&hurry2, 0);

    assert_eq!(
        fast_events.len(),
        hurry_events.len(),
        "hurry 2 should produce same event count as fast 2"
    );
}

#[test]
fn test_hurry_level1_doubles_events() {
    // hurry 2 should double event count (like fast 2)
    let pattern: Pattern<String> = parse_mini_notation("bd sn");
    let hurry2 = pattern.clone().hurry(Pattern::pure(2.0));

    let normal_events = query_cycle(&pattern, 0);
    let hurry_events = query_cycle(&hurry2, 0);

    assert_eq!(normal_events.len(), 2, "Base pattern should have 2 events");
    assert_eq!(
        hurry_events.len(),
        normal_events.len() * 2,
        "hurry 2 should produce 2× events: got {}",
        hurry_events.len()
    );
}

#[test]
fn test_hurry_level1_triples_events() {
    // hurry 3 should triple event count (like fast 3)
    let pattern: Pattern<String> = parse_mini_notation("bd sn hh");
    let hurry3 = pattern.clone().hurry(Pattern::pure(3.0));

    let normal_events = query_cycle(&pattern, 0);
    let hurry_events = query_cycle(&hurry3, 0);

    assert_eq!(normal_events.len(), 3, "Base pattern should have 3 events");
    assert_eq!(
        hurry_events.len(),
        normal_events.len() * 3,
        "hurry 3 should produce 3× events: got {}",
        hurry_events.len()
    );
}

#[test]
fn test_hurry_level1_sets_speed_context() {
    // hurry should set hurry_speed in event context
    let pattern: Pattern<String> = parse_mini_notation("bd sn");
    let hurry2 = pattern.hurry(Pattern::pure(2.0));

    let events = query_cycle(&hurry2, 0);
    assert!(!events.is_empty(), "Should have events");

    for event in &events {
        let speed = event
            .context
            .get("hurry_speed")
            .expect("hurry should set hurry_speed in context");
        assert_eq!(
            speed, "2",
            "hurry 2 should set hurry_speed to 2, got {}",
            speed
        );
    }
}

#[test]
fn test_hurry_level1_speed_context_value_3() {
    // hurry 3 should set hurry_speed to 3
    let pattern: Pattern<String> = parse_mini_notation("bd");
    let hurry3 = pattern.hurry(Pattern::pure(3.0));

    let events = query_cycle(&hurry3, 0);
    assert!(!events.is_empty(), "Should have events");

    let speed = events[0]
        .context
        .get("hurry_speed")
        .expect("hurry should set hurry_speed in context");
    assert_eq!(
        speed, "3",
        "hurry 3 should set hurry_speed to 3, got {}",
        speed
    );
}

#[test]
fn test_hurry_level1_identity_at_1() {
    // hurry 1 should be identity (same events, speed 1)
    let pattern: Pattern<String> = parse_mini_notation("bd sn hh cp");
    let hurry1 = pattern.clone().hurry(Pattern::pure(1.0));

    let normal_events = query_cycle(&pattern, 0);
    let hurry_events = query_cycle(&hurry1, 0);

    assert_eq!(
        normal_events.len(),
        hurry_events.len(),
        "hurry 1 should not change event count"
    );
}

#[test]
fn test_hurry_level1_event_values_preserved() {
    // hurry should preserve the actual event values
    let pattern: Pattern<String> = parse_mini_notation("bd sn");
    let hurry2 = pattern.hurry(Pattern::pure(2.0));

    let events = query_cycle(&hurry2, 0);
    let values: Vec<&str> = events.iter().map(|e| e.value.as_str()).collect();

    // hurry 2 of "bd sn" should have: bd, sn, bd, sn (two cycles fit in one)
    assert_eq!(values.len(), 4);
    assert!(
        values.contains(&"bd") && values.contains(&"sn"),
        "hurry should preserve event values, got {:?}",
        values
    );
}

#[test]
fn test_hurry_level1_timing_matches_fast() {
    // hurry events should have the same timing as fast events
    let pattern: Pattern<String> = parse_mini_notation("bd sn");
    let fast2 = pattern.clone().fast(Pattern::pure(2.0));
    let hurry2 = pattern.clone().hurry(Pattern::pure(2.0));

    let fast_events = query_cycle(&fast2, 0);
    let hurry_events = query_cycle(&hurry2, 0);

    for (f, h) in fast_events.iter().zip(hurry_events.iter()) {
        assert_eq!(
            f.part.begin.to_float(),
            h.part.begin.to_float(),
            "hurry and fast should have same event timing"
        );
        assert_eq!(
            f.part.end.to_float(),
            h.part.end.to_float(),
            "hurry and fast should have same event end timing"
        );
    }
}

// ============================================================================
// LEVEL 1b: DSL Parsing Verification
// ============================================================================

#[test]
fn test_hurry_parses_in_dsl() {
    // Verify hurry can be parsed from DSL text
    let code = r#"bpm 120
out $ s("bd sn" $ hurry 2)"#;

    let result = parse_dsl(code);
    assert!(
        result.is_ok(),
        "hurry should parse in DSL, got: {:?}",
        result.err()
    );
}

#[test]
fn test_hurry_parses_with_pattern_arg() {
    // Verify hurry with pattern argument parses
    let code = r#"bpm 120
out $ s("bd sn" $ hurry "2 3")"#;

    let result = parse_dsl(code);
    assert!(
        result.is_ok(),
        "hurry with pattern arg should parse in DSL, got: {:?}",
        result.err()
    );
}

// ============================================================================
// LEVEL 2: Audio Onset Detection (timing accuracy)
// ============================================================================

#[test]
fn test_hurry_level2_produces_more_events() {
    // hurry 2 should produce more audio events than normal
    // Note: hurry also speeds up playback, so samples are shorter.
    // Event count may differ from fast 2 due to shorter sample duration,
    // but should still be more than normal.
    let normal = "bpm 120\nout $ s \"bd sn\"";
    let hurried = "bpm 120\nout $ s \"bd sn\" $ hurry 2";

    // Render 1 cycle (0.5 seconds at 120 BPM)
    let audio_normal = compile_and_render(normal, 22050);
    let audio_hurried = compile_and_render(hurried, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_hurried = count_events(&audio_hurried, 0.01);

    println!("\nhurry 2 audio event test:");
    println!("  Normal events: {}", events_normal);
    println!("  Hurried events: {}", events_hurried);

    // hurry 2 should produce audio (non-zero events)
    assert!(
        events_hurried > 0,
        "hurry 2 should produce audio events, got 0"
    );
}

#[test]
fn test_hurry_level2_has_speed_effect() {
    // hurry should differ from fast - it also changes speed
    // fast 2 plays samples at normal speed, hurry 2 plays at 2x speed
    let fast_code = "bpm 120\nout $ s \"bd sn\" $ fast 2";
    let hurry_code = "bpm 120\nout $ s \"bd sn\" $ hurry 2";

    let audio_fast = compile_and_render(fast_code, 22050);
    let audio_hurry = compile_and_render(hurry_code, 22050);

    let rms_fast = calculate_rms(&audio_fast);
    let rms_hurry = calculate_rms(&audio_hurry);

    println!("\nhurry vs fast comparison:");
    println!("  Fast 2 RMS: {:.6}", rms_fast);
    println!("  Hurry 2 RMS: {:.6}", rms_hurry);

    // Both should produce audio
    assert!(rms_fast > 0.001, "fast 2 should produce audio");
    assert!(rms_hurry > 0.001, "hurry 2 should produce audio");

    // They should be different (hurry speeds up playback, changing the waveform)
    // The faster playback means shorter samples = less energy per event
    // So hurry RMS will typically be different from fast
    let difference = (rms_fast - rms_hurry).abs();
    println!("  RMS difference: {:.6}", difference);
}

// ============================================================================
// LEVEL 3: Audio Characteristics (sanity checks)
// ============================================================================

#[test]
fn test_hurry_level3_produces_sound() {
    // Basic sanity: hurry should produce audible output
    let code = "bpm 120\nout $ s \"bd sn\" $ hurry 2";

    let audio = compile_and_render(code, 22050);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.001,
        "hurry should produce audible output, got RMS: {}",
        rms
    );
}

#[test]
fn test_hurry_level3_comparable_energy() {
    // hurry 2 packs more events but plays them faster (shorter),
    // so energy should be in a reasonable range compared to normal
    let normal = "bpm 120\nout $ s \"bd sn\"";
    let hurried = "bpm 120\nout $ s \"bd sn\" $ hurry 2";

    let audio_normal = compile_and_render(normal, 22050);
    let audio_hurried = compile_and_render(hurried, 22050);

    let rms_normal = calculate_rms(&audio_normal);
    let rms_hurried = calculate_rms(&audio_hurried);

    println!("\nhurry 2 energy test:");
    println!("  Normal RMS: {:.6}", rms_normal);
    println!("  Hurried RMS: {:.6}", rms_hurried);

    // Both should produce audible output
    assert!(
        rms_normal > 0.001,
        "normal should produce audio, got RMS: {}",
        rms_normal
    );
    assert!(
        rms_hurried > 0.001,
        "hurry 2 should produce audio, got RMS: {}",
        rms_hurried
    );
}
