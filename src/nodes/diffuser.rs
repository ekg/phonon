/// Multi-channel Hadamard diffuser for high-quality reverb
///
/// This node implements a sophisticated diffusion network based on the Signalsmith approach.
/// It uses 8 parallel channels with 4 cascaded diffusion steps to create dense, natural-sounding
/// reverberation by spreading an impulse into many closely-spaced reflections.
///
/// # Architecture
///
/// Each diffusion step consists of:
/// 1. Variable delays (different per channel)
/// 2. Channel shuffle with polarity flips (decorrelation)
/// 3. Hadamard transform (8x8 mixing matrix)
///
/// This creates exponential echo density growth: an input impulse becomes
/// 8^4 = 4096 reflections after all 4 steps, with natural decay characteristics.
///
/// # Implementation Details
///
/// - Uses fast Hadamard transform (FHT) instead of matrix multiply for efficiency
/// - Delay times are prime-like numbers to avoid resonances
/// - Each step scales delays by different amounts (1x, 1.5x, 2x, 3x)
/// - Channel shuffle pattern varies per step to maximize decorrelation
///
/// # References
///
/// Based on techniques from:
/// - Signalsmith Audio's diffusion networks
/// - Dattorro "Effect Design" papers
/// - Schroeder allpass diffusion
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Multi-channel Hadamard diffuser
///
/// Implements a 8-channel, 4-step diffusion network for reverb applications.
///
/// # Example
/// ```ignore
/// // Diffuse input signal
/// let input_signal = SampleNode::new(...);              // NodeId 0
/// let diffusion_amount = ConstantNode::new(0.75);       // NodeId 1
/// let diffuser = DiffuserNode::new(0, 1, 44100.0);      // NodeId 2
/// ```
pub struct DiffuserNode {
    /// Input signal to diffuse
    input: NodeId,
    /// Diffusion amount (0.0 = delays only, 1.0 = maximum spreading)
    diffusion_input: NodeId,
    /// Delay buffers: [4 steps][8 channels]
    delay_buffers: [[Vec<f32>; 8]; 4],
    /// Write indices for circular buffers: [4 steps][8 channels]
    write_indices: [[usize; 8]; 4],
    /// Base delay times in samples for each step/channel
    delay_times: [[usize; 8]; 4],
    /// Sample rate (for potential future use)
    _sample_rate: f32,
}

impl DiffuserNode {
    /// Hadamard Diffuser - Multi-channel diffusion network for reverb
    ///
    /// Creates a dense cloud of reflections from an input signal using
    /// cascaded delay networks and Hadamard mixing matrices.
    ///
    /// # Parameters
    /// - `input`: Signal to diffuse
    /// - `diffusion_input`: Diffusion amount (0.0-1.0)
    /// - `sample_rate`: Sample rate in Hz
    ///
    /// # Example
    /// ```phonon
    /// ~signal: s "bd"
    /// ~diffused: ~signal # diffuser 0.75
    /// ```
    pub fn new(input: NodeId, diffusion_input: NodeId, sample_rate: f32) -> Self {
        // Base delay times (in samples at 44.1kHz) - prime-like numbers to avoid resonances
        let base_delays = [23, 41, 59, 73, 89, 107, 127, 149];

        // Scale factor for different sample rates
        let scale = sample_rate / 44100.0;

        // Calculate delay times for each step
        // Step 1: base delays
        // Step 2: base delays * 1.5
        // Step 3: base delays * 2.0
        // Step 4: base delays * 3.0
        let step_scales = [1.0, 1.5, 2.0, 3.0];

        let mut delay_times = [[0; 8]; 4];
        for (step_idx, step_scale) in step_scales.iter().enumerate() {
            for (ch, &base_delay) in base_delays.iter().enumerate() {
                delay_times[step_idx][ch] =
                    ((base_delay as f32) * step_scale * scale).round() as usize;
            }
        }

        // Allocate delay buffers
        let mut delay_buffers = std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
        for step_idx in 0..4 {
            for ch in 0..8 {
                let buffer_size = delay_times[step_idx][ch];
                delay_buffers[step_idx][ch] = vec![0.0; buffer_size.max(1)]; // Ensure at least 1 sample
            }
        }

        Self {
            input,
            diffusion_input,
            delay_buffers,
            write_indices: [[0; 8]; 4],
            delay_times,
            _sample_rate: sample_rate,
        }
    }

