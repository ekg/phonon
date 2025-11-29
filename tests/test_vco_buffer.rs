use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
/// Tests for VCO (Voltage-Controlled Oscillator) buffer-based evaluation
///
/// VCO is an analog-style oscillator with PolyBLEP anti-aliasing, supporting:
/// - Multiple waveforms: saw (0), square (1), triangle (2), sine (3)
/// - Pulse width modulation (PWM) for square wave
/// - Analog warmth characteristics
///
/// These tests verify correct waveform generation, anti-aliasing, and phase continuity.
use std::cell::RefCell;

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Find zero crossings (count sign changes)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i - 1] < 0.0 && buffer[i] >= 0.0) || (buffer[i - 1] >= 0.0 && buffer[i] < 0.0) {
            count += 1;
        }
    }
    count
}

/// Helper: Calculate peak amplitude
fn peak_amplitude(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
}

// ============================================================================
// TEST: Sawtooth Wave (Waveform = 0)
// ============================================================================

#[test]
fn test_vco_sawtooth_amplitude() {
    let mut graph = create_test_graph();

    // VCO sawtooth at 220 Hz, waveform=0, pw=0.5 (not used for saw)
    let vco_id = graph.add_node(SignalNode::VCO {
        frequency: Signal::Value(220.0),
        waveform: Signal::Value(0.0), // Sawtooth
        pulse_width: Signal::Value(0.5),
        phase: RefCell::new(0.0),
    });

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Sawtooth should have peak amplitude ~1.0
    let peak = peak_amplitude(&output);
    assert!(
        peak > 0.9 && peak <= 1.1,
        "Sawtooth peak amplitude should be ~1.0, got {}",
        peak
    );

    // RMS should be reasonable
    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "Sawtooth RMS too low: {}", rms);
}

#[test]
fn test_vco_sawtooth_frequency_accuracy() {
    let mut graph = create_test_graph();
    let sample_rate = 44100.0;
    let frequency = 220.0;

    let vco_id = graph.add_vco_node(
        Signal::Value(frequency),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    // Generate enough samples to capture multiple cycles
    let duration_seconds = 0.1; // 100ms
    let buffer_size = (sample_rate * duration_seconds) as usize;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Count zero crossings (each cycle has 2 zero crossings)
    let zero_crossings = count_zero_crossings(&output);
    let cycles = zero_crossings as f32 / 2.0;
    let measured_freq = cycles / duration_seconds;

    // Allow 5% tolerance
    let tolerance = frequency * 0.05;
    assert!(
        (measured_freq - frequency).abs() < tolerance,
        "Expected ~{} Hz, measured {} Hz (from {} zero crossings)",
        frequency,
        measured_freq,
        zero_crossings
    );
}

#[test]
fn test_vco_sawtooth_waveform_shape() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(110.0), // Low frequency for clear shape
        Signal::Value(0.0),   // Sawtooth
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 2048];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Sawtooth should ramp from -1 to 1 (or 1 to -1)
    // Look for mostly monotonic behavior within a cycle
    let mut rising_count = 0;
    let mut falling_count = 0;

    for i in 1..output.len() {
        if output[i] > output[i - 1] {
            rising_count += 1;
        } else if output[i] < output[i - 1] {
            falling_count += 1;
        }
    }

    // Sawtooth ramps down: mostly falling with sharp rises
    // Or ramps up: mostly rising with sharp falls
    let total = rising_count + falling_count;
    let dominant = rising_count.max(falling_count);

    assert!(
        dominant as f32 / total as f32 > 0.6,
        "Sawtooth should have clear directional ramp, rising={}, falling={}",
        rising_count,
        falling_count
    );
}

// ============================================================================
// TEST: Square Wave (Waveform = 1)
// ============================================================================

#[test]
fn test_vco_square_wave_50_percent_duty() {
    let mut graph = create_test_graph();

    // Square wave at 50% duty cycle (pw = 0.5)
    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(1.0), // Square wave
        Signal::Value(0.5), // 50% duty cycle
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Square wave should be mostly at ±1.0
    let near_one = output.iter().filter(|&&x| (x - 1.0).abs() < 0.1).count();
    let near_neg_one = output.iter().filter(|&&x| (x + 1.0).abs() < 0.1).count();
    let total_near_extremes = near_one + near_neg_one;

    // At least 80% should be at extremes (allowing for PolyBLEP smoothing)
    assert!(
        total_near_extremes > 512 * 80 / 100,
        "Square wave should mostly be at ±1.0, got {}/{} samples",
        total_near_extremes,
        512
    );

    // With 50% duty cycle, high and low should be roughly equal
    let ratio = near_one as f32 / near_neg_one as f32;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "50% duty cycle should have equal high/low time, ratio={}",
        ratio
    );
}

