/// Resonant Low-Pass Filter node - biquad lowpass with Q/resonance control
///
/// This node implements a 2nd-order resonant low-pass filter using the biquad
/// topology from the Audio EQ Cookbook. Unlike a basic Butterworth lowpass,
/// RLPF allows resonance values that create a peak at the cutoff frequency.
///
/// # Implementation Details
///
/// Uses the biquad cookbook lowpass formula:
/// - Q is derived from resonance parameter: Q = sqrt(2) / (2 - 2*res)
/// - res = 0.0 → Q = 0.707 (Butterworth, flat response)
/// - res = 0.5 → Q = 1.414 (moderate resonance)
/// - res = 1.0 → Q = ∞ (self-oscillation, clamped to prevent instability)
///
/// Based on:
/// - Robert Bristow-Johnson's Audio EQ Cookbook
/// - SuperCollider's RLPF UGen
/// - Analog synthesizer resonant filter designs
///
/// # Musical Characteristics
///
/// - 12 dB/octave rolloff (2-pole)
/// - Resonance boost at cutoff frequency
/// - Classic analog synth filter sound
/// - Self-oscillation at high resonance (becomes sine oscillator)
/// - Warm, musical character suitable for subtractive synthesis

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Internal state for the resonant lowpass filter
#[derive(Debug, Clone)]
struct RLPFState {
    /// First delay element (x[n-1])
    x1: f32,
    /// Second delay element (x[n-2])
    x2: f32,
    /// First output delay (y[n-1])
    y1: f32,
    /// Second output delay (y[n-2])
    y2: f32,
}

impl Default for RLPFState {
    fn default() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

/// Resonant low-pass filter node with pattern-controlled cutoff and resonance
///
/// # Example
/// ```ignore
/// // Classic synth bass - 500 Hz lowpass with moderate resonance
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let cutoff = ConstantNode::new(500.0);                  // NodeId 2
/// let resonance = ConstantNode::new(0.7);                 // NodeId 3
/// let rlpf = RLPFNode::new(1, 2, 3);                      // NodeId 4
/// ```
///
/// # Musical Applications
/// - Subtractive synthesis (filtering harmonics from oscillators)
/// - Classic analog synth sounds (resonant sweeps)
/// - Self-oscillation as sine oscillator (resonance near 1.0)
/// - Filter modulation effects (LFO on cutoff/resonance)
/// - Acid basslines (high resonance + cutoff modulation)
pub struct RLPFNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Cutoff frequency input (Hz)
    freq: NodeId,
    /// Resonance input (0.0 to 1.0, higher = more resonance)
    res: NodeId,
    /// Biquad filter state (maintains filter memory between blocks)
    filter: DirectForm2Transposed<f32>,
    /// Last cutoff value (for detecting changes)
    last_cutoff: f32,
    /// Last resonance value (for detecting changes)
    last_res: f32,
}

impl RLPFNode {
    /// Create a new resonant low-pass filter node
    ///
    /// # Arguments
    /// * `input` - NodeId providing signal to filter
    /// * `freq` - NodeId providing cutoff frequency in Hz (20 to 20000)
    /// * `res` - NodeId providing resonance amount (0.0 to 1.0)
    ///
    /// # Notes
    /// - Resonance = 0.0 gives Butterworth response (no resonance peak)
    /// - Resonance = 0.5 gives moderate resonance boost
    /// - Resonance = 0.9+ causes strong resonance and self-oscillation
    /// - Resonance near 1.0 is clamped to prevent instability
    /// - Cutoff frequency is clamped to 20 Hz - Nyquist
    pub fn new(input: NodeId, freq: NodeId, res: NodeId) -> Self {
        // Initialize with 1000 Hz cutoff, no resonance (Butterworth)
        let q = 0.707; // Butterworth Q
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            1000.0.hz(),
            q,
        )
        .unwrap();

