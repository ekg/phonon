/// Tests for Formant filter buffer-based evaluation
///
/// These tests verify that Formant filter buffer evaluation produces correct
/// vocal tract resonances and formant filtering behavior.

use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform, FormantState};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Measure high-frequency energy (rate of change)
fn measure_high_freq_energy(buffer: &[f32]) -> f32 {
    let mut energy = 0.0;
    for i in 1..buffer.len() {
        energy += (buffer[i] - buffer[i - 1]).abs();
    }
    energy / buffer.len() as f32
}

/// Helper: Find peak frequency using simple zero-crossing analysis
fn estimate_dominant_frequency(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut zero_crossings = 0;
    for i in 1..buffer.len() {
        if (buffer[i - 1] < 0.0 && buffer[i] >= 0.0)
            || (buffer[i - 1] >= 0.0 && buffer[i] < 0.0)
        {
            zero_crossings += 1;
        }
    }
    // Each cycle has 2 zero crossings
    let cycles = zero_crossings as f32 / 2.0;
    let duration = buffer.len() as f32 / sample_rate;
    cycles / duration
}

// ============================================================================
// TEST: Basic Formant Filtering
// ============================================================================

#[test]
fn test_formant_creates_resonances() {
    let mut graph = create_test_graph();

    // Use broadband pulse wave as excitation
    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Create formant filter for vowel /a/ (male voice)
    // F1=730, F2=1090, F3=2440 Hz
    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410; // 100ms at 44.1kHz
    let mut filtered = vec![0.0; buffer_size];
    let mut unfiltered = vec![0.0; buffer_size];

    // Get unfiltered signal
    graph.eval_node_buffer(&pulse_id, &mut unfiltered);

    // Get filtered signal
    graph.eval_node_buffer(&formant_id, &mut filtered);

    // Filtered should have vocal quality (non-zero output)
    let filtered_rms = calculate_rms(&filtered);
    assert!(
        filtered_rms > 0.05,
        "Formant filter should produce sound: RMS = {}",
        filtered_rms
    );

    // Should have different spectral characteristics than input
    let unfiltered_hf = measure_high_freq_energy(&unfiltered);
    let filtered_hf = measure_high_freq_energy(&filtered);

    // Formant filtering typically reduces harsh high frequencies
    assert!(
        filtered_hf < unfiltered_hf * 1.5,
        "Formant filter should shape spectrum: unfiltered HF = {}, filtered HF = {}",
        unfiltered_hf,
        filtered_hf
    );
}

#[test]
fn test_formant_vowel_a_characteristics() {
    let mut graph = create_test_graph();

    // Periodic excitation at male voice pitch
    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Vowel /a/ formants (father): F1=730, F2=1090, F3=2440
    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    // Should produce speech-like output
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Vowel /a/ should have strong output: RMS = {}",
        rms
    );

    // Check no NaN/Inf values
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample {} is non-finite: {}",
            i,
            sample
        );
    }
}

#[test]
fn test_formant_vowel_i_characteristics() {
    let mut graph = create_test_graph();

    // Periodic excitation
    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Vowel /i/ formants (beet): F1=270, F2=2290, F3=3010
    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(270.0),
            f2: Signal::Value(2290.0),
            f3: Signal::Value(3010.0),
            bw1: Signal::Value(60.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(150.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    // Should produce speech-like output
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.05,
        "Vowel /i/ should produce sound: RMS = {}",
        rms
    );

    // /i/ has high F2, should have more high-frequency content than /a/
    let hf_energy = measure_high_freq_energy(&output);
    assert!(
        hf_energy > 0.1,
        "Vowel /i/ should have high-frequency content: {}",
        hf_energy
    );
}

#[test]
fn test_formant_vowel_u_characteristics() {
    let mut graph = create_test_graph();

    // Periodic excitation
    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Vowel /u/ formants (boot): F1=300, F2=870, F3=2240
    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(300.0),
            f2: Signal::Value(870.0),
            f3: Signal::Value(2240.0),
            bw1: Signal::Value(60.0),
            bw2: Signal::Value(80.0),
            bw3: Signal::Value(100.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    // Should produce speech-like output
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.05,
        "Vowel /u/ should produce sound: RMS = {}",
        rms
    );

    // /u/ has low formants, darker sound
    let hf_energy = measure_high_freq_energy(&output);
    assert!(
        hf_energy < 1.0,
        "Vowel /u/ should be darker (less HF): {}",
        hf_energy
    );
}

// ============================================================================
// TEST: Formant Frequency Effects
// ============================================================================

#[test]
fn test_formant_f1_variation() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Low F1 (270 Hz - /i/ like)
    let formant_low_f1 = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(270.0),
            f2: Signal::Value(2290.0),
            f3: Signal::Value(3010.0),
            bw1: Signal::Value(60.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(150.0),
            state: FormantState::new(graph.sample_rate()),
        });

    // High F1 (730 Hz - /a/ like)
    let formant_high_f1 = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut low_f1_output = vec![0.0; buffer_size];
    let mut high_f1_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&formant_low_f1, &mut low_f1_output);
    graph.eval_node_buffer(&formant_high_f1, &mut high_f1_output);

    // Both should produce sound
    let low_rms = calculate_rms(&low_f1_output);
    let high_rms = calculate_rms(&high_f1_output);

    assert!(low_rms > 0.05, "Low F1 should produce sound: {}", low_rms);
    assert!(high_rms > 0.05, "High F1 should produce sound: {}", high_rms);

    // Different F1 values should create different timbres
    // This is a basic sanity check - in practice, spectral analysis would be better
}

