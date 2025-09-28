use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, Waveform};
use phonon::mini_notation_v3::parse_mini_notation;
use std::fs::File;
use std::io::Write;

#[test]
fn test_pattern_drives_oscillator_frequency() {
    // Test that a pattern can drive oscillator frequency changes
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second for faster testing

    // Create pattern: 220, 330, 440, 330 Hz
    let pattern = parse_mini_notation("220 330 440 330");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "220 330 440 330".to_string(),
        pattern,
        last_value: 220.0,
    });

    // Use pattern to control oscillator frequency
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    graph.set_output(osc);

    // Render 0.5 seconds (1 full cycle at 2 cps)
    let samples_per_cycle = 22050; // 0.5 seconds at 44100 Hz
    let buffer = graph.render(samples_per_cycle);

    // Save audio for manual inspection
    save_wav("test_pattern_freq.wav", &buffer, 44100);

    // Analyze frequency at different points in the cycle
    // Each pattern step should last 0.125 seconds (1/8 of a cycle)
    let step_samples = samples_per_cycle / 4;

    // Check first step (should be 220 Hz)
    let freq1 = estimate_frequency(&buffer[0..step_samples], 44100);
    assert!((freq1 - 220.0).abs() < 10.0, "First step should be 220 Hz, got {}", freq1);

    // Check second step (should be 330 Hz)
    let freq2 = estimate_frequency(&buffer[step_samples..step_samples*2], 44100);
    assert!((freq2 - 330.0).abs() < 15.0, "Second step should be 330 Hz, got {}", freq2);

    // Check third step (should be 440 Hz)
    let freq3 = estimate_frequency(&buffer[step_samples*2..step_samples*3], 44100);
    assert!((freq3 - 440.0).abs() < 20.0, "Third step should be 440 Hz, got {}", freq3);

    // Check fourth step (should be 330 Hz again)
    let freq4 = estimate_frequency(&buffer[step_samples*3..], 44100);
    assert!((freq4 - 330.0).abs() < 15.0, "Fourth step should be 330 Hz, got {}", freq4);

    println!("Pattern-driven frequency changes detected:");
    println!("  Step 1: {:.1} Hz (expected 220)", freq1);
    println!("  Step 2: {:.1} Hz (expected 330)", freq2);
    println!("  Step 3: {:.1} Hz (expected 440)", freq3);
    println!("  Step 4: {:.1} Hz (expected 330)", freq4);
}

#[test]
fn test_pattern_drives_filter_cutoff() {
    // Test that a pattern can modulate filter cutoff
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create noise source
    let noise = graph.add_node(SignalNode::Noise { seed: 42 });

    // Create pattern for filter cutoff: 500, 1000, 2000, 1000 Hz
    let pattern = parse_mini_notation("500 1000 2000 1000");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "500 1000 2000 1000".to_string(),
        pattern,
        last_value: 500.0,
    });

    // Apply pattern-controlled filter
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(noise),
        cutoff: Signal::Node(pattern_node),
        q: Signal::Value(5.0),
        state: Default::default(),
    });

    graph.set_output(filtered);

    // Render and analyze
    let buffer = graph.render(22050); // 0.5 seconds
    save_wav("test_pattern_filter.wav", &buffer, 44100);

    // Check that spectral content changes with the pattern
    let step_samples = 22050 / 4;

    let centroid1 = spectral_centroid(&buffer[0..step_samples], 44100);
    let centroid2 = spectral_centroid(&buffer[step_samples..step_samples*2], 44100);
    let centroid3 = spectral_centroid(&buffer[step_samples*2..step_samples*3], 44100);
    let centroid4 = spectral_centroid(&buffer[step_samples*3..], 44100);

    println!("Filter cutoff pattern modulation (spectral centroids):");
    println!("  Step 1 (500 Hz cutoff): {:.1} Hz", centroid1);
    println!("  Step 2 (1000 Hz cutoff): {:.1} Hz", centroid2);
    println!("  Step 3 (2000 Hz cutoff): {:.1} Hz", centroid3);
    println!("  Step 4 (1000 Hz cutoff): {:.1} Hz", centroid4);

    // Verify centroids follow the pattern (higher cutoff = higher centroid)
    assert!(centroid2 > centroid1, "1000 Hz cutoff should have higher centroid than 500 Hz");
    assert!(centroid3 > centroid2, "2000 Hz cutoff should have higher centroid than 1000 Hz");
    assert!(centroid3 > centroid4, "2000 Hz cutoff should have higher centroid than 1000 Hz (step 4)");
}

