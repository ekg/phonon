/// Peak detector node - tracks peak values with configurable decay
///
/// This node monitors an input signal and tracks the peak (maximum absolute value),
/// with configurable decay time when the signal drops below the current peak.
///
/// Common uses:
/// - Envelope following for compression/gating
/// - VU meter emulation
/// - Adaptive gain control
/// - Peak normalization
/// - Audio analysis and visualization

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Peak detector node: tracks peak values with decay
///
/// The algorithm:
/// ```text
/// for each sample:
///     sample_abs = |input[i]|
///     decay_time = max(0.001, decay_time_input[i])
///
///     if sample_abs > current_peak:
///         current_peak = sample_abs  // Instant attack on new peak
///     else:
///         // Decay toward zero
///         decay_per_sample = 1.0 / (decay_time * sample_rate)
///         current_peak = max(0.0, current_peak - decay_per_sample)
///
///     output[i] = current_peak
/// ```
///
/// # Example
/// ```ignore
/// // Track peaks of an oscillator with 0.5 second decay
/// let osc = OscillatorNode::new(...);           // NodeId 0
/// let decay = ConstantNode::new(0.5);           // NodeId 1 (0.5 seconds)
/// let peak = PeakDetectorNode::new(0, 1);       // NodeId 2
/// // Output will follow the peak envelope of the oscillator
/// ```
pub struct PeakDetectorNode {
    /// Input signal to track peaks from
    input: NodeId,

    /// Decay time in seconds (how long to decay from peak to zero)
    decay_time_input: NodeId,

    /// Currently tracked peak value
    current_peak: f32,
}

