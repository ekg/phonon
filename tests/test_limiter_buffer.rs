/// Tests for Limiter buffer-based evaluation
///
/// These tests verify that the lookahead Limiter produces correct
/// dynamics limiting behavior and maintains proper state continuity.
use phonon::unified_graph::{LimiterState, Signal, SignalNode, UnifiedSignalGraph, Waveform};

/// Helper: Create a test graph
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Calculate peak value in a buffer
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max)
}

/// Helper: Generate a loud signal (above threshold)
fn generate_loud_signal(graph: &mut UnifiedSignalGraph, amplitude: f32) -> Signal {
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let scaled = graph.add_multiply_node(Signal::Node(osc), Signal::Value(amplitude));
    Signal::Node(scaled)
}

/// Helper: Generate a quiet signal (below threshold)
fn generate_quiet_signal(graph: &mut UnifiedSignalGraph, amplitude: f32) -> Signal {
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let scaled = graph.add_multiply_node(Signal::Node(osc), Signal::Value(amplitude));
    Signal::Node(scaled)
}

/// Helper: Create a limiter node with default attack/release
fn add_limiter(
    graph: &mut UnifiedSignalGraph,
    input: Signal,
    threshold: f32,
) -> phonon::unified_graph::NodeId {
    let lookahead_samples = (0.005 * 44100.0) as usize; // 5ms
    graph.add_node(SignalNode::Limiter {
        input,
        threshold: Signal::Value(threshold),
        attack: Signal::Value(0.005),
        release: Signal::Value(0.05),
        state: LimiterState::new(lookahead_samples),
    })
}

// ============================================================================
// LEVEL 1: Pattern Query / Compilation Verification
// ============================================================================

#[test]
fn test_limiter_node_creation() {
    let mut graph = create_test_graph();
    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let limiter_id = add_limiter(&mut graph, loud_signal, 0.5);

    // Should produce a valid node
    let mut output = vec![0.0; 512];
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Should produce non-silence
    let rms = calculate_rms(&output);
    assert!(
        rms > 0.01,
        "Limiter should produce audible output, got RMS {}",
        rms
    );
}

#[test]
fn test_limiter_with_signal_threshold() {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    // Test via DSL for reliable signal routing
    let dsl = r#"
tempo: 1.0
~lfo $ sine 2 * 0.2 + 0.6
~hot $ sine 440 * 0.8
~limited $ limiter ~hot ~lfo
out $ ~limited
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0, None).unwrap();
    let samples = graph.render(4410); // 100ms

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.01,
        "Modulated threshold limiter should produce audible output, RMS = {}",
        rms
    );
}

// ============================================================================
// LEVEL 2: Audio Characteristics - Peak Limiting
// ============================================================================

#[test]
fn test_limiter_reduces_loud_signals() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let limiter_id = add_limiter(&mut graph, loud_signal.clone(), 0.3);

    let buffer_size = 4410; // 100ms at 44.1kHz
    let mut limited = vec![0.0; buffer_size];
    let mut unlimited = vec![0.0; buffer_size];

    // Get unlimited signal
    if let Signal::Node(sig_id) = loud_signal {
        graph.eval_node_buffer(&sig_id, &mut unlimited);
    }

    // Get limited signal
    graph.eval_node_buffer(&limiter_id, &mut limited);

    let unlimited_peak = calculate_peak(&unlimited);
    let limited_peak = calculate_peak(&limited);

    println!(
        "Unlimited peak: {}, Limited peak: {}",
        unlimited_peak, limited_peak
    );

    // Limited signal should have lower peak
    assert!(
        limited_peak < unlimited_peak,
        "Limiter should reduce loud signals: unlimited peak = {}, limited peak = {}",
        unlimited_peak,
        limited_peak
    );

    // Limited peak should be near threshold (0.3) with some tolerance
    assert!(
        limited_peak < 0.35,
        "Limited peak should be near threshold 0.3, got {}",
        limited_peak
    );
}

#[test]
fn test_limiter_passes_quiet_signals() {
    let mut graph = create_test_graph();

    let quiet_signal = generate_quiet_signal(&mut graph, 0.1);
    let limiter_id = add_limiter(&mut graph, quiet_signal.clone(), 0.8);

    let buffer_size = 4410;
    let mut limited = vec![0.0; buffer_size];
    let mut unlimited = vec![0.0; buffer_size];

    if let Signal::Node(sig_id) = quiet_signal {
        graph.eval_node_buffer(&sig_id, &mut unlimited);
    }

    graph.eval_node_buffer(&limiter_id, &mut limited);

    let unlimited_rms = calculate_rms(&unlimited);
    let limited_rms = calculate_rms(&limited);

    // Quiet signals should pass through with similar RMS
    // (delay causes phase shift but not amplitude change)
    let rms_diff = (limited_rms - unlimited_rms).abs();
    assert!(
        rms_diff < unlimited_rms * 0.15,
        "Quiet signals should pass unchanged: unlimited RMS = {}, limited RMS = {}, diff = {}",
        unlimited_rms,
        limited_rms,
        rms_diff
    );
}