#[test]
fn test_vco_square_wave_pwm_25_percent() {
    let mut graph = create_test_graph();

    // Square wave at 25% duty cycle
    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(1.0),  // Square wave
        Signal::Value(0.25), // 25% duty cycle
    );

    let mut output = vec![0.0; 2048]; // Larger buffer for better statistics
    graph.eval_node_buffer(&vco_id, &mut output);

    let near_one = output.iter().filter(|&&x| (x - 1.0).abs() < 0.1).count();
    let near_neg_one = output.iter().filter(|&&x| (x + 1.0).abs() < 0.1).count();

    // With 25% duty cycle, low time should be ~3x high time
    let ratio = near_neg_one as f32 / near_one as f32;
    assert!(
        ratio > 2.0 && ratio < 4.0,
        "25% duty cycle should have 3:1 low:high ratio, got {}",
        ratio
    );
}

#[test]
fn test_vco_square_wave_pwm_75_percent() {
    let mut graph = create_test_graph();

    // Square wave at 75% duty cycle
    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(1.0),  // Square wave
        Signal::Value(0.75), // 75% duty cycle
    );

    let mut output = vec![0.0; 2048]; // Larger buffer for better statistics
    graph.eval_node_buffer(&vco_id, &mut output);

    let near_one = output.iter().filter(|&&x| (x - 1.0).abs() < 0.1).count();
    let near_neg_one = output.iter().filter(|&&x| (x + 1.0).abs() < 0.1).count();

    // With 75% duty cycle, high time should be ~3x low time
    let ratio = near_one as f32 / near_neg_one as f32;
    assert!(
        ratio > 2.0 && ratio < 4.0,
        "75% duty cycle should have 3:1 high:low ratio, got {}",
        ratio
    );
}

// ============================================================================
// TEST: Triangle Wave (Waveform = 2)
// ============================================================================

#[test]
fn test_vco_triangle_wave_amplitude() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(2.0), // Triangle
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Triangle wave ranges from -1 to 1
    let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    assert!(
        max_val > 0.9 && max_val <= 1.1,
        "Triangle max should be ~1.0, got {}",
        max_val
    );
    assert!(
        min_val < -0.9 && min_val >= -1.1,
        "Triangle min should be ~-1.0, got {}",
        min_val
    );
}

#[test]
fn test_vco_triangle_wave_linearity() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(110.0), // Low frequency for clear shape
        Signal::Value(2.0),   // Triangle
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 2048];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Triangle wave should have roughly equal rising/falling regions
    let mut rising_count = 0;
    let mut falling_count = 0;

    for i in 1..output.len() {
        if output[i] > output[i - 1] {
            rising_count += 1;
        } else if output[i] < output[i - 1] {
            falling_count += 1;
        }
    }

    // Should be roughly 50/50 rising and falling
    let ratio = rising_count as f32 / falling_count as f32;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Triangle should have equal rising/falling time, ratio={}",
        ratio
    );
}

// ============================================================================
// TEST: Sine Wave (Waveform = 3)
// ============================================================================

#[test]
fn test_vco_sine_wave_amplitude() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(440.0),
        Signal::Value(3.0), // Sine
        Signal::Value(0.5),
    );

    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Sine wave peak should be ~1.0
    let peak = peak_amplitude(&output);
    assert!(
        peak > 0.9 && peak <= 1.0,
        "Sine wave peak should be ~1.0, got {}",
        peak
    );

    // Sine wave RMS = 1/sqrt(2) ≈ 0.707
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.6 && rms < 0.8,
        "Sine wave RMS should be ~0.707, got {}",
        rms
    );
}

#[test]
fn test_vco_sine_wave_frequency() {
    let mut graph = create_test_graph();
    let sample_rate = 44100.0;
    let frequency = 440.0;

    let vco_id = graph.add_vco_node(
        Signal::Value(frequency),
        Signal::Value(3.0), // Sine
        Signal::Value(0.5),
    );

    let duration_seconds = 0.1;
    let buffer_size = (sample_rate * duration_seconds) as usize;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    let zero_crossings = count_zero_crossings(&output);
    let cycles = zero_crossings as f32 / 2.0;
    let measured_freq = cycles / duration_seconds;

    let tolerance = frequency * 0.05;
    assert!(
        (measured_freq - frequency).abs() < tolerance,
        "Expected ~{} Hz, measured {} Hz",
        frequency,
        measured_freq
    );
}

// ============================================================================
// TEST: Phase Continuity
// ============================================================================

#[test]
fn test_vco_phase_continuity_saw() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&vco_id, &mut buffer1);
    graph.eval_node_buffer(&vco_id, &mut buffer2);

    // Phase should be continuous (no sudden jumps)
    let last = buffer1[buffer_size - 1];
    let first = buffer2[0];

    // Allow for phase wrap, but check continuity
    // If not wrapping, difference should be small
    // If wrapping (saw resets), that's also valid
    let diff = (first - last).abs();
    let is_continuous = diff < 0.1;
    let is_wrap = diff > 1.8; // Close to 2.0 (saw range is 2.0)

    assert!(
        is_continuous || is_wrap,
        "Phase should be continuous or wrap, diff={}",
        diff
    );
}

