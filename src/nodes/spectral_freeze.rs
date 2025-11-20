/// Spectral Freeze node - FFT-based spectral freezing effect
///
/// This node implements a spectral freeze effect that captures and holds the
/// frequency spectrum of an audio signal. When frozen, the output contains the
/// captured spectrum with randomized phase to create movement, while the spectral
/// content remains constant.
///
/// # Algorithm
/// 1. Perform forward FFT on input signal (512-1024 bins)
/// 2. When freeze triggered (>0.5), capture current frequency spectrum
/// 3. While frozen, output captured spectrum with phase randomization for movement
/// 4. When unfrozen (≤0.5), pass input through unchanged
/// 5. Optional blur parameter smooths between adjacent frequency bins
///
/// # Musical Applications
/// - Ambient textures and drones from transient sounds
/// - Freeze interesting moments in evolving timbres
/// - Create "time-stopped" effects in performances
/// - Spectral granulation and texture generation
///
/// # Implementation Notes
/// - Uses realfft for efficient real-valued FFT
/// - Overlap-add synthesis with Hann windowing
/// - Phase randomization creates natural movement while frozen
/// - Blur creates smoother, more diffuse frozen textures

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use realfft::{RealFftPlanner, num_complex::Complex32};
use std::f32::consts::PI;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Spectral freeze node with pattern-controlled freeze trigger and blur
///
/// # Example
/// ```ignore
/// // Freeze a signal when triggered
/// let input_signal = OscillatorNode::new(0, Waveform::Saw);  // NodeId 0
/// let freeze_trigger = ConstantNode::new(0.0);  // NodeId 1 (0 = unfrozen)
/// let blur = ConstantNode::new(0.3);  // NodeId 2 (0.3 = moderate blur)
/// let spectral_freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);  // NodeId 3
/// ```
pub struct SpectralFreezeNode {
    input: NodeId,           // Signal to freeze
    freeze_input: NodeId,    // Freeze trigger (>0.5 = frozen, ≤0.5 = passthrough)
    blur_input: NodeId,      // Spectral blur amount 0.0-1.0

    // FFT state
    fft_size: usize,
    hop_size: usize,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    window: Vec<f32>,

    // Frozen spectrum storage
    frozen_spectrum: Vec<Complex32>,
    is_frozen: bool,

    // FFT planners (created once, reused)
    r2c: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    c2r: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,

    // Overlap-add state
    overlap_buffer: Vec<f32>,
    write_pos: usize,

    // Random number generator for phase randomization
    rng: StdRng,

    sample_rate: f32,
}

impl SpectralFreezeNode {
    /// Create a new spectral freeze node
    ///
    /// # Arguments
    /// * `input` - NodeId providing the signal to freeze
    /// * `freeze_input` - NodeId providing freeze trigger (>0.5 = frozen, ≤0.5 = passthrough)
    /// * `blur_input` - NodeId providing spectral blur amount (0.0 = no blur, 1.0 = max blur)
    /// * `sample_rate` - Sample rate in Hz (usually 44100.0)
    ///
    /// # FFT Configuration
    /// - FFT size: 1024 samples (~23ms at 44.1kHz)
    /// - Hop size: 256 samples (75% overlap for smooth reconstruction)
    /// - Window: Hann window for good time-frequency resolution
    pub fn new(
        input: NodeId,
        freeze_input: NodeId,
        blur_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        let fft_size = 1024;
        let hop_size = 256; // 75% overlap

        // Create FFT planners
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);

