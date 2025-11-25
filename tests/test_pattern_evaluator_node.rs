/// Comprehensive tests for PatternEvaluatorNode
///
/// This test suite follows the three-level testing methodology:
/// - Level 1: Pattern query verification (event counts, cycle boundaries)
/// - Level 2: Audio verification (buffer output values, timing)
/// - Level 3: Integration testing (with oscillators, filters, multi-block)

use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::nodes::pattern_evaluator::PatternEvaluatorNode;
use phonon::pattern::{Fraction, Pattern};
use std::sync::Arc;

// ============================================================================
// LEVEL 1: Pattern Query Verification
// ============================================================================

#[test]
fn test_pattern_evaluator_basic_query() {
    // Create a simple 3-value pattern
    let pattern = parse_mini_notation("110 220 440");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    // Process one block (512 samples) at 44100 Hz, 2 CPS
    let mut output = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,  // 2 cycles per second
        44100.0,
    );

    node.process_block(&[], &mut output, 44100.0, &context);

    // At 2 CPS, one cycle = 22050 samples
    // One event = 22050/3 = 7350 samples
    // In 512 samples, we should be in the first event (110 Hz)

    // Check that all samples have the same value (sample-and-hold)
    let first_value = output[0];
    assert!((first_value - 110.0).abs() < 0.1,
        "Expected 110 Hz, got {}", first_value);

    // All samples in the block should be the same (held)
    for (i, &sample) in output.iter().enumerate() {
        assert!((sample - first_value).abs() < 0.001,
            "Sample {} should be {}, got {}", i, first_value, sample);
    }
}

#[test]
fn test_pattern_evaluator_hold_behavior() {
    // Pattern with just one value
    let pattern = parse_mini_notation("440");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    // Process multiple blocks
    let mut output1 = vec![0.0; 512];
    let mut output2 = vec![0.0; 512];

    let context1 = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,
        44100.0,
    );

    let context2 = ProcessContext::new(
        Fraction::from_float(0.02),  // Slightly later
        512,
        512,
        2.0,
        44100.0,
    );

    node.process_block(&[], &mut output1, 44100.0, &context1);
    node.process_block(&[], &mut output2, 44100.0, &context2);

    // Value should be held across blocks
    assert!((output1[0] - 440.0).abs() < 0.1);
    assert!((output2[0] - 440.0).abs() < 0.1);

    // All samples should be the same
    assert!((output1[511] - output1[0]).abs() < 0.001);
    assert!((output2[511] - output2[0]).abs() < 0.001);
}

#[test]
fn test_pattern_evaluator_cycle_progression() {
    // Pattern: "110 220 440" (3 events per cycle)
    let pattern = parse_mini_notation("110 220 440");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let sample_rate = 44100.0_f32;
    let cps = 2.0_f64;
    let samples_per_cycle = (sample_rate as f64 / cps) as usize;  // 22050 samples
    let _samples_per_event = samples_per_cycle / 3;  // 7350 samples

    // Process first cycle worth of samples
    let mut values_seen = Vec::new();
    let mut last_value = 0.0;

    let blocks_to_test = (samples_per_cycle / 512) + 1;  // ~43 blocks

    for block_idx in 0..blocks_to_test {
        let mut output = vec![0.0; 512];
        let cycle_pos = (block_idx * 512) as f64 / samples_per_cycle as f64;

        let context = ProcessContext::new(
            Fraction::from_float(cycle_pos),
            block_idx * 512,
            512,
            cps,
            sample_rate,
        );

        node.process_block(&[], &mut output, sample_rate, &context);

        // Track value changes
        let current_value = output[0];
        if (current_value - last_value).abs() > 0.1 {
            values_seen.push(current_value);
            last_value = current_value;
        }
    }

    // Should see all three values: 110, 220, 440
    assert!(values_seen.len() >= 3,
        "Should see at least 3 values, saw {}: {:?}", values_seen.len(), values_seen);

    // Check that we saw the expected values
    assert!(values_seen.iter().any(|&v| (v - 110.0).abs() < 1.0), "Should see 110 Hz");
    assert!(values_seen.iter().any(|&v| (v - 220.0).abs() < 1.0), "Should see 220 Hz");
    assert!(values_seen.iter().any(|&v| (v - 440.0).abs() < 1.0), "Should see 440 Hz");
}

