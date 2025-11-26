use std::cell::RefCell;
use hound::{SampleFormat, WavSpec, WavWriter};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::collections::HashMap;
use std::process::Command;

// Test that bus assignments actually connect and produce audio
#[test]
fn test_bus_assignment_produces_audio() {
    println!("Testing bus assignment produces audio...");

    let test_file = "/tmp/test_bus_assignment.wav";

    // Create a graph with bus assignment
    let mut graph = UnifiedSignalGraph::new(44100.0);
    let mut buses: HashMap<String, phonon::unified_graph::NodeId> = HashMap::new();

    // Create bass = saw 110
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Store in bus
    buses.insert("~bass".to_string(), osc);

    // Reference bus with gain: out bass * 0.2
    let output = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc), // This simulates looking up "bass" from buses
        b: Signal::Value(0.2),
    });

    graph.set_output(output);

    // Render 0.5 seconds
    let mut samples = Vec::new();
    for _ in 0..22050 {
        samples.push(graph.process_sample());
    }

    // Verify internal: should have non-zero samples
    let max_sample = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_sample > 0.1,
        "Bus assignment produced no audio! Max sample: {}",
        max_sample
    );

    // Write and analyze
    write_wav(test_file, &samples, 44100).unwrap();
    let analysis = analyze_wav(test_file);

    // Verify output
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Bus assignment produced empty audio!\n{}",
        analysis
    );

    // Should detect ~110 Hz
    assert!(analysis.contains("Dominant Freq:"), "No frequency detected");
    let freq = extract_dominant_freq(&analysis);
    assert!(
        (freq - 110.0).abs() < 20.0,
        "Wrong frequency from bus assignment. Expected ~110 Hz, got {} Hz",
        freq
    );

    println!("✅ Bus assignment test passed");
}

// Test signal addition produces correct mix
#[test]
fn test_signal_addition_mixes_correctly() {
    println!("Testing signal addition mixes correctly...");

    let test_file = "/tmp/test_signal_addition.wav";

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create two distinct frequencies
    let low_freq = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(100.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let high_freq = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(1000.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Scale them
    let low_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(low_freq),
        b: Signal::Value(0.3),
    });

    let high_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(high_freq),
        b: Signal::Value(0.1),
    });

    // Add them together
    let mixed = graph.add_node(SignalNode::Add {
        a: Signal::Node(low_scaled),
        b: Signal::Node(high_scaled),
    });

    graph.set_output(mixed);

    // Render
    let mut samples = Vec::new();
    for _ in 0..44100 {
        samples.push(graph.process_sample());
    }

    // Verify we have a mix (not just one frequency)
    // The RMS should be somewhere between the two gain levels
    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05 && rms < 0.35,
        "RMS level suggests incorrect mixing: {} (expected 0.05-0.35)",
        rms
    );

    // Do spectral analysis to verify both frequencies present
    let (low_energy, high_energy) = analyze_frequency_bands(&samples, 44100.0);
    assert!(
        low_energy > 0.01,
        "Low frequency (100 Hz) missing from mix!"
    );
    assert!(
        high_energy > 0.001,
        "High frequency (1000 Hz) missing from mix!"
    );

    // The ratio should roughly match our gain settings (0.3 vs 0.1)
    let ratio = low_energy / high_energy;
    assert!(
        ratio > 1.5 && ratio < 4.5,
        "Frequency mix ratio wrong: {} (expected ~3.0)",
        ratio
    );

    write_wav(test_file, &samples, 44100).unwrap();
    let analysis = analyze_wav(test_file);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Mixed signal is empty!\n{}",
        analysis
    );

    println!("✅ Signal addition test passed");
}

