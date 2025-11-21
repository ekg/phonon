/// ASR Envelope Generator - Attack, Sustain, Release
///
/// Gate-triggered envelope with three phases:
/// - Attack: Linear ramp from 0.0 to 1.0 when gate goes high
/// - Sustain: Hold at sustain level while gate is high
/// - Release: Linear ramp from sustain level to 0.0 when gate goes low
///
/// Unlike ADSR, ASR sustains at a configurable level (not at peak).
/// This is ideal for organ-style envelopes where sustain is below peak.
/// All time parameters are in seconds, sustain is level (0.0 to 1.0).

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// ASR envelope phase
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ASRPhase {
    Idle,       // Gate is low, envelope is at 0.0
    Attack,     // Ramping from 0.0 to 1.0
    Sustain,    // Holding at sustain_level
    Release,    // Ramping to 0.0
}

/// ASR envelope state machine
#[derive(Debug, Clone)]
struct ASRState {
    phase: ASRPhase,        // Current envelope phase
    value: f32,             // Current envelope value (0.0 to 1.0)
    gate_was_high: bool,    // Track gate transitions
}

impl Default for ASRState {
    fn default() -> Self {
        Self {
            phase: ASRPhase::Idle,
            value: 0.0,
            gate_was_high: false,
        }
    }
}

/// ASR Envelope Generator Node
///
/// # Inputs
/// 1. Gate input (> 0.5 = high, <= 0.5 = low)
/// 2. Attack time in seconds
/// 3. Sustain level (0.0 to 1.0)
/// 4. Release time in seconds
///
/// # Example
/// ```ignore
/// // Create ASR envelope
/// let gate = ConstantNode::new(1.0);           // NodeId 0 (gate on)
/// let attack = ConstantNode::new(0.01);        // NodeId 1 (10ms attack)
/// let sustain = ConstantNode::new(0.6);        // NodeId 2 (60% sustain)
/// let release = ConstantNode::new(0.2);        // NodeId 3 (200ms release)
/// let asr = ASREnvelopeNode::new(0, 1, 2, 3);  // NodeId 4
/// ```
pub struct ASREnvelopeNode {
    gate_input: NodeId,      // Trigger input (gate on/off)
    attack_input: NodeId,    // Attack time in seconds
    sustain_input: NodeId,   // Sustain level (0.0 to 1.0)
    release_input: NodeId,   // Release time in seconds
    state: ASRState,         // Internal state machine
}

impl ASREnvelopeNode {
    /// ASREnvelope - Gate-triggered Attack-Sustain-Release envelope
    ///
    /// Three-phase envelope: attack (0 to 1), sustain (held at configurable level), release (to 0).
    /// Ideal for organ-style instruments where sustain is below peak. Simpler than ADSR.
    ///
    /// # Parameters
    /// - `gate_input`: Gate signal (rising/falling edge controls envelope)
    /// - `attack_input`: Attack time in seconds
    /// - `sustain_input`: Sustain level (0.0 to 1.0)
    /// - `release_input`: Release time in seconds
    ///
    /// # Example
    /// ```phonon
    /// ~gate: "x ~ x ~"
    /// ~envelope: ~gate # asr_envelope 0.01 0.6 0.2
    /// ~synth: sine 440 * ~envelope
    /// ```
    pub fn new(
        gate_input: NodeId,
        attack_input: NodeId,
        sustain_input: NodeId,
        release_input: NodeId,
    ) -> Self {
        Self {
            gate_input,
            attack_input,
            sustain_input,
            release_input,
            state: ASRState::default(),
        }
    }

    /// Get current envelope value
    pub fn value(&self) -> f32 {
        self.state.value
    }

    /// Get current phase
    pub fn phase(&self) -> ASRPhase {
        self.state.phase
    }

    /// Reset envelope to idle state
    pub fn reset(&mut self) {
        self.state = ASRState::default();
    }
}