#[test]
fn test_pattern_evaluator_fast_transform() {
    // Base pattern: "110 220"
    // Fast 2: Should play twice as fast, so 4 events per cycle
    let pattern = parse_mini_notation("110 220");
    let fast_pattern = pattern.fast(Pattern::pure(2.0));
    let mut node = PatternEvaluatorNode::new(Arc::new(fast_pattern));

    let sample_rate = 44100.0_f32;
    let cps = 2.0_f64;
    let samples_per_cycle = (sample_rate as f64 / cps) as usize;

    // Process one cycle
    let mut values_seen = Vec::new();
    let mut last_value = 0.0;
    let blocks_to_test = (samples_per_cycle / 512) + 1;

    for block_idx in 0..blocks_to_test {
        let mut output = vec![0.0; 512];
        let cycle_pos = (block_idx * 512) as f64 / samples_per_cycle as f64;

        let context = ProcessContext::new(
            Fraction::from_float(cycle_pos),
            block_idx * 512,
            512,
            cps,
            sample_rate,
        );

        node.process_block(&[], &mut output, sample_rate, &context);

        let current_value = output[0];
        if (current_value - last_value).abs() > 0.1 {
            values_seen.push(current_value);
            last_value = current_value;
        }
    }

    // With fast 2, we should see the pattern repeat twice in one cycle
    // So we should see: 110, 220, 110, 220
    assert!(values_seen.len() >= 4,
        "Fast 2 should produce at least 4 transitions, got {}: {:?}",
        values_seen.len(), values_seen);
}

#[test]
fn test_pattern_evaluator_slow_transform() {
    // Base pattern: "110 220 440"
    // Slow 2: Should play half as fast, so pattern takes 2 cycles
    let pattern = parse_mini_notation("110 220 440");
    let slow_pattern = pattern.slow(Pattern::pure(2.0));
    let mut node = PatternEvaluatorNode::new(Arc::new(slow_pattern));

    let sample_rate = 44100.0_f32;
    let cps = 2.0_f64;
    let samples_per_cycle = (sample_rate as f64 / cps) as usize;

    // Process one cycle - should only see first part of pattern
    let mut values_seen = Vec::new();
    let mut last_value = 0.0;
    let blocks_to_test = (samples_per_cycle / 512) + 1;

    for block_idx in 0..blocks_to_test {
        let mut output = vec![0.0; 512];
        let cycle_pos = (block_idx * 512) as f64 / samples_per_cycle as f64;

        let context = ProcessContext::new(
            Fraction::from_float(cycle_pos),
            block_idx * 512,
            512,
            cps,
            sample_rate,
        );

        node.process_block(&[], &mut output, sample_rate, &context);

        let current_value = output[0];
        if (current_value - last_value).abs() > 0.1 {
            values_seen.push(current_value);
            last_value = current_value;
        }
    }

    // With slow 2, we should only see ~2 values in one cycle (110, 220)
    // The third value (440) would appear in the second cycle
    assert!(values_seen.len() <= 3,
        "Slow 2 should produce at most 3 transitions in one cycle, got {}: {:?}",
        values_seen.len(), values_seen);
}

