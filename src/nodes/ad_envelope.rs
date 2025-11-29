/// AD Envelope Generator - Attack, Decay
///
/// Trigger-based envelope with two phases:
/// - Attack: Linear ramp from 0.0 to 1.0 when triggered
/// - Decay: Linear ramp from 1.0 to 0.0 after attack completes
///
/// Unlike AR envelope (gate-based), AD is trigger-based (one-shot).
/// Ideal for percussion, drums, plucks - short transient sounds.
/// All time parameters are in seconds.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// AD envelope phase
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ADPhase {
    Idle,   // No trigger, envelope is at 0.0
    Attack, // Ramping from 0.0 to 1.0
    Decay,  // Ramping from 1.0 to 0.0
}

/// AD envelope state machine
#[derive(Debug, Clone)]
struct ADState {
    phase: ADPhase,    // Current envelope phase
    value: f32,        // Current envelope value (0.0 to 1.0)
    last_trigger: f32, // Track trigger transitions (rising edge)
}

impl Default for ADState {
    fn default() -> Self {
        Self {
            phase: ADPhase::Idle,
            value: 0.0,
            last_trigger: 0.0,
        }
    }
}

/// AD Envelope Generator Node
///
/// # Inputs
/// 1. Trigger input (> 0.5 = trigger on rising edge)
/// 2. Attack time in seconds
/// 3. Decay time in seconds
///
/// # Example
/// ```ignore
/// // Create AD envelope for percussion
/// let trigger = ConstantNode::new(1.0);        // NodeId 0 (trigger)
/// let attack = ConstantNode::new(0.001);       // NodeId 1 (1ms attack)
/// let decay = ConstantNode::new(0.2);          // NodeId 2 (200ms decay)
/// let ad = ADEnvelopeNode::new(0, 1, 2);       // NodeId 3
/// ```
pub struct ADEnvelopeNode {
    trigger_input: NodeId, // Trigger input (rising edge detection)
    attack_input: NodeId,  // Attack time in seconds
    decay_input: NodeId,   // Decay time in seconds
    state: ADState,        // Internal state machine
}

impl ADEnvelopeNode {
    /// ADEnvelope - Trigger-based Attack-Decay envelope generator
    ///
    /// One-shot envelope with two phases: attack (0 to 1) then decay (1 to 0).
    /// Retriggered on each rising edge of trigger signal. Ideal for percussion and short transients.
    ///
    /// # Parameters
    /// - `trigger_input`: Trigger signal (rising edge triggers envelope)
    /// - `attack_input`: Attack time in seconds
    /// - `decay_input`: Decay time in seconds
    ///
    /// # Example
    /// ```phonon
    /// ~trigger: "x ~ x ~"
    /// ~envelope: ~trigger # ad_envelope 0.001 0.2
    /// ~sound: sine 440 * ~envelope
    /// ```
    pub fn new(trigger_input: NodeId, attack_input: NodeId, decay_input: NodeId) -> Self {
        Self {
            trigger_input,
            attack_input,
            decay_input,
            state: ADState::default(),
        }
    }

    /// Get current envelope value
    pub fn value(&self) -> f32 {
        self.state.value
    }

    /// Get current phase
    pub fn phase(&self) -> ADPhase {
        self.state.phase
    }

    /// Reset envelope to idle state
    pub fn reset(&mut self) {
        self.state = ADState::default();
    }
}

