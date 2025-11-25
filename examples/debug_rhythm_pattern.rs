use std::cell::RefCell;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

fn main() {
    println!("Debugging rhythm pattern '1 1 0 1'");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Create rhythm pattern (on/off)
    let rhythm_pattern = parse_mini_notation("1 1 0 1");
    let rhythm_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 1 0 1".to_string(),
        pattern: rhythm_pattern,
        last_value: 1.0,
        last_trigger_time: -1.0,
    });

    // Create constant tone
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Gate the oscillator with the pattern
    let gated = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Node(rhythm_node),
    });

    // Scale volume
    let output = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(gated),
        b: Signal::Value(0.3),
    });

    graph.set_output(output);

    // Render 0.5 seconds (1 full cycle at 2 cps)
    let samples_per_cycle = 22050;
    let buffer = graph.render(samples_per_cycle);

    // Check each quarter of the cycle
    let step_samples = samples_per_cycle / 4;

    for i in 0..4 {
        let start = i * step_samples;
        let end = start + step_samples;
        let step_buffer = &buffer[start..end];

        let rms = calculate_rms(step_buffer);
        let expected = if i == 2 { "OFF (0)" } else { "ON (1)" };

        println!("Step {}: RMS = {:.4} - Expected: {}", i, rms, expected);

        if i == 2 && rms > 0.01 {
            println!("  ERROR: Step 2 should be silent but has RMS {}", rms);
        }
    }

    // Save for inspection
    save_wav("debug_rhythm.wav", &buffer, 44100);
    println!("\nSaved to debug_rhythm.wav");
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

fn save_wav(filename: &str, samples: &[f32], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for &sample in samples {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(sample_i16).unwrap();
    }
    writer.finalize().unwrap();
}
