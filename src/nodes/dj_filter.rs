/// DJ-style resonant crossfader filter node
///
/// This node implements a classic DJ mixer filter that crossfades between
/// lowpass and highpass filtering based on a position parameter.
///
/// # Implementation Details
///
/// Uses two parallel biquad filters (lowpass and highpass) with crossfading:
/// - position = -1.0 → full lowpass (bass only)
/// - position = 0.0 → neutral/passthrough (no filtering)
/// - position = +1.0 → full highpass (treble only)
///
/// The resonance parameter adds the characteristic "DJ sweep" sound by
/// creating a peak at the cutoff frequency.
///
/// Based on:
/// - Classic DJ mixer designs (Pioneer DJM, Allen & Heath Xone)
/// - Robert Bristow-Johnson's Audio EQ Cookbook (biquad filters)
/// - SuperCollider's filter UGens
///
/// # Musical Characteristics
///
/// - Smooth crossfade between lowpass and highpass
/// - Resonance creates dramatic sweep effects
/// - 12 dB/octave rolloff on both sides (2-pole)
/// - Neutral position allows clean mixing
/// - Perfect for live DJ performance and mixing

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// DJ filter node with pattern-controlled position and resonance
///
/// # Example
/// ```ignore
/// // Classic DJ filter sweep - position modulation
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let position = LFONode::new(0.25);                      // NodeId 2 (slow LFO)
/// let resonance = ConstantNode::new(0.7);                 // NodeId 3
/// let djf = DJFilterNode::new(1, 2, 3);                   // NodeId 4
/// ```
///
/// # Musical Applications
/// - DJ mixer filter sweeps (classic transition effect)
/// - Isolating bass or treble in mix
/// - Live performance filter effects
/// - Dramatic buildup/breakdown sections
/// - Creative automation in production
pub struct DJFilterNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Filter position input (-1.0 to +1.0)
    /// -1.0 = full lowpass, 0.0 = neutral, +1.0 = full highpass
    position: NodeId,
    /// Resonance input (0.0 to 1.0, higher = more resonance)
    resonance: NodeId,
    /// Lowpass filter state
    lowpass: DirectForm2Transposed<f32>,
    /// Highpass filter state
    highpass: DirectForm2Transposed<f32>,
    /// Last position value (for detecting changes)
    last_position: f32,
    /// Last resonance value (for detecting changes)
    last_resonance: f32,
}

impl DJFilterNode {
    /// Create a new DJ filter node
    ///
    /// # Arguments
    /// * `input` - NodeId providing signal to filter
    /// * `position` - NodeId providing filter position (-1.0 to +1.0)
    /// * `resonance` - NodeId providing resonance amount (0.0 to 1.0)
    ///
    /// # Notes
    /// - Position = -1.0: full lowpass at 500 Hz
    /// - Position = 0.0: neutral (both filters at extremes, minimal effect)
    /// - Position = +1.0: full highpass at 2000 Hz
    /// - Resonance = 0.0: Butterworth response (no peak)
    /// - Resonance = 0.5: moderate peak
    /// - Resonance = 0.9+: strong resonance (classic DJ sweep)
    pub fn new(input: NodeId, position: NodeId, resonance: NodeId) -> Self {
        // Initialize with neutral position (no filtering effect)
        let lp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            20.0.hz(), // Very low (out of way)
            0.707,
        )
        .unwrap();

