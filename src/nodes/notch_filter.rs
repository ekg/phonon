/// Notch (band-reject) filter node using biquad filter
///
/// This node implements a resonant notch filter using the `biquad` crate.
/// The filter attenuates frequencies near the center frequency and passes
/// frequencies both below and above it. The Q parameter controls the width
/// of the rejection band (higher Q = narrower notch).
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Notch filter node with pattern-controlled center frequency and Q
///
/// # Example
/// ```ignore
/// // 1000 Hz sine wave with 1000 Hz notch (should be attenuated)
/// let freq_const = ConstantNode::new(1000.0);  // NodeId 0
/// let osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
///
/// // Notch at 1000 Hz with Q=1.0
/// let center_const = ConstantNode::new(1000.0);  // NodeId 2
/// let q_const = ConstantNode::new(1.0);  // NodeId 3
/// let notch = NotchFilterNode::new(1, 2, 3);  // NodeId 4
/// // Frequencies near 1000 Hz are rejected, others pass through
/// ```
pub struct NotchFilterNode {
    input: NodeId,
    center_freq_input: NodeId,
    q_input: NodeId,
    filter: DirectForm2Transposed<f32>,
    last_center_freq: f32,
    last_q: f32,
}

impl NotchFilterNode {
    /// NotchFilterNode - Removes specific frequency with variable rejection width
    ///
    /// A resonant notch filter that creates a narrow dip at the center frequency.
    /// Used to remove hum, feedback, or specific tonal elements while preserving
    /// surrounding frequencies.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to filter
    /// - `center_freq_input`: NodeId of center frequency to reject (Hz)
    /// - `q_input`: NodeId of resonance/Q factor (1.0-100.0 typical)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~filtered: ~signal # notch_filter 1000 10
    /// ```
    pub fn new(input: NodeId, center_freq_input: NodeId, q_input: NodeId) -> Self {
        // Initialize with default coefficients (will be updated on first process)
        let coeffs =
            Coefficients::<f32>::from_params(biquad::Type::Notch, 44100.0.hz(), 1000.0.hz(), 1.0)
                .unwrap();

        Self {
            input,
            center_freq_input,
            q_input,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_center_freq: 1000.0,
            last_q: 1.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the center frequency input node ID
    pub fn center_freq_input(&self) -> NodeId {
        self.center_freq_input
    }

    /// Get the Q input node ID
    pub fn q_input(&self) -> NodeId {
        self.q_input
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::Notch,
            44100.0.hz(),
            self.last_center_freq.hz(),
            self.last_q,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for NotchFilterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "NotchFilterNode requires 3 inputs (signal, center_freq, q), got {}",
            inputs.len()
        );

        let signal_buffer = inputs[0];
        let center_freq_buffer = inputs[1];
        let q_buffer = inputs[2];

        debug_assert_eq!(
            signal_buffer.len(),
            output.len(),
            "Signal buffer length mismatch"
        );
        debug_assert_eq!(
            center_freq_buffer.len(),
            output.len(),
            "Center frequency buffer length mismatch"
        );
        debug_assert_eq!(q_buffer.len(), output.len(), "Q buffer length mismatch");

        for i in 0..output.len() {
            let center_freq = center_freq_buffer[i].max(20.0).min(20000.0); // Clamp to valid range
            let q = q_buffer[i].max(0.1).min(20.0); // Clamp to valid range

            // Update coefficients if parameters changed
            if (center_freq - self.last_center_freq).abs() > 0.1 || (q - self.last_q).abs() > 0.01 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::Notch,
                    sample_rate.hz(),
                    center_freq.hz(),
                    q,
                )
                .unwrap();

                self.filter.update_coefficients(coeffs);
                self.last_center_freq = center_freq;
                self.last_q = q;
            }

            // Process sample through filter
            output[i] = self.filter.run(signal_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.center_freq_input, self.q_input]
    }

