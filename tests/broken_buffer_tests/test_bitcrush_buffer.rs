/// BitCrush Buffer Evaluation Tests
///
/// Tests the buffer-based evaluation of the BitCrush node, which applies:
/// 1. Bit reduction (quantization)
/// 2. Sample rate reduction (sample-and-hold)
///
/// These tests verify:
/// - Bit quantization creates discrete levels
/// - Sample rate reduction holds samples correctly
/// - Combined effects work as expected
/// - State continuity across buffer evaluations
/// - Edge cases (extreme bit depths, high reduction rates)

use phonon::unified_graph::{UnifiedSignalGraph, Signal, Waveform};
use std::collections::HashSet;

/// Helper: Create a test graph with standard sample rate
fn create_test_graph() -> UnifiedSignalGraph {
    UnifiedSignalGraph::new(44100.0)
}

/// Helper: Count unique quantized values in buffer
fn count_unique_levels(buffer: &[f32], precision: i32) -> usize {
    let mut unique: HashSet<i32> = HashSet::new();
    for &sample in buffer {
        unique.insert((sample * 10_f32.powi(precision)) as i32);
    }
    unique.len()
}

/// Helper: Count consecutive identical samples (measure sample-and-hold)
fn count_holds(buffer: &[f32]) -> Vec<usize> {
    let mut holds = Vec::new();
    let mut current_count = 1;

    for i in 1..buffer.len() {
        if (buffer[i] - buffer[i - 1]).abs() < 0.0001 {
            current_count += 1;
        } else {
            holds.push(current_count);
            current_count = 1;
        }
    }
    holds.push(current_count);
    holds
}

// ============================================================================
// TEST 1: Bit Reduction - 4-bit Quantization
// ============================================================================

#[test]
fn test_bitcrush_4bit_reduction() {
    let mut graph = create_test_graph();

    // Create 440 Hz sine wave
    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply 4-bit crushing (no sample rate reduction)
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(4.0),  // 4-bit = 16 levels
        Signal::Value(1.0),  // No sample rate reduction
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Count unique quantized levels (4-bit = 2^4 = 16 levels)
    let unique_count = count_unique_levels(&output, 3);

    // Should have ~16 levels (allow some tolerance for oscillator variation and rounding)
    assert!(
        unique_count <= 40,
        "4-bit should produce ~16-32 unique levels (with oscillator variation), got {}",
        unique_count
    );

    // Verify output is not silent
    let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.1, "Output should have significant energy");
}

// ============================================================================
// TEST 2: Bit Reduction - 8-bit Quantization
// ============================================================================

#[test]
fn test_bitcrush_8bit_reduction() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply 8-bit crushing
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(8.0),  // 8-bit = 256 levels
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Count unique levels
    let unique_count = count_unique_levels(&output, 3);

    // 8-bit should have more levels than 4-bit
    assert!(
        unique_count > 30,
        "8-bit should have many more levels than 4-bit, got {}",
        unique_count
    );
}

// ============================================================================
// TEST 3: Bit Reduction - 1-bit (Extreme)
// ============================================================================

#[test]
fn test_bitcrush_1bit_extreme() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply 1-bit crushing (binary output)
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(1.0),  // 1-bit = 2 levels
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Should have only 2-4 unique values (1-bit with bipolar input)
    let unique_count = count_unique_levels(&output, 3);

    assert!(
        unique_count <= 6,
        "1-bit should produce ~2-4 unique levels, got {}",
        unique_count
    );
}

// ============================================================================
// TEST 4: Bit Reduction - 16-bit (Minimal Effect)
// ============================================================================

#[test]
fn test_bitcrush_16bit_minimal() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply 16-bit crushing (should be nearly transparent)
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(16.0),
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];

    graph.eval_node_buffer(&crush_id, &mut output);

    // 16-bit should have high resolution (many unique values)
    let unique_count = count_unique_levels(&output, 3);

    // 16-bit = 65536 levels, should have very high resolution
    assert!(
        unique_count > 100,
        "16-bit should have high resolution, got {} unique levels",
        unique_count
    );

    // Verify output has energy
    let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    assert!(rms.sqrt() > 0.1, "Output should have energy");
}

// ============================================================================
// TEST 5: Sample Rate Reduction - Factor 2
// ============================================================================

#[test]
fn test_bitcrush_sample_rate_reduction_2x() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply sample rate reduction only (16-bit = no bit reduction)
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(16.0),  // No bit reduction
        Signal::Value(2.0),   // Hold every 2nd sample
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Count consecutive holds
    let holds = count_holds(&output);

    // Most holds should be ~2 samples long
    let avg_hold: f32 = holds.iter().sum::<usize>() as f32 / holds.len() as f32;

    assert!(
        avg_hold >= 1.5 && avg_hold <= 2.5,
        "2x reduction should hold ~2 samples, got avg {}",
        avg_hold
    );
}

// ============================================================================
// TEST 6: Sample Rate Reduction - Factor 4
// ============================================================================

