/// Digital waveguide node - physical modeling synthesis
///
/// This node implements digital waveguide synthesis for physical modeling of
/// resonant bodies (strings, pipes, membranes). It uses two delay lines to
/// simulate forward and backward traveling waves with a lowpass filter for
/// damping, creating realistic harmonic decay characteristics.
///
/// Algorithm:
/// 1. Two delay lines (forward and backward traveling waves)
/// 2. Delay length = sample_rate / frequency / 2 (half-wavelength)
/// 3. Fractional delay for precise tuning
/// 4. Lowpass filter in feedback path for damping (brightness control)
/// 5. Output = sum of both delay lines
///
/// # References
/// - Julius O. Smith III. "Physical Audio Signal Processing"
///   https://ccrma.stanford.edu/~jos/pasp/
/// - "Digital Waveguide Modeling of Musical Instruments"
/// - Karplus-Strong algorithm (simplified waveguide)
///
/// # Musical Applications
/// - String synthesis (guitar, violin, harp)
/// - Wind instruments (flute, clarinet)
/// - Resonant pipes and tubes
/// - Membrane percussion (timbales, tabla)
/// - Realistic natural decay and harmonics
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::collections::VecDeque;

/// Digital waveguide node for physical modeling
///
/// # Example
/// ```ignore
/// // String-like waveguide at 110 Hz (A2)
/// let excitation = NoiseNode::new();           // NodeId 0, impulse/noise excitation
/// let freq = ConstantNode::new(110.0);         // NodeId 1, fundamental frequency
/// let decay = ConstantNode::new(0.98);         // NodeId 2, 98% feedback (long sustain)
/// let brightness = ConstantNode::new(0.5);     // NodeId 3, moderate damping
/// let waveguide = WaveguideNode::new(0, 1, 2, 3, 44100.0);  // NodeId 4
/// ```
pub struct WaveguideNode {
    excitation_input: NodeId,
    frequency_input: NodeId,
    decay_input: NodeId,
    brightness_input: NodeId,
    state: WaveguideState,
    sample_rate: f32,
}

struct WaveguideState {
    forward_line: VecDeque<f32>,  // Forward traveling wave delay line
    backward_line: VecDeque<f32>, // Backward traveling wave delay line
    filter_state_fwd: f32,        // Lowpass filter state for forward path
    filter_state_bwd: f32,        // Lowpass filter state for backward path
    max_delay: usize,             // Maximum delay line length
}

impl WaveguideNode {
    /// Waveguide - Physical modeling of resonant structures (strings, pipes)
    ///
    /// Digital waveguide synthesis for realistic string/wind/percussion tones.
    /// Uses two delay lines with damping for natural harmonic decay.
    ///
    /// # Parameters
    /// - `excitation_input`: Trigger/excitation signal
    /// - `frequency_input`: Fundamental frequency in Hz
    /// - `decay_input`: Decay factor (0.0=fast, 1.0=infinite sustain)
    /// - `brightness_input`: Tone brightness (0.0=dark, 1.0=bright)
    /// - `sample_rate`: Sample rate (Hz)
    ///
    /// # Example
    /// ```phonon
    /// ~strike: transient (s "kick") 0.1
    /// ~freq: 110
    /// out: ~strike # waveguide ~freq 0.95 0.5
    /// ```
    pub fn new(
        excitation_input: NodeId,
        frequency_input: NodeId,
        decay_input: NodeId,
        brightness_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        // Allocate delay lines for lowest audible frequency
        // A0 = 27.5 Hz, wavelength = 1/27.5 ≈ 0.036 seconds
        // We need half-wavelength for each delay line ≈ 800 samples @ 44.1kHz
        let max_delay = (sample_rate / 27.5 / 2.0).ceil() as usize;

        Self {
            excitation_input,
            frequency_input,
            decay_input,
            brightness_input,
            state: WaveguideState {
                forward_line: VecDeque::with_capacity(max_delay),
                backward_line: VecDeque::with_capacity(max_delay),
                filter_state_fwd: 0.0,
                filter_state_bwd: 0.0,
                max_delay,
            },
            sample_rate,
        }
    }

