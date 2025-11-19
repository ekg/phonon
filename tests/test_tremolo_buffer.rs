/// Buffer Evaluation Tests for Tremolo
///
/// Tests the buffer-based evaluation API for the tremolo effect
/// to ensure it produces correct amplitude modulation.

use phonon::unified_graph::{NodeId, Signal, UnifiedSignalGraph, Waveform};

const SAMPLE_RATE: f32 = 44100.0;

/// Helper to calculate RMS
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Helper to find min and max amplitude in buffer
fn calculate_amplitude_range(buffer: &[f32]) -> (f32, f32) {
    let min = buffer
        .iter()
        .map(|s| s.abs())
        .fold(f32::INFINITY, f32::min);
    let max = buffer
        .iter()
        .map(|s| s.abs())
        .fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}

/// LEVEL 1: Zero Depth Passes Through
/// Tests that tremolo with depth=0 passes signal unchanged
#[test]
fn test_tremolo_zero_depth() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Tremolo with zero depth (should pass through)
    let trem_id = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Value(5.0),
        Signal::Value(0.0), // No modulation
    );

    let buffer_size = 512;
    let mut tremolo_out = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&trem_id, &mut tremolo_out);

    // Reset and evaluate original oscillator for comparison
    let mut graph2 = UnifiedSignalGraph::new(SAMPLE_RATE);
    let osc2 = graph2.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    graph2.eval_node_buffer(&osc2, &mut original);

    // Should be very similar (RMS comparison)
    let trem_rms = calculate_rms(&tremolo_out);
    let orig_rms = calculate_rms(&original);

    println!(
        "Zero depth test: trem_rms={:.4}, orig_rms={:.4}, diff={:.4}",
        trem_rms,
        orig_rms,
        (trem_rms - orig_rms).abs()
    );

    assert!(
        (trem_rms - orig_rms).abs() < 0.05,
        "Zero depth should pass through: trem={}, orig={}",
        trem_rms,
        orig_rms
    );
}

/// LEVEL 2: Full Depth Creates Maximum Variation
/// Tests that tremolo with depth=1.0 creates significant amplitude variation
#[test]
fn test_tremolo_full_depth() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Strong tremolo with full depth
    let trem_id = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Value(5.0),  // 5 Hz
        Signal::Value(1.0),  // Full depth
    );

    // Generate multiple buffers to capture full LFO cycle
    let buffer_size = 512;
    let mut all_samples = Vec::new();

    for _ in 0..100 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&trem_id, &mut output);
        all_samples.extend_from_slice(&output);
    }

    // Calculate amplitude variation
    let (min_amp, max_amp) = calculate_amplitude_range(&all_samples);

    println!(
        "Full depth test: min_amp={:.4}, max_amp={:.4}, ratio={:.2}",
        min_amp,
        max_amp,
        max_amp / (min_amp + 0.001)
    );

    // With full depth, amplitude should vary from near 0 to maximum
    // Ratio should be very high
    let ratio = max_amp / (min_amp + 0.001);
    assert!(
        ratio > 5.0,
        "Full depth tremolo should create large amplitude variation: ratio={}",
        ratio
    );
}

/// LEVEL 2: Amplitude Modulation Creates Periodic Variation
/// Tests that tremolo creates periodic amplitude changes
#[test]
fn test_tremolo_amplitude_modulation() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Strong tremolo
    let trem_id = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Value(5.0),  // 5 Hz
        Signal::Value(0.8),  // Deep modulation
    );

    // Generate multiple buffers to capture LFO cycles
    let buffer_size = 512;
    let num_buffers = 100;
    let mut all_samples = Vec::new();

    for _ in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&trem_id, &mut output);
        all_samples.extend_from_slice(&output);
    }

    // Measure amplitude variation over time
    let chunk_size = (SAMPLE_RATE * 0.02) as usize; // 20ms chunks
    let mut amplitudes = Vec::new();

    for chunk in all_samples.chunks(chunk_size) {
        let rms = calculate_rms(chunk);
        amplitudes.push(rms);
    }

    // Find min and max amplitude
    let min_amp = amplitudes
        .iter()
        .cloned()
        .fold(f32::INFINITY, f32::min);
    let max_amp = amplitudes
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);

    println!(
        "Amplitude modulation test: min={:.4}, max={:.4}, ratio={:.2}",
        min_amp,
        max_amp,
        max_amp / min_amp.max(0.001)
    );

    // Should have significant amplitude variation
    let ratio = max_amp / min_amp.max(0.001);
    assert!(
        ratio > 2.0,
        "Tremolo should create amplitude variation: ratio={}",
        ratio
    );
}

