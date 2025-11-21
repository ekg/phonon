/// Pulse wave oscillator with pulse width modulation (PWM)
///
/// This node generates a pulse wave with controllable pulse width.
/// - width = 0.5: Square wave (50% duty cycle)
/// - width < 0.5: Narrow pulse
/// - width > 0.5: Wide pulse
///
/// Pulse width modulation (PWM) creates rich harmonic content and is
/// fundamental to many classic synthesizer sounds.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Pulse wave oscillator with pattern-controlled frequency and width
///
/// # Example
/// ```ignore
/// // 440 Hz square wave (width = 0.5)
/// let freq_const = ConstantNode::new(440.0);  // NodeId 0
/// let width_const = ConstantNode::new(0.5);   // NodeId 1
/// let pulse = PulseNode::new(0, 1);           // NodeId 2
/// ```
pub struct PulseNode {
    freq_input: NodeId,      // NodeId providing frequency values (Hz)
    width_input: NodeId,     // NodeId providing pulse width (0.0 to 1.0)
    phase: f32,              // Internal state (0.0 to 1.0)
}

impl PulseNode {
    /// PulseNode - Pulse wave oscillator with variable duty cycle modulation
    ///
    /// Generates pulse waves with continuously variable width from narrow to wide.
    /// Width modulation creates rich, evolving timbres from subtle to aggressive
    /// synthesis textures. Width of 0.5 produces classic square wave.
    ///
    /// # Parameters
    /// - `freq_input`: NodeId providing frequency in Hz
    /// - `width_input`: NodeId providing pulse width (0.0-1.0, 0.5 = square)
    ///
    /// # Example
    /// ```phonon
    /// ~lfo: sine 2
    /// ~pulse: pulse 220 ~lfo
    /// ```
    pub fn new(freq_input: NodeId, width_input: NodeId) -> Self {
        Self {
            freq_input,
            width_input,
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

impl AudioNode for PulseNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "PulseNode requires frequency and width inputs"
        );

        let freq_buffer = inputs[0];
        let width_buffer = inputs[1];

        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );
        debug_assert_eq!(
            width_buffer.len(),
            output.len(),
            "Width buffer length mismatch"
        );

        for i in 0..output.len() {
            let freq = freq_buffer[i];
            let width = width_buffer[i].clamp(0.0, 1.0);

            // Generate pulse wave
            output[i] = if self.phase < width { 1.0 } else { -1.0 };

            // Advance phase
            self.phase += freq / sample_rate;

            // Wrap phase to [0.0, 1.0)
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            while self.phase < 0.0 {
                self.phase += 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.freq_input, self.width_input]
    }

    fn name(&self) -> &str {
        "PulseNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_pulse_square_wave() {
        // Width = 0.5 should produce a square wave
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_width = ConstantNode::new(0.5);
        let mut pulse = PulseNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate frequency buffer
        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Generate width buffer
        let mut width_buf = vec![0.0; 512];
        const_width.process_block(&[], &mut width_buf, 44100.0, &context);

        // Generate pulse wave output
        let inputs = vec![freq_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; 512];
        pulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Should only contain 1.0 and -1.0
        for sample in &output {
            assert!(
                *sample == 1.0 || *sample == -1.0,
                "Pulse wave should only contain 1.0 or -1.0, got {}",
                sample
            );
        }

        // Count high and low samples
        let high_count = output.iter().filter(|&&x| x == 1.0).count();
        let low_count = output.iter().filter(|&&x| x == -1.0).count();

        // With width = 0.5, should have roughly equal high and low samples
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.5).abs() < 0.1,
            "Square wave duty cycle should be ~0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_pulse_narrow_width() {
        // Width = 0.1 should produce narrow pulses
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_width = ConstantNode::new(0.1);
        let mut pulse = PulseNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut width_buf = vec![0.0; 512];
        const_width.process_block(&[], &mut width_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; 512];
        pulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Count high and low samples
        let high_count = output.iter().filter(|&&x| x == 1.0).count();
        let low_count = output.iter().filter(|&&x| x == -1.0).count();