impl AudioNode for ADEnvelopeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "ADEnvelopeNode requires 3 inputs: trigger, attack, decay"
        );

        let trigger_buffer = inputs[0];
        let attack_buffer = inputs[1];
        let decay_buffer = inputs[2];

        debug_assert_eq!(
            trigger_buffer.len(),
            output.len(),
            "Trigger buffer length mismatch"
        );
        debug_assert_eq!(
            attack_buffer.len(),
            output.len(),
            "Attack buffer length mismatch"
        );
        debug_assert_eq!(
            decay_buffer.len(),
            output.len(),
            "Decay buffer length mismatch"
        );

        for i in 0..output.len() {
            let trigger = trigger_buffer[i];
            let attack_time = attack_buffer[i].max(0.0001); // Minimum 0.1ms
            let decay_time = decay_buffer[i].max(0.0001);

            // Detect rising edge (trigger goes high)
            let trigger_rising = trigger > 0.5 && self.state.last_trigger <= 0.5;
            self.state.last_trigger = trigger;

            // Trigger: start attack phase
            if trigger_rising {
                self.state.phase = ADPhase::Attack;
                self.state.value = 0.0; // Reset to 0 for one-shot behavior
            }

            // Process envelope based on current phase
            match self.state.phase {
                ADPhase::Idle => {
                    // Envelope is off, output 0.0
                    self.state.value = 0.0;
                }

                ADPhase::Attack => {
                    // Ramp from 0.0 to 1.0 over attack_time
                    let increment = 1.0 / (attack_time * sample_rate);
                    self.state.value += increment;

                    if self.state.value >= 1.0 {
                        self.state.value = 1.0;
                        // Automatically transition to decay phase
                        self.state.phase = ADPhase::Decay;
                    }
                }

                ADPhase::Decay => {
                    // Ramp from current value to 0.0 over decay_time
                    let decrement = 1.0 / (decay_time * sample_rate);
                    self.state.value -= decrement;

                    if self.state.value <= 0.0 {
                        self.state.value = 0.0;
                        self.state.phase = ADPhase::Idle;
                    }
                }
            }

            // Output current envelope value
            output[i] = self.state.value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.trigger_input, self.attack_input, self.decay_input]
    }

    fn name(&self) -> &str {
        "ADEnvelopeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_ad_idle_when_no_trigger() {
        // Test 1: When no trigger, envelope should be idle at 0.0
        let mut trigger = ConstantNode::new(0.0); // No trigger
        let mut attack = ConstantNode::new(0.01);
        let mut decay = ConstantNode::new(0.2);

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Generate input buffers
        let mut trigger_buf = vec![0.0; 512];
        let mut attack_buf = vec![0.0; 512];
        let mut decay_buf = vec![0.0; 512];

        trigger.process_block(&[], &mut trigger_buf, 44100.0, &context);
        attack.process_block(&[], &mut attack_buf, 44100.0, &context);
        decay.process_block(&[], &mut decay_buf, 44100.0, &context);

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; 512];
        ad.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be 0.0 (idle)
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(sample, 0.0, "Sample {} should be 0.0 when no trigger", i);
        }

        assert_eq!(ad.phase(), ADPhase::Idle);
        assert_eq!(ad.value(), 0.0);
    }

    #[test]
    fn test_ad_attack_phase_rises() {
        // Test 2: Attack phase should ramp from 0.0 to 1.0
        let sample_rate = 44100.0;
        let attack_time = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger on first sample
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0; // Rising edge
        for i in 1..block_size {
            trigger_buf[i] = 1.0; // Stay high (doesn't matter, only edge triggers)
        }

        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![0.2; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // First sample should start rising
        assert!(output[0] > 0.0, "Attack should start rising immediately");

        // Envelope should be monotonically rising during attack
        assert!(output[100] > output[50], "Attack should be rising");
        assert!(output[200] > output[100], "Attack should continue rising");

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
    fn test_ad_decay_phase_falls() {
        // Test 3: After attack completes, decay phase should fall to 0.0
        let sample_rate = 44100.0;
        let attack_time = 0.001; // 1ms (very fast)
        let decay_time = 0.01; // 10ms = 441 samples
        let block_size = 1024; // Large enough to observe full envelope

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0; // Rising edge

        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![decay_time; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Attack completes very quickly (44 samples)
        let attack_samples = (attack_time * sample_rate) as usize;

        // After attack, should be in decay phase
        let mid_decay = attack_samples + 100;
        let late_decay = attack_samples + 300;

        if late_decay < block_size {
            // Decay should be falling
            assert!(
                output[mid_decay] > output[late_decay],
                "Decay should be falling: {} > {}",
                output[mid_decay],
                output[late_decay]
            );
        }

        // Should reach idle eventually
        assert!(
            ad.phase() == ADPhase::Decay || ad.phase() == ADPhase::Idle,
            "Should be in Decay or Idle phase, got {:?}",
            ad.phase()
        );
    }

    #[test]
    fn test_ad_total_duration() {
        // Test 4: Total envelope duration should equal attack + decay
        let sample_rate = 44100.0;
        let attack_time = 0.002; // 2ms = 88 samples
        let decay_time = 0.003; // 3ms = 132 samples
        let total_time = attack_time + decay_time; // 5ms = 220 samples
        let block_size = 512;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![decay_time; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        let total_samples = (total_time * sample_rate) as usize;

        // Should be essentially complete (< 0.1) after total_time
        if total_samples + 50 < block_size {
            assert!(
                output[total_samples + 50] < 0.1,
                "Envelope should be nearly complete after total duration, got {}",
                output[total_samples + 50]
            );
        }

        // Should eventually reach idle
        assert!(
            ad.phase() == ADPhase::Idle || ad.phase() == ADPhase::Decay,
            "Should be idle or decaying, got {:?}",
            ad.phase()
        );
    }

    #[test]
    fn test_ad_retrigger() {
        // Test 5: Multiple triggers should restart envelope
        let sample_rate = 44100.0;
        let block_size = 128;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let attack_buf = vec![0.005; block_size]; // 5ms
        let decay_buf = vec![0.01; block_size]; // 10ms

        // First trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack or decay
        assert!(
            ad.phase() == ADPhase::Attack || ad.phase() == ADPhase::Decay,
            "Should be in Attack or Decay, got {:?}",
            ad.phase()
        );

        let first_value = ad.value();
        assert!(first_value > 0.0, "Envelope should be active");

        // Second trigger (retrigger)
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 0.0; // Low
        trigger_buf[1] = 1.0; // Rising edge

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Should restart in attack phase
        assert_eq!(
            ad.phase(),
            ADPhase::Attack,
            "Should restart in Attack phase on retrigger"
        );

        // Value at sample 1 should be reset (close to 0, just starting)
        assert!(
            output[1] < 0.1,
            "Retrigger should reset envelope, got {}",
            output[1]
        );
    }

    #[test]
    fn test_ad_attack_time_modulation() {
        // Test 6: Attack time parameter should affect attack slope
        let sample_rate = 44100.0;
        let block_size = 256;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Fast attack
        let mut ad_fast = ADEnvelopeNode::new(0, 1, 2);
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;
        let fast_attack_buf = vec![0.001; block_size]; // 1ms
        let decay_buf = vec![0.1; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            fast_attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output_fast = vec![0.0; block_size];
        ad_fast.process_block(&inputs, &mut output_fast, sample_rate, &context);

        // Slow attack
        let mut ad_slow = ADEnvelopeNode::new(0, 1, 2);
        let slow_attack_buf = vec![0.01; block_size]; // 10ms

        let inputs = vec![
            trigger_buf.as_slice(),
            slow_attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output_slow = vec![0.0; block_size];
        ad_slow.process_block(&inputs, &mut output_slow, sample_rate, &context);

        // Fast attack should reach higher values sooner
        assert!(
            output_fast[50] > output_slow[50],
            "Fast attack should be ahead of slow attack: {} > {}",
            output_fast[50],
            output_slow[50]
        );
    }

    #[test]
    fn test_ad_decay_time_modulation() {
        // Test 7: Decay time parameter should affect decay slope
        let sample_rate = 44100.0;
        let attack_time = 0.0001; // Very fast attack
        let block_size = 1024;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Fast decay
        let mut ad_fast = ADEnvelopeNode::new(0, 1, 2);
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;
        let attack_buf = vec![attack_time; block_size];
        let fast_decay_buf = vec![0.005; block_size]; // 5ms

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            fast_decay_buf.as_slice(),
        ];

        let mut output_fast = vec![0.0; block_size];
        ad_fast.process_block(&inputs, &mut output_fast, sample_rate, &context);

        // Slow decay
        let mut ad_slow = ADEnvelopeNode::new(0, 1, 2);
        let slow_decay_buf = vec![0.05; block_size]; // 50ms

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            slow_decay_buf.as_slice(),
        ];

        let mut output_slow = vec![0.0; block_size];
        ad_slow.process_block(&inputs, &mut output_slow, sample_rate, &context);

        // After peak, slow decay should retain higher values longer
        let mid_point = 400;
        if mid_point < block_size {
            assert!(
                output_slow[mid_point] > output_fast[mid_point],
                "Slow decay should be above fast decay: {} > {}",
                output_slow[mid_point],
                output_fast[mid_point]
            );
        }
    }

    #[test]
    fn test_ad_envelope_stays_at_zero_after_completion() {
        // Test 8: Envelope should stay at 0.0 after decay completes
        let sample_rate = 44100.0;
        let attack_time = 0.001; // 1ms
        let decay_time = 0.002; // 2ms
        let block_size = 512;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![decay_time; block_size];

        // Process multiple blocks to ensure completion
        for i in 0..5 {
            let inputs = vec![
                trigger_buf.as_slice(),
                attack_buf.as_slice(),
                decay_buf.as_slice(),
            ];

            let mut output = vec![0.0; block_size];
            ad.process_block(&inputs, &mut output, sample_rate, &context);

            // Reset trigger after first block
            if i == 0 {
                trigger_buf.fill(0.0);
            }
        }

        // Should be idle at 0.0
        assert_eq!(ad.phase(), ADPhase::Idle);
        assert_eq!(ad.value(), 0.0);

        // Process one more block - should stay at 0.0
        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(sample, 0.0, "Sample {} should be 0.0 after completion", i);
        }
    }

    #[test]
    fn test_ad_output_range() {
        // Test 9: Output should always be in range [0.0, 1.0]
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let attack_buf = vec![0.005; block_size];
        let decay_buf = vec![0.01; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Check all samples are in valid range
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample >= 0.0 && sample <= 1.0,
                "Sample {} out of range [0.0, 1.0]: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_ad_dependencies() {
        // Test 10: Verify input_nodes returns correct dependencies
        let ad = ADEnvelopeNode::new(10, 20, 30);
        let deps = ad.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // trigger_input
        assert_eq!(deps[1], 20); // attack_input
        assert_eq!(deps[2], 30); // decay_input
    }

    #[test]
    fn test_ad_with_constants() {
        // Test 11: Full envelope cycle with constant parameters
        let sample_rate = 44100.0;
        let attack_time = 0.002; // 2ms = 88 samples
        let decay_time = 0.003; // 3ms = 132 samples
        let block_size = 512;

        let mut attack_node = ConstantNode::new(attack_time);
        let mut decay_node = ConstantNode::new(decay_time);

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Create trigger buffer with rising edge
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0; // Rising edge on first sample

        // Generate constant attack/decay buffers
        let mut attack_buf = vec![0.0; block_size];
        let mut decay_buf = vec![0.0; block_size];

        attack_node.process_block(&[], &mut attack_buf, sample_rate, &context);
        decay_node.process_block(&[], &mut decay_buf, sample_rate, &context);

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        // Process first block
        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be in attack or decay (envelope completes in 220 samples)
        assert!(
            ad.phase() == ADPhase::Attack
                || ad.phase() == ADPhase::Decay
                || ad.phase() == ADPhase::Idle,
            "Should be in Attack, Decay, or Idle (envelope might complete)"
        );
        assert!(
            output[block_size - 1] >= 0.0,
            "Envelope should have valid output"
        );

        // Process more blocks to complete
        trigger_buf.fill(0.0); // No more triggers
        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        for _ in 0..5 {
            let mut output = vec![0.0; block_size];
            ad.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should reach idle
        assert_eq!(ad.phase(), ADPhase::Idle);
        assert!(ad.value() < 0.01, "Should be at 0.0, got {}", ad.value());
    }

    #[test]
    fn test_ad_percussion_use_case() {
        // Test 12: Typical percussion envelope (very short attack, medium decay)
        let sample_rate = 44100.0;
        let attack_time = 0.0005; // 0.5ms (percussive strike)
        let decay_time = 0.15; // 150ms (resonance)
        let block_size = 512;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![decay_time; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Attack should complete very quickly (within ~22 samples)
        let attack_samples = (attack_time * sample_rate) as usize;
        assert!(attack_samples < 30, "Attack should be very fast");

        // Should reach near-peak quickly
        if attack_samples + 10 < block_size {
            assert!(
                output[attack_samples + 10] >= 0.95,
                "Should reach peak quickly, got {}",
                output[attack_samples + 10]
            );
        }

        // Should still be decaying at end of first block (long decay)
        assert!(
            output[block_size - 1] > 0.5,
            "Should still be decaying (not complete), got {}",
            output[block_size - 1]
        );
        assert_eq!(ad.phase(), ADPhase::Decay, "Should be in decay phase");
    }

    #[test]
    fn test_ad_pluck_use_case() {
        // Test 13: Plucked string envelope (very short attack, long decay)
        let sample_rate = 44100.0;
        let attack_time = 0.0001; // 0.1ms (instant pluck)
        let decay_time = 1.0; // 1 second (string resonance)
        let block_size = 512;

        let mut ad = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let attack_buf = vec![attack_time; block_size];
        let decay_buf = vec![decay_time; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        ad.process_block(&inputs, &mut output, sample_rate, &context);

        // Attack should be essentially instant (minimum time enforced)
        let attack_samples = (attack_time * sample_rate).max(0.0001 * sample_rate) as usize;
        assert!(attack_samples < 10, "Attack should be near-instant");

        // Should be decaying very slowly (1 second decay)
        let decay_rate = 1.0 / (decay_time * sample_rate);
        assert!(decay_rate < 0.0001, "Decay should be very slow");

        // Should still be near peak at end of first block
        assert!(
            output[block_size - 1] > 0.95,
            "Should still be near peak with slow decay, got {}",
            output[block_size - 1]
        );
        assert_eq!(ad.phase(), ADPhase::Decay, "Should be in decay phase");
    }

    #[test]
    fn test_ad_state_isolation() {
        // Test 14: Multiple instances should have independent state
        let sample_rate = 44100.0;
        let block_size = 256;

        let mut ad1 = ADEnvelopeNode::new(0, 1, 2);
        let mut ad2 = ADEnvelopeNode::new(0, 1, 2);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Trigger ad1
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let attack_buf = vec![0.01; block_size];
        let decay_buf = vec![0.1; block_size];

        let inputs = vec![
            trigger_buf.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output1 = vec![0.0; block_size];
        ad1.process_block(&inputs, &mut output1, sample_rate, &context);

        // ad1 should be active
        assert!(ad1.value() > 0.0);

        // ad2 should still be idle (no trigger)
        let trigger_buf_idle = vec![0.0; block_size];
        let inputs_idle = vec![
            trigger_buf_idle.as_slice(),
            attack_buf.as_slice(),
            decay_buf.as_slice(),
        ];

        let mut output2 = vec![0.0; block_size];
        ad2.process_block(&inputs_idle, &mut output2, sample_rate, &context);

        assert_eq!(ad2.phase(), ADPhase::Idle, "ad2 should remain idle");
        assert_eq!(ad2.value(), 0.0, "ad2 should be at 0.0");
    }
}
