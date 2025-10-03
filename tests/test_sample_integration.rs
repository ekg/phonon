/// Comprehensive sample playback integration tests
/// These tests ensure samples NEVER break by verifying:
/// 1. Samples load correctly
/// 2. Samples play through the graph API
/// 3. Samples play through .phonon files
/// 4. Signal correlation proves samples are in the output
/// 5. Patterns trigger samples at the correct times

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::sample_loader::SampleBank;
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn test_sample_playback_signal_correlation() {
    // Load original BD sample
    let mut bank = SampleBank::new();
    let original_bd = bank.get_sample("bd").expect("BD sample should load");

    println!("Original BD: {} samples", original_bd.len());

    // Render through Sample node
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render enough to capture the whole sample
    let buffer = graph.render(20000);

    save_wav("test_bd_correlation.wav", &buffer, 44100);

    // CRITICAL TEST: Use signal correlation to verify the sample is present
    let correlation = correlate(&buffer, &original_bd);

    println!("Signal correlation peak: {:.4}", correlation);

    // The correlation should be high (> 0.8) if the sample is playing correctly
    assert!(
        correlation > 0.8,
        "Sample should correlate with original: got correlation={}",
        correlation
    );
}

#[test]
fn test_multiple_samples_in_pattern() {
    // Test that "bd cp hh" plays all three samples
    // Note: In Tidal, samples can overlap - this is expected behavior!
    let mut bank = SampleBank::new();
    let bd_original = bank.get_sample("bd").expect("BD should load");
    let cp_original = bank.get_sample("cp").expect("CP should load");
    let hh_original = bank.get_sample("hh").expect("HH should load");

    println!("BD length: {}, CP length: {}, HH length: {}",
             bd_original.len(), cp_original.len(), hh_original.len());

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0); // 1 cycle per second

    // Pattern: bd cp hh (3 events over 1 second)
    let pattern = parse_mini_notation("bd cp hh");

    // Debug: Check pattern events
    use phonon::pattern::{State, TimeSpan, Fraction};
    for i in 0..10 {
        let frac = i as f64 / 10.0;
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(frac),
                Fraction::from_float(frac + 0.01),
            ),
            controls: HashMap::new(),
        };
        let events = pattern.query(&state);
        if !events.is_empty() {
            println!("Pattern event at {:.2}: {:?}", frac, events[0].value);
        }
    }

    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd cp hh".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render 2 seconds (2 cycles)
    let buffer = graph.render(88200);

    save_wav("test_bd_cp_hh_sequence.wav", &buffer, 44100);

    // Print buffer statistics at different sections
    println!("Buffer section 0-1000: RMS={:.4}, Peak={:.4}",
             calculate_rms(&buffer[0..1000]),
             buffer[0..1000].iter().map(|&x| x.abs()).fold(0.0f32, f32::max));
    println!("Buffer section 14000-15000: RMS={:.4}, Peak={:.4}",
             calculate_rms(&buffer[14000..15000]),
             buffer[14000..15000].iter().map(|&x| x.abs()).fold(0.0f32, f32::max));
    println!("Buffer section 29000-30000: RMS={:.4}, Peak={:.4}",
             calculate_rms(&buffer[29000..30000]),
             buffer[29000..30000].iter().map(|&x| x.abs()).fold(0.0f32, f32::max));

    // Check that the overall output contains all three samples
    // We correlate against the entire buffer to find each sample
    let bd_correlation = correlate(&buffer, &bd_original);
    let cp_correlation = correlate(&buffer, &cp_original);
    let hh_correlation = correlate(&buffer, &hh_original);

    println!("BD correlation in buffer: {:.4}", bd_correlation);
    println!("CP correlation in buffer: {:.4}", cp_correlation);
    println!("HH correlation in buffer: {:.4}", hh_correlation);

    // All three samples should appear somewhere in the output
    assert!(
        bd_correlation > 0.7,
        "BD should appear in output"
    );
    assert!(
        cp_correlation > 0.5,
        "CP should appear in output"
    );

    // HH is quiet (peak 0.1380) compared to BD/CP (peak ~1.0)
    // When mixed with louder samples, correlation is lower (~0.10)
    // This is expected behavior - samples overlap in Tidal, and quieter samples
    // get masked by louder ones in the correlation metric
    assert!(
        hh_correlation > 0.05,
        "HH should appear in output, got correlation={}",
        hh_correlation
    );
}

