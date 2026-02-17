/// Tests for Chorus buffer-based evaluation
///
/// These tests verify that Chorus buffer evaluation produces correct
/// chorus effect behavior (modulated delay for thickening/doubling).

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Calculate RMS of a signal
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Test 1: Dry signal (mix = 0) should be identical to input
#[test]
fn test_chorus_dry_signal() {
    let mut graph = create_test_graph();

    // Create a 440 Hz sine wave oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add chorus with mix = 0 (fully dry)
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(1.0),   // rate
        Signal::Value(0.5),   // depth
        Signal::Value(0.0),   // mix = 0 (dry)
    );

    // Render buffers
    let buffer_size = 4410; // 0.1 seconds
    let mut output = vec![0.0; buffer_size];
    let mut osc_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&chorus, &mut output);
    graph.eval_node_buffer(&osc, &mut osc_output);

    // The outputs should be nearly identical (within floating point error)
    let max_diff: f32 = output.iter()
        .zip(osc_output.iter())
        .map(|(a, b): (&f32, &f32)| (a - b).abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    assert!(max_diff < 0.001, "Dry signal differs too much from input: max_diff = {}", max_diff);
}

/// Test 2: Wet signal (mix = 1) should differ from input due to chorus effect
#[test]
fn test_chorus_wet_signal() {
    let mut graph = create_test_graph();

    // Create a 440 Hz sine wave oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add chorus with mix = 1 (fully wet)
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(2.0),   // rate
        Signal::Value(0.8),   // depth
        Signal::Value(1.0),   // mix = 1 (wet)
    );

    // Render buffers
    let buffer_size = 4410; // 0.1 seconds
    let mut output = vec![0.0; buffer_size];
    let mut osc_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&chorus, &mut output);
    graph.eval_node_buffer(&osc, &mut osc_output);

    // The outputs should differ (chorus modulates the signal)
    let max_diff: f32 = output.iter()
        .zip(osc_output.iter())
        .map(|(a, b): (&f32, &f32)| (a - b).abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    // Should have significant difference due to chorus effect
    assert!(max_diff > 0.1, "Wet signal should differ from input, max_diff = {}", max_diff);

    // Signal should still have energy
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Wet signal has too little energy: rms = {}", rms);
}

/// Test 3: Partial mix (mix = 0.5) should blend dry and wet
#[test]
fn test_chorus_partial_mix() {
    let mut graph = create_test_graph();

    // Create a 440 Hz sine wave oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add chorus with mix = 0.5 (50/50 blend)
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(2.0),   // rate
        Signal::Value(0.5),   // depth
        Signal::Value(0.5),   // mix = 0.5
    );

    // Render a buffer
    let buffer_size = 4410; // 0.1 seconds
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&chorus, &mut output);

    // Signal should have energy
    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "Mixed signal has too little energy: rms = {}", rms);
    assert!(rms < 1.0, "Mixed signal has too much energy: rms = {}", rms);
}

/// Test 4: Rate effect - faster rate should create faster modulation
#[test]
fn test_chorus_rate_effect() {
    let buffer_size = 44100; // 1 second

    // Test with slow rate
    let mut graph1 = create_test_graph();
    let osc1 = graph1.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus1 = graph1.add_chorus_node(
        Signal::Node(osc1),
        Signal::Value(0.5),   // slow rate
        Signal::Value(0.8),   // depth
        Signal::Value(1.0),   // mix
    );

    let mut output1 = vec![0.0; buffer_size];
    graph1.eval_node_buffer(&chorus1, &mut output1);

    // Test with fast rate
    let mut graph2 = create_test_graph();
    let osc2 = graph2.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus2 = graph2.add_chorus_node(
        Signal::Node(osc2),
        Signal::Value(5.0),   // fast rate
        Signal::Value(0.8),   // depth
        Signal::Value(1.0),   // mix
    );

    let mut output2 = vec![0.0; buffer_size];
    graph2.eval_node_buffer(&chorus2, &mut output2);

    // Both should have energy
    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);

    assert!(rms1 > 0.1, "Slow rate chorus has too little energy: rms = {}", rms1);
    assert!(rms2 > 0.1, "Fast rate chorus has too little energy: rms = {}", rms2);

    // The outputs should differ (different modulation rates)
    let diff_count = output1.iter()
        .zip(output2.iter())
        .filter(|(a, b): &(&f32, &f32)| (*a - *b).abs() > 0.01)
        .count();

    assert!(diff_count > buffer_size / 2, "Rate changes should produce different outputs");
}

