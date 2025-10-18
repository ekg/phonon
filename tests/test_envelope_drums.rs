use hound::{SampleFormat, WavSpec, WavWriter};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::process::Command;

#[test]
fn test_kick_drum_with_envelope() {
    let test_file = "/tmp/test_kick_envelope.wav";
    render_kick_with_envelope(test_file);

    // Analyze the output
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--quiet", "--", test_file])
        .output()
        .expect("Failed to run wav_analyze");

    let analysis = String::from_utf8_lossy(&output.stdout);

    // Should have audio
    assert!(!analysis.contains("EMPTY AUDIO", "Kick drum is empty!");

    // Should detect 4 distinct beats in 2 seconds
    let onset_count = extract_onset_count(&analysis);
    assert!(
        onset_count >= 3 && onset_count <= 5,
        "Expected ~4 kick beats, got {}\nAnalysis:\n{}",
        onset_count,
        analysis
    );
}

fn render_kick_with_envelope(output_path: &str) {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a trigger pattern: 4 beats in 2 seconds
    // "1" = trigger, "0" = no trigger
    let trigger_pattern = parse_mini_notation("1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0");
    let trigger_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Noise source for kick
    let noise = graph.add_node(SignalNode::Noise { seed: 12345 });

    // Low-pass filter for kick body
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(noise),
        cutoff: Signal::Value(100.0),
        q: Signal::Value(5.0),
        state: Default::default(),
    });

    // Apply envelope to filtered noise
    let enveloped = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(filtered),
        trigger: Signal::Node(trigger_node),
        attack: 0.001, // 1ms attack
        decay: 0.1,    // 100ms decay
        sustain: 0.0,  // No sustain (percussive)
        release: 0.05, // 50ms release
        state: Default::default(),
    });

    // Output with gain
    let output = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(enveloped),
        b: Signal::Value(0.5),
    });

    graph.set_output(output);

    // Render 2 seconds
    let mut samples = Vec::new();
    for _ in 0..88200 {
        samples.push(graph.process_sample());
    }

    write_wav(output_path, &samples, 44100).unwrap();
}

fn extract_onset_count(analysis: &str) -> usize {
    for line in analysis.lines() {
        if line.contains("Onset Events:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(count) = parts[2].parse::<usize>() {
                    return count;
                }
            }
        }
    }
    0
}

fn write_wav(
    path: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    for &sample in samples {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(sample_i16)?;
    }
    writer.finalize()?;
    Ok(())
}