#[test]
fn test_limiter_threshold_effect() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // High threshold (0.7) - less limiting
    let limiter_high = add_limiter(&mut graph, loud_signal.clone(), 0.7);

    // Low threshold (0.2) - more limiting
    let limiter_low = add_limiter(&mut graph, loud_signal, 0.2);

    let buffer_size = 4410;
    let mut high_thresh_output = vec![0.0; buffer_size];
    let mut low_thresh_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&limiter_high, &mut high_thresh_output);
    graph.eval_node_buffer(&limiter_low, &mut low_thresh_output);

    let high_thresh_rms = calculate_rms(&high_thresh_output);
    let low_thresh_rms = calculate_rms(&low_thresh_output);

    println!(
        "High threshold RMS: {}, Low threshold RMS: {}",
        high_thresh_rms, low_thresh_rms
    );

    // Lower threshold should produce lower RMS (more limiting)
    assert!(
        low_thresh_rms < high_thresh_rms,
        "Lower threshold should limit more: high thresh RMS = {}, low thresh RMS = {}",
        high_thresh_rms,
        low_thresh_rms
    );
}

#[test]
fn test_limiter_peak_ceiling() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 1.5);
    let limiter_id = add_limiter(&mut graph, loud_signal, 0.5);

    // Render enough to get past initial transient
    let buffer_size = 8820; // 200ms
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Skip first 5ms (lookahead fill time) and check peaks
    let skip_samples = (0.005 * 44100.0) as usize;
    let peak = calculate_peak(&output[skip_samples..]);

    println!("Peak after lookahead fill: {}", peak);

    // Peak should not significantly exceed threshold
    assert!(
        peak < 0.55,
        "Limiter should keep peaks near threshold 0.5, got {}",
        peak
    );
}

// ============================================================================
// LEVEL 2: Release Behavior
// ============================================================================

#[test]
fn test_limiter_release_time() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);

    // Fast release (10ms)
    let lookahead = (0.005 * 44100.0) as usize;
    let limiter_fast = graph.add_node(SignalNode::Limiter {
        input: loud_signal.clone(),
        threshold: Signal::Value(0.3),
        attack: Signal::Value(0.005),
        release: Signal::Value(0.01),
        state: LimiterState::new(lookahead),
    });

    // Slow release (500ms)
    let limiter_slow = graph.add_node(SignalNode::Limiter {
        input: loud_signal,
        threshold: Signal::Value(0.3),
        attack: Signal::Value(0.005),
        release: Signal::Value(0.5),
        state: LimiterState::new(lookahead),
    });

    let buffer_size = 4410;
    let mut fast_output = vec![0.0; buffer_size];
    let mut slow_output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&limiter_fast, &mut fast_output);
    graph.eval_node_buffer(&limiter_slow, &mut slow_output);

    // Both should produce sound
    let fast_rms = calculate_rms(&fast_output);
    let slow_rms = calculate_rms(&slow_output);

    assert!(
        fast_rms > 0.01 && slow_rms > 0.01,
        "Both fast and slow release should produce sound: fast = {}, slow = {}",
        fast_rms,
        slow_rms
    );
}

// ============================================================================
// LEVEL 3: State Continuity Across Buffers
// ============================================================================

#[test]
fn test_limiter_state_continuity() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let limiter_id = add_limiter(&mut graph, loud_signal, 0.5);

    // Generate two consecutive buffers
    let buffer_size = 512;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&limiter_id, &mut buffer1);
    graph.eval_node_buffer(&limiter_id, &mut buffer2);

    // Check continuity at boundary
    let last_sample = buffer1[buffer_size - 1];
    let first_sample = buffer2[0];
    let discontinuity = (first_sample - last_sample).abs();

    println!(
        "Buffer boundary: last={}, first={}, discontinuity={}",
        last_sample, first_sample, discontinuity
    );

    // Should be reasonably continuous (no massive clicks)
    assert!(
        discontinuity < 0.15,
        "Limiter state should be continuous across buffers, discontinuity = {}",
        discontinuity
    );
}

#[test]
fn test_limiter_multiple_buffers() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let limiter_id = add_limiter(&mut graph, loud_signal, 0.5);

    // Generate 10 consecutive buffers
    let buffer_size = 512;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&limiter_id, &mut output);

        // Each buffer should have reasonable audio
        let rms = calculate_rms(&output);
        assert!(
            rms > 0.01 && rms < 1.0,
            "Buffer {} has unexpected RMS: {}",
            i,
            rms
        );

        // Check for no NaN/Inf
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
// LEVEL 3: Edge Cases
// ============================================================================

