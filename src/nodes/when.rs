/// When node - conditional signal routing (audio if-statement)
///
/// This node implements conditional signal routing based on a boolean condition.
/// It acts like an if-statement for audio signals, routing one of two inputs to
/// the output based on whether the condition is above or below a threshold.
///
/// # Algorithm
///
/// ```text
/// if condition[i] > threshold:
///     output[i] = true_input[i]
/// else:
///     output[i] = false_input[i]
/// ```
///
/// The threshold is fixed at 0.5, following the convention used by other
/// comparison nodes in Phonon (where 1.0 = true, 0.0 = false).
///
/// # Use Cases
///
/// ## Musical Gate/Mute
/// ```ignore
/// // Mute signal when LFO is low
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let lfo = OscillatorNode::new(2, Waveform::Sine);       // NodeId 3 (0.1 Hz)
/// let silence = ConstantNode::new(0.0);                   // NodeId 4
/// let when = WhenNode::new(3, 1, 4);  // If LFO > 0.5, play signal, else silence
/// ```
///
/// ## Pattern-Based Routing
/// ```ignore
/// // Switch between two synth patches based on pattern
/// let synth_a = OscillatorNode::new(0, Waveform::Sine);
/// let synth_b = OscillatorNode::new(2, Waveform::Saw);
/// let pattern = PatternNode::new("0 1 0 1");  // Alternating 0/1 pattern
/// let when = WhenNode::new(4, 2, 0);  // Route synth_b when pattern=1, synth_a when pattern=0
/// ```
///
/// ## Dynamic Mix Control
/// ```ignore
/// // Crossfade between signals based on analysis
/// let track_a = ...;
/// let track_b = ...;
/// let energy = RMSNode::new(...);  // Analyze input energy
/// let when = WhenNode::new(energy, track_b, track_a);  // High energy → track B
/// ```
///
/// ## Audio-Rate Conditions
/// ```ignore
/// // Waveshaping: rectify positive half of waveform
/// let signal = OscillatorNode::new(0, Waveform::Sine);
/// let zero = ConstantNode::new(0.0);
/// let positive = GreaterThanNode::new(signal, zero);  // 1.0 when signal > 0
/// let when = WhenNode::new(positive, signal, zero);  // Half-wave rectifier
/// ```
///
/// # Comparison with Other Nodes
///
/// - **XFadeNode**: Smooth crossfade with position control (0.0 to 1.0)
/// - **WhenNode**: Binary switch based on threshold (hard transition)
/// - **LerpNode**: Linear interpolation without clamping
/// - **GateNode**: Amplitude-based gating (different use case)
///
/// # Threshold Behavior
///
/// The threshold is fixed at 0.5, which means:
/// - `condition > 0.5` → route true_input
/// - `condition <= 0.5` → route false_input
///
/// This aligns with Phonon's boolean convention where comparison nodes
/// output 1.0 for true and 0.0 for false.
///
/// # Performance Notes
///
/// - Zero-copy routing (just selects which buffer to read from)
/// - Sample-accurate switching (no interpolation)
/// - Hard transitions may cause clicks (use XFadeNode for smooth transitions)
/// - Consider adding slew limiting to condition signal if clicks are undesirable
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// When node: conditional signal router
///
/// Routes true_input or false_input to output based on condition > 0.5.
pub struct WhenNode {
    /// Condition signal (>0.5 = true)
    condition: NodeId,
    /// Signal when condition is true
    true_input: NodeId,
    /// Signal when condition is false
    false_input: NodeId,
    /// Threshold for condition (fixed at 0.5)
    threshold: f32,
}

impl WhenNode {
    /// When - Conditional signal router (audio if-statement)
    ///
    /// Routes true_input or false_input to output based on condition > 0.5.
    /// Enables conditional audio flow and pattern-based signal switching.
    ///
    /// # Parameters
    /// - `condition`: Condition signal (compared against 0.5 threshold)
    /// - `true_input`: Signal when condition > 0.5
    /// - `false_input`: Signal when condition <= 0.5
    ///
    /// # Example
    /// ```phonon
    /// ~gate: lfo 1.0 0 1
    /// ~on_signal: sine 440
    /// ~off_signal: 0
    /// out: when ~gate ~on_signal ~off_signal
    /// ```
    pub fn new(condition: NodeId, true_input: NodeId, false_input: NodeId) -> Self {
        Self {
            condition,
            true_input,
            false_input,
            threshold: 0.5,
        }
    }

