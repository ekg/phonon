/// Impulse node - generates periodic single-sample spikes
///
/// This node generates a 1.0 spike for a single sample at each phase wrap,
/// producing 0.0 for all other samples. Useful for triggering envelopes,
/// sequencing, or creating rhythmic gates.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Impulse generator node with pattern-controlled frequency
///
/// Produces a single-sample spike (1.0) at regular intervals determined by
/// frequency. The signal is 0.0 between spikes, creating a sparse signal
/// ideal for triggering envelopes or other event-based processing.
///
/// # Example
/// ```ignore
/// // 1 Hz impulse (one spike per second)
/// let freq_const = ConstantNode::new(1.0);  // NodeId 0
/// let impulse = ImpulseNode::new(0);  // NodeId 1
/// ```
pub struct ImpulseNode {
    frequency_input: NodeId,  // NodeId providing frequency values
    phase: f32,               // Internal state (0.0 to 1.0)
}

impl ImpulseNode {
    /// Create a new impulse generator node
    ///
    /// # Arguments
    /// * `frequency_input` - NodeId that provides frequency (can be constant or pattern)
    pub fn new(frequency_input: NodeId) -> Self {
        Self {
            frequency_input,
            phase: 0.0,
        }
    }

    /// Get current phase (0.0 to 1.0)
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Reset phase to 0.0
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }
}

impl AudioNode for ImpulseNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "ImpulseNode requires frequency input"
        );

        let freq_buffer = inputs[0];

        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );

        for i in 0..output.len() {
            let freq = freq_buffer[i];

            // Advance phase
            self.phase += freq / sample_rate;

            // Detect phase wrap and generate impulse
            if self.phase >= 1.0 {
                output[i] = 1.0;  // Impulse!

                // Wrap phase to [0.0, 1.0)
                while self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
            } else {
                output[i] = 0.0;  // Silence between impulses
            }

            // Handle negative frequencies (wrap backwards)
            while self.phase < 0.0 {
                self.phase += 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.frequency_input]
    }

    fn name(&self) -> &str {
        "ImpulseNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_impulse_generates_spikes() {
        // Impulse should generate periodic spikes
        // Use 1.1 seconds to ensure we get at least one spike (floating point safety)
        let mut const_freq = ConstantNode::new(1.0);  // 1 Hz
        let mut impulse = ImpulseNode::new(0);

        let buffer_size = 48510;  // 1.1 seconds at 44100 Hz
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            buffer_size,
            2.0,
            44100.0,
        );

        // Generate frequency buffer (ConstantNode takes empty inputs array)
        let mut freq_buf = vec![0.0; buffer_size];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Verify frequency buffer is filled
        assert_eq!(freq_buf[0], 1.0, "Frequency buffer should be 1.0");

        // Generate impulse output
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; buffer_size];
        impulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have at least one spike (1.0)
        let has_spike = output.iter().any(|&x| x >= 0.99);
        assert!(has_spike, "Impulse should generate at least one spike");

        // Count spikes (values > 0.5)
        let spike_count = output.iter().filter(|&&x| x > 0.5).count();
        assert!(spike_count >= 1, "Should have at least 1 spike at 1 Hz");
        assert!(spike_count <= 2, "Should have at most 2 spikes at 1 Hz over 1.1 seconds");
    }

    #[test]
    fn test_impulse_frequency_control() {
        // At 1 Hz, should get 1 spike per second
        // Use 2 seconds to guarantee at least 1 spike (avoids floating point edge cases)
        let mut const_freq = ConstantNode::new(1.0);
        let mut impulse = ImpulseNode::new(0);

        let buffer_size = 88200;  // 2 seconds
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            buffer_size,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; buffer_size];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Verify frequency buffer is filled
        assert_eq!(freq_buf[0], 1.0, "Frequency buffer should be 1.0");

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; buffer_size];
        impulse.process_block(&inputs, &mut output, 44100.0, &context);

        // At 1 Hz over 2 seconds, expect 2 spikes (±1 for edge cases)
        let spike_count = output.iter().filter(|&&x| x > 0.5).count();
        assert!(
            spike_count >= 1 && spike_count <= 3,
            "At 1 Hz over 2 seconds, expected 1-3 spikes, got {}",
            spike_count
        );
    }

    #[test]
    fn test_impulse_phase_wraps() {
        let mut impulse = ImpulseNode::new(0);

        // Set phase close to 1.0
        impulse.phase = 0.99;

        // Process one sample at moderate frequency
        let freq_buf = vec![441.0];  // ~1% of sample rate
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        impulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should wrap back to [0.0, 1.0)
        assert!(
            impulse.phase() >= 0.0 && impulse.phase() < 1.0,
            "Phase didn't wrap: {}",
            impulse.phase()
        );

        // Should have generated an impulse since we crossed 1.0
        assert_eq!(output[0], 1.0, "Should generate impulse when phase wraps");
    }

    #[test]
    fn test_impulse_count_over_time() {
        // At 10 Hz for 1 second, should get ~10 spikes
        let mut const_freq = ConstantNode::new(10.0);
        let mut impulse = ImpulseNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            44100,  // 1 second
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 44100];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        impulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Count spikes
        let spike_count = output.iter().filter(|&&x| x > 0.5).count();

        // At 10 Hz, expect 10 spikes (allow ±1 for timing)
        assert!(
            spike_count >= 9 && spike_count <= 11,
            "At 10 Hz, expected ~10 spikes, got {}",
            spike_count
        );
    }

    #[test]
    fn test_impulse_mostly_zero() {
        // Most samples should be 0.0 (sparse signal)
        let mut const_freq = ConstantNode::new(10.0);
        let mut impulse = ImpulseNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            44100,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 44100];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        impulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Count non-zero samples
        let non_zero_count = output.iter().filter(|&&x| x.abs() > 0.01).count();
        let zero_count = output.len() - non_zero_count;

        // At 10 Hz, should have ~10 spikes and ~44090 zeros
        let zero_percentage = (zero_count as f32 / output.len() as f32) * 100.0;
        assert!(
            zero_percentage > 99.0,
            "Impulse should be mostly zero, got {:.1}% zeros",
            zero_percentage
        );
    }

    #[test]
    fn test_impulse_dependencies() {
        let impulse = ImpulseNode::new(42);
        let deps = impulse.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }

    #[test]
    fn test_impulse_with_constant() {
        // Integration test: constant frequency source
        let mut const_freq = ConstantNode::new(2.0);  // 2 Hz
        let mut impulse = ImpulseNode::new(0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            88200,  // 2 seconds
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 88200];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 88200];
        impulse.process_block(&inputs, &mut output, 44100.0, &context);

        // At 2 Hz for 2 seconds, expect ~4 spikes
        let spike_count = output.iter().filter(|&&x| x > 0.5).count();
        assert!(
            spike_count >= 3 && spike_count <= 5,
            "At 2 Hz for 2 seconds, expected ~4 spikes, got {}",
            spike_count
        );

        // Verify spikes are actually 1.0
        for &sample in output.iter() {
            if sample > 0.5 {
                assert!(
                    (sample - 1.0).abs() < 0.01,
                    "Spike should be 1.0, got {}",
                    sample
                );
            }
        }
    }

    #[test]
    fn test_impulse_reset_phase() {
        let mut impulse = ImpulseNode::new(0);

        // Advance phase
        impulse.phase = 0.5;
        assert_eq!(impulse.phase(), 0.5);

        // Reset
        impulse.reset_phase();
        assert_eq!(impulse.phase(), 0.0);
    }
}
