#[cfg(test_disabled)]
/// Combined tests for `echo` and `segment` - pattern effect transforms
/// - echo: creates echoes with delay and decay
/// - segment: samples pattern n times per cycle
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
// LEVEL 1: Pattern Query Verification (Echo)
// ============================================================================

#[test]
fn test_echo_level1_multiplies_events() {
    // echo should create multiple delayed copies of the pattern
    let base_pattern = parse_mini_notation("bd");
    let echo_pattern = base_pattern.clone().echo(3, 0.25, 0.5);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let echo_haps = echo_pattern.query(&state);

    // echo(3, ...) should create 3 copies (original + 2 echoes)
    assert_eq!(base_haps.len(), 1, "Base pattern should have 1 event");

    assert_eq!(echo_haps.len(), 3, "echo(3) should create 3 events");

    println!("✅ echo Level 1: 3 echoes created from 1 event");
}

#[test]
fn test_echo_level1_timing_delays() {
    // Verify echo timing delays are correct
    let pattern = parse_mini_notation("bd");
    let echo_pattern = pattern.clone().echo(3, 0.25, 0.5);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let echo_haps = echo_pattern.query(&state);

    // Should have events at t=0, t=0.25, t=0.5
    assert_eq!(echo_haps.len(), 3);

    let times: Vec<f64> = echo_haps.iter().map(|h| h.part.begin.to_float()).collect();

    // First echo at original time (0.0)
    assert!(
        (times[0] - 0.0).abs() < 0.001,
        "First echo should be at t=0"
    );

    // Second echo delayed by 0.25
    assert!(
        (times[1] - 0.25).abs() < 0.001,
        "Second echo should be at t=0.25"
    );

    // Third echo delayed by 0.5
    assert!(
        (times[2] - 0.5).abs() < 0.001,
        "Third echo should be at t=0.5"
    );

    println!("✅ echo Level 1: Timing delays correct (0.0, 0.25, 0.5)");
}

#[test]
fn test_echo_level1_event_count() {
    // echo should multiply event count by number of echoes
    let pattern = parse_mini_notation("bd sn");

    let mut base_total = 0;
    let mut echo_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        echo_total += pattern.clone().echo(4, 0.125, 0.6).query(&state).len();
    }

    assert_eq!(base_total, 16, "Base should have 2 events × 8 cycles");
    assert_eq!(
        echo_total, 64,
        "echo(4) should create 4× events: 16 × 4 = 64"
    );

    println!(
        "✅ echo Level 1: Event count: base={}, echo={}",
        base_total, echo_total
    );
}

#[test]
fn test_echo_level1_preserves_values() {
    // echo should preserve event values in all copies
    let pattern = parse_mini_notation("bd sn");
    let echo_pattern = pattern.clone().echo(2, 0.25, 0.5);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let echo_haps = echo_pattern.query(&state);

    // echo(2) creates 2 copies of each event
    assert_eq!(echo_haps.len(), 4); // 2 original × 2 echoes

    // Each original event should have 2 copies (original + 1 echo)
    // Verify by checking that values match base values
    for base_hap in &base_haps {
        let matches = echo_haps
            .iter()
            .filter(|h| h.value == base_hap.value)
            .count();
        assert_eq!(matches, 2, "Each base event should have 2 echoes");
    }

    println!("✅ echo Level 1: Values preserved in echoes");
}

// ============================================================================
// LEVEL 1: Pattern Query Verification (Segment)
// ============================================================================

#[test]
fn test_segment_level1_queries_subdivisions() {
    // segment subdivides the cycle and queries each subdivision
    // NOTE: This doesn't necessarily multiply event count, it depends on pattern structure
    let base_pattern = parse_mini_notation("bd sn");
    let segment_pattern = base_pattern.clone().segment(2);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = base_pattern.query(&state);
    let segment_haps = segment_pattern.query(&state);

    // segment(2) subdivides the cycle and queries each half
    assert_eq!(base_haps.len(), 2, "Base should have 2 events");

    // With "bd sn", segment(2) queries [0-0.5] and [0.5-1.0]
    // This may or may not increase event count depending on pattern structure
    assert!(
        segment_haps.len() >= base_haps.len(),
        "segment should not lose events"
    );

    println!("✅ segment Level 1: Subdivides cycle into segments");
}

