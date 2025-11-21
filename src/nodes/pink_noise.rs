/// Pink noise generator (1/f spectrum)
///
/// Generates pink noise using the Voss-McCartney algorithm, which produces
/// noise with a 1/f frequency spectrum (equal energy per octave). Pink noise
/// has more bass content than white noise and sounds "warmer".
///
/// # Algorithm
///
/// The Voss-McCartney algorithm maintains 7 octave bins, updating different
/// octaves at different rates based on bit patterns in a counter. This creates
/// a 1/f spectrum through averaging.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Pink noise node: generates 1/f spectrum noise scaled by amplitude
///
/// # Example
/// ```ignore
/// // Generate pink noise at 0.5 amplitude
/// let amplitude = ConstantNode::new(0.5);  // NodeId 0
/// let pink = PinkNoiseNode::new(0);         // NodeId 1
/// // Output will be pink noise in [-0.5, 0.5] with 1/f spectrum
/// ```
pub struct PinkNoiseNode {
    amplitude_input: NodeId,
    rng: StdRng,
    // Voss-McCartney algorithm state (7 octaves)
    octaves: [f32; 7],
    counter: u32,
}

impl PinkNoiseNode {
    /// PinkNoiseNode - 1/f noise generator with equal energy per octave
    ///
    /// Generates pink noise (1/f spectrum) using Voss-McCartney algorithm with seven
    /// octave bins. Warmer than white noise for natural-sounding textures, rain sounds,
    /// and ambient backgrounds.
    ///
    /// # Parameters
    /// - `amplitude_input`: NodeId of amplitude control signal (0.0-1.0 typical)
    ///
    /// # Example
    /// ```phonon
    /// ~pink: pink_noise 0.5
    /// ```
    pub fn new(amplitude_input: NodeId) -> Self {
        let mut rng = StdRng::from_entropy();
        // Initialize octaves with random values
        let octaves: [f32; 7] = [
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
        ];
        Self {
            amplitude_input,
            rng,
            octaves,
            counter: 0,
        }
    }

    /// Create a new pink noise node with a specific seed (for testing)
    ///
    /// # Arguments
    /// * `amplitude_input` - NodeId of the amplitude control signal
    /// * `seed` - Seed value for deterministic random generation
    pub fn new_with_seed(amplitude_input: NodeId, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        // Initialize octaves with seeded random values
        let octaves: [f32; 7] = [
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
            rng.gen::<f32>() * 2.0 - 1.0,
        ];
        Self {
            amplitude_input,
            rng,
            octaves,
            counter: 0,
        }
    }

    /// Get the amplitude input node ID
    pub fn amplitude_input(&self) -> NodeId {
        self.amplitude_input
    }
}