    /// Create a new when node with custom threshold
    ///
    /// # Arguments
    /// * `condition` - Condition signal (NodeId)
    /// * `true_input` - Signal to route when condition > threshold (NodeId)
    /// * `false_input` - Signal to route when condition <= threshold (NodeId)
    /// * `threshold` - Custom threshold value
    ///
    /// # Example
    /// ```ignore
    /// // Route based on RMS level (threshold = 0.1)
    /// let rms = RMSNode::new(input);
    /// let loud = OscillatorNode::new(220.0, Waveform::Saw);
    /// let quiet = OscillatorNode::new(110.0, Waveform::Sine);
    /// let when = WhenNode::with_threshold(rms, loud, quiet, 0.1);
    /// ```
    pub fn with_threshold(
        condition: NodeId,
        true_input: NodeId,
        false_input: NodeId,
        threshold: f32,
    ) -> Self {
        Self {
            condition,
            true_input,
            false_input,
            threshold,
        }
    }

    /// Get the condition input node ID
    pub fn condition(&self) -> NodeId {
        self.condition
    }

    /// Get the true input node ID
    pub fn true_input(&self) -> NodeId {
        self.true_input
    }

    /// Get the false input node ID
    pub fn false_input(&self) -> NodeId {
        self.false_input
    }

