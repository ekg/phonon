/// Onset Envelope Node - Simple attack ramp for continuous signals
///
/// Generates a linear ramp from 0.0 to 1.0 over the attack time,
/// then sustains at 1.0 indefinitely. Used as a modifier for
/// oscillator signals: `sine 440 # attack 0.05`
///
/// For release, this node does not apply fade-out since continuous
/// oscillators have no "note off" concept. Release is handled
/// separately for sample-based playback.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

pub struct OnsetEnvelopeNode {
    attack_input: NodeId,
    release_input: Option<NodeId>,
    elapsed_samples: u64,
    current_value: f32,
}

impl OnsetEnvelopeNode {
    pub fn new_attack(attack_input: NodeId) -> Self {
        Self {
            attack_input,
            release_input: None,
            elapsed_samples: 0,
            current_value: 0.0,
        }
    }

    pub fn new_ar(attack_input: NodeId, release_input: NodeId) -> Self {
        Self {
            attack_input,
            release_input: Some(release_input),
            elapsed_samples: 0,
            current_value: 0.0,
        }
    }
}

impl AudioNode for OnsetEnvelopeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        let attack_buffer = inputs[0];

        for i in 0..output.len() {
            let attack_time = attack_buffer[i].max(0.0001);
            let attack_samples = attack_time * sample_rate;

            if self.current_value < 1.0 {
                let increment = 1.0 / attack_samples;
                self.current_value = (self.current_value + increment).min(1.0);
            }

            self.elapsed_samples += 1;
            output[i] = self.current_value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        let mut nodes = vec![self.attack_input];
        if let Some(release_id) = self.release_input {
            nodes.push(release_id);
        }
        nodes
    }

    fn name(&self) -> &str {
        "OnsetEnvelopeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    #[test]
    fn test_onset_envelope_ramps_up() {
        let mut env = OnsetEnvelopeNode::new_attack(0);
        let sample_rate = 44100.0;
        let attack_time = 0.01; // 10ms = 441 samples
        let block_size = 512;

        let attack_buf = vec![attack_time; block_size];
        let inputs = vec![attack_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start near 0 and ramp up
        assert!(output[0] > 0.0, "Should start rising");
        assert!(output[0] < 0.1, "Should start low");

        // After 441 samples (10ms at 44100), should be at ~1.0
        let expected_end = (attack_time * sample_rate) as usize;
        assert!(
            output[expected_end.min(block_size - 1)] >= 0.95,
            "Should reach ~1.0 by end of attack"
        );

        // After attack, should sustain at 1.0
        assert!(
            (output[block_size - 1] - 1.0).abs() < 0.01,
            "Should sustain at 1.0"
        );
    }

    #[test]
    fn test_onset_envelope_fast_vs_slow() {
        let sample_rate = 44100.0;
        let block_size = 882; // 20ms

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Fast attack (1ms)
        let mut env_fast = OnsetEnvelopeNode::new_attack(0);
        let fast_buf = vec![0.001_f32; block_size];
        let inputs_fast = vec![fast_buf.as_slice()];
        let mut output_fast = vec![0.0; block_size];
        env_fast.process_block(&inputs_fast, &mut output_fast, sample_rate, &context);

        // Slow attack (100ms)
        let mut env_slow = OnsetEnvelopeNode::new_attack(0);
        let slow_buf = vec![0.1_f32; block_size];
        let inputs_slow = vec![slow_buf.as_slice()];
        let mut output_slow = vec![0.0; block_size];
        env_slow.process_block(&inputs_slow, &mut output_slow, sample_rate, &context);

        // Fast should reach 1.0 earlier
        let fast_rms: f32 =
            (output_fast.iter().map(|x| x * x).sum::<f32>() / block_size as f32).sqrt();
        let slow_rms: f32 =
            (output_slow.iter().map(|x| x * x).sum::<f32>() / block_size as f32).sqrt();

        assert!(
            fast_rms > slow_rms * 1.5,
            "Fast attack should have higher RMS: fast={:.3}, slow={:.3}",
            fast_rms,
            slow_rms
        );
    }
}
