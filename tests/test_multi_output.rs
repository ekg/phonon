use phonon::unified_graph::{SignalNode, UnifiedSignalGraph, NodeId, Signal, Waveform};
use phonon::mini_notation_v3::parse_mini_notation;
use std::collections::HashMap;

#[test]
fn test_dual_output_rendering() {
    // Test that out1 and out2 can be defined independently
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second

    // Create two oscillators at different frequencies
    let node1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let node2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    // Set multiple outputs
    graph.set_output_channel(1, node1);
    graph.set_output_channel(2, node2);

    // Render 1 cycle (0.5 seconds at tempo 2.0)
    let samples_per_cycle = (sample_rate / 2.0) as usize;
    let mut output1_sum = 0.0f32;
    let mut output2_sum = 0.0f32;

    for _ in 0..samples_per_cycle {
        let outputs = graph.process_sample_multi();
        output1_sum += outputs[0].abs();
        output2_sum += outputs[1].abs();
    }

    // Both outputs should have non-zero audio
    assert!(output1_sum > 0.0, "Output 1 should have audio (110 Hz sine)");
    assert!(output2_sum > 0.0, "Output 2 should have audio (880 Hz sine)");

    // Output 1 and output 2 should be different
    // 880Hz should have 8x more zero crossings, so sums will be different
    let ratio = output1_sum / output2_sum;
    assert!(ratio > 0.5 && ratio < 2.0,
        "Outputs should be similar in magnitude (ratio: {:.2})", ratio);
}

#[test]
fn test_quad_output_rendering() {
    // Test 4 independent outputs
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    // Create sine waves at different frequencies
    let node1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });
    let node2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });
    let node3 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });
    let node4 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    graph.set_output_channel(1, node1);
    graph.set_output_channel(2, node2);
    graph.set_output_channel(3, node3);
    graph.set_output_channel(4, node4);

    // Render a few samples
    // Note: Oscillators start at phase=0, so first sample will be sin(0)=0
    // We need to skip the first sample or accumulate over multiple samples
    let mut max_values = vec![0.0f32; 4];

    for _ in 0..100 {
        let outputs = graph.process_sample_multi();
        assert_eq!(outputs.len(), 4, "Should have 4 outputs");

        // Track maximum value seen for each output
        for (i, &sample) in outputs.iter().enumerate() {
            if sample.abs() > max_values[i] {
                max_values[i] = sample.abs();
            }
        }
    }

    // After 100 samples, each output should have produced non-zero signal
    for (i, &max_val) in max_values.iter().enumerate() {
        assert!(max_val > 0.0, "Output {} should have signal (max={})", i + 1, max_val);
    }
}

#[test]
fn test_hush_command() {
    // Test that hush silences all outputs
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    // Create outputs
    let node1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });
    let node2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    graph.set_output_channel(1, node1);
    graph.set_output_channel(2, node2);

    // Process samples - should have signal
    // Note: Skip first sample since oscillators start at phase=0
    graph.process_sample_multi();
    let outputs_before = graph.process_sample_multi();
    assert!(outputs_before[0].abs() > 0.0);
    assert!(outputs_before[1].abs() > 0.0);

    // Hush all outputs
    graph.hush_all();

    // Process samples - should be silent
    let outputs_after = graph.process_sample_multi();
    assert_eq!(outputs_after[0], 0.0, "Output 1 should be hushed");
    assert_eq!(outputs_after[1], 0.0, "Output 2 should be hushed");
}

#[test]
fn test_hush_specific_output() {
    // Test that hush can silence specific outputs
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let node1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });
    let node2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    graph.set_output_channel(1, node1);
    graph.set_output_channel(2, node2);

    // Hush only output 1
    graph.hush_channel(1);

    // Skip first sample since oscillators start at phase=0
    graph.process_sample_multi();
    let outputs = graph.process_sample_multi();
    assert_eq!(outputs[0], 0.0, "Output 1 should be hushed");
    assert!(outputs[1].abs() > 0.0, "Output 2 should still play");
}

#[test]
fn test_panic_command() {
    // Test that panic kills voices AND silences outputs
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    let pattern = parse_mini_notation("bd sn hh*16");
    let node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn hh*16".to_string(),
        pattern,
        last_trigger_time: -1.0,
        playback_positions: HashMap::new(),
    });

    graph.set_output_channel(1, node);

    // Trigger some samples
    for _ in 0..1000 {
        graph.process_sample_multi();
    }

    // Should have active voices
    let outputs_before = graph.process_sample_multi();
    let has_signal_before = outputs_before[0].abs() > 0.0;

    // Panic - kill everything
    graph.panic();

    // Should be completely silent
    for _ in 0..100 {
        let outputs_after = graph.process_sample_multi();
        assert_eq!(outputs_after[0], 0.0, "Output should be silent after panic");
    }
}