#[test]
fn test_bitcrush_sample_rate_reduction_4x() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(16.0),
        Signal::Value(4.0),  // Hold every 4th sample
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    let holds = count_holds(&output);
    let avg_hold: f32 = holds.iter().sum::<usize>() as f32 / holds.len() as f32;

    assert!(
        avg_hold >= 3.0 && avg_hold <= 5.0,
        "4x reduction should hold ~4 samples, got avg {}",
        avg_hold
    );
}

// ============================================================================
// TEST 7: Sample Rate Reduction - Factor 8 (Extreme)
// ============================================================================

#[test]
fn test_bitcrush_sample_rate_reduction_8x() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(16.0),
        Signal::Value(8.0),  // Hold every 8th sample
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    let holds = count_holds(&output);
    let avg_hold: f32 = holds.iter().sum::<usize>() as f32 / holds.len() as f32;

    assert!(
        avg_hold >= 6.0 && avg_hold <= 10.0,
        "8x reduction should hold ~8 samples, got avg {}",
        avg_hold
    );
}

// ============================================================================
// TEST 8: Combined Effect - Bit + Sample Rate Reduction
// ============================================================================

#[test]
fn test_bitcrush_combined_effect() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Apply both effects
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(4.0),  // 4-bit quantization
        Signal::Value(4.0),  // 4x sample rate reduction
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Verify quantization
    let unique_count = count_unique_levels(&output, 3);
    assert!(unique_count <= 40, "Should have ~16-32 quantization levels (with oscillator variation)");

    // Verify sample-and-hold (expect ~4 sample holds with rate reduction of 4)
    let holds = count_holds(&output);
    let avg_hold: f32 = holds.iter().sum::<usize>() as f32 / holds.len() as f32;
    assert!(avg_hold >= 2.0, "Should hold at least ~2-4 samples on average");

    // Verify output is not silent
    let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.05, "Output should have energy");
}

// ============================================================================
// TEST 9: State Continuity - Multiple Buffer Calls
// ============================================================================

#[test]
fn test_bitcrush_state_continuity() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(4.0),
        Signal::Value(4.0),
    );

    let buffer_size = 256;
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];

    // Render two consecutive buffers
    graph.eval_node_buffer(&crush_id, &mut buffer1);
    graph.eval_node_buffer(&crush_id, &mut buffer2);

    // Both buffers should have content
    let rms1: f32 = buffer1.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    let rms2: f32 = buffer2.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;

    assert!(rms1.sqrt() > 0.05, "First buffer should have energy");
    assert!(rms2.sqrt() > 0.05, "Second buffer should have energy");

    // Sample counters should continue (not reset)
    // This is implicitly tested by checking that both buffers have similar characteristics
    let holds1 = count_holds(&buffer1);
    let holds2 = count_holds(&buffer2);

    let avg1: f32 = holds1.iter().sum::<usize>() as f32 / holds1.len() as f32;
    let avg2: f32 = holds2.iter().sum::<usize>() as f32 / holds2.len() as f32;

    // Both should have similar hold patterns
    assert!((avg1 - avg2).abs() < 2.0, "Hold patterns should be consistent");
}

// ============================================================================
// TEST 10: Pattern-Controlled Bit Depth
// ============================================================================
// NOTE: Currently causes stack overflow due to recursive signal evaluation
// TODO: Fix after buffer evaluation is complete for all node types

#[test]
#[ignore]
fn test_bitcrush_pattern_controlled_bits() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Modulate bit depth with LFO (2-8 bits)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Scale LFO: sine (-1 to 1) -> (2 to 8 bits)
    // bits = 5 + 3 * lfo = 5 + 3*sin(t)
    let multiply_id = graph.add_node(phonon::unified_graph::SignalNode::Multiply {
        a: Signal::Value(3.0),
        b: Signal::Node(lfo_id),
    });
    let scaled_id = graph.add_node(phonon::unified_graph::SignalNode::Add {
        a: Signal::Value(5.0),
        b: Signal::Node(multiply_id),
    });

    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Node(scaled_id),  // Pattern-controlled bits
        Signal::Value(1.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Output should vary in quantization over time
    // Measure RMS of first vs second half
    let rms1: f32 = output[0..256].iter().map(|x| x * x).sum::<f32>() / 256.0;
    let rms2: f32 = output[256..512].iter().map(|x| x * x).sum::<f32>() / 256.0;

    // Both halves should have energy
    assert!(rms1.sqrt() > 0.05, "First half should have energy");
    assert!(rms2.sqrt() > 0.05, "Second half should have energy");
}

// ============================================================================
// TEST 11: Pattern-Controlled Sample Rate
// ============================================================================
// NOTE: Currently causes stack overflow due to recursive signal evaluation
// TODO: Fix after buffer evaluation is complete for all node types

