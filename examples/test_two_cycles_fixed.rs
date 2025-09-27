use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== Testing 2 Cycles - Fixed Peak Detection ===\n");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    // Simple 4-beat pattern
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");

    // Render 2 cycles (2 seconds)
    let audio = executor.render(&env, 2.0).expect("Failed to render");

    println!("Total samples generated: {}", audio.data.len());
    println!(
        "Expected: {} (2 seconds at {}Hz)\n",
        (sample_rate * 2.0) as usize,
        sample_rate
    );

    // Find beat onsets by looking for transitions from silence to sound
    let mut beat_onsets = Vec::new();
    let threshold = 0.01; // Low threshold to catch the beginning of the envelope
    let mut was_silent = true;

    for (i, &sample) in audio.data.iter().enumerate() {
        let is_sound = sample.abs() > threshold;

        if was_silent && is_sound {
            // Found onset of a beat
            beat_onsets.push(i);
            was_silent = false;
        } else if !was_silent && sample.abs() < 0.001 {
            // Back to silence
            was_silent = true;
        }
    }

    println!(
        "Found {} beat onsets (expecting 8 for 2 cycles):",
        beat_onsets.len()
    );

    // Expected positions for 8 beats across 2 seconds
    println!("\nExpected vs Actual beat onset positions:");
    println!("Beat | Expected Time | Expected Sample | Actual Sample | Actual Time | Difference");
    println!("-----|---------------|-----------------|---------------|-------------|------------");

    for beat in 0..8 {
        let expected_time = beat as f32 * 0.25;
        let expected_sample = (expected_time * sample_rate) as usize;

        if beat < beat_onsets.len() {
            let actual_sample = beat_onsets[beat];
            let actual_time = actual_sample as f32 / sample_rate;
            let diff_ms = ((actual_time - expected_time) * 1000.0).abs();

            println!(
                "{:4} | {:13.3}s | {:15} | {:13} | {:11.3}s | {:8.1}ms",
                beat, expected_time, expected_sample, actual_sample, actual_time, diff_ms
            );

            if diff_ms > 5.0 {
                println!("     ⚠️  Large timing deviation!");
            }
        } else {
            println!(
                "{:4} | {:13.3}s | {:15} | MISSING       | -           | -",
                beat, expected_time, expected_sample
            );
        }
    }

    // Check spacing between beats
    if beat_onsets.len() >= 2 {
        println!("\nSpacing between consecutive beats:");
        println!("Interval | Samples | Duration | Expected | Deviation");
        println!("---------|---------|----------|----------|----------");

        let expected_spacing = (0.25 * sample_rate) as usize;

        for i in 1..beat_onsets.len() {
            let spacing = beat_onsets[i] - beat_onsets[i - 1];
            let duration = spacing as f32 / sample_rate;
            let expected_duration = 0.25;
            let deviation = ((duration - expected_duration) * 1000.0).abs();

            println!(
                "Beat {}-{} | {:7} | {:8.3}s | {:8.3}s | {:7.1}ms",
                i - 1,
                i,
                spacing,
                duration,
                expected_duration,
                deviation
            );

            if deviation > 5.0 {
                println!("         ⚠️  Uneven spacing detected!");
            }
        }
    }

    // Check if pattern repeats correctly
    if beat_onsets.len() >= 8 {
        println!("\nCycle consistency check:");
        let cycle1_spacings: Vec<usize> = (1..4)
            .map(|i| beat_onsets[i] - beat_onsets[i - 1])
            .collect();
        let cycle2_spacings: Vec<usize> = (5..8)
            .map(|i| beat_onsets[i] - beat_onsets[i - 1])
            .collect();

        println!("Cycle 1 spacings: {:?}", cycle1_spacings);
        println!("Cycle 2 spacings: {:?}", cycle2_spacings);

        let consistent = cycle1_spacings
            .iter()
            .zip(cycle2_spacings.iter())
            .all(|(a, b)| (*a as i32 - *b as i32).abs() < 100);

        if consistent {
            println!("✓ Both cycles have consistent spacing");
        } else {
            println!("❌ Cycles have different spacing patterns!");
        }
    }

    // Check beat durations
    println!("\n=== Beat Duration Analysis ===");
    for (i, &onset) in beat_onsets.iter().enumerate() {
        // Find where this beat's audio ends
        let mut end_sample = onset;
        for j in onset..audio.data.len() {
            if audio.data[j].abs() > 0.001 {
                end_sample = j;
            } else if end_sample > onset && j > end_sample + 100 {
                // Found the end (100 samples of silence after last sound)
                break;
            }
        }

        let duration_samples = end_sample - onset;
        let duration_secs = duration_samples as f32 / sample_rate;
        let expected_duration = 0.25;

        println!(
            "Beat {}: {} samples ({:.3}s), expected {:.3}s",
            i, duration_samples, duration_secs, expected_duration
        );

        if duration_secs < expected_duration * 0.9 {
            println!("       ⚠️  Beat is truncated!");
        }
    }

    // Final verdict
    println!("\n=== FINAL VERDICT ===");
    if beat_onsets.len() == 8 {
        let all_even = (1..beat_onsets.len()).all(|i| {
            let spacing = beat_onsets[i] - beat_onsets[i - 1];
            let deviation = ((spacing as f32 / sample_rate - 0.25) * 1000.0).abs();
            deviation < 5.0
        });

        if all_even {
            println!("✓ All 8 beats are present and evenly spaced across 2 cycles");
        } else {
            println!("⚠️  Beats are present but spacing is uneven");
        }
    } else {
        println!("❌ Expected 8 beats but found {}", beat_onsets.len());
    }
}
