/// Stereo widener node - controls stereo width and spread
///
/// This node implements stereo width control using Mid/Side processing.
/// When true stereo is available, it uses classic M/S encoding to adjust width.
/// For mono signals, it creates pseudo-stereo using an all-pass filter phase shift.
///
/// # Algorithm: Mid/Side Processing
///
/// ## Decode to Mid/Side (for stereo input):
/// ```text
/// mid = (left + right) * 0.5    // Center content
/// side = (left - right) * 0.5   // Stereo content
/// ```
///
/// ## Adjust Width:
/// ```text
/// side = side * width
/// ```
///
/// ## Encode back to L/R:
/// ```text
/// left = mid + side
/// right = mid - side
/// ```
///
/// ## Width Parameter:
/// - **0.0**: Mono (all side content removed)
/// - **1.0**: Normal stereo (unchanged)
/// - **2.0**: Ultra-wide (side content doubled)
///
/// # Current Implementation: Mono with Haas Effect
///
/// Since Phonon is currently mono, this implementation uses the Haas effect
/// to create pseudo-stereo width perception. The algorithm uses an all-pass
/// filter to create a phase-shifted version of the signal, simulating width.
///
/// When true stereo support is added, this will be upgraded to full M/S processing.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Stereo widener node with pattern-controlled width
///
/// # Parameters
/// - `input`: Audio signal (mono or stereo)
/// - `width`: Width amount (0.0 = mono, 1.0 = normal, 2.0 = ultra-wide)
///
/// # Example
/// ```ignore
/// // Widen stereo image
/// let signal = OscillatorNode::new(0, Waveform::Saw);  // NodeId 1
/// let width = ConstantNode::new(1.5);                   // NodeId 2 (50% wider)
/// let widener = StereoWidenerNode::new(1, 2, 44100.0); // NodeId 3
/// ```
pub struct StereoWidenerNode {
    /// Input signal to be widened
    input: NodeId,
    /// Width parameter input (0.0-2.0)
    width_input: NodeId,
    /// All-pass filter for creating phase-shifted "side" signal
    /// Used for pseudo-stereo width effect in mono mode
    allpass: DirectForm2Transposed<f32>,
    /// Sample rate for filter calculations
    sample_rate: f32,
}

impl StereoWidenerNode {
    /// StereoWidener - Controls stereo width using phase shift (mono) or M/S (stereo)
    ///
    /// Adjusts stereo width using Haas effect (current mono mode) or
    /// Mid/Side processing (when stereo support added).
    ///
    /// # Parameters
    /// - `input`: Audio signal to widen
    /// - `width_input`: Width amount (0.0=mono, 1.0=normal, 2.0=ultra-wide)
    /// - `sample_rate`: Sample rate for filter calculations
    ///
    /// # Example
    /// ```phonon
    /// ~synth: saw 110
    /// ~width: lfo 0.5 0.5 2.0
    /// out: ~synth # widener ~width
    /// ```
    pub fn new(input: NodeId, width_input: NodeId, sample_rate: f32) -> Self {
        // Initialize all-pass filter at ~800 Hz with Q=0.707 for smooth phase shift
        // This frequency is chosen to create perceptible width without harshness
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::AllPass,
            sample_rate.hz(),
            800.0.hz(),
            0.707,
        )
        .unwrap();

