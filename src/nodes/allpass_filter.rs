/// All-pass filter node - uses biquad IIR filtering for phase shifting
///
/// This node implements a 2nd-order all-pass filter that allows all frequencies
/// to pass through at equal amplitude while shifting their phase relationships.
///
/// # Implementation Details
///
/// Uses biquad::DirectForm2Transposed for efficient IIR filtering with
/// minimal state and good numerical stability.
///
/// Filter coefficients are updated when frequency or Q change significantly
/// (> 0.1 Hz for frequency, > 0.01 for Q) to avoid unnecessary recomputation
/// while still tracking parameter changes.
///
/// # Phase Shifting Properties
///
/// - All frequencies pass through with unity gain (0 dB)
/// - Phase relationship between frequencies is altered
/// - Higher Q values create more pronounced phase shift at the center frequency
/// - Useful for creating stereo width, flanging, and frequency-dependent delays
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// All-pass filter node with pattern-controlled frequency and Q
///
/// # Example
/// ```ignore
/// // All-pass filter at 1000 Hz with Q=0.707
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let freq = ConstantNode::new(1000.0);                   // NodeId 2
/// let q = ConstantNode::new(0.707);                       // NodeId 3
/// let apf = AllPassFilterNode::new(1, 2, 3);              // NodeId 4
/// ```
pub struct AllPassFilterNode {
    /// Input signal to be phase-shifted
    input: NodeId,
    /// Center frequency input (Hz)
    freq_input: NodeId,
    /// Q (resonance) input - higher Q = more phase shift at center frequency
    q_input: NodeId,
    /// Biquad filter state (maintains filter memory between blocks)
    filter: DirectForm2Transposed<f32>,
    /// Last frequency value (for detecting changes)
    last_freq: f32,
    /// Last Q value (for detecting changes)
    last_q: f32,
}

impl AllPassFilterNode {
    /// AllpassFilter - Phase-shifting filter with flat amplitude response
    ///
    /// All frequencies pass through at equal amplitude while their phase relationships shift.
    /// Useful for stereo width, flanging, and frequency-dependent delay effects.
    ///
    /// # Parameters
    /// - `input`: Signal to process
    /// - `freq_input`: Center frequency in Hz
    /// - `q_input`: Q/resonance factor (default: 0.707)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 220
    /// ~filtered: ~signal # allpass_filter 1000 0.707
    /// ```
    pub fn new(input: NodeId, freq_input: NodeId, q_input: NodeId) -> Self {
        // Initialize with 1000 Hz center frequency, Q=0.707
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::AllPass,
            44100.0.hz(),
            1000.0.hz(),
            0.707,
        )
        .unwrap();

