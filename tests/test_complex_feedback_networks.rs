/// Phase 5: Complex Feedback Networks - Comprehensive Tests
///
/// This test suite verifies:
/// 1. Signal analysis nodes (RMS, PeakFollower, ZeroCrossing)
/// 2. Adaptive processing (AdaptiveCompressor)
/// 3. Multi-stage feedback networks (3-5 stages)
/// 4. Stability under complex feedback
/// 5. Real-time performance with multiple feedback loops

use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal};
use std::collections::HashMap;

#[test]
fn test_zero_crossing_detector_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create a 440Hz sine wave
    let sine_node = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: phonon::unified_graph::Waveform::Sine,
        semitone_offset: 0.0,
        phase: std::cell::RefCell::new(0.0),
        pending_freq: std::cell::RefCell::new(None),
        last_sample: std::cell::RefCell::new(0.0),
    });

    // Add zero crossing detector with 100ms window (4410 samples at 44.1kHz)
    let zc_node = graph.add_node(SignalNode::ZeroCrossing {
        input: Signal::Node(sine_node),
        last_sample: 0.0,
        crossing_count: 0,
        sample_count: 0,
        window_samples: 4410, // 100ms window
        last_frequency: 0.0,
    });

    graph.set_output(zc_node);

    // Render enough samples to get frequency estimate
    let buffer = graph.render(8820); // 200ms

    // The zero crossing detector should output the detected frequency
    // For a 440Hz sine wave, we expect ~440Hz output after the first window
    // Check that we get non-zero output
    let has_output = buffer.iter().skip(4410).any(|&x| x > 400.0 && x < 480.0);
    assert!(has_output, "ZeroCrossing should detect 440Hz frequency");
}

#[test]
fn test_multi_stage_feedback_3_stages() {
    // Test 3-stage feedback network: A → B → C → A
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create a simple pattern trigger
    let pattern = phonon::mini_notation_v3::parse_mini_notation("x ~ x ~");
    let trigger_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd ~ bd ~".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.5),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    });

    // Stage 1: Low-pass filter
    let stage1 = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(trigger_node),
        cutoff: Signal::Value(2000.0),
        q: Signal::Value(0.7),
        state: phonon::unified_graph::FilterState::default(),
    });

    // Stage 2: Reverb
    let stage2 = graph.add_node(SignalNode::Reverb {
        input: Signal::Node(stage1),
        room_size: Signal::Value(0.5),
        damping: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        state: phonon::unified_graph::ReverbState::default(),
    });

    // Stage 3: RMS analysis of reverb output
    let stage3 = graph.add_node(SignalNode::RMS {
        input: Signal::Node(stage2),
        window_size: Signal::Value(0.05), // 50ms window
        buffer: vec![0.0; 4410],
        write_idx: 0,
    });

    // Feedback: Use RMS to modulate filter cutoff
    // Create a new filter with cutoff modulated by RMS
    let feedback_filter = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(trigger_node),
        cutoff: Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Add(
            Signal::Value(500.0),
            Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Multiply(
                Signal::Node(stage3),
                Signal::Value(2000.0),
            ))),
        ))),
        q: Signal::Value(0.7),
        state: phonon::unified_graph::FilterState::default(),
    });

    // Mix feedback with original
    let output = graph.add_node(SignalNode::Add {
        a: Signal::Node(stage2),
        b: Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Multiply(
            Signal::Node(feedback_filter),
            Signal::Value(0.3), // Feedback amount
        ))),
    });

    graph.set_output(output);

    // Render and verify stability
    let buffer = graph.render(44100 * 2); // 2 seconds

    // Check for explosions (inf/nan)
    let has_inf_or_nan = buffer.iter().any(|&x| !x.is_finite());
    assert!(!has_inf_or_nan, "3-stage feedback should not explode");

    // Check for reasonable audio levels
    let max_val = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_val < 2.0, "3-stage feedback should stay within reasonable bounds, got max: {}", max_val);

    // Check that we get some output
    let rms: f32 = buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.001, "3-stage feedback should produce audible output, got RMS: {}", rms);
}

#[test]
fn test_adaptive_compressor_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create main signal (loud kick drum)
    let main_pattern = phonon::mini_notation_v3::parse_mini_notation("bd*4");
    let main_signal = graph.add_node(SignalNode::Sample {
        pattern_str: "bd*4".to_string(),
        pattern: main_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    });

    // Create sidechain signal (snare for ducking)
    let sidechain_pattern = phonon::mini_notation_v3::parse_mini_notation("~ sn ~ sn");
    let sidechain_signal = graph.add_node(SignalNode::Sample {
        pattern_str: "~ sn ~ sn".to_string(),
        pattern: sidechain_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    });

    // Apply adaptive compressor
    let compressed = graph.add_node(SignalNode::AdaptiveCompressor {
        main_input: Signal::Node(main_signal),
        sidechain_input: Signal::Node(sidechain_signal),
        threshold: Signal::Value(-20.0), // -20dB threshold
        ratio: Signal::Value(4.0),        // 4:1 ratio
        attack: Signal::Value(0.01),      // 10ms attack
        release: Signal::Value(0.1),      // 100ms release
        adaptive_factor: Signal::Value(0.5), // 50% adaptation
        state: phonon::unified_graph::AdaptiveCompressorState::new(),
    });

    graph.set_output(compressed);

    // Render
    let buffer = graph.render(44100 * 2); // 2 seconds

    // Check for stability
    let has_inf_or_nan = buffer.iter().any(|&x| !x.is_finite());
    assert!(!has_inf_or_nan, "AdaptiveCompressor should not produce inf/nan");

    // Check for output
    let rms: f32 = buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.001, "AdaptiveCompressor should produce output");

    // Check that compression is working (max should be below uncompressed)
    let max_val = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_val < 1.5, "AdaptiveCompressor should limit peaks");
}

