use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::fs::File;
use std::io::Write;

#[test]
fn debug_bus_triggered_synth_with_output() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2.0
~synth: sine 440
~trig: s "~synth"
out: ~trig
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate).expect("Compilation failed");

    // Render 2 seconds in multiple buffers
    let buffer_size = 512;
    let num_buffers = (sample_rate * 2.0) as usize / buffer_size;

    let mut full_audio = Vec::new();
    for i in 0..num_buffers {
        // Add debug logging BEFORE render
        if i <= 2 {
            eprintln!("\n=== Rendering buffer {} (samples {}-{}) ===",
                i, i * buffer_size, (i + 1) * buffer_size - 1);
        }

        std::env::set_var("DEBUG_SAMPLE_EVENTS", "1");
        std::env::set_var("DEBUG_VOICE_PROCESS", "1");

        let buffer = graph.render(buffer_size);

        std::env::remove_var("DEBUG_SAMPLE_EVENTS");
        std::env::remove_var("DEBUG_VOICE_PROCESS");

        // Print discontinuities at boundaries
        if i > 0 && !full_audio.is_empty() {
            let last_sample: f32 = full_audio[full_audio.len() - 1];
            let first_sample: f32 = buffer[0];
            let diff: f32 = (first_sample - last_sample).abs();
            if diff > 0.05 {
                eprintln!("\n!!! DISCONTINUITY at buffer {} boundary: {:.6} -> {:.6} (diff: {:.6}) !!!",
                    i, last_sample, first_sample, diff);
            }
        }

        full_audio.extend_from_slice(&buffer);

        // Only log first 3 buffers to keep output manageable
        if i >= 2 {
            std::env::remove_var("DEBUG_SAMPLE_EVENTS");
            std::env::remove_var("DEBUG_VOICE_PROCESS");
        }
    }

    // Save to file for analysis
    let mut file = File::create("/tmp/debug_continuous_synthesis.txt").unwrap();
    for (i, sample) in full_audio.iter().enumerate() {
        if i < 2000 || (i >= buffer_size - 5 && i < buffer_size + 5) {
            writeln!(file, "{}: {:.6}", i, sample).unwrap();
        }
    }

    // Check discontinuities
    let mut max_discontinuity = 0.0_f32;
    let mut max_location = 0;
    for i in (buffer_size..full_audio.len()).step_by(buffer_size) {
        if i > 0 && i < full_audio.len() {
            let diff = (full_audio[i] - full_audio[i - 1]).abs();
            if diff > max_discontinuity {
                max_discontinuity = diff;
                max_location = i;
            }
        }
    }

    eprintln!("Max discontinuity: {} at sample {}", max_discontinuity, max_location);
    eprintln!("Audio around discontinuity:");
    for i in (max_location.saturating_sub(5))..=(max_location + 5).min(full_audio.len() - 1) {
        eprintln!("  Sample {}: {:.6}", i, full_audio[i]);
    }

    // Don't assert, just report
    eprintln!("RMS: {}", (full_audio.iter().map(|s| s * s).sum::<f32>() / full_audio.len() as f32).sqrt());
}
