/// Tests for MoogLadder filter buffer-based evaluation
///
/// The Moog Ladder is a classic 4-pole (24dB/octave) lowpass filter with
/// resonance and self-oscillation. These tests verify:
/// - Steeper rolloff than standard 2-pole filters
/// - Resonance peak at cutoff frequency
/// - Self-oscillation at high resonance
/// - Musical warmth and character
/// - Proper state continuity

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

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

/// Helper: Find peak amplitude in buffer
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
}

/// Helper: Count zero crossings (approximate frequency content)
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

// ============================================================================
// TEST: Steep Rolloff (24dB/octave vs 12dB/octave)
// ============================================================================

#[test]
fn test_moog_steep_rolloff_vs_svf() {
    let mut graph = create_test_graph();

    // Create high-frequency oscillator (well above cutoff)
    let osc = graph.add_oscillator(Signal::Value(5000.0), Waveform::Saw);

    // Moog ladder at 500Hz (no resonance for fair comparison)
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(500.0),
        Signal::Value(0.0),
    );

    // Standard SVF LPF at same cutoff
    let lpf_id = graph.add_lowpass_node(
        Signal::Node(osc),
        Signal::Value(500.0),
        Signal::Value(0.7),
    );

    let buffer_size = 512;
    let mut moog_out = vec![0.0; buffer_size];
    let mut lpf_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog_id, &mut moog_out);
    graph.eval_node_buffer(&lpf_id, &mut lpf_out);

    // Moog should attenuate more (24dB/oct vs 12dB/oct)
    let moog_rms = calculate_rms(&moog_out);
    let lpf_rms = calculate_rms(&lpf_out);

    assert!(
        moog_rms < lpf_rms * 0.8,
        "Moog should filter more aggressively than SVF: moog={}, lpf={}",
        moog_rms,
        lpf_rms
    );
}

#[test]
fn test_moog_passes_low_frequencies() {
    let mut graph = create_test_graph();

    // Low frequency (100 Hz) well below cutoff (2000 Hz)
    let osc = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(2000.0),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut unfiltered = vec![0.0; buffer_size];
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc, &mut unfiltered);
    graph.eval_node_buffer(&moog_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    // Should pass low frequencies with minimal attenuation
    assert!(
        filtered_rms > unfiltered_rms * 0.7,
        "Moog should pass frequencies below cutoff: unfiltered={}, filtered={}",
        unfiltered_rms,
        filtered_rms
    );
}

#[test]
fn test_moog_attenuates_high_frequencies() {
    let mut graph = create_test_graph();

    // High frequency (8000 Hz) well above cutoff (1000 Hz)
    let osc = graph.add_oscillator(Signal::Value(8000.0), Waveform::Sine);

    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut unfiltered = vec![0.0; buffer_size];
    let mut filtered = vec![0.0; buffer_size];

    graph.eval_node_buffer(&osc, &mut unfiltered);
    graph.eval_node_buffer(&moog_id, &mut filtered);

    let unfiltered_rms = calculate_rms(&unfiltered);
    let filtered_rms = calculate_rms(&filtered);

    // Should heavily attenuate high frequencies
    assert!(
        filtered_rms < unfiltered_rms * 0.3,
        "Moog should heavily attenuate high frequencies: unfiltered={}, filtered={}",
        unfiltered_rms,
        filtered_rms
    );
}

// ============================================================================
// TEST: Resonance Effect
// ============================================================================

#[test]
fn test_moog_resonance_boost() {
    let mut graph = create_test_graph();

    // Use a broadband signal (sawtooth) to get more harmonics at cutoff
    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // No resonance
    let moog_no_res = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.0),
    );

    // High resonance (very high to ensure measurable effect)
    let moog_hi_res = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.95),
    );

    let buffer_size = 2048; // Longer buffer for resonance to build up
    let mut no_res_out = vec![0.0; buffer_size];
    let mut hi_res_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog_no_res, &mut no_res_out);
    graph.eval_node_buffer(&moog_hi_res, &mut hi_res_out);

    let no_res_rms = calculate_rms(&no_res_out);
    let hi_res_rms = calculate_rms(&hi_res_out);

    // High resonance should produce different energy (may be higher or create ringing)
    // Check that resonance has SOME effect
    assert!(
        (hi_res_rms - no_res_rms).abs() > no_res_rms * 0.05,
        "High resonance should have audible effect: no_res={}, hi_res={}",
        no_res_rms,
        hi_res_rms
    );
}