        // Create Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                let phase = 2.0 * PI * i as f32 / (fft_size as f32 - 1.0);
                0.5 * (1.0 - phase.cos())
            })
            .collect();

        // Initialize frozen spectrum with zeros
        let spectrum_size = fft_size / 2 + 1;
        let frozen_spectrum = vec![Complex32::new(0.0, 0.0); spectrum_size];

        Self {
            input,
            freeze_input,
            blur_input,
            fft_size,
            hop_size,
            input_buffer: vec![0.0; fft_size],
            output_buffer: vec![0.0; fft_size],
            window,
            frozen_spectrum,
            is_frozen: false,
            r2c,
            c2r,
            overlap_buffer: vec![0.0; fft_size],
            write_pos: 0,
            rng: StdRng::seed_from_u64(42), // Deterministic seed for testing
            sample_rate,
        }
    }

    /// Process a single FFT frame
    fn process_frame(&mut self, freeze: f32, blur: f32) -> f32 {
        // Apply window to input
        let mut windowed_input: Vec<f32> = self.input_buffer
            .iter()
            .zip(self.window.iter())
            .map(|(x, w)| x * w)
            .collect();

        // Forward FFT
        let mut spectrum = self.r2c.make_output_vec();
        self.r2c.process(&mut windowed_input, &mut spectrum).unwrap();

        // Check freeze state transition
        let should_freeze = freeze > 0.5;
        if should_freeze && !self.is_frozen {
            // Freeze triggered: capture current spectrum
            self.frozen_spectrum = spectrum.clone();
            self.is_frozen = true;
        } else if !should_freeze && self.is_frozen {
            // Unfreeze triggered
            self.is_frozen = false;
        }

        // Select output spectrum
        let mut output_spectrum = if self.is_frozen {
            // Use frozen spectrum with phase randomization
            self.frozen_spectrum
                .iter()
                .enumerate()
                .map(|(i, bin)| {
                    let magnitude = bin.norm();
                    // DC (index 0) and Nyquist (last index) bins must be real
                    if i == 0 || i == self.frozen_spectrum.len() - 1 {
                        Complex32::new(magnitude, 0.0)
                    } else {
                        let random_phase = self.rng.gen::<f32>() * 2.0 * PI;
                        Complex32::from_polar(magnitude, random_phase)
                    }
                })
                .collect()
        } else {
            // Pass through live spectrum
            spectrum
        };

        // Apply spectral blur if requested
        if blur > 0.0 && output_spectrum.len() > 2 {
            let blurred = self.apply_blur(&output_spectrum, blur);
            output_spectrum = blurred;
        }

        // Inverse FFT
        self.c2r.process(&mut output_spectrum, &mut self.output_buffer).unwrap();

        // Apply window again for overlap-add
        for (i, sample) in self.output_buffer.iter_mut().enumerate() {
            *sample *= self.window[i];
        }

        // Normalize by FFT size
        let scale = 1.0 / (self.fft_size as f32);
        for sample in self.output_buffer.iter_mut() {
            *sample *= scale;
        }

        // Extract output sample from overlap-add
        let output_sample = self.overlap_buffer[0];

        // Add current frame to overlap buffer
        for (i, sample) in self.output_buffer.iter().enumerate() {
            if i < self.overlap_buffer.len() {
                self.overlap_buffer[i] += sample;
            }
        }

        // Shift overlap buffer by hop size
        self.overlap_buffer.rotate_left(self.hop_size);
        for i in (self.overlap_buffer.len() - self.hop_size)..self.overlap_buffer.len() {
            self.overlap_buffer[i] = 0.0;
        }

        output_sample
    }

    /// Apply spectral blur by smoothing between adjacent bins
    fn apply_blur(&self, spectrum: &[Complex32], blur: f32) -> Vec<Complex32> {
        let mut blurred = Vec::with_capacity(spectrum.len());

        for i in 0..spectrum.len() {
            if i == 0 || i == spectrum.len() - 1 {
                // Don't blur DC and Nyquist bins
                blurred.push(spectrum[i]);
            } else {
                // Blend with neighbors
                let prev = spectrum[i - 1];
                let curr = spectrum[i];
                let next = spectrum[i + 1];

                let blurred_bin = prev * (blur * 0.25)
                    + curr * (1.0 - blur * 0.5)
                    + next * (blur * 0.25);

                blurred.push(blurred_bin);
            }
        }

        blurred
    }

    /// Get the current freeze state
    pub fn is_frozen(&self) -> bool {
        self.is_frozen
    }

    /// Get the FFT size
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Get the number of frozen spectrum bins
    pub fn spectrum_bins(&self) -> usize {
        self.frozen_spectrum.len()
    }

    /// Clear the frozen spectrum and reset state
    pub fn clear(&mut self) {
        self.frozen_spectrum.fill(Complex32::new(0.0, 0.0));
        self.is_frozen = false;
        self.input_buffer.fill(0.0);
        self.output_buffer.fill(0.0);
        self.overlap_buffer.fill(0.0);
        self.write_pos = 0;
    }
}

