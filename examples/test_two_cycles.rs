use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== Testing 2 Cycles - Verifying Even Spacing ===\n");

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

    // Find all peaks in the audio
    let mut peaks = Vec::new();
    let mut last_sample = 0.0;
    let mut rising = false;

    for (i, &sample) in audio.data.iter().enumerate() {
        if sample > last_sample && sample > 0.1 {
            rising = true;
        } else if rising && sample < last_sample {
            // Found a peak
            peaks.push(i - 1);
            rising = false;
        }
        last_sample = sample;
    }

    println!("Found {} peaks (expecting 8 for 2 cycles):", peaks.len());

    // Expected positions for 8 beats across 2 seconds
    println!("\nExpected vs Actual peak positions:");
    println!("Beat | Expected Time | Expected Sample | Actual Sample | Actual Time | Difference");
    println!("-----|---------------|-----------------|---------------|-------------|------------");

    for beat in 0..8 {
        let expected_time = beat as f32 * 0.25;
        let expected_sample = (expected_time * sample_rate) as usize;

        if beat < peaks.len() {
            let actual_sample = peaks[beat];
            let actual_time = actual_sample as f32 / sample_rate;
            let diff_ms = ((actual_time - expected_time) * 1000.0).abs();

            println!(
                "{:4} | {:13.3}s | {:15} | {:13} | {:11.3}s | {:8.1}ms",
                beat, expected_time, expected_sample, actual_sample, actual_time, diff_ms
            );

            if diff_ms > 10.0 {
                println!("     ⚠️  Large timing deviation!");
            }
        } else {
            println!(
                "{:4} | {:13.3}s | {:15} | MISSING       | -           | -",
                beat, expected_time, expected_sample
            );
        }
    }

    // Check spacing between peaks
    println!("\nSpacing between consecutive peaks:");
    println!("Interval | Samples | Duration | Expected | Deviation");
    println!("---------|---------|----------|----------|----------");

    let expected_spacing = (0.25 * sample_rate) as usize;

    for i in 1..peaks.len() {
        let spacing = peaks[i] - peaks[i - 1];
        let duration = spacing as f32 / sample_rate;
        let expected_duration = 0.25;
        let deviation = ((duration - expected_duration) * 1000.0).abs();

        println!(
            "Peak {}-{} | {:7} | {:8.3}s | {:8.3}s | {:7.1}ms",
            i - 1,
            i,
            spacing,
            duration,
            expected_duration,
            deviation
        );

        if deviation > 10.0 {
            println!("         ⚠️  Uneven spacing detected!");
        }
    }

    // Check if pattern repeats correctly
    if peaks.len() >= 8 {
        println!("\nCycle consistency check:");
        let cycle1_spacings: Vec<usize> = (1..4).map(|i| peaks[i] - peaks[i - 1]).collect();
        let cycle2_spacings: Vec<usize> = (5..8).map(|i| peaks[i] - peaks[i - 1]).collect();

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

    // Final verdict
    println!("\n=== FINAL VERDICT ===");
    if peaks.len() == 8 {
        let all_even = (1..peaks.len()).all(|i| {
            let spacing = peaks[i] - peaks[i - 1];
            let deviation = ((spacing as f32 / sample_rate - 0.25) * 1000.0).abs();
            deviation < 10.0
        });

        if all_even {
            println!("✓ All 8 beats are present and evenly spaced across 2 cycles");
        } else {
            println!("❌ Beats are present but spacing is uneven");
        }
    } else {
        println!("❌ Expected 8 beats but found {}", peaks.len());
    }
}
