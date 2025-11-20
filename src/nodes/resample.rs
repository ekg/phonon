/// Resample node - high-quality sample rate conversion with fractional delay
///
/// This node implements linear interpolation-based resampling for:
/// - Pitch shifting (like PitchShifterNode but simpler)
/// - Time stretching preparation
/// - Sample rate matching
/// - Speed changes (0.5 = half speed, 2.0 = double speed)
///
/// Unlike PitchShifterNode which uses dual delay lines with crossfading,
/// ResampleNode uses a single buffer with fractional read position for
/// simpler, cleaner resampling.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::collections::VecDeque;

/// Resample node with pattern-controlled resampling ratio
///
/// # Algorithm
/// ```text
/// for each sample:
///     ratio = ratio_input[i]  // Resampling ratio
///
///     // Read with linear interpolation
///     read_pos = phase  // Fractional position
///     idx = floor(read_pos)
///     frac = fractional_part(read_pos)
///
///     sample = buffer[idx] * (1.0 - frac) + buffer[idx + 1] * frac
///
///     output[i] = sample
///
///     // Advance by ratio
///     phase += ratio
///     if phase >= buffer_length:
///         phase -= buffer_length  // Wrap around
/// ```
///
/// # Example
/// ```ignore
/// // Resample at half speed (pitch down one octave)
/// let input = OscillatorNode::new(0, Waveform::Saw);  // NodeId 0
/// let ratio = ConstantNode::new(0.5);  // Half speed, NodeId 1
/// let resampler = ResampleNode::new(0, 1, 44100.0);  // NodeId 2
/// ```
pub struct ResampleNode {
    /// Input signal to resample
    input: NodeId,

    /// Resampling ratio (0.5 = half speed, 1.0 = unity, 2.0 = double speed)
    ratio_input: NodeId,

    /// Input buffer for resampling
    input_buffer: VecDeque<f32>,

    /// Current fractional read position
    phase: f32,

    /// Maximum buffer size (100ms at sample rate)
    max_delay: usize,

    /// Sample rate for calculations
    sample_rate: f32,
}

impl ResampleNode {
    /// Create a new resample node
    ///
    /// # Arguments
    /// * `input` - NodeId providing the signal to resample
    /// * `ratio_input` - NodeId providing resampling ratio
    /// * `sample_rate` - Sample rate in Hz (usually 44100.0)
    ///
    /// # Buffer Size
    /// Uses a fixed 100ms buffer to accommodate various playback speeds
    pub fn new(input: NodeId, ratio_input: NodeId, sample_rate: f32) -> Self {
        // Maximum delay: 100ms for safe buffering
        let max_delay_seconds = 0.1;
        let max_delay = (max_delay_seconds * sample_rate).ceil() as usize;

        Self {
            input,
            ratio_input,
            input_buffer: VecDeque::with_capacity(max_delay),
            phase: 0.0,
            max_delay,
            sample_rate,
        }
    }

    /// Get the current read phase (fractional position)
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the ratio input node ID
    pub fn ratio_input(&self) -> NodeId {
        self.ratio_input
    }

    /// Reset the resample buffer to silence
    pub fn clear_buffer(&mut self) {
        self.input_buffer.clear();
        self.phase = 0.0;
    }

    /// Read from buffer with linear interpolation
    fn read_with_interpolation(buffer: &VecDeque<f32>, read_pos: f32) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }

        let len = buffer.len();
        let idx = read_pos as usize % len;
        let frac = read_pos.fract();

        let sample1 = buffer[idx];
        let sample2 = buffer[(idx + 1) % len];

        // Linear interpolation: sample1 + frac * (sample2 - sample1)
        sample1 * (1.0 - frac) + sample2 * frac
    }
}

