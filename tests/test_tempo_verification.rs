//! Tempo and BPM verification tests using onset detection
//!
//! This test suite verifies that:
//! 1. BPM settings produce correct cycle rates
//! 2. Onset detection confirms the right number of events
//! 3. Measured tempo matches expected tempo

mod pattern_verification_utils;

use pattern_verification_utils::Event;
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code to audio buffer
fn render_dsl(code: &str, duration_seconds: f64) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_remaining, statements) = parse_program(code).expect("Failed to parse DSL");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile");

    let num_samples = (duration_seconds * sample_rate as f64) as usize;
    let mut buffer = vec![0.0; num_samples];

    for sample in buffer.iter_mut() {
        *sample = graph.process_sample();
    }

    buffer
}

/// Improved onset detection that filters out rapid retriggering
fn detect_musical_onsets(audio: &[f32], sample_rate: f32, threshold: f32, min_interval_ms: f64) -> Vec<Event> {
    let mut events = Vec::new();

    // Simple onset detection: look for sudden increases in RMS
    let window_size = (sample_rate * 0.01) as usize; // 10ms window
    let hop_size = window_size / 4;

    let mut prev_rms = 0.0;
    let mut last_onset_time = -1.0; // Track last onset to enforce minimum interval

    for (i, window) in audio.windows(window_size).step_by(hop_size).enumerate() {
        let rms: f32 = (window.iter().map(|x| x * x).sum::<f32>() / window.len() as f32).sqrt();

        // Detect onset: current RMS is significantly higher than previous
        let onset_strength = (rms - prev_rms).max(0.0);

        if onset_strength > threshold {
            let time = (i * hop_size) as f64 / sample_rate as f64;

            // Only add onset if enough time has passed since last one
            // This filters out internal structure in samples (like kick drum transients)
            if time - last_onset_time >= min_interval_ms / 1000.0 {
                events.push(Event {
                    time,
                    value: None,
                    amplitude: rms,
                });
                last_onset_time = time;
            }
        }

        prev_rms = rms * 0.9; // Decay for next comparison
    }

    events
}

/// Calculate tempo from onset intervals
#[allow(dead_code)]
fn calculate_tempo_from_onsets(onsets: &[Event], _cps: f64) -> Option<f64> {
    if onsets.len() < 2 {
        return None;
    }

    // Calculate intervals between consecutive onsets
    let mut intervals = Vec::new();
    for i in 1..onsets.len() {
        let interval = onsets[i].time - onsets[i - 1].time;
        intervals.push(interval);
    }

    // Average interval
    let avg_interval: f64 = intervals.iter().sum::<f64>() / intervals.len() as f64;

    // Convert to cycles per second
    // If we have 1 event per cycle, interval = 1/cps
    // If we have 4 events per cycle (like bd*4), interval = 1/(cps * 4)
    // We need to figure out events per cycle from the pattern
    Some(avg_interval)
}

#[test]
fn test_bpm_60_produces_correct_tempo() {
    // BPM 60 = 1 beat per second
    // With 4 beats per cycle → 0.25 cycles per second
    let code = r#"
        bpm: 60
        out $ s "bd"
    "#;

    let duration = 8.0; // 8 seconds
    let expected_cps = 60.0 / 240.0; // 0.25 cps
    let expected_cycles = duration * expected_cps; // 2 cycles
    let expected_events = expected_cycles as usize; // 2 events (1 per cycle)

    let audio = render_dsl(code, duration);

    // Level 2: Onset detection with minimum 100ms between onsets
    // This filters out internal sample structure
    let onsets = detect_musical_onsets(&audio, 44100.0, 0.02, 100.0);

    println!("BPM 60 test:");
    println!("  Expected CPS: {:.3}", expected_cps);
    println!("  Expected cycles: {:.3}", expected_cycles);
    println!("  Expected events: {}", expected_events);
    println!("  Detected onsets: {}", onsets.len());
    for (i, onset) in onsets.iter().enumerate() {
        println!("    Onset {}: {:.3}s", i + 1, onset.time);
    }

    // Verify onset count (allow ±1 for boundary effects)
    assert!(
        onsets.len() >= expected_events - 1 && onsets.len() <= expected_events + 1,
        "Expected ~{} onsets, got {}",
        expected_events,
        onsets.len()
    );

    // Verify onset timing - should be ~4 seconds apart (1 cycle = 4 seconds at 0.25 cps)
    if onsets.len() >= 2 {
        let interval = onsets[1].time - onsets[0].time;
        let expected_interval = 1.0 / expected_cps; // 4.0 seconds
        assert!(
            (interval - expected_interval).abs() < 0.1,
            "Expected interval ~{:.3}s, got {:.3}s",
            expected_interval,
            interval
        );
    }
}

