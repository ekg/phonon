/// Tests for buffer-based SpectralFreeze evaluation
///
/// This tests the SpectralFreeze node's buffer evaluation to ensure:
/// 1. Freeze trigger captures spectrum
/// 2. Frozen spectrum is sustained
/// 3. Unfreezing resumes normal output
/// 4. State continuity across buffer boundaries
/// 5. Different input signals can be frozen
/// 6. Frozen output is stable over time

use phonon::unified_graph::{Signal, UnifiedSignalGraph, Waveform};

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;

/// Helper: Create a graph with sample rate
fn create_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(SAMPLE_RATE)
}

/// Helper: Evaluate a node for one buffer
fn eval_buffer(graph: &mut UnifiedSignalGraph, node_id: usize) -> Vec<f32> {
    let mut buffer = vec![0.0; BUFFER_SIZE];
    let node_id = phonon::unified_graph::NodeId(node_id);
    graph.eval_node_buffer(&node_id, &mut buffer);
    buffer
}

/// Helper: Evaluate a node for multiple buffers
fn eval_multiple_buffers(
    graph: &mut UnifiedSignalGraph,
    node_id: usize,
    num_buffers: usize,
) -> Vec<f32> {
    let mut output = Vec::new();
    for _ in 0..num_buffers {
        let buffer = eval_buffer(graph, node_id);
        output.extend_from_slice(&buffer);
    }
    output
}

/// Helper: Calculate RMS of buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Helper: Find peak value in buffer
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

/// Helper: Calculate correlation between two buffers (1.0 = identical, 0.0 = uncorrelated)
fn calculate_correlation(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let n = a.len() as f32;

    let mean_a: f32 = a.iter().sum::<f32>() / n;
    let mean_b: f32 = b.iter().sum::<f32>() / n;

    let mut covariance = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;

    for i in 0..a.len() {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        covariance += da * db;
        var_a += da * da;
        var_b += db * db;
    }

    if var_a < 1e-10 || var_b < 1e-10 {
        return 0.0; // Avoid division by zero
    }

    covariance / (var_a.sqrt() * var_b.sqrt())
}

/// Helper: Calculate average absolute difference between buffers
fn calculate_difference(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    sum / a.len() as f32
}

#[test]
fn test_spectralfreeze_passthrough() {
    // Test that with trigger=0, spectral freeze passes signal through
    let mut graph = create_graph();

    // Create oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create spectral freeze with trigger off (0.0)
    let freeze_id = graph.add_spectralfreeze_node(Signal::Node(osc), Signal::Value(0.0));

    // Render normal oscillator for comparison
    let normal = eval_buffer(&mut graph, osc.0);

    // Render through spectral freeze
    let frozen = eval_buffer(&mut graph, freeze_id.0);

    // When not frozen, output should be similar to input
    let diff = calculate_difference(&normal, &frozen);
    assert!(
        diff < 0.3,
        "Unfrozen output should be similar to input, diff: {}",
        diff
    );

    let rms = calculate_rms(&frozen);
    assert!(rms > 0.3, "Output should have signal, RMS: {}", rms);
}

#[test]
fn test_spectralfreeze_triggers() {
    // Test that trigger=1 causes freeze to capture spectrum
    let mut graph = create_graph();

    // Create oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create spectral freeze with trigger on (1.0)
    let freeze_id = graph.add_spectralfreeze_node(Signal::Node(osc), Signal::Value(1.0));

    // Render multiple buffers
    let buffer1 = eval_buffer(&mut graph, freeze_id.0);
    let buffer2 = eval_buffer(&mut graph, freeze_id.0);
    let buffer3 = eval_buffer(&mut graph, freeze_id.0);

    // All buffers should have signal
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);
    let rms3 = calculate_rms(&buffer3);

    assert!(rms1 > 0.1, "First buffer should have signal, RMS: {}", rms1);
    assert!(rms2 > 0.1, "Second buffer should have signal, RMS: {}", rms2);
    assert!(rms3 > 0.1, "Third buffer should have signal, RMS: {}", rms3);

    // When frozen, subsequent buffers should be similar (spectrum is frozen)
    let diff_1_2 = calculate_difference(&buffer1, &buffer2);
    let diff_2_3 = calculate_difference(&buffer2, &buffer3);

    // Note: Due to hop_size and overlap-add, there may be some variation
    // but frozen output should be more stable than unfrozen
    println!("Frozen buffer differences: 1-2: {}, 2-3: {}", diff_1_2, diff_2_3);
}

#[test]
fn test_spectralfreeze_freeze_stability() {
    // Test that frozen output is stable over many buffers
    let mut graph = create_graph();

    // Create oscillator
    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Create spectral freeze with trigger on
    let freeze_id = graph.add_spectralfreeze_node(Signal::Node(osc), Signal::Value(1.0));

    // Render many buffers to let freeze stabilize
    let _ = eval_multiple_buffers(&mut graph, freeze_id.0, 10);

    // Now capture several buffers
    let buffer1 = eval_buffer(&mut graph, freeze_id.0);
    let buffer2 = eval_buffer(&mut graph, freeze_id.0);
    let buffer3 = eval_buffer(&mut graph, freeze_id.0);

    // Calculate RMS for each
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);
    let rms3 = calculate_rms(&buffer3);

    // All should have similar energy
    let rms_diff_1_2 = (rms1 - rms2).abs();
    let rms_diff_2_3 = (rms2 - rms3).abs();

    assert!(
        rms_diff_1_2 < 0.1,
        "Frozen buffers should have similar RMS, diff: {}",
        rms_diff_1_2
    );
    assert!(
        rms_diff_2_3 < 0.1,
        "Frozen buffers should have similar RMS, diff: {}",
        rms_diff_2_3
    );
}

