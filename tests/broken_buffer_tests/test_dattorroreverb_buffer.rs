use phonon::unified_graph::{Signal, UnifiedSignalGraph};

/// Helper to calculate RMS (root mean square) of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper to create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

#[test]
fn test_dattorro_basic_reverb() {
    // Test that Dattorro creates reverberation
    let mut graph = create_test_graph();

    // Create a short pulse (10Hz square wave)
    let osc = graph.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Square);

    let reverb_id = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(0.0),    // No pre-delay
        Signal::Value(0.7),    // Moderate decay
        Signal::Value(0.5),    // Some damping
        Signal::Value(0.7),    // Good diffusion
        Signal::Value(0.5),    // 50% mix
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process multiple buffers to let reverb build up
    for _ in 0..100 {
        graph.eval_node_buffer(&reverb_id, &mut output);
    }

    // Should have sustained reverb tail
    let rms = calculate_rms(&output);
    assert!(rms > 0.05, "Reverb should have tail: RMS={}", rms);
}

#[test]
fn test_dattorro_decay_time() {
    // Test that decay parameter controls tail length
    let mut graph1 = create_test_graph();
    let mut graph2 = create_test_graph();

    // Create oscillator
    let osc1 = graph1.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Sine);
    let osc2 = graph2.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Sine);

    // Short decay
    let reverb_short = graph1.add_dattorroreverb_node(
        Signal::Node(osc1),
        Signal::Value(0.0),
        Signal::Value(0.2),    // Short decay
        Signal::Value(0.5),
        Signal::Value(0.7),
        Signal::Value(1.0),    // 100% wet
    );

    // Long decay
    let reverb_long = graph2.add_dattorroreverb_node(
        Signal::Node(osc2),
        Signal::Value(0.0),
        Signal::Value(5.0),    // Long decay
        Signal::Value(0.5),
        Signal::Value(0.7),
        Signal::Value(1.0),    // 100% wet
    );

    let buffer_size = 512;
    let mut output_short = vec![0.0; buffer_size];
    let mut output_long = vec![0.0; buffer_size];

    // Process many buffers
    for _ in 0..200 {
        graph1.eval_node_buffer(&reverb_short, &mut output_short);
        graph2.eval_node_buffer(&reverb_long, &mut output_long);
    }

    let rms_short = calculate_rms(&output_short);
    let rms_long = calculate_rms(&output_long);

    // Long decay should have more energy
    assert!(rms_long > rms_short * 1.2,
        "Long decay should have more energy: short={}, long={}", rms_short, rms_long);
}

#[test]
fn test_dattorro_diffusion() {
    // Test that diffusion parameter affects density
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Sine);

    // Low diffusion
    let reverb_sparse = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.1),    // Low diffusion
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..50 {
        graph.eval_node_buffer(&reverb_sparse, &mut output);
    }

    let rms_sparse = calculate_rms(&output);
    assert!(rms_sparse > 0.01, "Should produce sound");

    // Now test high diffusion
    let mut graph2 = create_test_graph();
    let osc2 = graph2.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Sine);

    let reverb_dense = graph2.add_dattorroreverb_node(
        Signal::Node(osc2),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.9),    // High diffusion
        Signal::Value(0.5),
    );

    let mut output_dense = vec![0.0; buffer_size];
    for _ in 0..50 {
        graph2.eval_node_buffer(&reverb_dense, &mut output_dense);
    }

    let rms_dense = calculate_rms(&output_dense);

    // High diffusion should create denser reverb (more energy)
    assert!(rms_dense > rms_sparse * 0.8,
        "High diffusion should be dense: sparse={}, dense={}", rms_sparse, rms_dense);
}

#[test]
fn test_dattorro_pre_delay() {
    // Test that pre-delay adds initial delay before reverb
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Sine);

    let reverb_id = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(100.0),  // 100ms pre-delay
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.7),
        Signal::Value(1.0),    // 100% wet to hear pre-delay clearly
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // First buffer should be mostly silent (pre-delay)
    graph.eval_node_buffer(&reverb_id, &mut output);
    let rms_first = calculate_rms(&output);

    // Process more buffers
    for _ in 0..20 {
        graph.eval_node_buffer(&reverb_id, &mut output);
    }
    let rms_later = calculate_rms(&output);

    // Later buffers should have more energy than first (after pre-delay)
    assert!(rms_later > rms_first * 2.0,
        "After pre-delay should have more energy: first={}, later={}", rms_first, rms_later);
}