/// LEVEL 2: Rate Parameter Controls Modulation Speed
/// Tests that different rates produce different modulation speeds
#[test]
fn test_tremolo_rate_effect() {
    let mut graph_slow = UnifiedSignalGraph::new(SAMPLE_RATE);
    let mut graph_fast = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc_slow = graph_slow.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc_fast = graph_fast.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Slow tremolo
    let trem_slow = graph_slow.add_tremolo_node(
        Signal::Node(osc_slow),
        Signal::Value(2.0),  // 2 Hz
        Signal::Value(0.8),
    );

    // Fast tremolo
    let trem_fast = graph_fast.add_tremolo_node(
        Signal::Node(osc_fast),
        Signal::Value(10.0), // 10 Hz
        Signal::Value(0.8),
    );

    // Count amplitude peaks in both signals
    let buffer_size = 512;
    let num_buffers = 100;

    let count_peaks = |graph: &mut UnifiedSignalGraph, node_id: &NodeId| -> usize {
        let mut all_samples = Vec::new();
        for _ in 0..num_buffers {
            let mut output = vec![0.0; buffer_size];
            graph.eval_node_buffer(node_id, &mut output);
            all_samples.extend_from_slice(&output);
        }

        // Measure RMS over small windows to find peaks
        let window_size = (SAMPLE_RATE * 0.01) as usize; // 10ms windows
        let mut rms_values = Vec::new();

        for chunk in all_samples.chunks(window_size) {
            rms_values.push(calculate_rms(chunk));
        }

        // Count local maxima
        let mut peaks = 0;
        for i in 1..rms_values.len() - 1 {
            if rms_values[i] > rms_values[i - 1] && rms_values[i] > rms_values[i + 1] {
                peaks += 1;
            }
        }
        peaks
    };

    let peaks_slow = count_peaks(&mut graph_slow, &trem_slow);
    let peaks_fast = count_peaks(&mut graph_fast, &trem_fast);

    println!(
        "Rate test: slow_peaks={}, fast_peaks={}",
        peaks_slow, peaks_fast
    );

    // Fast tremolo should have more peaks
    assert!(
        peaks_fast > peaks_slow,
        "Faster tremolo should have more peaks: slow={}, fast={}",
        peaks_slow,
        peaks_fast
    );
}

/// LEVEL 2: State Continuity - Phase Persists
/// Tests that LFO phase persists across buffer evaluations
#[test]
fn test_tremolo_state_continuity() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let trem_id = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Value(5.0),
        Signal::Value(0.9),
    );

    // Render multiple buffers
    let buffer_size = 512;
    let mut all_samples = Vec::new();

    for _ in 0..50 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&trem_id, &mut output);
        all_samples.extend_from_slice(&output);
    }

    // Check that modulation is continuous (no phase resets)
    // Calculate RMS over windows
    let window_size = (SAMPLE_RATE * 0.02) as usize;
    let mut rms_values = Vec::new();

    for chunk in all_samples.chunks(window_size) {
        rms_values.push(calculate_rms(chunk));
    }

    // Check for smooth variation (no sudden jumps that would indicate phase reset)
    let mut max_jump = 0.0f32;
    for i in 1..rms_values.len() {
        let jump = (rms_values[i] - rms_values[i - 1]).abs();
        max_jump = max_jump.max(jump);
    }

    println!("State continuity test: max_jump={:.4}", max_jump);

    // Should have smooth variation (no large jumps)
    assert!(
        max_jump < 0.3,
        "Phase should persist smoothly: max_jump={}",
        max_jump
    );
}

