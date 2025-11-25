/// Tests for NoiseGateNode - smooth noise gate with attack/release
///
/// These tests verify all aspects of the noise gate:
/// - Signals above threshold pass through
/// - Signals below threshold are gated (silenced)
/// - Attack time controls how fast the gate opens
/// - Release time controls how fast the gate closes
/// - Threshold variations affect gating behavior
/// - Edge cases (zero input, boundary conditions)

use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::nodes::NoiseGateNode;
use phonon::pattern::Fraction;

fn create_context(size: usize) -> ProcessContext {
    ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
}

#[test]
fn test_noise_gate_above_threshold_passes() {
    // Signal above threshold should pass through
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // Input: 0.5 (-6 dB), Threshold: -20 dB
    let input = vec![0.5; 512];
    let threshold = vec![-20.0; 512];
    let attack = vec![0.001; 512];   // Fast attack
    let release = vec![0.1; 512];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Signal well above threshold should pass with minimal attenuation
    let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
    assert!(avg_output > 0.4, "Average output was {}, expected > 0.4", avg_output);
}

#[test]
fn test_noise_gate_below_threshold_silenced() {
    // Signal below threshold should be gated/silenced
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // Input: 0.01 (-40 dB), Threshold: -20 dB
    let input = vec![0.01; 512];
    let threshold = vec![-20.0; 512];
    let attack = vec![0.01; 512];
    let release = vec![0.001; 512];  // Fast release
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Signal below threshold should be heavily attenuated
    let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
    assert!(avg_output < 0.005, "Average output was {}, expected < 0.005", avg_output);
}

#[test]
fn test_noise_gate_attack_controls_opening_speed() {
    // Slow attack should have gradual gate opening
    let mut gate_slow = NoiseGateNode::new(0, 1, 2, 3);
    let mut gate_fast = NoiseGateNode::new(0, 1, 2, 3);

    // Signal above threshold
    let input = vec![0.5; 512];
    let threshold = vec![-20.0; 512];
    let release = vec![0.1; 512];

    // Slow attack
    let attack_slow = vec![0.02; 512];  // 20ms (realistic)
    let inputs_slow = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack_slow.as_slice(),
        release.as_slice(),
    ];

    let mut output_slow = vec![0.0; 512];
    let context = create_context(512);
    gate_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

    // Fast attack
    let attack_fast = vec![0.001; 512];  // 1ms
    let inputs_fast = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack_fast.as_slice(),
        release.as_slice(),
    ];

    let mut output_fast = vec![0.0; 512];
    gate_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);

    // Fast attack should reach fuller level sooner than slow attack
    // Check early samples (first ~0.5ms)
    let early_slow: f32 = output_slow.iter().skip(10).take(20).sum::<f32>() / 20.0;
    let early_fast: f32 = output_fast.iter().skip(10).take(20).sum::<f32>() / 20.0;

    assert!(early_fast > early_slow * 1.2,
        "Fast attack {} should be significantly > slow attack {}", early_fast, early_slow);

    // Slow attack hasn't fully opened yet in 512 samples (~11.6ms), but should be getting there
    let late_slow: f32 = output_slow.iter().skip(400).take(100).sum::<f32>() / 100.0;
    let late_fast: f32 = output_fast.iter().skip(400).take(100).sum::<f32>() / 100.0;

    // Slow attack should be at least 30% of fast attack by end of buffer
    assert!(late_slow > late_fast * 0.3,
        "Slow attack should be opening: slow={}, fast={}", late_slow, late_fast);
}

