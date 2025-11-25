/// ADSR Envelope Generator - Attack, Decay, Sustain, Release
///
/// Gate-triggered envelope with four phases:
/// - Attack: Linear ramp from 0.0 to 1.0
/// - Decay: Linear ramp from 1.0 to sustain level
/// - Sustain: Hold at sustain level while gate is high
/// - Release: Linear ramp to 0.0 when gate goes low
///
/// All time parameters are in seconds, sustain is level (0.0 to 1.0).

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// ADSR envelope phase
#[derive(Debug, Clone, Copy, PartialEq)]
enum ADSRPhase {
    Idle,       // Gate is low, envelope is at 0.0
    Attack,     // Ramping from 0.0 to 1.0
    Decay,      // Ramping from 1.0 to sustain_level
    Sustain,    // Holding at sustain_level
    Release,    // Ramping to 0.0
}

/// ADSR envelope state machine
#[derive(Debug, Clone)]
struct ADSRState {
    phase: ADSRPhase,       // Current envelope phase
    value: f32,             // Current envelope value (0.0 to 1.0)
    gate_was_high: bool,    // Track gate transitions
    release_start_value: f32, // Value when release phase started
}

impl Default for ADSRState {
    fn default() -> Self {
        Self {
            phase: ADSRPhase::Idle,
            value: 0.0,
            gate_was_high: false,
            release_start_value: 0.0,
        }
    }
}

/// ADSR Envelope Generator Node
///
/// # Inputs
/// 1. Gate input (> 0.5 = high, <= 0.5 = low)
/// 2. Attack time in seconds
/// 3. Decay time in seconds
/// 4. Sustain level (0.0 to 1.0)
/// 5. Release time in seconds
///
/// # Example
/// ```ignore
/// // Create ADSR envelope
/// let gate = ConstantNode::new(1.0);           // NodeId 0 (gate on)
/// let attack = ConstantNode::new(0.01);        // NodeId 1 (10ms attack)
/// let decay = ConstantNode::new(0.1);          // NodeId 2 (100ms decay)
/// let sustain = ConstantNode::new(0.7);        // NodeId 3 (70% sustain)
/// let release = ConstantNode::new(0.2);        // NodeId 4 (200ms release)
/// let adsr = ADSRNode::new(0, 1, 2, 3, 4);     // NodeId 5
/// ```
pub struct ADSRNode {
    gate_input: NodeId,      // Trigger input (gate on/off)
    attack_input: NodeId,    // Attack time in seconds
    decay_input: NodeId,     // Decay time in seconds
    sustain_input: NodeId,   // Sustain level (0.0 to 1.0)
    release_input: NodeId,   // Release time in seconds
    state: ADSRState,        // Internal state machine
}

impl ADSRNode {
    /// ADSR - Gate-triggered Attack-Decay-Sustain-Release envelope
    ///
    /// Four-phase envelope controlled by gate signal. Ideal for sustained instruments,
    /// pads, and any sound that needs to respond to note on/off events.
    ///
    /// # Parameters
    /// - `gate_input`: Gate signal (rising edge = note on, falling edge = note off)
    /// - `attack_input`: Attack time in seconds
    /// - `decay_input`: Decay time in seconds
    /// - `sustain_input`: Sustain level (0.0 to 1.0)
    /// - `release_input`: Release time in seconds
    ///
    /// # Example
    /// ```phonon
    /// ~gate: "x ~ x ~"
    /// ~env: ~gate # adsr 0.01 0.1 0.7 0.2
    /// ~synth: sine 440 * ~env
    /// ```
    pub fn new(
        gate_input: NodeId,
        attack_input: NodeId,
        decay_input: NodeId,
        sustain_input: NodeId,
        release_input: NodeId,
    ) -> Self {
        Self {
            gate_input,
            attack_input,
            decay_input,
            sustain_input,
            release_input,
            state: ADSRState::default(),
        }
    }

    /// Get current envelope value
    pub fn value(&self) -> f32 {
        self.state.value
    }

    /// Get current phase (for debugging)
    #[allow(dead_code)]
    fn phase(&self) -> ADSRPhase {
        self.state.phase
    }

    /// Reset envelope to idle state
    pub fn reset(&mut self) {
        self.state = ADSRState::default();
    }
}

