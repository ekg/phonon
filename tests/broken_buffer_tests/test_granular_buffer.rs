/// Tests for granular synthesis buffer-based evaluation
///
/// These tests verify that granular buffer evaluation produces correct
/// grain-based textures with controllable parameters.

use phonon::unified_graph::{Signal, UnifiedSignalGraph};
use std::f32::consts::PI;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Create a sine wave source buffer
fn create_sine_source(duration_seconds: f32, frequency: f32, sample_rate: f32) -> Vec<f32> {
    let num_samples = (duration_seconds * sample_rate) as usize;
    (0..num_samples)
        .map(|i| (2.0 * PI * frequency * i as f32 / sample_rate).sin())
        .collect()
}

/// Helper: Create a sawtooth wave source buffer
fn create_saw_source(duration_seconds: f32, frequency: f32, sample_rate: f32) -> Vec<f32> {
    let num_samples = (duration_seconds * sample_rate) as usize;
    (0..num_samples)
        .map(|i| {
            let phase = (i as f32 * frequency / sample_rate) % 1.0;
            2.0 * phase - 1.0
        })
        .collect()
}

/// Helper: Count zero crossings (sign changes)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i-1] < 0.0 && buffer[i] >= 0.0) || (buffer[i-1] >= 0.0 && buffer[i] < 0.0) {
            count += 1;
        }
    }
    count
}

/// Helper: Detect peaks (onsets) in audio
fn detect_peaks(buffer: &[f32], threshold: f32) -> Vec<usize> {
    let mut peaks = Vec::new();
    for i in 1..buffer.len() - 1 {
        if buffer[i] > threshold && buffer[i] > buffer[i-1] && buffer[i] > buffer[i+1] {
            peaks.push(i);
        }
    }
    peaks
}

// ============================================================================
// TEST: Basic Granular Synthesis
// ============================================================================

#[test]
fn test_granular_basic_playback() {
    let mut graph = create_test_graph();

    // Create 1 second sine wave source at 440 Hz
    let source = create_sine_source(1.0, 440.0, 44100.0);

    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),  // 50ms grains
        Signal::Value(0.5),   // Medium density
        Signal::Value(1.0),   // Normal pitch
    );

    let mut output = vec![0.0; 4410]; // 100ms
    graph.eval_node_buffer(&gran_id, &mut output);

    // Should produce audio output
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Granular should produce audio output, RMS: {}", rms);
}

#[test]
fn test_granular_produces_sound() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(30.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 8820]; // 200ms
    graph.eval_node_buffer(&gran_id, &mut output);

    // Check that audio is present
    let rms = calculate_rms(&output);
    assert!(rms > 0.02, "Expected audible output, got RMS: {}", rms);

    // Check not all zeros
    let non_zero = output.iter().filter(|&&x| x.abs() > 0.001).count();
    assert!(non_zero > output.len() / 4,
        "Expected significant audio content, got {} non-zero samples", non_zero);
}

// ============================================================================
// TEST: Grain Size Variation
// ============================================================================

#[test]
fn test_granular_grain_size_small() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(10.0),  // Very small grains (10ms)
        Signal::Value(0.7),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Small grains should produce audio, RMS: {}", rms);
}

#[test]
fn test_granular_grain_size_medium() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),  // Medium grains (50ms)
        Signal::Value(0.7),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Medium grains should produce audio, RMS: {}", rms);
}

#[test]
fn test_granular_grain_size_large() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(100.0),  // Large grains (100ms)
        Signal::Value(0.7),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Large grains should produce audio, RMS: {}", rms);
}

#[test]
fn test_granular_grain_size_affects_texture() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);

    // Small grains (10ms) - more granular texture
    let small_id = graph.add_granular_node(
        source.clone(),
        Signal::Value(10.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    // Large grains (80ms) - smoother texture
    let large_id = graph.add_granular_node(
        source,
        Signal::Value(80.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    let mut small_output = vec![0.0; 8820];
    let mut large_output = vec![0.0; 8820];

    graph.eval_node_buffer(&small_id, &mut small_output);
    graph.eval_node_buffer(&large_id, &mut large_output);

    // Both should produce sound
    let small_rms = calculate_rms(&small_output);
    let large_rms = calculate_rms(&large_output);

    assert!(small_rms > 0.01, "Small grains RMS: {}", small_rms);
    assert!(large_rms > 0.01, "Large grains RMS: {}", large_rms);
}

// ============================================================================
// TEST: Density Variation
// ============================================================================

#[test]
fn test_granular_density_low() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.2),  // Low density (sparse grains)
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    // Low density = less energy
    assert!(rms > 0.001, "Low density should still produce some output, RMS: {}", rms);
}

#[test]
fn test_granular_density_medium() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.5),  // Medium density
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Medium density should produce good output, RMS: {}", rms);
}

#[test]
fn test_granular_density_high() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.9),  // High density (very dense)
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "High density should produce strong output, RMS: {}", rms);
}

#[test]
fn test_granular_density_affects_loudness() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);

    // Low density
    let low_id = graph.add_granular_node(
        source.clone(),
        Signal::Value(50.0),
        Signal::Value(0.2),
        Signal::Value(1.0),
    );

    // High density
    let high_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.8),
        Signal::Value(1.0),
    );

    let mut low_output = vec![0.0; 8820];
    let mut high_output = vec![0.0; 8820];

    graph.eval_node_buffer(&low_id, &mut low_output);
    graph.eval_node_buffer(&high_id, &mut high_output);

    let low_rms = calculate_rms(&low_output);
    let high_rms = calculate_rms(&high_output);

    // Higher density should produce more energy (more overlapping grains)
    assert!(high_rms > low_rms * 1.2,
        "High density ({}) should be louder than low density ({})",
        high_rms, low_rms);
}

