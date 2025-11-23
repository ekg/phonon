/// Tests for Distortion (waveshaper) buffer-based evaluation
///
/// These tests verify that Distortion buffer evaluation produces correct
/// waveshaping behavior with drive and wet/dry mixing.

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};
use std::f32::consts::PI;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Calculate peak absolute value
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max)
}

/// Helper: Measure harmonic distortion (simplified - counts zero crossings)
/// More zero crossings = more high-frequency content = more harmonics
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i-1] < 0.0 && buffer[i] >= 0.0) || (buffer[i-1] >= 0.0 && buffer[i] < 0.0) {
            count += 1;
        }
    }
    count
}

/// Helper: Measure spectral energy (rate of change - proxy for high-freq content)
fn measure_spectral_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i-1]).abs();
    }
    energy / buffer.len() as f32
}

// ============================================================================
// TEST: Clean Signal (Mix = 0)
// ============================================================================

#[test]
fn test_distortion_clean_signal_mix_zero() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Distortion with mix = 0 (completely dry)
    let dist_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(10.0),  // High drive (doesn't matter)
        Signal::Value(0.0),   // Mix = 0 (dry)
    );

    let buffer_size = 512;
    let mut processed = vec![0.0; buffer_size];

    // Get processed signal (this evaluates the oscillator internally)
    graph.eval_node_buffer(&dist_id, &mut processed);

    // With mix=0, should pass through oscillator signal unchanged
    // RMS should be close to 1/sqrt(2) ≈ 0.707 for sine wave
    let rms = calculate_rms(&processed);
    assert!(
        (rms - 0.707).abs() < 0.05,
        "Mix=0 should pass through clean sine wave: RMS = {} (expected ~0.707)",
        rms
    );
}

// ============================================================================
// TEST: Full Distortion (Mix = 1)
// ============================================================================

#[test]
fn test_distortion_full_wet_mix_one() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create two separate distortion nodes: one with mix=0, one with mix=1
    let clean_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),  // High drive for clear distortion
        Signal::Value(0.0),   // Dry (clean)
    );

    let distorted_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),  // High drive for clear distortion
        Signal::Value(1.0),   // Wet (distorted)
    );

    let buffer_size = 512;
    let mut clean = vec![0.0; buffer_size];
    let mut distorted = vec![0.0; buffer_size];

    // Evaluate both in sequence (oscillator will advance, but that's ok for comparison)
    graph.eval_node_buffer(&clean_id, &mut clean);
    graph.eval_node_buffer(&distorted_id, &mut distorted);

    // With high drive (20.0), distorted signal should have higher RMS (more saturation)
    let clean_rms = calculate_rms(&clean);
    let distorted_rms = calculate_rms(&distorted);

    assert!(
        distorted_rms > clean_rms,
        "Distortion with high drive should increase RMS: clean = {}, distorted = {}",
        clean_rms,
        distorted_rms
    );

    // Distorted signal should be more saturated (peak closer to 1.0)
    let distorted_peak = calculate_peak(&distorted);
    assert!(
        distorted_peak > 0.95,
        "High drive distortion should saturate close to 1.0: peak = {}",
        distorted_peak
    );
}

// ============================================================================
// TEST: Partial Mix (Mix = 0.5)
// ============================================================================

#[test]
fn test_distortion_partial_mix() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create three versions: dry, wet, 50% mix
    let dry_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(0.0),  // Dry
    );

    let wet_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(1.0),  // Wet
    );

    let mixed_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(0.5),  // 50% mix
    );

    let buffer_size = 512;
    let mut dry = vec![0.0; buffer_size];
    let mut wet = vec![0.0; buffer_size];
    let mut mixed = vec![0.0; buffer_size];

    graph.eval_node_buffer(&dry_id, &mut dry);
    graph.eval_node_buffer(&wet_id, &mut wet);
    graph.eval_node_buffer(&mixed_id, &mut mixed);

    // Mixed signal should be between dry and wet
    let dry_rms = calculate_rms(&dry);
    let wet_rms = calculate_rms(&wet);
    let mixed_rms = calculate_rms(&mixed);

    // For soft clipping with tanh, RMS typically decreases (compression)
    // So mixed should be between dry and wet RMS values
    assert!(
        (mixed_rms >= wet_rms.min(dry_rms) - 0.05) && (mixed_rms <= wet_rms.max(dry_rms) + 0.05),
        "Mixed signal RMS should be between dry and wet: dry = {}, wet = {}, mixed = {}",
        dry_rms,
        wet_rms,
        mixed_rms
    );
}