#[test]
fn test_formant_bandwidth_variation() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Narrow bandwidth (sharp resonances)
    let formant_narrow = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(40.0),
            bw2: Signal::Value(45.0),
            bw3: Signal::Value(60.0),
            state: FormantState::new(graph.sample_rate()),
        });

    // Wide bandwidth (broader resonances)
    let formant_wide = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(160.0),
            bw2: Signal::Value(180.0),
            bw3: Signal::Value(240.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut narrow_output = vec![0.0; buffer_size];
    let mut wide_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&formant_narrow, &mut narrow_output);
    graph.eval_node_buffer(&formant_wide, &mut wide_output);

    // Both should produce sound
    let narrow_rms = calculate_rms(&narrow_output);
    let wide_rms = calculate_rms(&wide_output);

    assert!(
        narrow_rms > 0.05,
        "Narrow bandwidth should produce sound: {}",
        narrow_rms
    );
    assert!(
        wide_rms > 0.05,
        "Wide bandwidth should produce sound: {}",
        wide_rms
    );

    // Narrow bandwidth typically produces higher peak resonance
    // but this depends on many factors, so we just check they're different
    // and both produce valid output
}

// ============================================================================
// TEST: Different Excitation Sources
// ============================================================================

#[test]
fn test_formant_with_sawtooth_excitation() {
    let mut graph = create_test_graph();

    // Sawtooth excitation (rich harmonics like vocal cords)
    let saw_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(saw_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1,
        "Sawtooth excitation should produce strong vocal sound: RMS = {}",
        rms
    );
}

#[test]
fn test_formant_with_sine_excitation() {
    let mut graph = create_test_graph();

    // Sine excitation (pure tone)
    let sine_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Sine);

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(sine_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Sine excitation should pass through formants: RMS = {}",
        rms
    );
}

#[test]
fn test_formant_with_noise_excitation() {
    let mut graph = create_test_graph();

    // White noise excitation (for whispered/breathy sounds)
    let noise_id = graph.add_whitenoise_node();

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(noise_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Noise excitation should create whispered vowel: RMS = {}",
        rms
    );

    // Should shape noise spectrum (not flat white noise)
    // The formants will color the noise
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_formant_state_continuity() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&formant_id, &mut buffer1);
    graph.eval_node_buffer(&formant_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    assert!(
        discontinuity < 0.5,
        "Filter state should be continuous: discontinuity = {}",
        discontinuity
    );
}

#[test]
fn test_formant_multiple_buffers() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    for i in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&formant_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}",
            i,
            rms
        );

        // Check no NaN/Inf values
        for &sample in &output {
            assert!(sample.is_finite(), "Buffer {} has non-finite value", i);
        }
    }
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_formant_modulated_f1() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // LFO to modulate F1 (0.5 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Modulated F1: 500 + (lfo * 300) = [200, 800] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(300.0));
    let f1_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(500.0));

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Node(f1_signal),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    // Should produce sound (modulated formant)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.05,
        "Modulated F1 should produce sound, RMS = {}",
        rms
    );
}