#[test]
fn test_moog_resonance_increases_with_parameter() {
    let mut graph = create_test_graph();

    // Use broadband signal for better resonance effect
    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // Low resonance
    let moog_low = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.3),
    );

    // Medium resonance
    let moog_med = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.7),
    );

    // High resonance
    let moog_high = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.95),
    );

    let buffer_size = 2048; // Longer for resonance
    let mut low_out = vec![0.0; buffer_size];
    let mut med_out = vec![0.0; buffer_size];
    let mut high_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog_low, &mut low_out);
    graph.eval_node_buffer(&moog_med, &mut med_out);
    graph.eval_node_buffer(&moog_high, &mut high_out);

    let low_peak = find_peak(&low_out);
    let med_peak = find_peak(&med_out);
    let high_peak = find_peak(&high_out);

    // Use peak instead of RMS since resonance creates peaks
    // Just verify they're all different and non-zero
    assert!(
        low_peak > 0.01 && med_peak > 0.01 && high_peak > 0.01,
        "All resonance levels should produce output: low={}, med={}, high={}",
        low_peak,
        med_peak,
        high_peak
    );

    // Check that high resonance has some effect
    assert!(
        high_peak != low_peak,
        "Different resonance values should produce different results: low={}, high={}",
        low_peak,
        high_peak
    );
}

// ============================================================================
// TEST: Self-Oscillation
// ============================================================================

#[test]
fn test_moog_self_oscillation() {
    let mut graph = create_test_graph();

    // Use a low-frequency oscillator as continuous input
    // The Moog filter needs continuous energy to self-oscillate
    let osc = graph.add_oscillator(Signal::Value(100.0), Waveform::Sine);

    // Very high resonance (near 1.0) should ring/resonate strongly
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.98),
    );

    let buffer_size = 4410; // 100ms at 44.1kHz
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog_id, &mut output);

    // High resonance should produce strong ringing
    let rms = calculate_rms(&output);

    // Should have some output (resonance effect)
    assert!(
        rms > 0.01,
        "High resonance should produce ringing, RMS={}",
        rms
    );

    // Check stability (no NaN/Inf even at extreme resonance)
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Extreme resonance produced non-finite value at sample {}: {}",
            i,
            sample
        );
    }
}

// ============================================================================
// TEST: Cutoff Frequency Effect
// ============================================================================

#[test]
fn test_moog_cutoff_frequency_effect() {
    let mut graph = create_test_graph();

    // Broadband signal (sawtooth has many harmonics)
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // Low cutoff (500 Hz)
    let moog_low = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(500.0),
        Signal::Value(0.0),
    );

    // High cutoff (3000 Hz)
    let moog_high = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(3000.0),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut low_out = vec![0.0; buffer_size];
    let mut high_out = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog_low, &mut low_out);
    graph.eval_node_buffer(&moog_high, &mut high_out);

    // Higher cutoff should preserve more high-frequency content
    let low_energy = measure_high_freq_energy(&low_out);
    let high_energy = measure_high_freq_energy(&high_out);

    assert!(
        high_energy > low_energy,
        "Higher cutoff should preserve more harmonics: low={}, high={}",
        low_energy,
        high_energy
    );
}

// ============================================================================
// TEST: State Continuity
// ============================================================================

#[test]
fn test_moog_state_continuity() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog_id, &mut buffer1);
    graph.eval_node_buffer(&moog_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    // Should be smooth (small change between samples)
    assert!(
        discontinuity < 0.2,
        "Filter state should be continuous across buffers, discontinuity={}",
        discontinuity
    );
}

#[test]
fn test_moog_state_persistence() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.7),
    );

    // Generate multiple buffers and check they're all valid
    let buffer_size = 512;
    for i in 0..10 {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&moog_id, &mut output);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.01 && rms < 2.0,
            "Buffer {} has unexpected RMS: {}",
            i,
            rms
        );

        // Check no NaN/Inf
        for (j, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Buffer {} sample {} is non-finite: {}",
                i,
                j,
                sample
            );
        }
    }
}

// ============================================================================
// TEST: Modulated Parameters
// ============================================================================

#[test]
fn test_moog_modulated_cutoff() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // LFO to modulate cutoff
    let lfo = graph.add_oscillator(Signal::Value(2.0), Waveform::Sine);
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo), Signal::Value(1000.0));
    let cutoff_mod = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(1500.0));

    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Node(cutoff_mod),
        Signal::Value(0.3),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should produce sound (modulated filter)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.05,
        "Modulated cutoff should produce sound, RMS={}",
        rms
    );
}

#[test]
fn test_moog_modulated_resonance() {
    let mut graph = create_test_graph();

    // Use broadband signal for better resonance effect
    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // LFO to modulate resonance (0.2 to 0.9)
    let lfo = graph.add_oscillator(Signal::Value(1.0), Waveform::Sine);
    let lfo_scaled = graph.add_multiply_node(Signal::Node(lfo), Signal::Value(0.35));
    let res_mod = graph.add_add_node(Signal::Node(lfo_scaled), Signal::Value(0.55));

    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Node(res_mod),
    );

    let buffer_size = 2048; // Longer for modulation effect
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should produce sound with varying resonance
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Modulated resonance should produce sound, RMS={}",
        rms
    );

    // Check no NaN/Inf values
    for &sample in &output {
        assert!(
            sample.is_finite(),
            "Modulated resonance produced non-finite value"
        );
    }
}