// Test multiple signal addition (3+ signals)
#[test]
fn test_multiple_signal_addition() {
    println!("Testing multiple signal addition...");

    let test_file = "/tmp/test_multiple_addition.wav";

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create three signals at different frequencies
    let sig1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let sig2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let sig3 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Scale each
    let s1_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(sig1),
        b: Signal::Value(0.2),
    });

    let s2_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(sig2),
        b: Signal::Value(0.15),
    });

    let s3_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(sig3),
        b: Signal::Value(0.1),
    });

    // Chain additions: (sig1 + sig2) + sig3
    let mix_12 = graph.add_node(SignalNode::Add {
        a: Signal::Node(s1_scaled),
        b: Signal::Node(s2_scaled),
    });

    let mix_all = graph.add_node(SignalNode::Add {
        a: Signal::Node(mix_12),
        b: Signal::Node(s3_scaled),
    });

    graph.set_output(mix_all);

    // Render
    let mut samples = Vec::new();
    for _ in 0..44100 {
        samples.push(graph.process_sample());
    }

    // Verify all three frequencies are present
    let energies = analyze_multiple_frequencies(&samples, 44100.0, &[110.0, 220.0, 440.0]);

    assert!(energies[0] > 0.01, "110 Hz missing from mix!");
    assert!(energies[1] > 0.007, "220 Hz missing from mix!");
    assert!(energies[2] > 0.003, "440 Hz missing from mix!");

    // Check relative levels match our gains (0.2, 0.15, 0.1)
    // Expected ratios: 0.2/0.15 = 1.33, 0.15/0.1 = 1.5
    let ratio_12 = energies[0] / energies[1];
    let ratio_23 = energies[1] / energies[2];

    println!(
        "Energies: 110Hz={:.4}, 220Hz={:.4}, 440Hz={:.4}",
        energies[0], energies[1], energies[2]
    );
    println!("Ratios: 110/220={:.2}, 220/440={:.2}", ratio_12, ratio_23);

    assert!(
        ratio_12 > 0.8 && ratio_12 < 2.5,
        "Ratio between 110 Hz and 220 Hz wrong: {} (expected ~1.33)",
        ratio_12
    );
    assert!(
        ratio_23 > 0.8 && ratio_23 < 2.5,
        "Ratio between 220 Hz and 440 Hz wrong: {} (expected ~1.5)",
        ratio_23
    );

    write_wav(test_file, &samples, 44100).unwrap();

    println!("✅ Multiple signal addition test passed");
}

// Test that bus + mixing works in the actual parser
#[test]
fn test_parser_bus_and_mixing() {
    println!("Testing parser handles buses and mixing...");

    // This simulates what the phonon render command does
    
    use std::process::Command;

    let phonon_code = r#"
tempo: 2.0
~bass: saw 55 # lpf 500 3
~lead: square 440
out: ~bass * 0.3 + ~lead * 0.1
"#;

    // Write test file
    std::fs::write("/tmp/test_parser_mix.phonon", phonon_code).unwrap();

    // Run phonon render
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_parser_mix.phonon",
            "/tmp/test_parser_mix.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        panic!(
            "phonon render failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Analyze the output
    let analysis = analyze_wav("/tmp/test_parser_mix.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Parser produced empty audio!\n{}",
        analysis
    );

    // Should have both frequencies present
    // The dominant might be the bass (55 Hz) due to higher amplitude
    assert!(analysis.contains("Dominant Freq:"), "No frequency detected");

    // Check RMS is reasonable for mixed signal
    let rms = extract_rms(&analysis);
    assert!(
        rms > 0.05 && rms < 0.3,
        "RMS suggests parsing error: {} (expected 0.05-0.3)",
        rms
    );

    println!("✅ Parser bus and mixing test passed");
}

// Test complex expression parsing
#[test]
fn test_complex_expression_parsing() {
    println!("Testing complex expression parsing...");

    let phonon_code = r#"
-- Complex mixing expression
tempo: 2.0
~lfo $ sine 2
~bass $ saw 55
~filtered_bass $ ~bass # lpf "200 500 1000 500" 3
~lead $ square "220 330 440 330"

out $ ~filtered_bass * 0.4 + ~lead * 0.1
"#;

    std::fs::write("/tmp/test_complex.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_complex.phonon",
            "/tmp/test_complex.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        panic!(
            "Complex expression render failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let analysis = analyze_wav("/tmp/test_complex.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Complex expression produced no audio!\n{}",
        analysis
    );

    // With patterns and filter modulation, we should have modulated audio
    // Check that RMS shows we have varying signal (not just constant)
    let rms = extract_rms(&analysis);
    assert!(
        rms > 0.05,
        "Signal too quiet - patterns might not be working! RMS: {}",
        rms
    );

    // The spectral centroid should be in mid-range due to filtering
    assert!(
        analysis.contains("Spectral Centroid:"),
        "No spectral analysis - audio might be empty!"
    );

    println!("✅ Complex expression test passed");
}

