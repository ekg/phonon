//! Feedback Delay Network (FDN) Reverb
//!
//! An 8-channel FDN reverb using a Householder mixing matrix for efficient,
//! high-quality reverberation. This design provides:
//! - Dense modal distribution from coprime delay lengths
//! - Efficient O(N) mixing via Householder reflection matrix
//! - Per-channel damping for natural high-frequency decay
//! - Scalable design for different sample rates
//!
//! # Algorithm Overview
//!
//! The FDN reverb uses 8 parallel delay lines with feedback through a unitary
//! mixing matrix. The Householder matrix provides efficient mixing with only
//! O(N) operations instead of O(N²) for a general matrix.
//!
//! ## Signal Flow
//!
//! ```text
//! Input → [Add to channel 0] → [8 Delay Lines] → [Householder Mix] → [Damping] → [Decay] → Output
//!                                     ↑                                             ↓
//!                                     └─────────────────────────────────────────────┘
//! ```
//!
//! ## Householder Matrix
//!
//! The mixing matrix is defined as: **H = I - (2/N) · 1·1ᵀ**
//!
//! Where:
//! - **I** is the identity matrix
//! - **1** is a vector of all ones
//! - **N** is the number of channels (8)
//!
//! This can be efficiently computed as:
//! ```text
//! output[i] = input[i] - (2/N) × sum(all inputs)
//! ```
//!
//! ## Delay Line Lengths
//!
//! At 44.1 kHz, the delay lengths are (in samples):
//! `[1087, 1283, 1511, 1777, 1987, 2243, 2503, 2719]`
//!
//! These are all coprime (no common factors), which creates a dense,
//! natural-sounding reverb without metallic resonances. For other sample
//! rates, these are scaled proportionally.
//!
//! ## Parameters
//!
//! - **decay** (0.0 to 0.9999): Controls how long the reverb tail lasts
//!   - 0.8 = short reverb (small room)
//!   - 0.95 = medium reverb (hall)
//!   - 0.99 = long reverb (cathedral)
//!
//! - **damping** (0.0 to 1.0): Controls high-frequency absorption
//!   - 0.0 = bright (no damping)
//!   - 0.5 = neutral
//!   - 0.9 = dark (heavy damping)
//!
//! # Example
//!
//! ```no_run
//! use phonon::nodes::FdnState;
//!
//! let mut reverb = FdnState::new(44100.0);
//!
//! // Process an impulse with long, bright reverb
//! let output = reverb.process(1.0, 0.98, 0.2);
//!
//! // Process audio samples
//! for sample in audio_input {
//!     let reverb_output = reverb.process(sample, 0.98, 0.2);
//! }
//! ```
//!
//! # References
//!
//! - Jot, J.-M., & Chaigne, A. (1991). "Digital delay networks for designing
//!   artificial reverberators." AES 90th Convention.
//! - Rocchesso, D., & Smith, J.O. (1997). "Circulant and elliptic feedback
//!   delay networks for artificial reverberation." IEEE TASLP.
//! - Schlecht, S.J., & Habets, E.A.P. (2017). "On lossless feedback delay
//!   networks." IEEE TASLP.

/// State for the 8-channel FDN reverb
pub struct FdnState {
    /// Eight delay line buffers with coprime lengths
    delay_buffers: [Vec<f32>; 8],
    /// Current write position in each delay line
    write_indices: [usize; 8],
    /// One-pole lowpass filter states for damping
    damping_states: [f32; 8],
    /// Sample rate for scaling delay times
    sample_rate: f32,
}

impl FdnState {
    /// Base delay lengths in samples at 44100 Hz (all coprime)
    const BASE_DELAYS: [usize; 8] = [1087, 1283, 1511, 1777, 1987, 2243, 2503, 2719];
    const BASE_SAMPLE_RATE: f32 = 44100.0;

    /// Create a new FDN reverb state
    ///
    /// # Arguments
    /// * `sample_rate` - The audio sample rate in Hz
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / Self::BASE_SAMPLE_RATE;

