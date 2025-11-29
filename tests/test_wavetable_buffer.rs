/// Tests for wavetable oscillator buffer-based evaluation
///
/// These tests verify that wavetable buffer evaluation produces correct
/// waveforms, handles interpolation properly, and maintains phase continuity.
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

/// Helper: Count zero crossings (count sign changes)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i - 1] < 0.0 && buffer[i] >= 0.0) || (buffer[i - 1] >= 0.0 && buffer[i] < 0.0) {
            count += 1;
        }
    }
    count
}

/// Helper: Create a sine wavetable
fn create_sine_table(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| (2.0 * PI * i as f32 / size as f32).sin())
        .collect()
}

/// Helper: Create a sawtooth wavetable
fn create_saw_table(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| 2.0 * (i as f32 / size as f32) - 1.0)
        .collect()
}

/// Helper: Create a square wavetable
fn create_square_table(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| if i < size / 2 { 1.0 } else { -1.0 })
        .collect()
}

/// Helper: Create a triangle wavetable
fn create_triangle_table(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| {
            let phase = i as f32 / size as f32;
            if phase < 0.5 {
                4.0 * phase - 1.0
            } else {
                3.0 - 4.0 * phase
            }
        })
        .collect()
}

// ============================================================================
// TEST: Basic Wavetable Playback
// ============================================================================

#[test]
fn test_wavetable_basic_playback() {
    let mut graph = create_test_graph();

    // Create simple sine wavetable
    let table = create_sine_table(512);

    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Sine wave RMS should be ~0.707 (1/sqrt(2))
    let rms = calculate_rms(&output);
    assert!(
        (rms - 0.707).abs() < 0.1,
        "Expected sine RMS ~0.707, got {}",
        rms
    );
}

#[test]
fn test_wavetable_amplitude() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Check amplitude is reasonable (sine wave peak is 1.0)
    let max_amplitude = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    assert!(
        max_amplitude > 0.9 && max_amplitude <= 1.0,
        "Wavetable max amplitude should be ~1.0, got {}",
        max_amplitude
    );
}

#[test]
fn test_wavetable_frequency_accuracy() {
    let mut graph = create_test_graph();
    let sample_rate = 44100.0;
    let frequency = 440.0;

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(frequency), table);

    // Generate enough samples to capture multiple cycles
    let duration_seconds = 0.1; // 100ms
    let buffer_size = (sample_rate * duration_seconds) as usize;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Count zero crossings (each cycle has 2 zero crossings)
    let zero_crossings = count_zero_crossings(&output);
    let cycles = zero_crossings as f32 / 2.0;
    let measured_freq = cycles / duration_seconds;

    // Allow 5% tolerance
    let tolerance = frequency * 0.05;
    assert!(
        (measured_freq - frequency).abs() < tolerance,
        "Expected ~{} Hz, measured {} Hz",
        frequency,
        measured_freq
    );
}

// ============================================================================
// TEST: Different Wavetable Sizes
// ============================================================================

#[test]
fn test_wavetable_size_256() {
    let mut graph = create_test_graph();

    let table = create_sine_table(256);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        (rms - 0.707).abs() < 0.15,
        "256-sample table RMS should be ~0.707, got {}",
        rms
    );
}

#[test]
fn test_wavetable_size_512() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        (rms - 0.707).abs() < 0.1,
        "512-sample table RMS should be ~0.707, got {}",
        rms
    );
}

#[test]
fn test_wavetable_size_1024() {
    let mut graph = create_test_graph();

    let table = create_sine_table(1024);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        (rms - 0.707).abs() < 0.1,
        "1024-sample table RMS should be ~0.707, got {}",
        rms
    );
}

#[test]
fn test_wavetable_size_2048() {
    let mut graph = create_test_graph();

    let table = create_sine_table(2048);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        (rms - 0.707).abs() < 0.1,
        "2048-sample table RMS should be ~0.707, got {}",
        rms
    );
}

// ============================================================================
// TEST: Different Waveforms
// ============================================================================

#[test]
fn test_wavetable_saw_wave() {
    let mut graph = create_test_graph();

    let table = create_saw_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Saw wave ranges from -1 to 1
    let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    assert!(
        max_val > 0.8,
        "Saw max should be close to 1.0, got {}",
        max_val
    );
    assert!(
        min_val < -0.8,
        "Saw min should be close to -1.0, got {}",
        min_val
    );
}

