use std::cell::RefCell;
use hound::{SampleFormat, WavSpec, WavWriter};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::process::Command;

#[test]
fn test_simple_sine_generates_audio() {
    let test_file = "/tmp/test_sine.wav";
    render_simple_sine(test_file);
    assert_wav_has_audio(test_file, 440.0);
}

#[test]
fn test_pattern_modulation_works() {
    let test_file = "/tmp/test_pattern.wav";
    render_pattern_modulation(test_file);
    assert_wav_has_audio(test_file, 0.0); // Don't check specific frequency for patterns
}

#[test]
fn test_filter_modulation_works() {
    let test_file = "/tmp/test_filter.wav";
    render_filter_modulation(test_file);
    assert_wav_has_audio(test_file, 110.0);
}

fn render_simple_sine(output_path: &str) {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create sine 440 * 0.2
    let freq_signal = Signal::Value(440.0);
    let osc_node = graph.add_node(SignalNode::Oscillator {
        freq: freq_signal,
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let gain_node = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc_node),
        b: Signal::Value(0.2),
    });

    graph.set_output(gain_node);

    // Render 1 second
    let mut samples = Vec::new();
    for _ in 0..44100 {
        samples.push(graph.process_sample());
    }

    write_wav(output_path, &samples, 44100).unwrap();
}

fn render_pattern_modulation(output_path: &str) {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create sine "220 440 330" * 0.2
    let pattern = parse_mini_notation("220 440 330");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "220 440 330".to_string(),
        pattern,
        last_value: 440.0,
        last_trigger_time: -1.0,
    });

    let osc_node = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let gain_node = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc_node),
        b: Signal::Value(0.2),
    });

    graph.set_output(gain_node);

    // Render 2 seconds to hear pattern
    let mut samples = Vec::new();
    for _ in 0..88200 {
        samples.push(graph.process_sample());
    }

    write_wav(output_path, &samples, 44100).unwrap();
}

fn render_filter_modulation(output_path: &str) {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create saw 110 >> lpf("500 2000", 3) * 0.2
    let freq_signal = Signal::Value(110.0);
    let osc_node = graph.add_node(SignalNode::Oscillator {
        freq: freq_signal,
        waveform: Waveform::Saw,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let pattern = parse_mini_notation("500 2000");
    let cutoff_pattern = graph.add_node(SignalNode::Pattern {
        pattern_str: "500 2000".to_string(),
        pattern,
        last_value: 1000.0,
        last_trigger_time: -1.0,
    });

    let filter_node = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc_node),
        cutoff: Signal::Node(cutoff_pattern),
        q: Signal::Value(3.0),
        state: Default::default(),
    });

    let gain_node = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(filter_node),
        b: Signal::Value(0.2),
    });

    graph.set_output(gain_node);

    // Render 2 seconds
    let mut samples = Vec::new();
    for _ in 0..88200 {
        samples.push(graph.process_sample());
    }

    write_wav(output_path, &samples, 44100).unwrap();
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

fn assert_wav_has_audio(path: &str, expected_freq: f32) {
    // Run wav_analyze binary
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--quiet", "--", path])
        .output()
        .expect("Failed to run wav_analyze");

    let analysis = String::from_utf8_lossy(&output.stdout);

    // Check that it contains audio (not empty)
    assert!(
        !analysis.contains("EMPTY AUDIO"),
        "Generated WAV file {} is empty!\nAnalysis:\n{}",
        path,
        analysis
    );

    // Check RMS is reasonable (not too quiet)
    let has_reasonable_level = analysis
        .lines()
        .find(|line| line.contains("RMS Level:"))
        .map(|line| {
            // Extract RMS value
            if let Some(start) = line.find("RMS Level:") {
                let rest = &line[start + 10..].trim();
                if let Some(end) = rest.find(' ') {
                    if let Ok(rms) = rest[..end].parse::<f32>() {
                        return rms > 0.01; // At least -40 dB
                    }
                }
            }
            false
        })
        .unwrap_or(false);

    assert!(
        has_reasonable_level,
        "Generated audio level too low in {}\nAnalysis:\n{}",
        path, analysis
    );

    // If expected frequency is specified, check it's close
    if expected_freq > 0.0 {
        let has_correct_freq = analysis
            .lines()
            .find(|line| line.contains("Dominant Freq:"))
            .map(|line| {
                if let Some(start) = line.find("Dominant Freq:") {
                    let rest = &line[start + 14..].trim();
                    if let Some(end) = rest.find(" Hz") {
                        if let Ok(freq) = rest[..end].parse::<f32>() {
                            // Allow 10% deviation
                            return (freq - expected_freq).abs() / expected_freq < 0.1;
                        }
                    }
                }
                false
            })
            .unwrap_or(false);

        if expected_freq > 50.0 {
            // Only check for non-bass frequencies
            assert!(
                has_correct_freq,
                "Frequency mismatch in {}. Expected ~{} Hz\nAnalysis:\n{}",
                path, expected_freq, analysis
            );
        }
    }
}