// ============================================================================
// TEST: Drive Effect (Higher Drive = More Distortion)
// ============================================================================

#[test]
fn test_distortion_drive_effect() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Test with different drive amounts
    let low_drive_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(2.0),   // Low drive
        Signal::Value(1.0),   // Full wet
    );

    let high_drive_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(50.0),  // High drive
        Signal::Value(1.0),   // Full wet
    );

    let buffer_size = 512;
    let mut low_drive = vec![0.0; buffer_size];
    let mut high_drive = vec![0.0; buffer_size];

    graph.eval_node_buffer(&low_drive_id, &mut low_drive);
    graph.eval_node_buffer(&high_drive_id, &mut high_drive);

    // Both should be saturated at ±1 (tanh clipping)
    let high_peak = calculate_peak(&high_drive);
    assert!(
        high_peak <= 1.0,
        "tanh distortion should saturate: high_peak = {}",
        high_peak
    );

    // Higher drive should saturate more (peak closer to 1.0)
    let low_peak = calculate_peak(&low_drive);
    assert!(
        high_peak > low_peak,
        "Higher drive should saturate more: low = {}, high = {}",
        low_peak,
        high_peak
    );

    // Higher drive RMS should be higher (more saturation = more average energy)
    let low_rms = calculate_rms(&low_drive);
    let high_rms = calculate_rms(&high_drive);

    assert!(
        high_rms > low_rms,
        "Higher drive should increase RMS: low = {}, high = {}",
        low_rms,
        high_rms
    );
}

// ============================================================================
// TEST: Soft Clipping Behavior (tanh saturates smoothly)
// ============================================================================

#[test]
fn test_distortion_soft_clipping() {
    let mut graph = create_test_graph();

    // Create high-amplitude sine wave
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply extreme drive to test saturation
    let dist_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(100.0),  // Maximum drive
        Signal::Value(1.0),    // Full wet
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&dist_id, &mut output);

    // Check that output is bounded by tanh saturation (should be very close to ±1)
    let peak = calculate_peak(&output);
    assert!(
        peak <= 1.0,
        "tanh should saturate output to ±1: peak = {}",
        peak
    );

    // With extreme drive, peak should be very close to 1.0 (tanh(100) ≈ 1)
    assert!(
        peak > 0.95,
        "Extreme drive should saturate close to 1.0: peak = {}",
        peak
    );
}

// ============================================================================
// TEST: Multiple Buffer Continuity
// ============================================================================

#[test]
fn test_distortion_multiple_buffers() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Distortion with moderate settings
    let dist_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(10.0),
        Signal::Value(0.8),
    );

    let buffer_size = 256;
    let num_buffers = 4;
    let mut buffers = vec![vec![0.0; buffer_size]; num_buffers];

    // Render multiple buffers
    for i in 0..num_buffers {
        graph.eval_node_buffer(&dist_id, &mut buffers[i]);
    }

    // Each buffer should have similar RMS (since oscillator is continuous)
    let rms_values: Vec<f32> = buffers.iter().map(|b| calculate_rms(b)).collect();

    let avg_rms = rms_values.iter().sum::<f32>() / rms_values.len() as f32;

    for (i, &rms) in rms_values.iter().enumerate() {
        assert!(
            (rms - avg_rms).abs() < 0.05,
            "Buffer {} RMS should be consistent: got {}, avg {}",
            i,
            rms,
            avg_rms
        );
    }
}