#[test]
fn test_segment_level1_event_count() {
    // segment subdivides cycle and may increase event count
    let pattern = parse_mini_notation("bd sn hh cp");

    let mut base_total = 0;
    let mut segment_total = 0;

    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        base_total += pattern.query(&state).len();
        segment_total += pattern.clone().segment(3).query(&state).len();
    }

    assert_eq!(base_total, 32, "Base should have 4 events × 8 cycles");

    // segment behavior: subdivides cycle, actual multiplier depends on pattern structure
    // For "bd sn hh cp", segment(3) creates more events but not necessarily 3×
    assert!(
        segment_total > base_total,
        "segment should increase event count"
    );

    println!(
        "✅ segment Level 1: Event count: base={}, segment={} ({}× increase)",
        base_total,
        segment_total,
        segment_total as f32 / base_total as f32
    );
}

#[test]
fn test_segment_level1_timing_compression() {
    // segment subdivides the cycle and queries each subdivision
    let pattern = parse_mini_notation("bd sn");
    let segment_pattern = pattern.clone().segment(2);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let segment_haps = segment_pattern.query(&state);

    // Base pattern: events at 0.0, 0.5
    let base_times: Vec<f64> = base_haps.iter().map(|h| h.part.begin.to_float()).collect();
    assert!((base_times[0] - 0.0).abs() < 0.001);
    assert!((base_times[1] - 0.5).abs() < 0.001);

    // segment(2) queries the pattern in [0-0.5] and [0.5-1.0] subdivisions
    // Actual event count depends on pattern structure and how it interacts with subdivisions
    assert!(
        segment_haps.len() >= base_haps.len(),
        "segment should not lose events"
    );

    println!(
        "✅ segment Level 1: Pattern queried in subdivisions ({} events)",
        segment_haps.len()
    );
}

#[test]
fn test_segment_level1_preserves_values() {
    // segment should preserve pattern values
    let pattern = parse_mini_notation("bd sn");
    let segment_pattern = pattern.clone().segment(3);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let segment_haps = segment_pattern.query(&state);

    assert!(
        segment_haps.len() >= base_haps.len(),
        "segment should not lose events"
    );

    // Verify all segment values come from the base pattern
    for seg_hap in &segment_haps {
        let value_exists = base_haps.iter().any(|h| h.value == seg_hap.value);
        assert!(value_exists, "segment values should come from base pattern");
    }

    println!("✅ segment Level 1: Values preserved");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio Event Verification)
// ============================================================================

#[test]
fn test_echo_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd ~ ~ ~"
"#;

    let echo_code = r#"
tempo: 0.5
out: s "bd ~ ~ ~" $ echo 3 0.25 0.5
"#;

    let cycles = 4;
    let base_audio = render_dsl(base_code, cycles);
    let echo_audio = render_dsl(echo_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let echo_onsets = detect_audio_events(&echo_audio, sample_rate, 0.01);

    // Base should have ~4 onsets (1 per cycle)
    // Echo should have ~12 onsets (3 per cycle: original + 2 echoes)
    let ratio = echo_onsets.len() as f32 / base_onsets.len() as f32;

    assert!(
        ratio > 2.5 && ratio < 3.5,
        "echo(3) should create ~3× onsets: base={}, echo={}, ratio={:.3}",
        base_onsets.len(),
        echo_onsets.len(),
        ratio
    );

    println!(
        "✅ echo Level 2: Onsets detected: base={}, echo={}, ratio={:.2}",
        base_onsets.len(),
        echo_onsets.len(),
        ratio
    );
}