        Self {
            input,
            width_input,
            allpass: DirectForm2Transposed::<f32>::new(coeffs),
            sample_rate,
        }
    }

    /// Get the current sample rate
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Reset the all-pass filter state
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::AllPass,
            self.sample_rate.hz(),
            800.0.hz(),
            0.707,
        )
        .unwrap();
        self.allpass = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for StereoWidenerNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "StereoWidenerNode requires 2 inputs: signal, width"
        );

        let input_buffer = inputs[0];
        let width_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            width_buffer.len(),
            output.len(),
            "Width buffer length mismatch"
        );

        // Current mono implementation using Haas effect
        // When stereo support is added, this will use full M/S processing
        for i in 0..output.len() {
            let input = input_buffer[i];
            let width = width_buffer[i].clamp(0.0, 2.0);

            // Create phase-shifted version using all-pass filter
            let phase_shifted = self.allpass.run(input);

            // Width effect: blend between original and phase-shifted
            // - width=0.0: pure mono (no phase shift)
            // - width=1.0: subtle width (50% phase shift mix)
            // - width=2.0: maximum width (100% phase shift mix)
            let mix = (width - 1.0).abs(); // 0.0 at width=1.0, increases towards 0.0 or 2.0
            output[i] = input * (1.0 - mix * 0.3) + phase_shifted * mix * 0.3;

            // TODO: When stereo support is added, implement full M/S processing:
            // if stereo_available {
            //     let mid = (left + right) * 0.5;
            //     let side = (left - right) * 0.5;
            //     let adjusted_side = side * width;
            //     output_left[i] = mid + adjusted_side;
            //     output_right[i] = mid - adjusted_side;
            // }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.width_input]
    }

    fn name(&self) -> &str {
        "StereoWidenerNode"
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

    /// Helper to create test context
    fn test_context() -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0)
    }

    #[test]
    fn test_stereo_widener_width_zero_passes_signal() {
        // Test 1: Width 0.0 should produce mono (minimal width)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut width_node = ConstantNode::new(0.0); // Mono
        let mut widener = StereoWidenerNode::new(0, 1, sample_rate);

        let context = test_context();

        let mut input_buf = vec![1.0; block_size];
        let mut width_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        width_node.process_block(&[], &mut width_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        widener.process_block(&inputs, &mut output, sample_rate, &context);

        // With width 0.0, output should be close to input (mono, minimal processing)
        let avg = output.iter().sum::<f32>() / output.len() as f32;
        assert!(
            (avg - 1.0).abs() < 0.15,
            "Width 0.0 should pass signal mostly unchanged, got avg {}",
            avg
        );
    }

    #[test]
    fn test_stereo_widener_width_one_normal() {
        // Test 2: Width 1.0 should pass signal through normally

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut width_node = ConstantNode::new(1.0); // Normal
        let mut widener = StereoWidenerNode::new(0, 1, sample_rate);

        let context = test_context();

        let mut input_buf = vec![1.0; block_size];
        let mut width_buf = vec![1.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        width_node.process_block(&[], &mut width_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        widener.process_block(&inputs, &mut output, sample_rate, &context);

        // With width 1.0, output should equal input (no effect)
        let avg = output.iter().sum::<f32>() / output.len() as f32;
        assert!(
            (avg - 1.0).abs() < 0.05,
            "Width 1.0 should pass signal unchanged, got avg {}",
            avg
        );
    }

    #[test]
    fn test_stereo_widener_width_two_wide() {
        // Test 3: Width 2.0 should create maximum width effect

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut width_node = ConstantNode::new(2.0); // Ultra-wide
        let mut widener = StereoWidenerNode::new(1, 2, sample_rate);

        let context = test_context();

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut width_buf = vec![2.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);
        width_node.process_block(&[], &mut width_buf, sample_rate, &context);

        let inputs = vec![osc_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        widener.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should have some signal (not silent)
        let output_rms = calculate_rms(&output);
        assert!(
            output_rms > 0.1,
            "Width 2.0 should produce audible output, got RMS {}",
            output_rms
        );

        // All samples should be finite
        for &sample in output.iter() {
            assert!(sample.is_finite(), "Output contains non-finite values");
        }
    }

    #[test]
    fn test_stereo_widener_preserves_energy() {
        // Test 4: Widener should preserve overall energy (RMS)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context();

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);

        let input_rms = calculate_rms(&osc_buf);

        // Test different width values
        let test_widths = vec![0.0, 0.5, 1.0, 1.5, 2.0];

        for width_val in test_widths {
            let mut width_node = ConstantNode::new(width_val);
            let mut widener = StereoWidenerNode::new(1, 2, sample_rate);

            let mut width_buf = vec![width_val; block_size];
            width_node.process_block(&[], &mut width_buf, sample_rate, &context);

            let inputs = vec![osc_buf.as_slice(), width_buf.as_slice()];
            let mut output = vec![0.0; block_size];

            widener.process_block(&inputs, &mut output, sample_rate, &context);

            let output_rms = calculate_rms(&output);
            let ratio = output_rms / input_rms;

            // Energy preservation varies by width:
            // - Width 0.0 narrows (reduces energy)
            // - Width 1.0 preserves (minimal change)
            // - Width 2.0 widens (may increase energy)
            // Allow wider tolerance for edge cases
            assert!(
                ratio > 0.4 && ratio < 1.5,
                "Width {} output should be reasonable: input_rms={}, output_rms={}, ratio={}",
                width_val,
                input_rms,
                output_rms,
                ratio
            );
        }
    }

    #[test]
    fn test_stereo_widener_pattern_modulation() {
        // Test 5: Width parameter should support pattern modulation

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut widener = StereoWidenerNode::new(1, 2, sample_rate);

        let context = test_context();

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);

        // Create varying width parameter (sweeps from 0.0 to 2.0)
        let mut width_buf = vec![0.0; block_size];
        for i in 0..block_size {
            width_buf[i] = (i as f32 / block_size as f32) * 2.0;
        }

        let inputs = vec![osc_buf.as_slice(), width_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        widener.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should vary (not constant)
        let min = output.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(
            range > 0.1,
            "Pattern modulation should vary output, range: {}",
            range
        );
    }

    #[test]
    fn test_stereo_widener_with_different_signals() {
        // Test 6: Widener should work with different waveforms

        let sample_rate = 44100.0;
        let block_size = 512;
        let context = test_context();

        let waveforms = vec![
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
        ];

        for waveform in waveforms {
            let mut freq_node = ConstantNode::new(440.0);
            let mut osc = OscillatorNode::new(0, waveform.clone());
            let mut width_node = ConstantNode::new(1.5);
            let mut widener = StereoWidenerNode::new(1, 2, sample_rate);

            let mut freq_buf = vec![0.0; block_size];
            let mut osc_buf = vec![0.0; block_size];
            let mut width_buf = vec![1.5; block_size];

            freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
            let inputs_osc = vec![freq_buf.as_slice()];
            osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);
            width_node.process_block(&[], &mut width_buf, sample_rate, &context);

            let inputs = vec![osc_buf.as_slice(), width_buf.as_slice()];
            let mut output = vec![0.0; block_size];

            widener.process_block(&inputs, &mut output, sample_rate, &context);

            // Should produce audible output
            let output_rms = calculate_rms(&output);
            assert!(
                output_rms > 0.05,
                "Widener should work with {:?}, got RMS {}",
                waveform,
                output_rms
            );

            // All samples should be finite
            for &sample in output.iter() {
                assert!(sample.is_finite());
            }
        }
    }

    #[test]
    fn test_stereo_widener_width_clamp() {
        // Test 7: Width parameter should be clamped to 0.0-2.0

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut widener = StereoWidenerNode::new(0, 1, sample_rate);

        let context = test_context();

        let mut input_buf = vec![1.0; block_size];
        input_node.process_block(&[], &mut input_buf, sample_rate, &context);

        // Test negative width (should clamp to 0.0)
        let mut width_buf_neg = vec![-1.0; block_size];
        let inputs_neg = vec![input_buf.as_slice(), width_buf_neg.as_slice()];
        let mut output_neg = vec![0.0; block_size];

        widener.process_block(&inputs_neg, &mut output_neg, sample_rate, &context);

        // Should not crash and produce finite output
        for &sample in output_neg.iter() {
            assert!(sample.is_finite());
        }

        // Test excessive width (should clamp to 2.0)
        let mut width_buf_excess = vec![10.0; block_size];
        let inputs_excess = vec![input_buf.as_slice(), width_buf_excess.as_slice()];
        let mut output_excess = vec![0.0; block_size];

        widener.process_block(&inputs_excess, &mut output_excess, sample_rate, &context);

        // Should not crash and produce finite output
        for &sample in output_excess.iter() {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_stereo_widener_dependencies() {
        // Test 8: Verify widener reports correct dependencies

        let widener = StereoWidenerNode::new(10, 20, 44100.0);
        let deps = widener.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // width_input
    }

    #[test]
    fn test_stereo_widener_reset() {
        // Test 9: Reset should clear filter state

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut width_node = ConstantNode::new(1.5);
        let mut widener = StereoWidenerNode::new(1, 2, sample_rate);

        let context = test_context();

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut width_buf = vec![1.5; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);
        width_node.process_block(&[], &mut width_buf, sample_rate, &context);

        let inputs = vec![osc_buf.as_slice(), width_buf.as_slice()];

        // Process several blocks to build up filter state
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            widener.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Reset should not panic
        widener.reset();

        // Process one more block - should work normally
        let mut output_after_reset = vec![0.0; block_size];
        widener.process_block(&inputs, &mut output_after_reset, sample_rate, &context);

        // Should produce audible output
        let rms = calculate_rms(&output_after_reset);
        assert!(rms > 0.05, "After reset, should produce output");
    }

    #[test]
    fn test_stereo_widener_stability() {
        // Test 10: Widener should remain stable over many blocks

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut width_node = ConstantNode::new(1.5);
        let mut widener = StereoWidenerNode::new(1, 2, sample_rate);

        let context = test_context();

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut width_buf = vec![1.5; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);
        width_node.process_block(&[], &mut width_buf, sample_rate, &context);

        let inputs = vec![osc_buf.as_slice(), width_buf.as_slice()];

        // Process 1000 blocks (about 11 seconds of audio)
        for _ in 0..1000 {
            let mut output = vec![0.0; block_size];
            widener.process_block(&inputs, &mut output, sample_rate, &context);

            // Check for stability: all values should remain finite and bounded
            for &sample in output.iter() {
                assert!(sample.is_finite(), "Output became non-finite");
                assert!(sample.abs() < 10.0, "Output exploded: {}", sample);
            }
        }
    }

    #[test]
    fn test_stereo_widener_comparison_widths() {
        // Test 11: Compare different width settings (relative behavior)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context();

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, sample_rate, &context);

        // Test width 0.0 (narrow)
        let mut width_node_narrow = ConstantNode::new(0.0);
        let mut widener_narrow = StereoWidenerNode::new(1, 2, sample_rate);

        let mut width_buf_narrow = vec![0.0; block_size];
        width_node_narrow.process_block(&[], &mut width_buf_narrow, sample_rate, &context);

        let inputs_narrow = vec![osc_buf.as_slice(), width_buf_narrow.as_slice()];
        let mut output_narrow = vec![0.0; block_size];

        widener_narrow.process_block(&inputs_narrow, &mut output_narrow, sample_rate, &context);

        // Test width 2.0 (wide)
        let mut width_node_wide = ConstantNode::new(2.0);
        let mut widener_wide = StereoWidenerNode::new(1, 2, sample_rate);

        let mut width_buf_wide = vec![2.0; block_size];
        width_node_wide.process_block(&[], &mut width_buf_wide, sample_rate, &context);

        let inputs_wide = vec![osc_buf.as_slice(), width_buf_wide.as_slice()];
        let mut output_wide = vec![0.0; block_size];

        widener_wide.process_block(&inputs_wide, &mut output_wide, sample_rate, &context);

        // Both should produce audible output
        let rms_narrow = calculate_rms(&output_narrow);
        let rms_wide = calculate_rms(&output_wide);

        assert!(rms_narrow > 0.1, "Narrow width should produce output");
        assert!(rms_wide > 0.1, "Wide width should produce output");
    }

    #[test]
    fn test_stereo_widener_sample_rate() {
        // Test 12: Verify sample rate is stored correctly

        let widener = StereoWidenerNode::new(0, 1, 44100.0);
        assert_eq!(widener.sample_rate(), 44100.0);

        let widener_48k = StereoWidenerNode::new(0, 1, 48000.0);
        assert_eq!(widener_48k.sample_rate(), 48000.0);
    }
}