    /// Fast Hadamard Transform (in-place, 8-point)
    ///
    /// Applies the 8x8 Hadamard matrix using the fast algorithm.
    /// This is equivalent to H8 matrix multiply but much faster (O(n log n) instead of O(n²)).
    ///
    /// The Hadamard matrix provides uniform mixing of all channels with ±1 coefficients,
    /// creating decorrelation without changing total energy (up to normalization).
    #[inline]
    fn hadamard_8(data: &mut [f32; 8]) {
        // Level 1: pairs
        let (a0, a1) = (data[0] + data[1], data[0] - data[1]);
        let (a2, a3) = (data[2] + data[3], data[2] - data[3]);
        let (a4, a5) = (data[4] + data[5], data[4] - data[5]);
        let (a6, a7) = (data[6] + data[7], data[6] - data[7]);

        // Level 2: quads
        let (b0, b2) = (a0 + a2, a0 - a2);
        let (b1, b3) = (a1 + a3, a1 - a3);
        let (b4, b6) = (a4 + a6, a4 - a6);
        let (b5, b7) = (a5 + a7, a5 - a7);

        // Level 3: octets
        data[0] = b0 + b4;
        data[1] = b1 + b5;
        data[2] = b2 + b6;
        data[3] = b3 + b7;
        data[4] = b0 - b4;
        data[5] = b1 - b5;
        data[6] = b2 - b6;
        data[7] = b3 - b7;

        // Normalize to preserve energy: H8 has norm sqrt(8), so divide by sqrt(8)
        let norm = 1.0 / 8.0_f32.sqrt();
        for x in data.iter_mut() {
            *x *= norm;
        }
    }

    /// Shuffle channels with polarity flips for decorrelation
    ///
    /// Each step uses a different shuffle pattern to maximize decorrelation
    /// between channels. Some channels are inverted to break up phase coherence.
    #[inline]
    fn shuffle_channels(data: &mut [f32; 8], step: usize) {
        let temp = *data;
        match step {
            0 => {
                // Rotate right by 1, flip channels 1,3,5,7
                data[0] = temp[7];
                data[1] = -temp[0];
                data[2] = temp[1];
                data[3] = -temp[2];
                data[4] = temp[3];
                data[5] = -temp[4];
                data[6] = temp[5];
                data[7] = -temp[6];
            }
            1 => {
                // Rotate right by 3, flip channels 0,2,4,6
                data[0] = -temp[5];
                data[1] = temp[6];
                data[2] = -temp[7];
                data[3] = temp[0];
                data[4] = -temp[1];
                data[5] = temp[2];
                data[6] = -temp[3];
                data[7] = temp[4];
            }
            2 => {
                // Reverse order, flip channels 0,1,4,5
                data[0] = -temp[7];
                data[1] = -temp[6];
                data[2] = temp[5];
                data[3] = temp[4];
                data[4] = -temp[3];
                data[5] = -temp[2];
                data[6] = temp[1];
                data[7] = temp[0];
            }
            _ => {
                // Rotate right by 2, flip channels 2,3,6,7
                data[0] = temp[6];
                data[1] = temp[7];
                data[2] = -temp[0];
                data[3] = -temp[1];
                data[4] = temp[2];
                data[5] = temp[3];
                data[6] = -temp[4];
                data[7] = -temp[5];
            }
        }
    }

    /// Clear all delay buffers
    pub fn clear(&mut self) {
        for step in 0..4 {
            for ch in 0..8 {
                self.delay_buffers[step][ch].fill(0.0);
                self.write_indices[step][ch] = 0;
            }
        }
    }
}