impl AudioNode for PinkNoiseNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 1,
            "PinkNoiseNode requires 1 input (amplitude), got {}",
            inputs.len()
        );

        let amplitude = inputs[0];

        debug_assert_eq!(
            amplitude.len(),
            output.len(),
            "Amplitude input length mismatch"
        );

        // Generate pink noise using Voss-McCartney algorithm
        for i in 0..output.len() {
            let amp = amplitude[i];

            // Update octave bins based on bit pattern
            let mut pink = 0.0;
            for octave in 0..7 {
                if self.counter & (1 << octave) == 0 {
                    // This octave flips this sample
                    self.octaves[octave] = self.rng.gen::<f32>() * 2.0 - 1.0;
                }
                pink += self.octaves[octave];
            }
            self.counter = self.counter.wrapping_add(1);

            // Normalize (7 octaves → divide by 7.0 to guarantee ±1 range)
            // Each octave is [-1, 1], so sum is in [-7, 7]
            output[i] = (pink / 7.0) * amp;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.amplitude_input]
    }

    fn name(&self) -> &str {
        "PinkNoiseNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_pink_noise_generates_different_values() {
        // Pink noise should produce varying output (not constant)
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![1.0; 100];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 100];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            100,
            2.0,
            44100.0,
        );

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Check that not all samples are identical
        let first_sample = output[0];
        let all_same = output.iter().all(|&s| s == first_sample);

        assert!(!all_same, "Pink noise output should vary between samples");
    }

    #[test]
    fn test_pink_noise_spectrum_analysis() {
        // Pink noise should have more low-frequency content than white noise
        // We'll verify this by checking that consecutive samples are more
        // correlated in pink noise than in white noise

        let mut pink = PinkNoiseNode::new_with_seed(0, 12345);

        let amplitude = vec![1.0; 8192];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 8192];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            8192,
            2.0,
            44100.0,
        );

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate autocorrelation at lag 1 (adjacent samples)
        let mut correlation_sum = 0.0;
        for i in 1..output.len() {
            correlation_sum += output[i] * output[i - 1];
        }
        let autocorr = correlation_sum / (output.len() - 1) as f32;

        // Pink noise should have positive autocorrelation (smoother than white)
        // White noise has autocorr ≈ 0, pink noise should be significantly > 0
        // Voss-McCartney algorithm produces modest correlation (~0.02-0.03)
        assert!(autocorr > 0.01,
            "Pink noise autocorrelation {} should be > 0.01 (indicating low-freq bias)",
            autocorr);
    }

    #[test]
    fn test_pink_noise_average_near_zero() {
        // Pink noise should average to approximately zero
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![1.0; 44100];  // 1 second at 44.1kHz
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 44100];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            44100,
            2.0,
            44100.0,
        );

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate mean
        let sum: f32 = output.iter().sum();
        let mean = sum / output.len() as f32;

        // Mean should be close to zero
        assert!(mean.abs() < 0.05,
            "Mean {} not close to zero (pink noise should have zero mean)", mean);
    }

    #[test]
    fn test_pink_noise_seed_reproducibility() {
        // Same seed should produce same output
        let mut pink1 = PinkNoiseNode::new_with_seed(0, 99999);
        let mut pink2 = PinkNoiseNode::new_with_seed(0, 99999);

        let amplitude = vec![1.0; 200];
        let inputs = vec![amplitude.as_slice()];

        let mut output1 = vec![0.0; 200];
        let mut output2 = vec![0.0; 200];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            200,
            2.0,
            44100.0,
        );

        pink1.process_block(&inputs, &mut output1, 44100.0, &context);
        pink2.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be identical
        for i in 0..200 {
            assert_eq!(output1[i], output2[i],
                "Deterministic pink noise with same seed should produce identical output");
        }
    }

    #[test]
    fn test_pink_noise_amplitude_control() {
        // Pink noise should respect amplitude parameter
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.3; 512];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in range [-0.3, 0.3]
        for sample in &output {
            assert!(*sample >= -0.3 && *sample <= 0.3,
                "Sample {} out of range [-0.3, 0.3]", sample);
        }
    }

    #[test]
    fn test_pink_noise_dependencies() {
        // Should depend on exactly one input (amplitude)
        let pink = PinkNoiseNode::new(7);
        let deps = pink.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 7);
    }

    #[test]
    fn test_pink_noise_with_constant() {
        // Integration test with ConstantNode
        let mut amplitude_node = ConstantNode::new(0.4);
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

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

        // Generate pink noise
        let inputs = vec![amplitude_buf.as_slice()];
        let mut output = vec![0.0; 512];

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in range [-0.4, 0.4]
        for sample in &output {
            assert!(*sample >= -0.4 && *sample <= 0.4,
                "Sample {} out of range [-0.4, 0.4]", sample);
        }

        // Should not all be the same value
        let first = output[0];
        let all_same = output.iter().all(|&s| s == first);
        assert!(!all_same, "Pink noise should vary");
    }

    #[test]
    fn test_pink_noise_zero_amplitude_produces_silence() {
        // Zero amplitude should produce silence
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

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

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // All output should be zero
        for sample in &output {
            assert_eq!(*sample, 0.0, "Zero amplitude should produce silence");
        }
    }

    #[test]
    fn test_pink_noise_different_seeds_different_output() {
        // Different seeds should produce different output
        let mut pink1 = PinkNoiseNode::new_with_seed(0, 11111);
        let mut pink2 = PinkNoiseNode::new_with_seed(0, 22222);

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

        pink1.process_block(&inputs, &mut output1, 44100.0, &context);
        pink2.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be different
        let all_same = output1.iter().zip(output2.iter()).all(|(a, b)| a == b);
        assert!(!all_same, "Different seeds should produce different pink noise");
    }

    #[test]
    fn test_pink_noise_output_in_range() {
        // Output should be in range [-amplitude, amplitude]
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

        let amplitude = vec![0.5; 512];
        let inputs = vec![amplitude.as_slice()];

        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be in [-0.5, 0.5]
        for sample in &output {
            assert!(*sample >= -0.5 && *sample <= 0.5,
                "Sample {} out of range [-0.5, 0.5]", sample);
        }
    }

    #[test]
    fn test_pink_noise_varying_amplitude() {
        // Test with varying amplitude (like an LFO modulation)
        let mut pink = PinkNoiseNode::new_with_seed(0, 42);

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

        pink.process_block(&inputs, &mut output, 44100.0, &context);

        // Check ranges based on amplitude
        assert_eq!(output[0], 0.0);  // Zero amplitude = silence
        assert!(output[1].abs() <= 0.25, "Sample with 0.25 amplitude out of range");
        assert!(output[2].abs() <= 0.5, "Sample with 0.5 amplitude out of range");
        assert!(output[3].abs() <= 1.0, "Sample with 1.0 amplitude out of range");
    }
}