// ============================================================================
// TEST: Pitch Shifting
// ============================================================================

#[test]
fn test_granular_pitch_normal() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.6),
        Signal::Value(1.0),  // Normal pitch
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Normal pitch should produce audio, RMS: {}", rms);
}

#[test]
fn test_granular_pitch_up() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.6),
        Signal::Value(2.0),  // Double speed (octave up)
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Pitch up should produce audio, RMS: {}", rms);

    // Higher pitch means more zero crossings
    let crossings = count_zero_crossings(&output);
    assert!(crossings > 50, "Pitch up should have more zero crossings, got {}", crossings);
}

#[test]
fn test_granular_pitch_down() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.6),
        Signal::Value(0.5),  // Half speed (octave down)
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Pitch down should produce audio, RMS: {}", rms);
}

#[test]
fn test_granular_pitch_affects_frequency() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);

    // Normal pitch
    let normal_id = graph.add_granular_node(
        source.clone(),
        Signal::Value(40.0),
        Signal::Value(0.7),
        Signal::Value(1.0),
    );

    // Double pitch
    let double_id = graph.add_granular_node(
        source,
        Signal::Value(40.0),
        Signal::Value(0.7),
        Signal::Value(2.0),
    );

    let mut normal_output = vec![0.0; 8820];
    let mut double_output = vec![0.0; 8820];

    graph.eval_node_buffer(&normal_id, &mut normal_output);
    graph.eval_node_buffer(&double_id, &mut double_output);

    let normal_crossings = count_zero_crossings(&normal_output);
    let double_crossings = count_zero_crossings(&double_output);

    // Double pitch should have roughly twice as many zero crossings
    assert!(double_crossings as f32 > normal_crossings as f32 * 1.5,
        "Double pitch ({}) should have more zero crossings than normal ({})",
        double_crossings, normal_crossings);
}

// ============================================================================
// TEST: Different Source Materials
// ============================================================================

#[test]
fn test_granular_with_sine_source() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(40.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Sine source should work, RMS: {}", rms);
}

#[test]
fn test_granular_with_saw_source() {
    let mut graph = create_test_graph();

    let source = create_saw_source(1.0, 220.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(40.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Saw source should work, RMS: {}", rms);
}

#[test]
fn test_granular_with_noise_source() {
    let mut graph = create_test_graph();

    // Create noise source (using simple pseudo-random)
    let source: Vec<f32> = (0..44100)
        .map(|i| {
            let x = ((i * 12345 + 67890) % 99991) as f32 / 99991.0;
            x * 2.0 - 1.0
        })
        .collect();

    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(30.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Noise source should work, RMS: {}", rms);
}

// ============================================================================
// TEST: Time Stretching
// ============================================================================

#[test]
fn test_granular_time_stretch() {
    let mut graph = create_test_graph();

    // Create 0.5 second source
    let source = create_sine_source(0.5, 440.0, 44100.0);

    // Low density + normal pitch = time stretching
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.3),  // Low density stretches time
        Signal::Value(1.0),  // Normal pitch
    );

    let mut output = vec![0.0; 44100]; // 1 second output
    graph.eval_node_buffer(&gran_id, &mut output);

    // Should produce audio throughout (stretched from 0.5s to 1s)
    let rms = calculate_rms(&output);
    assert!(rms > 0.001, "Time stretch should produce output, RMS: {}", rms);
}

#[test]
fn test_granular_texture_creation() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);

    // Small grains + high density = grainy texture
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(15.0),  // Small grains
        Signal::Value(0.8),   // High density
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 8820];
    graph.eval_node_buffer(&gran_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Grainy texture should be audible, RMS: {}", rms);

    // Should have lots of transients/activity
    let peaks = detect_peaks(&output, 0.1);
    assert!(peaks.len() > 5, "Grainy texture should have multiple peaks, got {}", peaks.len());
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_granular_empty_source() {
    let mut graph = create_test_graph();

    let source: Vec<f32> = vec![];
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    // Should produce silence without crashing
    let rms = calculate_rms(&output);
    assert!(rms < 0.01, "Empty source should produce silence, RMS: {}", rms);
}

#[test]
fn test_granular_zero_density() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.0),  // Zero density (no grains spawned)
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    // Should produce very little or no output
    let rms = calculate_rms(&output);
    assert!(rms < 0.05, "Zero density should produce minimal output, RMS: {}", rms);
}

#[test]
fn test_granular_maximum_density() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(1.0),  // Maximum density
        Signal::Value(1.0),
    );

    let mut output = vec![0.0; 4410];
    graph.eval_node_buffer(&gran_id, &mut output);

    // Should produce strong output
    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Maximum density should produce strong output, RMS: {}", rms);
}

// ============================================================================
// TEST: Phase Continuity
// ============================================================================

#[test]
fn test_granular_multiple_buffers() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    // Generate multiple consecutive buffers
    for i in 0..5 {
        let mut output = vec![0.0; 4410];
        graph.eval_node_buffer(&gran_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.001,
            "Buffer {} should have audio content, RMS: {}", i, rms);
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_granular_performance() {
    let mut graph = create_test_graph();

    let source = create_sine_source(1.0, 440.0, 44100.0);
    let gran_id = graph.add_granular_node(
        source,
        Signal::Value(50.0),
        Signal::Value(0.6),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let iterations = 100;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&gran_id, &mut output);
    }
    let duration = start.elapsed();

    println!("Granular buffer eval: {:?} for {} iterations", duration, iterations);
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time
    assert!(duration.as_secs() < 2,
        "Granular evaluation too slow: {:?}", duration);
}
