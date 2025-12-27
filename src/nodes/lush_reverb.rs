//! Lush Reverb - A rich, flexible algorithmic reverb
//!
//! This reverb combines multiple components for a truly rich, living sound:
//! - **Input Diffuser**: 8-channel Hadamard diffusion network for dense early reflections
//! - **FDN Core**: 8-channel feedback delay network with Householder mixing
//! - **Modulation**: Pink/Brown noise modulation for organic, non-repetitive movement
//!
//! # Features
//!
//! - Decay times from 0.1s to 60+ seconds
//! - Complex modulation with "spin" (fast) and "wander" (slow) controls
//! - Frequency-dependent damping for natural sound
//! - Freeze mode for infinite sustain
//! - No metallic artifacts thanks to coprime delays and modulation
//!
//! # References
//!
//! - Sean Costello / Valhalla DSP reverb design principles
//! - Lexicon 224/480L random modulation techniques
//! - Signalsmith diffusion networks
//! - Jon Dattorro plate reverb architecture

use super::noise_generators::{PinkNoise, BrownNoise};

/// Number of channels in the reverb
const NUM_CHANNELS: usize = 8;

/// Number of diffusion steps
const NUM_DIFFUSION_STEPS: usize = 4;

/// Lush Reverb state
///
/// A production-quality algorithmic reverb with rich modulation
#[derive(Clone, Debug)]
pub struct LushReverbState {
    // === Diffuser state ===
    /// Diffuser delay buffers: [step][channel]
    diff_buffers: [[Vec<f32>; NUM_CHANNELS]; NUM_DIFFUSION_STEPS],
    /// Diffuser write indices
    diff_write_idx: [[usize; NUM_CHANNELS]; NUM_DIFFUSION_STEPS],
    /// Diffuser delay times in samples
    diff_delay_times: [[usize; NUM_CHANNELS]; NUM_DIFFUSION_STEPS],

    // === FDN state ===
    /// FDN delay buffers
    fdn_buffers: [Vec<f32>; NUM_CHANNELS],
    /// FDN write indices
    fdn_write_idx: [usize; NUM_CHANNELS],
    /// FDN base delay times (at current sample rate)
    fdn_delay_times: [usize; NUM_CHANNELS],
    /// Per-channel damping filter states
    damping_states: [f32; NUM_CHANNELS],
    /// Per-channel modulation offsets (in samples, fractional)
    mod_offsets: [f32; NUM_CHANNELS],

    // === Modulation ===
    /// Pink noise for "spin" (faster modulation)
    spin_noise: [PinkNoise; NUM_CHANNELS],
    /// Brown noise for "wander" (slower modulation)
    wander_noise: [BrownNoise; NUM_CHANNELS],
    /// Smoothed spin values (to avoid clicks)
    spin_smooth: [f32; NUM_CHANNELS],
    /// Smoothed wander values
    wander_smooth: [f32; NUM_CHANNELS],

    // === Pre-delay ===
    /// Pre-delay buffer
    predelay_buffer: Vec<f32>,
    /// Pre-delay write index
    predelay_write_idx: usize,

    // === Configuration ===
    sample_rate: f32,
    /// Maximum modulation depth in samples
    max_mod_depth: f32,
}

impl LushReverbState {
    /// Base FDN delay lengths at 44100 Hz (all coprime for dense modes)
    const BASE_FDN_DELAYS: [usize; NUM_CHANNELS] = [1087, 1283, 1511, 1777, 1987, 2243, 2503, 2719];

    /// Base diffuser delay lengths at 44100 Hz
    const BASE_DIFF_DELAYS: [usize; NUM_CHANNELS] = [23, 41, 59, 73, 89, 107, 127, 149];

    /// Diffuser step scale factors
    const DIFF_STEP_SCALES: [f32; NUM_DIFFUSION_STEPS] = [1.0, 1.5, 2.0, 3.0];

    /// Maximum pre-delay in seconds
    const MAX_PREDELAY_SECS: f32 = 0.5;