impl PeakDetectorNode {
    /// Create a new peak detector node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to track peaks from
    /// * `decay_time_input` - NodeId of decay time signal (in seconds)
    ///
    /// # Initial State
    /// - `current_peak` starts at 0.0
    pub fn new(input: NodeId, decay_time_input: NodeId) -> Self {
        Self {
            input,
            decay_time_input,
            current_peak: 0.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the decay time input node ID
    pub fn decay_time_input(&self) -> NodeId {
        self.decay_time_input
    }

    /// Get the current peak value (for debugging/testing)
    pub fn current_peak(&self) -> f32 {
        self.current_peak
    }
}

impl AudioNode for PeakDetectorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "PeakDetectorNode requires 2 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let decay_time_buf = inputs[1];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            decay_time_buf.len(),
            output.len(),
            "Decay time buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let sample = input_buf[i].abs();
            let decay_time = decay_time_buf[i].max(0.00001); // Prevent division by zero (10 microseconds min)

            // Update peak
            if sample > self.current_peak {
                // New peak - instant attack
                self.current_peak = sample;
            } else {
                // Decay toward zero
                let decay_per_sample = 1.0 / (decay_time * sample_rate);
                self.current_peak = (self.current_peak - decay_per_sample).max(0.0);
            }

            output[i] = self.current_peak;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.decay_time_input]
    }

    fn name(&self) -> &str {
        "PeakDetectorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_peak_detector_tracks_increasing_signal() {
        let mut peak = PeakDetectorNode::new(0, 1);

        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let decay_time = vec![1.0; 5]; // 1 second decay
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // Should track each increasing value
        assert!((output[0] - 0.1).abs() < 0.001);
        assert!((output[1] - 0.2).abs() < 0.001);
        assert!((output[2] - 0.3).abs() < 0.001);
        assert!((output[3] - 0.4).abs() < 0.001);
        assert!((output[4] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_peak_detector_holds_peak() {
        let mut peak = PeakDetectorNode::new(0, 1);

        // Peak at index 0, then silence
        let input = vec![1.0, 0.0, 0.0, 0.0];
        let decay_time = vec![100.0; 4]; // Very long decay (100 seconds)
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample captures peak
        assert!((output[0] - 1.0).abs() < 0.001);

        // With 100 second decay, should barely decay over 3 samples
        // Decay per sample = 1.0 / (100 * 44100) = 0.00000022675
        // After 3 samples: 1.0 - 3 * 0.00000022675 ≈ 0.99999932
        assert!(output[1] > 0.999);
        assert!(output[2] > 0.999);
        assert!(output[3] > 0.999);
    }

    #[test]
    fn test_peak_detector_decays_after_peak() {
        let mut peak = PeakDetectorNode::new(0, 1);

        // Peak followed by zeros, with fast decay
        let input = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let decay_time = vec![0.0001; 5]; // Very fast decay (0.1 ms)
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample should be at peak
        assert!((output[0] - 1.0).abs() < 0.001);

        // Decay per sample = 1.0 / (0.0001 * 44100) = 0.2267
        // Should decay significantly each sample
        assert!(output[1] < 0.9, "output[1] = {}", output[1]);
        assert!(output[2] < output[1], "output[2] = {}, output[1] = {}", output[2], output[1]);
        assert!(output[3] < output[2], "output[3] = {}, output[2] = {}", output[3], output[2]);
        assert!(output[4] < output[3], "output[4] = {}, output[3] = {}", output[4], output[3]);
    }

    #[test]
    fn test_peak_detector_fast_decay() {
        let mut peak = PeakDetectorNode::new(0, 1);

        // High peak, then zeros with very fast decay
        let input = vec![2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let decay_time = vec![0.0001; 8]; // 0.1 ms decay
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // Peak should be captured
        assert!((output[0] - 2.0).abs() < 0.001);

        // With decay_per_sample ≈ 0.2267, should decay rapidly
        // After 8 samples at 0.2267/sample: 2.0 - 8*0.2267 ≈ 0.186
        // (Actually it's exponential-ish since we clamp at 0, but should be close)
        assert!(output[7] < 0.5, "output[7] = {}", output[7]);
    }

    #[test]
    fn test_peak_detector_slow_decay() {
        let mut peak = PeakDetectorNode::new(0, 1);

        // Peak followed by zeros with slow decay
        let input = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let decay_time = vec![10.0; 5]; // 10 second decay
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // Peak captured
        assert!((output[0] - 1.0).abs() < 0.001);

        // Decay per sample = 1.0 / (10 * 44100) = 0.0000022675
        // After 4 samples: 1.0 - 4 * 0.0000022675 ≈ 0.999991
        // Should decay very slowly
        assert!(output[1] > 0.9999, "output[1] = {}", output[1]);
        assert!(output[2] > 0.9999, "output[2] = {}", output[2]);
        assert!(output[3] > 0.9999, "output[3] = {}", output[3]);
        assert!(output[4] > 0.9999, "output[4] = {}", output[4]);

        // But should still be decaying (each sample slightly less)
        assert!(output[1] < output[0]);
        assert!(output[2] < output[1]);
        assert!(output[3] < output[2]);
        assert!(output[4] < output[3]);
    }

    #[test]
    fn test_peak_detector_dependencies() {
        let peak = PeakDetectorNode::new(3, 7);
        let deps = peak.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 3);
        assert_eq!(deps[1], 7);
    }

    #[test]
    fn test_peak_detector_with_oscillator() {
        use std::f32::consts::PI;
        let mut peak = PeakDetectorNode::new(0, 1);

        // Create a sine wave burst (3 cycles) followed by silence
        let mut input = Vec::new();
        for i in 0..32 {
            if i < 24 {
                // 3 cycles of sine at 16 samples/cycle
                let phase = (i as f32 / 16.0) * 2.0 * PI;
                input.push(phase.sin());
            } else {
                input.push(0.0);
            }
        }

        let decay_time = vec![0.001; 32]; // 1 ms decay
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 32];
        let context = create_context(32);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // Peak should reach close to 1.0 (sine wave maximum)
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b));
        assert!(max_output > 0.95, "max_output = {}", max_output);

        // During silence (samples 24-31), should decay
        assert!(output[31] < output[24],
                "output[31] = {}, output[24] = {}",
                output[31], output[24]);
    }

    #[test]
    fn test_peak_detector_negative_values() {
        let mut peak = PeakDetectorNode::new(0, 1);

        // Test with negative values (should use absolute value)
        let input = vec![-0.5, -1.0, -0.3, 0.0];
        let decay_time = vec![1.0; 4];
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // Should track absolute values
        assert!((output[0] - 0.5).abs() < 0.001);
        assert!((output[1] - 1.0).abs() < 0.001); // Peak at |-1.0| = 1.0
        assert!(output[2] < 1.0); // Should start decaying
    }

    #[test]
    fn test_peak_detector_state_persistence() {
        // Test that peak state persists across multiple process_block calls
        let mut peak = PeakDetectorNode::new(0, 1);

        // First block: establish peak
        let input1 = vec![0.8, 0.9, 1.0];
        let decay1 = vec![10.0; 3]; // Slow decay
        let inputs1 = vec![input1.as_slice(), decay1.as_slice()];
        let mut output1 = vec![0.0; 3];
        let context = create_context(3);

        peak.process_block(&inputs1, &mut output1, 44100.0, &context);
        assert!((output1[2] - 1.0).abs() < 0.001); // Peak at 1.0

        // Second block: lower signal, should hold/decay from 1.0
        let input2 = vec![0.0, 0.0, 0.0];
        let decay2 = vec![10.0; 3];
        let inputs2 = vec![input2.as_slice(), decay2.as_slice()];
        let mut output2 = vec![0.0; 3];

        peak.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should still be close to 1.0 (slow decay)
        assert!(output2[0] > 0.999, "output2[0] = {}", output2[0]);
        assert!(output2[0] < 1.0); // But decaying
    }

    #[test]
    fn test_peak_detector_multiple_peaks() {
        let mut peak = PeakDetectorNode::new(0, 1);

        // Multiple peaks: 0.5, decay, 0.8, decay, 1.0
        let input = vec![0.5, 0.3, 0.2, 0.8, 0.4, 0.2, 1.0, 0.5];
        let decay_time = vec![0.001; 8]; // Fast decay
        let inputs = vec![input.as_slice(), decay_time.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        peak.process_block(&inputs, &mut output, 44100.0, &context);

        // First peak
        assert!((output[0] - 0.5).abs() < 0.001);

        // Second peak (should jump to 0.8)
        assert!((output[3] - 0.8).abs() < 0.001);

        // Third peak (should jump to 1.0)
        assert!((output[6] - 1.0).abs() < 0.001);
    }
}