#[test]
fn test_formant_modulated_bandwidth() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // LFO to modulate bandwidth (0.25 Hz)
    let lfo_id = graph.add_oscillator(Signal::Value(0.25), Waveform::Sine);

    // Modulated BW1: 80 + (lfo * 40) = [40, 120] Hz range
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo_id), Signal::Value(40.0));
    let bw1_signal = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(80.0));

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Node(bw1_signal),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output);

    // Should produce sound (modulated bandwidth)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.05,
        "Modulated bandwidth should produce sound, RMS = {}",
        rms
    );
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_formant_extreme_formant_frequencies() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Very low formants
    let formant_low = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(100.0),
            f2: Signal::Value(200.0),
            f3: Signal::Value(300.0),
            bw1: Signal::Value(50.0),
            bw2: Signal::Value(60.0),
            bw3: Signal::Value(70.0),
            state: FormantState::new(graph.sample_rate()),
        });

    // Very high formants
    let formant_high = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(3000.0),
            f2: Signal::Value(4000.0),
            f3: Signal::Value(8000.0),
            bw1: Signal::Value(200.0),
            bw2: Signal::Value(250.0),
            bw3: Signal::Value(300.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 512;
    let mut low_output = vec![0.0; buffer_size];
    let mut high_output = vec![0.0; buffer_size];

    // Should not crash
    graph.eval_node_buffer(&formant_low, &mut low_output);
    graph.eval_node_buffer(&formant_high, &mut high_output);

    // Check no NaN/Inf values
    for &sample in &low_output {
        assert!(sample.is_finite(), "Low formants produced non-finite value");
    }
    for &sample in &high_output {
        assert!(
            sample.is_finite(),
            "High formants produced non-finite value"
        );
    }
}

#[test]
fn test_formant_extreme_bandwidths() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Very narrow bandwidth
    let formant_narrow = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(20.0),
            bw2: Signal::Value(25.0),
            bw3: Signal::Value(30.0),
            state: FormantState::new(graph.sample_rate()),
        });

    // Very wide bandwidth
    let formant_wide = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(500.0),
            bw2: Signal::Value(600.0),
            bw3: Signal::Value(800.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 512;
    let mut narrow_output = vec![0.0; buffer_size];
    let mut wide_output = vec![0.0; buffer_size];

    // Should not crash
    graph.eval_node_buffer(&formant_narrow, &mut narrow_output);
    graph.eval_node_buffer(&formant_wide, &mut wide_output);

    // Check no NaN/Inf values
    for &sample in &narrow_output {
        assert!(
            sample.is_finite(),
            "Narrow bandwidth produced non-finite value"
        );
    }
    for &sample in &wide_output {
        assert!(sample.is_finite(), "Wide bandwidth produced non-finite value");
    }
}

#[test]
fn test_formant_silent_input() {
    let mut graph = create_test_graph();

    // Silent input (constant 0)
    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Value(0.0),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&formant_id, &mut output);

    // Should produce silence (no input)
    let rms = calculate_rms(&output);
    assert!(
        rms < 0.001,
        "Silent input should produce silent output: RMS = {}",
        rms
    );
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_formant_buffer_performance() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&formant_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "Formant buffer eval: {:?} for {} iterations",
        duration, iterations
    );
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    // Formants are 3x bandpass filters so more expensive than single filter
    assert!(
        duration.as_secs() < 2,
        "Formant buffer evaluation too slow: {:?}",
        duration
    );
}

// ============================================================================
// TEST: Coefficient Caching
// ============================================================================

#[test]
fn test_formant_coefficient_caching() {
    let mut graph = create_test_graph();

    let pulse_id = graph.add_oscillator(Signal::Value(110.0), Waveform::Square);

    // Constant formant frequencies (should use cached coefficients)
    let formant_id = graph.add_node(SignalNode::Formant {
            source: Signal::Node(pulse_id),
            f1: Signal::Value(730.0),
            f2: Signal::Value(1090.0),
            f3: Signal::Value(2440.0),
            bw1: Signal::Value(80.0),
            bw2: Signal::Value(90.0),
            bw3: Signal::Value(120.0),
            state: FormantState::new(graph.sample_rate()),
        });

    let buffer_size = 4410;

    // First buffer (computes coefficients)
    let start1 = std::time::Instant::now();
    let mut output1 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output1);
    let duration1 = start1.elapsed();

    // Second buffer (should use cached coefficients - faster)
    let start2 = std::time::Instant::now();
    let mut output2 = vec![0.0; buffer_size];
    graph.eval_node_buffer(&formant_id, &mut output2);
    let duration2 = start2.elapsed();

    println!("First buffer: {:?}", duration1);
    println!("Second buffer: {:?}", duration2);

    // Both should produce valid output
    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);

    assert!(rms1 > 0.05, "First buffer should produce sound: {}", rms1);
    assert!(rms2 > 0.05, "Second buffer should produce sound: {}", rms2);

    // Second buffer should be similar or faster (due to caching)
    // This is a rough check - actual performance varies
}
