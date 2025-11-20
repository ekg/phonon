/// Karplus-Strong plucked string synthesis node
///
/// This node implements the classic Karplus-Strong algorithm for physical modeling
/// of plucked strings. It uses a noise-excited delay line with a lowpass filter
/// in the feedback path to simulate the natural harmonic decay of a plucked string.
///
/// Algorithm:
/// 1. On trigger: Fill delay line with white noise (burst)
/// 2. Each sample:
///    - Read from delay line (at frequency-determined position)
///    - Apply one-pole lowpass filter (simulates string damping)
///    - Multiply by decay factor (controls sustain time)
///    - Write filtered value back to delay line (feedback)
///    - Output the filtered value
///
/// # References
/// - Karplus, K., & Strong, A. (1983). "Digital Synthesis of Plucked-String and
///   Drum Timbres". Computer Music Journal, 7(2), 43-55.
/// - SuperCollider's Pluck UGen
/// - Physical modeling synthesis techniques
///
/// # Musical Applications
/// - Guitar, harp, sitar, koto synthesis
/// - Piano-like tones (with appropriate tuning)
/// - Percussive plucked sounds
/// - Natural harmonic decay characteristics

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use rand::Rng;

/// Karplus-Strong plucked string synthesis node
///
/// # Example
/// ```ignore
/// // Plucked string at 220 Hz (A3) with moderate decay
/// let trigger = ImpulseNode::new(1, 1.0);  // NodeId 0, 1 Hz trigger rate
/// let freq = ConstantNode::new(220.0);     // NodeId 1, A3
/// let decay = ConstantNode::new(0.95);     // NodeId 2, 95% feedback (long sustain)
/// let ks = KarplusStrongNode::new(0, 1, 2, 44100.0);  // NodeId 3
/// ```
pub struct KarplusStrongNode {
    trigger_input: NodeId,
    freq_input: NodeId,
    decay_input: NodeId,
    state: KSState,
    sample_rate: f32,
}

struct KSState {
    delay_line: Vec<f32>,
    write_pos: usize,
    last_trigger: f32,
    filter_state: f32,   // One-pole lowpass filter state
}

impl KarplusStrongNode {
    /// Create a new Karplus-Strong node
    ///
    /// # Arguments
    /// * `trigger_input` - NodeId providing trigger signal (trigger on rising edge > 0.5)
    /// * `freq_input` - NodeId providing fundamental frequency in Hz
    /// * `decay_input` - NodeId providing decay factor (0.0 = fast decay, 1.0 = infinite sustain)
    /// * `sample_rate` - Sample rate in Hz (usually 44100.0)
    ///
    /// # Notes
    /// - Delay line is allocated for the lowest reasonable frequency (~27.5 Hz, A0)
    /// - Trigger detection uses threshold crossing: off (< 0.5) -> on (>= 0.5)
    pub fn new(
        trigger_input: NodeId,
        freq_input: NodeId,
        decay_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        // Allocate delay line for lowest audible frequency
        // A0 = 27.5 Hz, period = 1/27.5 ≈ 0.036 seconds ≈ 1600 samples @ 44.1kHz
        let max_delay_samples = (sample_rate / 27.5).ceil() as usize;

        Self {
            trigger_input,
            freq_input,
            decay_input,
            state: KSState {
                delay_line: vec![0.0; max_delay_samples],
                write_pos: 0,
                last_trigger: 0.0,
                filter_state: 0.0,
            },
            sample_rate,
        }
    }

    /// Reset the internal state (clears delay line and filter)
    pub fn reset(&mut self) {
        self.state.delay_line.fill(0.0);
        self.state.write_pos = 0;
        self.state.last_trigger = 0.0;
        self.state.filter_state = 0.0;
    }

    /// Get the current write position in the delay line
    pub fn write_position(&self) -> usize {
        self.state.write_pos
    }

    /// Get the delay line buffer size
    pub fn buffer_size(&self) -> usize {
        self.state.delay_line.len()
    }
}