#[test]
fn test_bpm_120_produces_correct_tempo() {
    // BPM 120 = 2 beats per second
    // With 4 beats per cycle → 0.5 cycles per second
    let code = r#"
        bpm: 120
        out $ s "bd"
    "#;

    let duration = 8.0; // 8 seconds
    let expected_cps = 120.0 / 240.0; // 0.5 cps
    let expected_cycles = duration * expected_cps; // 4 cycles
    let expected_events = expected_cycles as usize; // 4 events (1 per cycle)

    let audio = render_dsl(code, duration);

    // Level 2: Onset detection with minimum 100ms between onsets
    // This filters out internal sample structure
    let onsets = detect_musical_onsets(&audio, 44100.0, 0.02, 100.0);

    println!("BPM 120 test:");
    println!("  Expected CPS: {:.3}", expected_cps);
    println!("  Expected cycles: {:.3}", expected_cycles);
    println!("  Expected events: {}", expected_events);
    println!("  Detected onsets: {}", onsets.len());
    for (i, onset) in onsets.iter().enumerate() {
        println!("    Onset {}: {:.3}s", i + 1, onset.time);
    }

    // Verify onset count (allow ±1 for boundary effects)
    assert!(
        onsets.len() >= expected_events - 1 && onsets.len() <= expected_events + 1,
        "Expected ~{} onsets, got {}",
        expected_events,
        onsets.len()
    );

    // Verify onset timing - should be ~2 seconds apart (1 cycle = 2 seconds at 0.5 cps)
    if onsets.len() >= 2 {
        let interval = onsets[1].time - onsets[0].time;
        let expected_interval = 1.0 / expected_cps; // 2.0 seconds
        assert!(
            (interval - expected_interval).abs() < 0.1,
            "Expected interval ~{:.3}s, got {:.3}s",
            expected_interval,
            interval
        );
    }
}

#[test]
fn test_bpm_240_produces_correct_tempo() {
    // BPM 240 = 4 beats per second
    // With 4 beats per cycle → 1.0 cycles per second
    let code = r#"
        bpm: 240
        out $ s "bd"
    "#;

    let duration = 8.0; // 8 seconds
    let expected_cps = 240.0 / 240.0; // 1.0 cps
    let expected_cycles = duration * expected_cps; // 8 cycles
    let expected_events = expected_cycles as usize; // 8 events (1 per cycle)

    let audio = render_dsl(code, duration);

    // Level 2: Onset detection with minimum 100ms between onsets
    // This filters out internal sample structure
    let onsets = detect_musical_onsets(&audio, 44100.0, 0.02, 100.0);

    println!("BPM 240 test:");
    println!("  Expected CPS: {:.3}", expected_cps);
    println!("  Expected cycles: {:.3}", expected_cycles);
    println!("  Expected events: {}", expected_events);
    println!("  Detected onsets: {}", onsets.len());
    for (i, onset) in onsets.iter().enumerate().take(10) {
        println!("    Onset {}: {:.3}s", i + 1, onset.time);
    }

    // Verify onset count (allow ±1 for boundary effects)
    assert!(
        onsets.len() >= expected_events - 1 && onsets.len() <= expected_events + 1,
        "Expected ~{} onsets, got {}",
        expected_events,
        onsets.len()
    );

    // Verify onset timing - should be ~1 second apart (1 cycle = 1 second at 1.0 cps)
    if onsets.len() >= 2 {
        let interval = onsets[1].time - onsets[0].time;
        let expected_interval = 1.0 / expected_cps; // 1.0 second
        assert!(
            (interval - expected_interval).abs() < 0.1,
            "Expected interval ~{:.3}s, got {:.3}s",
            expected_interval,
            interval
        );
    }
}

#[test]
fn test_tempo_cps_directly() {
    // tempo: X sets cycles per second directly (not BPM)
    let code = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let duration = 4.0; // 4 seconds
    let expected_cps = 2.0; // 2 cycles per second
    let expected_cycles = duration * expected_cps; // 8 cycles
    let expected_events = expected_cycles as usize; // 8 events

    let audio = render_dsl(code, duration);

    // Level 2: Onset detection with minimum 100ms between onsets
    // This filters out internal sample structure
    let onsets = detect_musical_onsets(&audio, 44100.0, 0.02, 100.0);

    println!("Tempo 2.0 CPS test:");
    println!("  Expected CPS: {:.3}", expected_cps);
    println!("  Expected cycles: {:.3}", expected_cycles);
    println!("  Expected events: {}", expected_events);
    println!("  Detected onsets: {}", onsets.len());

    // Verify onset count (allow ±1 for boundary effects)
    assert!(
        onsets.len() >= expected_events - 1 && onsets.len() <= expected_events + 1,
        "Expected ~{} onsets, got {}",
        expected_events,
        onsets.len()
    );

    // Verify onset timing - should be 0.5 seconds apart (1 cycle = 0.5s at 2.0 cps)
    if onsets.len() >= 2 {
        let interval = onsets[1].time - onsets[0].time;
        let expected_interval = 1.0 / expected_cps; // 0.5 seconds
        assert!(
            (interval - expected_interval).abs() < 0.1,
            "Expected interval ~{:.3}s, got {:.3}s",
            expected_interval,
            interval
        );
    }
}

