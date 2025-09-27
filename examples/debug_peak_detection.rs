use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== Debugging Peak Detection ===\n");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    // The test was finding peaks by looking for local maxima above 0.1
    // Let's do the same and see what we get
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

    println!("Found {} peaks using threshold 0.1", peaks.len());

    // Show the first 10 peaks
    println!("\nFirst 10 peaks:");
    for (idx, &peak_sample) in peaks.iter().take(10).enumerate() {
        let peak_time = peak_sample as f32 / sample_rate;
        let peak_value = audio.data[peak_sample];
        println!(
            "  Peak {}: sample {} ({:.4}s), value = {:.4}",
            idx, peak_sample, peak_time, peak_value
        );
    }

    // That's the issue - we're detecting every cycle of the 1000Hz sine wave!
    // At 1000Hz, we get 1000 peaks per second, so 250 peaks per beat
    // Let's look for the envelope peaks instead

    println!("\n=== Looking for Envelope Peaks ===\n");

    // Calculate a simple envelope by taking the absolute value and smoothing
    let window_size = 100; // ~2ms window
    let mut envelope = vec![0.0; audio.data.len()];

    for i in 0..audio.data.len() {
        let start = i.saturating_sub(window_size / 2);
        let end = (i + window_size / 2).min(audio.data.len());

        let mut sum = 0.0;
        for j in start..end {
            sum += audio.data[j].abs();
        }
        envelope[i] = sum / (end - start) as f32;
    }

    // Now find peaks in the envelope
    let mut env_peaks = Vec::new();
    let mut last_env = 0.0;
    let mut env_rising = false;

    for (i, &env_val) in envelope.iter().enumerate() {
        if env_val > last_env && env_val > 0.05 {
            env_rising = true;
        } else if env_rising && env_val < last_env {
            // Found an envelope peak
            env_peaks.push(i - 1);
            env_rising = false;
        }
        last_env = env_val;
    }

    println!("Found {} envelope peaks", env_peaks.len());

    println!("\nEnvelope peaks (should be 4 for 4 beats):");
    for (idx, &peak_sample) in env_peaks.iter().enumerate() {
        let peak_time = peak_sample as f32 / sample_rate;
        let peak_value = envelope[peak_sample];
        let expected_time = idx as f32 * 0.25;
        let diff_ms = (peak_time - expected_time) * 1000.0;

        println!(
            "  Peak {}: sample {} ({:.4}s), envelope = {:.4}, expected = {:.4}s, diff = {:.1}ms",
            idx, peak_sample, peak_time, peak_value, expected_time, diff_ms
        );
    }
}