#[test]
fn test_noise_gate_release_controls_closing_speed() {
    // Test release phase by going from loud to quiet
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // First half loud, second half quiet
    let mut input = vec![0.5; 512];
    for i in 256..512 {
        input[i] = 0.001; // Very quiet (-60 dB)
    }

    let threshold = vec![-20.0; 512];
    let attack = vec![0.001; 512];  // Fast attack
    let release = vec![0.1; 512];   // Slow release
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // After transition to quiet, should gradually close
    // Sample right after transition should be higher than later samples
    let just_after: f32 = output.iter().skip(260).take(20).sum::<f32>() / 20.0;
    let much_later: f32 = output.iter().skip(400).take(50).sum::<f32>() / 50.0;

    assert!(just_after > much_later, "Just after {} should be > much later {}", just_after, much_later);
}

#[test]
fn test_noise_gate_threshold_boundary() {
    // Test signal near threshold boundary
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // Input slightly above threshold to test transition region
    // Threshold: -10 dB = 0.316, Input: -9 dB = 0.355
    let input = vec![0.355; 512];
    let threshold = vec![-10.0; 512];
    let attack = vec![0.005; 512];  // Medium attack
    let release = vec![0.1; 512];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Slightly above threshold, should open and pass signal
    let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
    assert!(avg_output > 0.2, "Should have significant signal through, got {}", avg_output);
    assert!(avg_output < 0.4, "Should not be fully open due to attack time, got {}", avg_output);
}

#[test]
fn test_noise_gate_preserves_sign() {
    // Gate should preserve positive and negative signs
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    let input = vec![0.5, -0.5, 0.4, -0.4];
    let threshold = vec![-20.0; 4];
    let attack = vec![0.001; 4];
    let release = vec![0.1; 4];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 4];
    let context = create_context(4);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Check signs are preserved
    assert!(output[0] > 0.0, "Positive input should remain positive");
    assert!(output[1] < 0.0, "Negative input should remain negative");
    assert!(output[2] > 0.0, "Positive input should remain positive");
    assert!(output[3] < 0.0, "Negative input should remain negative");
}

#[test]
fn test_noise_gate_varying_threshold() {
    // Test with varying threshold parameter
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    let input = vec![0.1; 512];
    let mut threshold = vec![-30.0; 512]; // Low threshold (gate open)
    for i in 256..512 {
        threshold[i] = -10.0; // High threshold (gate closed for 0.1 signal)
    }
    let attack = vec![0.001; 512];
    let release = vec![0.001; 512];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Low threshold = gate open = higher level
    let avg_low_thresh: f32 = output.iter().skip(50).take(100).sum::<f32>() / 100.0;
    // High threshold = gate closed = lower level
    let avg_high_thresh: f32 = output.iter().skip(300).take(100).sum::<f32>() / 100.0;

    assert!(avg_low_thresh > avg_high_thresh,
        "Low threshold {} should pass more signal than high threshold {}",
        avg_low_thresh, avg_high_thresh);
}

#[test]
fn test_noise_gate_zero_input_safe() {
    // Verify gate handles zero/very quiet input safely
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    let input = vec![0.0, 0.00001, -0.00001, 0.0];
    let threshold = vec![-20.0; 4];
    let attack = vec![0.01; 4];
    let release = vec![0.1; 4];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 4];
    let context = create_context(4);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Should not produce NaN or infinity
    for sample in &output {
        assert!(sample.is_finite(), "Sample {} is not finite", sample);
    }
}

#[test]
fn test_noise_gate_removes_noise_floor() {
    // Realistic scenario: signal with noise floor
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // Loud signal with quiet noise floor
    let mut input = vec![0.5; 256];      // Loud part
    input.extend(vec![0.01; 256]);       // Noise floor

    let threshold = vec![-30.0; 512];    // Between signal and noise
    let attack = vec![0.001; 512];       // Fast attack
    let release = vec![0.05; 512];       // Medium release
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Loud part should pass
    let avg_loud: f32 = output.iter().skip(50).take(100).sum::<f32>() / 100.0;
    assert!(avg_loud > 0.4, "Loud signal should pass, got {}", avg_loud);

    // Noise floor should be suppressed
    let avg_noise: f32 = output.iter().skip(400).take(100).sum::<f32>() / 100.0;
    assert!(avg_noise < 0.01, "Noise floor should be suppressed, got {}", avg_noise);

    // Signal should be much louder than noise
    assert!(avg_loud > avg_noise * 20.0, "Signal/noise ratio should be high");
}