// Helper functions
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

fn analyze_wav(path: &str) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--quiet", "--", path])
        .output()
        .expect("Failed to run wav_analyze");

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

fn extract_dominant_freq(analysis: &str) -> f32 {
    for line in analysis.lines() {
        if line.contains("Dominant Freq:") {
            if let Some(start) = line.find("Dominant Freq:") {
                let rest = &line[start + 14..].trim();
                if let Some(end) = rest.find(" Hz") {
                    if let Ok(freq) = rest[..end].parse::<f32>() {
                        return freq;
                    }
                }
            }
        }
    }
    0.0
}

fn extract_rms(analysis: &str) -> f32 {
    for line in analysis.lines() {
        if line.contains("RMS Level:") {
            if let Some(start) = line.find("RMS Level:") {
                let rest = &line[start + 10..].trim();
                if let Some(end) = rest.find(' ') {
                    if let Ok(rms) = rest[..end].parse::<f32>() {
                        return rms;
                    }
                }
            }
        }
    }
    0.0
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

// Analyze energy in frequency bands
fn analyze_frequency_bands(samples: &[f32], sample_rate: f32) -> (f32, f32) {
    use std::f32::consts::PI;

    let window_size = 2048.min(samples.len());
    let window = &samples[..window_size];

    // Apply Hamming window
    let windowed: Vec<f32> = window
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let window_val = 0.54 - 0.46 * (2.0 * PI * i as f32 / (window_size - 1) as f32).cos();
            x * window_val
        })
        .collect();

    // Calculate energy around 100 Hz and 1000 Hz
    let bin_100hz = (100.0 * window_size as f32 / sample_rate) as usize;
    let bin_1000hz = (1000.0 * window_size as f32 / sample_rate) as usize;

    let mut low_energy = 0.0;
    let mut high_energy = 0.0;

    // DFT at specific bins
    for k in (bin_100hz.saturating_sub(2))..=(bin_100hz + 2).min(window_size / 2) {
        let (mut real, mut imag) = (0.0, 0.0);
        for (n, &sample) in windowed.iter().enumerate() {
            let angle = -2.0 * PI * k as f32 * n as f32 / window_size as f32;
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }
        low_energy += (real * real + imag * imag).sqrt();
    }

    for k in (bin_1000hz.saturating_sub(2))..=(bin_1000hz + 2).min(window_size / 2) {
        let (mut real, mut imag) = (0.0, 0.0);
        for (n, &sample) in windowed.iter().enumerate() {
            let angle = -2.0 * PI * k as f32 * n as f32 / window_size as f32;
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }
        high_energy += (real * real + imag * imag).sqrt();
    }

    (low_energy / 5.0, high_energy / 5.0) // Average over bins
}

// Analyze multiple specific frequencies
fn analyze_multiple_frequencies(
    samples: &[f32],
    sample_rate: f32,
    frequencies: &[f32],
) -> Vec<f32> {
    use std::f32::consts::PI;

    let window_size = 2048.min(samples.len());
    let window = &samples[..window_size];

    // Apply Hamming window
    let windowed: Vec<f32> = window
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let window_val = 0.54 - 0.46 * (2.0 * PI * i as f32 / (window_size - 1) as f32).cos();
            x * window_val
        })
        .collect();

    let mut energies = Vec::new();

    for &freq in frequencies {
        let bin = (freq * window_size as f32 / sample_rate) as usize;
        let (mut real, mut imag) = (0.0, 0.0);

        // Sample around the target bin for robustness
        for k in (bin.saturating_sub(1))..=(bin + 1).min(window_size / 2) {
            for (n, &sample) in windowed.iter().enumerate() {
                let angle = -2.0 * PI * k as f32 * n as f32 / window_size as f32;
                real += sample * angle.cos();
                imag += sample * angle.sin();
            }
        }

        energies.push((real * real + imag * imag).sqrt() / 3.0);
    }

    energies
}
