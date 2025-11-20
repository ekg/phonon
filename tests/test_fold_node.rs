/// Standalone test for FoldNode
///
/// This tests the threshold-based wavefolding implementation
/// independently of other code compilation issues.

use phonon::nodes::{ConstantNode, FoldNode, OscillatorNode, Waveform};
use phonon::audio_node::AudioNode;
use phonon::pattern::Fraction;
use phonon::audio_node::ProcessContext;

fn test_context(block_size: usize) -> ProcessContext {
    ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        block_size,
        2.0,
        44100.0,
    )
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

#[test]
fn test_fold_below_threshold_unchanged() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![0.0, 0.25, 0.5, -0.25, -0.5];
    let threshold = vec![1.0; 5];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 5];
    let context = test_context(5);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    assert_eq!(output[0], 0.0);
    assert_eq!(output[1], 0.25);
    assert_eq!(output[2], 0.5);
    assert_eq!(output[3], -0.25);
    assert_eq!(output[4], -0.5);
}

#[test]
fn test_fold_single_positive_fold() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![1.3];
    let threshold = vec![1.0];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 1];
    let context = test_context(1);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // 1.3 with threshold 1.0 should fold to 0.7
    assert!((output[0] - 0.7).abs() < 1e-6, "Expected ~0.7, got {}", output[0]);
}

#[test]
fn test_fold_single_negative_fold() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![-1.3];
    let threshold = vec![1.0];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 1];
    let context = test_context(1);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // -1.3 with threshold 1.0 should fold to -0.7
    assert!((output[0] - (-0.7)).abs() < 1e-6, "Expected ~-0.7, got {}", output[0]);
}

#[test]
fn test_fold_multiple_folds() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![2.5, -2.5, 3.7, -3.7];
    let threshold = vec![1.0; 4];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // 2.5: excess=1.5, folds=1 (odd), remainder=0.5 → -0.5
    assert!((output[0] - (-0.5)).abs() < 1e-6, "Expected ~-0.5, got {}", output[0]);
    // -2.5: excess=1.5, folds=1 (odd), remainder=0.5 → 0.5
    assert!((output[1] - 0.5).abs() < 1e-6, "Expected ~0.5, got {}", output[1]);
    // 3.7: excess=2.7, folds=2 (even), remainder=0.7 → 0.3
    assert!((output[2] - 0.3).abs() < 1e-6, "Expected ~0.3, got {}", output[2]);
    // -3.7: excess=2.7, folds=2 (even), remainder=0.7 → -0.3
    assert!((output[3] - (-0.3)).abs() < 1e-6, "Expected ~-0.3, got {}", output[3]);
}

#[test]
fn test_fold_symmetrical() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![1.5, -1.5, 2.3, -2.3, 0.8, -0.8];
    let threshold = vec![1.0; 6];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 6];
    let context = test_context(6);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // Verify symmetry: fold(x) = -fold(-x)
    assert!((output[0] + output[1]).abs() < 1e-6, "fold(1.5) should equal -fold(-1.5)");
    assert!((output[2] + output[3]).abs() < 1e-6, "fold(2.3) should equal -fold(-2.3)");
    assert!((output[4] + output[5]).abs() < 1e-6, "fold(0.8) should equal -fold(-0.8)");
}

#[test]
fn test_fold_at_threshold_boundary() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![1.0, -1.0, 0.5, -0.5];
    let threshold = vec![1.0, 1.0, 0.5, 0.5];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // At threshold: should pass through unchanged
    assert_eq!(output[0], 1.0);
    assert_eq!(output[1], -1.0);
    assert_eq!(output[2], 0.5);
    assert_eq!(output[3], -0.5);
}

#[test]
fn test_fold_varying_threshold() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![2.0, 2.0, 2.0, 2.0];
    let threshold = vec![0.5, 1.0, 1.5, 2.0];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // Pattern-modulated threshold creates different results
    assert!((output[0] - (-0.5)).abs() < 1e-6);
    assert!((output[1] - (-1.0)).abs() < 1e-6);
    assert!((output[2] - 1.0).abs() < 1e-6);
    assert_eq!(output[3], 2.0);
}

#[test]
fn test_fold_small_threshold() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![1.0];
    let threshold = vec![0.2];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 1];
    let context = test_context(1);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // 1.0: excess=0.8, folds=4 (even), remainder=0.0 → 0.2
    assert!((output[0] - 0.2).abs() < 1e-6, "Expected ~0.2, got {}", output[0]);
}

#[test]
fn test_fold_zero_threshold_protection() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![0.5];
    let threshold = vec![0.0];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 1];
    let context = test_context(1);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // Should not crash or produce NaN
    assert!(output[0].is_finite());
}