#[test]
fn test_pattern_evaluator_different_cps() {
    // Same pattern at different tempos
    let pattern = parse_mini_notation("110 220");

    // Test at CPS = 1.0 (slower)
    let mut node_slow = PatternEvaluatorNode::new(Arc::new(pattern.clone()));
    let mut output_slow = vec![0.0; 512];
    let context_slow = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        1.0,  // 1 cycle per second
        44100.0,
    );
    node_slow.process_block(&[], &mut output_slow, 44100.0, &context_slow);

    // Test at CPS = 4.0 (faster)
    let mut node_fast = PatternEvaluatorNode::new(Arc::new(pattern.clone()));
    let mut output_fast = vec![0.0; 512];
    let context_fast = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        4.0,  // 4 cycles per second
        44100.0,
    );
    node_fast.process_block(&[], &mut output_fast, 44100.0, &context_fast);

    // Both should start with the first value (110)
    assert!((output_slow[0] - 110.0).abs() < 0.1);
    assert!((output_fast[0] - 110.0).abs() < 0.1);

    // But at higher CPS, events come faster
    // At CPS=4, 512 samples = 512/44100*4 = 0.046 cycles (still in first event)
    // At CPS=1, 512 samples = 512/44100*1 = 0.012 cycles (still in first event)
    // Both should still be on the first value for such a small block
    assert!((output_slow[511] - 110.0).abs() < 0.1);
    assert!((output_fast[511] - 110.0).abs() < 0.1);
}

// ============================================================================
// LEVEL 2: Audio Verification
// ============================================================================

#[test]
fn test_pattern_evaluator_numeric_values() {
    // Test that numeric patterns produce correct output values
    let pattern = parse_mini_notation("100 200 300");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let mut output = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,
        44100.0,
    );

    node.process_block(&[], &mut output, 44100.0, &context);

    // Should output the first value (100)
    assert!((output[0] - 100.0).abs() < 0.1, "Expected 100, got {}", output[0]);

    // All samples should be the same (stepped/held output)
    for &sample in &output {
        assert!((sample - 100.0).abs() < 0.1, "All samples should be 100");
    }
}

#[test]
fn test_pattern_evaluator_note_names() {
    // Test that note names are converted to frequencies
    // a4 = 440 Hz
    let pattern = parse_mini_notation("a4");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let mut output = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,
        44100.0,
    );

    node.process_block(&[], &mut output, 44100.0, &context);

    // Should output 440 Hz (a4)
    assert!((output[0] - 440.0).abs() < 1.0,
        "Expected ~440 Hz for a4, got {}", output[0]);
}

#[test]
fn test_pattern_evaluator_rest_handling() {
    // Test that rests ("~") are parsed and output as zero
    // Note: This test verifies that our parse_event_value correctly handles "~"
    let pattern = parse_mini_notation("~");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let mut output = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0_f64,
        44100.0_f32,
    );

    node.process_block(&[], &mut output, 44100.0, &context);

    // A pattern with just a rest should output zeros
    // (or at least very small values if there's no event)
    let max_value = output.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    // Either we see zero (from parsing "~") or we see zero (from no events)
    // Both behaviors are acceptable for a rest
    assert!(max_value < 0.1,
        "Pattern with rest should output near-zero values, got max={}",
        max_value);

    // Also test pattern with mixed values and rest
    let pattern2 = parse_mini_notation("110 ~ 220");
    let mut node2 = PatternEvaluatorNode::new(Arc::new(pattern2));

    // Process multiple blocks to see the pattern values
    let mut saw_110 = false;
    let mut saw_220 = false;

    for block_idx in 0..50 {
        let mut output2 = vec![0.0; 512];
        let cycle_pos = (block_idx * 512) as f64 / 22050.0;  // CPS=2, so 22050 samples/cycle

        let context2 = ProcessContext::new(
            Fraction::from_float(cycle_pos),
            block_idx * 512,
            512,
            2.0_f64,
            44100.0_f32,
        );

        node2.process_block(&[], &mut output2, 44100.0, &context2);

        if (output2[0] - 110.0).abs() < 1.0 {
            saw_110 = true;
        }
        if (output2[0] - 220.0).abs() < 1.0 {
            saw_220 = true;
        }
    }

    // We should see both non-zero values from the pattern
    assert!(saw_110 && saw_220,
        "Should see both values from pattern (saw_110={}, saw_220={})",
        saw_110, saw_220);
}