#[test]
fn test_noise_gate_fast_attack_slow_release() {
    // Common production setting: fast attack, slow release
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // Constant loud signal followed by quiet signal
    let mut input = vec![0.8; 256];      // Loud part
    input.extend(vec![0.001; 256]);      // Very quiet part (below threshold)

    let threshold = vec![-20.0; 512];
    let attack = vec![0.001; 512];       // Very fast attack (1ms)
    let release = vec![0.05; 512];       // Medium release (50ms)
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // First part should be well open
    let first_part: f32 = output.iter().skip(100).take(100).sum::<f32>() / 100.0;
    assert!(first_part > 0.6, "Should be well open, got {}", first_part);

    // After transition to quiet, should gradually close
    // Right after transition (sample ~256)
    let right_after: f32 = output.iter().skip(260).take(20).sum::<f32>() / 20.0;
    // Much later (sample ~400)
    let much_later: f32 = output.iter().skip(400).take(50).sum::<f32>() / 50.0;

    // Gate should be closing: much_later should be less than right_after
    assert!(much_later < right_after,
        "Gate should be closing: right_after={}, much_later={}", right_after, much_later);
}

#[test]
fn test_noise_gate_dependencies() {
    let gate = NoiseGateNode::new(5, 10, 15, 20);
    let deps = gate.input_nodes();

    assert_eq!(deps.len(), 4);
    assert_eq!(deps[0], 5);   // input
    assert_eq!(deps[1], 10);  // threshold
    assert_eq!(deps[2], 15);  // attack
    assert_eq!(deps[3], 20);  // release
}

#[test]
fn test_noise_gate_reset() {
    // Verify reset clears envelope state
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    // Process some audio to build up envelope
    let input = vec![0.5; 512];
    let threshold = vec![-20.0; 512];
    let attack = vec![0.1; 512];
    let release = vec![0.1; 512];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Reset and process again
    gate.reset();

    let mut output2 = vec![0.0; 512];
    gate.process_block(&inputs, &mut output2, 44100.0, &context);

    // First few samples should be similar (envelope starts from zero both times)
    let early1: f32 = output.iter().skip(0).take(10).sum::<f32>() / 10.0;
    let early2: f32 = output2.iter().skip(0).take(10).sum::<f32>() / 10.0;

    assert!((early1 - early2).abs() < 0.01, "Reset should clear state");
}

#[test]
fn test_noise_gate_extreme_threshold_high() {
    // Very high threshold should gate everything
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    let input = vec![0.9; 512];
    let threshold = vec![10.0; 512];  // Very high (above 0 dB)
    let attack = vec![0.01; 512];
    let release = vec![0.01; 512];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Everything should be gated
    let avg_output: f32 = output.iter().sum::<f32>() / 512.0;
    assert!(avg_output < 0.1, "High threshold should gate everything, got {}", avg_output);
}

#[test]
fn test_noise_gate_extreme_threshold_low() {
    // Very low threshold should pass everything
    let mut gate = NoiseGateNode::new(0, 1, 2, 3);

    let input = vec![0.01; 512];  // Very quiet
    let threshold = vec![-80.0; 512];  // Very low
    let attack = vec![0.001; 512];
    let release = vec![0.1; 512];
    let inputs = vec![
        input.as_slice(),
        threshold.as_slice(),
        attack.as_slice(),
        release.as_slice(),
    ];

    let mut output = vec![0.0; 512];
    let context = create_context(512);

    gate.process_block(&inputs, &mut output, 44100.0, &context);

    // Quiet signal should pass
    let avg_output: f32 = output.iter().skip(100).take(400).sum::<f32>() / 400.0;
    assert!(avg_output > 0.008, "Low threshold should pass signal, got {}", avg_output);
}
