//! Test that pattern-modulated filters actually affect the audio output

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::collections::HashMap;

/// Compute the spectral centroid of a signal to detect filter changes
fn compute_spectral_centroid(samples: &[f32], _sample_rate: f32) -> f32 {
    // Instead of zero-crossing, measure high-frequency energy
    // by looking at the difference between consecutive samples
    if samples.len() < 2 {
        return 0.0;
    }

    let mut high_freq_energy = 0.0;
    let mut low_freq_energy = 0.0;

    // Compute first derivative (high frequency indicator)
    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        high_freq_energy += diff * diff;
    }

    // Compute signal energy (overall)
    for sample in samples {
        low_freq_energy += sample * sample;
    }

    // Normalize
    high_freq_energy = (high_freq_energy / samples.len() as f32).sqrt();
    low_freq_energy = (low_freq_energy / samples.len() as f32).sqrt();

    // Return ratio indicating relative high frequency content
    // Higher values = more high frequency content
    if low_freq_energy > 0.0001 {
        (high_freq_energy / low_freq_energy) * 1000.0
    } else {
        0.0
    }
}

/// Test that a pattern-modulated filter actually changes the audio output
#[test]
fn test_pattern_modulated_filter_changes_audio() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0); // Fast tempo for testing

    // Create a saw wave
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        phase: 0.0,
    });

    // Create a pattern for filter cutoff: low -> high -> low
    let pattern = parse_mini_notation("200 5000 200");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "200 5000 200".to_string(),
        pattern,
        last_value: 200.0,
        last_trigger_time: -1.0,
    });

    // Apply filter with pattern-modulated cutoff
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc),
        cutoff: Signal::Node(pattern_node),
        q: Signal::Value(2.0),
        state: Default::default(),
    });

    // Scale and output
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(filtered),
        b: Signal::Value(0.5),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);

    // Collect samples for analysis
    // At 4 cps, one cycle = 0.25 seconds = 11025 samples
    let samples_per_cycle = (sample_rate / 4.0) as usize;
    let samples_per_segment = samples_per_cycle / 3; // Pattern has 3 values

    let mut segment_centroids = Vec::new();

    // Process 3 segments (one full pattern cycle)
    for segment in 0..3 {
        let mut segment_samples = Vec::new();

        for _ in 0..samples_per_segment {
            segment_samples.push(graph.process_sample());
        }

        // Compute spectral centroid for this segment
        let centroid = compute_spectral_centroid(&segment_samples, sample_rate);
        segment_centroids.push(centroid);

        // Also compute RMS to see if signal is present
        let rms: f32 = (segment_samples.iter().map(|x| x * x).sum::<f32>()
            / segment_samples.len() as f32)
            .sqrt();

        println!(
            "Segment {} centroid: {:.0} Hz, RMS: {:.4}",
            segment, centroid, rms
        );
    }

    // Verify the pattern: low -> high -> low
    // Using the new metric where higher values mean more high frequency content

    println!(
        "Centroids: [{:.1}, {:.1}, {:.1}]",
        segment_centroids[0], segment_centroids[1], segment_centroids[2]
    );

    // Segment 1 (5000 Hz cutoff) should have significantly more HF than segments 0 and 2 (200 Hz)
    let avg_low = (segment_centroids[0] + segment_centroids[2]) / 2.0;
    let high = segment_centroids[1];

    assert!(
        high > avg_low * 1.5,
        "High cutoff (5000Hz) should have at least 1.5x more HF content than low cutoff (200Hz). \
             Low avg: {:.1}, High: {:.1}, Ratio: {:.2}",
        avg_low,
        high,
        high / avg_low
    );

    // Also verify segments 0 and 2 are similar (both 200 Hz cutoff)
    let similarity = (segment_centroids[0] - segment_centroids[2]).abs()
        / segment_centroids[0].max(segment_centroids[2]);
    assert!(
        similarity < 0.3,
        "Segments 0 and 2 (both 200Hz) should be similar, got {:.1} and {:.1} (diff: {:.0}%)",
        segment_centroids[0],
        segment_centroids[2],
        similarity * 100.0
    );
}