impl AudioNode for ASREnvelopeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "ASREnvelopeNode requires 4 inputs: gate, attack, sustain, release"
        );

        let gate_buffer = inputs[0];
        let attack_buffer = inputs[1];
        let sustain_buffer = inputs[2];
        let release_buffer = inputs[3];

        debug_assert_eq!(gate_buffer.len(), output.len(), "Gate buffer length mismatch");
        debug_assert_eq!(attack_buffer.len(), output.len(), "Attack buffer length mismatch");
        debug_assert_eq!(sustain_buffer.len(), output.len(), "Sustain buffer length mismatch");
        debug_assert_eq!(release_buffer.len(), output.len(), "Release buffer length mismatch");

        for i in 0..output.len() {
            let gate = gate_buffer[i];
            let attack_time = attack_buffer[i].max(0.0001); // Minimum 0.1ms
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
                self.state.phase = ASRPhase::Attack;
                // Keep current value for smooth retrigger
            }

            // Gate falling edge: trigger release phase
            if gate_falling {
                self.state.phase = ASRPhase::Release;
                // Keep current value for smooth release
            }

            // Process envelope based on current phase
            match self.state.phase {
                ASRPhase::Idle => {
                    // Envelope is off, output 0.0
                    self.state.value = 0.0;
                }

                ASRPhase::Attack => {
                    // Ramp from current value to 1.0 over attack_time
                    let increment = 1.0 / (attack_time * sample_rate);
                    self.state.value += increment;

                    if self.state.value >= 1.0 {
                        self.state.value = 1.0;
                        // Transition to sustain phase
                        self.state.phase = ASRPhase::Sustain;
                    }
                }

                ASRPhase::Sustain => {
                    // Hold at sustain_level (NOT at peak like ADSR)
                    self.state.value = sustain_level;

                    // Stay in sustain while gate is high
                    if !gate_high {
                        self.state.phase = ASRPhase::Release;
                    }
                }

                ASRPhase::Release => {
                    // Exponential decay from current value to 0.0
                    // Use time constant such that value reaches ~1% in release_time
                    // Time constant tau = release_time / 4.6 (since e^-4.6 ≈ 0.01)
                    let tau = release_time / 4.6;
                    let decay_factor = (-1.0 / (tau * sample_rate)).exp();
                    self.state.value *= decay_factor;

                    if self.state.value <= 0.001 {
                        self.state.value = 0.0;
                        self.state.phase = ASRPhase::Idle;
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
            self.sustain_input,
            self.release_input,
        ]
    }

    fn name(&self) -> &str {
        "ASREnvelopeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_asr_attack_phase() {
        // Test 1: Attack phase should ramp from 0.0 to 1.0
        let sample_rate = 44100.0;
        let attack_time = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let mut gate = ConstantNode::new(1.0);  // Gate on
        let mut attack = ConstantNode::new(attack_time);
        let mut sustain = ConstantNode::new(0.6);
        let mut release = ConstantNode::new(0.2);

        let mut asr = ASREnvelopeNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut gate_buf = vec![1.0; block_size];
        let mut attack_buf = vec![attack_time; block_size];
        let mut sustain_buf = vec![0.6; block_size];
        let mut release_buf = vec![0.2; block_size];

        gate.process_block(&[], &mut gate_buf, sample_rate, &context);
        attack.process_block(&[], &mut attack_buf, sample_rate, &context);
        sustain.process_block(&[], &mut sustain_buf, sample_rate, &context);
        release.process_block(&[], &mut release_buf, sample_rate, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Envelope should be rising
        assert!(output[0] > 0.0, "Attack should start rising");
        assert!(output[100] > output[50], "Attack should be rising");

        // After attack completes, should transition to sustain at sustain_level
        let expected_samples = (attack_time * sample_rate) as usize;
        if expected_samples < block_size {
            // Check a sample during attack (before completion)
            let mid_attack = expected_samples / 2;
            assert!(
                output[mid_attack] > 0.1 && output[mid_attack] < 0.9,
                "Mid-attack should be between 0.1 and 0.9, got {}",
                output[mid_attack]
            );

            // After attack completes, should be at sustain level (0.6)
            assert!(
                (output[expected_samples + 10] - 0.6).abs() < 0.1,
                "Should reach sustain level after attack, got {}",
                output[expected_samples + 10]
            );
        }
    }

    #[test]
    fn test_asr_sustain_holds_at_level() {
        // Test 2: Sustain phase should hold at sustain_level (NOT at 1.0)
        let sample_rate = 44100.0;
        let sustain_level = 0.5;
        let block_size = 512;

        let mut asr = ASREnvelopeNode::new(0, 1, 2, 3);

        // Manually set to sustain phase at peak (will transition to sustain level)
        asr.state.phase = ASRPhase::Sustain;
        asr.state.value = 1.0;
        asr.state.gate_was_high = true;

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let gate_buf = vec![1.0; block_size];
        let attack_buf = vec![0.01; block_size];
        let sustain_buf = vec![sustain_level; block_size];
        let release_buf = vec![0.2; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // All samples should be at sustain level (0.5), NOT at 1.0
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                (sample - sustain_level).abs() < 0.01,
                "Sample {} should be at sustain level {}, got {}",
                i,
                sustain_level,
                sample
            );
        }

        assert_eq!(asr.phase(), ASRPhase::Sustain);
        assert!((asr.value() - sustain_level).abs() < 0.01,
            "Sustain should hold at level {}, got {}", sustain_level, asr.value());
    }

    #[test]
    fn test_asr_release_phase() {
        // Test 3: When gate goes low, should release to 0.0
        let sample_rate = 44100.0;
        let release_time = 0.05; // 50ms (longer to observe release in first block)
        let sustain_level = 0.6;
        let block_size = 512;

        let mut asr = ASREnvelopeNode::new(0, 1, 2, 3);

        // Start in sustain phase
        asr.state.phase = ASRPhase::Sustain;
        asr.state.value = sustain_level;
        asr.state.gate_was_high = true;

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
        let sustain_buf = vec![sustain_level; block_size];
        let release_buf = vec![release_time; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release phase (or already be idle if release was fast)
        assert!(
            asr.phase() == ASRPhase::Release || asr.phase() == ASRPhase::Idle,
            "Should be in Release or Idle phase, got {:?}",
            asr.phase()
        );

        // Envelope should be falling
        assert!(output[0] >= output[100], "Release should be falling or at zero");
        assert!(output[100] >= output[200], "Release should continue falling or at zero");

        // Process more blocks to reach idle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            asr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle at 0.0
        assert_eq!(asr.phase(), ASRPhase::Idle);
        assert!(asr.value() < 0.01, "Should be at 0.0, got {}", asr.value());
    }

    #[test]
    fn test_asr_retrigger() {
        // Test 4: Retriggering (gate high again) should restart attack
        let sample_rate = 44100.0;
        let block_size = 64; // Smaller block to prevent phase completion

        let mut asr = ASREnvelopeNode::new(0, 1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let attack_buf = vec![0.05; block_size]; // Longer attack
        let sustain_buf = vec![0.6; block_size];
        let release_buf = vec![0.2; block_size];

        // First trigger: gate on
        let gate_buf = vec![1.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack or sustain phase
        assert!(
            asr.phase() == ASRPhase::Attack || asr.phase() == ASRPhase::Sustain,
            "Should be in Attack or Sustain, got {:?}",
            asr.phase()
        );

        // Gate goes low
        let gate_buf = vec![0.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in release or idle
        assert!(
            asr.phase() == ASRPhase::Release || asr.phase() == ASRPhase::Idle,
            "Should be in Release or Idle, got {:?}",
            asr.phase()
        );

        // Retrigger: gate high again
        let gate_buf = vec![1.0; block_size];
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should re-enter attack phase (first sample triggers on rising edge)
        assert_eq!(asr.phase(), ASRPhase::Attack, "Should restart in Attack phase on retrigger");
    }

    #[test]
    fn test_asr_gate_off_releases() {
        // Test 5: Gate off should immediately trigger release from any phase
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut asr = ASREnvelopeNode::new(0, 1, 2, 3);

        // Start in attack phase (mid-attack)
        asr.state.phase = ASRPhase::Attack;
        asr.state.value = 0.5;
        asr.state.gate_was_high = true;

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Gate goes low immediately
        let gate_buf = vec![0.0; block_size];
        let attack_buf = vec![0.01; block_size];
        let sustain_buf = vec![0.6; block_size];
        let release_buf = vec![0.1; block_size];

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release phase immediately
        assert!(
            asr.phase() == ASRPhase::Release || asr.phase() == ASRPhase::Idle,
            "Should enter Release on gate off, got {:?}",
            asr.phase()
        );

        // Envelope should be falling or zero
        assert!(output[0] >= output[block_size - 1], "Should be releasing");
    }

    #[test]
    fn test_asr_dependencies() {
        // Test 6: Verify input_nodes returns correct dependencies
        let asr = ASREnvelopeNode::new(10, 20, 30, 40);
        let deps = asr.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // gate_input
        assert_eq!(deps[1], 20); // attack_input
        assert_eq!(deps[2], 30); // sustain_input
        assert_eq!(deps[3], 40); // release_input
    }

    #[test]
    fn test_asr_with_constants() {
        // Test 7: Full envelope cycle with constant parameters
        let sample_rate = 44100.0;
        let attack_time = 0.001;  // 1ms = 44 samples
        let sustain_level = 0.5;
        let release_time = 0.001; // 1ms = 44 samples
        let block_size = 64;

        let mut gate_node = ConstantNode::new(1.0);
        let mut attack_node = ConstantNode::new(attack_time);
        let mut sustain_node = ConstantNode::new(sustain_level);
        let mut release_node = ConstantNode::new(release_time);

        let mut asr = ASREnvelopeNode::new(0, 1, 2, 3);

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
        let mut sustain_buf = vec![0.0; block_size];
        let mut release_buf = vec![0.0; block_size];

        gate_node.process_block(&[], &mut gate_buf, sample_rate, &context);
        attack_node.process_block(&[], &mut attack_buf, sample_rate, &context);
        sustain_node.process_block(&[], &mut sustain_buf, sample_rate, &context);
        release_node.process_block(&[], &mut release_buf, sample_rate, &context);

        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        // Process attack
        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack or sustain phase
        assert!(
            asr.phase() == ASRPhase::Attack || asr.phase() == ASRPhase::Sustain,
            "Should be in Attack or Sustain"
        );
        assert!(output[block_size - 1] > 0.0, "Envelope should be active");

        // Process more to reach sustain
        for _ in 0..3 {
            let mut output = vec![0.0; block_size];
            asr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should be at sustain
        assert_eq!(asr.phase(), ASRPhase::Sustain);
        assert!(
            (asr.value() - sustain_level).abs() < 0.01,
            "Should be at sustain level {}, got {}",
            sustain_level,
            asr.value()
        );

        // Release: gate off
        gate_buf.fill(0.0);
        let inputs = vec![
            gate_buf.as_slice(),
            attack_buf.as_slice(),
            sustain_buf.as_slice(),
            release_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        asr.process_block(&inputs, &mut output, sample_rate, &context);

        // Should enter release (or already be idle with fast release)
        assert!(
            asr.phase() == ASRPhase::Release || asr.phase() == ASRPhase::Idle,
            "Should be in Release or Idle, got {:?}",
            asr.phase()
        );

        // Process to idle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            asr.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle
        assert_eq!(asr.phase(), ASRPhase::Idle);
        assert!(asr.value() < 0.01);
    }
}