#[test]
fn test_vco_phase_continuity_sine() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(440.0),
        Signal::Value(3.0), // Sine
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&vco_id, &mut buffer1);
    graph.eval_node_buffer(&vco_id, &mut buffer2);

    // For sine wave, check smoothness at boundary
    let last = buffer1[buffer_size - 1];
    let first = buffer2[0];

    let diff = (first - last).abs();
    assert!(
        diff < 0.2,
        "Sine phase should be smooth at buffer boundary, diff={}",
        diff
    );
}

// ============================================================================
// TEST: Frequency Modulation
// ============================================================================

#[test]
fn test_vco_frequency_modulation() {
    let mut graph = create_test_graph();

    // Create LFO for frequency modulation
    let lfo_id = graph.add_oscillator(
        Signal::Value(5.0), // 5 Hz LFO
        phonon::unified_graph::Waveform::Sine,
    );

    // VCO with modulated frequency: 220 + (lfo * 50)
    let lfo_scaled_id = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(50.0));

    let freq_mod_id = graph.add_add_node(Signal::Value(220.0), Signal::Node(lfo_scaled_id));

    let vco_id = graph.add_vco_node(
        Signal::Node(freq_mod_id),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    // Generate audio
    let buffer_size = 4410; // 100ms
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Should produce varying output (frequency modulation)
    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "FM VCO should produce audio, RMS={}", rms);

    // Frequency should vary, so zero crossings won't be perfectly regular
    let zc = count_zero_crossings(&output);
    assert!(zc > 10, "FM VCO should have multiple zero crossings");
}

// ============================================================================
// TEST: Pulse Width Modulation
// ============================================================================

#[test]
fn test_vco_pulse_width_modulation() {
    let mut graph = create_test_graph();

    // Create LFO for PWM
    let lfo_id = graph.add_oscillator(
        Signal::Value(2.0), // 2 Hz LFO
        phonon::unified_graph::Waveform::Sine,
    );

    // Scale LFO to 0.2-0.8 range: 0.5 + (lfo * 0.3)
    let lfo_scaled_id = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(0.3));

    let pw_mod_id = graph.add_add_node(Signal::Value(0.5), Signal::Node(lfo_scaled_id));

    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(1.0), // Square wave
        Signal::Node(pw_mod_id),
    );

    // Generate audio
    let buffer_size = 8820; // 200ms to capture PWM sweep
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    // PWM square should still produce square-ish output
    let near_extremes = output.iter().filter(|&&x| x.abs() > 0.8).count();

    assert!(
        near_extremes > buffer_size * 70 / 100,
        "PWM square should still be mostly at extremes"
    );
}

// ============================================================================
// TEST: Anti-Aliasing (PolyBLEP)
// ============================================================================

#[test]
fn test_vco_polyblep_antialiasing() {
    let mut graph = create_test_graph();

    // High frequency saw wave (near Nyquist)
    let vco_id = graph.add_vco_node(
        Signal::Value(5000.0),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    // PolyBLEP should prevent extreme discontinuities
    // Check that no sample-to-sample jump exceeds a threshold
    let mut max_jump = 0.0f32;
    for i in 1..output.len() {
        let jump = (output[i] - output[i - 1]).abs();
        max_jump = max_jump.max(jump);
    }

    // Without PolyBLEP, sawtooth resets would cause jumps ~2.0
    // With PolyBLEP, even at phase reset, transition is smoothed
    // At 5kHz, phase increment is ~0.113, so max smooth jump ~0.23
    assert!(
        max_jump < 0.5,
        "PolyBLEP should reduce discontinuities, max_jump={}",
        max_jump
    );
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_vco_zero_frequency() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(0.0),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Zero frequency should produce constant DC
    let first = output[0];
    for &sample in &output {
        assert!(
            (sample - first).abs() < 0.01,
            "Zero frequency should produce constant output"
        );
    }
}

#[test]
fn test_vco_very_high_frequency() {
    let mut graph = create_test_graph();

    // Very high frequency (near Nyquist)
    let vco_id = graph.add_vco_node(
        Signal::Value(20000.0),
        Signal::Value(3.0), // Sine (naturally band-limited)
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&vco_id, &mut output);

    // Should still produce output
    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "High frequency VCO should produce output");
}

#[test]
fn test_vco_multiple_buffers() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(220.0),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    // Generate multiple consecutive buffers
    let buffer_size = 512;

    for i in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&vco_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.3,
            "Buffer {} should have reasonable RMS: {}",
            i,
            rms
        );
    }
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_vco_buffer_performance() {
    let mut graph = create_test_graph();

    let vco_id = graph.add_vco_node(
        Signal::Value(440.0),
        Signal::Value(0.0), // Sawtooth
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&vco_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "VCO buffer eval: {:?} for {} iterations",
        duration, iterations
    );
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time
    assert!(
        duration.as_secs() < 2,
        "VCO buffer evaluation too slow: {:?}",
        duration
    );
}