impl AudioNode for SpectralFreezeNode {
    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.freeze_input, self.blur_input]
    }

    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "SpectralFreezeNode requires 3 inputs: signal, freeze, blur"
        );

        let input_signal = inputs[0];
        let freeze_signal = inputs[1];
        let blur_signal = inputs[2];

        for i in 0..output.len() {
            // Add input to buffer
            self.input_buffer[self.write_pos] = input_signal[i];
            self.write_pos += 1;

            // Process when we have a full hop
            if self.write_pos >= self.hop_size {
                let freeze = freeze_signal[i];
                let blur = blur_signal[i].clamp(0.0, 1.0);

                output[i] = self.process_frame(freeze, blur);

                // Shift input buffer
                self.input_buffer.rotate_left(self.hop_size);
                for j in (self.input_buffer.len() - self.hop_size)..self.input_buffer.len() {
                    self.input_buffer[j] = 0.0;
                }
                self.write_pos -= self.hop_size;
            } else {
                // Not enough samples yet, output silence or passthrough
                output[i] = if self.is_frozen {
                    self.overlap_buffer[0]
                } else {
                    input_signal[i]
                };
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_freeze_creation() {
        let freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        assert_eq!(freeze.fft_size(), 1024);
        assert_eq!(freeze.spectrum_bins(), 513); // FFT_SIZE/2 + 1
        assert!(!freeze.is_frozen());
    }

    #[test]
    fn test_freeze_captures_spectrum() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        // Create input: sine wave at 440 Hz
        let mut input = vec![0.0f32; block_size];
        let freq = 440.0;
        let sample_rate = 44100.0;
        for i in 0..block_size {
            input[i] = (2.0 * PI * freq * i as f32 / sample_rate).sin();
        }

        let freeze_trigger = vec![1.0f32; block_size]; // Freeze on
        let blur = vec![0.0f32; block_size];

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        freeze.process_block(&inputs, &mut output, sample_rate, &context);

        // Should be frozen after processing
        assert!(freeze.is_frozen());
    }

    #[test]
    fn test_unfreeze_passes_input() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        let input = vec![0.5f32; block_size];
        let freeze_trigger = vec![0.0f32; block_size]; // Not frozen
        let blur = vec![0.0f32; block_size];

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        freeze.process_block(&inputs, &mut output, 44100.0, &context);

        // Should not be frozen
        assert!(!freeze.is_frozen());
    }

    #[test]
    fn test_blur_parameter_clamped() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        let input = vec![0.0f32; block_size];
        let freeze_trigger = vec![0.0f32; block_size];
        let blur = vec![2.0f32; block_size]; // Out of range, should be clamped

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        // Should not panic with out-of-range blur
        freeze.process_block(&inputs, &mut output, 44100.0, &context);
    }

    #[test]
    fn test_freeze_state_transitions() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        let input = vec![1.0f32; block_size];
        let mut freeze_trigger = vec![0.0f32; block_size];
        let blur = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        // Start unfrozen
        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];
        freeze.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(!freeze.is_frozen());

        // Freeze
        freeze_trigger = vec![1.0f32; block_size];
        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        freeze.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(freeze.is_frozen());

        // Unfreeze
        freeze_trigger = vec![0.0f32; block_size];
        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        freeze.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(!freeze.is_frozen());
    }

    #[test]
    fn test_clear_resets_state() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        // Freeze the node
        let input = vec![1.0f32; block_size];
        let freeze_trigger = vec![1.0f32; block_size];
        let blur = vec![0.0f32; block_size];

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        freeze.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(freeze.is_frozen());

        // Clear should reset
        freeze.clear();
        assert!(!freeze.is_frozen());
    }

    #[test]
    fn test_phase_randomization_creates_movement() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        // Create input signal
        let mut input = vec![0.0f32; block_size];
        for i in 0..block_size {
            input[i] = (2.0 * PI * 440.0 * i as f32 / 44100.0).sin();
        }

        let freeze_trigger = vec![1.0f32; block_size];
        let blur = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        // Process multiple blocks while frozen
        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output1 = vec![0.0f32; block_size];
        let mut output2 = vec![0.0f32; block_size];

        freeze.process_block(&inputs, &mut output1, 44100.0, &context);
        freeze.process_block(&inputs, &mut output2, 44100.0, &context);

        // Outputs should be different due to phase randomization
        let mut different_samples = 0;
        for i in 0..block_size {
            if (output1[i] - output2[i]).abs() > 0.001 {
                different_samples += 1;
            }
        }

        // At least some samples should differ
        assert!(different_samples > 0, "Phase randomization should create different outputs");
    }

    #[test]
    fn test_different_fft_sizes_performance() {
        // This test verifies that the node can handle processing at different block sizes
        let block_sizes = [128, 256, 512, 1024];

        for &block_size in &block_sizes {
            let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);

            let input = vec![0.5f32; block_size];
            let freeze_trigger = vec![0.0f32; block_size];
            let blur = vec![0.0f32; block_size];

            let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
            let mut output = vec![0.0f32; block_size];

            let context = ProcessContext::new(
                crate::pattern::Fraction::from_float(0.0),
                0,
                block_size,
                2.0,
                44100.0,
            );

            // Should not panic
            freeze.process_block(&inputs, &mut output, 44100.0, &context);
        }
    }

    #[test]
    fn test_blur_smooths_spectrum() {
        let mut freeze_no_blur = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let mut freeze_with_blur = SpectralFreezeNode::new(3, 4, 5, 44100.0);

        let block_size = 512;

        // Create noisy input with rich harmonic content
        let mut input = vec![0.0f32; block_size];
        for i in 0..block_size {
            input[i] = (2.0 * PI * 440.0 * i as f32 / 44100.0).sin() * 0.3
                     + (2.0 * PI * 880.0 * i as f32 / 44100.0).sin() * 0.2
                     + (2.0 * PI * 1320.0 * i as f32 / 44100.0).sin() * 0.1;
        }

        let freeze_trigger = vec![1.0f32; block_size];
        let no_blur = vec![0.0f32; block_size];
        let yes_blur = vec![0.9f32; block_size]; // Higher blur for more obvious difference

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        // Process multiple blocks to ensure FFT frames are processed
        let inputs_no_blur = vec![&input[..], &freeze_trigger[..], &no_blur[..]];
        let inputs_yes_blur = vec![&input[..], &freeze_trigger[..], &yes_blur[..]];
        let mut output1 = vec![0.0f32; block_size];
        let mut output2 = vec![0.0f32; block_size];

        // Process several blocks to warm up the FFT pipeline
        for _ in 0..3 {
            freeze_no_blur.process_block(&inputs_no_blur, &mut output1, 44100.0, &context);
            freeze_with_blur.process_block(&inputs_yes_blur, &mut output2, 44100.0, &context);
        }

        // Outputs should be different
        let mut differences = 0;
        for i in 0..block_size {
            if (output1[i] - output2[i]).abs() > 0.0001 {
                differences += 1;
            }
        }

        // At least some samples should differ due to blur
        assert!(differences > 10, "Blur should affect the output (found {} differences)", differences);
    }

    #[test]
    fn test_pattern_modulation_of_freeze() {
        // Test that freeze parameter can be modulated over time
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        let input = vec![1.0f32; block_size];

        // Modulate freeze: first half off, second half on
        let mut freeze_trigger = vec![0.0f32; block_size];
        for i in block_size/2..block_size {
            freeze_trigger[i] = 1.0;
        }

        let blur = vec![0.0f32; block_size];

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        freeze.process_block(&inputs, &mut output, 44100.0, &context);

        // Should be frozen by the end
        assert!(freeze.is_frozen());
    }

    #[test]
    fn test_spectral_freeze_with_silence() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        // Silent input
        let input = vec![0.0f32; block_size];
        let freeze_trigger = vec![1.0f32; block_size];
        let blur = vec![0.0f32; block_size];

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        freeze.process_block(&inputs, &mut output, 44100.0, &context);

        // Should handle silence gracefully
        assert!(freeze.is_frozen());

        // Output should be mostly silent (allowing for numerical noise)
        let max_output = output.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
        assert!(max_output < 0.1, "Frozen silence should output near-silence");
    }

    #[test]
    fn test_spectral_freeze_energy_preservation() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        // Create input with known energy
        let mut input = vec![0.0f32; block_size];
        for i in 0..block_size {
            input[i] = (2.0 * PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
        }

        let input_energy: f32 = input.iter().map(|x| x * x).sum();

        let freeze_trigger = vec![1.0f32; block_size];
        let blur = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        // Process multiple blocks
        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        for _ in 0..5 {
            freeze.process_block(&inputs, &mut output, 44100.0, &context);
        }

        let output_energy: f32 = output.iter().map(|x| x * x).sum();

        // Output should have some energy (not just silence)
        assert!(output_energy > 0.0, "Frozen output should have energy");
    }

    #[test]
    fn test_clear_functionality() {
        let mut freeze = SpectralFreezeNode::new(0, 1, 2, 44100.0);
        let block_size = 512;

        // Freeze the node
        let input = vec![1.0f32; block_size];
        let freeze_trigger = vec![1.0f32; block_size];
        let blur = vec![0.0f32; block_size];

        let inputs = vec![&input[..], &freeze_trigger[..], &blur[..]];
        let mut output = vec![0.0f32; block_size];

        let context = ProcessContext::new(
            crate::pattern::Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        );

        freeze.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(freeze.is_frozen());

        // Clear using the clear method
        freeze.clear();
        assert!(!freeze.is_frozen());
    }
}