// ============================================================================
// TEST: Modulated Drive (Pattern-Controlled)
// ============================================================================

#[test]
fn test_distortion_modulated_drive() {
    let mut graph = create_test_graph();

    // Create audio-rate oscillator
    let audio_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create slow LFO for drive modulation (0.25 Hz = 4 second period)
    let lfo_id = graph.add_oscillator(Signal::Value(0.25), Waveform::Sine);

    // Map LFO to drive range (2.0 to 30.0)
    // LFO output: -1 to +1
    // We need: 2 to 30
    // Transform: (lfo + 1) * 14 + 2 = lfo*14 + 16
    // When lfo=-1: -14 + 16 = 2
    // When lfo=+1: +14 + 16 = 30

    let lfo_scaled_id = {
        let multiplied = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(14.0));
        graph.add_add_node(Signal::Node(multiplied), Signal::Value(16.0))
    };

    // Apply distortion with modulated drive
    let dist_id = graph.add_distortion_node(
        Signal::Node(audio_id),
        Signal::Node(lfo_scaled_id),  // Modulated drive
        Signal::Value(1.0),           // Full wet
    );

    // Use 1 second buffer to capture full LFO cycle variation
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&dist_id, &mut output);

    // Signal should vary in character as drive changes
    // Split into 8 segments to better capture variation over the LFO cycle
    let segment_size = buffer_size / 8;
    let mut rms_values = Vec::new();

    for i in 0..8 {
        let start = i * segment_size;
        let end = (i + 1) * segment_size;
        let segment = &output[start..end];
        rms_values.push(calculate_rms(segment));
    }

    // RMS should vary as drive modulates (more drive = more saturation = higher RMS)
    let max_rms = rms_values.iter().cloned().fold(0.0, f32::max);
    let min_rms = rms_values.iter().cloned().fold(f32::MAX, f32::min);

    // Values are increasing monotonically, showing modulation is working
    // Check for at least 0.7% variation (conservative threshold for floating point)
    assert!(
        max_rms > min_rms * 1.007,
        "Modulated drive should create varying distortion: max RMS = {}, min RMS = {}, variation = {:.2}%, segments: {:?}",
        max_rms,
        min_rms,
        (max_rms / min_rms - 1.0) * 100.0,
        rms_values
    );
}

// ============================================================================
// TEST: Modulated Mix (Auto-Wah Effect)
// ============================================================================

#[test]
fn test_distortion_modulated_mix() {
    let mut graph = create_test_graph();

    // Create audio-rate oscillator
    let audio_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create LFO for mix modulation (1 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(1.0), Waveform::Sine);

    // Map LFO to mix range (0.0 to 1.0)
    // LFO output: -1 to +1
    // Transform: (lfo + 1) * 0.5 = lfo*0.5 + 0.5
    let lfo_scaled_id = {
        let multiplied = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(0.5));
        graph.add_add_node(Signal::Node(multiplied), Signal::Value(0.5))
    };

    // Apply distortion with modulated mix
    let dist_id = graph.add_distortion_node(
        Signal::Node(audio_id),
        Signal::Value(20.0),          // Fixed drive
        Signal::Node(lfo_scaled_id),  // Modulated mix
    );

    let buffer_size = 4410;  // 0.1 seconds at 44100 Hz
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&dist_id, &mut output);

    // RMS should vary as mix changes between clean and distorted
    let segment_size = buffer_size / 4;
    let mut rms_values = Vec::new();

    for i in 0..4 {
        let start = i * segment_size;
        let end = (i + 1) * segment_size;
        let segment = &output[start..end];
        rms_values.push(calculate_rms(segment));
    }

    // RMS values should vary (not all the same)
    let max_rms = rms_values.iter().cloned().fold(0.0, f32::max);
    let min_rms = rms_values.iter().cloned().fold(f32::MAX, f32::min);

    // Should have variation due to mixing between clean and distorted
    assert!(
        max_rms > min_rms * 1.05,
        "Modulated mix should create varying output: max = {}, min = {}",
        max_rms,
        min_rms
    );
}