impl AudioNode for ADSRNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "ADSRNode requires 5 inputs: gate, attack, decay, sustain, release"
        );

        let gate_buffer = inputs[0];
        let attack_buffer = inputs[1];
        let decay_buffer = inputs[2];
        let sustain_buffer = inputs[3];
        let release_buffer = inputs[4];

        debug_assert_eq!(gate_buffer.len(), output.len(), "Gate buffer length mismatch");
        debug_assert_eq!(attack_buffer.len(), output.len(), "Attack buffer length mismatch");
        debug_assert_eq!(decay_buffer.len(), output.len(), "Decay buffer length mismatch");
        debug_assert_eq!(sustain_buffer.len(), output.len(), "Sustain buffer length mismatch");
        debug_assert_eq!(release_buffer.len(), output.len(), "Release buffer length mismatch");

        for i in 0..output.len() {
            let gate = gate_buffer[i];
            let attack_time = attack_buffer[i].max(0.0001); // Minimum 0.1ms
            let decay_time = decay_buffer[i].max(0.0001);
            let sustain_level = sustain_buffer[i].clamp(0.0, 1.0);
            let release_time = release_buffer[i].max(0.0001);

            let gate_high = gate > 0.5;

            // Detect gate transitions
            let gate_rising = gate_high && !self.state.gate_was_high;
            let gate_falling = !gate_high && self.state.gate_was_high;

            // Update gate state
            self.state.gate_was_high = gate_high;

            // Gate rising edge: trigger attack phase
            if gate_rising {
                self.state.phase = ADSRPhase::Attack;
                // Keep current value for smooth retrigger
            }

            // Gate falling edge: trigger release phase
            if gate_falling {
                self.state.phase = ADSRPhase::Release;
                self.state.release_start_value = self.state.value;
                // Keep current value for smooth release
            }

            // Process envelope based on current phase
            match self.state.phase {
                ADSRPhase::Idle => {
                    // Envelope is off, output 0.0
                    self.state.value = 0.0;
                }

                ADSRPhase::Attack => {
                    // Ramp from current value to 1.0 over attack_time
                    let increment = 1.0 / (attack_time * sample_rate);
                    self.state.value += increment;

                    if self.state.value >= 1.0 {
                        self.state.value = 1.0;
                        self.state.phase = ADSRPhase::Decay;
                    }
                }

                ADSRPhase::Decay => {
                    // Ramp from 1.0 to sustain_level over decay_time
                    let target = sustain_level;
                    let decrement = (1.0 - target) / (decay_time * sample_rate);
                    self.state.value -= decrement;

                    if self.state.value <= target {
                        self.state.value = target;
                        self.state.phase = ADSRPhase::Sustain;
                    }
                }

                ADSRPhase::Sustain => {
                    // Hold at sustain_level
                    self.state.value = sustain_level;

                    // Stay in sustain while gate is high
                    if !gate_high {
                        self.state.phase = ADSRPhase::Release;
                    }
                }

                ADSRPhase::Release => {
                    // Ramp from release_start_value to 0.0 over release_time
                    let total_samples = (release_time * sample_rate).max(1.0);
                    let decrement = self.state.release_start_value / total_samples;
                    self.state.value -= decrement;

                    if self.state.value <= 0.0 {
                        self.state.value = 0.0;
                        self.state.phase = ADSRPhase::Idle;
                    }
                }
            }

            // Output current envelope value
            output[i] = self.state.value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.gate_input,
            self.attack_input,
            self.decay_input,
            self.sustain_input,
            self.release_input,
        ]
    }

    fn name(&self) -> &str {
        "ADSRNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_adsr_idle_when_gate_low() {
        // Test 1: When gate is low, envelope should be idle at 0.0
        let mut gate = ConstantNode::new(0.0);  // Gate off
        let mut attack = ConstantNode::new(0.01);
        let mut decay = ConstantNode::new(0.1);
        let mut sustain = ConstantNode::new(0.7);
        let mut release = ConstantNode::new(0.2);

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate input buffers
        let mut gate_buf = vec![0.0; 512];
        let mut attack_buf = vec![0.0; 512];
        let mut decay_buf = vec![0.0; 512];
        let mut sustain_buf = vec![0.0; 512];
        let mut release_buf = vec![0.0; 512];

        gate.process_block(&[], &mut gate_buf, 44100.0, &context);
        attack.process_block(&[], &mut attack_buf, 44100.0, &context);
        decay.process_block(&[], &mut decay_buf, 44100.0, &context);
        sustain.process_block(&[], &mut sustain_buf, 44100.0, &context);
        release.process_block(&[], &mut release_buf, 44100.0, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        adsr.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be 0.0 (idle)
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(
                sample, 0.0,
                "Sample {} should be 0.0 when gate is low",
                i
            );
        }

        assert_eq!(adsr.phase(), ADSRPhase::Idle);
    }

    #[test]
    fn test_adsr_attack_phase() {
        // Test 2: Attack phase should ramp from 0.0 to 1.0
        let sample_rate = 44100.0;
        let attack_time = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let mut gate = ConstantNode::new(1.0);  // Gate on
        let mut attack = ConstantNode::new(attack_time);
        let mut decay = ConstantNode::new(0.1);
        let mut sustain = ConstantNode::new(0.7);
        let mut release = ConstantNode::new(0.2);

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut gate_buf = vec![1.0; block_size];
        let mut attack_buf = vec![attack_time; block_size];
        let mut decay_buf = vec![0.1; block_size];
        let mut sustain_buf = vec![0.7; block_size];
        let mut release_buf = vec![0.2; block_size];

        gate.process_block(&[], &mut gate_buf, sample_rate, &context);
        attack.process_block(&[], &mut attack_buf, sample_rate, &context);
        decay.process_block(&[], &mut decay_buf, sample_rate, &context);
        sustain.process_block(&[], &mut sustain_buf, sample_rate, &context);
        release.process_block(&[], &mut release_buf, sample_rate, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

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
    fn test_adsr_decay_to_sustain() {
        // Test 3: After attack, should decay to sustain level
        let sample_rate = 44100.0;
        let attack_time = 0.001; // 1ms
        let decay_time = 0.01;   // 10ms
        let sustain_level = 0.5;
        let block_size = 512;

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let gate_buf = vec![1.0; block_size];
        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![decay_time; block_size];
        let sustain_buf = vec![sustain_level; block_size];
        let release_buf = vec![0.2; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        // Process multiple blocks to go through attack and decay
        for _ in 0..5 {
            let mut output = vec![0.0; block_size];
            adsr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should now be in sustain phase at 0.5
        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        assert_eq!(adsr.phase(), ADSRPhase::Sustain);
        assert!(
            (output[block_size - 1] - sustain_level).abs() < 0.01,
            "Should be at sustain level, got {}",
            output[block_size - 1]
        );
    }

    #[test]
    fn test_adsr_sustain_holds() {
        // Test 4: Sustain phase should hold constant while gate is high
        let sample_rate = 44100.0;
        let sustain_level = 0.6;
        let block_size = 512;

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        // Manually set to sustain phase
        adsr.state.phase = ADSRPhase::Sustain;
        adsr.state.value = sustain_level;
        adsr.state.gate_was_high = true;

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let gate_buf = vec![1.0; block_size];
        let attack_buf = vec![0.01; block_size];
        let decay_buf = vec![0.1; block_size];
        let sustain_buf = vec![sustain_level; block_size];
        let release_buf = vec![0.2; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // All samples should be at sustain level
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                (sample - sustain_level).abs() < 0.01,
                "Sample {} should be at sustain level {}, got {}",
                i,
                sustain_level,
                sample
            );
        }

        assert_eq!(adsr.phase(), ADSRPhase::Sustain);
    }

    #[test]
    fn test_adsr_release_phase() {
        // Test 5: When gate goes low, should release to 0.0
        let sample_rate = 44100.0;
        let release_time = 0.05; // 50ms (longer to observe release in first block)
        let block_size = 512;

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        // Start in sustain phase
        adsr.state.phase = ADSRPhase::Sustain;
        adsr.state.value = 0.7;
        adsr.state.gate_was_high = true;

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Gate goes low
        let gate_buf = vec![0.0; block_size];
        let attack_buf = vec![0.01; block_size];
        let decay_buf = vec![0.1; block_size];
        let sustain_buf = vec![0.7; block_size];
        let release_buf = vec![release_time; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release phase (or already be idle if release was fast)
        assert!(
            adsr.phase() == ADSRPhase::Release || adsr.phase() == ADSRPhase::Idle,
            "Should be in Release or Idle phase, got {:?}",
            adsr.phase()
        );

        // Envelope should be falling
        assert!(output[0] >= output[100], "Release should be falling or at zero");
        assert!(output[100] >= output[200], "Release should continue falling or at zero");

        // Process more blocks to reach idle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            adsr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle at 0.0
        assert_eq!(adsr.phase(), ADSRPhase::Idle);
        assert!(adsr.value() < 0.01, "Should be at 0.0, got {}", adsr.value());
    }

    #[test]
    fn test_adsr_retrigger() {
        // Test 6: Retriggering (gate high again) should restart attack
        let sample_rate = 44100.0;
        let block_size = 64; // Smaller block to prevent phase completion

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let attack_buf = vec![0.05; block_size]; // Longer attack
        let decay_buf = vec![0.1; block_size];
        let sustain_buf = vec![0.7; block_size];
        let release_buf = vec![0.2; block_size];

        // First trigger: gate on
        let gate_buf = vec![1.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack or decay phase (attack may complete in first block)
        assert!(
            adsr.phase() == ADSRPhase::Attack || adsr.phase() == ADSRPhase::Decay,
            "Should be in Attack or Decay, got {:?}",
            adsr.phase()
        );

        // Gate goes low
        let gate_buf = vec![0.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in release or idle
        assert!(
            adsr.phase() == ADSRPhase::Release || adsr.phase() == ADSRPhase::Idle,
            "Should be in Release or Idle, got {:?}",
            adsr.phase()
        );

        // Retrigger: gate high again
        let gate_buf = vec![1.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should re-enter attack phase (first sample triggers on rising edge)
        assert_eq!(adsr.phase(), ADSRPhase::Attack, "Should restart in Attack phase on retrigger");
    }

    #[test]
    fn test_adsr_dependencies() {
        // Test 7: Verify input_nodes returns correct dependencies
        let adsr = ADSRNode::new(10, 20, 30, 40, 50);
        let deps = adsr.input_nodes();

        assert_eq!(deps.len(), 5);
        assert_eq!(deps[0], 10); // gate_input
        assert_eq!(deps[1], 20); // attack_input
        assert_eq!(deps[2], 30); // decay_input
        assert_eq!(deps[3], 40); // sustain_input
        assert_eq!(deps[4], 50); // release_input
    }

    #[test]
    fn test_adsr_with_constants() {
        // Test 8: Full envelope cycle with constant parameters
        let sample_rate = 44100.0;
        let attack_time = 0.001;  // 1ms = 44 samples
        let decay_time = 0.001;   // 1ms = 44 samples
        let sustain_level = 0.5;
        let release_time = 0.001; // 1ms = 44 samples
        let block_size = 64;

        let mut gate_node = ConstantNode::new(1.0);
        let mut attack_node = ConstantNode::new(attack_time);
        let mut decay_node = ConstantNode::new(decay_time);
        let mut sustain_node = ConstantNode::new(sustain_level);
        let mut release_node = ConstantNode::new(release_time);

        let mut adsr = ADSRNode::new(0, 1, 2, 3, 4);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Generate constant buffers
        let mut gate_buf = vec![0.0; block_size];
        let mut attack_buf = vec![0.0; block_size];
        let mut decay_buf = vec![0.0; block_size];
        let mut sustain_buf = vec![0.0; block_size];
        let mut release_buf = vec![0.0; block_size];

        gate_node.process_block(&[], &mut gate_buf, sample_rate, &context);
        attack_node.process_block(&[], &mut attack_buf, sample_rate, &context);
        decay_node.process_block(&[], &mut decay_buf, sample_rate, &context);
        sustain_node.process_block(&[], &mut sustain_buf, sample_rate, &context);
        release_node.process_block(&[], &mut release_buf, sample_rate, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        // Process attack
        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack or decay phase
        assert!(output[block_size - 1] > 0.0, "Envelope should be active");

        // Process more to reach sustain
        for _ in 0..3 {
            let mut output = vec![0.0; block_size];
            adsr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should be at sustain
        assert_eq!(adsr.phase(), ADSRPhase::Sustain);

        // Release: gate off
        gate_buf.fill(0.0);
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        adsr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release (or already be idle with fast release)
        assert!(
            adsr.phase() == ADSRPhase::Release || adsr.phase() == ADSRPhase::Idle,
            "Should be in Release or Idle, got {:?}",
            adsr.phase()
        );

        // Process to idle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            adsr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle
        assert_eq!(adsr.phase(), ADSRPhase::Idle);
        assert!(adsr.value() < 0.01);
    }
}
