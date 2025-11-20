/// Brown noise generator (Brownian/red noise)
///
/// Generates random walk values in the range [-amplitude, amplitude] using a
/// deterministic random number generator. Brown noise has a 6dB/octave rolloff,
/// giving it a warmer, smoother sound than white noise.
///
/// The algorithm uses a random walk (Brownian motion) where each sample is a
/// small random step from the previous sample, bounded to stay within [-1, 1].

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Brown noise node: generates random walk values scaled by amplitude
///
/// # Example
/// ```ignore
/// // Generate brown noise at 0.5 amplitude
/// let amplitude = ConstantNode::new(0.5);  // NodeId 0
/// let noise = BrownNoiseNode::new(0);      // NodeId 1
/// // Output will be smooth random walk values in [-0.5, 0.5]
/// ```
pub struct BrownNoiseNode {
    amplitude_input: NodeId,
    rng: StdRng,
    last_value: f32,  // Random walk accumulator
}

impl BrownNoiseNode {
    /// Create a new brown noise node with a random seed
    ///
    /// # Arguments
    /// * `amplitude_input` - NodeId of the amplitude control signal
    pub fn new(amplitude_input: NodeId) -> Self {
        Self {
            amplitude_input,
            rng: StdRng::from_entropy(),
            last_value: 0.0,
        }
    }

    /// Create a new brown noise node with a specific seed (for testing)
    ///
    /// # Arguments
    /// * `amplitude_input` - NodeId of the amplitude control signal
    /// * `seed` - Seed value for deterministic random generation
    pub fn new_with_seed(amplitude_input: NodeId, seed: u64) -> Self {
        Self {
            amplitude_input,
            rng: StdRng::seed_from_u64(seed),
            last_value: 0.0,
        }
    }

    /// Get the amplitude input node ID
    pub fn amplitude_input(&self) -> NodeId {
        self.amplitude_input
    }
}