#[test]
fn test_bpm_with_multiple_events_per_cycle() {
    // BPM 120 with "bd*4" = 4 events per cycle
    let code = r#"
        bpm: 120
        out $ s "bd*4"
    "#;

    let duration = 4.0; // 4 seconds
    let expected_cps = 120.0 / 240.0; // 0.5 cps
    let expected_cycles = duration * expected_cps; // 2 cycles
    let events_per_cycle = 4;
    let expected_events = (expected_cycles * events_per_cycle as f64) as usize; // 8 events

    let audio = render_dsl(code, duration);

    // Level 2: Onset detection with minimum 100ms between onsets
    // This filters out internal sample structure
    let onsets = detect_musical_onsets(&audio, 44100.0, 0.02, 100.0);

    println!("BPM 120 with bd*4 test:");
    println!("  Expected CPS: {:.3}", expected_cps);
    println!("  Expected cycles: {:.3}", expected_cycles);
    println!("  Events per cycle: {}", events_per_cycle);
    println!("  Expected total events: {}", expected_events);
    println!("  Detected onsets: {}", onsets.len());
    for (i, onset) in onsets.iter().enumerate() {
        println!("    Onset {}: {:.3}s (amp: {:.3})", i + 1, onset.time, onset.amplitude);
    }

    // Verify onset count (allow ±2 for boundary effects and detection variance)
    assert!(
        onsets.len() >= expected_events - 2 && onsets.len() <= expected_events + 2,
        "Expected ~{} onsets, got {}",
        expected_events,
        onsets.len()
    );

    // Verify average interval between events
    if onsets.len() >= 2 {
        let mut total_interval = 0.0;
        for i in 1..onsets.len().min(5) {
            total_interval += onsets[i].time - onsets[i - 1].time;
        }
        let avg_interval = total_interval / (onsets.len().min(5) - 1) as f64;

        // Expected: 1 cycle = 2 seconds, 4 events per cycle → 0.5 seconds per event
        let expected_interval = 1.0 / (expected_cps * events_per_cycle as f64);

        println!("  Average interval: {:.3}s (expected: {:.3}s)", avg_interval, expected_interval);

        assert!(
            (avg_interval - expected_interval).abs() < 0.1,
            "Expected avg interval ~{:.3}s, got {:.3}s",
            expected_interval,
            avg_interval
        );
    }
}

#[test]
fn test_bpm_120_matches_expected_cycle_duration() {
    // This is the key test for the "240 BPM" issue
    // Verify that BPM 120 actually produces 2-second cycles, not 1-second cycles
    let code = r#"
        bpm: 120
        out $ s "bd ~ ~ ~"
    "#;

    let duration = 8.0; // 8 seconds
    let expected_cps = 0.5; // 120 BPM ÷ 240 = 0.5 cps
    let expected_cycle_duration = 2.0; // 1 / 0.5 cps = 2 seconds per cycle

    let audio = render_dsl(code, duration);
    let onsets = detect_musical_onsets(&audio, 44100.0, 0.02, 100.0);

    println!("BPM 120 cycle duration test:");
    println!("  Expected CPS: {:.3}", expected_cps);
    println!("  Expected cycle duration: {:.3}s", expected_cycle_duration);
    println!("  Detected onsets: {}", onsets.len());
    for (i, onset) in onsets.iter().enumerate() {
        println!("    Onset {}: {:.3}s", i + 1, onset.time);
    }

    // Should have 4 events (1 per cycle, 4 cycles in 8 seconds)
    assert!(
        onsets.len() >= 3 && onsets.len() <= 5,
        "Expected 4 onsets (±1), got {}",
        onsets.len()
    );

    // Measure actual cycle duration from onset intervals
    if onsets.len() >= 2 {
        let measured_cycle_duration = onsets[1].time - onsets[0].time;
        println!("  Measured cycle duration: {:.3}s", measured_cycle_duration);

        // If this is 1.0 seconds, BPM is being doubled (240 BPM instead of 120)
        // If this is 2.0 seconds, BPM is correct
        assert!(
            (measured_cycle_duration - expected_cycle_duration).abs() < 0.1,
            "Expected cycle duration {:.3}s, got {:.3}s. BPM may be doubled!",
            expected_cycle_duration,
            measured_cycle_duration
        );

        // Verify it's NOT running at 240 BPM (which would be 1.0 second cycles)
        assert!(
            (measured_cycle_duration - 1.0).abs() > 0.5,
            "Cycle duration is ~1.0s, suggesting BPM is doubled to 240!"
        );
    }
}
