/// Low-pass filter node - uses biquad IIR filtering
///
/// This node implements a 2nd-order Butterworth low-pass filter with
/// pattern-controllable cutoff frequency and resonance (Q).
///
/// # Implementation Details
///
/// Uses biquad::DirectForm2Transposed for efficient IIR filtering with
/// minimal state and good numerical stability.
///
/// Filter coefficients are updated when cutoff or Q change significantly
/// (> 0.1 Hz for cutoff, > 0.01 for Q) to avoid unnecessary recomputation
/// while still tracking parameter changes.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz, Q_BUTTERWORTH_F32};

/// Low-pass filter node with pattern-controlled cutoff and Q
///
/// # Example
/// ```ignore
/// // Filter signal through 1000 Hz lowpass with Q=0.707
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let cutoff = ConstantNode::new(1000.0);                 // NodeId 2
/// let q = ConstantNode::new(Q_BUTTERWORTH_F32);           // NodeId 3
/// let lpf = LowPassFilterNode::new(1, 2, 3);              // NodeId 4
/// ```
pub struct LowPassFilterNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Cutoff frequency input (Hz)
    cutoff_input: NodeId,
    /// Q (resonance) input - higher Q = more resonance at cutoff
    q_input: NodeId,
    /// Biquad filter state (maintains filter memory between blocks)
    filter: DirectForm2Transposed<f32>,
    /// Last cutoff value (for detecting changes)
    last_cutoff: f32,
    /// Last Q value (for detecting changes)
    last_q: f32,
}

impl LowPassFilterNode {
    /// LowPass Filter - Resonant lowpass filter with pattern control
    ///
    /// Attenuates frequencies above cutoff while passing lower frequencies.
    /// The Q parameter controls resonance peak at cutoff frequency.
    ///
    /// # Parameters
    /// - `input`: Audio signal to filter
    /// - `cutoff_input`: Cutoff frequency in Hz (20-20000)
    /// - `q_input`: Resonance factor (0.1-20.0, 0.707 for Butterworth)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~cutoff: sine 0.5 * 4000 + 2000
    /// ~filtered: ~signal # lpf ~cutoff 0.8
    /// out: ~filtered
    /// ```
    pub fn new(input: NodeId, cutoff_input: NodeId, q_input: NodeId) -> Self {
        // Initialize with 1000 Hz cutoff, Butterworth Q
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            1000.0.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        Self {
            input,
            cutoff_input,
            q_input,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_cutoff: 1000.0,
            last_q: Q_BUTTERWORTH_F32,
        }
    }

    /// Get current cutoff frequency
    pub fn cutoff(&self) -> f32 {
        self.last_cutoff
    }

    /// Get current Q factor
    pub fn q(&self) -> f32 {
        self.last_q
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        // Reinitialize filter with current parameters
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            self.last_cutoff.hz(),
            self.last_q,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for LowPassFilterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            3,
            "LowPassFilterNode requires 3 inputs: signal, cutoff, q"
        );

        let input_buffer = inputs[0];
        let cutoff_buffer = inputs[1];
        let q_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
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
            let cutoff = cutoff_buffer[i].max(10.0).min(sample_rate * 0.49); // Clamp to valid range
            let q = q_buffer[i].max(0.01).min(20.0); // Clamp Q to reasonable range