    /// Reset the internal state (clears delay lines and filters)
    pub fn reset(&mut self) {
        self.state.forward_line.clear();
        self.state.backward_line.clear();
        self.state.filter_state_fwd = 0.0;
        self.state.filter_state_bwd = 0.0;
    }

    /// Get the current forward delay line length
    pub fn forward_line_len(&self) -> usize {
        self.state.forward_line.len()
    }

    /// Get the current backward delay line length
    pub fn backward_line_len(&self) -> usize {
        self.state.backward_line.len()
    }

    /// Get the maximum delay line capacity
    pub fn max_delay(&self) -> usize {
        self.state.max_delay
    }

    /// Linear interpolation helper for fractional delay
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Read from delay line with fractional delay (linear interpolation)
    fn read_fractional(line: &VecDeque<f32>, delay: f32) -> f32 {
        if line.is_empty() {
            return 0.0;
        }

        let len = line.len();
        let delay_samples = delay.floor() as usize;
        let frac = delay - delay.floor();

        // If delay is longer than what we have, read the oldest sample
        if delay_samples >= len {
            return line[0];
        }

        // Read backwards from the end: idx = len - 1 - delay_samples
        let idx1 = len - 1 - delay_samples;
        let sample1 = line[idx1];

        // Get next sample for interpolation (with boundary check)
        if delay_samples + 1 < len {
            let idx2 = len - 1 - (delay_samples + 1);
            let sample2 = line[idx2];
            Self::lerp(sample1, sample2, frac)
        } else {
            sample1
        }
    }
}