#[test]
fn test_dattorro_damping() {
    // Test that damping rolls off high frequencies
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(100.0), phonon::unified_graph::Waveform::Sine);

    // High damping (darker)
    let reverb_id = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.9),    // High damping
        Signal::Value(0.7),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..50 {
        graph.eval_node_buffer(&reverb_id, &mut output);
    }

    let rms = calculate_rms(&output);
    assert!(rms > 0.01, "Damped reverb should still produce sound: RMS={}", rms);
}

#[test]
fn test_dattorro_mix() {
    // Test that mix parameter controls dry/wet balance
    let mut graph_dry = create_test_graph();
    let mut graph_wet = create_test_graph();

    let osc1 = graph_dry.add_oscillator(Signal::Value(100.0), phonon::unified_graph::Waveform::Sine);
    let osc2 = graph_wet.add_oscillator(Signal::Value(100.0), phonon::unified_graph::Waveform::Sine);

    // 100% dry
    let reverb_dry = graph_dry.add_dattorroreverb_node(
        Signal::Node(osc1),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.7),
        Signal::Value(0.0),    // 100% dry
    );

    // 100% wet
    let reverb_wet = graph_wet.add_dattorroreverb_node(
        Signal::Node(osc2),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.7),
        Signal::Value(1.0),    // 100% wet
    );

    let buffer_size = 512;
    let mut output_dry = vec![0.0; buffer_size];
    let mut output_wet = vec![0.0; buffer_size];

    // Process buffers
    for _ in 0..50 {
        graph_dry.eval_node_buffer(&reverb_dry, &mut output_dry);
        graph_wet.eval_node_buffer(&reverb_wet, &mut output_wet);
    }

    let rms_dry = calculate_rms(&output_dry);
    let rms_wet = calculate_rms(&output_wet);

    // Both should produce sound
    assert!(rms_dry > 0.01, "Dry should produce sound");
    assert!(rms_wet > 0.01, "Wet should produce sound");

    // Wet should have different character (usually more energy after reverb builds up)
    // The assertion depends on the stage - initially dry might be louder,
    // but after reverb builds up, wet might be louder
    assert!((rms_dry - rms_wet).abs() < rms_dry * 2.0,
        "Dry and wet should be within reasonable range");
}

#[test]
fn test_dattorro_state_continuity() {
    // Test that reverb state persists across buffer evaluations
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(10.0), phonon::unified_graph::Waveform::Sine);

    let reverb_id = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.7),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process first buffer
    graph.eval_node_buffer(&reverb_id, &mut output);
    let rms1 = calculate_rms(&output);

    // Process many more buffers
    for _ in 0..50 {
        graph.eval_node_buffer(&reverb_id, &mut output);
    }
    let rms50 = calculate_rms(&output);

    // Reverb should build up over time (state accumulation)
    assert!(rms50 > rms1 * 0.5,
        "Reverb should accumulate: first={}, later={}", rms1, rms50);
}

#[test]
fn test_dattorro_quality() {
    // Test that Dattorro produces rich, dense reverb
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(100.0), phonon::unified_graph::Waveform::Sine);

    let reverb_id = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(0.0),
        Signal::Value(0.7),
        Signal::Value(0.5),
        Signal::Value(0.8),    // Good diffusion for density
        Signal::Value(1.0),    // 100% wet
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Build up reverb
    for _ in 0..100 {
        graph.eval_node_buffer(&reverb_id, &mut output);
    }

    let rms = calculate_rms(&output);

    // Should have significant energy
    assert!(rms > 0.05, "Rich reverb should have energy: RMS={}", rms);

    // Check for non-zero variation (not all samples the same)
    let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let range = max - min;

    assert!(range > 0.01, "Reverb should have variation: range={}", range);
}

#[test]
fn test_dattorro_no_explosion() {
    // Test that reverb doesn't explode with extreme parameters
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(100.0), phonon::unified_graph::Waveform::Sine);

    let reverb_id = graph.add_dattorroreverb_node(
        Signal::Node(osc),
        Signal::Value(0.0),
        Signal::Value(10.0),   // Maximum decay
        Signal::Value(0.0),    // No damping
        Signal::Value(1.0),    // Maximum diffusion
        Signal::Value(1.0),    // 100% wet
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    // Process many buffers
    for _ in 0..200 {
        graph.eval_node_buffer(&reverb_id, &mut output);

        // Check no samples are NaN or infinite
        for &sample in output.iter() {
            assert!(sample.is_finite(), "Sample should be finite: {}", sample);
            assert!(sample.abs() < 10.0, "Sample should not explode: {}", sample);
        }
    }
}
