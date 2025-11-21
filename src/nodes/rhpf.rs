/// Resonant HighPass Filter (RHPF) node using biquad filter
///
/// This node implements a resonant highpass filter using the `biquad` crate.
/// The filter attenuates frequencies below the cutoff and passes higher frequencies.
/// The resonance parameter controls the amount of boost at the cutoff frequency,
/// creating the classic "resonant filter" sound used in analog synthesizers.
///
/// # Implementation
///
/// Based on the Audio EQ Cookbook biquad design. The resonance parameter
/// directly maps to Q (quality factor), where higher values create sharper
/// resonance peaks at the cutoff frequency.
///
/// # References
///
/// - Robert Bristow-Johnson's Audio EQ Cookbook
/// - SuperCollider RHPF UGen
/// - Classic analog filter designs (Moog, Roland, etc.)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Resonant HighPass Filter node with pattern-controlled cutoff and resonance
///
/// # Example
/// ```ignore
/// // 55 Hz saw wave (bass)
/// let freq_const = ConstantNode::new(55.0);        // NodeId 0
/// let osc = OscillatorNode::new(0, Waveform::Saw); // NodeId 1
///
/// // High-pass at 500 Hz with resonance=5.0 (aggressive resonance)
/// let cutoff_const = ConstantNode::new(500.0);     // NodeId 2
/// let res_const = ConstantNode::new(5.0);          // NodeId 3
/// let rhpf = RHPFNode::new(1, 2, 3);               // NodeId 4
/// // Low frequencies (55 Hz fundamental) blocked, harmonics pass with resonant peak at 500 Hz
/// ```
///
/// # Parameters
///
/// - **cutoff**: Cutoff frequency in Hz (20 Hz to 20 kHz)
/// - **resonance**: Q factor (0.1 to 20.0)
///   - 0.707: Butterworth response (flat, no resonance)
///   - 1.0 to 3.0: Mild resonance
///   - 3.0 to 10.0: Strong resonance (classic analog sound)
///   - 10.0+: Extreme resonance (self-oscillation territory)
pub struct RHPFNode {
    input: NodeId,
    cutoff_input: NodeId,
    res_input: NodeId,
    filter: DirectForm2Transposed<f32>,
    last_cutoff: f32,
    last_res: f32,
}

impl RHPFNode {
    /// RHPFNode - Resonant highpass filter with q-factor control
    ///
    /// Implements a resonant highpass filter using biquad design, attenuating
    /// frequencies below the cutoff with adjustable resonance peaks.
    ///
    /// # Parameters
    /// - `input`: NodeId providing audio signal to filter
    /// - `cutoff_input`: NodeId providing cutoff frequency (Hz, 20-20000)
    /// - `res_input`: NodeId providing resonance/Q factor (0.1-20.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 55
    /// ~filtered: ~signal # rhpf 500 5.0
    /// ```
    pub fn new(input: NodeId, cutoff_input: NodeId, res_input: NodeId) -> Self {
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
            res_input,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_cutoff: 1000.0,
            last_res: 1.0,
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

    /// Get the resonance input node ID
    pub fn res_input(&self) -> NodeId {
        self.res_input
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            self.last_cutoff.hz(),
            self.last_res,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for RHPFNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "RHPFNode requires 3 inputs (signal, cutoff, resonance), got {}",
            inputs.len()
        );

        let signal_buffer = inputs[0];
        let cutoff_buffer = inputs[1];
        let res_buffer = inputs[2];

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
            res_buffer.len(),
            output.len(),
            "Resonance buffer length mismatch"
        );

        for i in 0..output.len() {
            let cutoff = cutoff_buffer[i].max(20.0).min(20000.0); // Clamp to valid range
            let res = res_buffer[i].max(0.1).min(20.0); // Clamp to valid range

            // Update coefficients if parameters changed
            if (cutoff - self.last_cutoff).abs() > 0.1 || (res - self.last_res).abs() > 0.01 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::HighPass,
                    sample_rate.hz(),
                    cutoff.hz(),
                    res,
                )
                .unwrap();