/// LEVEL 2: Depth Parameter Controls Modulation Amount
/// Tests that different depths produce different modulation intensities
#[test]
fn test_tremolo_depth_parameter() {
    let mut graph_shallow = UnifiedSignalGraph::new(SAMPLE_RATE);
    let mut graph_deep = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc_shallow = graph_shallow.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc_deep = graph_deep.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Shallow tremolo
    let trem_shallow = graph_shallow.add_tremolo_node(
        Signal::Node(osc_shallow),
        Signal::Value(5.0),
        Signal::Value(0.2), // Light modulation
    );

    // Deep tremolo
    let trem_deep = graph_deep.add_tremolo_node(
        Signal::Node(osc_deep),
        Signal::Value(5.0),
        Signal::Value(0.9), // Heavy modulation
    );

    // Measure amplitude variation for both
    let buffer_size = 512;
    let num_buffers = 100;

    let measure_variation = |graph: &mut UnifiedSignalGraph, node_id: &NodeId| -> f32 {
        let mut all_samples = Vec::new();
        for _ in 0..num_buffers {
            let mut output = vec![0.0; buffer_size];
            graph.eval_node_buffer(node_id, &mut output);
            all_samples.extend_from_slice(&output);
        }

        // Measure RMS over time chunks to find amplitude variation
        let chunk_size = (SAMPLE_RATE * 0.02) as usize; // 20ms chunks
        let mut rms_values = Vec::new();

        for chunk in all_samples.chunks(chunk_size) {
            rms_values.push(calculate_rms(chunk));
        }

        // Find min and max RMS
        let min_rms = rms_values
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max_rms = rms_values
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);

        max_rms / min_rms.max(0.001)
    };

    let variation_shallow = measure_variation(&mut graph_shallow, &trem_shallow);
    let variation_deep = measure_variation(&mut graph_deep, &trem_deep);

    println!(
        "Depth parameter test: shallow_variation={:.2}, deep_variation={:.2}",
        variation_shallow, variation_deep
    );

    // Deep tremolo should have more variation than shallow
    assert!(
        variation_deep > variation_shallow * 1.5,
        "Deeper tremolo should vary more: shallow={:.2}, deep={:.2}",
        variation_shallow,
        variation_deep
    );
}

/// LEVEL 3: RMS Comparison - Tremolo Reduces Average Level
/// Tests that tremolo reduces overall RMS (due to amplitude modulation)
#[test]
fn test_tremolo_rms_comparison() {
    let mut graph_dry = UnifiedSignalGraph::new(SAMPLE_RATE);
    let mut graph_wet = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc_dry = graph_dry.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let osc_wet = graph_wet.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let trem = graph_wet.add_tremolo_node(
        Signal::Node(osc_wet),
        Signal::Value(5.0),
        Signal::Value(0.8),
    );

    let buffer_size = 2048;
    let mut dry_samples = Vec::new();
    let mut wet_samples = Vec::new();

    // Render multiple buffers
    for _ in 0..50 {
        let mut dry_buf = vec![0.0; buffer_size];
        let mut wet_buf = vec![0.0; buffer_size];

        graph_dry.eval_node_buffer(&osc_dry, &mut dry_buf);
        graph_wet.eval_node_buffer(&trem, &mut wet_buf);

        dry_samples.extend_from_slice(&dry_buf);
        wet_samples.extend_from_slice(&wet_buf);
    }

    let rms_dry = calculate_rms(&dry_samples);
    let rms_wet = calculate_rms(&wet_samples);

    println!(
        "RMS comparison: dry={:.4}, wet={:.4}, ratio={:.2}",
        rms_dry,
        rms_wet,
        rms_wet / rms_dry
    );

    // Tremolo should reduce average RMS (amplitude modulation reduces average level)
    assert!(
        rms_wet < rms_dry * 0.95,
        "Tremolo should reduce RMS: dry={:.4}, wet={:.4}",
        rms_dry,
        rms_wet
    );
}