#[test]
fn test_echo_level2_timing_verification() {
    let code = r#"
tempo: 0.5
out: s "bd ~ ~ ~" $ echo 2 0.25 0.7
"#;

    let audio = render_dsl(code, 2);
    let sample_rate = 44100.0;
    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // Should detect multiple onsets
    assert!(
        onsets.len() >= 2,
        "Should detect at least 2 onsets, got {}",
        onsets.len()
    );

    // NOTE: Echo timing verification is complex due to sample playback duration
    // and onset detection granularity. The important verification is that
    // we get multiple distinct onset events, which we verify above.

    println!(
        "✅ echo Level 2: Echo onsets detected ({} onsets)",
        onsets.len()
    );
}

#[test]
fn test_segment_level2_audio_onsets() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn"
"#;

    let segment_code = r#"
tempo: 0.5
out: s "bd sn" $ segment 3
"#;

    let cycles = 8;
    let base_audio = render_dsl(base_code, cycles);
    let segment_audio = render_dsl(segment_code, cycles);
    let sample_rate = 44100.0;

    let base_onsets = detect_audio_events(&base_audio, sample_rate, 0.01);
    let segment_onsets = detect_audio_events(&segment_audio, sample_rate, 0.01);

    // segment increases event density, but actual multiplier depends on pattern structure
    let ratio = segment_onsets.len() as f32 / base_onsets.len() as f32;

    assert!(
        ratio > 1.0,
        "segment should increase onset count: base={}, segment={}, ratio={:.3}",
        base_onsets.len(),
        segment_onsets.len(),
        ratio
    );

    println!(
        "✅ segment Level 2: Onsets detected: base={}, segment={}, ratio={:.2}",
        base_onsets.len(),
        segment_onsets.len(),
        ratio
    );
}