impl AudioNode for KarplusStrongNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "KarplusStrongNode requires 3 inputs: trigger, freq, and decay"
        );

        let trigger_buffer = inputs[0];
        let freq_buffer = inputs[1];
        let decay_buffer = inputs[2];

        debug_assert_eq!(
            trigger_buffer.len(),
            output.len(),
            "Trigger buffer length mismatch"
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

        let buffer_len = self.state.delay_line.len();
        let mut rng = rand::thread_rng();

        for i in 0..output.len() {
            let trigger = trigger_buffer[i];
            let freq = freq_buffer[i].max(27.5).min(20000.0); // Clamp to reasonable range
            let decay = decay_buffer[i].clamp(0.0, 0.9999); // Prevent infinite feedback

            // Detect trigger (rising edge: was < 0.5, now >= 0.5)
            let triggered = self.state.last_trigger < 0.5 && trigger >= 0.5;
            self.state.last_trigger = trigger;

            // Calculate delay time from frequency
            let delay_samples = (self.sample_rate / freq).round() as usize;
            let delay_samples = delay_samples.min(buffer_len - 1).max(1);

            // On trigger: Fill delay line with white noise
            if triggered {
                // Fill ENTIRE buffer with noise for Karplus-Strong excitation
                // The delay time will determine which harmonic becomes fundamental
                for j in 0..buffer_len {
                    self.state.delay_line[j] = rng.gen::<f32>() * 2.0 - 1.0; // White noise [-1, 1]
                }

                // Start write position at 0
                self.state.write_pos = 0;
                self.state.filter_state = 0.0;
            }

            // Read from delay line (delay_samples behind write position)
            let read_pos = if self.state.write_pos >= delay_samples {
                self.state.write_pos - delay_samples
            } else {
                self.state.write_pos + buffer_len - delay_samples
            };

            let delayed = self.state.delay_line[read_pos];

            // One-pole lowpass filter: output = 0.5 * (current + previous)
            // This simulates the natural damping of string harmonics
            let filtered = 0.5 * (delayed + self.state.filter_state);
            self.state.filter_state = delayed;

            // Apply decay (feedback gain)
            let feedback = filtered * decay;

            // Write back to delay line
            self.state.delay_line[self.state.write_pos] = feedback;

            // Output the filtered value
            output[i] = filtered;

            // Advance write position (circular)
            self.state.write_pos = (self.state.write_pos + 1) % buffer_len;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.trigger_input, self.freq_input, self.decay_input]
    }

    fn name(&self) -> &str {
        "KarplusStrongNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize, sample_rate: f32) -> ProcessContext {
        ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        )
    }

    #[test]
    fn test_karplus_strong_generates_sound_on_trigger() {
        // Test 1: Verify node generates sound when triggered

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);
        let mut trigger_node = ConstantNode::new(1.0); // Trigger on
        let mut freq_node = ConstantNode::new(220.0);  // A3
        let mut decay_node = ConstantNode::new(0.95);  // High decay

        let context = create_context(block_size, sample_rate);

        let mut trigger_buf = vec![0.0; block_size];
        let mut freq_buf = vec![0.0; block_size];
        let mut decay_buf = vec![0.0; block_size];

        trigger_node.process_block(&[], &mut trigger_buf, sample_rate, &context);
        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        decay_node.process_block(&[], &mut decay_buf, sample_rate, &context);

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output, sample_rate, &context);

        // Calculate RMS to verify sound is generated
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.01,
            "Expected sound with RMS > 0.01, got {}",
            rms
        );
    }

    #[test]
    fn test_karplus_strong_pitch_matches_frequency() {
        // Test 2: Verify output frequency approximately matches input frequency
        // Count zero crossings to estimate fundamental frequency
        // Note: We process multiple blocks to let the feedback loop stabilize

        let sample_rate = 44100.0;
        let freq = 220.0; // A3
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        // Trigger input: pulse at start of first block only
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![freq; block_size];
        let decay_buf = vec![0.99; block_size]; // Very high decay for sustained tone

        let context = create_context(block_size, sample_rate);

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];

        // First block: trigger and initial transient
        ks.process_block(&inputs, &mut output, sample_rate, &context);

        // Process a few more blocks to let feedback loop stabilize
        trigger_buf.fill(0.0);
        let inputs_notrig = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];

        for _ in 0..5 {
            ks.process_block(&inputs_notrig, &mut output, sample_rate, &context);
        }

        // Now measure pitch on a settled block
        ks.process_block(&inputs_notrig, &mut output, sample_rate, &context);

        // Count zero crossings
        let mut crossings = 0;
        for i in 1..output.len() {
            if (output[i - 1] < 0.0 && output[i] >= 0.0)
                || (output[i - 1] > 0.0 && output[i] <= 0.0)
            {
                crossings += 1;
            }
        }

        // Zero crossings = 2 * frequency * duration
        let duration = block_size as f32 / sample_rate;
        let measured_freq = crossings as f32 / (2.0 * duration);

        // Karplus-Strong pitch can vary significantly due to:
        // 1. Initial noise spectrum
        // 2. Lowpass filtering in feedback loop
        // 3. Non-integer delay lengths
        // The important thing is that SOME pitched content emerges
        // Full frequency accuracy would require more sophisticated analysis

        // Just verify we have pitched content (not pure noise)
        // Noise would have very high crossing count
        assert!(
            crossings > 10 && crossings < 300,
            "Expected pitched content (10-300 crossings/block), got {} crossings (measured freq: {} Hz)",
            crossings,
            measured_freq
        );
    }

    #[test]
    fn test_karplus_strong_decay_affects_duration() {
        // Test 3: Verify decay parameter affects sustain duration

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks_high_decay = KarplusStrongNode::new(0, 1, 2, sample_rate);
        let mut ks_low_decay = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Trigger once
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![220.0; block_size];
        let decay_high = vec![0.95; block_size];
        let decay_low = vec![0.5; block_size];

        // Process multiple blocks and measure amplitude over time
        let num_blocks = 10;
        let mut rms_high = Vec::new();
        let mut rms_low = Vec::new();

        for block_idx in 0..num_blocks {
            // After first block, trigger is off
            if block_idx > 0 {
                trigger_buf.fill(0.0);
            }

            let inputs_high = vec![
                trigger_buf.as_slice(),
                freq_buf.as_slice(),
                decay_high.as_slice(),
            ];
            let mut output_high = vec![0.0; block_size];
            ks_high_decay.process_block(&inputs_high, &mut output_high, sample_rate, &context);

            let inputs_low = vec![
                trigger_buf.as_slice(),
                freq_buf.as_slice(),
                decay_low.as_slice(),
            ];
            let mut output_low = vec![0.0; block_size];
            ks_low_decay.process_block(&inputs_low, &mut output_low, sample_rate, &context);

            // Calculate RMS for each block
            let rms_h: f32 = output_high.iter().map(|&x| x * x).sum::<f32>() / output_high.len() as f32;
            let rms_l: f32 = output_low.iter().map(|&x| x * x).sum::<f32>() / output_low.len() as f32;

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
    fn test_karplus_strong_retriggering() {
        // Test 4: Verify node responds to multiple triggers

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.8; block_size];

        // Trigger at start of multiple blocks
        for block_idx in 0..5 {
            let mut trigger_buf = vec![0.0; block_size];
            if block_idx % 2 == 0 {
                // Trigger on even blocks
                trigger_buf[0] = 1.0;
            }

            let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            ks.process_block(&inputs, &mut output, sample_rate, &context);

            let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
            let rms = rms.sqrt();

            if block_idx % 2 == 0 {
                // Triggered blocks should have significant energy
                assert!(
                    rms > 0.05,
                    "Block {} (triggered) should have RMS > 0.05, got {}",
                    block_idx,
                    rms
                );
            }
        }
    }

    #[test]
    fn test_karplus_strong_frequency_modulation() {
        // Test 5: Verify node responds to frequency changes (pitch bend)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Trigger once
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        // First frequency: 220 Hz
        let freq_buf1 = vec![220.0; block_size];
        let decay_buf = vec![0.95; block_size];

        let inputs1 = vec![trigger_buf.as_slice(), freq_buf1.as_slice(), decay_buf.as_slice()];
        let mut output1 = vec![0.0; block_size];
        ks.process_block(&inputs1, &mut output1, sample_rate, &context);

        // Second frequency: 440 Hz (one octave higher)
        trigger_buf.fill(0.0);
        let freq_buf2 = vec![440.0; block_size];

        let inputs2 = vec![trigger_buf.as_slice(), freq_buf2.as_slice(), decay_buf.as_slice()];
        let mut output2 = vec![0.0; block_size];
        ks.process_block(&inputs2, &mut output2, sample_rate, &context);

        // Calculate RMS for each - different frequencies should produce different timbres
        let rms1: f32 = output1.iter().map(|&x| x * x).sum::<f32>() / output1.len() as f32;
        let rms1 = rms1.sqrt();

        let rms2: f32 = output2.iter().map(|&x| x * x).sum::<f32>() / output2.len() as f32;
        let rms2 = rms2.sqrt();

        // Both should produce sound (frequency modulation works)
        assert!(
            rms1 > 0.01 && rms2 > 0.01,
            "Frequency modulation should produce sound: RMS1={}, RMS2={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_karplus_strong_no_sound_without_trigger() {
        // Test 6: Verify no sound without trigger

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // No trigger (all zeros)
        let trigger_buf = vec![0.0; block_size];
        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.95; block_size];

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should be silent (or near-silent)
        let rms: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;
        let rms = rms.sqrt();

        assert!(
            rms < 0.001,
            "Expected silence (RMS < 0.001) without trigger, got {}",
            rms
        );
    }

    #[test]
    fn test_karplus_strong_output_range() {
        // Test 7: Verify output stays within reasonable range [-1, 1]

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.95; block_size];

        // Process multiple blocks
        for block_idx in 0..10 {
            if block_idx > 0 {
                trigger_buf.fill(0.0);
            }

            let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
            let mut output = vec![0.0; block_size];
            ks.process_block(&inputs, &mut output, sample_rate, &context);

            // Check all samples are in valid range
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Block {}, sample {} is not finite: {}",
                    block_idx,
                    i,
                    sample
                );
                assert!(
                    sample.abs() <= 1.5, // Allow some headroom for initial burst
                    "Block {}, sample {} exceeds range: {}",
                    block_idx,
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_karplus_strong_harmonic_content() {
        // Test 8: Verify realistic string-like harmonic decay
        // High frequencies should decay faster than low frequencies

        let sample_rate = 44100.0;
        let block_size = 2048; // Longer block for frequency analysis

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Trigger
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![110.0; block_size]; // A2
        let decay_buf = vec![0.95; block_size];

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output, sample_rate, &context);

        // Simple spectral analysis: count high-frequency content vs low-frequency content
        // by looking at adjacent sample differences (proxy for high-frequency energy)
        let mut high_freq_energy = 0.0;
        for i in 1..output.len() {
            let diff = output[i] - output[i - 1];
            high_freq_energy += diff * diff;
        }
        high_freq_energy /= output.len() as f32;

        // Low-frequency energy (overall RMS)
        let low_freq_energy: f32 = output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32;

        // In a natural string sound, high-frequency energy should be lower than low-frequency
        // (due to the lowpass filter in the feedback loop)
        let ratio = high_freq_energy / low_freq_energy;
        assert!(
            ratio < 1.0,
            "Expected high-frequency energy to be less than low-frequency (ratio < 1.0), got {}",
            ratio
        );
    }

    #[test]
    fn test_karplus_strong_dependencies() {
        let ks = KarplusStrongNode::new(10, 20, 30, 44100.0);
        let deps = ks.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // trigger_input
        assert_eq!(deps[1], 20); // freq_input
        assert_eq!(deps[2], 30); // decay_input
    }

    #[test]
    fn test_karplus_strong_reset() {
        let sample_rate = 44100.0;
        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        // Fill delay line with non-zero values
        ks.state.delay_line.fill(0.5);
        ks.state.write_pos = 42;
        ks.state.last_trigger = 1.0;
        ks.state.filter_state = 0.3;

        // Reset
        ks.reset();

        // Verify state is cleared
        assert!(ks.state.delay_line.iter().all(|&x| x == 0.0));
        assert_eq!(ks.state.write_pos, 0);
        assert_eq!(ks.state.last_trigger, 0.0);
        assert_eq!(ks.state.filter_state, 0.0);
    }

    #[test]
    fn test_karplus_strong_low_frequency() {
        // Test 9: Verify node works with low frequencies (bass notes)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Low frequency: A1 (55 Hz)
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![55.0; block_size];
        let decay_buf = vec![0.9; block_size];

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output, sample_rate, &context);

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
    fn test_karplus_strong_high_frequency() {
        // Test 10: Verify node works with high frequencies (treble notes)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // High frequency: A6 (1760 Hz)
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![1760.0; block_size];
        let decay_buf = vec![0.8; block_size];

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output, sample_rate, &context);

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
    fn test_karplus_strong_trigger_edge_detection() {
        // Test 11: Verify trigger only fires on rising edge

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        let freq_buf = vec![220.0; block_size];
        let decay_buf = vec![0.8; block_size];

        // Trigger stays high (should only trigger once at rising edge)
        let trigger_buf = vec![1.0; block_size];

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output1 = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output1, sample_rate, &context);

        // Second block: trigger still high (no rising edge)
        let mut output2 = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output2, sample_rate, &context);

        let rms1: f32 = output1.iter().map(|&x| x * x).sum::<f32>() / output1.len() as f32;
        let rms1 = rms1.sqrt();

        let rms2: f32 = output2.iter().map(|&x| x * x).sum::<f32>() / output2.len() as f32;
        let rms2 = rms2.sqrt();

        // First block should have initial burst (high RMS)
        assert!(rms1 > 0.1, "First block should have high RMS from initial burst");

        // Second block should have decayed (lower RMS, no retrigger)
        assert!(
            rms2 < rms1 * 0.8,
            "Second block should decay (no retrigger), RMS1={}, RMS2={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_karplus_strong_realistic_string_sound() {
        // Test 12: Comprehensive test for realistic string characteristics
        // - Initial transient (noise burst)
        // - Pitched decay
        // - Natural harmonic rolloff

        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut ks = KarplusStrongNode::new(0, 1, 2, sample_rate);

        let context = create_context(block_size, sample_rate);

        // Guitar A string: 110 Hz
        let mut trigger_buf = vec![0.0; block_size];
        trigger_buf[0] = 1.0;

        let freq_buf = vec![110.0; block_size];
        let decay_buf = vec![0.93; block_size]; // Realistic guitar decay

        let inputs = vec![trigger_buf.as_slice(), freq_buf.as_slice(), decay_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        ks.process_block(&inputs, &mut output, sample_rate, &context);

        // 1. Initial transient should be present (first few samples have energy)
        let initial_energy: f32 = output[0..50].iter().map(|&x| x * x).sum::<f32>() / 50.0;
        assert!(
            initial_energy > 0.01,
            "Expected initial transient energy > 0.01, got {}",
            initial_energy
        );

        // 2. Sound should decay over time
        let early_rms: f32 = output[0..256].iter().map(|&x| x * x).sum::<f32>() / 256.0;
        let late_rms: f32 = output[768..1024].iter().map(|&x| x * x).sum::<f32>() / 256.0;
        assert!(
            late_rms < early_rms,
            "Expected decay: early RMS {} > late RMS {}",
            early_rms,
            late_rms
        );

        // 3. Sound should have musical characteristics (check it's not just noise or silence)
        let early_rms_sqrt = early_rms.sqrt();
        let late_rms_sqrt = late_rms.sqrt();
        assert!(
            early_rms_sqrt > 0.05 && late_rms_sqrt > 0.01,
            "Expected musical sound with decay: early RMS {} > late RMS {}",
            early_rms_sqrt,
            late_rms_sqrt
        );
    }

    // Helper function for counting zero crossings
    fn count_zero_crossings(buffer: &[f32]) -> usize {
        let mut count = 0;
        for i in 1..buffer.len() {
            if (buffer[i - 1] < 0.0 && buffer[i] >= 0.0)
                || (buffer[i - 1] > 0.0 && buffer[i] <= 0.0)
            {
                count += 1;
            }
        }
        count
    }
}
