/// BandPass filter node using biquad filter
///
/// This node implements a resonant bandpass filter using the `biquad` crate.
/// The filter passes frequencies near the center frequency and attenuates
/// frequencies both below and above it. The Q parameter controls the bandwidth
/// of the passband.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// BandPass filter node with pattern-controlled center frequency and Q
///
/// # Example
/// ```ignore
/// // 1000 Hz sine wave
/// let freq_const = ConstantNode::new(1000.0);  // NodeId 0
/// let osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
///
/// // Center frequency at 1000 Hz with Q=1.0
/// let center_const = ConstantNode::new(1000.0);  // NodeId 2
/// let q_const = ConstantNode::new(1.0);  // NodeId 3
/// let bpf = BandPassFilterNode::new(1, 2, 3);  // NodeId 4
/// // Frequencies near 1000 Hz pass through, others are attenuated
/// ```
pub struct BandPassFilterNode {
    input: NodeId,
    center_input: NodeId,
    q_input: NodeId,
    filter: DirectForm2Transposed<f32>,
    last_center: f32,
    last_q: f32,
}

impl BandPassFilterNode {
    /// Create a new bandpass filter node
    ///
    /// # Arguments
    /// * `input` - NodeId providing audio signal to filter
    /// * `center_input` - NodeId providing center frequency (Hz)
    /// * `q_input` - NodeId providing resonance (Q factor)
    pub fn new(input: NodeId, center_input: NodeId, q_input: NodeId) -> Self {
        // Initialize with default coefficients (will be updated on first process)
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            44100.0.hz(),
            1000.0.hz(),
            1.0,
        )
        .unwrap();

        Self {
            input,
            center_input,
            q_input,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_center: 1000.0,
            last_q: 1.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the center frequency input node ID
    pub fn center_input(&self) -> NodeId {
        self.center_input
    }

    /// Get the Q input node ID
    pub fn q_input(&self) -> NodeId {
        self.q_input
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            44100.0.hz(),
            self.last_center.hz(),
            self.last_q,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for BandPassFilterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "BandPassFilterNode requires 3 inputs (signal, center, q), got {}",
            inputs.len()
        );

        let signal_buffer = inputs[0];
        let center_buffer = inputs[1];
        let q_buffer = inputs[2];

        debug_assert_eq!(
            signal_buffer.len(),
            output.len(),
            "Signal buffer length mismatch"
        );
        debug_assert_eq!(
            center_buffer.len(),
            output.len(),
            "Center buffer length mismatch"
        );
        debug_assert_eq!(
            q_buffer.len(),
            output.len(),
            "Q buffer length mismatch"
        );

        for i in 0..output.len() {
            let center = center_buffer[i].max(20.0).min(20000.0); // Clamp to valid range
            let q = q_buffer[i].max(0.1).min(20.0); // Clamp to valid range

            // Update coefficients if parameters changed
            if (center - self.last_center).abs() > 0.1 || (q - self.last_q).abs() > 0.01 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::BandPass,
                    sample_rate.hz(),
                    center.hz(),
                    q,
                )
                .unwrap();

                self.filter.update_coefficients(coeffs);
                self.last_center = center;
                self.last_q = q;
            }

            // Process sample through filter
            output[i] = self.filter.run(signal_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.center_input, self.q_input]
    }

    fn name(&self) -> &str {
        "BandPassFilterNode"
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
    fn test_bandpass_passes_center_frequency() {
        // 1000 Hz oscillator
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Bandpass filter at 1000 Hz (should pass 1000 Hz)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut bpf = BandPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

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
        let bpf_inputs = vec![unfiltered.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        bpf.process_block(&bpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Bandpass should pass center frequency (within 20%)
        assert!(
            filtered_rms > unfiltered_rms * 0.8,
            "Bandpass at 1000 Hz should pass 1000 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_bandpass_attenuates_below_center() {
        // Low frequency oscillator (100 Hz)
        let mut const_freq = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Bandpass filter at 1000 Hz (should attenuate 100 Hz)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut bpf = BandPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

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
        let bpf_inputs = vec![unfiltered.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        bpf.process_block(&bpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Bandpass should significantly attenuate frequencies below center
        assert!(
            filtered_rms < unfiltered_rms * 0.3,
            "Bandpass at 1000 Hz should attenuate 100 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_bandpass_attenuates_above_center() {
        // High frequency oscillator (8000 Hz)
        let mut const_freq = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Bandpass filter at 1000 Hz (should attenuate 8000 Hz)
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut bpf = BandPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

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
        let bpf_inputs = vec![unfiltered.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        bpf.process_block(&bpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Bandpass should significantly attenuate frequencies above center
        assert!(
            filtered_rms < unfiltered_rms * 0.3,
            "Bandpass at 1000 Hz should attenuate 8000 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_bandpass_dependencies() {
        let bpf = BandPassFilterNode::new(10, 20, 30);
        let deps = bpf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // center
        assert_eq!(deps[2], 30); // q
    }

    #[test]
    fn test_bandpass_filter_state_updates() {
        // Test that filter coefficients update when parameters change
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Start with center at 1000 Hz
        let mut const_center = ConstantNode::new(1000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut bpf = BandPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // First pass
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

        let bpf_inputs = vec![signal.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        bpf.process_block(&bpf_inputs, &mut output1, 44100.0, &context);

        let rms1 = calculate_rms(&output1);

        // Change center to 2000 Hz
        const_center.set_value(2000.0);
        const_center.process_block(&[], &mut center_buf, 44100.0, &context);

        let mut output2 = vec![0.0; 512];
        let bpf_inputs2 = vec![signal.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        bpf.process_block(&bpf_inputs2, &mut output2, 44100.0, &context);

        let rms2 = calculate_rms(&output2);

        // With center at 2000 Hz, 1000 Hz signal should be more attenuated than at 1000 Hz center
        assert!(
            rms2 < rms1,
            "Center at 2000 Hz should attenuate 1000 Hz more than center at 1000 Hz: center=1000Hz RMS={}, center=2000Hz RMS={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_bandpass_reset() {
        let mut bpf = BandPassFilterNode::new(0, 1, 2);

        // Change internal state
        bpf.last_center = 5000.0;
        bpf.last_q = 10.0;

        // Reset should reinitialize filter
        bpf.reset();

        // Filter should still work after reset
        let signal_buffer = vec![0.5; 512];
        let center_buf = vec![1000.0; 512];
        let q_buf = vec![1.0; 512];
        let mut output = vec![0.0; 512];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let inputs = vec![signal_buffer.as_slice(), center_buf.as_slice(), q_buf.as_slice()];
        bpf.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        assert!(output.iter().all(|&x| x.is_finite()));
    }
}
