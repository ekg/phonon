/// Transient detector node - detects rapid amplitude changes and outputs trigger pulses
///
/// A transient detector identifies sudden changes in signal amplitude, such as
/// drum hits, percussive attacks, or discontinuities in waveforms. It outputs
/// a pulse (1.0) when a transient is detected, and 0.0 otherwise.
///
/// # Algorithm
/// ```text
/// diff = |current_sample - previous_sample|
/// If diff > threshold:
///   output = 1.0  // Transient detected
/// Else:
///   output = 0.0  // No transient
/// ```
///
/// # Applications
/// - Drum/percussion trigger detection
/// - Envelope triggering on transients
/// - Onset detection for rhythmic analysis
/// - Dynamic effects triggering (reverb on snare hits, etc.)
/// - Detecting discontinuities in waveforms
///
/// # Example
/// ```ignore
/// // Detect transients in audio signal
/// let audio = SampleNode::new("drums.wav");     // NodeId 1
/// let threshold = ConstantNode::new(0.1);        // NodeId 2 (10% change)
/// let detector = TransientNode::new(1, 2);       // NodeId 3
/// // Output: pulses on drum hits
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Transient detector state
#[derive(Debug, Clone)]
struct TransientState {
    last_value: f32,  // Previous sample value for difference calculation
}

impl Default for TransientState {
    fn default() -> Self {
        Self { last_value: 0.0 }
    }
}

/// Transient detector node: detects rapid amplitude changes
///
/// Outputs a pulse when the amplitude difference between consecutive
/// samples exceeds the threshold, useful for detecting drum hits and
/// percussive transients.
pub struct TransientNode {
    input: NodeId,
    threshold_input: NodeId,  // Threshold for transient detection (0.0 to 1.0)
    state: TransientState,
}

