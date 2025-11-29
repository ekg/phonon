/// AR Envelope Generator - Attack, Release
///
/// Gate-triggered envelope with two phases:
/// - Attack: Linear ramp from 0.0 to 1.0 when gate goes high
/// - Release: Linear ramp from current value to 0.0 when gate goes low
///
/// Simpler than ADSR - no decay or sustain phases.
/// All time parameters are in seconds.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// AR envelope phase
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ARPhase {
    Idle,    // Gate is low, envelope is at 0.0
    Attack,  // Ramping from 0.0 to 1.0
    Release, // Ramping to 0.0
}

/// AR envelope state machine
#[derive(Debug, Clone)]
struct ARState {
    phase: ARPhase,      // Current envelope phase
    value: f32,          // Current envelope value (0.0 to 1.0)
    gate_was_high: bool, // Track gate transitions
}

impl Default for ARState {
    fn default() -> Self {
        Self {
            phase: ARPhase::Idle,
            value: 0.0,
            gate_was_high: false,
        }
    }
}

/// AR Envelope Generator Node
///
/// # Inputs
/// 1. Gate input (> 0.5 = high, <= 0.5 = low)
/// 2. Attack time in seconds
/// 3. Release time in seconds
///
/// # Example
/// ```ignore
/// // Create AR envelope
/// let gate = ConstantNode::new(1.0);           // NodeId 0 (gate on)
/// let attack = ConstantNode::new(0.01);        // NodeId 1 (10ms attack)
/// let release = ConstantNode::new(0.2);        // NodeId 2 (200ms release)
/// let ar = AREnvelopeNode::new(0, 1, 2);       // NodeId 3
/// ```
pub struct AREnvelopeNode {
    gate_input: NodeId,    // Trigger input (gate on/off)
    attack_input: NodeId,  // Attack time in seconds
    release_input: NodeId, // Release time in seconds
    state: ARState,        // Internal state machine
}

impl AREnvelopeNode {
    /// AREnvelope - Gate-triggered Attack-Release envelope
    ///
    /// Two-phase envelope: attack (0 to 1) then release (current to 0).
    /// Simpler than ADSR - no decay or sustain. Ideal for pad instruments and gates.
    ///
    /// # Parameters
    /// - `gate_input`: Gate signal (rising/falling edge controls envelope)
    /// - `attack_input`: Attack time in seconds
    /// - `release_input`: Release time in seconds
    ///
    /// # Example
    /// ```phonon
    /// ~gate: "x ~ x ~"
    /// ~envelope: ~gate # ar_envelope 0.01 0.2
    /// ~synth: sine 440 * ~envelope
    /// ```
    pub fn new(gate_input: NodeId, attack_input: NodeId, release_input: NodeId) -> Self {
        Self {
            gate_input,
            attack_input,
            release_input,
            state: ARState::default(),
        }
    }

    /// Get current envelope value
    pub fn value(&self) -> f32 {
        self.state.value
    }

    /// Get current phase
    pub fn phase(&self) -> ARPhase {
        self.state.phase
    }

    /// Reset envelope to idle state
    pub fn reset(&mut self) {
        self.state = ARState::default();
    }
}