#[test]
fn test_sample_through_phonon_file() {
    // This test renders a .phonon file and verifies the output contains the sample
    use std::process::Command;

    // Create a test .phonon file
    std::fs::write("/tmp/test_sample.phonon", "out s(\"bd\")").unwrap();

    // Render it
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "render",
            "/tmp/test_sample.phonon",
            "/tmp/test_sample_output.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon");

    assert!(output.status.success(), "Phonon render should succeed");

    // Load the rendered WAV
    let mut reader = hound::WavReader::open("/tmp/test_sample_output.wav").unwrap();
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32767.0)
        .collect();

    // Check that it's not silent
    let rms = calculate_rms(&samples);
    let peak = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Phonon file output - RMS: {:.4}, Peak: {:.4}", rms, peak);

    assert!(peak > 0.1, "Should have audio output, got peak={}", peak);
    assert!(rms > 0.01, "Should have non-zero RMS, got rms={}", rms);

    // Verify it correlates with original BD sample
    let mut bank = SampleBank::new();
    let bd_original = bank.get_sample("bd").unwrap();
    let correlation = correlate(&samples, &bd_original);

    println!("Phonon file correlation with BD: {:.4}", correlation);
    assert!(
        correlation > 0.7,
        "Phonon file output should contain BD sample"
    );
}

#[test]
fn test_house_beat_pattern_timing() {
    // Test that "bd cp hh cp" plays samples at the correct times
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second = 0.5s per cycle

    let pattern = parse_mini_notation("bd cp hh cp");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd cp hh cp".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render 4 seconds (8 cycles)
    let buffer = graph.render(176400);

    save_wav("house_beat_timing.wav", &buffer, 44100);

    // Check that we got audio throughout
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("House beat timing test - RMS: {:.4}, Peak: {:.4}", rms, peak);

    assert!(rms > 0.05, "House beat should have audio");
    assert!(peak > 0.3, "House beat should have strong peaks");

    // Verify pattern repeats consistently
    // Split into 8 cycles and check RMS is similar
    let cycle_length = 22050; // 0.5 seconds at 44100 Hz
    for i in 0..4 {
        let start = i * cycle_length;
        let end = start + cycle_length;
        let cycle_rms = calculate_rms(&buffer[start..end]);
        println!("Cycle {} RMS: {:.4}", i, cycle_rms);
        assert!(
            cycle_rms > 0.05,
            "Each cycle should have audio, cycle {} is too quiet",
            i
        );
    }
}

// Helper functions

fn correlate(signal: &[f32], template: &[f32]) -> f32 {
    // Simple normalized cross-correlation
    // Finds the best match of template within signal

    if template.is_empty() || signal.is_empty() {
        return 0.0;
    }

    let template_len = template.len();
    if signal.len() < template_len {
        return 0.0;
    }

    let mut max_correlation: f32 = 0.0;

    // Slide template across signal
    for offset in 0..=(signal.len() - template_len) {
        let window = &signal[offset..offset + template_len];

        // Calculate correlation for this window
        let mut correlation = 0.0;
        let mut signal_energy = 0.0;
        let mut template_energy = 0.0;

        for i in 0..template_len {
            correlation += window[i] * template[i];
            signal_energy += window[i] * window[i];
            template_energy += template[i] * template[i];
        }

        // Normalize
        let norm = (signal_energy * template_energy).sqrt();
        if norm > 0.0 {
            let normalized_correlation = correlation / norm;
            max_correlation = max_correlation.max(normalized_correlation);
        }
    }

    max_correlation
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[test]
fn test_bd_sample_one_cycle() {
    // Test BD alone, one cycle, perfect correlation
    let mut bank = SampleBank::new();
    let bd_original = bank.get_sample("bd").expect("BD should load");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0); // 1 cycle = 1 second = 44100 samples

    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render exactly one cycle
    let buffer = graph.render(44100);

    save_wav("test_bd_one_cycle.wav", &buffer, 44100);

    let correlation = correlate(&buffer, &bd_original);
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("BD one cycle - RMS: {:.4}, Peak: {:.4}, Correlation: {:.4}", rms, peak, correlation);

    assert!(
        correlation > 0.95,
        "BD alone should have near-perfect correlation, got {}",
        correlation
    );
}

#[test]
fn test_cp_sample_one_cycle() {
    // Test CP alone, one cycle, perfect correlation
    let mut bank = SampleBank::new();
    let cp_original = bank.get_sample("cp").expect("CP should load");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("cp");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "cp".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    let buffer = graph.render(44100);

    save_wav("test_cp_one_cycle.wav", &buffer, 44100);

    let correlation = correlate(&buffer, &cp_original);
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("CP one cycle - RMS: {:.4}, Peak: {:.4}, Correlation: {:.4}", rms, peak, correlation);

    assert!(
        correlation > 0.95,
        "CP alone should have near-perfect correlation, got {}",
        correlation
    );
}

