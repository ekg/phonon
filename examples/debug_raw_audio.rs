use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;
use std::fs::File;
use std::io::Write;

fn main() {
    println!("=== Raw Audio Debug ===\n");

    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;

    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");

    // Write raw audio data to a file for inspection
    let mut file = File::create("debug_audio.txt").unwrap();

    // Look at specific regions where beats should be
    for beat in 0..4 {
        let beat_start = ((beat as f32 * 0.25) * sample_rate) as usize;
        let beat_end = ((beat as f32 * 0.25 + 0.01) * sample_rate) as usize; // First 10ms of each beat

        writeln!(
            file,
            "\n=== Beat {} (samples {}-{}) ===",
            beat, beat_start, beat_end
        )
        .unwrap();

        for i in beat_start..beat_end.min(audio.data.len()) {
            if (i - beat_start) % 10 == 0 {
                writeln!(file, "Sample {}: {:.6}", i, audio.data[i]).unwrap();
            }
        }
    }

    // Find regions with actual audio
    println!("Scanning for audio regions...");
    let mut in_audio = false;
    let mut audio_start = 0;
    let mut audio_regions = Vec::new();

    for (i, &sample) in audio.data.iter().enumerate() {
        if !in_audio && sample.abs() > 0.001 {
            in_audio = true;
            audio_start = i;
        } else if in_audio && sample.abs() < 0.0001 {
            // Check if we've had 100 samples of silence
            let mut is_silent = true;
            for j in i..((i + 100).min(audio.data.len())) {
                if audio.data[j].abs() > 0.0001 {
                    is_silent = false;
                    break;
                }
            }
            if is_silent {
                in_audio = false;
                audio_regions.push((audio_start, i));
            }
        }
    }

    if in_audio {
        audio_regions.push((audio_start, audio.data.len()));
    }

    println!("\nFound {} audio regions:", audio_regions.len());
    for (idx, (start, end)) in audio_regions.iter().enumerate() {
        let duration_samples = end - start;
        let start_time = *start as f32 / sample_rate;
        let end_time = *end as f32 / sample_rate;
        let duration_secs = duration_samples as f32 / sample_rate;

        println!(
            "Region {}: samples {}-{} ({:.4}s - {:.4}s), duration: {} samples ({:.4}s)",
            idx, start, end, start_time, end_time, duration_samples, duration_secs
        );

        // Expected beat positions
        let expected_beat = (start_time / 0.25) as usize;
        let expected_start = (expected_beat as f32 * 0.25 * sample_rate) as usize;

        if (*start as i32 - expected_start as i32).abs() < 100 {
            println!(
                "         ✓ Matches expected beat {} position",
                expected_beat
            );
        } else {
            println!("         ⚠️  Does not match any expected beat position");
        }
    }

    println!("\nExpected beat positions:");
    for beat in 0..4 {
        let expected_start = ((beat as f32 * 0.25) * sample_rate) as usize;
        let expected_end = (((beat + 1) as f32 * 0.25) * sample_rate) as usize;
        println!(
            "Beat {}: samples {}-{} ({:.3}s - {:.3}s)",
            beat,
            expected_start,
            expected_end,
            expected_start as f32 / sample_rate,
            expected_end as f32 / sample_rate
        );
    }

    println!("\nDebug data written to debug_audio.txt");
}
