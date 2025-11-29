/// State Variable Filter (SVF) node - multi-mode resonant filter
///
/// This node implements a state-variable filter topology that can produce
/// lowpass, highpass, bandpass, and notch filter responses from a single
/// structure. The SVF uses a topology-preserving transform for excellent
/// stability and smooth parameter modulation.
///
/// # Implementation Details
///
/// Based on:
/// - Hal Chamberlin "Musical Applications of Microprocessors" (1985)
/// - Andrew Simper's topology-preserving transform
/// - Widely used in Eurorack modules (Mutable Instruments, etc.)
///
/// The SVF maintains two integrator states (ic1eq, ic2eq) and computes
/// intermediate values (v1, v2, v3) that are combined differently for
/// each filter mode:
/// - LowPass: v2 (output of second integrator)
/// - BandPass: v1 (output of first integrator)
/// - HighPass: input - k*v1 - v2
/// - Notch: input - k*v1
///
/// # Stability
///
/// The filter uses carefully designed coefficient calculations with
/// clamping to prevent instability at extreme parameter values.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// SVF filter mode selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SVFMode {
    /// Low-pass filter (passes low frequencies, attenuates high)
    LowPass,
    /// High-pass filter (passes high frequencies, attenuates low)
    HighPass,
    /// Band-pass filter (passes frequencies near cutoff)
    BandPass,
    /// Notch filter (attenuates frequencies near cutoff)
    Notch,
}

/// State Variable Filter node with multiple filter modes
///
/// # Example
/// ```ignore
/// // Lowpass SVF at 1000 Hz with Q=2.0
/// let signal = OscillatorNode::new(0, Waveform::Saw);      // NodeId 1
/// let cutoff = ConstantNode::new(1000.0);                  // NodeId 2
/// let q = ConstantNode::new(2.0);                          // NodeId 3
/// let svf = SVFNode::new(1, 2, 3, SVFMode::LowPass);       // NodeId 4
/// ```
pub struct SVFNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Cutoff frequency input (Hz)
    cutoff: NodeId,
    /// Q (resonance) input - higher Q = sharper resonance
    q: NodeId,
    /// Filter mode (set at construction)
    mode: SVFMode,
    /// First integrator state
    ic1eq: f32,
    /// Second integrator state
    ic2eq: f32,
}