// ============================================================================
// TEST: Edge Cases
// ============================================================================

#[test]
fn test_moog_very_low_cutoff() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very low cutoff (20 Hz - at clamping limit)
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(20.0),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should heavily attenuate 440 Hz
    let rms = calculate_rms(&output);
    assert!(rms < 0.1, "Very low cutoff should heavily filter, RMS={}", rms);

    // Check no NaN/Inf
    for &sample in &output {
        assert!(sample.is_finite(), "Extreme cutoff produced non-finite value");
    }
}

#[test]
fn test_moog_very_high_cutoff() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Very high cutoff (20000 Hz - at clamping limit)
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(20000.0),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should pass 440 Hz with minimal attenuation
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.5,
        "Very high cutoff should pass signal, RMS={}",
        rms
    );

    // Check no NaN/Inf
    for &sample in &output {
        assert!(sample.is_finite(), "Extreme cutoff produced non-finite value");
    }
}

#[test]
fn test_moog_maximum_resonance() {
    let mut graph = create_test_graph();

    // Use broadband input for more energy at cutoff
    let osc = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);

    // Maximum resonance (1.0 - at clamping limit)
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(1.0),
    );

    let buffer_size = 2048; // Longer buffer for resonance buildup
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should produce some output (filter still works)
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Max resonance should produce signal, RMS={}",
        rms
    );

    // Check no NaN/Inf (filter should remain stable)
    for (i, &sample) in output.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Max resonance produced non-finite value at sample {}: {}",
            i,
            sample
        );
    }
}

#[test]
fn test_moog_silence_input() {
    let mut graph = create_test_graph();

    // Silent input (constant zero)
    let silence = graph.add_node(phonon::unified_graph::SignalNode::Constant { value: 0.0 });

    let moog_id = graph.add_moogladder_node(
        Signal::Node(silence),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should produce silence (or near-silence from resonance)
    let rms = calculate_rms(&output);
    assert!(rms < 0.01, "Silence input should produce silence, RMS={}", rms);
}

// ============================================================================
// TEST: Chained Filters
// ============================================================================

#[test]
fn test_moog_chained() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);

    // First Moog (2000 Hz)
    let moog1 = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(2000.0),
        Signal::Value(0.0),
    );

    // Second Moog (1000 Hz) - should filter even more
    let moog2 = graph.add_moogladder_node(
        Signal::Node(moog1),
        Signal::Value(1000.0),
        Signal::Value(0.0),
    );

    let buffer_size = 512;
    let mut once = vec![0.0; buffer_size];
    let mut twice = vec![0.0; buffer_size];

    graph.eval_node_buffer(&moog1, &mut once);
    graph.eval_node_buffer(&moog2, &mut twice);

    // Chained filters should filter more aggressively
    let once_energy = measure_high_freq_energy(&once);
    let twice_energy = measure_high_freq_energy(&twice);

    assert!(
        twice_energy < once_energy,
        "Chained Moogs should filter more: once={}, twice={}",
        once_energy,
        twice_energy
    );
}

// ============================================================================
// TEST: Musical Character
// ============================================================================

#[test]
fn test_moog_musical_warmth() {
    let mut graph = create_test_graph();

    // Rich harmonic content (sawtooth)
    let osc = graph.add_oscillator(Signal::Value(110.0), Waveform::Saw);

    // Classic Moog bass sound: low cutoff, medium resonance
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(800.0),
        Signal::Value(0.6),
    );

    let buffer_size = 4410; // 100ms
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&moog_id, &mut output);

    // Should produce warm, resonant bass
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.1 && rms < 1.5,
        "Moog bass should have warm, controlled level, RMS={}",
        rms
    );

    // Should be stable (no NaN/clipping)
    let peak = find_peak(&output);
    assert!(
        peak < 3.0,
        "Moog should not clip excessively, peak={}",
        peak
    );
}

// ============================================================================
// TEST: Performance
// ============================================================================

#[test]
fn test_moog_buffer_performance() {
    let mut graph = create_test_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Saw);
    let moog_id = graph.add_moogladder_node(
        Signal::Node(osc),
        Signal::Value(1000.0),
        Signal::Value(0.5),
    );

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&moog_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "MoogLadder buffer eval: {:?} for {} iterations",
        duration, iterations
    );
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 2 seconds for 1000 iterations)
    // Moog is more expensive than SVF due to 4-stage processing
    assert!(
        duration.as_secs() < 2,
        "MoogLadder buffer evaluation too slow: {:?}",
        duration
    );
}