                self.filter.update_coefficients(coeffs);
                self.last_cutoff = cutoff;
                self.last_res = res;
            }

            // Process sample through filter
            output[i] = self.filter.run(signal_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.cutoff_input, self.res_input]
    }

    fn name(&self) -> &str {
        "RHPFNode"
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

    /// Helper: Create test context
    fn test_context() -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0)
    }

    #[test]
    fn test_rhpf_blocks_low_frequencies() {
        // Low frequency oscillator (100 Hz)
        let mut const_freq = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Resonant highpass at 2000 Hz (should heavily attenuate 100 Hz)
        let mut const_cutoff = ConstantNode::new(2000.0);
        let mut const_res = ConstantNode::new(2.0);
        let mut rhpf = RHPFNode::new(1, 2, 3);

        let context = test_context();

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let rhpf_inputs = vec![
            unfiltered.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        rhpf.process_block(&rhpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // RHPF should heavily attenuate low frequencies
        assert!(
            filtered_rms < unfiltered_rms * 0.1,
            "RHPF at 2000 Hz should heavily attenuate 100 Hz: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_rhpf_passes_high_frequencies() {
        // High frequency oscillator (8000 Hz)
        let mut const_freq = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Resonant highpass at 1000 Hz (should pass 8000 Hz)
        let mut const_cutoff = ConstantNode::new(1000.0);
        let mut const_res = ConstantNode::new(2.0);
        let mut rhpf = RHPFNode::new(1, 2, 3);

        let context = test_context();

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];
        let mut unfiltered = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, 44100.0, &context);

        // Get filtered signal
        let rhpf_inputs = vec![
            unfiltered.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        rhpf.process_block(&rhpf_inputs, &mut filtered, 44100.0, &context);

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // RHPF should pass high frequencies
        assert!(
            filtered_rms > unfiltered_rms * 0.7,
            "RHPF at 1000 Hz should pass 8000 Hz: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_rhpf_resonance_boosts_at_cutoff() {
        // Test that higher resonance creates boost at cutoff frequency
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut const_cutoff = ConstantNode::new(1000.0);

        // Low resonance
        let mut const_res_low = ConstantNode::new(0.707);
        let mut rhpf_low = RHPFNode::new(1, 2, 3);

        // High resonance
        let mut const_res_high = ConstantNode::new(8.0);
        let mut rhpf_high = RHPFNode::new(1, 2, 3);

        let context = test_context();

        // Generate buffers
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_low_buf = vec![0.0; 512];
        let mut res_high_buf = vec![0.0; 512];
        let mut signal = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_res_low.process_block(&[], &mut res_low_buf, 44100.0, &context);
        const_res_high.process_block(&[], &mut res_high_buf, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        let input_rms = calculate_rms(&signal);

        // Process with low resonance
        let rhpf_inputs_low = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_low_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; 512];
        rhpf_low.process_block(&rhpf_inputs_low, &mut output_low, 44100.0, &context);
        let rms_low = calculate_rms(&output_low);

        // Process with high resonance
        let rhpf_inputs_high = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_high_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; 512];
        rhpf_high.process_block(&rhpf_inputs_high, &mut output_high, 44100.0, &context);
        let rms_high = calculate_rms(&output_high);

        // High resonance should boost signal at cutoff frequency
        assert!(
            rms_high > rms_low,
            "High resonance should boost at cutoff: low_res RMS={}, high_res RMS={}, input={}",
            rms_low,
            rms_high,
            input_rms
        );
    }

    #[test]
    fn test_rhpf_blocks_dc() {
        // DC signal (0 Hz) should be completely blocked
        let dc_value = 0.5;
        let dc_buffer = vec![dc_value; 512];

        let mut const_cutoff = ConstantNode::new(100.0);
        let mut const_res = ConstantNode::new(2.0);
        let mut rhpf = RHPFNode::new(0, 1, 2);

        let context = test_context();

        // Generate cutoff and resonance buffers
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];
        let mut filtered = vec![0.0; 512];

        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);

        // Filter DC signal
        let rhpf_inputs = vec![dc_buffer.as_slice(), cutoff_buf.as_slice(), res_buf.as_slice()];
        rhpf.process_block(&rhpf_inputs, &mut filtered, 44100.0, &context);

        // DC should be heavily attenuated (but with settling time)
        let filtered_rms = calculate_rms(&filtered);
        assert!(
            filtered_rms < dc_value * 0.5,
            "RHPF should attenuate DC: input={}, output RMS={}",
            dc_value,
            filtered_rms
        );
    }

    #[test]
    fn test_rhpf_cutoff_modulation() {
        // Test that cutoff can be modulated
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Start with cutoff at 500 Hz
        let mut const_cutoff = ConstantNode::new(500.0);
        let mut const_res = ConstantNode::new(2.0);
        let mut rhpf = RHPFNode::new(1, 2, 3);

        let context = test_context();

        // Generate signal
        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];
        let mut signal = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        // Process with cutoff at 500 Hz (1000 Hz should pass well)
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        let rhpf_inputs1 = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; 512];
        rhpf.process_block(&rhpf_inputs1, &mut output1, 44100.0, &context);
        let rms1 = calculate_rms(&output1);

        // Change cutoff to 2000 Hz (1000 Hz should be more attenuated)
        const_cutoff.set_value(2000.0);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        let rhpf_inputs2 = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; 512];
        rhpf.process_block(&rhpf_inputs2, &mut output2, 44100.0, &context);
        let rms2 = calculate_rms(&output2);

        // Higher cutoff should attenuate 1000 Hz more
        assert!(
            rms2 < rms1,
            "Higher cutoff (2000 Hz) should attenuate 1000 Hz more than lower cutoff (500 Hz): cutoff=500Hz RMS={}, cutoff=2000Hz RMS={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_rhpf_resonance_modulation() {
        // Test that resonance can be modulated
        let mut const_freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut const_cutoff = ConstantNode::new(1000.0);

        // Start with low resonance
        let mut const_res = ConstantNode::new(0.707);
        let mut rhpf = RHPFNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];
        let mut signal = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        // Process with low resonance
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);
        let rhpf_inputs1 = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; 512];
        rhpf.process_block(&rhpf_inputs1, &mut output1, 44100.0, &context);
        let rms1 = calculate_rms(&output1);

        // Change to high resonance
        const_res.set_value(10.0);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);
        let rhpf_inputs2 = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; 512];
        rhpf.process_block(&rhpf_inputs2, &mut output2, 44100.0, &context);
        let rms2 = calculate_rms(&output2);

        // Higher resonance should boost at cutoff
        assert!(
            rms2 > rms1,
            "Higher resonance should boost signal: res=0.707 RMS={}, res=10.0 RMS={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_rhpf_dependencies() {
        let rhpf = RHPFNode::new(10, 20, 30);
        let deps = rhpf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // cutoff
        assert_eq!(deps[2], 30); // resonance
    }

    #[test]
    fn test_rhpf_reset() {
        let mut rhpf = RHPFNode::new(0, 1, 2);

        // Change internal state
        rhpf.last_cutoff = 5000.0;
        rhpf.last_res = 10.0;

        // Reset should reinitialize filter
        rhpf.reset();

        // Filter should still work after reset
        let dc_buffer = vec![0.5; 512];
        let cutoff_buf = vec![100.0; 512];
        let res_buf = vec![2.0; 512];
        let mut output = vec![0.0; 512];

        let context = test_context();

        let inputs = vec![dc_buffer.as_slice(), cutoff_buf.as_slice(), res_buf.as_slice()];
        rhpf.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_rhpf_stability_extreme_parameters() {
        // Test stability with extreme parameter values
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Extreme resonance near self-oscillation
        let mut const_cutoff = ConstantNode::new(10000.0);
        let mut const_res = ConstantNode::new(19.9);
        let mut rhpf = RHPFNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];
        let mut signal = vec![0.0; 512];
        let mut output = vec![0.0; 512];

        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);
        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

        let rhpf_inputs = vec![
            signal.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        rhpf.process_block(&rhpf_inputs, &mut output, 44100.0, &context);

        // Should produce finite output even with extreme resonance
        assert!(
            output.iter().all(|&x| x.is_finite()),
            "RHPF should remain stable with extreme resonance"
        );
    }

    #[test]
    fn test_rhpf_parameter_clamping() {
        // Test that parameters are clamped to safe ranges
        let signal_buf = vec![1.0; 512];
        let cutoff_buf = vec![100000.0; 512]; // Way above Nyquist
        let res_buf = vec![100.0; 512]; // Extremely high resonance

        let mut rhpf = RHPFNode::new(0, 1, 2);
        let context = test_context();

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];

        // Should not panic or produce invalid output
        rhpf.process_block(&inputs, &mut output, 44100.0, &context);

        assert!(
            output.iter().all(|&x| x.is_finite()),
            "RHPF should clamp parameters and produce finite output"
        );
    }

    #[test]
    fn test_rhpf_name() {
        let rhpf = RHPFNode::new(0, 1, 2);
        assert_eq!(rhpf.name(), "RHPFNode");
    }

    #[test]
    fn test_rhpf_frequency_response_sweep() {
        // Test frequency response at different points in spectrum
        let mut const_cutoff = ConstantNode::new(1000.0);
        let mut const_res = ConstantNode::new(3.0);
        let mut rhpf = RHPFNode::new(0, 1, 2);

        let context = test_context();

        let mut cutoff_buf = vec![0.0; 512];
        let mut res_buf = vec![0.0; 512];

        const_cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        const_res.process_block(&[], &mut res_buf, 44100.0, &context);

        // Test different frequencies
        let test_freqs = vec![50.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0];
        let mut rms_values = Vec::new();

        for &freq in &test_freqs {
            let mut freq_node = ConstantNode::new(freq);
            let mut osc = OscillatorNode::new(0, Waveform::Sine);

            let mut freq_buf = vec![0.0; 512];
            let mut signal = vec![0.0; 512];
            let mut output = vec![0.0; 512];

            freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
            let osc_inputs = vec![freq_buf.as_slice()];
            osc.process_block(&osc_inputs, &mut signal, 44100.0, &context);

            let rhpf_inputs = vec![
                signal.as_slice(),
                cutoff_buf.as_slice(),
                res_buf.as_slice(),
            ];
            rhpf.process_block(&rhpf_inputs, &mut output, 44100.0, &context);

            let rms = calculate_rms(&output);
            rms_values.push(rms);
        }

        // Verify frequency response characteristics:
        // - Low frequencies (50, 200 Hz) should be heavily attenuated
        // - High frequencies (5000, 10000 Hz) should pass well
        assert!(
            rms_values[0] < rms_values[6] * 0.1,
            "50 Hz should be much more attenuated than 10 kHz"
        );
        assert!(
            rms_values[1] < rms_values[5] * 0.2,
            "200 Hz should be much more attenuated than 5 kHz"
        );

        // Verify high frequencies pass better than low frequencies
        // (Some ripple is ok, just verify overall trend)
        let high_freq_avg = (rms_values[5] + rms_values[6]) / 2.0;
        let low_freq_avg = (rms_values[0] + rms_values[1]) / 2.0;
        assert!(
            high_freq_avg > low_freq_avg * 5.0,
            "High frequencies should pass much better than low frequencies"
        );
    }
}