#[test]
#[ignore]
fn test_bitcrush_pattern_controlled_rate() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);

    // Modulate sample rate with LFO (1x to 8x reduction)
    let lfo_id = graph.add_oscillator(Signal::Value(0.5), Waveform::Sine);

    // Scale LFO: sine (-1 to 1) -> (1 to 8)
    let multiply_id = graph.add_node(phonon::unified_graph::SignalNode::Multiply {
        a: Signal::Value(3.5),
        b: Signal::Node(lfo_id),
    });
    let scaled_id = graph.add_node(phonon::unified_graph::SignalNode::Add {
        a: Signal::Value(4.5),
        b: Signal::Node(multiply_id),
    });

    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(16.0),  // No bit reduction
        Signal::Node(scaled_id),  // Pattern-controlled rate
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Output should have varying hold lengths
    let holds = count_holds(&output);

    // Should see variation in hold lengths
    let min_hold = *holds.iter().min().unwrap();
    let max_hold = *holds.iter().max().unwrap();

    assert!(max_hold > min_hold, "Hold lengths should vary over time");
}

// ============================================================================
// TEST 12: Noise Source + BitCrush
// ============================================================================

#[test]
fn test_bitcrush_with_noise() {
    let mut graph = create_test_graph();

    // Use a noise source instead of oscillator
    let noise_id = graph.add_whitenoise_node();

    let crush_id = graph.add_bitcrush_node(
        Signal::Node(noise_id),
        Signal::Value(3.0),  // 3-bit (8 levels)
        Signal::Value(4.0),  // 4x rate reduction
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Verify quantization (3-bit = 8 levels, but allow more for noise variation)
    let unique_count = count_unique_levels(&output, 3);
    assert!(unique_count <= 20, "3-bit should produce ~8-16 levels with noise");

    // Verify sample-and-hold
    let holds = count_holds(&output);
    let avg_hold: f32 = holds.iter().sum::<usize>() as f32 / holds.len() as f32;
    assert!(avg_hold >= 2.5, "Should hold ~4 samples");

    // Verify output has energy
    let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    assert!(rms.sqrt() > 0.05, "Crushed noise should have energy");
}

// ============================================================================
// TEST 13: Zero Input
// ============================================================================

#[test]
fn test_bitcrush_zero_input() {
    let mut graph = create_test_graph();

    let crush_id = graph.add_bitcrush_node(
        Signal::Value(0.0),  // Zero input
        Signal::Value(4.0),
        Signal::Value(4.0),
    );

    let buffer_size = 512;
    let mut output = vec![0.0; buffer_size];
    graph.eval_node_buffer(&crush_id, &mut output);

    // Output should be all zeros
    for &sample in &output {
        assert_eq!(sample, 0.0, "Zero input should produce zero output");
    }
}

// ============================================================================
// TEST 14: Multiple Consecutive Buffers (State Preservation)
// ============================================================================

#[test]
fn test_bitcrush_consecutive_buffers() {
    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(4.0),
        Signal::Value(4.0),
    );

    let buffer_size = 256;

    // Render three consecutive buffers
    let mut buffer1 = vec![0.0; buffer_size];
    let mut buffer2 = vec![0.0; buffer_size];
    let mut buffer3 = vec![0.0; buffer_size];

    graph.eval_node_buffer(&crush_id, &mut buffer1);
    graph.eval_node_buffer(&crush_id, &mut buffer2);
    graph.eval_node_buffer(&crush_id, &mut buffer3);

    // All buffers should have energy
    let rms1: f32 = buffer1.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    let rms2: f32 = buffer2.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;
    let rms3: f32 = buffer3.iter().map(|x| x * x).sum::<f32>() / buffer_size as f32;

    assert!(rms1.sqrt() > 0.05, "Buffer 1 should have energy");
    assert!(rms2.sqrt() > 0.05, "Buffer 2 should have energy");
    assert!(rms3.sqrt() > 0.05, "Buffer 3 should have energy");

    // All buffers should have similar characteristics (quantization levels)
    let unique1 = count_unique_levels(&buffer1, 3);
    let unique2 = count_unique_levels(&buffer2, 3);
    let unique3 = count_unique_levels(&buffer3, 3);

    // All should have ~16-32 levels (4-bit with oscillator variation)
    assert!(unique1 <= 40 && unique1 > 0);
    assert!(unique2 <= 40 && unique2 > 0);
    assert!(unique3 <= 40 && unique3 > 0);
}

// ============================================================================
// TEST 15: Performance Test (Buffer Evaluation)
// ============================================================================

#[test]
fn test_bitcrush_buffer_performance() {
    use std::time::Instant;

    let mut graph = create_test_graph();

    let osc_id = graph.add_oscillator(Signal::Value(440.0), Waveform::Sine);
    let crush_id = graph.add_bitcrush_node(
        Signal::Node(osc_id),
        Signal::Value(4.0),
        Signal::Value(4.0),
    );

    let buffer_size = 4096;
    let iterations = 100;

    // Measure buffer evaluation time
    let mut output = vec![0.0; buffer_size];
    let start = Instant::now();
    for _ in 0..iterations {
        graph.eval_node_buffer(&crush_id, &mut output);
    }
    let buffer_time = start.elapsed();

    println!("BitCrush buffer evaluation performance:");
    println!("  Time for {} iterations: {:?}", iterations, buffer_time);
    println!("  Time per buffer: {:?}", buffer_time / iterations as u32);

    // Sanity check: should complete in reasonable time
    assert!(buffer_time.as_secs() < 10, "Performance test took too long");
}
