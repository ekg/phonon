/// Bitcrusher node - reduces bit depth and sample rate for lo-fi digital effects
///
/// A bitcrusher creates lo-fi, retro digital artifacts by:
/// 1. **Bit depth reduction**: Reduces the number of quantization levels,
///    creating digital noise and distortion
/// 2. **Sample rate reduction**: Holds and repeats samples, creating aliasing
///    and a characteristic stepped waveform
///
/// # Algorithm
/// ```text
/// phase += rate_reduction  // Accumulate phase
/// if phase >= 1.0:
///   levels = 2^bits
///   quantized = round(input * levels) / levels  // Reduce bit depth
///   last_sample = quantized
///   phase = phase - floor(phase)
/// output = last_sample  // Hold until next sample
/// ```
///
/// # Applications
/// - Lo-fi/8-bit retro game sounds
/// - Telephone/radio degradation effects
/// - Aggressive electronic music textures
/// - Creative sound design
///
/// # Example
/// ```ignore
/// // 8-bit game boy style effect
/// let synth = OscillatorNode::new(Waveform::Saw);  // NodeId 1
/// let bits = ConstantNode::new(4.0);                // NodeId 2 (4-bit)
/// let rate = ConstantNode::new(4.0);                // NodeId 3 (1/4 sample rate)
/// let crush = BitCrushNode::new(1, 2, 3);           // NodeId 4
/// ```
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Bitcrusher state
#[derive(Debug, Clone)]
struct BitCrushState {
    phase: f32,       // Accumulated phase for sample rate reduction
    last_sample: f32, // Last quantized sample (held and repeated)
}

impl Default for BitCrushState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            last_sample: 0.0,
        }
    }
}

/// Bitcrusher node: reduces bit depth and sample rate for lo-fi effects
///
/// Combines two types of degradation:
/// - Bit depth reduction creates quantization noise
/// - Sample rate reduction creates aliasing artifacts
pub struct BitCrushNode {
    input: NodeId,
    bits_input: NodeId,        // Bit depth (1.0 to 16.0)
    sample_rate_input: NodeId, // Sample rate reduction factor (1.0 = no reduction, 64.0 = extreme)
    state: BitCrushState,
}