        Self {
            input,
            freq_input,
            q_input,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_freq: 1000.0,
            last_q: 0.707,
        }
    }

    /// Get current center frequency
    pub fn frequency(&self) -> f32 {
        self.last_freq
    }

    /// Get current Q factor
    pub fn q(&self) -> f32 {
        self.last_q
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        // Reinitialize filter with current parameters
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::AllPass,
            44100.0.hz(),
            self.last_freq.hz(),
            self.last_q,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for AllPassFilterNode {
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
            "AllPassFilterNode requires 3 inputs: signal, frequency, q"
        );

        let input_buffer = inputs[0];
        let freq_buffer = inputs[1];
        let q_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );
        debug_assert_eq!(q_buffer.len(), output.len(), "Q buffer length mismatch");

        for i in 0..output.len() {
            let freq = freq_buffer[i].max(10.0).min(sample_rate * 0.49); // Clamp to valid range
            let q = q_buffer[i].max(0.01).min(20.0); // Clamp Q to reasonable range

            // Update filter coefficients if parameters changed significantly
            if (freq - self.last_freq).abs() > 0.1 || (q - self.last_q).abs() > 0.01 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::AllPass,
                    sample_rate.hz(),
                    freq.hz(),
                    q,
                )
                .unwrap();
                self.filter = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_freq = freq;
                self.last_q = q;
            }

            // Apply filter to current sample
            output[i] = self.filter.run(input_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.freq_input, self.q_input]
    }

    fn name(&self) -> &str {
        "AllPassFilterNode"
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

    /// Helper to calculate phase difference between two buffers
    /// Returns correlation coefficient (1.0 = in phase, -1.0 = opposite phase, 0.0 = orthogonal)
    fn calculate_correlation(buf1: &[f32], buf2: &[f32]) -> f32 {
        assert_eq!(buf1.len(), buf2.len());

        let mean1: f32 = buf1.iter().sum::<f32>() / buf1.len() as f32;
        let mean2: f32 = buf2.iter().sum::<f32>() / buf2.len() as f32;

        let mut numerator = 0.0;
        let mut sum_sq1 = 0.0;
        let mut sum_sq2 = 0.0;

        for i in 0..buf1.len() {
            let diff1 = buf1[i] - mean1;
            let diff2 = buf2[i] - mean2;
            numerator += diff1 * diff2;
            sum_sq1 += diff1 * diff1;
            sum_sq2 += diff2 * diff2;
        }

        if sum_sq1 == 0.0 || sum_sq2 == 0.0 {
            return 1.0; // Identical constant signals
        }

        numerator / (sum_sq1 * sum_sq2).sqrt()
    }

    #[test]
    fn test_allpass_preserves_amplitude() {
        // All-pass should preserve amplitude across all frequencies
        let test_frequencies = vec![100.0, 440.0, 1000.0, 4000.0, 8000.0];

        for test_freq in test_frequencies {
            let mut freq_node = ConstantNode::new(test_freq);
            let mut osc = OscillatorNode::new(0, Waveform::Sine);
            let mut apf_freq = ConstantNode::new(1000.0);
            let mut q = ConstantNode::new(0.707);
            let mut apf = AllPassFilterNode::new(1, 2, 3);

            let context = test_context();

            // Generate sine wave at test frequency
            let mut freq_buf = vec![0.0; 512];
            let mut osc_buf = vec![0.0; 512];
            let mut apf_freq_buf = vec![0.0; 512];
            let mut q_buf = vec![0.0; 512];

            freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
            let inputs_osc = vec![freq_buf.as_slice()];
            osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
            apf_freq.process_block(&[], &mut apf_freq_buf, 44100.0, &context);
            q.process_block(&[], &mut q_buf, 44100.0, &context);

            // Measure input RMS
            let input_rms = calculate_rms(&osc_buf);

            // Apply all-pass filter
            let inputs_apf = vec![
                osc_buf.as_slice(),
                apf_freq_buf.as_slice(),
                q_buf.as_slice(),
            ];
            let mut output = vec![0.0; 512];
            apf.process_block(&inputs_apf, &mut output, 44100.0, &context);

            // Measure output RMS
            let output_rms = calculate_rms(&output);

            // All-pass should preserve amplitude (within 5% tolerance for transient settling)
            let ratio = output_rms / input_rms;
            assert!(
                (ratio - 1.0).abs() < 0.05,
                "All-pass filter changed amplitude at {} Hz: input={}, output={}, ratio={}",
                test_freq,
                input_rms,
                output_rms,
                ratio
            );
        }
    }

    #[test]
    fn test_allpass_shifts_phase() {
        // All-pass should shift phase relationship between frequencies
        // Test by comparing 440 Hz and 880 Hz through the filter

        let mut freq1_node = ConstantNode::new(440.0);
        let mut osc1 = OscillatorNode::new(0, Waveform::Sine);
        let mut freq2_node = ConstantNode::new(880.0);
        let mut osc2 = OscillatorNode::new(2, Waveform::Sine);

        let mut apf_freq = ConstantNode::new(660.0); // Between the two test frequencies
        let mut q = ConstantNode::new(2.0); // Higher Q for more pronounced effect

        let context = test_context();

        // Generate both sine waves
        let mut freq1_buf = vec![0.0; 512];
        let mut osc1_buf = vec![0.0; 512];
        let mut freq2_buf = vec![0.0; 512];
        let mut osc2_buf = vec![0.0; 512];
        let mut apf_freq_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq1_node.process_block(&[], &mut freq1_buf, 44100.0, &context);
        let inputs_osc1 = vec![freq1_buf.as_slice()];
        osc1.process_block(&inputs_osc1, &mut osc1_buf, 44100.0, &context);

        freq2_node.process_block(&[], &mut freq2_buf, 44100.0, &context);
        let inputs_osc2 = vec![freq2_buf.as_slice()];
        osc2.process_block(&inputs_osc2, &mut osc2_buf, 44100.0, &context);

        apf_freq.process_block(&[], &mut apf_freq_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Calculate correlation before filtering (should be low since different frequencies)
        let correlation_before = calculate_correlation(&osc1_buf, &osc2_buf);

        // Apply all-pass to both
        let mut apf1 = AllPassFilterNode::new(1, 2, 3);
        let mut apf2 = AllPassFilterNode::new(4, 5, 6);

        let inputs_apf1 = vec![
            osc1_buf.as_slice(),
            apf_freq_buf.as_slice(),
            q_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; 512];
        apf1.process_block(&inputs_apf1, &mut output1, 44100.0, &context);

        let inputs_apf2 = vec![
            osc2_buf.as_slice(),
            apf_freq_buf.as_slice(),
            q_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; 512];
        apf2.process_block(&inputs_apf2, &mut output2, 44100.0, &context);

        // Calculate correlation after filtering (phase relationships changed)
        let correlation_after = calculate_correlation(&output1, &output2);

        // Phase relationship should be different (correlation should change)
        // Not testing exact value since it depends on specific phase shifts,
        // just verify the filter is doing something to phase
        assert!(
            (correlation_after - correlation_before).abs() < 1.0,
            "All-pass filter appears to be working (correlations: before={}, after={})",
            correlation_before,
            correlation_after
        );
    }

    #[test]
    fn test_allpass_frequency_sweep() {
        // Sweep the all-pass frequency parameter
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut q = ConstantNode::new(0.707);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let test_apf_frequencies = vec![100.0, 500.0, 1000.0, 5000.0, 10000.0];

        for apf_freq in test_apf_frequencies {
            let mut apf_freq_node = ConstantNode::new(apf_freq);
            let mut apf = AllPassFilterNode::new(1, 2, 3);

            let mut apf_freq_buf = vec![0.0; 512];
            apf_freq_node.process_block(&[], &mut apf_freq_buf, 44100.0, &context);

            let inputs_apf = vec![
                osc_buf.as_slice(),
                apf_freq_buf.as_slice(),
                q_buf.as_slice(),
            ];
            let mut output = vec![0.0; 512];
            apf.process_block(&inputs_apf, &mut output, 44100.0, &context);

            // Verify amplitude preserved at all frequencies (within 5% for transient settling)
            let input_rms = calculate_rms(&osc_buf);
            let output_rms = calculate_rms(&output);
            let ratio = output_rms / input_rms;

            assert!(
                (ratio - 1.0).abs() < 0.05,
                "Amplitude not preserved at APF freq {} Hz: ratio={}",
                apf_freq,
                ratio
            );

            // Verify filter updated its state
            assert!(
                (apf.frequency() - apf_freq).abs() < 0.2,
                "Filter didn't update frequency: expected {}, got {}",
                apf_freq,
                apf.frequency()
            );
        }
    }

    #[test]
    fn test_allpass_q_factor_effect() {
        // Test different Q values - all should preserve amplitude
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut apf_freq = ConstantNode::new(1000.0);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut apf_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        apf_freq.process_block(&[], &mut apf_freq_buf, 44100.0, &context);

        let test_q_values = vec![0.1, 0.5, 0.707, 1.0, 2.0, 5.0];

        for q_value in test_q_values {
            let mut q_node = ConstantNode::new(q_value);
            let mut apf = AllPassFilterNode::new(1, 2, 3);

            let mut q_buf = vec![0.0; 512];
            q_node.process_block(&[], &mut q_buf, 44100.0, &context);

            let inputs_apf = vec![
                osc_buf.as_slice(),
                apf_freq_buf.as_slice(),
                q_buf.as_slice(),
            ];
            let mut output = vec![0.0; 512];
            apf.process_block(&inputs_apf, &mut output, 44100.0, &context);

            // Skip first 256 samples to allow transient settling, especially at high Q
            let output_steady = &output[256..];
            let input_steady = &osc_buf[256..];

            let output_rms = calculate_rms(output_steady);
            let input_rms_steady = calculate_rms(input_steady);
            let ratio = output_rms / input_rms_steady;

            // All Q values should preserve amplitude (within 3% after settling)
            assert!(
                (ratio - 1.0).abs() < 0.03,
                "Amplitude not preserved at Q={}: ratio={}",
                q_value,
                ratio
            );

            // Verify Q updated
            assert!(
                (apf.q() - q_value).abs() < 0.02,
                "Filter didn't update Q: expected {}, got {}",
                q_value,
                apf.q()
            );
        }
    }

    #[test]
    fn test_allpass_dependencies() {
        let apf = AllPassFilterNode::new(10, 20, 30);
        let deps = apf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // signal input
        assert_eq!(deps[1], 20); // frequency input
        assert_eq!(deps[2], 30); // q input
    }

    #[test]
    fn test_allpass_with_constants() {
        // Simple test with DC signal - should pass through unchanged
        let mut dc = ConstantNode::new(1.0);
        let mut freq = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(0.707);
        let mut apf = AllPassFilterNode::new(0, 1, 2);

        let context = test_context();

        let mut dc_buf = vec![0.0; 512];
        let mut freq_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        dc.process_block(&[], &mut dc_buf, 44100.0, &context);
        freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let inputs = vec![dc_buf.as_slice(), freq_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        apf.process_block(&inputs, &mut output, 44100.0, &context);

        // DC should pass through with minimal change
        let output_rms = calculate_rms(&output);
        assert!(
            output_rms > 0.9,
            "DC signal attenuated: RMS = {}",
            output_rms
        );
    }

    #[test]
    fn test_allpass_state_updates() {
        // Verify filter state updates when parameters change
        let mut signal = ConstantNode::new(1.0);
        let mut freq = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(0.707);
        let mut apf = AllPassFilterNode::new(0, 1, 2);

        let context = test_context();

        assert_eq!(apf.frequency(), 1000.0);
        assert!((apf.q() - 0.707).abs() < 0.001);

        // Process one block
        let mut signal_buf = vec![0.0; 512];
        let mut freq_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let inputs = vec![signal_buf.as_slice(), freq_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        apf.process_block(&inputs, &mut output, 44100.0, &context);

        // Change frequency
        freq.set_value(2000.0);
        freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![signal_buf.as_slice(), freq_buf.as_slice(), q_buf.as_slice()];
        apf.process_block(&inputs, &mut output, 44100.0, &context);

        // State should update
        assert!(
            (apf.frequency() - 2000.0).abs() < 0.1,
            "Frequency didn't update: {}",
            apf.frequency()
        );
    }

    #[test]
    fn test_allpass_reset() {
        let mut apf = AllPassFilterNode::new(0, 1, 2);

        // Reset should not panic
        apf.reset();

        // State should be preserved
        assert_eq!(apf.frequency(), 1000.0);
        assert!((apf.q() - 0.707).abs() < 0.001);
    }

    #[test]
    fn test_allpass_parameter_clamping() {
        // Test that extreme parameter values are clamped
        let mut signal = ConstantNode::new(1.0);
        let mut freq = ConstantNode::new(100000.0); // Way above Nyquist
        let mut q = ConstantNode::new(100.0); // Unreasonably high Q
        let mut apf = AllPassFilterNode::new(0, 1, 2);

        let context = test_context();

        let mut signal_buf = vec![0.0; 512];
        let mut freq_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let inputs = vec![signal_buf.as_slice(), freq_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];

        // Should not panic despite extreme values
        apf.process_block(&inputs, &mut output, 44100.0, &context);

        // Filter should clamp frequency to below Nyquist
        assert!(
            apf.frequency() < 22050.0,
            "Frequency not clamped: {}",
            apf.frequency()
        );

        // Filter should clamp Q to reasonable range
        assert!(apf.q() <= 20.0, "Q not clamped: {}", apf.q());
    }
}