impl SVFNode {
    /// SVF - State Variable Filter with four modes (LP, HP, BP, Notch)
    ///
    /// Multimode resonant filter with topology-preserving design for smooth
    /// parameter modulation. Excellent for subtractive synthesis.
    ///
    /// # Parameters
    /// - `input`: Signal to filter
    /// - `cutoff`: Cutoff frequency in Hz (20-20000)
    /// - `q`: Resonance factor (0.1-20.0, higher = sharper)
    /// - `mode`: Filter type (LowPass, HighPass, BandPass, Notch)
    ///
    /// # Example
    /// ```phonon
    /// ~osc: saw 110
    /// ~cutoff: lfo 0.5 500 2000
    /// out: ~osc # svf ~cutoff 2.0 lpf
    /// ```
    pub fn new(input: NodeId, cutoff: NodeId, q: NodeId, mode: SVFMode) -> Self {
        Self {
            input,
            cutoff,
            q,
            mode,
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    /// Get the filter mode
    pub fn mode(&self) -> SVFMode {
        self.mode
    }

    /// Reset filter state (clear integrators)
    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    /// Process a single sample through the SVF
    ///
    /// Uses topology-preserving transform for stable filtering
    #[inline]
    fn process_sample(&mut self, input: f32, cutoff: f32, q: f32, sample_rate: f32) -> f32 {
        // Clamp parameters to safe ranges
        let cutoff = cutoff.max(20.0).min(sample_rate * 0.49);
        let q = q.max(0.1).min(20.0);

        // Calculate filter coefficients using topology-preserving transform
        let g = (PI * cutoff / sample_rate).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        // Compute intermediate values
        let v3 = input - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;

        // Update integrator states (trapezoidal integration)
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        // Select output based on mode
        match self.mode {
            SVFMode::LowPass => v2,
            SVFMode::BandPass => v1,
            SVFMode::HighPass => input - k * v1 - v2,
            SVFMode::Notch => input - k * v1,
        }
    }
}

impl AudioNode for SVFNode {
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
            "SVFNode requires 3 inputs: signal, cutoff, q"
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
        debug_assert_eq!(q_buffer.len(), output.len(), "Q buffer length mismatch");

        for i in 0..output.len() {
            output[i] =
                self.process_sample(input_buffer[i], cutoff_buffer[i], q_buffer[i], sample_rate);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.cutoff, self.q]
    }

    fn name(&self) -> &str {
        "SVFNode"
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
    fn test_svf_lowpass_attenuates_high_frequencies() {
        // High frequency (8000 Hz) should be attenuated by 1000 Hz lowpass
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(0.707);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::LowPass);

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
        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        // Measure output RMS
        let output_rms = calculate_rms(&output);

        // 8000 Hz should be heavily attenuated by 1000 Hz lowpass
        assert!(
            output_rms < input_rms * 0.1,
            "Lowpass: High frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_lowpass_passes_low_frequencies() {
        // 440 Hz should pass through 1000 Hz lowpass relatively unchanged
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(0.707);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::LowPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 440 Hz should pass through with minimal attenuation
        assert!(
            output_rms > input_rms * 0.8,
            "Lowpass: Passband signal attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_highpass_attenuates_low_frequencies() {
        // Low frequency (100 Hz) should be attenuated by 1000 Hz highpass
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(0.707);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::HighPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 100 Hz should be heavily attenuated by 1000 Hz highpass
        assert!(
            output_rms < input_rms * 0.2,
            "Highpass: Low frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_highpass_passes_high_frequencies() {
        // 8000 Hz should pass through 1000 Hz highpass
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(0.707);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::HighPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 8000 Hz should pass through with reasonable attenuation
        assert!(
            output_rms > input_rms * 0.7,
            "Highpass: High frequency attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_bandpass_passes_center_frequency() {
        // 1000 Hz should pass through 1000 Hz bandpass
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(2.0);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::BandPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // Center frequency should pass through with minimal attenuation
        assert!(
            output_rms > input_rms * 0.5,
            "Bandpass: Center frequency attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_bandpass_attenuates_off_center() {
        // 100 Hz should be attenuated by 1000 Hz bandpass with Q=5.0
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(5.0);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::BandPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // Off-center frequency should be attenuated (relaxed for 2nd-order filter)
        assert!(
            output_rms < input_rms * 0.3,
            "Bandpass: Off-center frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_notch_attenuates_center_frequency() {
        // 1000 Hz should be attenuated by 1000 Hz notch
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(5.0);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::Notch);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // Center frequency should be attenuated (relaxed for 2nd-order filter)
        assert!(
            output_rms < input_rms * 0.3,
            "Notch: Center frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_notch_passes_off_center() {
        // 440 Hz should pass through 1000 Hz notch
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut q = ConstantNode::new(5.0);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::Notch);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output = vec![0.0; 512];
        svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // Off-center frequency should pass through
        assert!(
            output_rms > input_rms * 0.8,
            "Notch: Off-center frequency attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_svf_q_factor_affects_resonance() {
        // Higher Q should create sharper resonance peak
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);

        // Low Q
        let mut q_low = ConstantNode::new(0.707);
        let mut svf_low = SVFNode::new(1, 2, 3, SVFMode::BandPass);

        // High Q
        let mut q_high = ConstantNode::new(10.0);
        let mut svf_high = SVFNode::new(1, 2, 3, SVFMode::BandPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_low_buf = vec![0.0; 512];
        let mut q_high_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        q_low.process_block(&[], &mut q_low_buf, 44100.0, &context);
        q_high.process_block(&[], &mut q_high_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        // Process with low Q
        let inputs_low = vec![
            osc_buf.as_slice(),
            cutoff_buf.as_slice(),
            q_low_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; 512];
        svf_low.process_block(&inputs_low, &mut output_low, 44100.0, &context);
        let rms_low = calculate_rms(&output_low);

        // Process with high Q
        let inputs_high = vec![
            osc_buf.as_slice(),
            cutoff_buf.as_slice(),
            q_high_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; 512];
        svf_high.process_block(&inputs_high, &mut output_high, 44100.0, &context);
        let rms_high = calculate_rms(&output_high);

        // High Q should produce higher amplitude at center frequency due to resonance
        assert!(
            rms_high > rms_low,
            "High Q should produce more resonance: low_Q={} (RMS={}), high_Q={} (RMS={}), input={}",
            0.707,
            rms_low,
            10.0,
            rms_high,
            input_rms
        );
    }

    #[test]
    fn test_svf_modulated_cutoff() {
        // Test that cutoff can be modulated over time
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Start with cutoff at 2000 Hz
        let mut cutoff = ConstantNode::new(2000.0);
        let mut q = ConstantNode::new(1.0);
        let mut svf = SVFNode::new(1, 2, 3, SVFMode::LowPass);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];
        let mut q_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        q.process_block(&[], &mut q_buf, 44100.0, &context);

        // Process with cutoff at 2000 Hz (440 Hz should pass)
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        let inputs1 = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output1 = vec![0.0; 512];
        svf.process_block(&inputs1, &mut output1, 44100.0, &context);
        let rms1 = calculate_rms(&output1);

        // Change cutoff to 200 Hz (440 Hz should be attenuated)
        cutoff.set_value(200.0);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
        let inputs2 = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
        let mut output2 = vec![0.0; 512];
        svf.process_block(&inputs2, &mut output2, 44100.0, &context);
        let rms2 = calculate_rms(&output2);

        // With low cutoff, signal should be more attenuated
        assert!(
            rms2 < rms1 * 0.5,
            "Lower cutoff should attenuate more: cutoff=2000Hz RMS={}, cutoff=200Hz RMS={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_svf_input_nodes() {
        let svf = SVFNode::new(10, 20, 30, SVFMode::LowPass);
        let deps = svf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // signal input
        assert_eq!(deps[1], 20); // cutoff input
        assert_eq!(deps[2], 30); // q input
    }

    #[test]
    fn test_svf_reset() {
        let mut svf = SVFNode::new(0, 1, 2, SVFMode::LowPass);

        // Set some state
        svf.ic1eq = 1.0;
        svf.ic2eq = 2.0;

        // Reset should clear state
        svf.reset();

        assert_eq!(svf.ic1eq, 0.0);
        assert_eq!(svf.ic2eq, 0.0);
    }

    #[test]
    fn test_svf_parameter_clamping() {
        // Test that extreme parameter values are clamped
        let mut signal = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(100000.0); // Way above Nyquist
        let mut q = ConstantNode::new(100.0); // Unreasonably high Q
        let mut svf = SVFNode::new(0, 1, 2, SVFMode::LowPass);

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
        svf.process_block(&inputs, &mut output, 44100.0, &context);

        // All output values should be finite
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_svf_all_modes_produce_valid_output() {
        // Test that all four modes produce valid (finite) output
        let modes = [
            SVFMode::LowPass,
            SVFMode::HighPass,
            SVFMode::BandPass,
            SVFMode::Notch,
        ];

        for mode in &modes {
            let mut freq_node = ConstantNode::new(1000.0);
            let mut osc = OscillatorNode::new(0, Waveform::Sine);
            let mut cutoff = ConstantNode::new(1000.0);
            let mut q = ConstantNode::new(2.0);
            let mut svf = SVFNode::new(1, 2, 3, *mode);

            let context = test_context();

            let mut freq_buf = vec![0.0; 512];
            let mut osc_buf = vec![0.0; 512];
            let mut cutoff_buf = vec![0.0; 512];
            let mut q_buf = vec![0.0; 512];

            freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
            let inputs_osc = vec![freq_buf.as_slice()];
            osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
            cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);
            q.process_block(&[], &mut q_buf, 44100.0, &context);

            let inputs_svf = vec![osc_buf.as_slice(), cutoff_buf.as_slice(), q_buf.as_slice()];
            let mut output = vec![0.0; 512];
            svf.process_block(&inputs_svf, &mut output, 44100.0, &context);

            // All output should be finite
            assert!(
                output.iter().all(|&x| x.is_finite()),
                "Mode {:?} produced non-finite output",
                mode
            );

            // Should produce some output
            let rms = calculate_rms(&output);
            assert!(rms > 0.0, "Mode {:?} produced no output", mode);
        }
    }

    #[test]
    fn test_svf_state_isolation() {
        // Two instances should have independent state
        let mut svf1 = SVFNode::new(0, 1, 2, SVFMode::LowPass);
        let mut svf2 = SVFNode::new(0, 1, 2, SVFMode::LowPass);

        // Process with svf1
        svf1.ic1eq = 1.0;
        svf1.ic2eq = 2.0;

        // svf2 should still have zero state
        assert_eq!(svf2.ic1eq, 0.0);
        assert_eq!(svf2.ic2eq, 0.0);
    }
}