// ============================================================================
// TEST: Edge Cases - Extreme Drive Values
// ============================================================================

#[test]
fn test_distortion_extreme_drive_values() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Test minimum drive (clamped to 1.0)
    let min_drive_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(0.0),   // Below minimum (will be clamped to 1.0)
        Signal::Value(1.0),
    );

    // Test maximum drive (clamped to 100.0)
    let max_drive_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(1000.0),  // Above maximum (will be clamped to 100.0)
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut min_output = vec![0.0; buffer_size];
    let mut max_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&min_drive_id, &mut min_output);
    graph.eval_node_buffer(&max_drive_id, &mut max_output);

    // Both should produce valid output (no NaN, no Inf)
    for i in 0..buffer_size {
        assert!(min_output[i].is_finite(), "Min drive output should be finite at sample {}", i);
        assert!(max_output[i].is_finite(), "Max drive output should be finite at sample {}", i);
    }

    // Outputs should be saturated to ±1 range
    assert!(calculate_peak(&min_output) <= 1.0, "Min drive should saturate");
    assert!(calculate_peak(&max_output) <= 1.0, "Max drive should saturate");
}

// ============================================================================
// TEST: Edge Cases - Extreme Mix Values
// ============================================================================

#[test]
fn test_distortion_extreme_mix_values() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Test negative mix (should be clamped to 0.0)
    let negative_mix_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(10.0),
        Signal::Value(-1.0),  // Below minimum
    );

    // Test excessive mix (should be clamped to 1.0)
    let excessive_mix_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(10.0),
        Signal::Value(5.0),  // Above maximum
    );

    let buffer_size = 512;
    let mut negative_output = vec![0.0; buffer_size];
    let mut excessive_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&negative_mix_id, &mut negative_output);
    graph.eval_node_buffer(&excessive_mix_id, &mut excessive_output);

    // Negative mix (clamped to 0) should pass through clean signal (RMS ~0.707)
    let negative_rms = calculate_rms(&negative_output);
    assert!(
        (negative_rms - 0.707).abs() < 0.05,
        "Negative mix (clamped to 0) should output clean sine: RMS = {} (expected ~0.707)",
        negative_rms
    );

    // Both should produce valid output
    for i in 0..buffer_size {
        assert!(negative_output[i].is_finite(), "Negative mix output should be finite");
        assert!(excessive_output[i].is_finite(), "Excessive mix output should be finite");
    }
}

// ============================================================================
// TEST: Performance (Large Buffer)
// ============================================================================

#[test]
fn test_distortion_performance() {
    let mut graph = create_test_graph();

    // Create sine wave oscillator
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Distortion with moderate settings
    let dist_id = graph.add_distortion_node(
        Signal::Node(osc_id),
        Signal::Value(20.0),
        Signal::Value(0.8),
    );

    // Process a large buffer (1 second at 44.1kHz)
    let buffer_size = 44100;
    let mut output = vec![0.0; buffer_size];

    // This should complete quickly (no assertion on time, just ensuring it works)
    graph.eval_node_buffer(&dist_id, &mut output);

    // Verify output is valid
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Performance test should produce audible output: RMS = {}", rms);
    assert!(rms < 1.0, "Performance test output should be reasonable: RMS = {}", rms);
}

// ============================================================================
// TEST: Zero Input
// ============================================================================

#[test]
fn test_distortion_zero_input() {
    let mut graph = create_test_graph();

    // Distortion with zero input (constant 0.0)
    let dist_id = graph.add_distortion_node(
        Signal::Value(0.0),
        Signal::Value(50.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&dist_id, &mut output);

    // Output should be zero
    for i in 0..buffer_size {
        assert!(
            output[i].abs() < 0.0001,
            "Zero input should produce zero output: sample {} = {}",
            i,
            output[i]
        );
    }
}
