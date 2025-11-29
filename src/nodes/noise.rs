/// White noise generator
///
/// Generates random values in the range [-amplitude, amplitude] using a
/// deterministic random number generator. White noise has equal energy
/// across all frequencies.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// White noise node: generates random values scaled by amplitude
///
/// # Example
/// ```ignore
/// // Generate white noise at 0.5 amplitude
/// let amplitude = ConstantNode::new(0.5);  // NodeId 0
/// let noise = NoiseNode::new(0);            // NodeId 1
/// // Output will be random values in [-0.5, 0.5]
/// ```
pub struct NoiseNode {
    amplitude_input: NodeId,
    rng: StdRng,
}

impl NoiseNode {
    /// NoiseNode - White noise generator with amplitude control
    ///
    /// Generates uniform random values in [-1, 1] scaled by input amplitude.
    /// Equal energy across all frequencies for ambient textures, noise beds, and
    /// synthesis source material.
    ///
    /// # Parameters
    /// - `amplitude_input`: NodeId of the amplitude control signal (0.0-1.0 typical)
    ///
    /// # Example
    /// ```phonon
    /// ~noise: noise 0.5
    /// ```
    pub fn new(amplitude_input: NodeId) -> Self {
        Self {
            amplitude_input,
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a new noise node with a specific seed (for testing)
    ///
    /// # Arguments
    /// * `amplitude_input` - NodeId of the amplitude control signal
    /// * `seed` - Seed value for deterministic random generation
    pub fn new_with_seed(amplitude_input: NodeId, seed: u64) -> Self {
        Self {
            amplitude_input,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Get the amplitude input node ID
    pub fn amplitude_input(&self) -> NodeId {
        self.amplitude_input
    }
}

impl AudioNode for NoiseNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "NoiseNode requires 1 input (amplitude), got {}",
            inputs.len()
        );

        let amplitude = inputs[0];

        debug_assert_eq!(
            amplitude.len(),
            output.len(),
            "Amplitude input length mismatch"
        );

        // Generate white noise: random values in [-1, 1] scaled by amplitude
        for i in 0..output.len() {
            // Generate random value in [-1.0, 1.0]
            let random_value = self.rng.gen_range(-1.0..=1.0);
            output[i] = random_value * amplitude[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.amplitude_input]
    }

    fn name(&self) -> &str {
        "NoiseNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_noise_output_in_range() {
        // Output should be in range [-amplitude, amplitude]
        let mut noise = NoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.5; 512];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in [-0.5, 0.5]
        for sample in &output {
            assert!(
                *sample >= -0.5 && *sample <= 0.5,
                "Sample {} out of range [-0.5, 0.5]",
                sample
            );
        }
    }

    #[test]
    fn test_noise_average_near_zero() {
        // White noise should average to approximately zero
        let mut noise = NoiseNode::new_with_seed(0, 42);

        let amplitude = vec![1.0; 44100]; // 1 second at 44.1kHz
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 44100];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 44100, 2.0, 44100.0);

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate mean
        let sum: f32 = output.iter().sum();
        let mean = sum / output.len() as f32;

        // Mean should be close to zero (within 0.05 for white noise)
        assert!(
            mean.abs() < 0.05,
            "Mean {} not close to zero (white noise should have zero mean)",
            mean
        );
    }

    #[test]
    fn test_noise_not_constant() {
        // Output should change between samples (not constant)
        let mut noise = NoiseNode::new_with_seed(0, 42);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 100];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 100, 2.0, 44100.0);

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Check that not all samples are identical
        let first_sample = output[0];
        let all_same = output.iter().all(|&s| s == first_sample);

        assert!(!all_same, "Noise output should vary between samples");
    }

    #[test]
    fn test_noise_dependencies() {
        let noise = NoiseNode::new(7);
        let deps = noise.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_noise_with_varying_amplitude() {
        // Test with varying amplitude (like an LFO modulation)
        let mut noise = NoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.0, 0.25, 0.5, 1.0];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, 44100.0);

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Check ranges based on amplitude
        assert_eq!(output[0], 0.0); // Zero amplitude = silence
        assert!(
            output[1].abs() <= 0.25,
            "Sample with 0.25 amplitude out of range"
        );
        assert!(
            output[2].abs() <= 0.5,
            "Sample with 0.5 amplitude out of range"
        );
        assert!(
            output[3].abs() <= 1.0,
            "Sample with 1.0 amplitude out of range"
        );
    }

    #[test]
    fn test_noise_deterministic_with_seed() {
        // Same seed should produce same output
        let mut noise1 = NoiseNode::new_with_seed(0, 12345);
        let mut noise2 = NoiseNode::new_with_seed(0, 12345);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output1 = vec![0.0; 100];
        let mut output2 = vec![0.0; 100];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 100, 2.0, 44100.0);

        noise1.process_block(&inputs, &mut output1, 44100.0, &context);
        noise2.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be identical
        for i in 0..100 {
            assert_eq!(
                output1[i], output2[i],
                "Deterministic noise with same seed should produce identical output"
            );
        }
    }

    #[test]
    fn test_noise_different_seeds_different_output() {
        // Different seeds should produce different output
        let mut noise1 = NoiseNode::new_with_seed(0, 111);
        let mut noise2 = NoiseNode::new_with_seed(0, 222);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output1 = vec![0.0; 100];
        let mut output2 = vec![0.0; 100];
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 100, 2.0, 44100.0);

        noise1.process_block(&inputs, &mut output1, 44100.0, &context);
        noise2.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be different
        let all_same = output1.iter().zip(output2.iter()).all(|(a, b)| a == b);
        assert!(!all_same, "Different seeds should produce different noise");
    }

    #[test]
    fn test_noise_with_constant_integration() {
        // Integration test with ConstantNode
        let mut amplitude_node = ConstantNode::new(0.3);
        let mut noise = NoiseNode::new_with_seed(0, 42);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        // Process amplitude constant
        let mut amplitude_buf = vec![0.0; 512];
        amplitude_node.process_block(&[], &mut amplitude_buf, 44100.0, &context);

        // Generate noise
        let inputs = vec![amplitude_buf.as_slice()];
        let mut output = vec![0.0; 512];

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in range [-0.3, 0.3]
        for sample in &output {
            assert!(
                *sample >= -0.3 && *sample <= 0.3,
                "Sample {} out of range [-0.3, 0.3]",
                sample
            );
        }

        // Should not all be the same value
        let first = output[0];
        let all_same = output.iter().all(|&s| s == first);
        assert!(!all_same, "Noise should vary");
    }

    #[test]
    fn test_noise_zero_amplitude_produces_silence() {
        // Zero amplitude should produce silence
        let mut noise = NoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![999.0; 100]; // Initialize with non-zero
        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 100, 2.0, 44100.0);

        noise.process_block(&inputs, &mut output, 44100.0, &context);

        // All output should be zero
        for sample in &output {
            assert_eq!(*sample, 0.0, "Zero amplitude should produce silence");
        }
    }
}