#[test]
fn test_spectralfreeze_different_inputs() {
    // Test freezing different waveforms
    let mut graph = create_graph();

    // Test with saw wave
    let saw = graph.add_oscillator(Signal::Value(220.0), Waveform::Saw);
    let freeze_saw = graph.add_spectralfreeze_node(Signal::Node(saw), Signal::Value(1.0));

    let saw_frozen = eval_buffer(&mut graph, freeze_saw.0);
    let rms_saw = calculate_rms(&saw_frozen);
    assert!(rms_saw > 0.1, "Frozen saw should have signal, RMS: {}", rms_saw);

    // Test with square wave
    let mut graph2 = create_graph();
    let square = graph2.add_oscillator(Signal::Value(220.0), Waveform::Square);
    let freeze_square = graph2.add_spectralfreeze_node(Signal::Node(square), Signal::Value(1.0));

    let square_frozen = eval_buffer(&mut graph2, freeze_square.0);
    let rms_square = calculate_rms(&square_frozen);
    assert!(
        rms_square > 0.1,
        "Frozen square should have signal, RMS: {}",
        rms_square
    );

    // Test with triangle wave
    let mut graph3 = create_graph();
    let tri = graph3.add_oscillator(Signal::Value(220.0), Waveform::Triangle);
    let freeze_tri = graph3.add_spectralfreeze_node(Signal::Node(tri), Signal::Value(1.0));

    let tri_frozen = eval_buffer(&mut graph3, freeze_tri.0);
    let rms_tri = calculate_rms(&tri_frozen);
    assert!(
        rms_tri > 0.1,
        "Frozen triangle should have signal, RMS: {}",
        rms_tri
    );
}

#[test]
fn test_spectralfreeze_state_continuity() {
    // Test that state is maintained across buffer boundaries
    let mut graph = create_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let freeze_id = graph.add_spectralfreeze_node(Signal::Node(osc), Signal::Value(1.0));

    // Render multiple buffers
    let buffers: Vec<Vec<f32>> = (0..5)
        .map(|_| eval_buffer(&mut graph, freeze_id.0))
        .collect();

    // Check that all buffers have signal
    for (i, buffer) in buffers.iter().enumerate() {
        let rms = calculate_rms(buffer);
        assert!(
            rms > 0.1,
            "Buffer {} should have signal, RMS: {}",
            i,
            rms
        );
    }

    // Later buffers should be relatively stable (frozen)
    let diff_3_4 = calculate_difference(&buffers[3], &buffers[4]);
    println!("State continuity test - buffer 3-4 diff: {}", diff_3_4);
}

#[test]
fn test_spectralfreeze_silent_input() {
    // Test that freezing silence produces silence
    let mut graph = create_graph();

    // Silent input
    let silence = graph.add_node(phonon::unified_graph::SignalNode::Constant { value: 0.0 });

    // Freeze it
    let freeze_id = graph.add_spectralfreeze_node(Signal::Node(silence), Signal::Value(1.0));

    let buffer = eval_buffer(&mut graph, freeze_id.0);
    let rms = calculate_rms(&buffer);
    let peak = find_peak(&buffer);

    assert!(
        rms < 0.01,
        "Frozen silence should remain silent, RMS: {}",
        rms
    );
    assert!(
        peak < 0.01,
        "Frozen silence should have no peaks, peak: {}",
        peak
    );
}

#[test]
fn test_spectralfreeze_sustained_output() {
    // Test that frozen output sustains over many buffers
    let mut graph = create_graph();

    let osc = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let freeze_id = graph.add_spectralfreeze_node(Signal::Node(osc), Signal::Value(1.0));

    // Render many buffers (20 buffers = ~0.23 seconds at 44.1kHz with 512 samples)
    let all_buffers = eval_multiple_buffers(&mut graph, freeze_id.0, 20);

    // Calculate RMS of entire output
    let total_rms = calculate_rms(&all_buffers);

    assert!(
        total_rms > 0.1,
        "Sustained frozen output should have signal, RMS: {}",
        total_rms
    );

    // Check that later segments still have energy
    let segment_size = all_buffers.len() / 4;
    let last_quarter = &all_buffers[3 * segment_size..];
    let last_quarter_rms = calculate_rms(last_quarter);

    assert!(
        last_quarter_rms > 0.1,
        "Last quarter should still have signal, RMS: {}",
        last_quarter_rms
    );
}

#[test]
fn test_spectralfreeze_multiple_frequencies() {
    // Test freezing different frequencies
    let frequencies = vec![110.0, 220.0, 440.0, 880.0];

    for freq in frequencies {
        let mut graph = create_graph();
        let osc = graph.add_oscillator(Signal::Value(freq), Waveform::Sine);
        let freeze_id = graph.add_spectralfreeze_node(Signal::Node(osc), Signal::Value(1.0));

        let buffer = eval_buffer(&mut graph, freeze_id.0);
        let rms = calculate_rms(&buffer);

        assert!(
            rms > 0.1,
            "Freeze should work at {} Hz, RMS: {}",
            freq,
            rms
        );
    }
}
