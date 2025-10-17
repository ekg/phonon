use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

#[test]
fn test_sample_node_reproduces_original_sample() {
    // Load original BD sample
    let mut bank = phonon::sample_loader::SampleBank::new();
    let original_bd = bank.get_sample("bd").expect("BD sample should load");

    println!("Original BD sample: {} samples", original_bd.len());

    // Now render it through the Sample node
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    // Single "bd" event
    let pattern = parse_mini_notation("bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render enough to capture the whole sample
    // BD is 12532 samples, so render a bit more
    let buffer = graph.render(15000);

    save_wav("test_bd_sample.wav", &buffer, 44100);

    // Check that we got audio
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Rendered audio:");
    println!("  RMS: {:.4}", rms);
    println!("  Peak: {:.4}", peak);
    println!("  Buffer length: {}", buffer.len());

    // The beginning of the buffer should match the original sample
    let original_rms = calculate_rms(&original_bd[..]);
    let original_peak = original_bd.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Original sample:");
    println!("  RMS: {:.4}", original_rms);
    println!("  Peak: {:.4}", original_peak);

    assert!(peak > 0.01, "Should have produced audio, got peak={}", peak);
    assert!(rms > 0.001, "Should have non-zero RMS, got rms={}", rms);

    // Check similarity: rendered peak should be close to original
    assert!(
        (peak - original_peak).abs() < 0.1,
        "Peak should match original: got {} vs {}",
        peak,
        original_peak
    );
}

#[test]
fn test_pattern_with_three_drums() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Pattern: bd, cp, hh in sequence
    let pattern = parse_mini_notation("bd cp hh");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd cp hh".to_string(),
        pattern,
        last_trigger_time: 0.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output(sample_node);

    // Render 2 seconds (4 cycles)
    let buffer = graph.render(88200);

    save_wav("test_bd_cp_hh.wav", &buffer, 44100);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("Three drums test:");
    println!("  RMS: {:.4}", rms);
    println!("  Peak: {:.4}", peak);

    assert!(peak > 0.01, "Should have audio from samples");
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