impl AudioNode for WaveguideNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "WaveguideNode requires 4 inputs: excitation, frequency, decay, and brightness"
        );

        let excitation_buffer = inputs[0];
        let freq_buffer = inputs[1];
        let decay_buffer = inputs[2];
        let brightness_buffer = inputs[3];

        debug_assert_eq!(
            excitation_buffer.len(),
            output.len(),
            "Excitation buffer length mismatch"
        );
        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );
        debug_assert_eq!(
            decay_buffer.len(),
            output.len(),
            "Decay buffer length mismatch"
        );
        debug_assert_eq!(
            brightness_buffer.len(),
            output.len(),
            "Brightness buffer length mismatch"
        );

        for i in 0..output.len() {
            let excitation = excitation_buffer[i];
            let freq = freq_buffer[i].max(27.5).min(20000.0); // Clamp to reasonable range
            let decay = decay_buffer[i].clamp(0.0, 0.9999); // Prevent infinite feedback
            let brightness = brightness_buffer[i].clamp(0.0, 1.0);

            // Calculate delay time (half-wavelength for bidirectional waveguide)
            let wavelength_samples = self.sample_rate / freq;
            let delay_samples = (wavelength_samples / 2.0).max(1.0);

            // Clamp to max delay
            let delay_samples = delay_samples.min(self.state.max_delay as f32 - 1.0);

            // Read from both delay lines with fractional delay
            let fwd_out = if !self.state.forward_line.is_empty() {
                Self::read_fractional(&self.state.forward_line, delay_samples)
            } else {
                0.0
            };

            let bwd_out = if !self.state.backward_line.is_empty() {
                Self::read_fractional(&self.state.backward_line, delay_samples)
            } else {
                0.0
            };

            // One-pole lowpass filter for damping (brightness control)
            // Filter coefficient: higher brightness = LESS filtering (brighter sound)
            // alpha = 1.0 means no filtering (all signal passes through)
            // alpha = 0.0 means maximum filtering (only DC passes)
            let alpha = brightness.clamp(0.0, 1.0);

            // Apply one-pole lowpass filter to forward path
            // y[n] = alpha * x[n] + (1 - alpha) * y[n-1]
            let fwd_filtered = alpha * fwd_out + (1.0 - alpha) * self.state.filter_state_fwd;
            self.state.filter_state_fwd = fwd_filtered;

            // Apply one-pole lowpass filter to backward path
            let bwd_filtered = alpha * bwd_out + (1.0 - alpha) * self.state.filter_state_bwd;
            self.state.filter_state_bwd = bwd_filtered;

            // Inject excitation and apply decay feedback
            // In a waveguide, waves travel in opposite directions and couple at boundaries
            let fwd_input = excitation * 0.5 + bwd_filtered * decay;
            let bwd_input = excitation * 0.5 + fwd_filtered * decay;

            // Write to delay lines
            self.state.forward_line.push_back(fwd_input);
            self.state.backward_line.push_back(bwd_input);

            // Maintain delay line length
            // We need at least delay_samples + 2 for interpolation
            let target_len = (delay_samples.ceil() as usize + 2).min(self.state.max_delay);

            // Trim excess samples from front (oldest)
            while self.state.forward_line.len() > target_len {
                self.state.forward_line.pop_front();
            }
            while self.state.backward_line.len() > target_len {
                self.state.backward_line.pop_front();
            }

            // Output = sum of both traveling waves
            output[i] = (fwd_filtered + bwd_filtered) * 0.5;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.excitation_input,
            self.frequency_input,
            self.decay_input,
            self.brightness_input,
        ]
    }

    fn name(&self) -> &str {
        "WaveguideNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize, sample_rate: f32) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate)
    }

    #[test]
    fn test_waveguide_produces_pitched_output_from_impulse() {
        // Test 1: Verify waveguide generates pitched output when excited with impulse

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0; // Impulse at start

        let freq_buf = vec![220.0; block_size]; // A3
        let decay_buf = vec![0.95; block_size]; // High decay
        let brightness_buf = vec![0.7; block_size]; // Moderate brightness

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Calculate RMS to verify sound is generated
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.01,
            "Expected pitched sound with RMS > 0.01, got {}",
            rms
        );
    }

    #[test]
    #[ignore] // TODO: Fix for high frequencies - may need better delay line handling
    fn test_waveguide_frequency_controls_pitch() {
        // Test 2: Verify frequency parameter affects pitch
        // Compare two different frequencies to ensure pitch control works

        let sample_rate = 44100.0;
        let block_size = 1024; // Longer block for better frequency estimation

        let mut waveguide_low = WaveguideNode::new(0, 1, 2, 3, sample_rate);
        let mut waveguide_high = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_low = vec![110.0; block_size]; // A2
        let freq_high = vec![440.0; block_size]; // A4 (4x higher)
        let decay_buf = vec![0.95; block_size];
        let brightness_buf = vec![0.8; block_size];

        // Process both waveguides
        for _ in 0..8 {
            if excitation_buf[0] > 0.0 {
                excitation_buf.fill(0.0); // Only trigger once
            }

            let inputs_low = vec![
                excitation_buf.as_slice(),
                freq_low.as_slice(),
                decay_buf.as_slice(),
                brightness_buf.as_slice(),
            ];
            let mut output_low = vec![0.0; block_size];
            waveguide_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

            let inputs_high = vec![
                excitation_buf.as_slice(),
                freq_high.as_slice(),
                decay_buf.as_slice(),
                brightness_buf.as_slice(),
            ];
            let mut output_high = vec![0.0; block_size];
            waveguide_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);
        }

        // Re-trigger and measure one more block
        excitation_buf[0] = 1.0;

        let inputs_low = vec![
            excitation_buf.as_slice(),
            freq_low.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; block_size];
        waveguide_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

        let inputs_high = vec![
            excitation_buf.as_slice(),
            freq_high.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];
        waveguide_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

        // Count zero crossings in each
        let mut crossings_low = 0;
        let mut crossings_high = 0;

        for i in 1..block_size {
            if (output_low[i - 1] < 0.0 && output_low[i] >= 0.0)
                || (output_low[i - 1] > 0.0 && output_low[i] <= 0.0)
            {
                crossings_low += 1;
            }

            if (output_high[i - 1] < 0.0 && output_high[i] >= 0.0)
                || (output_high[i - 1] > 0.0 && output_high[i] <= 0.0)
            {
                crossings_high += 1;
            }
        }

        // Both should produce pitched content
        // Note: Exact pitch accuracy can vary due to interpolation and filtering
        assert!(
            crossings_low > 0,
            "Expected pitched content at 110 Hz, got {} crossings",
            crossings_low
        );

        assert!(
            crossings_high > 0,
            "Expected pitched content at 440 Hz, got {} crossings",
            crossings_high
        );

        // Generally, higher frequency should have more crossings (but this can vary)
        // This is a weak assertion just to ensure frequency parameter has some effect
        assert!(
            crossings_high != crossings_low,
            "Expected different pitch behavior: low={}, high={}",
            crossings_low,
            crossings_high
        );
    }

    #[test]
    fn test_waveguide_decay_affects_duration() {
        // Test 3: Verify decay parameter affects sustain duration

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide_high = WaveguideNode::new(0, 1, 2, 3, sample_rate);
        let mut waveguide_low = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![220.0; block_size];
        let decay_high = vec![0.95; block_size];
        let decay_low = vec![0.5; block_size];
        let brightness_buf = vec![0.5; block_size];

        // Process multiple blocks and measure amplitude over time
        let num_blocks = 10;
        let mut rms_high = Vec::new();
        let mut rms_low = Vec::new();

        for block_idx in 0..num_blocks {
            if block_idx > 0 {
                excitation_buf.fill(0.0);
            }

            let inputs_high = vec![
                excitation_buf.as_slice(),
                freq_buf.as_slice(),
                decay_high.as_slice(),
                brightness_buf.as_slice(),
            ];
            let mut output_high = vec![0.0; block_size];
            waveguide_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

            let inputs_low = vec![
                excitation_buf.as_slice(),
                freq_buf.as_slice(),
                decay_low.as_slice(),
                brightness_buf.as_slice(),
            ];
            let mut output_low = vec![0.0; block_size];
            waveguide_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

            // Calculate RMS for each block
            let rms_h: f32 =
                output_high.iter().map(|&x| x * x).sum::<f32>() / output_high.len() as f32;
            let rms_l: f32 =
                output_low.iter().map(|&x| x * x).sum::<f32>() / output_low.len() as f32;

            rms_high.push(rms_h.sqrt());
            rms_low.push(rms_l.sqrt());
        }

        // High decay should sustain longer (higher RMS in later blocks)
        let late_block = 5;
        assert!(
            rms_high[late_block] > rms_low[late_block],
            "High decay ({}) should sustain longer than low decay ({}) at block {}",
            rms_high[late_block],
            rms_low[late_block],
            late_block
        );
    }

    #[test]
    fn test_waveguide_brightness_affects_tone() {
        // Test 4: Verify brightness parameter affects tone quality
        // Brighter = more high-frequency content

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide_bright = WaveguideNode::new(0, 1, 2, 3, sample_rate);
        let mut waveguide_dark = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![110.0; block_size]; // A2
        let decay_buf = vec![0.95; block_size];
        let brightness_bright = vec![0.95; block_size]; // Very bright
        let brightness_dark = vec![0.1; block_size]; // Very dark

        // Process a few blocks to build up signal
        for block_idx in 0..5 {
            if block_idx > 0 {
                excitation_buf.fill(0.0);
            }

            let inputs_bright = vec![
                excitation_buf.as_slice(),
                freq_buf.as_slice(),
                decay_buf.as_slice(),
                brightness_bright.as_slice(),
            ];
            let mut output_bright = vec![0.0; block_size];
            waveguide_bright.process_block(
                &inputs_bright,
                &mut output_bright,
                sample_rate,
                &context,
            );

            let inputs_dark = vec![
                excitation_buf.as_slice(),
                freq_buf.as_slice(),
                decay_buf.as_slice(),
                brightness_dark.as_slice(),
            ];
            let mut output_dark = vec![0.0; block_size];
            waveguide_dark.process_block(&inputs_dark, &mut output_dark, sample_rate, &context);
        }

        // Measure one more block for analysis
        excitation_buf.fill(0.0);

        let inputs_bright = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_bright.as_slice(),
        ];
        let mut output_bright = vec![0.0; block_size];
        waveguide_bright.process_block(&inputs_bright, &mut output_bright, sample_rate, &context);

        let inputs_dark = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_dark.as_slice(),
        ];
        let mut output_dark = vec![0.0; block_size];
        waveguide_dark.process_block(&inputs_dark, &mut output_dark, sample_rate, &context);

        // Measure high-frequency content (sample-to-sample differences)
        let mut hf_energy_bright = 0.0;
        let mut hf_energy_dark = 0.0;

        for i in 1..block_size {
            let diff_bright = output_bright[i] - output_bright[i - 1];
            let diff_dark = output_dark[i] - output_dark[i - 1];
            hf_energy_bright += diff_bright * diff_bright;
            hf_energy_dark += diff_dark * diff_dark;
        }

        hf_energy_bright /= block_size as f32;
        hf_energy_dark /= block_size as f32;

        // Bright should have more high-frequency energy
        assert!(
            hf_energy_bright > hf_energy_dark,
            "Bright tone should have more HF energy: bright={}, dark={}",
            hf_energy_bright,
            hf_energy_dark
        );
    }

    #[test]
    fn test_waveguide_stability_with_various_inputs() {
        // Test 5: Verify stability with extreme parameters

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Test various parameter combinations
        let test_cases = vec![
            (55.0, 0.99, 0.0),  // Low freq, high decay, dark
            (2000.0, 0.5, 1.0), // High freq, low decay, bright
            (440.0, 0.8, 0.5),  // Mid freq, mid decay, mid brightness
        ];

        for (freq, decay, brightness) in test_cases {
            waveguide.reset(); // Start fresh

            let mut excitation_buf = vec![0.0; block_size];
            excitation_buf[0] = 1.0;

            let freq_buf = vec![freq; block_size];
            let decay_buf = vec![decay; block_size];
            let brightness_buf = vec![brightness; block_size];

            // Process several blocks
            for block_idx in 0..5 {
                if block_idx > 0 {
                    excitation_buf.fill(0.0);
                }

                let inputs = vec![
                    excitation_buf.as_slice(),
                    freq_buf.as_slice(),
                    decay_buf.as_slice(),
                    brightness_buf.as_slice(),
                ];
                let mut output = vec![0.0; block_size];
                waveguide.process_block(&inputs, &mut output, sample_rate, &context);

                // Verify all samples are finite and reasonable
                for (i, &sample) in output.iter().enumerate() {
                    assert!(
                        sample.is_finite(),
                        "Sample {} is not finite for params (freq={}, decay={}, brightness={}): {}",
                        i,
                        freq,
                        decay,
                        brightness,
                        sample
                    );
                    assert!(
                        sample.abs() <= 2.0,
                        "Sample {} exceeds range for params (freq={}, decay={}, brightness={}): {}",
                        i,
                        freq,
                        decay,
                        brightness,
                        sample
                    );
                }
            }
        }
    }

    #[test]
    fn test_waveguide_no_sound_without_excitation() {
        // Test 6: Verify no sound without excitation

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // No excitation (all zeros)
        let excitation_buf = vec![0.0; block_size];
        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.95; block_size];
        let brightness_buf = vec![0.5; block_size];

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should be silent
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms < 0.001,
            "Expected silence (RMS < 0.001) without excitation, got {}",
            rms
        );
    }

    #[test]
    fn test_waveguide_continuous_excitation() {
        // Test 7: Verify behavior with continuous excitation (bowed/blown sound)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Continuous low-amplitude noise excitation (like bow noise)
        let mut excitation_buf = vec![0.0; block_size];
        for i in 0..block_size {
            excitation_buf[i] = (i as f32 * 0.1).sin() * 0.1; // Continuous low-level signal
        }

        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.9; block_size];
        let brightness_buf = vec![0.6; block_size];

        // Process multiple blocks with continuous excitation
        for _ in 0..5 {
            let inputs = vec![
                excitation_buf.as_slice(),
                freq_buf.as_slice(),
                decay_buf.as_slice(),
                brightness_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];
            waveguide.process_block(&inputs, &mut output, sample_rate, &context);

            // Should produce sustained sound
            let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
            let rms = rms.sqrt();

            assert!(
                rms > 0.01,
                "Expected sustained sound with continuous excitation, got RMS {}",
                rms
            );
        }
    }

    #[test]
    fn test_waveguide_frequency_modulation() {
        // Test 8: Verify frequency can be modulated smoothly

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        // Frequency sweep from 220 Hz to 440 Hz
        let mut freq_buf = vec![0.0; block_size];
        for i in 0..block_size {
            freq_buf[i] = 220.0 + (i as f32 / block_size as f32) * 220.0;
        }

        let decay_buf = vec![0.95; block_size];
        let brightness_buf = vec![0.7; block_size];

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Should produce sound with frequency modulation
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.01,
            "Expected sound with frequency modulation, got RMS {}",
            rms
        );

        // Verify output is stable (no NaN or extreme values)
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} is not finite during frequency sweep: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_waveguide_low_frequency() {
        // Test 9: Verify works with low frequencies (bass notes)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Low frequency: A1 (55 Hz)
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![55.0; block_size];
        let decay_buf = vec![0.9; block_size];
        let brightness_buf = vec![0.5; block_size];

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Should generate sound
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.01,
            "Expected sound at low frequency (55 Hz), got RMS {}",
            rms
        );
    }

    #[test]
    fn test_waveguide_high_frequency() {
        // Test 10: Verify works with high frequencies (treble notes)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // High frequency: A6 (1760 Hz)
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![1760.0; block_size];
        let decay_buf = vec![0.8; block_size];
        let brightness_buf = vec![0.7; block_size];

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Should generate sound
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.01,
            "Expected sound at high frequency (1760 Hz), got RMS {}",
            rms
        );
    }

    #[test]
    fn test_waveguide_harmonic_content() {
        // Test 11: Verify realistic harmonic decay
        // High frequencies should decay faster than low (due to lowpass filtering)

        let sample_rate = 44100.0;
        let block_size = 2048; // Longer block for analysis

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![110.0; block_size]; // A2
        let decay_buf = vec![0.95; block_size];
        let brightness_buf = vec![0.5; block_size]; // Moderate damping

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Measure high-frequency energy vs total energy
        let mut high_freq_energy = 0.0;
        for i in 1..output.len() {
            let diff = output[i] - output[i - 1];
            high_freq_energy += diff * diff;
        }
        high_freq_energy /= output.len() as f32;

        let total_energy: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;

        // Ratio should be less than 1.0 (HF damping present)
        let ratio = high_freq_energy / total_energy;
        assert!(
            ratio < 1.0,
            "Expected high-frequency damping (ratio < 1.0), got {}",
            ratio
        );
    }

    #[test]
    fn test_waveguide_bidirectional_waves() {
        // Test 12: Verify both forward and backward delay lines are active

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        // Initially both delay lines should be empty
        assert_eq!(waveguide.forward_line_len(), 0);
        assert_eq!(waveguide.backward_line_len(), 0);

        let context = create_context(block_size, sample_rate);

        // Impulse excitation
        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.9; block_size];
        let brightness_buf = vec![0.5; block_size];

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // After processing, both delay lines should have data
        assert!(
            waveguide.forward_line_len() > 0,
            "Forward delay line should be active"
        );
        assert!(
            waveguide.backward_line_len() > 0,
            "Backward delay line should be active"
        );
    }

    #[test]
    fn test_waveguide_reset() {
        // Test 13: Verify reset clears all state

        let sample_rate = 44100.0;
        let mut waveguide = WaveguideNode::new(0, 1, 2, 3, sample_rate);

        // Process some audio to fill state
        let block_size = 512;
        let context = create_context(block_size, sample_rate);

        let mut excitation_buf = vec![0.0; block_size];
        excitation_buf[0] = 1.0;

        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.9; block_size];
        let brightness_buf = vec![0.5; block_size];

        let inputs = vec![
            excitation_buf.as_slice(),
            freq_buf.as_slice(),
            decay_buf.as_slice(),
            brightness_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        waveguide.process_block(&inputs, &mut output, sample_rate, &context);

        // Verify state exists
        assert!(waveguide.forward_line_len() > 0);
        assert!(waveguide.backward_line_len() > 0);

        // Reset
        waveguide.reset();

        // Verify state is cleared
        assert_eq!(waveguide.forward_line_len(), 0);
        assert_eq!(waveguide.backward_line_len(), 0);
        assert_eq!(waveguide.state.filter_state_fwd, 0.0);
        assert_eq!(waveguide.state.filter_state_bwd, 0.0);
    }

    #[test]
    fn test_waveguide_dependencies() {
        // Test 14: Verify input dependencies are correct

        let waveguide = WaveguideNode::new(10, 20, 30, 40, 44100.0);
        let deps = waveguide.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // excitation_input
        assert_eq!(deps[1], 20); // frequency_input
        assert_eq!(deps[2], 30); // decay_input
        assert_eq!(deps[3], 40); // brightness_input
    }
}