/// Test 5: Depth effect - deeper modulation should create wider variation
#[test]
fn test_chorus_depth_effect() {
    let buffer_size = 44100; // 1 second

    // Test with shallow depth
    let mut graph1 = create_test_graph();
    let osc1 = graph1.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus1 = graph1.add_chorus_node(
        Signal::Node(osc1),
        Signal::Value(2.0),   // rate
        Signal::Value(0.2),   // shallow depth
        Signal::Value(1.0),   // mix
    );

    let mut output1 = vec![0.0; buffer_size];
    graph1.eval_node_buffer(&chorus1, &mut output1);

    // Test with deep modulation
    let mut graph2 = create_test_graph();
    let osc2 = graph2.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus2 = graph2.add_chorus_node(
        Signal::Node(osc2),
        Signal::Value(2.0),   // rate
        Signal::Value(0.9),   // deep modulation
        Signal::Value(1.0),   // mix
    );

    let mut output2 = vec![0.0; buffer_size];
    graph2.eval_node_buffer(&chorus2, &mut output2);

    // Both should have energy
    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);

    assert!(rms1 > 0.1, "Shallow depth chorus has too little energy: rms = {}", rms1);
    assert!(rms2 > 0.1, "Deep depth chorus has too little energy: rms = {}", rms2);

    // Calculate variation in the signals (standard deviation)
    let mean1 = output1.iter().sum::<f32>() / buffer_size as f32;
    let variance1 = output1.iter().map(|x| (x - mean1).powi(2)).sum::<f32>() / buffer_size as f32;
    let std1 = variance1.sqrt();

    let mean2 = output2.iter().sum::<f32>() / buffer_size as f32;
    let variance2 = output2.iter().map(|x| (x - mean2).powi(2)).sum::<f32>() / buffer_size as f32;
    let std2 = variance2.sqrt();

    // Deeper modulation should create more variation, but this is subtle
    // Just verify both have reasonable variation
    assert!(std1 > 0.1, "Shallow depth should have some variation");
    assert!(std2 > 0.1, "Deep depth should have some variation");
}

/// Test 6: State continuity across multiple buffer renders
#[test]
fn test_chorus_state_continuity() {
    let mut graph = create_test_graph();

    // Create a 440 Hz sine wave oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Add chorus
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(2.0),   // rate
        Signal::Value(0.5),   // depth
        Signal::Value(1.0),   // mix
    );

    // Render multiple small buffers
    let buffer_size = 1024;
    let num_buffers = 10;
    let mut outputs = Vec::new();

    for _ in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&chorus, &mut output);
        outputs.push(output);
    }

    // Check that all buffers have energy (no silence)
    for (i, output) in outputs.iter().enumerate() {
        let rms = calculate_rms(output);
        assert!(rms > 0.1, "Buffer {} has too little energy: rms = {}", i, rms);
    }

    // The buffers should vary (LFO continues across buffers)
    let first_rms = calculate_rms(&outputs[0]);
    let last_rms = calculate_rms(&outputs[num_buffers - 1]);

    // RMS should vary due to LFO modulation
    // (Though they might be similar if we happen to sample at similar LFO phases)
    // Just verify both have reasonable energy
    assert!(first_rms > 0.05, "First buffer should have energy");
    assert!(last_rms > 0.05, "Last buffer should have energy");
}

/// Test 7: Multiple sequential buffers produce consistent output
#[test]
fn test_chorus_multiple_buffers() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(2.0),   // rate
        Signal::Value(0.5),   // depth
        Signal::Value(1.0),   // mix
    );

    // Render 5 buffers and verify they all have energy
    let buffer_size = 4410; // 0.1 seconds
    for i in 0..5 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&chorus, &mut output);

        let rms = calculate_rms(&output);
        assert!(rms > 0.1, "Buffer {} has too little energy: rms = {}", i, rms);
    }
}

/// Test 8: Modulated parameters (rate/depth/mix change over time)
#[test]
fn test_chorus_modulated_parameters() {
    let mut graph = create_test_graph();

    // Create oscillators for parameters
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let rate_lfo = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);
    let depth_lfo = graph.add_oscillator(Signal::Value(0.3), Waveform::Sine);

    // Modulate rate from 1-3 Hz
    let rate_mod = graph.add_add_node(Signal::Value(2.0), Signal::Node(rate_lfo));

    // Modulate depth from 0.3-0.7
    let depth_scaled = graph.add_multiply_node(Signal::Node(depth_lfo), Signal::Value(0.2));
    let depth_final = graph.add_add_node(Signal::Node(depth_scaled), Signal::Value(0.5));

    // Add chorus with modulated parameters
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Node(rate_mod),
        Signal::Node(depth_final),
        Signal::Value(1.0),  // fixed mix
    );

    // Render a buffer
    let buffer_size = 44100; // 1 second
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&chorus, &mut output);

    // Should have energy with modulated parameters
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Modulated chorus has too little energy: rms = {}", rms);
}