impl BitCrushNode {
    /// BitCrush - Reduces bit depth and sample rate for lo-fi digital effects
    ///
    /// Creates retro digital artifacts through bit depth quantization and sample rate stepping.
    /// Ideal for lo-fi, 8-bit game sounds, telephone/radio effects, and electronic textures.
    ///
    /// # Parameters
    /// - `input`: Signal to process
    /// - `bits_input`: Bit depth (1-16, 16=CD quality, 4=lo-fi, 1=extreme)
    /// - `sample_rate_input`: Sample rate reduction factor (1-64, 1=full, 64=extreme)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 220
    /// ~crushed: ~signal # bitcrush 4 4
    /// ```
    pub fn new(input: NodeId, bits_input: NodeId, sample_rate_input: NodeId) -> Self {
        Self {
            input,
            bits_input,
            sample_rate_input,
            state: BitCrushState::default(),
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the bits input node ID
    pub fn bits_input(&self) -> NodeId {
        self.bits_input
    }

    /// Get the sample rate input node ID
    pub fn sample_rate_input(&self) -> NodeId {
        self.sample_rate_input
    }

    /// Reset bitcrusher state
    pub fn reset(&mut self) {
        self.state = BitCrushState::default();
    }

    /// Get current phase (for debugging/testing)
    pub fn phase(&self) -> f32 {
        self.state.phase
    }

    /// Get last sample (for debugging/testing)
    pub fn last_sample(&self) -> f32 {
        self.state.last_sample
    }
}

impl AudioNode for BitCrushNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "BitCrushNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let bits_buf = inputs[1];
        let rate_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let sample = input_buf[i];
            let bit_depth = bits_buf[i].clamp(1.0, 16.0);
            let rate_reduction = rate_buf[i].clamp(1.0, 64.0);

            // Accumulate phase for sample rate reduction
            // rate_reduction is the reduction factor (e.g., 4.0 = 1/4 sample rate)
            // So we increment phase by 1/rate_reduction to cross 1.0 every rate_reduction samples
            self.state.phase += 1.0 / rate_reduction;

            // Sample rate reduction: only update sample when phase crosses 1.0
            if self.state.phase >= 1.0 {
                // Bit depth reduction
                let levels = 2.0_f32.powf(bit_depth);
                let quantized = (sample * levels).round() / levels;

                // Store quantized sample
                self.state.last_sample = quantized;

                // Reset phase (keep fractional part for accuracy)
                self.state.phase = self.state.phase - self.state.phase.floor();
            }

            // Output held sample (creates sample rate reduction effect)
            output[i] = self.state.last_sample;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.bits_input, self.sample_rate_input]
    }

    fn name(&self) -> &str {
        "BitCrushNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
    }

    #[test]
    fn test_bitcrush_no_reduction() {
        // Test that max settings pass signal through (nearly) unchanged
        let size = 512;

        let input = vec![0.5; size];
        let bits = vec![16.0; size]; // Full bit depth
        let rate = vec![1.0; size]; // No sample rate reduction

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be very close to input
        // (slight quantization from 16-bit is acceptable)
        for i in 0..size {
            assert!(
                (output[i] - input[i]).abs() < 0.01,
                "With max settings, output should approximate input"
            );
        }
    }

    #[test]
    fn test_bitcrush_bit_depth_reduction() {
        // Test that low bit depth quantizes signal
        let size = 512;

        // Smooth ramp from 0.0 to 1.0
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = i as f32 / size as f32;
        }

        let bits = vec![2.0; size]; // 2-bit (only 4 levels: 0, 0.33, 0.67, 1.0)
        let rate = vec![1.0; size]; // No sample rate reduction

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // Count unique output values (should be ~4 for 2-bit)
        let mut unique_values: Vec<f32> = output.clone();
        unique_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        unique_values.dedup_by(|a, b| (*a - *b).abs() < 0.01);

        assert!(
            unique_values.len() <= 6,
            "2-bit should have ~4 unique values, got {}",
            unique_values.len()
        );
    }

    #[test]
    fn test_bitcrush_sample_rate_reduction() {
        // Test that sample rate reduction holds and repeats samples
        let size = 512;

        // Ramp input
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = i as f32 / size as f32;
        }

        let bits = vec![16.0; size]; // Full bit depth
        let rate = vec![4.0; size]; // 1/4 sample rate

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should have repeated values (sample-and-hold effect)
        let mut repeat_count = 0;
        for i in 1..size {
            if (output[i] - output[i - 1]).abs() < 0.0001 {
                repeat_count += 1;
            }
        }

        assert!(
            repeat_count > size / 2,
            "Sample rate reduction should create repeated samples, got {} repeats out of {}",
            repeat_count,
            size
        );
    }

    #[test]
    fn test_bitcrush_combined_effect() {
        // Test combined bit depth and sample rate reduction
        let size = 512;

        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = 0.5 * ((i as f32 * 0.1).sin());
        }

        let bits = vec![4.0; size]; // 4-bit (game boy)
        let rate = vec![8.0; size]; // 1/8 sample rate

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be quantized and stepped
        // Check for quantization (limited unique values)
        let mut unique_values: Vec<f32> = output.clone();
        unique_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        unique_values.dedup_by(|a, b| (*a - *b).abs() < 0.001);

        assert!(
            unique_values.len() < 30,
            "4-bit should have limited unique values, got {}",
            unique_values.len()
        );
    }

    #[test]
    fn test_bitcrush_extreme_reduction() {
        // Test extreme settings (1-bit, very low sample rate)
        let size = 512;

        let input = vec![0.5; size];
        let bits = vec![1.0; size]; // 1-bit (only 0 and 1)
        let rate = vec![64.0; size]; // 1/64 sample rate

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // 1-bit output should be either 0.0 or 1.0 (or 0.5)
        for &val in &output {
            assert!(
                (val - 0.0).abs() < 0.1 || (val - 0.5).abs() < 0.1 || (val - 1.0).abs() < 0.1,
                "1-bit output should be quantized to 0/0.5/1, got {}",
                val
            );
        }
    }

    #[test]
    fn test_bitcrush_phase_accumulation() {
        // Test that phase accumulates correctly over time
        let size = 128;

        // Create varying input so we can see phase accumulation effects
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = (i as f32 / size as f32) * 2.0 - 1.0; // Ramp -1 to 1
        }

        let bits = vec![16.0; size];
        let rate = vec![2.0; size]; // Phase increments by 1/2.0 = 0.5 per sample

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // With rate=2.0, phase accumulates by 0.5 per sample
        // So phase crosses 1.0 every 2 samples, causing ~64 updates in 128 samples
        let mut changes = 0;
        for i in 1..size {
            if (output[i] - output[i - 1]).abs() > 0.0001 {
                changes += 1;
            }
        }

        // With rate=2.0, we expect phase to wrap frequently (every 2 samples)
        // With a ramp input, this creates noticeable changes
        assert!(
            changes > size / 4,
            "Phase should accumulate and cross 1.0 frequently, got {} changes",
            changes
        );
    }

    #[test]
    fn test_bitcrush_varying_parameters() {
        // Test that parameters can vary over time
        let size = 512;

        // Create a ramp input so we can see quantization effects
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = (i as f32 / size as f32) * 2.0 - 1.0; // Ramp from -1 to 1
        }

        // Vary bit depth over time
        let mut bits = vec![0.0; size];
        for i in 0..size {
            bits[i] = 2.0 + (i as f32 / size as f32) * 14.0; // Ramp from 2-bit to 16-bit
        }

        let rate = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should vary as bit depth changes
        // Early samples (low bits) should be more quantized
        let early_vals: Vec<f32> = output[0..size / 4].iter().copied().collect();
        let late_vals: Vec<f32> = output[3 * size / 4..].iter().copied().collect();

        let mut early_unique = early_vals.clone();
        early_unique.sort_by(|a, b| a.partial_cmp(b).unwrap());
        early_unique.dedup_by(|a, b| (*a - *b).abs() < 0.001);

        let mut late_unique = late_vals.clone();
        late_unique.sort_by(|a, b| a.partial_cmp(b).unwrap());
        late_unique.dedup_by(|a, b| (*a - *b).abs() < 0.001);

        // Early samples (lower bits) should have fewer unique values than late (higher bits)
        // 2-bit can have max ~4 unique values, 16-bit can have ~65536
        assert!(
            early_unique.len() < late_unique.len(),
            "Lower bit depth should create fewer unique values: early={}, late={}",
            early_unique.len(),
            late_unique.len()
        );
    }

    #[test]
    fn test_bitcrush_node_interface() {
        // Test node getters
        let crush = BitCrushNode::new(15, 16, 17);

        assert_eq!(crush.input(), 15);
        assert_eq!(crush.bits_input(), 16);
        assert_eq!(crush.sample_rate_input(), 17);

        let inputs = crush.input_nodes();
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0], 15);
        assert_eq!(inputs[1], 16);
        assert_eq!(inputs[2], 17);

        assert_eq!(crush.name(), "BitCrushNode");
    }

    #[test]
    fn test_bitcrush_reset() {
        // Test that reset clears state
        // With rate=4.0: phase += 1/4.0 = 0.25 per sample
        // After 99 samples: phase = 99 * 0.25 = 24.75, which wraps to 0.75 (24.75 - 24 = 0.75)
        let size = 99;

        let input = vec![0.8; size];
        let bits = vec![8.0; size];
        let rate = vec![4.0; size];
        let inputs: Vec<&[f32]> = vec![&input, &bits, &rate];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut crush = BitCrushNode::new(0, 1, 2);

        // Process to build up state
        crush.process_block(&inputs, &mut output, 44100.0, &context);

        let phase_before = crush.phase();
        let sample_before = crush.last_sample();
        assert!(
            phase_before > 0.0,
            "Phase should be non-zero after processing"
        );
        assert!(sample_before.abs() > 0.0, "Last sample should be non-zero");

        // Reset
        crush.reset();
        assert_eq!(crush.phase(), 0.0, "Phase should be cleared");
        assert_eq!(crush.last_sample(), 0.0, "Last sample should be cleared");
    }
}