/// Test that static filters maintain consistent spectral content
#[test]
fn test_static_filter_consistent_output() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Create a saw wave with static filter
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        phase: 0.0,
    });

    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc),
        cutoff: Signal::Value(1000.0), // Static cutoff
        q: Signal::Value(2.0),
        state: Default::default(),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(filtered),
        b: Signal::Value(0.5),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);

    // Collect samples from multiple time windows
    let window_size = 1024;
    let mut window_centroids = Vec::new();

    for window in 0..5 {
        let mut window_samples = Vec::new();

        for _ in 0..window_size {
            window_samples.push(graph.process_sample());
        }

        let centroid = compute_spectral_centroid(&window_samples, sample_rate);
        window_centroids.push(centroid);

        println!("Window {} centroid: {:.0} Hz", window, centroid);
    }

    // All windows should have similar spectral content (within 20%)
    let mean_centroid: f32 = window_centroids.iter().sum::<f32>() / window_centroids.len() as f32;

    for (i, &centroid) in window_centroids.iter().enumerate() {
        let deviation = (centroid - mean_centroid).abs() / mean_centroid;
        assert!(
            deviation < 0.2,
            "Window {} deviates too much from mean: {:.0}Hz vs {:.0}Hz (deviation: {:.1}%)",
            i,
            centroid,
            mean_centroid,
            deviation * 100.0
        );
    }
}

/// Test extreme filter modulation (from sub-bass to ultrasonic)
#[test]
fn test_extreme_filter_modulation() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // White noise source (contains all frequencies)
    let noise = graph.add_node(SignalNode::Noise { seed: 12345 });

    // Extreme filter pattern: 50 Hz to 15000 Hz
    let pattern = parse_mini_notation("50 15000");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "50 15000".to_string(),
        pattern,
        last_value: 50.0,
        last_trigger_time: -1.0,
    });

    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(noise),
        cutoff: Signal::Node(pattern_node),
        q: Signal::Value(5.0), // High resonance
        state: Default::default(),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(filtered),
        b: Signal::Value(0.3),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);

    // Collect samples from low and high cutoff periods
    let samples_per_half_cycle = (sample_rate / 4.0) as usize; // 2 cps, 2 values = 4 changes/sec

    // Low cutoff period
    let mut low_samples = Vec::new();
    for _ in 0..samples_per_half_cycle {
        low_samples.push(graph.process_sample());
    }

    // High cutoff period
    let mut high_samples = Vec::new();
    for _ in 0..samples_per_half_cycle {
        high_samples.push(graph.process_sample());
    }

    // Compute RMS energy (filtered noise should have less energy at low cutoff)
    let low_rms: f32 =
        (low_samples.iter().map(|x| x * x).sum::<f32>() / low_samples.len() as f32).sqrt();
    let high_rms: f32 =
        (high_samples.iter().map(|x| x * x).sum::<f32>() / high_samples.len() as f32).sqrt();

    // High cutoff should pass more energy from white noise
    assert!(high_rms > low_rms * 1.5,
            "High cutoff (15kHz) should pass significantly more white noise energy than low cutoff (50Hz). \
             Low RMS: {:.4}, High RMS: {:.4}", low_rms, high_rms);

    // Also check spectral content
    let low_centroid = compute_spectral_centroid(&low_samples, sample_rate);
    let high_centroid = compute_spectral_centroid(&high_samples, sample_rate);

    println!(
        "Low cutoff centroid: {:.0} Hz, High cutoff centroid: {:.0} Hz",
        low_centroid, high_centroid
    );

    assert!(
        high_centroid > low_centroid * 3.0,
        "High cutoff spectrum should be much brighter than low cutoff"
    );
}
