/// Oscillator node - generates waveforms (sine, saw, square, triangle)
///
/// This node demonstrates stateful processing (phase tracking) and
/// pattern-controlled parameters (frequency can be modulated).

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Waveform types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

/// Oscillator node with pattern-controlled frequency
///
/// # Example
/// ```ignore
/// // 440 Hz sine wave
/// let freq_const = ConstantNode::new(440.0);  // NodeId 0
/// let osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
/// ```
pub struct OscillatorNode {
    freq_input: NodeId,     // NodeId providing frequency values
    waveform: Waveform,
    phase: f32,             // Internal state (0.0 to 1.0)
}

impl OscillatorNode {
    /// OscillatorNode - Multi-waveform synthesizer oscillator with phase tracking
    ///
    /// Generates periodic waveforms (sine, sawtooth, square, triangle) at variable
    /// frequency. The phase continuously accumulates across blocks for smooth,
    /// aliasing-prone synthesis suitable for DSP experimentation.
    ///
    /// # Parameters
    /// - `freq_input`: NodeId providing frequency in Hz
    /// - `waveform`: Waveform type (Sine, Saw, Square, Triangle)
    ///
    /// # Example
    /// ```phonon
    /// ~osc: oscillator 440 sine
    /// ```
    pub fn new(freq_input: NodeId, waveform: Waveform) -> Self {
        Self {
            freq_input,
            waveform,
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

    /// Set waveform type
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    /// Get waveform type
    pub fn waveform(&self) -> Waveform {
        self.waveform
    }
}

impl AudioNode for OscillatorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "OscillatorNode requires frequency input"
        );

        let freq_buffer = inputs[0];

        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );

        for i in 0..output.len() {
            let freq = freq_buffer[i];

            // Generate sample based on waveform
            output[i] = match self.waveform {
                Waveform::Sine => (self.phase * 2.0 * PI).sin(),

                Waveform::Saw => 2.0 * self.phase - 1.0,

                Waveform::Square => {
                    if self.phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }

                Waveform::Triangle => {
                    if self.phase < 0.5 {
                        4.0 * self.phase - 1.0
                    } else {
                        -4.0 * self.phase + 3.0
                    }
                }
            };

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
        vec![self.freq_input]
    }

    fn name(&self) -> &str {
        "OscillatorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_oscillator_sine_dc_offset() {
        // Sine wave should have zero DC offset
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

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

        // Generate oscillator output
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate average (should be near 0 for sine wave)
        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;

        assert!(avg.abs() < 0.1, "Sine wave DC offset too high: {}", avg);
    }

    #[test]
    fn test_oscillator_sine_range() {
        // Sine wave should be in [-1.0, 1.0]
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                *sample >= -1.0 && *sample <= 1.0,
                "Sine sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_oscillator_phase_advances() {
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        assert_eq!(osc.phase(), 0.0);

        // Process one sample at 440 Hz
        let freq_buf = vec![440.0];
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should have advanced by 440/44100
        let expected_phase = 440.0 / 44100.0;
        assert!(
            (osc.phase() - expected_phase).abs() < 0.0001,
            "Phase mismatch: got {}, expected {}",
            osc.phase(),
            expected_phase
        );
    }

    #[test]
    fn test_oscillator_phase_wraps() {
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Set phase close to 1.0
        osc.phase = 0.99;

        // Process one sample at high frequency
        let freq_buf = vec![4410.0];  // 10% of sample rate
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should wrap back to [0.0, 1.0)
        assert!(
            osc.phase() >= 0.0 && osc.phase() < 1.0,
            "Phase didn't wrap: {}",
            osc.phase()
        );
    }

    #[test]
    fn test_oscillator_waveforms() {
        let waveforms = vec![
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
        ];

        for waveform in waveforms {
            let mut const_freq = ConstantNode::new(440.0);
            let mut osc = OscillatorNode::new(0, waveform);

            let context = ProcessContext::new(
                Fraction::from_float(0.0),
                0,
                512,
                2.0,
                44100.0,
            );

            let mut freq_buf = vec![0.0; 512];
            const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice()];
            let mut output = vec![0.0; 512];
            osc.process_block(&inputs, &mut output, 44100.0, &context);

            // All waveforms should produce some non-zero output
            let has_signal = output.iter().any(|&x| x.abs() > 0.1);
            assert!(
                has_signal,
                "Waveform {:?} produced no signal",
                waveform
            );

            // All samples should be in valid range
            for sample in &output {
                assert!(
                    sample.abs() <= 1.1,  // Allow slight overshoot for rounding
                    "Waveform {:?} sample out of range: {}",
                    waveform,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_oscillator_reset_phase() {
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Advance phase
        osc.phase = 0.5;
        assert_eq!(osc.phase(), 0.5);

        // Reset
        osc.reset_phase();
        assert_eq!(osc.phase(), 0.0);
    }

    #[test]
    fn test_oscillator_dependencies() {
        let osc = OscillatorNode::new(42, Waveform::Sine);
        let deps = osc.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }
}