impl AudioNode for AREnvelopeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "AREnvelopeNode requires 3 inputs: gate, attack, release"
        );

        let gate_buffer = inputs[0];
        let attack_buffer = inputs[1];
        let release_buffer = inputs[2];

        debug_assert_eq!(
            gate_buffer.len(),
            output.len(),
            "Gate buffer length mismatch"
        );
        debug_assert_eq!(
            attack_buffer.len(),
            output.len(),
            "Attack buffer length mismatch"
        );
        debug_assert_eq!(
            release_buffer.len(),
            output.len(),
            "Release buffer length mismatch"
        );

        for i in 0..output.len() {
            let gate = gate_buffer[i];
            let attack_time = attack_buffer[i].max(0.0001); // Minimum 0.1ms
            let release_time = release_buffer[i].max(0.0001);

            let gate_high = gate > 0.5;

            // Detect gate transitions
            let gate_rising = gate_high && !self.state.gate_was_high;
            let gate_falling = !gate_high && self.state.gate_was_high;

            // Update gate state
            self.state.gate_was_high = gate_high;

            // Gate rising edge: trigger attack phase
            if gate_rising {
                self.state.phase = ARPhase::Attack;
                // Keep current value for smooth retrigger
            }

            // Gate falling edge: trigger release phase
            if gate_falling {
                self.state.phase = ARPhase::Release;
                // Keep current value for smooth release
            }

            // Process envelope based on current phase
            match self.state.phase {
                ARPhase::Idle => {
                    // Envelope is off, output 0.0
                    self.state.value = 0.0;
                }

                ARPhase::Attack => {
                    // Ramp from current value to 1.0 over attack_time
                    let increment = 1.0 / (attack_time * sample_rate);
                    self.state.value += increment;

                    if self.state.value >= 1.0 {
                        self.state.value = 1.0;
                        // AR envelope stays at 1.0 while gate is high
                        // (no automatic decay to sustain like ADSR)
                    }
                }

                ARPhase::Release => {
                    // Ramp from current value to 0.0 over release_time
                    let decrement = 1.0 / (release_time * sample_rate);
                    self.state.value -= decrement;

                    if self.state.value <= 0.0 {
                        self.state.value = 0.0;
                        self.state.phase = ARPhase::Idle;
                    }
                }
            }

            // Output current envelope value
            output[i] = self.state.value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.gate_input, self.attack_input, self.release_input]
    }

    fn name(&self) -> &str {
        "AREnvelopeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_ar_idle_when_gate_low() {
        // Test 1: When gate is low, envelope should be idle at 0.0
        let mut gate = ConstantNode::new(0.0); // Gate off
        let mut attack = ConstantNode::new(0.01);
        let mut release = ConstantNode::new(0.2);

        let mut ar = AREnvelopeNode::new(0, 1, 2);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Generate input buffers
        let mut gate_buf = vec![0.0; 512];
        let mut attack_buf = vec![0.0; 512];
        let mut release_buf = vec![0.0; 512];

        gate.process_block(&[], &mut gate_buf, 44100.0, &context);
        attack.process_block(&[], &mut attack_buf, 44100.0, &context);
        release.process_block(&[], &mut release_buf, 44100.0, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        ar.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be 0.0 (idle)
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(sample, 0.0, "Sample {} should be 0.0 when gate is low", i);
        }

        assert_eq!(ar.phase(), ARPhase::Idle);
    }

    #[test]
    fn test_ar_attack_phase() {
        // Test 2: Attack phase should ramp from 0.0 to 1.0
        let sample_rate = 44100.0;
        let attack_time = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let mut gate = ConstantNode::new(1.0); // Gate on
        let mut attack = ConstantNode::new(attack_time);
        let mut release = ConstantNode::new(0.2);

        let mut ar = AREnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut gate_buf = vec![1.0; block_size];
        let mut attack_buf = vec![attack_time; block_size];
        let mut release_buf = vec![0.2; block_size];

        gate.process_block(&[], &mut gate_buf, sample_rate, &context);
        attack.process_block(&[], &mut attack_buf, sample_rate, &context);
        release.process_block(&[], &mut release_buf, sample_rate, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Envelope should be rising
        assert!(output[0] > 0.0, "Attack should start rising");
        assert!(output[100] > output[50], "Attack should be rising");

        // Should reach ~1.0 around sample 441
        let expected_samples = (attack_time * sample_rate) as usize;
        if expected_samples < block_size {
            assert!(
                output[expected_samples] >= 0.95,
                "Attack should reach ~1.0 at expected time, got {}",
                output[expected_samples]
            );
        }
    }

    #[test]
    fn test_ar_release_phase() {
        // Test 3: When gate goes low, should release to 0.0
        let sample_rate = 44100.0;
        let release_time = 0.05; // 50ms (longer to observe release in first block)
        let block_size = 512;

        let mut ar = AREnvelopeNode::new(0, 1, 2);

        // Start at peak (simulating post-attack)
        ar.state.phase = ARPhase::Attack;
        ar.state.value = 1.0;
        ar.state.gate_was_high = true;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Gate goes low
        let gate_buf = vec![0.0; block_size];
        let attack_buf = vec![0.01; block_size];
        let release_buf = vec![release_time; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release phase (or already be idle if release was fast)
        assert!(
            ar.phase() == ARPhase::Release || ar.phase() == ARPhase::Idle,
            "Should be in Release or Idle phase, got {:?}",
            ar.phase()
        );

        // Envelope should be falling
        assert!(
            output[0] >= output[100],
            "Release should be falling or at zero"
        );
        assert!(
            output[100] >= output[200],
            "Release should continue falling or at zero"
        );

        // Process more blocks to reach idle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            ar.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle at 0.0
        assert_eq!(ar.phase(), ARPhase::Idle);
        assert!(ar.value() < 0.01, "Should be at 0.0, got {}", ar.value());
    }

    #[test]
    fn test_ar_retrigger() {
        // Test 4: Retriggering (gate high again) should restart attack
        let sample_rate = 44100.0;
        let block_size = 64; // Smaller block to prevent phase completion

        let mut ar = AREnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let attack_buf = vec![0.05; block_size]; // Longer attack
        let release_buf = vec![0.2; block_size];

        // First trigger: gate on
        let gate_buf = vec![1.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack phase
        assert_eq!(ar.phase(), ARPhase::Attack, "Should be in Attack");

        // Gate goes low
        let gate_buf = vec![0.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in release or idle
        assert!(
            ar.phase() == ARPhase::Release || ar.phase() == ARPhase::Idle,
            "Should be in Release or Idle, got {:?}",
            ar.phase()
        );

        // Retrigger: gate high again
        let gate_buf = vec![1.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Should re-enter attack phase (first sample triggers on rising edge)
        assert_eq!(
            ar.phase(),
            ARPhase::Attack,
            "Should restart in Attack phase on retrigger"
        );
    }

    #[test]
    fn test_ar_fast_attack_slow_release() {
        // Test 5: Fast attack, slow release characteristics
        let sample_rate = 44100.0;
        let attack_time = 0.001; // 1ms = 44 samples
        let release_time = 0.1; // 100ms = 4410 samples
        let block_size = 512;

        let mut ar = AREnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Gate on
        let gate_buf = vec![1.0; block_size];
        let attack_buf = vec![attack_time; block_size];
        let release_buf = vec![release_time; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Attack should complete quickly
        let attack_samples = (attack_time * sample_rate) as usize;
        assert!(attack_samples < 100, "Attack should be very fast");
        assert!(
            output[attack_samples + 10] >= 0.95,
            "Should reach peak quickly"
        );

        // Now trigger release
        ar.state.value = 1.0; // Ensure at peak
        ar.state.gate_was_high = true;

        let gate_buf = vec![0.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Release should be slow (not complete in one block)
        assert_eq!(ar.phase(), ARPhase::Release);
        assert!(
            output[block_size - 1] > 0.5,
            "Release should be slow, still above 0.5"
        );
    }

    #[test]
    fn test_ar_dependencies() {
        // Test 6: Verify input_nodes returns correct dependencies
        let ar = AREnvelopeNode::new(10, 20, 30);
        let deps = ar.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // gate_input
        assert_eq!(deps[1], 20); // attack_input
        assert_eq!(deps[2], 30); // release_input
    }

    #[test]
    fn test_ar_with_constants() {
        // Test 7: Full envelope cycle with constant parameters
        let sample_rate = 44100.0;
        let attack_time = 0.002; // 2ms = 88 samples
        let release_time = 0.002; // 2ms = 88 samples
        let block_size = 128;

        let mut gate_node = ConstantNode::new(1.0);
        let mut attack_node = ConstantNode::new(attack_time);
        let mut release_node = ConstantNode::new(release_time);

        let mut ar = AREnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Generate constant buffers
        let mut gate_buf = vec![0.0; block_size];
        let mut attack_buf = vec![0.0; block_size];
        let mut release_buf = vec![0.0; block_size];

        gate_node.process_block(&[], &mut gate_buf, sample_rate, &context);
        attack_node.process_block(&[], &mut attack_buf, sample_rate, &context);
        release_node.process_block(&[], &mut release_buf, sample_rate, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        // Process attack
        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack phase
        assert_eq!(ar.phase(), ARPhase::Attack);
        assert!(output[block_size - 1] > 0.0, "Envelope should be active");

        // Process more to complete attack
        for _ in 0..2 {
            let mut output = vec![0.0; block_size];
            ar.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach peak
        assert!(ar.value() >= 0.95, "Should reach peak");

        // Release: gate off
        gate_buf.fill(0.0);
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ar.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release
        assert!(
            ar.phase() == ARPhase::Release || ar.phase() == ARPhase::Idle,
            "Should be in Release or Idle, got {:?}",
            ar.phase()
        );

        // Process to idle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            ar.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle
        assert_eq!(ar.phase(), ARPhase::Idle);
        assert!(ar.value() < 0.01, "Should reach 0.0");
    }
}