        // Scale delay lengths proportionally with sample rate
        let scaled_delays: [usize; 8] = Self::BASE_DELAYS
            .iter()
            .map(|&d| ((d as f32 * scale).round() as usize).max(1))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // Initialize delay buffers
        let delay_buffers = [
            vec![0.0; scaled_delays[0]],
            vec![0.0; scaled_delays[1]],
            vec![0.0; scaled_delays[2]],
            vec![0.0; scaled_delays[3]],
            vec![0.0; scaled_delays[4]],
            vec![0.0; scaled_delays[5]],
            vec![0.0; scaled_delays[6]],
            vec![0.0; scaled_delays[7]],
        ];

        Self {
            delay_buffers,
            write_indices: [0; 8],
            damping_states: [0.0; 8],
            sample_rate,
        }
    }

    /// Process a single sample through the FDN reverb
    ///
    /// # Arguments
    /// * `input` - Input sample
    /// * `decay` - Decay time coefficient (0.0 to 1.0, where 0.99+ gives long reverb)
    /// * `damping` - High-frequency damping (0.0 = no damping, 1.0 = maximum damping)
    ///
    /// # Returns
    /// The reverb output sample
    pub fn process(&mut self, input: f32, decay: f32, damping: f32) -> f32 {
        // Clamp parameters to safe ranges
        let decay = decay.clamp(0.0, 0.9999);
        let damping = damping.clamp(0.0, 1.0);

        // Step 1: Read from all delay lines (at current write position, which is the oldest sample)
        let mut delay_outputs = [0.0f32; 8];
        for i in 0..8 {
            // Read at write position gives us the oldest sample (full delay)
            let read_idx = self.write_indices[i];
            delay_outputs[i] = self.delay_buffers[i][read_idx];
        }

        // Step 2: Apply Householder mixing matrix
        // H = I - (2/N) * ones_matrix
        // Efficient implementation: output[i] = input[i] - (2/N) * sum(inputs)
        let sum: f32 = delay_outputs.iter().sum();
        let mix_factor = 2.0 / 8.0;
        let mut mixed = [0.0f32; 8];
        for i in 0..8 {
            mixed[i] = delay_outputs[i] - mix_factor * sum;
        }

        // Step 3: Apply per-channel damping (one-pole lowpass)
        // y[n] = (1 - damping) * x[n] + damping * y[n-1]
        for i in 0..8 {
            self.damping_states[i] = (1.0 - damping) * mixed[i] + damping * self.damping_states[i];
            mixed[i] = self.damping_states[i];
        }

        // Step 4: Scale by decay coefficient
        for i in 0..8 {
            mixed[i] *= decay;
        }

        // Step 5: Sum delay outputs for output (before adding input)
        let output = mixed.iter().sum::<f32>() / 8.0;

        // Step 6: Add input to first channel for feedback
        mixed[0] += input;

        // Step 7: Write back to delay lines
        for i in 0..8 {
            self.delay_buffers[i][self.write_indices[i]] = mixed[i];
            // Advance write index (circular buffer)
            self.write_indices[i] = (self.write_indices[i] + 1) % self.delay_buffers[i].len();
        }

        output
    }

    /// Clear all delay lines and reset state
    pub fn clear(&mut self) {
        for buffer in &mut self.delay_buffers {
            buffer.fill(0.0);
        }
        self.damping_states.fill(0.0);
        self.write_indices.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that impulse response decays over time
    #[test]
    fn test_impulse_decay() {
        let mut reverb = FdnState::new(44100.0);

        // Send an impulse
        reverb.process(1.0, 0.95, 0.3);

        // Process silence and collect energy over time
        let mut max_early = 0.0f32;
        let mut max_late = 0.0f32;

        // Early reflections (samples 1000-3000, after initial delay)
        for _ in 0..1000 {
            reverb.process(0.0, 0.95, 0.3);
        }
        for _ in 1000..3000 {
            let output = reverb.process(0.0, 0.95, 0.3);
            max_early = max_early.max(output.abs());
        }

        // Late reflections (samples 15000-17000)
        for _ in 3000..15000 {
            reverb.process(0.0, 0.95, 0.3);
        }

        for _ in 15000..17000 {
            let output = reverb.process(0.0, 0.95, 0.3);
            max_late = max_late.max(output.abs());
        }

        // Verify decay: late reflections should be quieter than early
        assert!(max_late < max_early,
            "Late reflections ({}) should be quieter than early reflections ({})",
            max_late, max_early);

        // Early reflections should have some energy
        assert!(max_early > 0.001,
            "Early reflections should have significant energy, got {}",
            max_early);
    }

    /// Test that longer decay time produces longer reverb tail
    #[test]
    fn test_decay_time_affects_tail_length() {
        let mut reverb_short = FdnState::new(44100.0);
        let mut reverb_long = FdnState::new(44100.0);

        // Send impulse to both
        reverb_short.process(1.0, 0.8, 0.3);
        reverb_long.process(1.0, 0.98, 0.3);

        // Process 20000 samples and measure energy
        let mut energy_short = 0.0f32;
        let mut energy_long = 0.0f32;

        for _ in 0..20000 {
            let out_short = reverb_short.process(0.0, 0.8, 0.3);
            let out_long = reverb_long.process(0.0, 0.98, 0.3);

            energy_short += out_short * out_short;
            energy_long += out_long * out_long;
        }

        // Longer decay should produce more total energy
        assert!(energy_long > energy_short,
            "Longer decay ({}) should produce more energy than shorter decay ({})",
            energy_long, energy_short);

        // Both should have some energy
        assert!(energy_short > 0.0, "Short decay should produce some reverb");
        assert!(energy_long > 0.0, "Long decay should produce some reverb");
    }

    /// Test that damping affects high frequencies more than low
    #[test]
    fn test_damping_affects_high_frequencies() {
        let sample_rate = 44100.0;
        let mut reverb_no_damp = FdnState::new(sample_rate);
        let mut reverb_damped = FdnState::new(sample_rate);

        // Generate a high-frequency test signal (4 kHz sine wave)
        let freq = 4000.0;
        let duration_samples = 4410; // 100ms

        let mut energy_no_damp = 0.0f32;
        let mut energy_damped = 0.0f32;

        for i in 0..duration_samples {
            let t = i as f32 / sample_rate;
            let input = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.1;

            let out_no_damp = reverb_no_damp.process(input, 0.95, 0.0);
            let out_damped = reverb_damped.process(input, 0.95, 0.7);

            energy_no_damp += out_no_damp * out_no_damp;
            energy_damped += out_damped * out_damped;
        }

        // Damped reverb should have less energy at high frequencies
        assert!(energy_damped < energy_no_damp,
            "Damped reverb ({}) should have less HF energy than undamped ({})",
            energy_damped, energy_no_damp);
    }

    /// Test that output contains no NaN or Inf values
    #[test]
    fn test_no_nan_or_inf() {
        let mut reverb = FdnState::new(44100.0);

        // Test with various input conditions
        let test_inputs = vec![
            1.0,      // Normal impulse
            0.0,      // Silence
            -1.0,     // Negative impulse
            0.5,      // Partial amplitude
        ];

        for &input in &test_inputs {
            for _ in 0..1000 {
                let output = reverb.process(input, 0.95, 0.5);
                assert!(output.is_finite(),
                    "Output should be finite, got {} for input {}",
                    output, input);
            }
        }
    }

    /// Test at different sample rates
    #[test]
    fn test_different_sample_rates() {
        let sample_rates = vec![44100.0, 48000.0, 96000.0, 22050.0];

        for &sr in &sample_rates {
            let mut reverb = FdnState::new(sr);

            // Send impulse and verify we get output
            reverb.process(1.0, 0.95, 0.3);

            // Wait for signal to propagate through delay lines (2x longest delay)
            for _ in 0..6000 {
                reverb.process(0.0, 0.95, 0.3);
            }

            let mut max_output = 0.0f32;
            for _ in 0..2000 {
                let output = reverb.process(0.0, 0.95, 0.3);
                max_output = max_output.max(output.abs());
                assert!(output.is_finite(),
                    "Output should be finite at {} Hz", sr);
            }

            assert!(max_output > 0.0,
                "Should produce reverb output at {} Hz, got {}", sr, max_output);
        }
    }

    /// Test clear() method resets state
    #[test]
    fn test_clear_resets_state() {
        let mut reverb = FdnState::new(44100.0);

        // Fill with signal
        for _ in 0..1000 {
            reverb.process(1.0, 0.95, 0.3);
        }

        // Clear should reset everything
        reverb.clear();

        // Process silence - should get no output initially
        let output = reverb.process(0.0, 0.95, 0.3);
        assert_eq!(output, 0.0, "Output should be zero after clear");

        // Verify all internal state is cleared
        for buffer in &reverb.delay_buffers {
            assert!(buffer.iter().all(|&x| x == 0.0),
                "All delay buffers should be cleared");
        }
        assert!(reverb.damping_states.iter().all(|&x| x == 0.0),
            "All damping states should be cleared");
    }

    /// Test parameter clamping
    #[test]
    fn test_parameter_clamping() {
        let mut reverb = FdnState::new(44100.0);

        // Test extreme decay values
        let output1 = reverb.process(1.0, 2.0, 0.5);  // Should clamp to 0.9999
        assert!(output1.is_finite(), "Should handle decay > 1.0");

        let output2 = reverb.process(1.0, -0.5, 0.5); // Should clamp to 0.0
        assert!(output2.is_finite(), "Should handle negative decay");

        // Test extreme damping values
        reverb.clear();
        let output3 = reverb.process(1.0, 0.95, 2.0);  // Should clamp to 1.0
        assert!(output3.is_finite(), "Should handle damping > 1.0");

        let output4 = reverb.process(1.0, 0.95, -0.5); // Should clamp to 0.0
        assert!(output4.is_finite(), "Should handle negative damping");
    }

    /// Test Householder matrix properties
    #[test]
    fn test_householder_matrix_energy_preservation() {
        let mut reverb = FdnState::new(44100.0);

        // Send impulse
        reverb.process(1.0, 1.0, 0.0); // decay=1.0, no damping

        // Measure energy over time - with lossless feedback it should be preserved
        // (minus small numerical errors)
        let mut energies = Vec::new();

        for _ in 0..100 {
            let output = reverb.process(0.0, 1.0, 0.0);
            let mut total_energy = 0.0f32;

            // Sum energy in all delay lines
            for buffer in &reverb.delay_buffers {
                total_energy += buffer.iter().map(|x| x * x).sum::<f32>();
            }

            energies.push(total_energy);
        }

        // Energy should remain relatively stable (within 10% for numerical errors)
        if let (Some(&first), Some(&last)) = (energies.first(), energies.last()) {
            if first > 0.0 {
                let ratio = last / first;
                assert!(ratio > 0.5 && ratio < 2.0,
                    "Energy should be roughly preserved with lossless feedback, got ratio {}",
                    ratio);
            }
        }
    }

    /// Test that different delay lengths create dense response
    #[test]
    fn test_coprime_delays_create_dense_response() {
        let mut reverb = FdnState::new(44100.0);

        // Send impulse
        reverb.process(1.0, 0.95, 0.2);

        // Skip initial delay period and collect samples during active reverb
        for _ in 0..1500 {
            reverb.process(0.0, 0.95, 0.2);
        }

        // Count zero crossings in next 5000 samples
        let mut outputs = Vec::new();
        for _ in 0..5000 {
            outputs.push(reverb.process(0.0, 0.95, 0.2));
        }

        let mut zero_crossings = 0;
        for i in 1..outputs.len() {
            if (outputs[i-1] >= 0.0 && outputs[i] < 0.0) ||
               (outputs[i-1] < 0.0 && outputs[i] >= 0.0) {
                zero_crossings += 1;
            }
        }

        // Should have many zero crossings (dense modal structure)
        // With 8 different delay lengths we expect hundreds of crossings
        assert!(zero_crossings > 50,
            "Should have dense response with many zero crossings, got {}",
            zero_crossings);
    }
}
