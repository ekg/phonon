/// Noise Gate node - smooth dynamics gate with attack/release
///
/// This node provides smooth noise gating. It silences signals below the threshold
/// using an envelope follower with attack and release times. Unlike the hard GateNode,
/// this provides smooth transitions to prevent clicks and artifacts.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Noise gate state for envelope follower
#[derive(Debug, Clone)]
struct NoiseGateState {
    envelope: f32, // Current gate envelope (0.0 = closed, 1.0 = open)
}

impl Default for NoiseGateState {
    fn default() -> Self {
        Self { envelope: 0.0 }
    }
}

/// Noise Gate node: smooth gate with attack/release
///
/// The noise gate algorithm:
/// ```text
/// 1. Convert input to dB: input_db = 20 * log10(abs(sample))
/// 2. Determine target envelope:
///    if input_db > threshold_db:
///        target = 1.0  // Gate open
///    else:
///        target = 0.0  // Gate closed
/// 3. Smooth with attack/release envelope follower:
///    attack_coeff = exp(-1 / (attack_time * sample_rate))
///    release_coeff = exp(-1 / (release_time * sample_rate))
///    if target > envelope:
///        envelope = attack_coeff * envelope + (1 - attack_coeff) * target
///    else:
///        envelope = release_coeff * envelope + (1 - release_coeff) * target
/// 4. Apply envelope: output = input * envelope
/// ```
///
/// This provides:
/// - Smooth gate opening/closing with no clicks
/// - Independent attack and release times
/// - Noise floor suppression for recording/production
///
/// # Example
/// ```ignore
/// // Gate signal with -30 dB threshold, fast attack, slow release
/// let input = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// let threshold = ConstantNode::new(-30.0);            // NodeId 2 (-30 dB)
/// let attack = ConstantNode::new(0.001);               // NodeId 3 (1ms)
/// let release = ConstantNode::new(0.1);                // NodeId 4 (100ms)
/// let gate = NoiseGateNode::new(1, 2, 3, 4);           // NodeId 5
/// ```
pub struct NoiseGateNode {
    input: NodeId,
    threshold_input: NodeId, // Threshold in dB (e.g., -30.0)
    attack_input: NodeId,    // Attack time in seconds (0.001 to 0.1)
    release_input: NodeId,   // Release time in seconds (0.01 to 1.0)
    state: NoiseGateState,   // Envelope follower state
}

impl NoiseGateNode {
    /// NoiseGateNode - Smooth noise suppression gate with envelope shaping
    ///
    /// Silences signals below a threshold using an envelope follower with independent
    /// attack and release times. Provides click-free gating for noise suppression and
    /// dynamic control without artifacts.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to gate
    /// - `threshold_input`: NodeId of threshold in dB (e.g., -30.0)
    /// - `attack_input`: NodeId of attack time in seconds (0.001-0.1 typical)
    /// - `release_input`: NodeId of release time in seconds (0.01-1.0 typical)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~gated: ~signal # noise_gate -30 0.001 0.1
    /// ```
    pub fn new(
        input: NodeId,
        threshold_input: NodeId,
        attack_input: NodeId,
        release_input: NodeId,
    ) -> Self {
        Self {
            input,
            threshold_input,
            attack_input,
            release_input,
            state: NoiseGateState::default(),
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

    /// Get the attack input node ID
    pub fn attack_input(&self) -> NodeId {
        self.attack_input
    }

    /// Get the release input node ID
    pub fn release_input(&self) -> NodeId {
        self.release_input
    }

    /// Reset noise gate state
    pub fn reset(&mut self) {
        self.state = NoiseGateState::default();
    }
}

impl AudioNode for NoiseGateNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "NoiseGateNode requires 4 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let threshold_buf = inputs[1];
        let attack_buf = inputs[2];
        let release_buf = inputs[3];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            threshold_buf.len(),
            output.len(),
            "Threshold buffer length mismatch"
        );
        debug_assert_eq!(
            attack_buf.len(),
            output.len(),
            "Attack buffer length mismatch"
        );
        debug_assert_eq!(
            release_buf.len(),
            output.len(),
            "Release buffer length mismatch"
        );

        // Apply noise gate with envelope follower
        for i in 0..output.len() {
            let sample = input_buf[i];
            let threshold_db = threshold_buf[i];
            let attack_time = attack_buf[i].max(0.0001); // Min 0.1ms
            let release_time = release_buf[i].max(0.001); // Min 1ms

            // Calculate attack/release coefficients (exponential smoothing)
            let attack_coeff = (-1.0 / (attack_time * sample_rate)).exp();
            let release_coeff = (-1.0 / (release_time * sample_rate)).exp();

            // Convert input to dB (with safety for zero/negative)
            let abs_sample = sample.abs().max(1e-10); // Prevent log(0)
            let input_db = 20.0 * abs_sample.log10();

            // Determine target envelope (gate open or closed)
            let target = if input_db > threshold_db {
                1.0 // Gate open
            } else {
                0.0 // Gate closed
            };

            // Smooth envelope with attack/release
            if target > self.state.envelope {
                // Attack: gate opening (signal above threshold)
                self.state.envelope =
                    attack_coeff * self.state.envelope + (1.0 - attack_coeff) * target;
            } else {
                // Release: gate closing (signal below threshold)
                self.state.envelope =
                    release_coeff * self.state.envelope + (1.0 - release_coeff) * target;
            }

            // Apply envelope to gate the signal
            output[i] = sample * self.state.envelope;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.threshold_input,
            self.attack_input,
            self.release_input,
        ]
    }

    fn name(&self) -> &str {
        "NoiseGateNode"
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
    fn test_noise_gate_above_threshold_passes() {
        // Signal above threshold should pass through
        let mut gate = NoiseGateNode::new(0, 1, 2, 3);

        // Input: 0.5 (-6 dB), Threshold: -20 dB
        let input = vec![0.5; 512];
        let threshold = vec![-20.0; 512];
        let attack = vec![0.001; 512]; // Fast attack
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
        assert!(
            avg_output > 0.4,
            "Average output was {}, expected > 0.4",
            avg_output
        );
    }

    #[test]
    fn test_noise_gate_below_threshold_silenced() {
        // Signal below threshold should be gated/silenced
        let mut gate = NoiseGateNode::new(0, 1, 2, 3);

        // Input: 0.01 (-40 dB), Threshold: -20 dB
        let input = vec![0.01; 512];
        let threshold = vec![-20.0; 512];
        let attack = vec![0.01; 512];
        let release = vec![0.001; 512]; // Fast release
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
        assert!(
            avg_output < 0.005,
            "Average output was {}, expected < 0.005",
            avg_output
        );
    }

    #[test]
    fn test_noise_gate_dependencies() {
        let gate = NoiseGateNode::new(5, 10, 15, 20);
        let deps = gate.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 5); // input
        assert_eq!(deps[1], 10); // threshold
        assert_eq!(deps[2], 15); // attack
        assert_eq!(deps[3], 20); // release
    }
}