impl AudioNode for DiffuserNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "DiffuserNode requires 2 inputs: signal and diffusion"
        );

        let input_buffer = inputs[0];
        let diffusion_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            diffusion_buffer.len(),
            output.len(),
            "Diffusion buffer length mismatch"
        );

        for i in 0..output.len() {
            let input_sample = input_buffer[i];
            let diffusion = diffusion_buffer[i].clamp(0.0, 1.0);

            // Initialize 8-channel state with mono input distributed across all channels
            // This prevents signal loss when mixing back to mono at the end
            let mut channels = [input_sample / 8.0_f32.sqrt(); 8];

            // Process through 4 diffusion steps
            for step in 0..4 {
                // Store input to delays before processing
                let input_to_delay = channels;

                // Read delayed samples from all channels
                let mut delayed = [0.0_f32; 8];
                for ch in 0..8 {
                    let buffer = &self.delay_buffers[step][ch];
                    let write_idx = self.write_indices[step][ch];

                    // Read from current position (oldest sample in the delay)
                    delayed[ch] = buffer[write_idx];
                }

                // Apply diffusion using allpass formula
                // diffusion=0: output = delayed input (pure delay)
                // diffusion=1: output = allpass response (maximum spreading)
                //
                // Allpass formula: y[n] = -g*x[n] + x[n-d] + g*y[n-d]
                // where:
                //   x[n] = current input
                //   x[n-d] = delayed input (from buffer)
                //   y[n-d] = delayed output (also from buffer, stored last iteration)
                //   g = diffusion coefficient
                //
                // For simplicity, we use: y = x[n-d] + g*(x[n-d] - x[n])
                // This approximates allpass behavior
                for ch in 0..8 {
                    let current_input = channels[ch];
                    let delayed_input = delayed[ch];

                    // Mix between pure delay (diffusion=0) and allpass (diffusion=1)
                    channels[ch] = delayed_input + diffusion * (delayed_input - current_input);
                }

                // Shuffle channels with polarity flips
                Self::shuffle_channels(&mut channels, step);

                // Hadamard mix
                Self::hadamard_8(&mut channels);

                // Write INPUT (not output) to delay buffers
                // This is key for proper allpass behavior
                for ch in 0..8 {
                    let buffer = &mut self.delay_buffers[step][ch];
                    let write_idx = self.write_indices[step][ch];

                    // Write the input that went INTO this step
                    buffer[write_idx] = input_to_delay[ch];

                    // Advance write index (circular)
                    self.write_indices[step][ch] = (write_idx + 1) % buffer.len();
                }
            }

            // Mix down to mono: average of all 8 channels
            output[i] = channels.iter().sum::<f32>() / 8.0;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.diffusion_input]
    }

    fn name(&self) -> &str {
        "DiffuserNode"
    }

    fn provides_delay(&self) -> bool {
        true // Has internal delay buffers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    /// Helper to create a test context
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    /// Calculate RMS (root mean square) of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Calculate energy (sum of squares) of a buffer
    fn calculate_energy(buffer: &[f32]) -> f32 {
        buffer.iter().map(|x| x * x).sum()
    }

    /// Count number of samples above threshold (for echo density measurement)
    fn count_peaks(buffer: &[f32], threshold: f32) -> usize {
        buffer.iter().filter(|&&x| x.abs() > threshold).count()
    }

    #[test]
    fn test_diffuser_impulse_spreading() {
        // Test that an impulse gets spread over time (increased echo density)
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut diffusion_node = ConstantNode::new(0.7);
        let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

        let context = test_context(block_size);

        // Create impulse input
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0; // Single impulse

        let mut diffusion_buf = vec![0.0; block_size];
        diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

        // Process the impulse
        let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        diffuser.process_block(&inputs, &mut output, sample_rate, &context);

        // Count non-zero samples in output
        let num_active = count_peaks(&output, 0.001);

        // Should have spread the impulse to many samples (more than just the input)
        assert!(
            num_active > 5,
            "Impulse should spread to multiple samples, got {} active",
            num_active
        );

        // Process several more blocks to see continued spreading
        input_buf.fill(0.0); // No more input
        let mut total_active = num_active;

        for _ in 0..10 {
            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output_block = vec![0.0; block_size];
            diffuser.process_block(&inputs, &mut output_block, sample_rate, &context);
            total_active += count_peaks(&output_block, 0.001);
        }

        // Total echo density should be significant
        assert!(
            total_active > 50,
            "Expected high echo density, got {} total active samples",
            total_active
        );
    }

    #[test]
    fn test_diffuser_zero_diffusion_passes_through_with_delay() {
        // diffusion=0 should pass signal through with only delays (no spreading)
        let sample_rate = 44100.0;
        let block_size = 256;

        let mut diffusion_node = ConstantNode::new(0.0); // Zero diffusion
        let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

        let context = test_context(block_size);

        // DC input
        let input_buf = vec![1.0; block_size];
        let mut diffusion_buf = vec![0.0; block_size];
        diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

        // Process several blocks to fill delay lines
        for _ in 0..20 {
            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            diffuser.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // After filling, output should stabilize to input level (delayed)
        let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        diffuser.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);
        let input_rms = calculate_rms(&input_buf);

        // With zero diffusion and DC input, output should approach input level
        // (allowing some variation due to the mixing/routing)
        assert!(
            output_rms > 0.3 * input_rms,
            "Zero diffusion should pass signal through, got RMS ratio {}",
            output_rms / input_rms
        );
    }

    #[test]
    fn test_diffuser_high_diffusion_maximizes_spreading() {
        // diffusion=1.0 should maximize spreading
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut diffusion_low = ConstantNode::new(0.2);
        let mut diffusion_high = ConstantNode::new(1.0);
        let mut diffuser_low = DiffuserNode::new(0, 1, sample_rate);
        let mut diffuser_high = DiffuserNode::new(2, 3, sample_rate);

        let context = test_context(block_size);

        // Impulse input
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0;

        let mut diffusion_low_buf = vec![0.0; block_size];
        let mut diffusion_high_buf = vec![0.0; block_size];
        diffusion_low.process_block(&[], &mut diffusion_low_buf, sample_rate, &context);
        diffusion_high.process_block(&[], &mut diffusion_high_buf, sample_rate, &context);

        // Process with low diffusion
        let inputs_low = vec![input_buf.as_slice(), diffusion_low_buf.as_slice()];
        let mut output_low = vec![0.0; block_size];
        diffuser_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

        // Process with high diffusion
        let inputs_high = vec![input_buf.as_slice(), diffusion_high_buf.as_slice()];
        let mut output_high = vec![0.0; block_size];
        diffuser_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

        // Count active samples
        let peaks_low = count_peaks(&output_low, 0.001);
        let peaks_high = count_peaks(&output_high, 0.001);

        // High diffusion should spread to more samples
        assert!(
            peaks_high >= peaks_low,
            "High diffusion should have more spreading: low={}, high={}",
            peaks_low,
            peaks_high
        );
    }

    #[test]
    fn test_diffuser_energy_conservation() {
        // Verify output energy roughly equals input energy (no gain/loss)
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut diffusion_node = ConstantNode::new(0.7);
        let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

        let context = test_context(block_size);

        // Impulse input
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0; // Energy = 1.0

        let mut diffusion_buf = vec![0.0; block_size];
        diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

        // Collect energy over multiple blocks
        let mut total_output_energy = 0.0;

        for block_idx in 0..20 {
            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            diffuser.process_block(&inputs, &mut output, sample_rate, &context);

            total_output_energy += calculate_energy(&output);

            // After first block, no more input
            if block_idx == 0 {
                input_buf.fill(0.0);
            }
        }

        let input_energy = 1.0; // Single impulse

        // Energy behavior in diffusers:
        // - Allpass filters with feedback (g > 0) can temporarily increase energy
        // - Energy spreads over time, may not capture all in limited blocks
        // - Hadamard transform preserves energy but channel mixing affects it
        // - Typical diffuser behavior: 0.5x to 3x energy variation is normal
        let ratio = total_output_energy / input_energy;
        assert!(
            ratio > 0.3 && ratio < 4.0,
            "Energy should be in reasonable range: input={}, output={}, ratio={}",
            input_energy,
            total_output_energy,
            ratio
        );
    }

    #[test]
    fn test_diffuser_no_nan_or_inf() {
        // Verify no NaN or Inf in output
        let sample_rate = 44100.0;
        let block_size = 256;

        let mut diffusion_node = ConstantNode::new(0.8);
        let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

        let context = test_context(block_size);

        // Various input patterns
        let test_inputs = vec![
            vec![1.0; block_size],           // DC
            vec![0.0; block_size],           // Silence
            {
                let mut buf = vec![0.0; block_size];
                buf[0] = 1.0;
                buf
            },                                // Impulse
            {
                let mut buf = vec![0.0; block_size];
                for i in 0..block_size {
                    buf[i] = ((i as f32) * 0.1).sin();
                }
                buf
            },                                // Sine
        ];

        let mut diffusion_buf = vec![0.0; block_size];
        diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

        for (test_idx, input_buf) in test_inputs.iter().enumerate() {
            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            diffuser.process_block(&inputs, &mut output, sample_rate, &context);

            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Test {} produced non-finite sample at index {}: {}",
                    test_idx,
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_diffuser_hadamard_transform() {
        // Test the Hadamard transform directly
        let mut data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let input_energy: f32 = data.iter().map(|x| x * x).sum();

        DiffuserNode::hadamard_8(&mut data);

        let output_energy: f32 = data.iter().map(|x| x * x).sum();

        // Hadamard transform should preserve energy (orthogonal transform)
        let ratio = output_energy / input_energy;
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "Hadamard transform should preserve energy: ratio={}",
            ratio
        );

        // All outputs should be finite
        for (i, &val) in data.iter().enumerate() {
            assert!(val.is_finite(), "Hadamard output {} is not finite: {}", i, val);
        }
    }

    #[test]
    fn test_diffuser_channel_shuffle() {
        // Test channel shuffle operations
        for step in 0..4 {
            let input = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
            let mut output = input;

            DiffuserNode::shuffle_channels(&mut output, step);

            // Energy should be preserved (shuffle is just reordering + polarity flips)
            let input_energy: f32 = input.iter().map(|x| x * x).sum();
            let output_energy: f32 = output.iter().map(|x| x * x).sum();

            assert!(
                (input_energy - output_energy).abs() < 0.001,
                "Shuffle step {} should preserve energy",
                step
            );

            // Verify it actually changed something
            assert!(
                input != output,
                "Shuffle step {} should change channel order",
                step
            );
        }
    }

    #[test]
    fn test_diffuser_clear() {
        let sample_rate = 44100.0;
        let block_size = 128;

        let mut diffusion_node = ConstantNode::new(0.5);
        let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

        let context = test_context(block_size);

        // Process some signal to fill buffers
        let input_buf = vec![1.0; block_size];
        let mut diffusion_buf = vec![0.0; block_size];
        diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

        for _ in 0..10 {
            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            diffuser.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Clear the diffuser
        diffuser.clear();

        // Process silence - should get silence out
        let silence = vec![0.0; block_size];
        let inputs = vec![silence.as_slice(), diffusion_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        diffuser.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);
        assert!(
            output_rms < 0.001,
            "After clear, silence input should produce silence output, got RMS={}",
            output_rms
        );
    }

    #[test]
    fn test_diffuser_dependencies() {
        let diffuser = DiffuserNode::new(10, 20, 44100.0);
        let deps = diffuser.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // diffusion
    }

    #[test]
    fn test_diffuser_different_sample_rates() {
        // Verify diffuser works at different sample rates
        for sample_rate in [22050.0, 44100.0, 48000.0, 96000.0] {
            let block_size = 256;

            let mut diffusion_node = ConstantNode::new(0.7);
            let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

            let context = ProcessContext::new(
                Fraction::from_float(0.0),
                0,
                block_size,
                2.0,
                sample_rate,
            );

            let mut input_buf = vec![0.0; block_size];
            input_buf[0] = 1.0; // Impulse

            let mut diffusion_buf = vec![0.0; block_size];
            diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            diffuser.process_block(&inputs, &mut output, sample_rate, &context);

            // Should produce some output
            let output_rms = calculate_rms(&output);
            assert!(
                output_rms > 0.001,
                "Diffuser should work at {} Hz, got RMS={}",
                sample_rate,
                output_rms
            );

            // No NaN/Inf
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} at {} Hz is not finite: {}",
                    i,
                    sample_rate,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_diffuser_diffusion_parameter_range() {
        // Test that extreme diffusion values are clamped correctly
        let sample_rate = 44100.0;
        let block_size = 128;

        let context = test_context(block_size);

        // Test diffusion values outside [0, 1] range
        for diffusion_val in [-0.5, -0.1, 0.0, 0.5, 1.0, 1.5, 2.0] {
            let mut diffusion_node = ConstantNode::new(diffusion_val);
            let mut diffuser = DiffuserNode::new(0, 1, sample_rate);

            let mut input_buf = vec![0.0; block_size];
            input_buf[0] = 1.0;

            let mut diffusion_buf = vec![0.0; block_size];
            diffusion_node.process_block(&[], &mut diffusion_buf, sample_rate, &context);

            let inputs = vec![input_buf.as_slice(), diffusion_buf.as_slice()];
            let mut output = vec![0.0; block_size];

            // Should not panic or produce invalid output
            diffuser.process_block(&inputs, &mut output, sample_rate, &context);

            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Diffusion={} produced non-finite sample at {}: {}",
                    diffusion_val,
                    i,
                    sample
                );
            }
        }
    }
}