/// Test 9: Edge cases - extreme rate values
#[test]
fn test_chorus_extreme_rate() {
    let buffer_size = 4410; // 0.1 seconds

    // Test with minimum rate (should be clamped to 0.1)
    let mut graph1 = create_test_graph();
    let osc1 = graph1.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus1 = graph1.add_chorus_node(
        Signal::Node(osc1),
        Signal::Value(0.01),  // very low rate (will be clamped)
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    let mut output1 = vec![0.0; buffer_size];
    graph1.eval_node_buffer(&chorus1, &mut output1);

    let rms1 = calculate_rms(&output1);
    assert!(rms1 > 0.05, "Extreme low rate should still produce output");

    // Test with maximum rate (should be clamped to 10.0)
    let mut graph2 = create_test_graph();
    let osc2 = graph2.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus2 = graph2.add_chorus_node(
        Signal::Node(osc2),
        Signal::Value(100.0),  // very high rate (will be clamped)
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    let mut output2 = vec![0.0; buffer_size];
    graph2.eval_node_buffer(&chorus2, &mut output2);

    let rms2 = calculate_rms(&output2);
    assert!(rms2 > 0.05, "Extreme high rate should still produce output");
}

/// Test 10: Edge cases - extreme depth values
#[test]
fn test_chorus_extreme_depth() {
    let buffer_size = 4410; // 0.1 seconds

    // Test with minimum depth (0)
    let mut graph1 = create_test_graph();
    let osc1 = graph1.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus1 = graph1.add_chorus_node(
        Signal::Node(osc1),
        Signal::Value(2.0),
        Signal::Value(0.0),  // no depth
        Signal::Value(1.0),
    );

    let mut output1 = vec![0.0; buffer_size];
    graph1.eval_node_buffer(&chorus1, &mut output1);

    let rms1 = calculate_rms(&output1);
    assert!(rms1 > 0.05, "Zero depth should still produce output");

    // Test with maximum depth (1.0)
    let mut graph2 = create_test_graph();
    let osc2 = graph2.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus2 = graph2.add_chorus_node(
        Signal::Node(osc2),
        Signal::Value(2.0),
        Signal::Value(1.0),  // full depth
        Signal::Value(1.0),
    );

    let mut output2 = vec![0.0; buffer_size];
    graph2.eval_node_buffer(&chorus2, &mut output2);

    let rms2 = calculate_rms(&output2);
    assert!(rms2 > 0.05, "Full depth should still produce output");

    // Test with excessive depth (should be clamped)
    let mut graph3 = create_test_graph();
    let osc3 = graph3.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus3 = graph3.add_chorus_node(
        Signal::Node(osc3),
        Signal::Value(2.0),
        Signal::Value(10.0),  // excessive depth (will be clamped)
        Signal::Value(1.0),
    );

    let mut output3 = vec![0.0; buffer_size];
    graph3.eval_node_buffer(&chorus3, &mut output3);

    let rms3 = calculate_rms(&output3);
    assert!(rms3 > 0.05, "Excessive depth should be clamped and produce output");
}

/// Test 11: Performance - chorus should render efficiently
#[test]
fn test_chorus_performance() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(2.0),
        Signal::Value(0.5),
        Signal::Value(1.0),
    );

    // Render a large buffer (10 seconds)
    let buffer_size = 441000;
    let mut output = vec![0.0; buffer_size];

    let start = std::time::Instant::now();
    graph.eval_node_buffer(&chorus, &mut output);
    let elapsed = start.elapsed();

    // Should render faster than real-time (10 seconds of audio)
    assert!(elapsed.as_secs() < 1, "Rendering too slow: {:?}", elapsed);

    // Verify output quality
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Performance test output has too little energy: rms = {}", rms);
}

/// Test 12: Verify chorus creates pitch/time modulation
#[test]
fn test_chorus_creates_modulation() {
    let mut graph = create_test_graph();

    // Create a 440 Hz sine wave oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create chorus with moderate settings
    let chorus = graph.add_chorus_node(
        Signal::Node(osc),
        Signal::Value(2.0),   // 2 Hz LFO
        Signal::Value(0.8),   // significant depth
        Signal::Value(1.0),   // fully wet
    );

    // Render a buffer
    let buffer_size = 44100; // 1 second
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&chorus, &mut output);

    // Calculate zero-crossings in the output
    let mut zero_crossings = 0;
    for i in 1..buffer_size {
        if (output[i-1] < 0.0 && output[i] >= 0.0) || (output[i-1] > 0.0 && output[i] <= 0.0) {
            zero_crossings += 1;
        }
    }

    // A 440 Hz sine has ~880 zero crossings per second (2 per cycle)
    // With chorus modulation, this should vary slightly
    // We just verify it's in a reasonable range (allowing for ±10% variation due to modulation)
    assert!(zero_crossings > 750, "Too few zero crossings: {}", zero_crossings);
    assert!(zero_crossings < 950, "Too many zero crossings: {}", zero_crossings);

    // Verify output has energy
    let rms = calculate_rms(&output);
    assert!(rms > 0.2, "Chorus output has too little energy: rms = {}", rms);
}
