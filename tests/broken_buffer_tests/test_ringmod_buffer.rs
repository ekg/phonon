use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

/// Helper function to create a test graph with standard settings
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0) // 44.1kHz sample rate
}

/// Helper function to calculate RMS (root mean square) of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_of_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_of_squares / buffer.len() as f32).sqrt()
}

/// Helper function to calculate peak amplitude
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
}

/// Helper function to detect zero crossings (for frequency estimation)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    let mut count = 0;
    for i in 1..buffer.len() {
        if (buffer[i - 1] < 0.0 && buffer[i] >= 0.0) || (buffer[i - 1] >= 0.0 && buffer[i] < 0.0)
        {
            count += 1;
        }
    }
    count
}

#[test]
fn test_ringmod_basic_operation() {
    let mut graph = create_test_graph();

    // Create a simple sine wave input at 440 Hz
    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Ring modulate with 100 Hz carrier
    let ringmod_id =
        graph.add_ringmod_node(Signal::Node(input), Signal::Value(100.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut output);

    // Should produce sound (sidebands at 440±100 = 340, 540 Hz)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.3,
        "Ring mod should produce audible sound: RMS={}",
        rms
    );

    // Peak should be reasonable (not clipping, not silent)
    let peak = calculate_peak(&output);
    assert!(
        peak > 0.5 && peak <= 1.1,
        "Ring mod peak should be in reasonable range: peak={}",
        peak
    );
}

#[test]
fn test_ringmod_creates_sidebands() {
    let mut graph = create_test_graph();

    // Two oscillators at different frequencies
    let carrier = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let modulator_freq = 100.0;

    let ringmod_id = graph.add_ringmod_node(Signal::Node(carrier), Signal::Value(modulator_freq));

    let buffer_size = 4410; // 0.1 seconds at 44.1kHz
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut output);

    // Ring modulation creates sum and difference frequencies:
    // 440 + 100 = 540 Hz and 440 - 100 = 340 Hz
    // The result should be complex (not a pure sine)

    let rms = calculate_rms(&output);
    assert!(rms > 0.3, "Ring mod should produce sound: RMS={}", rms);

    // Count zero crossings to estimate frequency content
    // Should have more complex waveform than simple sine
    let crossings = count_zero_crossings(&output);
    // Rough estimate: should have crossings from both sidebands
    assert!(
        crossings > 50,
        "Ring mod should create complex waveform with many zero crossings: {}",
        crossings
    );
}

#[test]
fn test_ringmod_constant_modulator() {
    let mut graph = create_test_graph();

    let carrier = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Constant modulator acts like amplitude modulation with DC offset
    // ringmod with constant 0.5 should produce output at half amplitude
    let ringmod_id =
        graph.add_ringmod_node(Signal::Node(carrier), Signal::Value(0.5));

    let buffer_size = 512;
    let mut modulated = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut modulated);
    graph.eval_node_buffer(&carrier, &mut original);

    // The constant 0.5 is treated as carrier frequency and clamped to 20 Hz minimum
    // So this test actually creates a slow 20 Hz modulation

    let mod_rms = calculate_rms(&modulated);
    let orig_rms = calculate_rms(&original);

    // Both should have sound
    assert!(mod_rms > 0.1, "Modulated should have sound: {}", mod_rms);
    assert!(
        orig_rms > 0.5,
        "Original should have sound: {}",
        orig_rms
    );

    // The modulated version will have lower average amplitude due to 20Hz modulation
    assert!(
        mod_rms < orig_rms,
        "Modulated RMS should be less than original due to slow modulation: mod={}, orig={}",
        mod_rms,
        orig_rms
    );
}

#[test]
fn test_ringmod_zero_frequency() {
    let mut graph = create_test_graph();

    let carrier = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Zero frequency will be clamped to 20 Hz minimum
    let ringmod_id = graph.add_ringmod_node(Signal::Node(carrier), Signal::Value(0.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut output);

    // Should produce sound (20 Hz modulation of 440 Hz carrier)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Ring mod with clamped frequency should produce sound: RMS={}",
        rms
    );
}

#[test]
fn test_ringmod_different_waveforms() {
    let mut graph = create_test_graph();

    let buffer_size = 512;

    // Test with different input waveforms
    let waveforms = vec![
        Waveform::Sine,
        Waveform::Saw,
        Waveform::Square,
        Waveform::Triangle,
    ];

    for waveform in waveforms {
        let input = graph.add_oscillator(Signal::Value(220.0), waveform);
        let ringmod_id =
            graph.add_ringmod_node(Signal::Node(input), Signal::Value(50.0));

        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&ringmod_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.1,
            "Ring mod with {:?} should produce sound: RMS={}",
            waveform, rms
        );
    }
}

#[test]
fn test_ringmod_amplitude_scaling() {
    let mut graph = create_test_graph();

    // Create two inputs with different amplitudes
    let input1 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let input2 = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Scale input2 to half amplitude
    let scaled_input = graph.add_multiply_node(Signal::Node(input2), Signal::Value(0.5));

    // Ring modulate both with same carrier frequency
    let ringmod1 = graph.add_ringmod_node(Signal::Node(input1), Signal::Value(100.0));
    let ringmod2 =
        graph.add_ringmod_node(Signal::Node(scaled_input), Signal::Value(100.0));

    let buffer_size = 512;
    let mut output1 = vec![0.0; buffer_size];
    let mut output2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod1, &mut output1);
    graph.eval_node_buffer(&ringmod2, &mut output2);

    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);

    // RMS of output2 should be approximately half of output1
    let ratio = rms2 / rms1;
    assert!(
        (ratio - 0.5).abs() < 0.15,
        "Amplitude scaling should be preserved: ratio={} (expected ~0.5)",
        ratio
    );
}