    /// Get the threshold value
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

impl AudioNode for WhenNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            3,
            "WhenNode expects 3 inputs (condition, true_input, false_input), got {}",
            inputs.len()
        );

        let condition = inputs[0];
        let true_sig = inputs[1];
        let false_sig = inputs[2];

        debug_assert_eq!(
            condition.len(),
            output.len(),
            "Condition buffer length mismatch"
        );
        debug_assert_eq!(
            true_sig.len(),
            output.len(),
            "True input buffer length mismatch"
        );
        debug_assert_eq!(
            false_sig.len(),
            output.len(),
            "False input buffer length mismatch"
        );

        // Sample-accurate conditional routing
        for i in 0..output.len() {
            output[i] = if condition[i] > self.threshold {
                true_sig[i]
            } else {
                false_sig[i]
            };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.condition, self.true_input, self.false_input]
    }

    fn name(&self) -> &str {
        "WhenNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_when_condition_high_routes_to_true_input() {
        // When condition > 0.5, should route true_input
        let mut when = WhenNode::new(0, 1, 2);

        let condition = vec![1.0, 1.0, 1.0, 1.0];
        let true_input = vec![100.0, 200.0, 300.0, 400.0];
        let false_input = vec![10.0, 20.0, 30.0, 40.0];
        let inputs = vec![
            condition.as_slice(),
            true_input.as_slice(),
            false_input.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // Should output true_input
        assert_eq!(output[0], 100.0);
        assert_eq!(output[1], 200.0);
        assert_eq!(output[2], 300.0);
        assert_eq!(output[3], 400.0);
    }

    #[test]
    fn test_when_condition_low_routes_to_false_input() {
        // When condition <= 0.5, should route false_input
        let mut when = WhenNode::new(0, 1, 2);

        let condition = vec![0.0, 0.0, 0.0, 0.0];
        let true_input = vec![100.0, 200.0, 300.0, 400.0];
        let false_input = vec![10.0, 20.0, 30.0, 40.0];
        let inputs = vec![
            condition.as_slice(),
            true_input.as_slice(),
            false_input.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // Should output false_input
        assert_eq!(output[0], 10.0);
        assert_eq!(output[1], 20.0);
        assert_eq!(output[2], 30.0);
        assert_eq!(output[3], 40.0);
    }

    #[test]
    fn test_when_threshold_boundary() {
        // Test exact threshold boundary (0.5)
        let mut when = WhenNode::new(0, 1, 2);

        // Test values at and around threshold
        let condition = vec![0.49, 0.5, 0.50001, 0.51];
        let true_input = vec![100.0, 100.0, 100.0, 100.0];
        let false_input = vec![10.0, 10.0, 10.0, 10.0];
        let inputs = vec![
            condition.as_slice(),
            true_input.as_slice(),
            false_input.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // 0.49 < 0.5 → false_input
        assert_eq!(output[0], 10.0);
        // 0.5 == 0.5 (not >) → false_input
        assert_eq!(output[1], 10.0);
        // 0.50001 > 0.5 → true_input
        assert_eq!(output[2], 100.0);
        // 0.51 > 0.5 → true_input
        assert_eq!(output[3], 100.0);
    }

    #[test]
    fn test_when_alternating_condition() {
        // Test rapid switching between conditions
        let mut when = WhenNode::new(0, 1, 2);

        let condition = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let true_input = vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0];
        let false_input = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
        let inputs = vec![
            condition.as_slice(),
            true_input.as_slice(),
            false_input.as_slice(),
        ];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // Alternating pattern: true, false, true, false, true, false
        assert_eq!(output[0], 100.0);
        assert_eq!(output[1], 10.0);
        assert_eq!(output[2], 100.0);
        assert_eq!(output[3], 10.0);
        assert_eq!(output[4], 100.0);
        assert_eq!(output[5], 10.0);
    }

    #[test]
    fn test_when_with_negative_values() {
        // Test with negative signal values
        let mut when = WhenNode::new(0, 1, 2);

        let condition = vec![1.0, 0.0, 1.0, 0.0];
        let true_input = vec![-50.0, -40.0, -30.0, -20.0];
        let false_input = vec![50.0, 40.0, 30.0, 20.0];
        let inputs = vec![
            condition.as_slice(),
            true_input.as_slice(),
            false_input.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        assert_eq!(output[0], -50.0); // condition=1.0 → true_input
        assert_eq!(output[1], 40.0); // condition=0.0 → false_input
        assert_eq!(output[2], -30.0); // condition=1.0 → true_input
        assert_eq!(output[3], 20.0); // condition=0.0 → false_input
    }

    #[test]
    fn test_when_custom_threshold() {
        // Test with custom threshold
        let mut when = WhenNode::with_threshold(0, 1, 2, 0.8);

        let condition = vec![0.7, 0.8, 0.81, 1.0];
        let true_input = vec![100.0, 100.0, 100.0, 100.0];
        let false_input = vec![10.0, 10.0, 10.0, 10.0];
        let inputs = vec![
            condition.as_slice(),
            true_input.as_slice(),
            false_input.as_slice(),
        ];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // 0.7 <= 0.8 → false_input
        assert_eq!(output[0], 10.0);
        // 0.8 == 0.8 (not >) → false_input
        assert_eq!(output[1], 10.0);
        // 0.81 > 0.8 → true_input
        assert_eq!(output[2], 100.0);
        // 1.0 > 0.8 → true_input
        assert_eq!(output[3], 100.0);
    }

    #[test]
    fn test_when_with_constants() {
        // Integration test with ConstantNode
        let mut const_cond = ConstantNode::new(1.0);
        let mut const_true = ConstantNode::new(440.0);
        let mut const_false = ConstantNode::new(220.0);
        let mut when = WhenNode::new(0, 1, 2);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process constants first
        let mut buf_cond = vec![0.0; 512];
        let mut buf_true = vec![0.0; 512];
        let mut buf_false = vec![0.0; 512];

        const_cond.process_block(&[], &mut buf_cond, 44100.0, &context);
        const_true.process_block(&[], &mut buf_true, 44100.0, &context);
        const_false.process_block(&[], &mut buf_false, 44100.0, &context);

        // Now route through when node
        let inputs = vec![
            buf_cond.as_slice(),
            buf_true.as_slice(),
            buf_false.as_slice(),
        ];
        let mut output = vec![0.0; 512];

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // Condition = 1.0 (> 0.5), should output true_input (440.0)
        for sample in &output {
            assert_eq!(*sample, 440.0);
        }
    }

    #[test]
    fn test_when_mute_scenario() {
        // Realistic scenario: mute signal when condition is low
        let mut when = WhenNode::new(0, 1, 2);

        // Condition: gate pattern (1.0 for first half, 0.0 for second half)
        let condition = vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        // Signal: some audio
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        // Silence: all zeros
        let silence = vec![0.0; 8];

        let inputs = vec![condition.as_slice(), signal.as_slice(), silence.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // First half: signal passes through
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 4.0);

        // Second half: muted (silence)
        assert_eq!(output[4], 0.0);
        assert_eq!(output[5], 0.0);
        assert_eq!(output[6], 0.0);
        assert_eq!(output[7], 0.0);
    }

    #[test]
    fn test_when_pattern_routing_scenario() {
        // Realistic scenario: route between two different signals based on pattern
        let mut when = WhenNode::new(0, 1, 2);

        // Pattern: alternating 0/1 (simulates pattern control)
        let pattern = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        // Synth A: low frequencies
        let synth_a = vec![110.0, 110.0, 110.0, 110.0, 110.0, 110.0];
        // Synth B: high frequencies
        let synth_b = vec![440.0, 440.0, 440.0, 440.0, 440.0, 440.0];

        let inputs = vec![pattern.as_slice(), synth_b.as_slice(), synth_a.as_slice()];

        let mut output = vec![0.0; 6];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 6, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // Alternating between synth_a (pattern=0) and synth_b (pattern=1)
        assert_eq!(output[0], 110.0); // pattern=0 → synth_a
        assert_eq!(output[1], 440.0); // pattern=1 → synth_b
        assert_eq!(output[2], 110.0); // pattern=0 → synth_a
        assert_eq!(output[3], 440.0); // pattern=1 → synth_b
        assert_eq!(output[4], 110.0); // pattern=0 → synth_a
        assert_eq!(output[5], 440.0); // pattern=1 → synth_b
    }

    #[test]
    fn test_when_half_wave_rectifier_scenario() {
        // Audio-rate scenario: half-wave rectifier
        // Pass positive values, zero out negative values
        let mut when = WhenNode::new(0, 1, 2);

        // Input waveform: oscillating between -1 and 1
        let waveform = vec![-1.0, -0.5, 0.0, 0.5, 1.0, 0.5, 0.0, -0.5];
        // Condition: is waveform positive? (1.0 when true, 0.0 when false)
        let is_positive = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0];
        // Zero signal
        let zero = vec![0.0; 8];

        let inputs = vec![is_positive.as_slice(), waveform.as_slice(), zero.as_slice()];

        let mut output = vec![0.0; 8];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

        when.process_block(&inputs, &mut output, 44100.0, &context);

        // Negative values → zero
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.0);

        // Positive values → pass through
        assert_eq!(output[3], 0.5);
        assert_eq!(output[4], 1.0);
        assert_eq!(output[5], 0.5);

        // Zero and negative → zero
        assert_eq!(output[6], 0.0);
        assert_eq!(output[7], 0.0);
    }

    #[test]
    fn test_when_dependencies() {
        // Test that input_nodes returns all three dependencies in correct order
        let when = WhenNode::new(5, 10, 15);
        let deps = when.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5); // condition
        assert_eq!(deps[1], 10); // true_input
        assert_eq!(deps[2], 15); // false_input
    }

    #[test]
    fn test_when_getter_methods() {
        // Test getter methods
        let when = WhenNode::new(5, 10, 15);

        assert_eq!(when.condition(), 5);
        assert_eq!(when.true_input(), 10);
        assert_eq!(when.false_input(), 15);
        assert_eq!(when.threshold(), 0.5);

        // Test with custom threshold
        let when_custom = WhenNode::with_threshold(1, 2, 3, 0.75);
        assert_eq!(when_custom.threshold(), 0.75);
    }
}