#[test]
fn test_limiter_zero_input() {
    let mut graph = create_test_graph();

    let lookahead = (0.005 * 44100.0) as usize;
    let limiter_id = graph.add_node(SignalNode::Limiter {
        input: Signal::Value(0.0),
        threshold: Signal::Value(0.5),
        attack: Signal::Value(0.005),
        release: Signal::Value(0.05),
        state: LimiterState::new(lookahead),
    });

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Should produce silence (no division by zero, no NaN)
    for (i, &sample) in output.iter().enumerate() {
        assert!(sample.is_finite(), "Sample {} is non-finite: {}", i, sample);
        assert_eq!(sample, 0.0, "Sample {} should be silent, got {}", i, sample);
    }
}

#[test]
fn test_limiter_very_hot_signal() {
    let mut graph = create_test_graph();

    // Extremely loud signal (10x)
    let hot_signal = generate_loud_signal(&mut graph, 10.0);
    let limiter_id = add_limiter(&mut graph, hot_signal, 0.5);

    let buffer_size = 4410; // 100ms
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Skip lookahead fill, check peaks
    let skip = (0.005 * 44100.0) as usize;
    let peak = calculate_peak(&output[skip..]);

    println!("Peak with very hot signal: {}", peak);

    // Even with extreme input, peak should be controlled
    assert!(
        peak < 0.6,
        "Limiter should control even very hot signals, peak = {}",
        peak
    );

    // Should not have NaN/Inf
    for (i, &s) in output.iter().enumerate() {
        assert!(s.is_finite(), "Sample {} is non-finite: {}", i, s);
    }
}

#[test]
fn test_limiter_threshold_near_zero() {
    let mut graph = create_test_graph();

    let signal = generate_loud_signal(&mut graph, 0.5);
    let limiter_id = add_limiter(&mut graph, signal, 0.01);

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&limiter_id, &mut output);

    // Very low threshold should produce very quiet output
    let rms = calculate_rms(&output);
    assert!(
        rms < 0.05,
        "Very low threshold should produce quiet output, RMS = {}",
        rms
    );

    // No NaN/Inf
    for (i, &s) in output.iter().enumerate() {
        assert!(s.is_finite(), "Sample {} is non-finite: {}", i, s);
    }
}

#[test]
fn test_limiter_threshold_above_signal() {
    let mut graph = create_test_graph();

    // Signal amplitude 0.3, threshold 0.8 - no limiting should occur
    let quiet_signal = generate_quiet_signal(&mut graph, 0.3);
    let limiter_id = add_limiter(&mut graph, quiet_signal, 0.8);

    let buffer_size = 4410;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&limiter_id, &mut output);

    let peak = calculate_peak(&output);
    let rms = calculate_rms(&output);

    println!(
        "Below-threshold: peak = {}, RMS = {}",
        peak, rms
    );

    // Peak should be approximately the signal amplitude (no limiting)
    assert!(
        peak > 0.2,
        "Signal below threshold should pass through, peak = {}",
        peak
    );
    assert!(
        rms > 0.1,
        "Signal below threshold should be audible, RMS = {}",
        rms
    );
}

// ============================================================================
// LEVEL 3: Performance
// ============================================================================

#[test]
fn test_limiter_buffer_performance() {
    let mut graph = create_test_graph();

    let loud_signal = generate_loud_signal(&mut graph, 0.8);
    let limiter_id = add_limiter(&mut graph, loud_signal, 0.5);

    let buffer_size = 512;
    let iterations = 1000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut output = vec![0.0; buffer_size];
        graph.eval_node_buffer(&limiter_id, &mut output);
    }
    let duration = start.elapsed();

    println!(
        "Limiter buffer eval: {:?} for {} iterations",
        duration, iterations
    );
    println!("Per iteration: {:?}", duration / iterations);

    // Should complete in reasonable time (< 10 seconds for 1000 iterations)
    assert!(
        duration.as_secs() < 10,
        "Limiter buffer evaluation too slow: {:?}",
        duration
    );
}

// ============================================================================
// LEVEL 3: DSL Integration (chained usage)
// ============================================================================

#[test]
fn test_limiter_chained_dsl() {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    let dsl = r#"
tempo: 1.0
~hot $ saw 220 * 1.5
out $ ~hot # limiter 0.6
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0, None).unwrap();
    let samples = graph.render(4410);

    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = calculate_rms(&samples);

    println!("Chained limiter: peak = {}, RMS = {}", peak, rms);

    assert!(
        rms > 0.05,
        "Chained limiter should produce audible output, RMS = {}",
        rms
    );

    // Peak should be controlled near threshold
    assert!(
        peak < 0.7,
        "Chained limiter should control peaks near 0.6, got {}",
        peak
    );
}

#[test]
fn test_limiter_with_custom_attack_release() {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    let dsl = r#"
tempo: 1.0
~hot $ sine 440 * 2.0
~limited $ limiter ~hot 0.5 0.01 0.1
out $ ~limited
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0, None).unwrap();
    let samples = graph.render(4410);

    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = calculate_rms(&samples);

    println!(
        "Custom attack/release limiter: peak = {}, RMS = {}",
        peak, rms
    );

    assert!(
        rms > 0.05,
        "Custom limiter should produce audible output, RMS = {}",
        rms
    );
}