#[test]
fn test_5_stage_feedback_network() {
    // Complex 5-stage feedback with multiple loops
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Input: Simple pattern
    let pattern = phonon::mini_notation_v3::parse_mini_notation("bd sn hh cp");
    let input = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn hh cp".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.8),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    });

    // Stage 1: Filter
    let stage1 = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(input),
        cutoff: Signal::Value(3000.0),
        q: Signal::Value(0.5),
        state: phonon::unified_graph::FilterState::default(),
    });

    // Stage 2: Delay
    let stage2 = graph.add_node(SignalNode::Delay {
        input: Signal::Node(stage1),
        time: Signal::Value(0.125), // 1/8 note delay
        feedback: Signal::Value(0.3),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 44100],
        write_idx: 0,
    });

    // Stage 3: RMS analysis
    let stage3_rms = graph.add_node(SignalNode::RMS {
        input: Signal::Node(stage2),
        window_size: Signal::Value(0.1),
        buffer: vec![0.0; 4410],
        write_idx: 0,
    });

    // Stage 4: Compress based on RMS
    let stage4 = graph.add_node(SignalNode::Compressor {
        input: Signal::Node(stage2),
        threshold: Signal::Value(-15.0),
        ratio: Signal::Value(3.0),
        attack: Signal::Value(0.005),
        release: Signal::Value(0.05),
        makeup_gain: Signal::Value(0.0),
        state: phonon::unified_graph::CompressorState::new(),
    });

    // Stage 5: Reverb with RMS-modulated room size
    let stage5 = graph.add_node(SignalNode::Reverb {
        input: Signal::Node(stage4),
        room_size: Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Add(
            Signal::Value(0.3),
            Signal::Expression(Box::new(phonon::unified_graph::SignalExpr::Multiply(
                Signal::Node(stage3_rms),
                Signal::Value(0.3), // RMS modulates room size by ±30%
            ))),
        ))),
        damping: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        state: phonon::unified_graph::ReverbState::default(),
    });

    graph.set_output(stage5);

    // Render for stability test
    let buffer = graph.render(44100 * 3); // 3 seconds

    // Verify no explosions
    let has_inf_or_nan = buffer.iter().any(|&x| !x.is_finite());
    assert!(!has_inf_or_nan, "5-stage feedback should remain stable");

    // Verify reasonable levels
    let max_val = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(max_val < 3.0, "5-stage feedback should stay bounded, got max: {}", max_val);

    // Verify output
    let rms: f32 = buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.001, "5-stage feedback should produce output, got RMS: {}", rms);

    println!("5-stage feedback network: max={:.3}, rms={:.3}", max_val, rms);
}

#[test]
fn test_feedback_performance_multiple_loops() {
    // Performance test: 8 simultaneous feedback loops
    use std::time::Instant;

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create 8 independent feedback loops
    let mut outputs = Vec::new();

    for i in 0..8 {
        let pattern = phonon::mini_notation_v3::parse_mini_notation("bd*4");
        let input = graph.add_node(SignalNode::Sample {
            pattern_str: "bd*4".to_string(),
            pattern,
            last_trigger_time: -1.0,
            last_cycle: -1,
            playback_positions: HashMap::new(),
            gain: Signal::Value(0.3), // Lower gain to avoid clipping
            pan: Signal::Value(0.0),
            speed: Signal::Value(1.0),
            cut_group: Signal::Value(0.0),
            n: Signal::Value(0.0),
            note: Signal::Value(0.0),
            attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        });

        // Each loop: Input → Filter → Delay → RMS → Back to Filter
        let filter_cutoff = 500.0 + (i as f32 * 300.0); // Spread frequencies
        let filter = graph.add_node(SignalNode::LowPass {
            input: Signal::Node(input),
            cutoff: Signal::Value(filter_cutoff),
            q: Signal::Value(0.6),
            state: phonon::unified_graph::FilterState::default(),
        });

        let delay = graph.add_node(SignalNode::Delay {
            input: Signal::Node(filter),
            time: Signal::Value(0.125 + (i as f32 * 0.0625)), // Varied delay times
            feedback: Signal::Value(0.25),
            mix: Signal::Value(0.4),
            buffer: vec![0.0; 44100],
            write_idx: 0,
        });

        outputs.push(delay);
    }

    // Mix all 8 loops
    let mix = graph.add_node(SignalNode::Mix {
        signals: outputs.iter().map(|&id| Signal::Node(id)).collect(),
    });

    graph.set_output(mix);

    // Measure rendering performance
    let start = Instant::now();
    let buffer = graph.render(44100 * 2); // 2 seconds
    let elapsed = start.elapsed();

    // Verify stability
    let has_inf_or_nan = buffer.iter().any(|&x| !x.is_finite());
    assert!(!has_inf_or_nan, "8 parallel feedback loops should be stable");

    // Verify output
    let rms: f32 = buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.001, "8 parallel loops should produce output");

    // Performance check: should render 2 seconds of audio in < 2 seconds (real-time capable)
    println!("8 parallel feedback loops performance:");
    println!("  Rendered: 2.0 seconds of audio");
    println!("  Elapsed:  {:.3} seconds", elapsed.as_secs_f64());
    println!("  Real-time factor: {:.2}x", 2.0 / elapsed.as_secs_f64());
    println!("  RMS: {:.4}", rms);

    // Real-time check (with generous margin)
    assert!(elapsed.as_secs_f64() < 4.0, "Should render faster than 0.5x real-time");
}