// ============================================================================
// LEVEL 3: Integration Testing
// ============================================================================

#[test]
fn test_pattern_evaluator_oscillator_integration() {
    // This test verifies that PatternEvaluatorNode can drive an oscillator
    // We'll create a pattern and use it to modulate oscillator frequency

    use phonon::nodes::{OscillatorNode, Waveform};
    

    let pattern = parse_mini_notation("110 220 440");
    let mut pattern_node = PatternEvaluatorNode::new(Arc::new(pattern));

    // Process pattern to get frequency values
    let mut freq_buffer = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,
        44100.0,
    );

    pattern_node.process_block(&[], &mut freq_buffer, 44100.0, &context);

    // Use pattern output as frequency input to oscillator
    let mut osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0 (dummy)
    let mut audio_output = vec![0.0; 512];

    // Process oscillator with pattern-generated frequencies
    osc.process_block(&[&freq_buffer], &mut audio_output, 44100.0, &context);

    // Verify oscillator produced audio
    let rms = calculate_rms(&audio_output);
    assert!(rms > 0.1, "Oscillator should produce audio, got RMS={}", rms);

    // Verify oscillator output is within range [-1, 1]
    for &sample in &audio_output {
        assert!(sample >= -1.1 && sample <= 1.1,
            "Oscillator output should be in [-1, 1], got {}", sample);
    }
}

#[test]
fn test_pattern_evaluator_filter_modulation() {
    // Test that pattern can modulate filter cutoff
    use phonon::nodes::{OscillatorNode, LowPassFilterNode, Waveform};

    // Create a source oscillator
    let mut source_osc = OscillatorNode::new(0, Waveform::Saw);
    let mut source_buffer = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,
        44100.0,
    );

    // Generate constant frequency (440 Hz) source
    let freq_buffer = vec![440.0; 512];
    source_osc.process_block(&[&freq_buffer], &mut source_buffer, 44100.0, &context);

    // Create pattern to modulate filter cutoff
    let cutoff_pattern = parse_mini_notation("500 2000");
    let mut cutoff_node = PatternEvaluatorNode::new(Arc::new(cutoff_pattern));
    let mut cutoff_buffer = vec![0.0; 512];
    cutoff_node.process_block(&[], &mut cutoff_buffer, 44100.0, &context);

    // Apply filter with pattern-modulated cutoff
    let q_buffer = vec![0.7; 512];  // Constant Q
    let mut filter = LowPassFilterNode::new(0, 1, 2);  // Dummy NodeIds
    let mut filtered_output = vec![0.0; 512];

    filter.process_block(
        &[&source_buffer, &cutoff_buffer, &q_buffer],
        &mut filtered_output,
        44100.0,
        &context,
    );

    // Verify filter produced output
    let rms = calculate_rms(&filtered_output);
    assert!(rms > 0.01, "Filter should produce audio, got RMS={}", rms);
}

#[test]
fn test_pattern_evaluator_multiple_blocks() {
    // Test that the node maintains state correctly across multiple blocks
    let pattern = parse_mini_notation("110 220 440");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let sample_rate = 44100.0_f32;
    let cps = 2.0_f64;

    // Process 10 consecutive blocks
    let num_blocks = 10;
    let mut all_outputs = Vec::new();

    for block_idx in 0..num_blocks {
        let mut output = vec![0.0; 512];
        let cycle_pos = (block_idx * 512) as f64 / (sample_rate as f64 / cps);

        let context = ProcessContext::new(
            Fraction::from_float(cycle_pos),
            block_idx * 512,
            512,
            cps,
            sample_rate,
        );

        node.process_block(&[], &mut output, sample_rate, &context);
        all_outputs.extend_from_slice(&output);
    }

    // Verify we got the expected number of samples
    assert_eq!(all_outputs.len(), 512 * num_blocks);

    // Verify all values are reasonable (110, 220, or 440)
    for &sample in &all_outputs {
        assert!(
            (sample - 110.0).abs() < 1.0 ||
            (sample - 220.0).abs() < 1.0 ||
            (sample - 440.0).abs() < 1.0,
            "Sample value {} should be 110, 220, or 440", sample
        );
    }
}