#[test]
fn test_wavetable_square_wave() {
    let mut graph = create_test_graph();

    let table = create_square_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Square wave should be mostly 1.0 or -1.0
    let near_one = output.iter().filter(|&&x| (x - 1.0).abs() < 0.1).count();
    let near_neg_one = output.iter().filter(|&&x| (x + 1.0).abs() < 0.1).count();
    let total_near_extremes = near_one + near_neg_one;

    // Most samples should be at extremes
    assert!(
        total_near_extremes > output.len() * 80 / 100,
        "Square wave should mostly be at ±1.0"
    );
}

#[test]
fn test_wavetable_triangle_wave() {
    let mut graph = create_test_graph();

    let table = create_triangle_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Triangle wave ranges from -1 to 1
    let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    assert!(max_val > 0.8, "Triangle max should be close to 1.0");
    assert!(min_val < -0.8, "Triangle min should be close to -1.0");
}

// ============================================================================
// TEST: Phase Continuity
// ============================================================================

#[test]
fn test_wavetable_phase_continuity() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&wt_id, &mut buffer1);
    graph.eval_node_buffer(&wt_id, &mut buffer2);

    // Check that phase is continuous across buffers
    // by verifying zero crossings are consistent
    let crossings1 = count_zero_crossings(&buffer1);
    let crossings2 = count_zero_crossings(&buffer2);

    // Both buffers should have similar number of zero crossings
    assert!(
        (crossings1 as i32 - crossings2 as i32).abs() <= 2,
        "Phase should be continuous, crossings differ too much: {} vs {}",
        crossings1,
        crossings2
    );
}

#[test]
fn test_wavetable_multiple_buffers() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&wt_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(
            rms > 0.5 && rms < 0.9,
            "Buffer {} has unexpected RMS: {}",
            i,
            rms
        );
    }
}

// ============================================================================
// TEST: Frequency Sweep
// ============================================================================

#[test]
fn test_wavetable_low_frequency() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(55.0), table); // A1

    let mut output = vec![0.0; 4410]; // 100ms at 44.1kHz
    graph.eval_node_buffer(&wt_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.5,
        "Low frequency should still produce good output, RMS: {}",
        rms
    );
}

#[test]
fn test_wavetable_high_frequency() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(4400.0), table); // High pitch

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.5,
        "High frequency should produce output, RMS: {}",
        rms
    );
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_wavetable_zero_frequency() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(0.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Zero frequency should produce constant value (DC at table[0])
    let first_val = output[0];
    for &sample in &output {
        assert!(
            (sample - first_val).abs() < 0.01,
            "Zero frequency should produce constant output"
        );
    }
}

#[test]
fn test_wavetable_empty_table() {
    let mut graph = create_test_graph();

    let table: Vec<f32> = vec![]; // Empty table
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Should produce silence without crashing
    let rms = calculate_rms(&output);
    assert!(rms < 0.01, "Empty table should produce silence");
}

#[test]
fn test_wavetable_single_sample_table() {
    let mut graph = create_test_graph();

    let table = vec![0.5]; // Single constant value
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Should produce constant DC value
    for &sample in &output {
        assert!(
            (sample - 0.5).abs() < 0.01,
            "Single-sample table should produce constant 0.5"
        );
    }
}

// ============================================================================
// TEST: Interpolation Quality
// ============================================================================

#[test]
fn test_wavetable_interpolation() {
    let mut graph = create_test_graph();

    // Use a coarse table to make interpolation effects visible
    let table = create_sine_table(32); // Only 32 samples
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&wt_id, &mut output);

    // Even with coarse table, interpolation should smooth it out
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.5,
        "Interpolation should smooth coarse table, RMS: {}",
        rms
    );
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_wavetable_performance() {
    let mut graph = create_test_graph();

    let table = create_sine_table(512);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&wt_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "Wavetable buffer eval: {:?} for {} iterations",
        duration, iterations
    );
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 1 second for 1000 iterations)
    assert!(
        duration.as_secs() < 1,
        "Wavetable buffer evaluation too slow: {:?}",
        duration
    );
}

#[test]
fn test_wavetable_large_table_performance() {
    let mut graph = create_test_graph();

    // Very large table (4096 samples = high quality)
    let table = create_sine_table(4096);
    let wt_id = graph.add_wavetable_node(Signal::Value(440.0), table);

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&wt_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "Large wavetable (4096) buffer eval: {:?} for {} iterations",
        duration, iterations
    );

    // Should still be fast even with large table
    assert!(
        duration.as_secs() < 2,
        "Large wavetable too slow: {:?}",
        duration
    );
}
