/// Test for Euclidean rhythm bug - ensures consistent event count per cycle
///
/// This test verifies that `s "cp(2,4)"` produces EXACTLY 2 events per cycle,
/// not an alternating 2/3 pattern as reported in the bug.
///
/// The test uses THREE verification levels:
/// 1. Pattern query - verify pattern generates correct events
/// 2. Onset detection - verify audio has correct number of onsets
/// 3. Expected timing - verify onsets occur at correct times (from first principles)

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

/// Generate expected onset times for cp(2,4) from Euclidean algorithm
///
/// Euclidean(2,4) = [x . x .] = positions [0, 2] out of [0,1,2,3]
/// In normalized cycle: [0.0, 0.5]
fn expected_euclidean_times(cycles: usize, tempo: f64) -> Vec<f64> {
    let cycle_duration = 1.0 / tempo;
    let mut times = Vec::new();

    // Euclidean(2,4) pattern: events at positions 0 and 2 out of 4 steps
    // In normalized time: 0.0 and 0.5 of each cycle
    let positions = vec![0.0, 0.5];

    for cycle in 0..cycles {
        for &pos in &positions {
            let time = (cycle as f64 + pos) * cycle_duration;
            times.push(time);
        }
    }

    times
}

#[test]
fn test_euclidean_pattern_query_consistency() {
    // LEVEL 1: Pattern query verification
    let pattern = parse_mini_notation("cp(2,4)");

    println!("Testing cp(2,4) pattern query over 32 cycles:");

    let mut event_counts = Vec::new();
    for cycle in 0..32 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        let non_rest_events: Vec<_> = events.iter()
            .filter(|e| e.value != "~" && !e.value.is_empty())
            .collect();

        event_counts.push(non_rest_events.len());

        // Verify each cycle has exactly 2 events
        assert_eq!(
            non_rest_events.len(),
            2,
            "Cycle {} should have exactly 2 events, got {}",
            cycle,
            non_rest_events.len()
        );
    }

    println!("✓ All 32 cycles have exactly 2 events");

    // Verify no alternating pattern (all should be 2)
    let total: usize = event_counts.iter().sum();
    assert_eq!(total, 64, "Total events should be 64 (2 × 32 cycles)");

    // Check for alternating pattern
    let alternating = event_counts.windows(2).any(|w| w[0] != w[1]);
    assert!(
        !alternating,
        "Event counts should not alternate. Counts: {:?}",
        event_counts
    );
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use std::process::Command;
    use std::fs;

    fn render_dsl(code: &str, cycles: u32, tempo: f64) -> Vec<f32> {
        let test_file = "/tmp/test_euclidean_unit.ph";
        let output_file = "/tmp/test_euclidean_unit.wav";

        // Write DSL code with specified tempo
        let full_code = format!("tempo: {}\n{}", tempo, code);
        fs::write(test_file, full_code).expect("Failed to write test file");

        // Render to WAV
        let status = Command::new("cargo")
            .args(&[
                "run",
                "--release",
                "--bin",
                "phonon",
                "--",
                "render",
                "--cycles",
                &cycles.to_string(),
                test_file,
                output_file,
            ])
            .status()
            .expect("Failed to run phonon render");

        assert!(status.success(), "Rendering failed");

        // Read WAV file
        let mut reader = hound::WavReader::open(output_file).expect("Failed to open WAV");
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();

        samples
    }

    fn detect_onsets(audio: &[f32], sample_rate: f32, threshold: f32) -> Vec<f64> {
        let mut onsets = Vec::new();
        let window_size = (sample_rate * 0.01) as usize; // 10ms window

        let mut prev_energy = 0.0;
        let mut i = 0;

        while i + window_size < audio.len() {
            // Calculate energy in current window
            let energy: f32 = audio[i..i + window_size]
                .iter()
                .map(|&s| s * s)
                .sum::<f32>() / window_size as f32;

            // Detect onset as energy spike
            if energy > threshold && energy > prev_energy * 2.0 {
                let time = i as f64 / sample_rate as f64;
                onsets.push(time);

                // Skip ahead to avoid multiple detections of same onset
                i += window_size * 2;
            } else {
                i += window_size / 4;
            }

            prev_energy = energy;
        }

        onsets
    }

    #[test]
    fn test_euclidean_audio_onset_count() {
        // LEVEL 2: Audio onset detection
        // Use clicks instead of handclaps - cleaner transients for reliable detection
        let cycles = 32;
        let tempo = 0.4;

        let audio = render_dsl("out $ s \"click(2,4)\" # gain 0.8", cycles, tempo);
        let onsets = detect_onsets(&audio, 44100.0, 0.001); // Adjusted threshold for clicks

        println!("Detected {} onsets over {} cycles", onsets.len(), cycles);
        println!("Expected: {} onsets (2 per cycle)", cycles * 2);

        // Allow tolerance for onset detection variability
        // Real-world onset detection is ~90-95% accurate even with clean samples
        let expected = (cycles * 2) as usize;
        let tolerance = (expected as f32 * 0.1) as usize; // 10% tolerance

        assert!(
            onsets.len() >= expected - tolerance,
            "Too few onsets: got {}, expected ~{} (±{})",
            onsets.len(),
            expected,
            tolerance
        );

        assert!(
            onsets.len() <= expected + tolerance,
            "Too many onsets: got {}, expected ~{} (±{})",
            onsets.len(),
            expected,
            tolerance
        );
    }

    #[test]
    fn test_euclidean_timing_from_first_principles() {
        // LEVEL 3: Verify timing matches expected from Euclidean algorithm
        // Use clicks for cleaner onset detection
        let cycles = 8;  // Reduced for easier analysis
        let tempo = 0.4;

        let audio = render_dsl("out $ s \"click(2,4)\" # gain 0.8", cycles as u32, tempo);
        let detected_onsets = detect_onsets(&audio, 44100.0, 0.001); // Adjusted threshold
        let expected_onsets = expected_euclidean_times(cycles, tempo);

        println!("\n=== Euclidean Timing Verification ===");
        println!("Tempo: {} cycles/sec", tempo);
        println!("Cycle duration: {:.3} seconds", 1.0 / tempo);
        println!("Expected onsets: {}", expected_onsets.len());
        println!("Detected onsets: {}", detected_onsets.len());
        println!("\nALL detected onset times:");
        for (i, &time) in detected_onsets.iter().enumerate() {
            let cycle_num = (time * tempo) as usize;
            let pos_in_cycle = (time * tempo) - cycle_num as f64;
            println!("  {:3}: {:.6}s (cycle {}, pos {:.3})", i, time, cycle_num, pos_in_cycle);
        }

        // Check count matches (with tolerance for detection variability)
        let count_tolerance = (expected_onsets.len() as f32 * 0.15) as i32; // 15% tolerance
        assert!(
            (detected_onsets.len() as i32 - expected_onsets.len() as i32).abs() <= count_tolerance,
            "Onset count mismatch: expected {}, got {} (±{})",
            expected_onsets.len(),
            detected_onsets.len(),
            count_tolerance
        );

        // Verify timing of first few onsets
        // Note: Skip the first expected onset if missed (common in onset detection - no prior silence)
        let offset = if detected_onsets.is_empty() || (detected_onsets[0] - expected_onsets[0]).abs() > 0.5 {
            println!("\n⚠️  First onset missed (at t=0, no prior silence for detection)");
            1 // Skip first expected onset
        } else {
            0
        };

        println!("\nFirst 10 onset times:");
        println!("Expected    | Detected  | Δ");
        println!("------------|-----------|--------");

        for i in 0..std::cmp::min(10, std::cmp::min(expected_onsets.len() - offset, detected_onsets.len())) {
            let delta = (detected_onsets[i] - expected_onsets[i + offset]).abs();
            println!(
                "{:10.6} | {:9.6} | {:6.3}",
                expected_onsets[i + offset],
                detected_onsets[i],
                delta
            );

            // Verify timing within 50ms tolerance (realistic for onset detection)
            assert!(
                delta < 0.05,
                "Onset {} timing off by {:.3}s (>50ms)",
                i,
                delta
            );
        }

        // Check for alternating pattern in inter-onset intervals
        if detected_onsets.len() >= 4 {
            let intervals: Vec<f64> = detected_onsets
                .windows(2)
                .map(|w| w[1] - w[0])
                .collect();

            println!("\nInter-onset intervals:");
            for (i, &interval) in intervals.iter().take(10).enumerate() {
                println!("  Interval {}: {:.6}s", i, interval);
            }

            // For click(2,4) with tempo 0.4:
            // Cycle duration = 2.5s
            // Expected intervals: [1.25, 1.25, 1.25, 1.25, ...]
            // (consistent spacing between events)

            let expected_interval = 1.25; // (2.5 cycle / 2 events)

            // Skip if detection found too few onsets
            if intervals.len() >= 4 {
                for (i, &interval) in intervals.iter().take(4).enumerate() {
                    assert!(
                        (interval - expected_interval).abs() < 0.15,
                        "Interval {} is {:.3}s, expected {:.3}s (±150ms)",
                        i,
                        interval,
                        expected_interval
                    );
                }
            }
        }
    }
}