        let hp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            20000.0.hz(), // Very high (out of way)
            0.707,
        )
        .unwrap();

        Self {
            input,
            position,
            resonance,
            lowpass: DirectForm2Transposed::<f32>::new(lp_coeffs),
            highpass: DirectForm2Transposed::<f32>::new(hp_coeffs),
            last_position: 0.0,
            last_resonance: 0.0,
        }
    }

    /// Convert resonance (0.0 to 1.0) to Q factor
    ///
    /// Formula: Q = sqrt(2) / (2 - 2*res)
    /// - res = 0.0 → Q = 0.707 (Butterworth)
    /// - res = 0.5 → Q = 1.414
    /// - res → 1.0 → Q → ∞ (clamped to prevent instability)
    fn resonance_to_q(res: f32) -> f32 {
        // Clamp resonance to prevent division by zero
        let res_clamped = res.max(0.0).min(0.99);
        let q = (2.0_f32).sqrt() / (2.0 - 2.0 * res_clamped);
        // Clamp Q to reasonable range
        q.max(0.1).min(100.0)
    }

    /// Calculate filter frequencies and mix amounts from position
    ///
    /// Returns: (lp_freq, hp_freq, lp_mix, hp_mix)
    fn calculate_filter_params(position: f32) -> (f32, f32, f32, f32) {
        // Clamp position to valid range
        let pos = position.max(-1.0).min(1.0);

        if pos < 0.0 {
            // Lowpass side: -1.0 to 0.0
            let amt = -pos; // 0 to 1
            let lp_freq = 500.0 + amt * 1500.0; // 500-2000 Hz sweep
            let hp_freq = 20000.0; // Out of way
            (lp_freq, hp_freq, amt, 0.0)
        } else {
            // Highpass side: 0.0 to 1.0
            let amt = pos; // 0 to 1
            let lp_freq = 20.0; // Out of way
            let hp_freq = 2000.0 - amt * 1500.0; // 2000-500 Hz sweep
            (lp_freq, hp_freq, 0.0, amt)
        }
    }

    /// Get current position value
    pub fn position(&self) -> f32 {
        self.last_position
    }

    /// Get current resonance value
    pub fn resonance(&self) -> f32 {
        self.last_resonance
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        let q = Self::resonance_to_q(self.last_resonance);
        let (lp_freq, hp_freq, _, _) = Self::calculate_filter_params(self.last_position);

        let lp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            lp_freq.hz(),
            q,
        )
        .unwrap();

        let hp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            hp_freq.hz(),
            q,
        )
        .unwrap();

        self.lowpass = DirectForm2Transposed::<f32>::new(lp_coeffs);
        self.highpass = DirectForm2Transposed::<f32>::new(hp_coeffs);
    }
}