        Self {
            input,
            freq,
            res,
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_cutoff: 1000.0,
            last_res: 0.0,
        }
    }

    /// Convert resonance (0.0 to 1.0) to Q factor
    ///
    /// Formula: Q = sqrt(2) / (2 - 2*res)
    /// - res = 0.0 → Q = 0.707 (Butterworth)
    /// - res = 0.5 → Q = 1.414
    /// - res → 1.0 → Q → ∞ (clamped to prevent instability)
    fn resonance_to_q(res: f32) -> f32 {
        // Clamp resonance to prevent division by zero and instability
        let res_clamped = res.max(0.0).min(0.99);
        let q = (2.0_f32).sqrt() / (2.0 - 2.0 * res_clamped);
        // Clamp Q to reasonable range (0.1 to 100)
        q.max(0.1).min(100.0)
    }

    /// Get current cutoff frequency
    pub fn cutoff(&self) -> f32 {
        self.last_cutoff
    }

    /// Get current resonance value
    pub fn resonance(&self) -> f32 {
        self.last_res
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        let q = Self::resonance_to_q(self.last_res);
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            self.last_cutoff.hz(),
            q,
        )
        .unwrap();
        self.filter = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for RLPFNode {
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
            "RLPFNode requires 3 inputs: signal, freq, res"
        );

        let input_buffer = inputs[0];
        let freq_buffer = inputs[1];
        let res_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Freq buffer length mismatch"
        );
        debug_assert_eq!(
            res_buffer.len(),
            output.len(),
            "Res buffer length mismatch"
        );

        for i in 0..output.len() {
            // Clamp cutoff to valid range (20 Hz to Nyquist)
            let cutoff = freq_buffer[i].max(20.0).min(sample_rate * 0.49);

            // Clamp resonance to valid range (0.0 to 0.99)
            let res = res_buffer[i].max(0.0).min(0.99);

            // Update filter coefficients if parameters changed significantly
            if (cutoff - self.last_cutoff).abs() > 0.1 || (res - self.last_res).abs() > 0.01 {
                let q = Self::resonance_to_q(res);
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::LowPass,
                    sample_rate.hz(),
                    cutoff.hz(),
                    q,
                )
                .unwrap();
                self.filter = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_cutoff = cutoff;
                self.last_res = res;
            }

            // Apply filter to current sample
            output[i] = self.filter.run(input_buffer[i]);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.freq, self.res]
    }

    fn name(&self) -> &str {
        "RLPFNode"
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
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_rlpf_low_frequency_passes() {
        // Test 1: Low frequency (440 Hz) should pass through 1000 Hz filter
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut res = ConstantNode::new(0.0); // No resonance
        let mut rlpf = RLPFNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        rlpf.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);

        // 440 Hz should pass through 1000 Hz lowpass with minimal attenuation
        assert!(
            output_rms > input_rms * 0.85,
            "Low frequency should pass: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_rlpf_high_frequency_attenuation() {
        // Test 2: High frequency (8000 Hz) should be attenuated by 1000 Hz filter
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut res = ConstantNode::new(0.0);
        let mut rlpf = RLPFNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        rlpf.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);

        // 8000 Hz should be heavily attenuated by 1000 Hz lowpass
        // 12 dB/octave = 36 dB attenuation at 3 octaves above cutoff
        assert!(
            output_rms < input_rms * 0.1,
            "High frequency should be attenuated: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_rlpf_resonance_boost() {
        // Test 3: High resonance should boost signal at cutoff frequency
        let sample_rate = 44100.0;
        let block_size = 1024; // Longer for stable measurements

        // Signal at cutoff frequency
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);

        // Test with no resonance
        let mut res_low = ConstantNode::new(0.0);
        let mut res_low_buf = vec![0.0; block_size];
        res_low.process_block(&[], &mut res_low_buf, sample_rate, &context);

        let mut rlpf_low = RLPFNode::new(1, 2, 3);
        let inputs_low = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_low_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            rlpf_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);
        }

        let rms_low = calculate_rms(&output_low);

        // Test with high resonance
        let mut res_high = ConstantNode::new(0.8);
        let mut res_high_buf = vec![0.0; block_size];
        res_high.process_block(&[], &mut res_high_buf, sample_rate, &context);

        let mut rlpf_high = RLPFNode::new(1, 2, 3);
        let inputs_high = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_high_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            rlpf_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);
        }

        let rms_high = calculate_rms(&output_high);

        // High resonance should boost signal at cutoff frequency
        assert!(
            rms_high > rms_low * 1.5,
            "High resonance should boost signal: low={}, high={}, ratio={}",
            rms_low,
            rms_high,
            rms_high / rms_low
        );
    }

    #[test]
    fn test_rlpf_no_resonance_butterworth() {
        // Test 4: res=0 should give Butterworth response (Q=0.707)
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut res = ConstantNode::new(0.0); // Butterworth
        let mut rlpf = RLPFNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..3 {
            rlpf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // Should have significant output (filter is working)
        assert!(
            output_rms > 0.4,
            "Butterworth response should pass signal: {}",
            output_rms
        );
    }

    #[test]
    fn test_rlpf_high_resonance_characteristics() {
        // Test 5: Very high resonance creates strong peak at cutoff
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut res = ConstantNode::new(0.95); // Very high resonance

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let mut rlpf = RLPFNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..5 {
            rlpf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // Very high resonance should amplify signal at cutoff
        assert!(
            output_rms > input_rms * 1.5,
            "Very high resonance should amplify: input={}, output={}",
            input_rms,
            output_rms
        );

        // Should remain stable (no NaN/Inf)
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} should be finite: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_rlpf_frequency_modulation() {
        // Test 6: Cutoff frequency changes should affect filtering
        let sample_rate = 44100.0;
        let block_size = 512;

        // Rich harmonic content
        let mut freq_node = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut res = ConstantNode::new(0.5);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        // Test with low cutoff (300 Hz)
        let mut cutoff_low = ConstantNode::new(300.0);
        let mut cutoff_low_buf = vec![0.0; block_size];
        cutoff_low.process_block(&[], &mut cutoff_low_buf, sample_rate, &context);

        let mut rlpf_low = RLPFNode::new(1, 2, 3);
        let inputs_low = vec![
            signal_buf.as_slice(),
            cutoff_low_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; block_size];
        rlpf_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

        let rms_low = calculate_rms(&output_low);

        // Test with high cutoff (2000 Hz)
        let mut cutoff_high = ConstantNode::new(2000.0);
        let mut cutoff_high_buf = vec![0.0; block_size];
        cutoff_high.process_block(&[], &mut cutoff_high_buf, sample_rate, &context);

        let mut rlpf_high = RLPFNode::new(1, 2, 3);
        let inputs_high = vec![
            signal_buf.as_slice(),
            cutoff_high_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];
        rlpf_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

        let rms_high = calculate_rms(&output_high);

        // Higher cutoff should pass more harmonics = higher RMS
        assert!(
            rms_high > rms_low * 1.3,
            "Higher cutoff should pass more harmonics: low={}, high={}",
            rms_low,
            rms_high
        );
    }

    #[test]
    fn test_rlpf_resonance_modulation() {
        // Test 7: Resonance changes should affect output
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Signal at cutoff frequency
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);

        // Test different resonance values
        let resonance_values = [0.0, 0.4, 0.7];
        let mut rms_values = Vec::new();

        for &res_val in &resonance_values {
            let mut res_node = ConstantNode::new(res_val);
            let mut res_buf = vec![0.0; block_size];
            res_node.process_block(&[], &mut res_buf, sample_rate, &context);

            let mut rlpf = RLPFNode::new(1, 2, 3);
            let inputs = vec![
                signal_buf.as_slice(),
                cutoff_buf.as_slice(),
                res_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];

            // Multiple blocks to reach steady state
            for _ in 0..3 {
                rlpf.process_block(&inputs, &mut output, sample_rate, &context);
            }

            rms_values.push(calculate_rms(&output));
        }

        // RMS should increase with resonance at cutoff frequency
        assert!(
            rms_values[1] > rms_values[0],
            "RMS should increase with resonance: {} -> {}",
            rms_values[0],
            rms_values[1]
        );
        assert!(
            rms_values[2] > rms_values[1],
            "RMS should increase with resonance: {} -> {}",
            rms_values[1],
            rms_values[2]
        );
    }

    #[test]
    fn test_rlpf_stability() {
        // Test 8: Filter should remain stable with extreme parameters
        let sample_rate = 44100.0;
        let block_size = 512;

        // Complex input signal
        let mut signal_buf = vec![0.0; block_size];
        for i in 0..block_size {
            signal_buf[i] = ((i as f32 * 0.1).sin() * 0.8).clamp(-1.0, 1.0);
        }

        let mut cutoff = ConstantNode::new(18000.0); // Very high cutoff
        let mut res = ConstantNode::new(0.98); // Near maximum resonance

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let mut rlpf = RLPFNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks
        for _ in 0..10 {
            rlpf.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} became infinite/NaN",
                    i
                );
                assert!(
                    sample.abs() < 100.0,
                    "Sample {} has extreme value: {}",
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_rlpf_input_nodes() {
        // Test 9: Verify input node dependencies
        let rlpf = RLPFNode::new(10, 20, 30);
        let deps = rlpf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // freq
        assert_eq!(deps[2], 30); // res
    }

    #[test]
    fn test_rlpf_resonance_to_q_conversion() {
        // Test 10: Verify resonance to Q conversion
        // res = 0.0 → Q ≈ 0.707 (Butterworth)
        let q0 = RLPFNode::resonance_to_q(0.0);
        assert!(
            (q0 - 0.707).abs() < 0.01,
            "res=0 should give Q≈0.707, got {}",
            q0
        );

        // res = 0.5 → Q ≈ 1.414
        let q5 = RLPFNode::resonance_to_q(0.5);
        assert!(
            (q5 - 1.414).abs() < 0.01,
            "res=0.5 should give Q≈1.414, got {}",
            q5
        );

        // res = 0.9 → Q should be high but not infinite
        let q9 = RLPFNode::resonance_to_q(0.9);
        assert!(
            q9 > 5.0 && q9 < 100.0,
            "res=0.9 should give high Q (5-100), got {}",
            q9
        );
    }

    #[test]
    fn test_rlpf_parameter_clamping() {
        // Test 11: Extreme parameters should be clamped safely
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        signal_buf[0] = 1.0;

        let mut cutoff = ConstantNode::new(100000.0); // Way above Nyquist
        let mut res = ConstantNode::new(5.0); // Way above valid range

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let mut rlpf = RLPFNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Should not panic despite extreme values
        rlpf.process_block(&inputs, &mut output, sample_rate, &context);

        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} not finite with extreme parameters: {}",
                i,
                sample
            );
        }

        // Filter should clamp cutoff below Nyquist
        assert!(
            rlpf.cutoff() < 22050.0,
            "Cutoff not clamped: {}",
            rlpf.cutoff()
        );

        // Filter should clamp resonance to valid range
        assert!(
            rlpf.resonance() < 1.0,
            "Resonance not clamped: {}",
            rlpf.resonance()
        );
    }

    #[test]
    fn test_rlpf_reset() {
        // Test 12: Reset should clear state
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        signal_buf[0] = 1.0; // Impulse

        let mut cutoff = ConstantNode::new(1000.0);
        let mut res = ConstantNode::new(0.7);

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let mut rlpf = RLPFNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process to build up state
        for _ in 0..5 {
            rlpf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Reset
        rlpf.reset();

        // State should be preserved (cutoff/resonance)
        assert_eq!(rlpf.cutoff(), 1000.0);
        assert!(
            (rlpf.resonance() - 0.7).abs() < 0.01,
            "Resonance should be preserved: {}",
            rlpf.resonance()
        );
    }

    #[test]
    fn test_rlpf_dc_response() {
        // Test 13: DC signal should pass through lowpass
        let sample_rate = 44100.0;
        let block_size = 512;

        let signal_buf = vec![0.5; block_size]; // DC at 0.5

        let mut cutoff = ConstantNode::new(1000.0);
        let mut res = ConstantNode::new(0.2);

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let mut rlpf = RLPFNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..10 {
            rlpf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // DC should pass through (lowpass passes DC)
        let output_mean: f32 = output.iter().sum::<f32>() / output.len() as f32;

        assert!(
            output_mean > 0.45,
            "DC should pass through with minimal attenuation: got {}",
            output_mean
        );
    }

    #[test]
    fn test_rlpf_12db_rolloff() {
        // Test 14: Verify 12 dB/octave rolloff characteristic
        let sample_rate = 44100.0;
        let block_size = 1024;
        let cutoff_freq = 1000.0;

        // Test frequencies: 1 octave and 2 octaves above cutoff
        let freq_1oct = cutoff_freq * 2.0; // 2000 Hz
        let freq_2oct = cutoff_freq * 4.0; // 4000 Hz

        let mut cutoff = ConstantNode::new(cutoff_freq);
        let mut res = ConstantNode::new(0.0); // No resonance

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        // Test at 1 octave above cutoff
        let mut freq_node1 = ConstantNode::new(freq_1oct);
        let mut osc1 = OscillatorNode::new(0, Waveform::Sine);
        let mut freq_buf1 = vec![0.0; block_size];
        let mut signal_buf1 = vec![0.0; block_size];

        freq_node1.process_block(&[], &mut freq_buf1, sample_rate, &context);
        osc1.process_block(&[freq_buf1.as_slice()], &mut signal_buf1, sample_rate, &context);

        let mut rlpf1 = RLPFNode::new(1, 2, 3);
        let inputs1 = vec![
            signal_buf1.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; block_size];

        for _ in 0..3 {
            rlpf1.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        let input_rms1 = calculate_rms(&signal_buf1);
        let output_rms1 = calculate_rms(&output1);
        let attenuation_1oct = output_rms1 / input_rms1;

        // Test at 2 octaves above cutoff
        let mut freq_node2 = ConstantNode::new(freq_2oct);
        let mut osc2 = OscillatorNode::new(0, Waveform::Sine);
        let mut freq_buf2 = vec![0.0; block_size];
        let mut signal_buf2 = vec![0.0; block_size];

        freq_node2.process_block(&[], &mut freq_buf2, sample_rate, &context);
        osc2.process_block(&[freq_buf2.as_slice()], &mut signal_buf2, sample_rate, &context);

        let mut rlpf2 = RLPFNode::new(1, 2, 3);
        let inputs2 = vec![
            signal_buf2.as_slice(),
            cutoff_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; block_size];

        for _ in 0..3 {
            rlpf2.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let input_rms2 = calculate_rms(&signal_buf2);
        let output_rms2 = calculate_rms(&output2);
        let attenuation_2oct = output_rms2 / input_rms2;

        // 12 dB/octave means:
        // 1 octave above = -12 dB ≈ 0.25x amplitude
        // 2 octaves above = -24 dB ≈ 0.063x amplitude
        // Ratio should be roughly 4:1
        let attenuation_ratio = attenuation_1oct / attenuation_2oct;

        assert!(
            attenuation_ratio > 2.5 && attenuation_ratio < 6.0,
            "12dB/oct rolloff: 1oct={:.4}, 2oct={:.4}, ratio={:.2} (expected ~3-5)",
            attenuation_1oct,
            attenuation_2oct,
            attenuation_ratio
        );
    }
}