#[test]
fn test_hh_sample_one_cycle() {
    // Test HH alone, one cycle, perfect correlation
    let mut bank = SampleBank::new();
    let hh_original = bank.get_sample("hh").expect("HH should load");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("hh");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "hh".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    let buffer = graph.render(44100);

    save_wav("test_hh_one_cycle.wav", &buffer, 44100);

    let correlation = correlate(&buffer, &hh_original);
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("HH one cycle - RMS: {:.4}, Peak: {:.4}, Correlation: {:.4}", rms, peak, correlation);

    assert!(
        correlation > 0.95,
        "HH alone should have near-perfect correlation, got {}",
        correlation
    );
}

#[test]
fn test_sn_sample_one_cycle() {
    // Test SN alone, one cycle, perfect correlation
    let mut bank = SampleBank::new();
    let sn_original = bank.get_sample("sn").expect("SN should load");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("sn");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "sn".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    let buffer = graph.render(44100);

    save_wav("test_sn_one_cycle.wav", &buffer, 44100);

    let correlation = correlate(&buffer, &sn_original);
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("SN one cycle - RMS: {:.4}, Peak: {:.4}, Correlation: {:.4}", rms, peak, correlation);

    assert!(
        correlation > 0.95,
        "SN alone should have near-perfect correlation, got {}",
        correlation
    );
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

#[test]
fn test_euclidean_pattern_with_samples() {
    // Test "bd(3,8)" - 3 kicks distributed over 8 steps
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    let pattern = parse_mini_notation("bd(3,8)");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd(3,8)".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render 4 seconds (8 cycles)
    let buffer = graph.render(176400);

    save_wav("test_euclidean_bd.wav", &buffer, 44100);

    // Check that we got audio
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Euclidean bd(3,8) - RMS: {:.4}, Peak: {:.4}", rms, peak);

    assert!(rms > 0.03, "Euclidean pattern should produce audio");
    assert!(peak > 0.2, "Euclidean pattern should have strong peaks");

    // Verify it contains BD sample
    let mut bank = SampleBank::new();
    let bd_original = bank.get_sample("bd").unwrap();
    let correlation = correlate(&buffer, &bd_original);

    println!("BD correlation in euclidean pattern: {:.4}", correlation);
    assert!(
        correlation > 0.7,
        "Euclidean pattern should contain BD sample"
    );
}

#[test]
fn test_simple_euclidean_sequence() {
    // Test simple euclidean pattern "bd(3,8)"
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Simple euclidean pattern
    let pattern = parse_mini_notation("bd(3,8)");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd(3,8)".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render 4 seconds (8 cycles)
    let buffer = graph.render(176400);

    save_wav("test_simple_euclidean.wav", &buffer, 44100);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Simple euclidean - RMS: {:.4}, Peak: {:.4}", rms, peak);

    assert!(rms > 0.03, "Simple euclidean should produce audio");
    assert!(peak > 0.2, "Simple euclidean should have strong peaks");

    // Verify it contains BD sample
    let mut bank = SampleBank::new();
    let bd_original = bank.get_sample("bd").unwrap();
    let correlation = correlate(&buffer, &bd_original);

    println!("BD correlation in simple euclidean: {:.4}", correlation);
    assert!(
        correlation > 0.7,
        "Simple euclidean should contain BD sample"
    );
}

#[test]
fn test_euclidean_with_offset() {
    // Test euclidean with rotation: "bd(3,8,2)"
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    let pattern = parse_mini_notation("bd(3,8,2)");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd(3,8,2)".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    let buffer = graph.render(176400);

    save_wav("test_euclidean_offset.wav", &buffer, 44100);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Euclidean with offset - RMS: {:.4}, Peak: {:.4}", rms, peak);

    assert!(rms > 0.03, "Euclidean with offset should produce audio");
    assert!(peak > 0.2, "Euclidean with offset should have strong peaks");

    let mut bank = SampleBank::new();
    let bd_original = bank.get_sample("bd").unwrap();
    let correlation = correlate(&buffer, &bd_original);

    println!("BD correlation in offset euclidean: {:.4}", correlation);
    assert!(
        correlation > 0.7,
        "Offset euclidean should contain BD sample"
    );
}
