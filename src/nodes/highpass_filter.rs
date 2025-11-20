/// HighPass filter node using biquad filter
///
/// This node implements a resonant highpass filter using the `biquad` crate.
/// The filter attenuates frequencies below the cutoff and passes higher frequencies.
/// The Q parameter controls resonance at the cutoff frequency.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// HighPass filter node with pattern-controlled cutoff and Q
///
/// # Example
/// ```ignore
/// // 5000 Hz sine wave
/// let freq_const = ConstantNode::new(5000.0);  // NodeId 0
/// let osc = OscillatorNode::new(0, Waveform::Sine);  // NodeId 1
///
/// // Cutoff at 1000 Hz with Q=1.0
/// let cutoff_const = ConstantNode::new(1000.0);  // NodeId 2
/// let q_const = ConstantNode::new(1.0);  // NodeId 3
/// let hpf = HighPassFilterNode::new(1, 2, 3);  // NodeId 4
/// // High frequencies (5000 Hz) pass through, low frequencies blocked
/// ```
pub struct HighPassFilterNode {
    input: NodeId,
    cutoff_input: NodeId,
    q_input: NodeId,
    filter: DirectForm2Transposed<f32>,
    last_cutoff: f32,
    last_q: f32,
}

impl HighPassFilterNode {
    /// Create a new highpass filter node
    ///
    /// # Arguments
    /// * `input` - NodeId providing audio signal to filter
    /// * `cutoff_input` - NodeId providing cutoff frequency (Hz)
    /// * `q_input` - NodeId providing resonance (Q factor)
    pub fn new(input: NodeId, cutoff_input: NodeId, q_input: NodeId) -> Self {
        // Initialize with default coefficients (will be updated on first process)
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            1000.0.hz(),
            1.0,
        )
        .unwrap();

        Self {
            input,
            cutoff_input,
            q_input,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_cutoff: 1000.0,
            last_q: 1.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the cutoff frequency input node ID
    pub fn cutoff_input(&self) -> NodeId {
        self.cutoff_input
    }

    /// Get the Q input node ID
    pub fn q_input(&self) -> NodeId {
        self.q_input
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            self.last_cutoff.hz(),
            self.last_q,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for HighPassFilterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "HighPassFilterNode requires 3 inputs (signal, cutoff, q), got {}",
            inputs.len()
        );

        let signal_buffer = inputs[0];
        let cutoff_buffer = inputs[1];
        let q_buffer = inputs[2];

        debug_assert_eq!(
            signal_buffer.len(),
            output.len(),
            "Signal buffer length mismatch"
        );
        debug_assert_eq!(
            cutoff_buffer.len(),
            output.len(),
            "Cutoff buffer length mismatch"
        );
        debug_assert_eq!(
            q_buffer.len(),
            output.len(),
            "Q buffer length mismatch"
        );

        for i in 0..output.len() {
            let cutoff = cutoff_buffer[i].max(20.0).min(20000.0); // Clamp to valid range
            let q = q_buffer[i].max(0.1).min(20.0); // Clamp to valid range

            // Update coefficients if parameters changed
            if (cutoff - self.last_cutoff).abs() > 0.1 || (q - self.last_q).abs() > 0.01 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::HighPass,
                    sample_rate.hz(),
                    cutoff.hz(),
                    q,
                )
                .unwrap();

                self.filter.update_coefficients(coeffs);
                self.last_cutoff = cutoff;
                self.last_q = q;
            }

            // Process sample through filter
            output[i] = self.filter.run(signal_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.cutoff_input, self.q_input]
    }

    fn name(&self) -> &str {
        "HighPassFilterNode"
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
    fn test_highpass_blocks_low_frequencies() {
        // Low frequency oscillator (100 Hz)
        let mut const_freq = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Highpass filter at 4000 Hz (should attenuate 100 Hz)
        let mut const_cutoff = ConstantNode::new(4000.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut hpf = HighPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let hpf_inputs = vec![unfiltered.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        hpf.process_block(&hpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Highpass should significantly attenuate low frequencies
        assert!(
            filtered_rms < unfiltered_rms * 0.3,
            "Highpass at 4000 Hz should attenuate 100 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_highpass_passes_high_frequencies() {
        // High frequency oscillator (4000 Hz)
        let mut const_freq = ConstantNode::new(4000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Highpass filter at 100 Hz (should pass 4000 Hz)
        let mut const_cutoff = ConstantNode::new(100.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut hpf = HighPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let hpf_inputs = vec![unfiltered.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        hpf.process_block(&hpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Highpass should pass high frequencies (within 20%)
        assert!(
            filtered_rms > unfiltered_rms * 0.8,
            "Highpass at 100 Hz should pass 4000 Hz signal: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_highpass_blocks_dc() {
        // Constant DC signal (0 Hz)
        let dc_value = 0.5;
        let dc_buffer = vec![dc_value; 512];

        let mut const_cutoff = ConstantNode::new(100.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut hpf = HighPassFilterNode::new(0, 1, 2);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate cutoff and Q buffers
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Filter DC signal
        let hpf_inputs = vec![dc_buffer.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        hpf.process_block(&hpf_inputs, &mut filtered, 44100.0, &context);

        // DC should be heavily attenuated compared to input
        // Highpass filters take time to settle, so we check that it's significantly reduced
        let filtered_rms = calculate_rms(&filtered);
        assert!(
            filtered_rms < dc_value * 0.5,
            "Highpass should significantly reduce DC component: input={}, output RMS={}",
            dc_value,
            filtered_rms
        );
    }

    #[test]
    fn test_highpass_dependencies() {
        let hpf = HighPassFilterNode::new(10, 20, 30);
        let deps = hpf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // cutoff
        assert_eq!(deps[2], 30); // q
    }

    #[test]
    fn test_highpass_filter_state_updates() {
        // Test that filter coefficients update when parameters change
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Start with cutoff at 500 Hz
        let mut const_cutoff = ConstantNode::new(500.0);
        let mut const_q = ConstantNode::new(1.0);
        let mut hpf = HighPassFilterNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // First pass
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];
        let mut signal = vec![0.0; 512];
        let mut output1 = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_q.process_block(&[], &mut q_buf, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        let hpf_inputs = vec![signal.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        hpf.process_block(&hpf_inputs, &mut output1, 44100.0, &context);

        let rms1 = calculate_rms(&output1);

        // Change cutoff to 2000 Hz
        const_cutoff.set_value(2000.0);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let mut output2 = vec![0.0; 512];
        let hpf_inputs2 = vec![signal.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        hpf.process_block(&hpf_inputs2, &mut output2, 44100.0, &context);

        let rms2 = calculate_rms(&output2);

        // With higher cutoff, 1000 Hz signal should be more attenuated
        assert!(
            rms2 < rms1,
            "Higher cutoff (2000 Hz) should attenuate 1000 Hz more than lower cutoff (500 Hz): cutoff=500Hz RMS={}, cutoff=2000Hz RMS={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_highpass_reset() {
        let mut hpf = HighPassFilterNode::new(0, 1, 2);

        // Change internal state
        hpf.last_cutoff = 5000.0;
        hpf.last_q = 10.0;

        // Reset should reinitialize filter
        hpf.reset();

        // Filter should still work after reset
        let dc_buffer = vec![0.5; 512];
        let cutoff_buf = vec![100.0; 512];
        let q_buf = vec![1.0; 512];
        let mut output = vec![0.0; 512];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let inputs = vec![dc_buffer.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        hpf.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        assert!(output.iter().all(|&x| x.is_finite()));
    }
}