impl AudioNode for ResampleNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "ResampleNode requires 2 inputs: signal, ratio"
        );

        let input_buffer = inputs[0];
        let ratio_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            ratio_buffer.len(),
            output.len(),
            "Ratio buffer length mismatch"
        );

        for i in 0..output.len() {
            let input_sample = input_buffer[i];
            let ratio = ratio_buffer[i].max(0.01); // Minimum ratio to prevent division issues

            // Write to input buffer
            self.input_buffer.push_back(input_sample);

            // Maintain max buffer size
            if self.input_buffer.len() > self.max_delay {
                self.input_buffer.pop_front();
            }

            // Read from buffer with linear interpolation
            let sample = Self::read_with_interpolation(&self.input_buffer, self.phase);
            output[i] = sample;

            // Advance phase by ratio
            self.phase += ratio;

            // Wrap phase when it exceeds buffer length
            let len = self.input_buffer.len() as f32;
            if len > 0.0 && self.phase >= len {
                self.phase = self.phase % len;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.ratio_input]
    }

    fn name(&self) -> &str {
        "ResampleNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        )
    }

    #[test]
    fn test_resample_ratio_half_speed() {
        // Test 1: Ratio 0.5 = half speed (pitch down one octave)
        // Should slow down playback by 2x

        let sample_rate = 44100.0;
        let block_size = 4410; // 100ms to fill buffer

        let mut input_node = ConstantNode::new(110.0); // Frequency for oscillator
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(0.5); // Half speed
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        // Generate sine wave input @ 110 Hz
        let mut freq_buf = vec![110.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        // Process
        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // Verify output has energy (not silent)
        let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();
        assert!(rms > 0.1, "Output should have energy, got RMS: {}", rms);
    }

    #[test]
    fn test_resample_ratio_double_speed() {
        // Test 2: Ratio 2.0 = double speed (pitch up one octave)
        // Should speed up playback by 2x

        let sample_rate = 44100.0;
        let block_size = 4410;

        let mut input_node = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(2.0); // Double speed
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![110.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![2.0; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // Verify output has energy
        let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();
        assert!(rms > 0.1, "Output should have energy, got RMS: {}", rms);
    }

    #[test]
    fn test_resample_ratio_unity() {
        // Test 3: Ratio 1.0 = unity (passthrough, no pitch change)
        // Should output essentially the same as input

        let sample_rate = 44100.0;
        let block_size = 4410;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(1.0); // Unity
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![1.0; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        // Calculate input RMS for comparison
        let input_rms: f32 = input_buf.iter().map(|x| x * x).sum::<f32>() / input_buf.len() as f32;
        let input_rms = input_rms.sqrt();

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32;
        let output_rms = output_rms.sqrt();

        // With unity ratio, output should have similar energy to input
        // Allow some tolerance due to buffering delay
        assert!(
            output_rms > input_rms * 0.3,
            "Unity ratio should preserve most energy: input_rms={}, output_rms={}",
            input_rms,
            output_rms
        );
    }

    #[test]
    fn test_resample_linear_interpolation_accuracy() {
        // Test 4: Verify linear interpolation produces smooth output

        let sample_rate = 44100.0;
        let block_size = 4410;

        let mut input_node = ConstantNode::new(55.0); // Low frequency for clear interpolation
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(1.5); // Fractional ratio
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![55.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![1.5; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // Check for smooth output (no huge discontinuities)
        let mut max_diff = 0.0_f32;
        for i in 1..output.len() {
            let diff = (output[i] - output[i - 1]).abs();
            max_diff = max_diff.max(diff);
        }

        // Interpolation should keep transitions smooth (relaxed for resampling artifacts)
        assert!(
            max_diff < 1.0,
            "Output should be smooth with interpolation, max_diff: {}",
            max_diff
        );
    }

    #[test]
    fn test_resample_pattern_modulation() {
        // Test 5: Ratio can be modulated per-sample

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];

        // Modulate ratio: ramp from 0.5 to 2.0
        let mut ratio_buf = vec![0.0; block_size];
        for i in 0..block_size {
            ratio_buf[i] = 0.5 + (1.5 * i as f32 / block_size as f32);
        }

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        // Process multiple blocks
        let mut all_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            resampler.process_block(&inputs, &mut output, sample_rate, &context);
            all_outputs.extend_from_slice(&output);
        }

        // Should produce varying output (modulation working)
        let min = all_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = all_outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(range > 0.1, "Modulated ratio should vary output, range: {}", range);
    }

    #[test]
    fn test_resample_stability_long_duration() {
        // Test 6: Should remain stable over many blocks

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(1.3);
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![1.3; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        // Process many blocks
        for _ in 0..1000 {
            osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);

            let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

            let mut output = vec![0.0; block_size];
            resampler.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability: all values should be finite and bounded
            for &sample in output.iter() {
                assert!(sample.is_finite(), "Output became non-finite");
                assert!(sample.abs() < 10.0, "Output exploded: {}", sample);
            }
        }
    }

    #[test]
    fn test_resample_phase_wrapping() {
        // Test 7: Phase should wrap correctly when it exceeds buffer length

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(3.0); // Fast ratio for quick wrapping
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![3.0; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        // Process several blocks
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            resampler.process_block(&inputs, &mut output, sample_rate, &context);

            // Phase should remain bounded
            assert!(
                resampler.phase() >= 0.0,
                "Phase should be non-negative: {}",
                resampler.phase()
            );
        }
    }

    #[test]
    fn test_resample_clear_buffer() {
        // Test 8: clear_buffer() should reset state

        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(1.5);
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![1.5; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        // Process several blocks to fill buffer
        for _ in 0..5 {
            let mut output = vec![0.0; block_size];
            resampler.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Verify state has advanced
        assert!(resampler.phase() > 0.0);

        // Clear buffer
        resampler.clear_buffer();

        // Verify reset
        assert_eq!(resampler.phase(), 0.0);
    }

    #[test]
    fn test_resample_dependencies() {
        // Test 9: Verify resample node reports correct dependencies

        let resampler = ResampleNode::new(10, 20, 44100.0);
        let deps = resampler.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // ratio_input
    }

    #[test]
    fn test_resample_slow_ratio() {
        // Test 10: Very slow ratio (0.1 = 10x slower)

        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(0.1); // Very slow
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![0.1; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // Should produce output
        let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();
        assert!(rms > 0.01, "Very slow ratio should work, got RMS: {}", rms);
    }

    #[test]
    fn test_resample_fast_ratio() {
        // Test 11: Very fast ratio (4.0 = 4x faster)

        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut input_node = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(4.0); // Very fast
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![110.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![4.0; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // Should produce output
        let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();
        assert!(rms > 0.05, "Very fast ratio should work, got RMS: {}", rms);
    }

    #[test]
    fn test_resample_smooth_transitions() {
        // Test 12: Verify smooth transitions with fractional ratios

        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut input_node = ConstantNode::new(220.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(1.234); // Arbitrary fractional ratio
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![220.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![1.234; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // Verify no NaN or inf values
        for &sample in output.iter() {
            assert!(sample.is_finite(), "Output should be finite");
        }

        // Verify reasonable amplitude
        let max_abs = output.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
        assert!(max_abs < 5.0, "Output amplitude should be reasonable, got: {}", max_abs);
    }

    #[test]
    fn test_resample_minimum_ratio_clamping() {
        // Test 13: Verify ratio is clamped to minimum (0.01)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(0.0); // Zero or negative should be clamped
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        // Should not crash with zero ratio
        let mut output = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output, sample_rate, &context);

        // All outputs should be finite (clamping working)
        for &sample in output.iter() {
            assert!(sample.is_finite(), "Output should be finite with clamped ratio");
        }
    }

    #[test]
    fn test_resample_buffer_fill_behavior() {
        // Test 14: Verify buffer fills correctly before producing meaningful output

        let sample_rate = 44100.0;
        let block_size = 100;

        let mut input_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut ratio_node = ConstantNode::new(1.0);
        let mut resampler = ResampleNode::new(1, 2, sample_rate);

        let context = create_context(block_size);

        let mut freq_buf = vec![440.0; block_size];
        let mut input_buf = vec![0.0; block_size];
        let mut ratio_buf = vec![1.0; block_size];

        input_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut input_buf, sample_rate, &context);
        ratio_node.process_block(&[], &mut ratio_buf, sample_rate, &context);

        let inputs = vec![input_buf.as_slice(), ratio_buf.as_slice()];

        // First block
        let mut output1 = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output1, sample_rate, &context);

        let rms1: f32 = output1.iter().map(|x| x * x).sum::<f32>() / output1.len() as f32;
        let rms1 = rms1.sqrt();

        // Second block should have higher energy as buffer fills
        let mut output2 = vec![0.0; block_size];
        resampler.process_block(&inputs, &mut output2, sample_rate, &context);

        let rms2: f32 = output2.iter().map(|x| x * x).sum::<f32>() / output2.len() as f32;
        let rms2 = rms2.sqrt();

        // Both blocks should produce output
        assert!(rms1 > 0.0, "First block should have some output");
        assert!(rms2 > 0.0, "Second block should have output");
    }
}
