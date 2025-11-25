//! Tests that verify audio is actually modulating over time
//! Writes audio to buffers and performs spectral/temporal analysis

use std::cell::RefCell;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::fs::File;
use std::io::Write;

/// Write samples to a WAV file for manual inspection
fn write_wav(filename: &str, samples: &[f32], sample_rate: f32) -> std::io::Result<()> {
    let mut file = File::create(filename)?;

    // WAV header
    let num_samples = samples.len() as u32;
    let byte_rate = (sample_rate as u32) * 2; // 16-bit mono

    file.write_all(b"RIFF")?;
    file.write_all(&(36 + num_samples * 2).to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // fmt chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample
    file.write_all(b"data")?;
    file.write_all(&(num_samples * 2).to_le_bytes())?;

    // Convert to 16-bit and write
    for &sample in samples {
        let s16 = (sample.max(-1.0).min(1.0) * 32767.0) as i16;
        file.write_all(&s16.to_le_bytes())?;
    }

    Ok(())
}

/// Analyze frequency content in a window of samples
fn analyze_spectrum_window(samples: &[f32]) -> f32 {
    // Simple spectral brightness measure using first derivative
    let mut brightness = 0.0;
    for i in 1..samples.len() {
        let diff = samples[i] - samples[i - 1];
        brightness += diff.abs();
    }
    brightness / samples.len() as f32
}

/// Detect if a signal is modulating by checking if windows have different characteristics
fn detect_modulation(samples: &[f32], window_size: usize) -> bool {
    if samples.len() < window_size * 4 {
        return false;
    }

    let mut window_brightnesses = Vec::new();
    let num_windows = samples.len() / window_size;

    for i in 0..num_windows {
        let start = i * window_size;
        let end = (i + 1) * window_size;
        let brightness = analyze_spectrum_window(&samples[start..end]);
        window_brightnesses.push(brightness);

        println!("Window {}: brightness = {:.4}", i, brightness);
    }

    // Check if there's significant variation
    let mean: f32 = window_brightnesses.iter().sum::<f32>() / window_brightnesses.len() as f32;
    let variance: f32 = window_brightnesses
        .iter()
        .map(|b| (b - mean).powi(2))
        .sum::<f32>()
        / window_brightnesses.len() as f32;

    let std_dev = variance.sqrt();
    let coefficient_of_variation = std_dev / mean;

    println!(
        "Mean brightness: {:.4}, Std Dev: {:.4}, CV: {:.4}",
        mean, std_dev, coefficient_of_variation
    );

    // If coefficient of variation > 0.1, we have modulation
    coefficient_of_variation > 0.1
}

#[test]
fn test_filter_pattern_actually_modulates() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Set tempo to 2 cycles per second for faster testing
    graph.set_cps(2.0);

    // Create oscillator with pattern-modulated filter
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Pattern alternates between very low and very high cutoff
    let pattern = parse_mini_notation("100 5000");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "100 5000".to_string(),
        pattern,
        last_value: 100.0,
        last_trigger_time: -1.0,
    });

    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc),
        cutoff: Signal::Node(pattern_node),
        q: Signal::Value(5.0),
        state: Default::default(),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(filtered),
    });

    graph.set_output(output);

    // Generate 2 seconds of audio (4 complete cycles at 2 cps)
    let num_samples = (sample_rate * 2.0) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let sample = graph.process_sample();
        samples.push(sample);

        // Debug: print pattern changes
        if i % 1000 == 0 {
            let cycle_pos = (i as f32 / sample_rate) * 2.0; // 2 cps
            println!("Sample {}: cycle position = {:.3}", i, cycle_pos);
        }
    }

    // Write to WAV for inspection
    write_wav("/tmp/filter_pattern_test.wav", &samples, sample_rate).expect("Failed to write WAV");

    println!("\nWrote test audio to /tmp/filter_pattern_test.wav");

    // Analyze: we should see alternating bright/dark sections
    // At 2 cps with pattern "100 5000", we get 2 values per cycle
    // So we should see changes every 0.25 seconds = 11025 samples
    let window_size = 5512; // Quarter of the change period for better resolution

    assert!(
        detect_modulation(&samples, window_size),
        "Filter pattern should create modulation in the audio output"
    );
}