#[test]
fn test_pattern_timing_synchronization() {
    // Test that patterns stay synchronized with the cycle timing
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0); // 1 cycle per second

    // Create a simple on/off pattern
    let pattern = parse_mini_notation("1 0 1 0");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 1 0".to_string(),
        pattern,
        last_value: 1.0,
    });

    // Create constant tone
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    // Gate the oscillator with the pattern
    let gated = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Node(pattern_node),
    });

    graph.set_output(gated);

    // Render 2 seconds (2 full cycles)
    let buffer = graph.render(88200);
    save_wav("test_pattern_timing.wav", &buffer, 44100);

    // Check that the pattern repeats exactly after 1 second
    let first_cycle = &buffer[0..44100];
    let second_cycle = &buffer[44100..88200];

    // Calculate RMS for each quarter of each cycle
    for i in 0..4 {
        let start = i * 11025;
        let end = start + 11025;

        let rms1 = calculate_rms(&first_cycle[start..end]);
        let rms2 = calculate_rms(&second_cycle[start..end]);

        // Pattern should repeat: on, off, on, off
        let expected_on = i % 2 == 0;

        if expected_on {
            assert!(rms1 > 0.1, "Cycle 1 step {} should be ON", i);
            assert!(rms2 > 0.1, "Cycle 2 step {} should be ON", i);
        } else {
            assert!(rms1 < 0.01, "Cycle 1 step {} should be OFF", i);
            assert!(rms2 < 0.01, "Cycle 2 step {} should be OFF", i);
        }

        println!("Step {}: Cycle 1 RMS={:.3}, Cycle 2 RMS={:.3} ({})",
                 i, rms1, rms2, if expected_on { "ON" } else { "OFF" });
    }
}

#[test]
fn test_complex_pattern_synthesis() {
    // Test a more complex pattern-driven synthesis setup
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Melody pattern
    let melody_pattern = parse_mini_notation("440 550 660 550");
    let melody_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "440 550 660 550".to_string(),
        pattern: melody_pattern,
        last_value: 440.0,
    });

    // Rhythm pattern (on/off)
    let rhythm_pattern = parse_mini_notation("1 1 0 1");
    let rhythm_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 1 0 1".to_string(),
        pattern: rhythm_pattern,
        last_value: 1.0,
    });

    // Create melodic oscillator
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(melody_node),
        waveform: Waveform::Square,
        phase: 0.0,
    });

    // Apply rhythm gating
    let gated = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Node(rhythm_node),
    });

    // Add some filtering
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(gated),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(2.0),
        state: Default::default(),
    });

    // Scale volume
    let output = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(filtered),
        b: Signal::Value(0.3),
    });

    graph.set_output(output);

    // Render
    let buffer = graph.render(22050);
    save_wav("test_complex_pattern.wav", &buffer, 44100);

    // Verify the pattern produces audio
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.05, "Complex pattern should produce audible output");

    // Check that silence occurs in the third step (rhythm = 0)
    let step_samples = 22050 / 4;
    let step3_rms = calculate_rms(&buffer[step_samples*2..step_samples*3]);

    // Debug: print all step RMS values
    for i in 0..4 {
        let start = i * step_samples;
        let end = start + step_samples;
        let rms = calculate_rms(&buffer[start..end]);
        println!("  Step {}: RMS = {:.4}", i, rms);
    }

    // Third step might have some residual signal from filtering
    assert!(step3_rms < 0.05, "Third step should be mostly silent (rhythm=0), got RMS={}", step3_rms);

    println!("Complex pattern synthesis verified:");
    println!("  Overall RMS: {:.3}", rms);
    println!("  Step 3 RMS: {:.3} (should be silent)", step3_rms);
}

// Helper functions
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

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

fn estimate_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    // Simple zero-crossing frequency estimation
    let mut crossings = 0;
    let mut last_sign = samples[0] >= 0.0;

    for &sample in &samples[1..] {
        let sign = sample >= 0.0;
        if sign != last_sign {
            crossings += 1;
            last_sign = sign;
        }
    }

    // Frequency = crossings / 2 / duration
    let duration = samples.len() as f32 / sample_rate as f32;
    (crossings as f32 / 2.0) / duration
}

fn spectral_centroid(samples: &[f32], sample_rate: u32) -> f32 {
    // Simple DFT-based spectral centroid
    let n = samples.len();
    let mut magnitude_sum = 0.0;
    let mut weighted_sum = 0.0;

    // Only check up to Nyquist frequency
    for k in 0..n/2 {
        let freq = k as f32 * sample_rate as f32 / n as f32;

        // Calculate DFT magnitude at this frequency
        let mut real = 0.0;
        let mut imag = 0.0;

        for (i, &sample) in samples.iter().enumerate() {
            let angle = -2.0 * std::f32::consts::PI * k as f32 * i as f32 / n as f32;
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }

        let magnitude = (real * real + imag * imag).sqrt();
        magnitude_sum += magnitude;
        weighted_sum += magnitude * freq;
    }

    if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    }
}