#[test]
fn test_fold_creates_harmonics() {
    let mut fold = FoldNode::new(0, 1);

    // Sine-like waveform exceeding threshold
    let input: Vec<f32> = (0..512)
        .map(|i| {
            let phase = (i as f32) / 512.0 * 2.0 * std::f32::consts::PI;
            phase.sin() * 1.5
        })
        .collect();
    let threshold = vec![0.5; 512];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 512];
    let context = test_context(512);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // Verify output is different from input (distorted)
    let mut differences = 0;
    for i in 0..512 {
        if (output[i] - input[i]).abs() > 0.01 {
            differences += 1;
        }
    }

    assert!(
        differences > 100,
        "Expected significant waveform distortion, got {} differences",
        differences
    );

    // Verify output is bounded
    for sample in &output {
        assert!(sample.abs() <= 1.0, "Output should be bounded, got {}", sample);
    }
}

#[test]
fn test_fold_full_scale_signal() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![5.0, -5.0, 10.0, -10.0];
    let threshold = vec![1.0; 4];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // All outputs should be within threshold bounds
    for sample in &output {
        assert!(sample.abs() <= 1.0, "Output should be within threshold, got {}", sample);
    }
}

#[test]
fn test_fold_dc_offset() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![2.5, 2.5, 2.5, 2.5];
    let threshold = vec![0.8; 4];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // All outputs should be identical
    let first = output[0];
    for sample in &output {
        assert_eq!(*sample, first, "DC signal should fold consistently");
    }
}

#[test]
fn test_fold_dependencies() {
    let fold = FoldNode::new(5, 10);
    let deps = fold.input_nodes();

    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0], 5);
    assert_eq!(deps[1], 10);
}

#[test]
fn test_fold_progressive_distortion() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![2.0; 4];
    let threshold = vec![0.5, 1.0, 1.5, 3.0];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // Higher threshold reduces distortion
    assert!(output[0].abs() < 1.0);
    assert!(output[1].abs() < 2.0);
    assert!(output[2].abs() < 2.0);
    assert_eq!(output[3], 2.0);
}

#[test]
fn test_fold_fractional_values() {
    let mut fold = FoldNode::new(0, 1);

    let input = vec![0.12, 0.45, 0.78, 1.23];
    let threshold = vec![0.3; 4];
    let inputs = vec![input.as_slice(), threshold.as_slice()];

    let mut output = vec![0.0; 4];
    let context = test_context(4);

    fold.process_block(&inputs, &mut output, 44100.0, &context);

    // 0.12: below threshold, passes through
    assert_eq!(output[0], 0.12);

    // 0.45: excess=0.15, folds=0 (even), remainder=0.15 → 0.3 - 0.15 = 0.15
    assert!((output[1] - 0.15).abs() < 1e-6, "Expected ~0.15, got {}", output[1]);

    // 0.78: excess=0.48, folds=1 (odd), remainder=0.18 → -0.3 + 0.18 = -0.12
    assert!((output[2] - (-0.12)).abs() < 1e-6, "Expected ~-0.12, got {}", output[2]);

    // 1.23: excess=0.93, folds=3 (odd), remainder=0.03 → -0.3 + 0.03 = -0.27
    assert!((output[3] - (-0.27)).abs() < 1e-6, "Expected ~-0.27, got {}", output[3]);
}

#[test]
fn test_fold_with_oscillator() {
    // Integration test: Fold a sine wave
    let block_size = 512;
    let sample_rate = 44100.0;
    let context = test_context(block_size);

    let mut constant_freq = ConstantNode::new(440.0);
    let mut constant_threshold = ConstantNode::new(0.3);
    let mut oscillator = OscillatorNode::new(0, Waveform::Sine);
    let mut fold = FoldNode::new(1, 2);

    let mut freq_buf = vec![0.0; block_size];
    let mut threshold_buf = vec![0.0; block_size];
    let mut osc_buf = vec![0.0; block_size];
    let mut output = vec![0.0; block_size];

    // Generate buffers
    constant_freq.process_block(&[], &mut freq_buf, sample_rate, &context);
    constant_threshold.process_block(&[], &mut threshold_buf, sample_rate, &context);
    oscillator.process_block(&[freq_buf.as_slice()], &mut osc_buf, sample_rate, &context);

    // Apply folding
    let inputs = vec![osc_buf.as_slice(), threshold_buf.as_slice()];
    fold.process_block(&inputs, &mut output, sample_rate, &context);

    // Verify output has content
    let rms = calculate_rms(&output);
    assert!(rms > 0.1, "Folded sine should have significant energy");

    // Verify output is bounded
    for sample in &output {
        assert!(sample.abs() <= 0.5, "Output should be bounded by folding");
    }
}