#[test]
fn test_oscillator_frequency_pattern_modulates() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Oscillator with pattern-controlled frequency
    let pattern = parse_mini_notation("110 220 440");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "110 220 440".to_string(),
        pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Value(0.5),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);

    // Generate 1.5 seconds (3 complete cycles, should hear all 3 frequencies)
    let num_samples = (sample_rate * 1.5) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    // Track zero crossings to detect frequency changes
    let mut zero_crossings_per_window = Vec::new();
    let window_size = (sample_rate / 8.0) as usize; // 1/8 second windows
    let mut current_window = Vec::new();

    for _i in 0..num_samples {
        let sample = graph.process_sample();
        samples.push(sample);
        current_window.push(sample);

        if current_window.len() >= window_size {
            // Count zero crossings in this window
            let mut crossings = 0;
            for j in 1..current_window.len() {
                if (current_window[j - 1] <= 0.0 && current_window[j] > 0.0)
                    || (current_window[j - 1] > 0.0 && current_window[j] <= 0.0)
                {
                    crossings += 1;
                }
            }

            // Approximate frequency from zero crossings
            let freq = (crossings as f32 / 2.0) / (window_size as f32 / sample_rate);
            zero_crossings_per_window.push(freq);

            println!(
                "Window {} freq estimate: {:.1} Hz",
                zero_crossings_per_window.len(),
                freq
            );

            current_window.clear();
        }
    }

    write_wav("/tmp/freq_pattern_test.wav", &samples, sample_rate).expect("Failed to write WAV");

    println!("\nWrote test audio to /tmp/freq_pattern_test.wav");

    // We should see approximately 110Hz, 220Hz, and 440Hz in different windows
    let mut found_110 = false;
    let mut found_220 = false;
    let mut found_440 = false;

    for &freq in &zero_crossings_per_window {
        if (freq - 110.0).abs() < 20.0 {
            found_110 = true;
        }
        if (freq - 220.0).abs() < 40.0 {
            found_220 = true;
        }
        if (freq - 440.0).abs() < 80.0 {
            found_440 = true;
        }
    }

    assert!(
        found_110 && found_220 && found_440,
        "Should find all three frequencies (110, 220, 440 Hz) in the pattern. \
             Found 110: {}, 220: {}, 440: {}",
        found_110,
        found_220,
        found_440
    );
}

#[test]
fn test_pattern_timing_verification() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Test at exactly 1 cycle per second for easy verification
    graph.set_cps(1.0);

    // Simple pattern that outputs its own values
    let pattern = parse_mini_notation("1 2 3 4");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 2 3 4".to_string(),
        pattern,
        last_value: 1.0,
        last_trigger_time: -1.0,
    });

    // Just output the pattern value directly (multiplied for visibility)
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(pattern_node),
        b: Signal::Value(0.1),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);

    // Generate exactly 1 second = 1 cycle
    let num_samples = sample_rate as usize;
    let mut samples = Vec::with_capacity(num_samples);
    let mut value_changes = Vec::new();
    let mut last_value = 0.0;

    for i in 0..num_samples {
        let sample = graph.process_sample();
        samples.push(sample);

        // Detect value changes
        if (sample - last_value).abs() > 0.01 {
            value_changes.push((i, last_value, sample));
            println!(
                "Value change at sample {}: {:.2} -> {:.2}",
                i, last_value, sample
            );
            last_value = sample;
        }
    }

    // We should see 4 value changes in 1 second (at 1 cps)
    // Values should be 0.1, 0.2, 0.3, 0.4 (pattern values * 0.1)
    assert!(
        value_changes.len() >= 3,
        "Should see at least 3 value changes in pattern '1 2 3 4', got {}",
        value_changes.len()
    );

    // Check timing: changes should be roughly every 0.25 seconds (11025 samples)
    if value_changes.len() >= 2 {
        let expected_interval = sample_rate / 4.0;
        for i in 1..value_changes.len() {
            let interval = value_changes[i].0 - value_changes[i - 1].0;
            let error = ((interval as f32 - expected_interval) / expected_interval).abs();
            assert!(
                error < 0.1,
                "Pattern timing is off. Expected ~{} samples between changes, got {}",
                expected_interval as usize,
                interval
            );
        }
    }
}