            // Update filter coefficients if parameters changed significantly
            if (cutoff - self.last_cutoff).abs() > 0.1 || (q - self.last_q).abs() > 0.01 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::LowPass,
                    sample_rate.hz(),
                    cutoff.hz(),
                    q,
                )
                .unwrap();
                self.filter = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_cutoff = cutoff;
                self.last_q = q;
            }

            // Apply filter to current sample
            output[i] = self.filter.run(input_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.cutoff_input, self.q_input]
    }

    fn name(&self) -> &str {
        "LowPassFilterNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{ConstantNode, OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    /// Helper to calculate RMS (root mean square) of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Helper to create a test context
    fn test_context() -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0)
    }

    #[test]
    fn test_lowpass_dc_blocking() {
        // DC (0 Hz) should pass through unchanged
        let mut dc_input = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(Q_BUTTERWORTH_F32);
        let mut lpf = LowPassFilterNode::new(0, 1, 2);

        let context = test_context();

        // Generate input buffers
        let mut dc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        dc_input.process_block(&[], &mut dc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Apply filter
        let inputs = vec![dc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        lpf.process_block(&inputs, &mut output, 44100.0, &context);

        // DC should pass through with minimal attenuation
        let output_rms = calculate_rms(&output);
        assert!(
            output_rms > 0.9,
            "DC signal attenuated too much: RMS = {}",
            output_rms
        );
    }

    #[test]
    fn test_lowpass_high_freq_attenuation() {
        // High frequency (8000 Hz) should be attenuated by 1000 Hz lowpass
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(Q_BUTTERWORTH_F32);
        let mut lpf = LowPassFilterNode::new(1, 2, 3);

        let context = test_context();

        // Generate 8000 Hz sine wave
        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Measure input RMS
        let input_rms = calculate_rms(&osc_buf);

        // Apply lowpass filter
        let inputs_lpf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        lpf.process_block(&inputs_lpf, &mut output, 44100.0, &context);

        // Measure output RMS
        let output_rms = calculate_rms(&output);

        // 8000 Hz should be heavily attenuated by 1000 Hz lowpass
        // At 8x cutoff, expect ~-48 dB attenuation (12 dB/octave * 3 octaves)
        // That's roughly 1/250 amplitude ratio
        assert!(
            output_rms < input_rms * 0.1,
            "High frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_lowpass_passband() {
        // 440 Hz should pass through 1000 Hz lowpass relatively unchanged
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(Q_BUTTERWORTH_F32);
        let mut lpf = LowPassFilterNode::new(1, 2, 3);

        let context = test_context();

        // Generate 440 Hz sine wave
        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Measure input RMS
        let input_rms = calculate_rms(&osc_buf);

        // Apply lowpass filter
        let inputs_lpf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        lpf.process_block(&inputs_lpf, &mut output, 44100.0, &context);

        // Measure output RMS
        let output_rms = calculate_rms(&output);

        // 440 Hz should pass through 1000 Hz lowpass with minimal attenuation
        // Butterworth is -3dB at cutoff, so well below cutoff should be nearly flat
        assert!(
            output_rms > input_rms * 0.9,
            "Passband signal attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_lowpass_dependencies() {
        let lpf = LowPassFilterNode::new(10, 20, 30);
        let deps = lpf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // signal input
        assert_eq!(deps[1], 20); // cutoff input
        assert_eq!(deps[2], 30); // q input
    }

    #[test]
    fn test_lowpass_state_updates() {
        // Verify filter state updates when parameters change
        let mut signal = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(Q_BUTTERWORTH_F32);
        let mut lpf = LowPassFilterNode::new(0, 1, 2);

        let context = test_context();

        assert_eq!(lpf.cutoff(), 1000.0);
        assert_eq!(lpf.q(), Q_BUTTERWORTH_F32);

        // Process one block
        let mut signal_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            q_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        lpf.process_block(&inputs, &mut output, 44100.0, &context);

        // Change cutoff
        cutoff.set_value(2000.0);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            q_buf.as_slice(),
        ];
        lpf.process_block(&inputs, &mut output, 44100.0, &context);

        // State should update
        assert!(
            (lpf.cutoff() - 2000.0).abs() < 0.1,
            "Cutoff didn't update: {}",
            lpf.cutoff()
        );
    }

    #[test]
    fn test_lowpass_reset() {
        let mut lpf = LowPassFilterNode::new(0, 1, 2);

        // Reset should not panic
        lpf.reset();

        // State should be preserved
        assert_eq!(lpf.cutoff(), 1000.0);
        assert_eq!(lpf.q(), Q_BUTTERWORTH_F32);
    }

    #[test]
    fn test_lowpass_parameter_clamping() {
        // Test that extreme parameter values are clamped
        let mut signal = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(100000.0); // Way above Nyquist
        let mut q = ConstantNode::new(100.0); // Unreasonably high Q
        let mut lpf = LowPassFilterNode::new(0, 1, 2);

        let context = test_context();

        let mut signal_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            q_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];

        // Should not panic despite extreme values
        lpf.process_block(&inputs, &mut output, 44100.0, &context);

        // Filter should clamp cutoff to below Nyquist
        assert!(
            lpf.cutoff() < 22050.0,
            "Cutoff not clamped: {}",
            lpf.cutoff()
        );

        // Filter should clamp Q to reasonable range
        assert!(lpf.q() <= 20.0, "Q not clamped: {}", lpf.q());
    }
}