impl AudioNode for BrownNoiseNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "BrownNoiseNode requires 1 input (amplitude), got {}",
            inputs.len()
        );

        let amplitude = inputs[0];

        debug_assert_eq!(
            amplitude.len(),
            output.len(),
            "Amplitude input length mismatch"
        );

        // Generate brown noise: random walk bounded to [-1, 1]
        for i in 0..output.len() {
            // Random walk step (small increments)
            let step = (self.rng.gen::<f32>() * 2.0 - 1.0) * 0.02;
            self.last_value += step;

            // Keep bounded to [-1, 1] with soft limiting
            if self.last_value > 1.0 {
                self.last_value = 1.0;
            } else if self.last_value < -1.0 {
                self.last_value = -1.0;
            }

            output[i] = self.last_value * amplitude[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.amplitude_input]
    }

    fn name(&self) -> &str {
        "BrownNoiseNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_brown_noise_random_walk() {
        // Brown noise should be a random walk where consecutive samples are correlated
        let mut noise = BrownNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![1.0; 1000];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 1000];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1000,
            2.0,
            44100.0,
        );

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate average step size
        let mut total_step = 0.0;
        for i in 1..output.len() {
            total_step += (output[i] - output[i - 1]).abs();
        }
        let avg_step = total_step / (output.len() - 1) as f32;

        // Average step should be small (around 0.02 or less)
        assert!(avg_step < 0.03,
            "Average step size {} too large for brown noise (should be ~0.02)", avg_step);
    }

    #[test]
    fn test_brown_noise_smoother_than_white() {
        // Brown noise should have smaller step sizes than white noise
        // due to the random walk nature (each sample depends on previous)

        let mut brown = BrownNoiseNode::new_with_seed(0, 42);
        let amplitude = vec![1.0; 1000];
        let inputs = vec![amplitude.as_slice()];
        let mut brown_output = vec![0.0; 1000];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1000,
            2.0,
            44100.0,
        );

        brown.process_block(&inputs, &mut brown_output, 44100.0, &context);

        // Calculate average absolute difference between consecutive samples
        let mut brown_diff = 0.0;
        for i in 1..brown_output.len() {
            brown_diff += (brown_output[i] - brown_output[i - 1]).abs();
        }
        brown_diff /= (brown_output.len() - 1) as f32;

        // White noise would have average diff around 1.15 (random [-1,1] to random [-1,1])
        // Brown noise should be much smaller (around 0.02)
        assert!(brown_diff < 0.1,
            "Brown noise average diff {} should be much smaller than white noise (~1.15)", brown_diff);
    }

    #[test]
    fn test_brown_noise_bounded() {
        // Output should stay bounded in [-amplitude, amplitude]
        let mut noise = BrownNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.5; 10000];  // Long run to test bounds
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 10000];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            10000,
            2.0,
            44100.0,
        );

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in [-0.5, 0.5]
        for (i, sample) in output.iter().enumerate() {
            assert!(*sample >= -0.5 && *sample <= 0.5,
                "Sample {} at index {} out of range [-0.5, 0.5]", sample, i);
        }
    }

    #[test]
    fn test_brown_noise_seed_reproducibility() {
        // Same seed should produce same output
        let mut noise1 = BrownNoiseNode::new_with_seed(0, 12345);
        let mut noise2 = BrownNoiseNode::new_with_seed(0, 12345);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output1 = vec![0.0; 100];
        let mut output2 = vec![0.0; 100];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        noise1.process_block(&inputs, &mut output1, 44100.0, &context);
        noise2.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be identical
        for i in 0..100 {
            assert_eq!(output1[i], output2[i],
                "Deterministic brown noise with same seed should produce identical output");
        }
    }

    #[test]
    fn test_brown_noise_amplitude_control() {
        // Test with varying amplitude (like an LFO modulation)
        let mut noise = BrownNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.0, 0.25, 0.5, 1.0];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Check ranges based on amplitude
        assert_eq!(output[0], 0.0);  // Zero amplitude = silence
        assert!(output[1].abs() <= 0.25, "Sample with 0.25 amplitude out of range");
        assert!(output[2].abs() <= 0.5, "Sample with 0.5 amplitude out of range");
        assert!(output[3].abs() <= 1.0, "Sample with 1.0 amplitude out of range");
    }

    #[test]
    fn test_brown_noise_dependencies() {
        let noise = BrownNoiseNode::new(7);
        let deps = noise.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_brown_noise_with_constant() {
        // Integration test with ConstantNode
        let mut amplitude_node = ConstantNode::new(0.3);
        let mut noise = BrownNoiseNode::new_with_seed(0, 42);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process amplitude constant
        let mut amplitude_buf = vec![0.0; 512];
        amplitude_node.process_block(&[], &mut amplitude_buf, 44100.0, &context);

        // Generate brown noise
        let inputs = vec![amplitude_buf.as_slice()];
        let mut output = vec![0.0; 512];

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in range [-0.3, 0.3]
        for sample in &output {
            assert!(*sample >= -0.3 && *sample <= 0.3,
                "Sample {} out of range [-0.3, 0.3]", sample);
        }

        // Should vary (random walk)
        let first = output[0];
        let all_same = output.iter().all(|&s| s == first);
        assert!(!all_same, "Brown noise should vary");
    }

    #[test]
    fn test_brown_noise_zero_amplitude_produces_silence() {
        // Zero amplitude should produce silence
        let mut noise = BrownNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![999.0; 100];  // Initialize with non-zero
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // All output should be zero
        for sample in &output {
            assert_eq!(*sample, 0.0, "Zero amplitude should produce silence");
        }
    }

    #[test]
    fn test_brown_noise_different_seeds_different_output() {
        // Different seeds should produce different output
        let mut noise1 = BrownNoiseNode::new_with_seed(0, 111);
        let mut noise2 = BrownNoiseNode::new_with_seed(0, 222);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output1 = vec![0.0; 100];
        let mut output2 = vec![0.0; 100];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        noise1.process_block(&inputs, &mut output1, 44100.0, &context);
        noise2.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be different
        let all_same = output1.iter().zip(output2.iter()).all(|(a, b)| a == b);
        assert!(!all_same, "Different seeds should produce different brown noise");
    }

    #[test]
    fn test_brown_noise_continuous_across_blocks() {
        // Brown noise should maintain continuity across multiple process_block calls
        let mut noise = BrownNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        // First block
        let mut output1 = vec![0.0; 100];
        noise.process_block(&inputs, &mut output1, 44100.0, &context);

        let last_of_first_block = output1[output1.len() - 1];

        // Second block
        let mut output2 = vec![0.0; 100];
        noise.process_block(&inputs, &mut output2, 44100.0, &context);

        let first_of_second_block = output2[0];

        // The step between blocks should be small (continuous random walk)
        let step = (first_of_second_block - last_of_first_block).abs();
        assert!(step < 0.05,
            "Step between blocks {} too large, should maintain continuity", step);
    }
}