    fn name(&self) -> &str {
        "NotchFilterNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    /// Helper: Calculate RMS of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    #[test]
    fn test_notch_rejects_center_frequency() {
        // 1000 Hz oscillator
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Notch filter at 1000 Hz (should reject 1000 Hz)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut notch = NotchFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut center_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let notch_inputs = vec![
            unfiltered.as_slice(),
            center_buf.as_slice(),
            q_buf.as_slice(),
        ];
        notch.process_block(&notch_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Notch should strongly attenuate center frequency
        assert!(
            filtered_rms < unfiltered_rms * 0.3,
            "Notch at 1000 Hz should reject 1000 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_notch_passes_low_frequencies() {
        // Low frequency oscillator (100 Hz)
        let mut const_freq = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Notch filter at 1000 Hz (should pass 100 Hz)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut notch = NotchFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut center_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let notch_inputs = vec![
            unfiltered.as_slice(),
            center_buf.as_slice(),
            q_buf.as_slice(),
        ];
        notch.process_block(&notch_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Notch should pass frequencies well below center with minimal attenuation
        assert!(
            filtered_rms > unfiltered_rms * 0.8,
            "Notch at 1000 Hz should pass 100 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_notch_passes_high_frequencies() {
        // High frequency oscillator (8000 Hz)
        let mut const_freq = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Notch filter at 1000 Hz (should pass 8000 Hz)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut notch = NotchFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut center_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let notch_inputs = vec![
            unfiltered.as_slice(),
            center_buf.as_slice(),
            q_buf.as_slice(),
        ];
        notch.process_block(&notch_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Notch should pass frequencies well above center with minimal attenuation
        assert!(
            filtered_rms > unfiltered_rms * 0.8,
            "Notch at 1000 Hz should pass 8000 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_notch_q_factor_affects_bandwidth() {
        // Test with frequency NEAR center (1200 Hz) with notch at 1000 Hz
        // Low Q (wide notch) should still attenuate 1200 Hz
        // High Q (narrow notch) should pass 1200 Hz more
        let mut const_freq = ConstantNode::new(1200.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Notch at 1000 Hz with low Q (wide notch)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q_low = ConstantNode::new(0.5);
        let mut notch_low_q = NotchFilterNode::new(1, 2, 3);

        // Notch at 1000 Hz with high Q (narrow notch)
        let mut const_q_high = ConstantNode::new(5.0);
        let mut notch_high_q = NotchFilterNode::new(1, 2, 4);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut center_buf = vec![0.0; 512];
        let mut q_buf_low = vec![0.0; 512];
        let mut q_buf_high = vec![0.0; 512];
        let mut signal = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);
        const_q_low.process_block(&[], &mut q_buf_low, 44100.0, &context);
        const_q_high.process_block(&[], &mut q_buf_high, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&signal);

        // Test low Q
        let mut filtered_low_q = vec![0.0; 512];
        let notch_inputs_low = vec![
            signal.as_slice(),
            center_buf.as_slice(),
            q_buf_low.as_slice(),
        ];
        notch_low_q.process_block(&notch_inputs_low, &mut filtered_low_q, 44100.0, &context);

        // Test high Q
        let mut filtered_high_q = vec![0.0; 512];
        let notch_inputs_high = vec![
            signal.as_slice(),
            center_buf.as_slice(),
            q_buf_high.as_slice(),
        ];
        notch_high_q.process_block(&notch_inputs_high, &mut filtered_high_q, 44100.0, &context);

        let rms_low_q = calculate_rms(&filtered_low_q);
        let rms_high_q = calculate_rms(&filtered_high_q);

        // High Q (narrow notch) should pass 1200 Hz better than low Q (wide notch)
        // because 1200 Hz is outside the narrow notch but inside the wide notch
        assert!(
            rms_high_q > rms_low_q,
            "High Q (narrow) should pass 1200 Hz better than low Q (wide): low_q_rms={}, high_q_rms={}, unfiltered={}",
            rms_low_q,
            rms_high_q,
            unfiltered_rms
        );
    }

    #[test]
    fn test_notch_dependencies() {
        let notch = NotchFilterNode::new(10, 20, 30);
        let deps = notch.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // center_freq
        assert_eq!(deps[2], 30); // q
    }

    #[test]
    fn test_notch_with_constants() {
        // Test that notch filter works with constant inputs
        let mut signal = ConstantNode::new(1.0);
        let mut center = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(1.0);
        let mut notch = NotchFilterNode::new(0, 1, 2);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        let mut signal_buf = vec![0.0; 512];
        let mut center_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut output = vec![0.0; 512];

        signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        center.process_block(&[], &mut center_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            center_buf.as_slice(),
            q_buf.as_slice(),
        ];
        notch.process_block(&inputs, &mut output, 44100.0, &context);

        // DC signal should pass through notch filter (center is 1000 Hz, not DC)
        let output_rms = calculate_rms(&output);
        assert!(
            output_rms > 0.8,
            "DC should pass through 1000 Hz notch filter: RMS = {}",
            output_rms
        );
    }

    #[test]
    fn test_notch_filter_state_updates() {
        // Test that filter coefficients update when parameters change
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Start with center at 2000 Hz (1000 Hz signal should pass)
        let mut const_center = ConstantNode::new(2000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut notch = NotchFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // First pass: 1000 Hz signal with notch at 2000 Hz
        let mut freq_buf = vec![0.0; 512];
        let mut center_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut signal = vec![0.0; 512];
        let mut output1 = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        let notch_inputs = vec![signal.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        notch.process_block(&notch_inputs, &mut output1, 44100.0, &context);

        let rms1 = calculate_rms(&output1);

        // Change center to 1000 Hz (1000 Hz signal should be rejected)
        const_center.set_value(1000.0);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);

        let mut output2 = vec![0.0; 512];
        let notch_inputs2 = vec![signal.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        notch.process_block(&notch_inputs2, &mut output2, 44100.0, &context);

        let rms2 = calculate_rms(&output2);

        // With notch at 1000 Hz, 1000 Hz signal should be more attenuated than with notch at 2000 Hz
        assert!(
            rms2 < rms1 * 0.5,
            "Notch at 1000 Hz should reject 1000 Hz more than notch at 2000 Hz: notch=2000Hz RMS={}, notch=1000Hz RMS={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_notch_reset() {
        let mut notch = NotchFilterNode::new(0, 1, 2);

        // Change internal state
        notch.last_center_freq = 5000.0;
        notch.last_q = 10.0;

        // Reset should reinitialize filter
        notch.reset();

        // Filter should still work after reset
        let signal_buffer = vec![0.5; 512];
        let center_buf = vec![1000.0; 512];
        let q_buf = vec![1.0; 512];
        let mut output = vec![0.0; 512];

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        let inputs = vec![
            signal_buffer.as_slice(),
            center_buf.as_slice(),
            q_buf.as_slice(),
        ];
        notch.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        assert!(output.iter().all(|&x| x.is_finite()));
    }
}