        // With width = 0.1, high samples should be ~10% of total
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.1).abs() < 0.1,
            "Narrow pulse duty cycle should be ~0.1, got {}",
            ratio
        );
    }

    #[test]
    fn test_pulse_wide_width() {
        // Width = 0.9 should produce wide pulses
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_width = ConstantNode::new(0.9);
        let mut pulse = PulseNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut width_buf = vec![0.0; 512];
        const_width.process_block(&[], &mut width_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; 512];
        pulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Count high and low samples
        let high_count = output.iter().filter(|&&x| x == 1.0).count();
        let low_count = output.iter().filter(|&&x| x == -1.0).count();

        // With width = 0.9, high samples should be ~90% of total
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.9).abs() < 0.1,
            "Wide pulse duty cycle should be ~0.9, got {}",
            ratio
        );
    }

    #[test]
    fn test_pulse_phase_advances() {
        let pulse = PulseNode::new(0, 1);

        assert_eq!(pulse.phase(), 0.0);

        // Process one sample at 440 Hz
        let freq_buf = vec![440.0];
        let width_buf = vec![0.5];
        let inputs = vec![freq_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        let mut pulse = pulse;
        pulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should have advanced by 440/44100
        let expected_phase = 440.0 / 44100.0;
        assert!(
            (pulse.phase() - expected_phase).abs() < 0.0001,
            "Phase mismatch: got {}, expected {}",
            pulse.phase(),
            expected_phase
        );
    }

    #[test]
    fn test_pulse_phase_wraps() {
        let mut pulse = PulseNode::new(0, 1);

        // Set phase close to 1.0
        pulse.phase = 0.99;

        // Process one sample at high frequency
        let freq_buf = vec![4410.0];  // 10% of sample rate
        let width_buf = vec![0.5];
        let inputs = vec![freq_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        pulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should wrap back to [0.0, 1.0)
        assert!(
            pulse.phase() >= 0.0 && pulse.phase() < 1.0,
            "Phase didn't wrap: {}",
            pulse.phase()
        );
    }

    #[test]
    fn test_pulse_dependencies() {
        let pulse = PulseNode::new(42, 43);
        let deps = pulse.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 42);
        assert_eq!(deps[1], 43);
    }

    #[test]
    fn test_pulse_dc_offset() {
        // Pulse wave should have zero DC offset (average near 0)
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_width = ConstantNode::new(0.5);
        let mut pulse = PulseNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4410,  // Process multiple complete cycles
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 4410];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut width_buf = vec![0.0; 4410];
        const_width.process_block(&[], &mut width_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        pulse.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate average (should be near 0 for square wave)
        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;

        assert!(
            avg.abs() < 0.1,
            "Pulse wave DC offset too high: {}",
            avg
        );
    }

    #[test]
    fn test_pulse_width_clamping() {
        // Test that width values outside [0.0, 1.0] are clamped
        let mut const_freq = ConstantNode::new(440.0);
        let mut pulse = PulseNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Test width > 1.0 (should clamp to 1.0)
        let mut const_width_high = ConstantNode::new(1.5);
        let mut width_buf_high = vec![0.0; 512];
        const_width_high.process_block(&[], &mut width_buf_high, 44100.0, &context);

        let inputs_high = vec![freq_buf.as_slice(), width_buf_high.as_slice()];
        let mut output_high = vec![0.0; 512];
        pulse.process_block(&inputs_high, &mut output_high, 44100.0, &context);

        // With width clamped to 1.0, should be all high (1.0)
        let all_high = output_high.iter().all(|&x| x == 1.0);
        assert!(
            all_high,
            "Width > 1.0 should clamp to 1.0 (all samples = 1.0)"
        );

        // Reset phase
        pulse.reset_phase();

        // Test width < 0.0 (should clamp to 0.0)
        let mut const_width_low = ConstantNode::new(-0.5);
        let mut width_buf_low = vec![0.0; 512];
        const_width_low.process_block(&[], &mut width_buf_low, 44100.0, &context);

        let inputs_low = vec![freq_buf.as_slice(), width_buf_low.as_slice()];
        let mut output_low = vec![0.0; 512];
        pulse.process_block(&inputs_low, &mut output_low, 44100.0, &context);

        // With width clamped to 0.0, should be all low (-1.0)
        let all_low = output_low.iter().all(|&x| x == -1.0);
        assert!(
            all_low,
            "Width < 0.0 should clamp to 0.0 (all samples = -1.0)"
        );
    }
}