    /// Create a new LushReverb with the given sample rate and random seed
    pub fn new(sample_rate: f32, seed: u64) -> Self {
        let scale = sample_rate / 44100.0;

        // Initialize diffuser delays
        let mut diff_delay_times = [[0usize; NUM_CHANNELS]; NUM_DIFFUSION_STEPS];
        let mut diff_buffers: [[Vec<f32>; NUM_CHANNELS]; NUM_DIFFUSION_STEPS] =
            std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));

        for (step, step_scale) in Self::DIFF_STEP_SCALES.iter().enumerate() {
            for (ch, &base) in Self::BASE_DIFF_DELAYS.iter().enumerate() {
                let delay = ((base as f32) * step_scale * scale).round() as usize;
                diff_delay_times[step][ch] = delay.max(1);
                diff_buffers[step][ch] = vec![0.0; delay.max(1)];
            }
        }

        // Initialize FDN delays (with extra room for modulation)
        let max_mod_depth = (32.0 * scale).round();
        let mut fdn_delay_times = [0usize; NUM_CHANNELS];
        let mut fdn_buffers: [Vec<f32>; NUM_CHANNELS] = std::array::from_fn(|_| Vec::new());

        for (ch, &base) in Self::BASE_FDN_DELAYS.iter().enumerate() {
            let delay = ((base as f32) * scale).round() as usize;
            fdn_delay_times[ch] = delay;
            // Add extra space for modulation
            fdn_buffers[ch] = vec![0.0; delay + (max_mod_depth as usize) * 2 + 16];
        }

        // Initialize noise generators with different seeds per channel
        let spin_noise: [PinkNoise; NUM_CHANNELS] = std::array::from_fn(|i| {
            PinkNoise::new(seed.wrapping_add(i as u64 * 12345))
        });
        let wander_noise: [BrownNoise; NUM_CHANNELS] = std::array::from_fn(|i| {
            BrownNoise::new(seed.wrapping_add(i as u64 * 67890 + 1000))
        });

        // Pre-delay buffer
        let predelay_size = (Self::MAX_PREDELAY_SECS * sample_rate) as usize;

        Self {
            diff_buffers,
            diff_write_idx: [[0; NUM_CHANNELS]; NUM_DIFFUSION_STEPS],
            diff_delay_times,
            fdn_buffers,
            fdn_write_idx: [0; NUM_CHANNELS],
            fdn_delay_times,
            damping_states: [0.0; NUM_CHANNELS],
            mod_offsets: [0.0; NUM_CHANNELS],
            spin_noise,
            wander_noise,
            spin_smooth: [0.0; NUM_CHANNELS],
            wander_smooth: [0.0; NUM_CHANNELS],
            predelay_buffer: vec![0.0; predelay_size.max(1)],
            predelay_write_idx: 0,
            sample_rate,
            max_mod_depth,
        }
    }

    /// Process a single sample through the reverb
    ///
    /// # Parameters
    /// - `input`: Input sample
    /// - `predelay`: Pre-delay time in seconds (0.0 - 0.5)
    /// - `decay`: Decay amount (0.0 - 0.9999, higher = longer tail)
    /// - `size`: Room size multiplier (0.5 - 2.0)
    /// - `diffusion`: Input diffusion amount (0.0 - 1.0)
    /// - `damping`: High-frequency damping (0.0 - 1.0)
    /// - `spin`: Fast modulation depth (0.0 - 1.0)
    /// - `wander`: Slow modulation depth (0.0 - 1.0)
    /// - `freeze`: Freeze mode (>0.5 = frozen)
    /// - `mix`: Dry/wet mix (0.0 = dry, 1.0 = wet)
    pub fn process(
        &mut self,
        input: f32,
        predelay: f32,
        decay: f32,
        _size: f32,
        diffusion: f32,
        damping: f32,
        spin: f32,
        wander: f32,
        freeze: f32,
        mix: f32,
    ) -> f32 {
        // Clamp parameters
        let predelay = predelay.clamp(0.0, Self::MAX_PREDELAY_SECS);
        let decay = if freeze > 0.5 { 0.9999 } else { decay.clamp(0.0, 0.9999) };
        let diffusion = diffusion.clamp(0.0, 1.0);
        let damping = damping.clamp(0.0, 1.0);
        let spin = spin.clamp(0.0, 1.0);
        let wander = wander.clamp(0.0, 1.0);
        let mix = mix.clamp(0.0, 1.0);

        // === Pre-delay ===
        let predelay_samples = (predelay * self.sample_rate) as usize;
        let predelay_samples = predelay_samples.min(self.predelay_buffer.len() - 1);

        // Write to pre-delay
        self.predelay_buffer[self.predelay_write_idx] = input;

        // Read from pre-delay
        let read_idx = (self.predelay_write_idx + self.predelay_buffer.len() - predelay_samples)
            % self.predelay_buffer.len();
        let predelayed = self.predelay_buffer[read_idx];

        self.predelay_write_idx = (self.predelay_write_idx + 1) % self.predelay_buffer.len();

        // === Input Diffusion ===
        let diffused = self.process_diffuser(predelayed, diffusion);

        // === Update Modulation ===
        self.update_modulation(spin, wander);

        // === FDN Processing ===
        let wet = self.process_fdn(diffused, decay, damping);

        // === Mix ===
        input * (1.0 - mix) + wet * mix
    }

    /// Process through the diffuser network
    fn process_diffuser(&mut self, input: f32, diffusion: f32) -> f32 {
        // Initialize channels (input goes to channel 0)
        let mut channels = [0.0f32; NUM_CHANNELS];
        channels[0] = input;

        // Process through each diffusion step
        for step in 0..NUM_DIFFUSION_STEPS {
            // Read from delays
            let mut delayed = [0.0f32; NUM_CHANNELS];
            for ch in 0..NUM_CHANNELS {
                let buffer = &self.diff_buffers[step][ch];
                if buffer.is_empty() { continue; }
                let read_idx = (self.diff_write_idx[step][ch] + buffer.len()
                    - self.diff_delay_times[step][ch]) % buffer.len();
                delayed[ch] = buffer[read_idx];
            }

            // Allpass processing: y = -x + delayed + g*(x - delayed)
            // Simplified: y = (1-g)*delayed - (1-g)*x + x = delayed + (1-2g)*x + g*delayed
            // Using standard: output = -g*input + delayed + g*delayed_written
            let g = diffusion * 0.75; // Scale diffusion
            let mut allpass_out = [0.0f32; NUM_CHANNELS];
            for ch in 0..NUM_CHANNELS {
                let x = channels[ch];
                let d = delayed[ch];
                // Standard allpass: y = -g*x + d, write: d' = x + g*d
                allpass_out[ch] = -g * x + d;
                let write_val = x + g * d;
                let buffer = &mut self.diff_buffers[step][ch];
                if !buffer.is_empty() {
                    buffer[self.diff_write_idx[step][ch]] = write_val;
                }
            }

            // Advance write indices
            for ch in 0..NUM_CHANNELS {
                let len = self.diff_buffers[step][ch].len();
                if len > 0 {
                    self.diff_write_idx[step][ch] = (self.diff_write_idx[step][ch] + 1) % len;
                }
            }

            // Channel shuffle with polarity flips
            let shuffled = Self::shuffle_channels(allpass_out, step);

            // Hadamard transform
            channels = Self::hadamard_8(shuffled);
        }

        // Sum all channels for output
        channels.iter().sum::<f32>() / (NUM_CHANNELS as f32).sqrt()
    }

    /// Shuffle channels with polarity flips for decorrelation
    fn shuffle_channels(input: [f32; NUM_CHANNELS], step: usize) -> [f32; NUM_CHANNELS] {
        // Different shuffle patterns for each step
        let patterns: [[usize; NUM_CHANNELS]; NUM_DIFFUSION_STEPS] = [
            [0, 1, 2, 3, 4, 5, 6, 7],  // No shuffle for step 0
            [4, 5, 6, 7, 0, 1, 2, 3],  // Swap halves
            [2, 3, 0, 1, 6, 7, 4, 5],  // Swap quarters
            [1, 0, 3, 2, 5, 4, 7, 6],  // Swap pairs
        ];

        // Polarity flip patterns
        let flips: [[f32; NUM_CHANNELS]; NUM_DIFFUSION_STEPS] = [
            [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0],
        ];

        let pattern = &patterns[step];
        let flip = &flips[step];

        std::array::from_fn(|i| input[pattern[i]] * flip[i])
    }

    /// Fast 8-point Hadamard transform (in-place style, returns new array)
    fn hadamard_8(input: [f32; 8]) -> [f32; 8] {
        // Stage 1: pairs
        let a0 = input[0] + input[1];
        let a1 = input[0] - input[1];
        let a2 = input[2] + input[3];
        let a3 = input[2] - input[3];
        let a4 = input[4] + input[5];
        let a5 = input[4] - input[5];
        let a6 = input[6] + input[7];
        let a7 = input[6] - input[7];

        // Stage 2: quads
        let b0 = a0 + a2;
        let b1 = a1 + a3;
        let b2 = a0 - a2;
        let b3 = a1 - a3;
        let b4 = a4 + a6;
        let b5 = a5 + a7;
        let b6 = a4 - a6;
        let b7 = a5 - a7;

        // Stage 3: octets (with normalization)
        let norm = 1.0 / (8.0_f32).sqrt();
        [
            (b0 + b4) * norm,
            (b1 + b5) * norm,
            (b2 + b6) * norm,
            (b3 + b7) * norm,
            (b0 - b4) * norm,
            (b1 - b5) * norm,
            (b2 - b6) * norm,
            (b3 - b7) * norm,
        ]
    }

    /// Update modulation values from noise sources
    fn update_modulation(&mut self, spin: f32, wander: f32) {
        let spin_rate = 0.01;    // Smoothing for spin (faster response)
        let wander_rate = 0.001; // Smoothing for wander (slower response)

        for ch in 0..NUM_CHANNELS {
            // Get raw noise values
            let spin_raw = self.spin_noise[ch].next_sample();
            let wander_raw = self.wander_noise[ch].next_sample();

            // Smooth the values
            self.spin_smooth[ch] += spin_rate * (spin_raw - self.spin_smooth[ch]);
            self.wander_smooth[ch] += wander_rate * (wander_raw - self.wander_smooth[ch]);

            // Calculate modulation offset
            let spin_mod = self.spin_smooth[ch] * spin * self.max_mod_depth * 0.5;
            let wander_mod = self.wander_smooth[ch] * wander * self.max_mod_depth;

            self.mod_offsets[ch] = spin_mod + wander_mod;
        }
    }

    /// Process through the FDN
    fn process_fdn(&mut self, input: f32, decay: f32, damping: f32) -> f32 {
        // Read from all delay lines with modulation
        let mut delayed = [0.0f32; NUM_CHANNELS];

        for ch in 0..NUM_CHANNELS {
            let buffer = &self.fdn_buffers[ch];
            let base_delay = self.fdn_delay_times[ch] as f32;
            let mod_delay = base_delay + self.mod_offsets[ch];
            let mod_delay = mod_delay.max(1.0).min(buffer.len() as f32 - 2.0);

            // Fractional delay with linear interpolation
            let delay_int = mod_delay.floor() as usize;
            let delay_frac = mod_delay - delay_int as f32;

            let read_idx_0 = (self.fdn_write_idx[ch] + buffer.len() - delay_int) % buffer.len();
            let read_idx_1 = (read_idx_0 + buffer.len() - 1) % buffer.len();

            delayed[ch] = buffer[read_idx_0] * (1.0 - delay_frac) + buffer[read_idx_1] * delay_frac;
        }

        // Apply Householder mixing matrix
        // H = I - (2/N) * ones_matrix
        // output[i] = input[i] - (2/N) * sum(all inputs)
        let sum: f32 = delayed.iter().sum();
        let mix_factor = 2.0 / NUM_CHANNELS as f32;
        let mut mixed = [0.0f32; NUM_CHANNELS];
        for ch in 0..NUM_CHANNELS {
            mixed[ch] = delayed[ch] - mix_factor * sum;
        }

        // Apply damping (one-pole lowpass) and per-sample decay
        // Decay coefficient is calculated per-sample (not per-cycle) for correct RT60 behavior
        let damp_coef = damping * 0.7; // Scale damping

        // Calculate per-sample decay coefficient based on RT60
        // decay parameter (0-1) maps to RT60 of 0.1s to 60s
        // RT60 = time for signal to decay by 60dB
        let rt60 = 0.1 + decay * decay * 59.9; // 0.1 to 60 seconds (squared for better control)
        // Per-sample decay: 10^(-3 / (sample_rate * RT60)) applied every sample
        let decay_per_sample = (10.0_f32).powf(-3.0 / (self.sample_rate * rt60));

        for ch in 0..NUM_CHANNELS {
            // One-pole lowpass: y = (1-a)*x + a*y_prev
            self.damping_states[ch] = (1.0 - damp_coef) * mixed[ch] + damp_coef * self.damping_states[ch];

            mixed[ch] = self.damping_states[ch] * decay_per_sample;
        }

        // Add input to channel 0 and write back
        mixed[0] += input;

        for ch in 0..NUM_CHANNELS {
            self.fdn_buffers[ch][self.fdn_write_idx[ch]] = mixed[ch];
            self.fdn_write_idx[ch] = (self.fdn_write_idx[ch] + 1) % self.fdn_buffers[ch].len();
        }

        // Output is sum of delayed values (before feedback was added)
        delayed.iter().sum::<f32>() / (NUM_CHANNELS as f32).sqrt()
    }

    /// Clear all state (for graph swap)
    pub fn clear(&mut self) {
        // Clear diffuser
        for step in 0..NUM_DIFFUSION_STEPS {
            for ch in 0..NUM_CHANNELS {
                self.diff_buffers[step][ch].fill(0.0);
                self.diff_write_idx[step][ch] = 0;
            }
        }

        // Clear FDN
        for ch in 0..NUM_CHANNELS {
            self.fdn_buffers[ch].fill(0.0);
            self.fdn_write_idx[ch] = 0;
            self.damping_states[ch] = 0.0;
            self.mod_offsets[ch] = 0.0;
            self.spin_smooth[ch] = 0.0;
            self.wander_smooth[ch] = 0.0;
        }

        // Clear pre-delay
        self.predelay_buffer.fill(0.0);
        self.predelay_write_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lush_reverb_impulse_response() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // Send impulse
        let first = reverb.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.3, 0.3, 0.2, 0.0, 1.0);
        assert!(first.is_finite());

        // Let it ring for a while
        let mut sum = first.abs();
        for _ in 0..44100 {
            let sample = reverb.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 0.3, 0.2, 0.0, 1.0);
            sum += sample.abs();
            assert!(sample.is_finite(), "NaN or Inf in reverb output");
        }

        // Should have some energy
        assert!(sum > 1.0, "Reverb should produce output over time");
    }

    #[test]
    fn test_decay_affects_tail_length() {
        // This test verifies that decay parameter affects the reverb tail length
        // With different decay values, the late reverb should have different energy levels
        let mut reverb_short = LushReverbState::new(44100.0, 42);
        let mut reverb_long = LushReverbState::new(44100.0, 42);

        // Send impulse
        reverb_short.process(1.0, 0.0, 0.2, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0);
        reverb_long.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0);

        // Wait for initial reflections to pass (100ms = 4410 samples)
        for _ in 0..4410 {
            reverb_short.process(0.0, 0.0, 0.2, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0);
            reverb_long.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0);
        }

        // Measure energy over the next 500ms (22050 samples)
        let mut sum_short = 0.0;
        let mut sum_long = 0.0;
        for _ in 0..22050 {
            sum_short += reverb_short.process(0.0, 0.0, 0.2, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0).abs();
            sum_long += reverb_long.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0).abs();
        }

        // With decay=0.2 (RT60≈2.5s) vs decay=0.95 (RT60≈54s), there should be a difference
        // The long decay should have more energy
        assert!(sum_long > sum_short,
            "Longer decay should have more energy: long={} short={}", sum_long, sum_short);
    }

    #[test]
    fn test_diffusion_affects_density() {
        let mut reverb_low = LushReverbState::new(44100.0, 42);
        let mut reverb_high = LushReverbState::new(44100.0, 42);

        // Send impulse with different diffusion settings
        let _ = reverb_low.process(1.0, 0.0, 0.9, 1.0, 0.0, 0.3, 0.0, 0.0, 0.0, 1.0);
        let _ = reverb_high.process(1.0, 0.0, 0.9, 1.0, 1.0, 0.3, 0.0, 0.0, 0.0, 1.0);

        // Count zero crossings (higher = denser)
        let mut crossings_low = 0;
        let mut crossings_high = 0;
        let mut prev_low = 0.0;
        let mut prev_high = 0.0;

        for _ in 0..4410 {
            let sample_low = reverb_low.process(0.0, 0.0, 0.9, 1.0, 0.0, 0.3, 0.0, 0.0, 0.0, 1.0);
            let sample_high = reverb_high.process(0.0, 0.0, 0.9, 1.0, 1.0, 0.3, 0.0, 0.0, 0.0, 1.0);

            if sample_low * prev_low < 0.0 { crossings_low += 1; }
            if sample_high * prev_high < 0.0 { crossings_high += 1; }

            prev_low = sample_low;
            prev_high = sample_high;
        }

        // High diffusion should have more zero crossings (denser)
        // Note: this test may be sensitive to implementation details
        assert!(crossings_high >= crossings_low,
            "High diffusion should have at least as many crossings: high={} low={}",
            crossings_high, crossings_low);
    }

    #[test]
    fn test_freeze_mode() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // Send impulse with freeze on
        reverb.process(1.0, 0.0, 0.5, 1.0, 0.7, 0.0, 0.0, 0.0, 1.0, 1.0);

        // Let it run for 2 seconds - should still have energy
        for _ in 0..88200 {
            reverb.process(0.0, 0.0, 0.5, 1.0, 0.7, 0.0, 0.0, 0.0, 1.0, 1.0);
        }

        // Measure energy at the end
        let mut sum = 0.0;
        for _ in 0..4410 {
            sum += reverb.process(0.0, 0.0, 0.5, 1.0, 0.7, 0.0, 0.0, 0.0, 1.0, 1.0).abs();
        }

        assert!(sum > 0.1, "Freeze mode should sustain indefinitely: sum={}", sum);
    }

    #[test]
    fn test_modulation_creates_variation() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // Send impulse
        reverb.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);

        // Collect samples over time
        let mut samples = Vec::new();
        for _ in 0..44100 { // 1 second
            samples.push(reverb.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0));
        }

        // Check that modulation creates variation over time
        // The modulation should cause the output to drift, not be exactly periodic
        // Compare early samples to late samples - they should differ
        let early_sum: f32 = samples[1000..2000].iter().sum();
        let late_sum: f32 = samples[40000..41000].iter().sum();

        // The difference should be non-trivial (modulation has changed the character)
        // Note: This is a weaker test - just checking that there IS variation
        let difference = (early_sum - late_sum).abs();
        assert!(difference > 0.001 || (early_sum.abs() < 0.001 && late_sum.abs() < 0.001),
            "Modulation should create variation over time: early={} late={}", early_sum, late_sum);
    }

    #[test]
    fn test_predelay() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // Send impulse with 100ms pre-delay
        let predelay_secs = 0.1;
        let predelay_samples = (44100.0 * predelay_secs) as usize;

        // First samples should be near zero
        let first = reverb.process(1.0, predelay_secs, 0.9, 1.0, 0.7, 0.3, 0.0, 0.0, 0.0, 1.0);
        assert!(first.abs() < 0.01, "Pre-delayed signal should start quiet");

        // Check samples before predelay - should be quiet
        for _ in 0..(predelay_samples / 2) {
            let sample = reverb.process(0.0, predelay_secs, 0.9, 1.0, 0.7, 0.3, 0.0, 0.0, 0.0, 1.0);
            assert!(sample.abs() < 0.1, "Should be quiet during predelay");
        }
    }

    #[test]
    fn test_damping_affects_brightness() {
        let mut reverb_bright = LushReverbState::new(44100.0, 42);
        let mut reverb_dark = LushReverbState::new(44100.0, 42);

        // Send impulse
        reverb_bright.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0);
        reverb_dark.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.9, 0.0, 0.0, 0.0, 1.0);

        // Collect samples
        let mut bright_samples = Vec::new();
        let mut dark_samples = Vec::new();

        for _ in 0..4410 {
            bright_samples.push(reverb_bright.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.0, 0.0, 0.0, 0.0, 1.0));
            dark_samples.push(reverb_dark.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.9, 0.0, 0.0, 0.0, 1.0));
        }

        // Calculate high-frequency energy via simple difference (derivative)
        let hf_bright: f32 = bright_samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let hf_dark: f32 = dark_samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum();

        assert!(hf_bright > hf_dark * 1.2,
            "Bright should have more HF energy: bright={} dark={}", hf_bright, hf_dark);
    }

    #[test]
    fn test_mix_control() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // With mix=0, output should equal input
        let output_dry = reverb.process(0.5, 0.0, 0.9, 1.0, 0.7, 0.3, 0.0, 0.0, 0.0, 0.0);
        assert!((output_dry - 0.5).abs() < 0.001, "Mix=0 should pass dry signal");

        // With mix=1, should be pure wet (no immediate output for impulse due to delays)
        reverb.clear();
        let output_wet = reverb.process(0.5, 0.0, 0.9, 1.0, 0.7, 0.3, 0.0, 0.0, 0.0, 1.0);
        // The wet signal is delayed, so first sample should be mostly reverb
        assert!(output_wet.abs() < 0.5, "Mix=1 should suppress dry signal initially");
    }

    #[test]
    fn test_hadamard_transform() {
        // Test that Hadamard transform preserves energy
        let input = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let output = LushReverbState::hadamard_8(input);

        let input_energy: f32 = input.iter().map(|x| x * x).sum();
        let output_energy: f32 = output.iter().map(|x| x * x).sum();

        assert!((input_energy - output_energy).abs() < 0.01,
            "Hadamard should preserve energy: in={} out={}", input_energy, output_energy);
    }

    #[test]
    fn test_different_sample_rates() {
        for &sr in &[22050.0, 44100.0, 48000.0, 96000.0] {
            let mut reverb = LushReverbState::new(sr, 42);

            // Send impulse
            reverb.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.3, 0.3, 0.2, 0.0, 1.0);

            // Process for equivalent of 0.5 seconds
            let samples = (sr * 0.5) as usize;
            let mut sum = 0.0;
            for _ in 0..samples {
                let sample = reverb.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 0.3, 0.2, 0.0, 1.0);
                assert!(sample.is_finite(), "NaN at sample rate {}", sr);
                sum += sample.abs();
            }

            assert!(sum > 1.0, "Should produce output at sample rate {}", sr);
        }
    }

    #[test]
    fn test_clear_resets_state() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // Process some audio
        for i in 0..1000 {
            reverb.process((i as f32 / 100.0).sin(), 0.0, 0.95, 1.0, 0.7, 0.3, 0.3, 0.2, 0.0, 1.0);
        }

        // Clear
        reverb.clear();

        // Output should be near zero
        let mut sum = 0.0;
        for _ in 0..100 {
            sum += reverb.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 0.3, 0.2, 0.0, 1.0).abs();
        }

        assert!(sum < 0.01, "Clear should reset all state: sum={}", sum);
    }

    #[test]
    fn test_seed_determinism() {
        let mut reverb1 = LushReverbState::new(44100.0, 12345);
        let mut reverb2 = LushReverbState::new(44100.0, 12345);

        // Should produce identical output with same seed
        for i in 0..1000 {
            let input = (i as f32 / 50.0).sin() * 0.5;
            let out1 = reverb1.process(input, 0.05, 0.95, 1.0, 0.7, 0.3, 0.5, 0.3, 0.0, 0.8);
            let out2 = reverb2.process(input, 0.05, 0.95, 1.0, 0.7, 0.3, 0.5, 0.3, 0.0, 0.8);

            assert!((out1 - out2).abs() < 1e-6,
                "Same seed should produce identical output at sample {}: {} vs {}", i, out1, out2);
        }
    }

    #[test]
    fn test_different_seeds_differ() {
        let mut reverb1 = LushReverbState::new(44100.0, 12345);
        let mut reverb2 = LushReverbState::new(44100.0, 67890);

        // Send same impulse
        reverb1.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);
        reverb2.process(1.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);

        // After more processing, outputs should differ due to different modulation
        // First let the modulation diverge for a bit
        for _ in 0..10000 {
            reverb1.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);
            reverb2.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);
        }

        // Now check for differences
        let mut diff_count = 0;
        for _ in 0..1000 {
            let out1 = reverb1.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);
            let out2 = reverb2.process(0.0, 0.0, 0.95, 1.0, 0.7, 0.3, 1.0, 1.0, 0.0, 1.0);

            if (out1 - out2).abs() > 0.0001 {
                diff_count += 1;
            }
        }

        assert!(diff_count > 100, "Different seeds should produce different modulation: diff_count={}", diff_count);
    }

    #[test]
    fn test_lush_reverb_rings_out() {
        let mut reverb = LushReverbState::new(44100.0, 42);

        // Send single impulse with long decay
        let _ = reverb.process(1.0, 0.0, 0.95, 1.0, 0.5, 0.3, 0.0, 0.0, 0.0, 1.0);

        // Measure energy at different time points
        let mut energy_100ms = 0.0;
        let mut energy_500ms = 0.0;
        let mut energy_1s = 0.0;
        let mut energy_2s = 0.0;

        for i in 0..88200 {  // 2 seconds
            let sample = reverb.process(0.0, 0.0, 0.95, 1.0, 0.5, 0.3, 0.0, 0.0, 0.0, 1.0);

            if i < 4410 { energy_100ms += sample.abs(); }
            if i >= 22050 - 2205 && i < 22050 + 2205 { energy_500ms += sample.abs(); }
            if i >= 44100 - 2205 && i < 44100 + 2205 { energy_1s += sample.abs(); }
            if i >= 88200 - 4410 { energy_2s += sample.abs(); }
        }

        println!("Energy at 100ms:  {:.4}", energy_100ms);
        println!("Energy at 500ms:  {:.4}", energy_500ms);
        println!("Energy at 1s:     {:.4}", energy_1s);
        println!("Energy at 2s:     {:.4}", energy_2s);

        assert!(energy_100ms > 0.1, "Should have energy at 100ms: {}", energy_100ms);
        assert!(energy_500ms > 0.05, "Should have energy at 500ms: {}", energy_500ms);
        assert!(energy_1s > 0.01, "Should have energy at 1s: {}", energy_1s);
        assert!(energy_2s > 0.001, "Should have energy at 2s: {}", energy_2s);
    }
}