impl AudioNode for DJFilterNode {
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
            "DJFilterNode requires 3 inputs: signal, position, resonance"
        );

        let input_buffer = inputs[0];
        let position_buffer = inputs[1];
        let resonance_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            position_buffer.len(),
            output.len(),
            "Position buffer length mismatch"
        );
        debug_assert_eq!(
            resonance_buffer.len(),
            output.len(),
            "Resonance buffer length mismatch"
        );

        for i in 0..output.len() {
            let position = position_buffer[i].max(-1.0).min(1.0);
            let resonance = resonance_buffer[i].max(0.0).min(0.99);

            // Update filter coefficients if parameters changed significantly
            if (position - self.last_position).abs() > 0.01
                || (resonance - self.last_resonance).abs() > 0.01
            {
                let q = Self::resonance_to_q(resonance);
                let (lp_freq, hp_freq, _, _) = Self::calculate_filter_params(position);

                // Clamp frequencies to safe range
                let lp_freq = lp_freq.max(20.0).min(sample_rate * 0.49);
                let hp_freq = hp_freq.max(20.0).min(sample_rate * 0.49);

                let lp_coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::LowPass,
                    sample_rate.hz(),
                    lp_freq.hz(),
                    q,
                )
                .unwrap();

                let hp_coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::HighPass,
                    sample_rate.hz(),
                    hp_freq.hz(),
                    q,
                )
                .unwrap();

                self.lowpass = DirectForm2Transposed::<f32>::new(lp_coeffs);
                self.highpass = DirectForm2Transposed::<f32>::new(hp_coeffs);
                self.last_position = position;
                self.last_resonance = resonance;
            }

            // Get mix amounts for crossfade
            let (_, _, lp_mix, hp_mix) = Self::calculate_filter_params(position);

            // Apply both filters
            let lp_out = self.lowpass.run(input_buffer[i]);
            let hp_out = self.highpass.run(input_buffer[i]);

            // Crossfade between filters
            // At position = 0, both mixes are 0, so we get dry signal
            if lp_mix > 0.0 {
                output[i] = lp_out * lp_mix + input_buffer[i] * (1.0 - lp_mix);
            } else if hp_mix > 0.0 {
                output[i] = hp_out * hp_mix + input_buffer[i] * (1.0 - hp_mix);
            } else {
                // Neutral position - passthrough
                output[i] = input_buffer[i];
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.position, self.resonance]
    }

    fn name(&self) -> &str {
        "DJFilterNode"
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

    // ========================================================================
    // TEST 1: Full lowpass at -1.0
    // ========================================================================
    #[test]
    fn test_djfilter_full_lowpass_cuts_highs() {
        let sample_rate = 44100.0;
        let block_size = 1024;

        // High-frequency signal (8000 Hz)
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut position = ConstantNode::new(-1.0); // Full lowpass
        let mut res = ConstantNode::new(0.0); // No resonance

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pos_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        position.process_block(&[], &mut pos_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let mut djf = DJFilterNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            djf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // 8000 Hz should be strongly attenuated by lowpass
        assert!(
            output_rms < input_rms * 0.3,
            "Full lowpass should cut highs: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    // ========================================================================
    // TEST 2: Full highpass at +1.0
    // ========================================================================
    #[test]
    fn test_djfilter_full_highpass_cuts_lows() {
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Low-frequency signal (100 Hz)
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut position = ConstantNode::new(1.0); // Full highpass
        let mut res = ConstantNode::new(0.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pos_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        position.process_block(&[], &mut pos_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let mut djf = DJFilterNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            djf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // 100 Hz should be strongly attenuated by highpass
        assert!(
            output_rms < input_rms * 0.3,
            "Full highpass should cut lows: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    // ========================================================================
    // TEST 3: Passthrough at 0.0
    // ========================================================================
    #[test]
    fn test_djfilter_neutral_position_passthrough() {
        let sample_rate = 44100.0;
        let block_size = 512;

        // Mid-frequency signal (1000 Hz)
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut position = ConstantNode::new(0.0); // Neutral position
        let mut res = ConstantNode::new(0.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pos_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        position.process_block(&[], &mut pos_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let mut djf = DJFilterNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        djf.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);

        // At neutral position, signal should pass through unchanged
        assert!(
            (output_rms - input_rms).abs() < 0.01,
            "Neutral position should be passthrough: input={}, output={}",
            input_rms,
            output_rms
        );
    }

    // ========================================================================
    // TEST 4: Smooth crossfade
    // ========================================================================
    #[test]
    fn test_djfilter_smooth_crossfade() {
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Broadband signal (sawtooth)
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut res = ConstantNode::new(0.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        // Test sweep from -1.0 to +1.0
        let positions = [-1.0, -0.5, 0.0, 0.5, 1.0];
        let mut rms_values = Vec::new();

        for &pos in &positions {
            let mut position = ConstantNode::new(pos);
            let mut pos_buf = vec![0.0; block_size];
            position.process_block(&[], &mut pos_buf, sample_rate, &context);

            let mut djf = DJFilterNode::new(1, 2, 3);
            let inputs = vec![
                signal_buf.as_slice(),
                pos_buf.as_slice(),
                res_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];

            // Multiple blocks for steady state
            for _ in 0..3 {
                djf.process_block(&inputs, &mut output, sample_rate, &context);
            }

            rms_values.push(calculate_rms(&output));
        }

        // All positions should produce reasonable output
        for (i, &rms) in rms_values.iter().enumerate() {
            assert!(
                rms > 0.01 && rms < 2.0,
                "Position {} should have reasonable RMS: got {}",
                positions[i],
                rms
            );
        }
    }

    // ========================================================================
    // TEST 5: Resonance affects Q
    // ========================================================================
    #[test]
    fn test_djfilter_resonance_boosts_cutoff() {
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Signal at lowpass cutoff frequency (when position = -0.5)
        // position -0.5 → cutoff = 500 + 0.5 * 1500 = 1250 Hz
        let mut freq_node = ConstantNode::new(1250.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut position = ConstantNode::new(-0.5);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pos_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        position.process_block(&[], &mut pos_buf, sample_rate, &context);

        // Test with low resonance
        let mut res_low = ConstantNode::new(0.0);
        let mut res_low_buf = vec![0.0; block_size];
        res_low.process_block(&[], &mut res_low_buf, sample_rate, &context);

        let mut djf_low = DJFilterNode::new(1, 2, 3);
        let inputs_low = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_low_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; block_size];

        for _ in 0..3 {
            djf_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);
        }
        let rms_low = calculate_rms(&output_low);

        // Test with high resonance
        let mut res_high = ConstantNode::new(0.8);
        let mut res_high_buf = vec![0.0; block_size];
        res_high.process_block(&[], &mut res_high_buf, sample_rate, &context);

        let mut djf_high = DJFilterNode::new(1, 2, 3);
        let inputs_high = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_high_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];

        for _ in 0..3 {
            djf_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);
        }
        let rms_high = calculate_rms(&output_high);

        // High resonance should boost signal at cutoff
        assert!(
            rms_high > rms_low * 1.3,
            "High resonance should boost cutoff frequency: low={}, high={}",
            rms_low,
            rms_high
        );
    }

    // ========================================================================
    // TEST 6: Frequency sweep is continuous
    // ========================================================================
    #[test]
    fn test_djfilter_continuous_frequency_sweep() {
        let sample_rate = 44100.0;
        let block_size = 512;

        // Broadband signal
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut res = ConstantNode::new(0.3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        // Sweep through many positions
        let mut rms_values = Vec::new();
        for i in 0..21 {
            let pos = -1.0 + (i as f32 * 0.1);
            let mut position = ConstantNode::new(pos);
            let mut pos_buf = vec![0.0; block_size];
            position.process_block(&[], &mut pos_buf, sample_rate, &context);

            let mut djf = DJFilterNode::new(1, 2, 3);
            let inputs = vec![
                signal_buf.as_slice(),
                pos_buf.as_slice(),
                res_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];

            for _ in 0..2 {
                djf.process_block(&inputs, &mut output, sample_rate, &context);
            }

            rms_values.push(calculate_rms(&output));
        }

        // Check for discontinuities (large jumps)
        for i in 1..rms_values.len() {
            let ratio = if rms_values[i - 1] > 0.0 {
                rms_values[i] / rms_values[i - 1]
            } else {
                1.0
            };
            assert!(
                ratio < 5.0 && ratio > 0.2,
                "Discontinuous jump at position {}: prev={}, curr={}, ratio={}",
                -1.0 + (i as f32 * 0.1),
                rms_values[i - 1],
                rms_values[i],
                ratio
            );
        }
    }

    // ========================================================================
    // TEST 7: Pattern modulation works
    // ========================================================================
    #[test]
    fn test_djfilter_parameter_modulation() {
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        // Modulate position and resonance
        let mut pos_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        for i in 0..block_size {
            // Sweep position from -1 to +1
            pos_buf[i] = -1.0 + (i as f32 / block_size as f32) * 2.0;
            // Vary resonance
            res_buf[i] = 0.5 + 0.3 * (i as f32 * 0.1).sin();
        }

        let mut djf = DJFilterNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        djf.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should be valid
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} should be finite: got {}",
                i,
                sample
            );
        }

        let rms = calculate_rms(&output);
        assert!(rms > 0.01, "Modulated filter should produce output: {}", rms);
    }

    // ========================================================================
    // TEST 8: Classic DJ sweep sound
    // ========================================================================
    #[test]
    fn test_djfilter_classic_sweep_effect() {
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Broadband signal (sawtooth has lots of harmonics)
        let mut freq_node = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut res = ConstantNode::new(0.8); // High resonance for sweep

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        // Test at lowpass extreme with high resonance
        let mut position = ConstantNode::new(-0.8);
        let mut pos_buf = vec![0.0; block_size];
        position.process_block(&[], &mut pos_buf, sample_rate, &context);

        let mut djf = DJFilterNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            djf.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let rms = calculate_rms(&output);

        // Should produce characteristic resonant sound
        assert!(
            rms > 0.1,
            "DJ sweep should produce audible resonant output: {}",
            rms
        );
        // Should remain stable
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite() && sample.abs() < 10.0,
                "Sample {} should be stable: got {}",
                i,
                sample
            );
        }
    }

    // ========================================================================
    // TEST 9: Input nodes verification
    // ========================================================================
    #[test]
    fn test_djfilter_input_nodes() {
        let djf = DJFilterNode::new(10, 20, 30);
        let deps = djf.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // position
        assert_eq!(deps[2], 30); // resonance
    }

    // ========================================================================
    // TEST 10: Reset functionality
    // ========================================================================
    #[test]
    fn test_djfilter_reset() {
        let mut djf = DJFilterNode::new(0, 1, 2);

        // Set some state
        djf.last_position = -0.5;
        djf.last_resonance = 0.7;

        // Reset should preserve parameters
        djf.reset();

        assert_eq!(djf.position(), -0.5);
        assert!(
            (djf.resonance() - 0.7).abs() < 0.01,
            "Resonance should be preserved"
        );
    }

    // ========================================================================
    // TEST 11: Stability with extreme parameters
    // ========================================================================
    #[test]
    fn test_djfilter_stability_extreme_params() {
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        for i in 0..block_size {
            signal_buf[i] = ((i as f32 * 0.1).sin() * 0.8).clamp(-1.0, 1.0);
        }

        let mut position = ConstantNode::new(5.0); // Way out of range
        let mut res = ConstantNode::new(10.0); // Way out of range

        let context = test_context(block_size);

        let mut pos_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        position.process_block(&[], &mut pos_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        let mut djf = DJFilterNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            pos_buf.as_slice(),
            res_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Should not panic
        for _ in 0..10 {
            djf.process_block(&inputs, &mut output, sample_rate, &context);

            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} should be finite with extreme params: got {}",
                    i,
                    sample
                );
                assert!(
                    sample.abs() < 100.0,
                    "Sample {} should not explode: got {}",
                    i,
                    sample
                );
            }
        }
    }

    // ========================================================================
    // TEST 12: Position clamping
    // ========================================================================
    #[test]
    fn test_djfilter_position_clamping() {
        let sample_rate = 44100.0;
        let block_size = 256;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut res = ConstantNode::new(0.3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut res_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        res.process_block(&[], &mut res_buf, sample_rate, &context);

        // Test various out-of-range positions
        let test_positions = [-5.0, -2.0, 3.0, 10.0];

        for &pos in &test_positions {
            let mut position = ConstantNode::new(pos);
            let mut pos_buf = vec![0.0; block_size];
            position.process_block(&[], &mut pos_buf, sample_rate, &context);

            let mut djf = DJFilterNode::new(1, 2, 3);
            let inputs = vec![
                signal_buf.as_slice(),
                pos_buf.as_slice(),
                res_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];

            djf.process_block(&inputs, &mut output, sample_rate, &context);

            // Should produce finite output
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Position {} should be clamped (sample {} = {})",
                    pos,
                    i,
                    sample
                );
            }

            // Position should be clamped to -1.0 to +1.0
            assert!(
                djf.position() >= -1.0 && djf.position() <= 1.0,
                "Position should be clamped: input={}, clamped={}",
                pos,
                djf.position()
            );
        }
    }

    // ========================================================================
    // TEST 13: Resonance clamping
    // ========================================================================
    #[test]
    fn test_djfilter_resonance_clamping() {
        let sample_rate = 44100.0;
        let block_size = 256;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut position = ConstantNode::new(-0.5);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut pos_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        position.process_block(&[], &mut pos_buf, sample_rate, &context);

        // Test out-of-range resonance values
        let test_resonances = [-1.0, 5.0, 100.0];

        for &res_val in &test_resonances {
            let mut res = ConstantNode::new(res_val);
            let mut res_buf = vec![0.0; block_size];
            res.process_block(&[], &mut res_buf, sample_rate, &context);

            let mut djf = DJFilterNode::new(1, 2, 3);
            let inputs = vec![
                signal_buf.as_slice(),
                pos_buf.as_slice(),
                res_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];

            djf.process_block(&inputs, &mut output, sample_rate, &context);

            // Should produce finite output
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Resonance {} should be clamped (sample {} = {})",
                    res_val,
                    i,
                    sample
                );
            }

            // Resonance should be clamped to 0.0 to 0.99
            assert!(
                djf.resonance() >= 0.0 && djf.resonance() <= 0.99,
                "Resonance should be clamped: input={}, clamped={}",
                res_val,
                djf.resonance()
            );
        }
    }

    // ========================================================================
    // TEST 14: Multiple instances independence
    // ========================================================================
    #[test]
    fn test_djfilter_multiple_instances_independent() {
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        // Create two filters with different settings
        let mut pos1 = ConstantNode::new(-0.8);
        let mut res1 = ConstantNode::new(0.2);
        let mut pos1_buf = vec![0.0; block_size];
        let mut res1_buf = vec![0.0; block_size];
        pos1.process_block(&[], &mut pos1_buf, sample_rate, &context);
        res1.process_block(&[], &mut res1_buf, sample_rate, &context);

        let mut pos2 = ConstantNode::new(0.8);
        let mut res2 = ConstantNode::new(0.7);
        let mut pos2_buf = vec![0.0; block_size];
        let mut res2_buf = vec![0.0; block_size];
        pos2.process_block(&[], &mut pos2_buf, sample_rate, &context);
        res2.process_block(&[], &mut res2_buf, sample_rate, &context);

        let mut djf1 = DJFilterNode::new(1, 2, 3);
        let mut djf2 = DJFilterNode::new(1, 2, 3);

        let inputs1 = vec![
            signal_buf.as_slice(),
            pos1_buf.as_slice(),
            res1_buf.as_slice(),
        ];
        let inputs2 = vec![
            signal_buf.as_slice(),
            pos2_buf.as_slice(),
            res2_buf.as_slice(),
        ];

        let mut output1 = vec![0.0; block_size];
        let mut output2 = vec![0.0; block_size];

        for _ in 0..3 {
            djf1.process_block(&inputs1, &mut output1, sample_rate, &context);
            djf2.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let rms1 = calculate_rms(&output1);
        let rms2 = calculate_rms(&output2);

        // Both should produce output
        assert!(rms1 > 0.01, "Filter 1 should produce output: {}", rms1);
        assert!(rms2 > 0.01, "Filter 2 should produce output: {}", rms2);

        // Outputs should be different (different filter settings)
        assert!(
            (rms1 - rms2).abs() > 0.01,
            "Independent filters should produce different outputs: rms1={}, rms2={}",
            rms1,
            rms2
        );
    }
}