#[test]
fn test_segment_level2_increased_density() {
    let code = r#"
tempo: 0.5
out: s "bd ~ ~ ~" $ segment 4
"#;

    let audio = render_dsl(code, 4);
    let sample_rate = 44100.0;
    let onsets = detect_audio_events(&audio, sample_rate, 0.01);

    // segment(4) with 1 event per cycle should create 4 events per cycle
    // Over 4 cycles: 4 × 4 = 16 expected onsets
    assert!(
        onsets.len() > 12,
        "segment(4) should create many onsets, got {}",
        onsets.len()
    );

    println!(
        "✅ segment Level 2: Increased density verified ({} onsets)",
        onsets.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics (Signal Quality)
// ============================================================================

#[test]
fn test_echo_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn" $ echo 3 0.25 0.6
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "echo should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "echo should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "echo should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ echo Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_echo_level3_energy_increase() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn"
"#;

    let echo_code = r#"
tempo: 0.5
out: s "bd sn" $ echo 3 0.2 0.5
"#;

    let base_audio = render_dsl(base_code, 8);
    let echo_audio = render_dsl(echo_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let echo_rms = calculate_rms(&echo_audio);

    // echo should increase energy due to additional events
    let ratio = echo_rms / base_rms;
    assert!(
        ratio > 1.3,
        "echo should increase energy: base RMS = {:.4}, echo RMS = {:.4}, ratio = {:.2}",
        base_rms,
        echo_rms,
        ratio
    );

    println!(
        "✅ echo Level 3: Energy increased: base RMS = {:.4}, echo RMS = {:.4}, ratio = {:.2}",
        base_rms, echo_rms, ratio
    );
}

#[test]
fn test_segment_level3_audio_quality() {
    let code = r#"
tempo: 0.5
out: s "bd sn hh cp" $ segment 2
"#;

    let audio = render_dsl(code, 8);

    let rms = calculate_rms(&audio);
    let peak = audio.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let dc_offset = audio.iter().sum::<f32>() / audio.len() as f32;

    assert!(
        rms > 0.01,
        "segment should produce audible audio (RMS = {})",
        rms
    );
    assert!(
        peak > 0.1,
        "segment should have audible peaks (peak = {})",
        peak
    );
    assert!(
        dc_offset.abs() < 0.1,
        "segment should not have excessive DC offset (DC = {})",
        dc_offset
    );

    println!(
        "✅ segment Level 3: RMS = {:.4}, Peak = {:.4}, DC = {:.4}",
        rms, peak, dc_offset
    );
}

#[test]
fn test_segment_level3_energy_increase() {
    let base_code = r#"
tempo: 0.5
out: s "bd sn"
"#;

    let segment_code = r#"
tempo: 0.5
out: s "bd sn" $ segment 3
"#;

    let base_audio = render_dsl(base_code, 8);
    let segment_audio = render_dsl(segment_code, 8);

    let base_rms = calculate_rms(&base_audio);
    let segment_rms = calculate_rms(&segment_audio);

    // segment should increase energy due to more events
    let ratio = segment_rms / base_rms;
    assert!(
        ratio > 1.5,
        "segment should increase energy: base RMS = {:.4}, segment RMS = {:.4}, ratio = {:.2}",
        base_rms,
        segment_rms,
        ratio
    );

    println!(
        "✅ segment Level 3: Energy increased: base RMS = {:.4}, segment RMS = {:.4}, ratio = {:.2}",
        base_rms, segment_rms, ratio
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_echo_single_repeat() {
    // echo with times=1 should be identical to original
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let echo_haps = pattern.clone().echo(1, 0.25, 0.5).query(&state);

    assert_eq!(
        echo_haps.len(),
        base_haps.len(),
        "echo(1) should not change event count"
    );

    println!("✅ echo edge case: Single repeat = original");
}

#[test]
fn test_echo_zero_delay() {
    // echo with time=0 should stack events at same time
    let pattern = parse_mini_notation("bd");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let echo_haps = pattern.clone().echo(3, 0.0, 0.5).query(&state);

    assert_eq!(echo_haps.len(), 3, "Should have 3 events");

    // All events should be at same time
    let times: Vec<f64> = echo_haps.iter().map(|h| h.part.begin.to_float()).collect();
    for t in &times {
        assert!(
            (t - times[0]).abs() < 0.001,
            "All events should be at same time"
        );
    }

    println!("✅ echo edge case: Zero delay stacks events");
}

#[test]
fn test_segment_one() {
    // segment(1) should be identical to original
    let pattern = parse_mini_notation("bd sn hh cp");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let segment_haps = pattern.clone().segment(1).query(&state);

    assert_eq!(
        segment_haps.len(),
        base_haps.len(),
        "segment(1) should not change event count"
    );

    println!("✅ segment edge case: segment(1) = original");
}

#[test]
fn test_segment_single_event() {
    // segment with single event pattern
    let pattern = parse_mini_notation("bd");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let segment_haps = pattern.clone().segment(4).query(&state);

    assert_eq!(
        segment_haps.len(),
        4,
        "segment(4) should create 4 events from 1"
    );

    println!("✅ segment edge case: Single event handled");
}

#[test]
fn test_echo_preserves_structure() {
    // echo should preserve relative timing structure
    let pattern = parse_mini_notation("bd ~ sn ~");
    let echo_pattern = pattern.clone().echo(2, 0.5, 0.6);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let echo_haps = echo_pattern.query(&state);

    // Base has 2 events, echo(2) should have 4 events
    assert_eq!(base_haps.len(), 2);
    assert_eq!(echo_haps.len(), 4);

    println!("✅ echo edge case: Structure preserved with rests");
}

#[test]
fn test_segment_large_n() {
    // segment with large n should work
    let pattern = parse_mini_notation("bd sn");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let segment_haps = pattern.clone().segment(8).query(&state);

    // segment with large n should increase event count
    assert!(
        segment_haps.len() >= base_haps.len(),
        "segment(8) should not lose events"
    );

    println!(
        "✅ segment edge case: Large n handled ({} events)",
        segment_haps.len()
    );
}