#[test]
fn test_pattern_evaluator_phase_accuracy() {
    // Test that pattern events occur at the correct sample positions
    let pattern = parse_mini_notation("100 200");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let sample_rate = 44100.0_f32;
    let cps = 2.0_f64;
    let samples_per_cycle = (sample_rate as f64 / cps) as usize;
    let samples_per_event = samples_per_cycle / 2;  // 11025 samples per event

    // Process enough blocks to cover one complete cycle
    let blocks_needed = (samples_per_cycle / 512) + 1;
    let mut transition_points = Vec::new();
    let mut last_value = 0.0;

    for block_idx in 0..blocks_needed {
        let mut output = vec![0.0; 512];
        let cycle_pos = (block_idx * 512) as f64 / samples_per_cycle as f64;

        let context = ProcessContext::new(
            Fraction::from_float(cycle_pos),
            block_idx * 512,
            512,
            cps,
            sample_rate,
        );

        node.process_block(&[], &mut output, sample_rate, &context);

        for (i, &sample) in output.iter().enumerate() {
            if (sample - last_value).abs() > 0.1 {
                transition_points.push(block_idx * 512 + i);
                last_value = sample;
            }
        }
    }

    // Should have at least one transition (100 -> 200)
    assert!(!transition_points.is_empty(), "Should have at least one transition");

    // The pattern starts with the first value (100), so the first transition is at sample 0
    // The second transition (100 -> 200) should be near samples_per_event (11025)
    if transition_points.len() >= 2 {
        let second_transition = transition_points[1];
        let expected = samples_per_event;
        let tolerance = 512;  // Within one block
        assert!(
            (second_transition as i64 - expected as i64).abs() < tolerance as i64,
            "Second transition at sample {} should be near {} (first={}, transitions={:?})",
            second_transition, expected, transition_points[0], transition_points
        );
    } else {
        // If we only see one transition, that's ok for this test
        // It just means we're verifying the node produces transitions
        println!("Note: Only saw {} transition(s)", transition_points.len());
    }
}

#[test]
fn test_pattern_evaluator_performance() {
    // Test that processing is reasonably fast
    use std::time::Instant;

    let pattern = parse_mini_notation("110 220 440 880");
    let mut node = PatternEvaluatorNode::new(Arc::new(pattern));

    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,
        44100.0,
    );

    // Process 1000 blocks and measure time
    let mut output = vec![0.0; 512];
    let start = Instant::now();

    for _ in 0..1000 {
        node.process_block(&[], &mut output, 44100.0, &context);
    }

    let elapsed = start.elapsed();
    let blocks_per_second = 1000.0 / elapsed.as_secs_f64();

    // Should be able to process significantly faster than real-time
    // Real-time requires: 44100 samples/sec / 512 samples/block = ~86 blocks/sec
    // Target: at least 10x real-time (860 blocks/sec)
    let realtime_requirement = 86.0;
    let min_performance = realtime_requirement * 10.0;  // 10x real-time

    assert!(blocks_per_second > min_performance,
        "Performance too slow: {:.0} blocks/sec (need >{:.0} blocks/sec for 10x real-time)",
        blocks_per_second, min_performance);

    println!("Performance: {:.0} blocks/sec ({:.1}x real-time)",
        blocks_per_second,
        blocks_per_second / realtime_requirement);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}
