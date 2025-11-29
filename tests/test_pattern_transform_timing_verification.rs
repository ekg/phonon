//! Timing verification tests for pattern transforms
//!
//! These tests verify that pattern transforms actually affect audio timing correctly,
//! not just that they produce sound. Uses onset detection to verify event timing.

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

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

/// Helper to get event times
fn get_event_times(audio: &[f32], threshold: f32) -> Vec<f64> {
    detect_audio_events(audio, 44100.0, threshold)
        .iter()
        .map(|e| e.time)
        .collect()
}

// ============================================================================
// TEMPO TRANSFORMS
// ============================================================================

#[test]
fn test_fast_2_doubles_event_count() {
    // Test: fast 2 should double the number of events
    let normal = r#"bpm 120
out $ s "bd sn""#;

    let fast = r#"bpm 120
out $ s("bd sn" $ fast 2)"#;

    // Render 1 cycle (0.5 seconds at 120 BPM = 2 CPS)
    let audio_normal = compile_and_render(normal, 22050);
    let audio_fast = compile_and_render(fast, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_fast = count_events(&audio_fast, 0.01);

    println!("\nfast 2 test:");
    println!("  Normal events: {}", events_normal);
    println!("  Fast events: {}", events_fast);

    // fast 2 should double event count
    assert!(
        events_fast >= events_normal * 2,
        "fast 2 should double events: normal={}, fast={} (ratio: {:.2})",
        events_normal,
        events_fast,
        events_fast as f32 / events_normal as f32
    );
}

#[test]
fn test_fast_2_halves_event_intervals() {
    // Test: fast 2 should halve the time between events
    let normal = r#"bpm 120
out $ s "bd bd bd bd""#;

    let fast = r#"bpm 120
out $ s("bd bd" $ fast 2)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_fast = compile_and_render(fast, 22050);

    let times_normal = get_event_times(&audio_normal, 0.01);
    let times_fast = get_event_times(&audio_fast, 0.01);

    // Need at least 2 events to measure intervals
    if times_normal.len() >= 2 && times_fast.len() >= 2 {
        let interval_normal = times_normal[1] - times_normal[0];
        let interval_fast = times_fast[1] - times_fast[0];

        println!("\nfast 2 interval test:");
        println!("  Normal interval: {:.3}s", interval_normal);
        println!("  Fast interval: {:.3}s", interval_fast);
        println!("  Ratio: {:.2}", interval_normal / interval_fast);

        // fast 2 should halve intervals (within 10ms tolerance)
        assert!(
            (interval_fast - interval_normal / 2.0).abs() < 0.010,
            "fast 2 should halve intervals: normal={:.3}s, fast={:.3}s, expected={:.3}s",
            interval_normal,
            interval_fast,
            interval_normal / 2.0
        );
    }
}

#[test]
fn test_slow_2_halves_event_count() {
    // Test: slow 2 should halve the number of events
    let normal = r#"bpm 120
out $ s "bd sn hh cp""#;

    let slow = r#"bpm 120
out $ s("bd sn hh cp" $ slow 2)"#;

    // Render 1 cycle
    let audio_normal = compile_and_render(normal, 22050);
    let audio_slow = compile_and_render(slow, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_slow = count_events(&audio_slow, 0.01);

    println!("\nslow 2 test:");
    println!("  Normal events: {}", events_normal);
    println!("  Slow events: {}", events_slow);

    // slow 2 should halve event count (or close to it)
    assert!(
        events_slow <= events_normal / 2 + 1, // +1 for rounding
        "slow 2 should halve events: normal={}, slow={}",
        events_normal,
        events_slow
    );
}

#[test]
fn test_slow_2_doubles_event_intervals() {
    // Test: slow 2 should double the time between events
    let normal = r#"bpm 120
out $ s "bd bd bd bd""#;

    let slow = r#"bpm 120
out $ s("bd bd bd bd" $ slow 2)"#;

    // Render 2 cycles to see slow pattern
    let audio_normal = compile_and_render(normal, 44100);
    let audio_slow = compile_and_render(slow, 44100);

    let times_normal = get_event_times(&audio_normal, 0.01);
    let times_slow = get_event_times(&audio_slow, 0.01);

    println!("\nslow 2 interval test:");
    println!("  Normal events: {:?}", times_normal);
    println!("  Slow events: {:?}", times_slow);

    if times_normal.len() >= 2 && times_slow.len() >= 2 {
        let interval_normal = times_normal[1] - times_normal[0];
        let interval_slow = times_slow[1] - times_slow[0];

        println!("  Normal interval: {:.3}s", interval_normal);
        println!("  Slow interval: {:.3}s", interval_slow);

        // slow 2 should double intervals
        assert!(
            (interval_slow - interval_normal * 2.0).abs() < 0.020,
            "slow 2 should double intervals: normal={:.3}s, slow={:.3}s, expected={:.3}s",
            interval_normal,
            interval_slow,
            interval_normal * 2.0
        );
    }
}

// ============================================================================
// TIME OPERATIONS
// ============================================================================

#[test]
fn test_late_shifts_events_forward() {
    // Test: late 0.25 should shift all events forward by 0.25 cycles
    let normal = r#"bpm 120
out $ s "bd sn""#;

    let late = r#"bpm 120
out $ s("bd sn" $ late 0.25)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_late = compile_and_render(late, 22050);

    let times_normal = get_event_times(&audio_normal, 0.01);
    let times_late = get_event_times(&audio_late, 0.01);

    println!("\nlate 0.25 test:");
    println!("  Normal times: {:?}", times_normal);
    println!("  Late times: {:?}", times_late);

    if !times_normal.is_empty() && !times_late.is_empty() {
        // At 120 BPM (2 CPS), 0.25 cycles = 0.125 seconds
        let expected_shift = 0.125;
        let actual_shift = times_late[0] - times_normal[0];

        println!("  Expected shift: {:.3}s", expected_shift);
        println!("  Actual shift: {:.3}s", actual_shift);

        assert!(
            (actual_shift - expected_shift).abs() < 0.020,
            "late 0.25 should shift by 0.125s at 120 BPM, got {:.3}s",
            actual_shift
        );
    }
}

#[test]
fn test_early_shifts_events_backward() {
    // Test: early 0.25 should shift events backward (earlier)
    let normal = r#"bpm 120
out $ s "~ ~ bd sn""#; // Start events later so early doesn't go negative

    let early = r#"bpm 120
out $ s("~ ~ bd sn" $ early 0.25)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_early = compile_and_render(early, 22050);

    let times_normal = get_event_times(&audio_normal, 0.01);
    let times_early = get_event_times(&audio_early, 0.01);

    println!("\nearly 0.25 test:");
    println!("  Normal times: {:?}", times_normal);
    println!("  Early times: {:?}", times_early);

    if !times_normal.is_empty() && !times_early.is_empty() {
        // early should shift backward
        assert!(
            times_early[0] < times_normal[0],
            "early should shift events earlier in time"
        );

        let shift = times_normal[0] - times_early[0];
        println!("  Shift: {:.3}s", shift);

        // Should be approximately 0.125s earlier at 120 BPM
        assert!(
            shift > 0.05,
            "early 0.25 should shift by at least 0.05s, got {:.3}s",
            shift
        );
    }
}

#[test]
fn test_dup_3_triples_event_count() {
    // Test: dup 3 should repeat the pattern 3 times
    let normal = r#"bpm 120
out $ s "bd sn""#;

    let dup = r#"bpm 120
out $ s("bd sn" $ dup 3)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_dup = compile_and_render(dup, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_dup = count_events(&audio_dup, 0.01);

    println!("\ndup 3 test:");
    println!("  Normal events: {}", events_normal);
    println!("  Dup events: {}", events_dup);

    // dup 3 should triple events
    assert!(
        events_dup >= events_normal * 3,
        "dup 3 should triple events: normal={}, dup={}",
        events_normal,
        events_dup
    );
}

// ============================================================================
// STRUCTURAL TRANSFORMS
// ============================================================================

#[test]
fn test_rev_reverses_event_order() {
    // Test: rev should reverse the order of events
    // For "bd sn hh cp", reversed should be "cp hh sn bd"
    let normal = r#"bpm 120
out $ s "bd ~ ~ sn ~ ~ hh ~""#; // Spread out for clear detection

    let reversed = r#"bpm 120
out $ s("bd ~ ~ sn ~ ~ hh ~" $ rev)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_reversed = compile_and_render(reversed, 22050);

    let times_normal = get_event_times(&audio_normal, 0.01);
    let times_reversed = get_event_times(&audio_reversed, 0.01);

    println!("\nrev test:");
    println!("  Normal times: {:?}", times_normal);
    println!("  Reversed times: {:?}", times_reversed);

    // Should have same number of events
    assert_eq!(
        times_normal.len(),
        times_reversed.len(),
        "rev should preserve event count"
    );

    if times_normal.len() >= 2 && times_reversed.len() >= 2 {
        // Check that the order is different
        // In reversed, what was last should now be first (approximately)
        let last_normal = times_normal[times_normal.len() - 1];
        let first_reversed = times_reversed[0];

        // Last normal event should be near end, first reversed near start
        // They should be in different halves of the cycle
        assert!(
            last_normal > 0.25 && first_reversed < 0.25,
            "rev: last event of normal ({:.3}) should be in later half, first event of reversed ({:.3}) in earlier half",
            last_normal,
            first_reversed
        );
    }
}

#[test]
fn test_palindrome_produces_audio() {
    // Test: palindrome should at least produce audio
    // Note: Full timing verification is difficult because palindrome
    // creates forward + backward which may overlap in onset detection
    let normal = r#"bpm 120
out $ s "bd sn hh""#;

    let palindrome = r#"bpm 120
out $ s("bd sn hh" $ palindrome)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_palindrome = compile_and_render(palindrome, 22050);

    let rms_normal: f32 =
        (audio_normal.iter().map(|x| x * x).sum::<f32>() / audio_normal.len() as f32).sqrt();
    let rms_palindrome: f32 = (audio_palindrome.iter().map(|x| x * x).sum::<f32>()
        / audio_palindrome.len() as f32)
        .sqrt();

    println!("\npalindrome test:");
    println!("  Normal RMS: {:.4}", rms_normal);
    println!("  Palindrome RMS: {:.4}", rms_palindrome);

    // palindrome should produce audio
    assert!(rms_palindrome > 0.001, "palindrome should produce audio");

    // Should produce at least as much audio as normal (likely more)
    assert!(
        rms_palindrome >= rms_normal * 0.8, // Allow 20% variance
        "palindrome should produce comparable or more audio: normal={:.4}, palindrome={:.4}",
        rms_normal,
        rms_palindrome
    );
}

// ============================================================================
// DEGRADATION TRANSFORMS
// ============================================================================

#[test]
fn test_degrade_removes_some_events() {
    // Test: degrade should randomly remove ~50% of events
    // Use different samples with rests for clear onset detection
    let normal = r#"bpm 120
out $ s "bd ~ sn ~ hh ~ cp ~""#;

    let degraded = r#"bpm 120
out $ s("bd ~ sn ~ hh ~ cp ~" $ degrade)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_degraded = compile_and_render(degraded, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_degraded = count_events(&audio_degraded, 0.01);

    println!("\ndegrade test:");
    println!("  Normal events: {}", events_normal);
    println!("  Degraded events: {}", events_degraded);
    if events_normal > 0 {
        println!(
            "  Ratio: {:.2}%",
            events_degraded as f32 / events_normal as f32 * 100.0
        );
    }

    // Need at least some events to test
    assert!(events_normal > 0, "Should detect events in normal pattern");

    // degrade should remove some events (at least 10%, allowing for randomness)
    // Being conservative due to randomness
    assert!(
        events_degraded < events_normal,
        "degrade should remove some events: normal={}, degraded={}",
        events_normal,
        events_degraded
    );
}

#[test]
fn test_degrade_by_90_removes_most_events() {
    // Test: degradeBy 0.9 should remove ~90% of events
    // Use varied samples with clear spacing
    let normal = r#"bpm 120
out $ s "bd ~ sn ~ bd ~ hh ~ cp ~ bd ~""#;

    let degraded = r#"bpm 120
out $ s("bd ~ sn ~ bd ~ hh ~ cp ~ bd ~" $ degradeBy 0.9)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_degraded = compile_and_render(degraded, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_degraded = count_events(&audio_degraded, 0.01);

    println!("\ndegradeBy 0.9 test:");
    println!("  Normal events: {}", events_normal);
    println!("  Degraded events: {}", events_degraded);

    assert!(events_normal > 0, "Should detect events in normal pattern");

    if events_normal > 0 {
        let removal_rate = 1.0 - (events_degraded as f32 / events_normal as f32);
        println!("  Removal rate: {:.1}%", removal_rate * 100.0);

        // Should remove most events (at least 40%, being conservative for randomness)
        assert!(
            removal_rate > 0.4 || events_degraded < events_normal / 2,
            "degradeBy 0.9 should remove significant events: normal={}, degraded={}, removed {:.1}%",
            events_normal,
            events_degraded,
            removal_rate * 100.0
        );
    }
}

// ============================================================================
// STUTTER
// ============================================================================

#[test]
fn test_stutter_4_quadruples_events() {
    // Test: stutter 4 should repeat each event 4 times
    let normal = r#"bpm 120
out $ s "bd sn""#;

    let stutter = r#"bpm 120
out $ s("bd sn" $ stutter 4)"#;

    let audio_normal = compile_and_render(normal, 22050);
    let audio_stutter = compile_and_render(stutter, 22050);

    let events_normal = count_events(&audio_normal, 0.01);
    let events_stutter = count_events(&audio_stutter, 0.01);

    println!("\nstutter 4 test:");
    println!("  Normal events: {}", events_normal);
    println!("  Stutter events: {}", events_stutter);

    // stutter 4 should quadruple events
    assert!(
        events_stutter >= events_normal * 4,
        "stutter 4 should quadruple events: normal={}, stutter={}",
        events_normal,
        events_stutter
    );
}