impl TransientNode {
    /// Create a new transient detector node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to analyze
    /// * `threshold_input` - NodeId of threshold (typical range: 0.01 to 0.5)
    ///
    /// # Threshold Guidelines
    /// - 0.01-0.05: Very sensitive (detects subtle changes)
    /// - 0.1-0.2: Medium sensitivity (typical drum hits)
    /// - 0.3-0.5: Low sensitivity (only very loud transients)
    pub fn new(input: NodeId, threshold_input: NodeId) -> Self {
        Self {
            input,
            threshold_input,
            state: TransientState::default(),
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the threshold input node ID
    pub fn threshold_input(&self) -> NodeId {
        self.threshold_input
    }

    /// Reset transient detector state
    pub fn reset(&mut self) {
        self.state = TransientState::default();
    }

    /// Get last sample value (for debugging/testing)
    pub fn last_value(&self) -> f32 {
        self.state.last_value
    }
}

impl AudioNode for TransientNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "TransientNode requires 2 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let threshold_buf = inputs[1];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let sample = input_buf[i].abs(); // Use absolute value for amplitude
            let threshold = threshold_buf[i].max(0.0); // Ensure non-negative

            // Calculate absolute difference from previous sample
            let diff = (sample - self.state.last_value).abs();

            // Detect transient: output pulse if difference exceeds threshold
            output[i] = if diff > threshold { 1.0 } else { 0.0 };

            // Update last_value for next sample
            self.state.last_value = sample;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.threshold_input]
    }

    fn name(&self) -> &str {
        "TransientNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
    }

    #[test]
    fn test_transient_detects_sharp_change() {
        // Test that transient detector triggers on sharp amplitude changes
        let size = 512;

        // Signal: quiet, then sudden loud spike
        let mut input = vec![0.1; size];
        input[256] = 0.8; // Sharp transient at midpoint

        let threshold = vec![0.1; size]; // Threshold: 10% change

        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);
        detector.process_block(&inputs, &mut output, 44100.0, &context);

        // Before transient: no triggers
        for i in 0..255 {
            assert_eq!(
                output[i], 0.0,
                "No transient before spike at sample {}",
                i
            );
        }

        // At transient: trigger
        assert_eq!(
            output[256], 1.0,
            "Transient should be detected at spike"
        );

        // After transient: no more triggers (level stays constant)
        let after_count = output[257..].iter().filter(|&&x| x > 0.5).count();
        assert_eq!(
            after_count, 0,
            "Should have no transients after spike settles"
        );
    }

    #[test]
    fn test_transient_detects_falling_edge() {
        // Test that transient detector also catches falling transients
        let size = 512;

        // Signal: loud, then sudden drop
        let mut input = vec![0.8; size];
        input[256] = 0.1; // Sharp drop at midpoint

        let threshold = vec![0.1; size];

        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);
        detector.process_block(&inputs, &mut output, 44100.0, &context);

        // At drop: should detect transient
        assert_eq!(
            output[256], 1.0,
            "Transient should be detected at sharp drop"
        );
    }

    #[test]
    fn test_transient_threshold_sensitivity() {
        // Test that threshold controls sensitivity
        let size = 512;

        // Signal with small change
        let mut input = vec![0.5; size];
        input[256] = 0.6; // Small change: 0.1

        // Low threshold (sensitive)
        let threshold_low = vec![0.05; size];
        let inputs_low: Vec<&[f32]> = vec![&input, &threshold_low];
        let mut output_low = vec![0.0; size];
        let context = create_context(size);

        let mut detector_low = TransientNode::new(0, 1);
        detector_low.process_block(&inputs_low, &mut output_low, 44100.0, &context);

        // Should trigger with low threshold
        assert_eq!(
            output_low[256], 1.0,
            "Low threshold should detect small change"
        );

        // High threshold (less sensitive)
        let threshold_high = vec![0.2; size];
        let inputs_high: Vec<&[f32]> = vec![&input, &threshold_high];
        let mut output_high = vec![0.0; size];

        let mut detector_high = TransientNode::new(0, 1);
        detector_high.process_block(&inputs_high, &mut output_high, 44100.0, &context);

        // Should NOT trigger with high threshold
        assert_eq!(
            output_high[256], 0.0,
            "High threshold should NOT detect small change"
        );
    }

    #[test]
    fn test_transient_multiple_hits() {
        // Test detection of multiple transients in one block
        let size = 512;

        // Signal with multiple spikes
        let mut input = vec![0.1; size];
        input[100] = 0.8; // First hit
        input[200] = 0.8; // Second hit
        input[300] = 0.8; // Third hit

        let threshold = vec![0.1; size];

        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);
        detector.process_block(&inputs, &mut output, 44100.0, &context);

        // Count total transients detected
        let transient_count = output.iter().filter(|&&x| x > 0.5).count();

        // Should detect 3 rising edges + 3 falling edges = 6 transients
        assert!(
            transient_count >= 3 && transient_count <= 6,
            "Should detect 3-6 transients (rising and falling edges), got {}",
            transient_count
        );

        // Specifically check the rising edges
        assert_eq!(output[100], 1.0, "First rising transient at 100");
        assert_eq!(output[200], 1.0, "Second rising transient at 200");
        assert_eq!(output[300], 1.0, "Third rising transient at 300");
    }

    #[test]
    fn test_transient_smooth_signal() {
        // Test that smooth signals don't trigger transients
        let size = 512;

        // Smooth sine-like ramp
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = 0.5 + 0.3 * ((i as f32 / size as f32) * std::f32::consts::PI * 2.0).sin();
        }

        let threshold = vec![0.05; size]; // Sensitive threshold

        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);
        detector.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have very few or no transients (smooth changes)
        let transient_count = output.iter().filter(|&&x| x > 0.5).count();
        assert!(
            transient_count < 10,
            "Smooth signal should have few transients, got {}",
            transient_count
        );
    }

    #[test]
    fn test_transient_drum_hit_simulation() {
        // Simulate a drum hit: sudden spike followed by decay
        let size = 512;
        let mut input = vec![0.0; size];

        // Drum hit at sample 100: sudden attack + exponential decay
        for i in 100..size {
            let t = (i - 100) as f32;
            input[i] = 0.8 * (-t / 50.0).exp(); // Exponential decay
        }

        let threshold = vec![0.1; size];

        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);
        detector.process_block(&inputs, &mut output, 44100.0, &context);

        // Should detect transient at attack (sample 100)
        assert_eq!(
            output[100], 1.0,
            "Should detect drum hit attack at sample 100"
        );

        // Should have few or no transients during smooth decay
        let decay_transients = output[105..].iter().filter(|&&x| x > 0.5).count();
        assert!(
            decay_transients < 5,
            "Should have minimal transients during decay, got {}",
            decay_transients
        );
    }

    #[test]
    fn test_transient_node_interface() {
        // Test node getters
        let detector = TransientNode::new(7, 8);

        assert_eq!(detector.input(), 7);
        assert_eq!(detector.threshold_input(), 8);

        let inputs = detector.input_nodes();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], 7);
        assert_eq!(inputs[1], 8);

        assert_eq!(detector.name(), "TransientNode");
    }

    #[test]
    fn test_transient_reset() {
        // Test that reset clears state
        let size = 512;

        let input = vec![0.5; size];
        let threshold = vec![0.1; size];
        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);

        // Process to build up state
        detector.process_block(&inputs, &mut output, 44100.0, &context);
        let last_before = detector.last_value();
        assert!(last_before > 0.0, "State should have been updated");

        // Reset
        detector.reset();
        assert_eq!(detector.last_value(), 0.0, "State should be cleared after reset");
    }

    #[test]
    fn test_transient_uses_absolute_value() {
        // Test that detector uses absolute value (negative spike detected)
        let size = 512;

        // Signal with negative spike
        let mut input = vec![0.1; size];
        input[256] = -0.8; // Negative spike (should be treated as 0.8 absolute)

        let threshold = vec![0.1; size];

        let inputs: Vec<&[f32]> = vec![&input, &threshold];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut detector = TransientNode::new(0, 1);
        detector.process_block(&inputs, &mut output, 44100.0, &context);

        // Should detect transient at negative spike
        assert_eq!(
            output[256], 1.0,
            "Should detect transient at negative spike (using absolute value)"
        );
    }
}