#[test]
fn test_ringmod_high_frequency_carrier() {
    let mut graph = create_test_graph();

    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // High carrier frequency (near the clamp limit of 5000 Hz)
    let ringmod_id =
        graph.add_ringmod_node(Signal::Node(input), Signal::Value(4500.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut output);

    // Should produce sound
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.3,
        "Ring mod with high carrier should produce sound: RMS={}",
        rms
    );

    // Should have many zero crossings due to high frequency
    let crossings = count_zero_crossings(&output);
    assert!(
        crossings > 100,
        "High frequency ring mod should have many zero crossings: {}",
        crossings
    );
}

#[test]
fn test_ringmod_low_frequency_carrier() {
    let mut graph = create_test_graph();

    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Low carrier frequency (near the clamp minimum of 20 Hz)
    let ringmod_id = graph.add_ringmod_node(Signal::Node(input), Signal::Value(25.0));

    let buffer_size = 4410; // 0.1 seconds
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut output);

    // Should produce sound
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Ring mod with low carrier should produce sound: RMS={}",
        rms
    );
}

#[test]
fn test_ringmod_metallic_effect() {
    let mut graph = create_test_graph();

    // Create a musical tone
    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Ring modulate with non-harmonic frequency for metallic effect
    let ringmod_id =
        graph.add_ringmod_node(Signal::Node(input), Signal::Value(333.0));

    let buffer_size = 4410; // 0.1 seconds
    let mut modulated = vec![0.0; buffer_size];
    let mut original = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod_id, &mut modulated);
    graph.eval_node_buffer(&input, &mut original);

    // Both should have sound
    let mod_rms = calculate_rms(&modulated);
    let orig_rms = calculate_rms(&original);

    assert!(mod_rms > 0.2, "Modulated should have sound: {}", mod_rms);
    assert!(
        orig_rms > 0.5,
        "Original should have sound: {}",
        orig_rms
    );

    // The modulated version should have more complex frequency content
    // (evidenced by different zero crossing patterns)
    let mod_crossings = count_zero_crossings(&modulated);
    let orig_crossings = count_zero_crossings(&original);

    // Ring modulation should create more complex waveform
    assert!(
        mod_crossings != orig_crossings,
        "Ring mod should change frequency content: mod={}, orig={}",
        mod_crossings,
        orig_crossings
    );
}

#[test]
fn test_ringmod_multiple_buffers() {
    let mut graph = create_test_graph();

    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let ringmod_id =
        graph.add_ringmod_node(Signal::Node(input), Signal::Value(100.0));

    let buffer_size = 512;

    // Render multiple buffers to ensure phase continuity
    for _ in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&ringmod_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.2,
            "Each buffer should have consistent sound: RMS={}",
            rms
        );
    }
}

#[test]
fn test_ringmod_phase_continuity() {
    let mut graph = create_test_graph();

    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let ringmod_id =
        graph.add_ringmod_node(Signal::Node(input), Signal::Value(100.0));

    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    // Render two consecutive buffers
    graph.eval_node_buffer(&ringmod_id, &mut buffer1);
    graph.eval_node_buffer(&ringmod_id, &mut buffer2);

    // Check that the last sample of buffer1 and first sample of buffer2
    // are reasonably close (phase continuity)
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];

    // They shouldn't be wildly different (no phase discontinuity)
    let diff: f32 = (last_sample - first_sample).abs();
    assert!(
        diff < 1.0,
        "Phase should be continuous between buffers: diff={}",
        diff
    );
}

#[test]
fn test_ringmod_chaining() {
    let mut graph = create_test_graph();

    // Create a chain of ring modulators
    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let ringmod1 = graph.add_ringmod_node(Signal::Node(input), Signal::Value(100.0));
    let ringmod2 =
        graph.add_ringmod_node(Signal::Node(ringmod1), Signal::Value(50.0));

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&ringmod2, &mut output);

    // Chained ring modulation should still produce sound
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Chained ring modulators should produce sound: RMS={}",
        rms
    );
}

#[test]
fn test_ringmod_dynamic_frequency() {
    let mut graph = create_test_graph();

    let input = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Use a slow LFO as modulation frequency (will be clamped to 20 Hz minimum)
    let lfo = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Scale LFO to modulation range (e.g., 50-150 Hz)
    // lfo output is -1 to 1, so lfo * 50 + 100 gives 50 to 150 Hz
    let scaled_lfo = graph.add_multiply_node(Signal::Node(lfo), Signal::Value(50.0));
    let freq_signal = graph.add_add_node(Signal::Node(scaled_lfo), Signal::Value(100.0));

    let ringmod_id = graph.add_ringmod_node(Signal::Node(input), Signal::Node(freq_signal));

    let buffer_size = 512;

    // Render multiple buffers with varying modulation frequency
    for _ in 0..5 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&ringmod_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.1,
            "Ring mod with dynamic frequency should produce sound: RMS={}",
            rms
        );
    }
}