/// LEVEL 3: Stability Test - No NaN or Inf
/// Tests that tremolo doesn't produce invalid values
#[test]
fn test_tremolo_stability() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let trem_id = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Value(7.0),
        Signal::Value(0.95),
    );

    let buffer_size = 2048;

    for _ in 0..100 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&trem_id, &mut output);

        // Check for invalid values
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                !sample.is_nan() && !sample.is_infinite(),
                "Sample {} is invalid: {}",
                i,
                sample
            );
        }

        // Check for reasonable range
        let max_val = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_val < 10.0,
            "Output should be in reasonable range, got max={}",
            max_val
        );
    }
}

/// LEVEL 3: Different Waveforms
/// Tests tremolo with different carrier waveforms
#[test]
fn test_tremolo_different_waveforms() {
    for waveform in [Waveform::Sine, Waveform::Saw, Waveform::Square, Waveform::Triangle] {
        let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

        let osc = graph.add_oscillator(Signal::Value(220.0), waveform);
        let trem_id = graph.add_tremolo_node(
            Signal::Node(osc),
            Signal::Value(6.0),
            Signal::Value(0.7),
        );

        let buffer_size = 2048;
        let mut output = vec![0.0; buffer_size];

        // Should work with all waveforms
        graph.eval_node_buffer(&trem_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.05,
            "Tremolo with {:?} should produce audible output, got RMS={}",
            waveform,
            rms
        );
    }
}

/// LEVEL 3: Pattern-Controlled Parameters
/// Tests that rate can be controlled by another signal
#[test]
fn test_tremolo_pattern_controlled_params() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // LFO for rate modulation (slow oscillation)
    let rate_lfo = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Tremolo with modulated rate (using Signal::Expression for modulation)
    // This creates: rate = rate_lfo * 2.0 + 5.0 (ranges from 3-7 Hz)
    let trem_id = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Add(
            Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Multiply(
                Signal::Node(rate_lfo),
                Signal::Value(2.0),
            ))),
            Signal::Value(5.0),
        ))),
        Signal::Value(0.6),
    );

    let buffer_size = 512;
    let mut all_samples = Vec::new();

    // Generate fewer buffers to avoid stack overflow
    for _ in 0..20 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&trem_id, &mut output);
        all_samples.extend_from_slice(&output);
    }

    // Should produce varying tremolo effect
    let rms = calculate_rms(&all_samples);
    assert!(
        rms > 0.05,
        "Pattern-controlled tremolo should be audible, got RMS={}",
        rms
    );

    // Check for amplitude variation
    let (min, max) = calculate_amplitude_range(&all_samples);
    let ratio = max / min.max(0.001);
    assert!(
        ratio > 1.5,
        "Pattern-controlled tremolo should create variation: ratio={}",
        ratio
    );
}

/// LEVEL 3: Extreme Parameter Values
/// Tests that tremolo handles edge cases gracefully
#[test]
fn test_tremolo_extreme_parameters() {
    let mut graph = UnifiedSignalGraph::new(SAMPLE_RATE);

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Test extreme rate (should be clamped)
    let trem_extreme_rate = graph.add_tremolo_node(
        Signal::Node(osc),
        Signal::Value(100.0), // Will be clamped to 20 Hz
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&trem_extreme_rate, &mut output);

    // Should not crash or produce invalid values
    let has_invalid = output.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_invalid, "Extreme rate should not produce NaN/Inf");

    // Test extreme depth (should be clamped)
    let mut graph2 = UnifiedSignalGraph::new(SAMPLE_RATE);
    let osc2 = graph2.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let trem_extreme_depth = graph2.add_tremolo_node(
        Signal::Node(osc2),
        Signal::Value(5.0),
        Signal::Value(2.0), // Will be clamped to 1.0
    );

    let mut output2 = vec![0.0; buffer_size];
    graph2.eval_node_buffer(&trem_extreme_depth, &mut output2);

    let has_invalid2 = output2.iter().any(|s| s.is_nan() || s.is_infinite());
    assert!(!has_invalid2, "Extreme depth should not produce NaN/Inf");
}
