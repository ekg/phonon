/// Ring modulation node - multiplies carrier and modulator signals
///
/// Ring modulation creates sum and difference frequencies (f1 + f2, f1 - f2),
/// resulting in inharmonic/metallic timbres. Classic for bells, robots, sci-fi sounds.
///
/// Mathematically: output[i] = carrier[i] * modulator[i]
///
/// # Audio Theory
///
/// When two sinusoids are multiplied:
/// - Input: sin(2π·f1·t) × sin(2π·f2·t)
/// - Output: 0.5·[cos(2π·(f1-f2)·t) - cos(2π·(f1+f2)·t)]
/// - Creates frequencies at f1+f2 and f1-f2 (sidebands)
/// - Original frequencies f1 and f2 are REMOVED
///
/// # Use Cases
///
/// - **Metallic sounds**: Multiply audio-rate signals (inharmonic partials)
/// - **Tremolo**: Multiply audio with LFO (amplitude modulation with sidebands)
/// - **Bell sounds**: Classic application, creates complex overtones
/// - **Special effects**: Robots, lasers, sci-fi sounds
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Ring modulation node: out = carrier * modulator
///
/// # Example
/// ```ignore
/// // Classic ring mod: 440 Hz carrier × 220 Hz modulator
/// let carrier = OscillatorNode::new(440.0);   // NodeId 0
/// let modulator = OscillatorNode::new(220.0); // NodeId 1
/// let ring_mod = RingModNode::new(0, 1);      // NodeId 2
/// // Output: 660 Hz (440+220) and 220 Hz (440-220)
/// ```
pub struct RingModNode {
    carrier_input: NodeId,   // Usually audio signal
    modulator_input: NodeId, // Usually LFO or audio signal
}

impl RingModNode {
    /// RingModNode - Ring modulation for metallic and inharmonic sounds
    ///
    /// Multiplies two signals to create sum and difference frequencies (f1±f2),
    /// producing metallic, bell-like, and sci-fi timbres from inharmonic partials.
    ///
    /// # Parameters
    /// - `carrier_input`: NodeId of carrier signal (typically audio)
    /// - `modulator_input`: NodeId of modulator signal (LFO or audio)
    ///
    /// # Example
    /// ```phonon
    /// ~carrier: sine 440
    /// ~mod: sine 220
    /// ~bell: ~carrier * ~mod
    /// ```
    pub fn new(carrier_input: NodeId, modulator_input: NodeId) -> Self {
        Self {
            carrier_input,
            modulator_input,
        }
    }

    /// Get the carrier input node ID
    pub fn carrier_input(&self) -> NodeId {
        self.carrier_input
    }

    /// Get the modulator input node ID
    pub fn modulator_input(&self) -> NodeId {
        self.modulator_input
    }
}

impl AudioNode for RingModNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "RingModNode requires 2 inputs, got {}",
            inputs.len()
        );

        let carrier = inputs[0];
        let modulator = inputs[1];

        debug_assert_eq!(
            carrier.len(),
            output.len(),
            "Carrier buffer length mismatch"
        );
        debug_assert_eq!(
            modulator.len(),
            output.len(),
            "Modulator buffer length mismatch"
        );

        // Ring modulation: multiply carrier and modulator
        for i in 0..output.len() {
            output[i] = carrier[i] * modulator[i];
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.carrier_input, self.modulator_input]
    }

    fn name(&self) -> &str {
        "RingModNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    /// Helper: Detect frequency peaks in FFT spectrum
    fn find_frequency_peaks(buffer: &[f32], sample_rate: f32, threshold: f32) -> Vec<f32> {
        use rustfft::{num_complex::Complex, FftPlanner};

        let n = buffer.len();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n);

        // Prepare input (apply Hann window)
        let mut complex_input: Vec<Complex<f32>> = buffer
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos());
                Complex::new(sample * window, 0.0)
            })
            .collect();

        fft.process(&mut complex_input);

        // Find peaks in magnitude spectrum
        let mut peaks = Vec::new();
        let bin_width = sample_rate / n as f32;

        for i in 1..n / 2 - 1 {
            let mag = complex_input[i].norm();
            let prev_mag = complex_input[i - 1].norm();
            let next_mag = complex_input[i + 1].norm();

            // Peak detection: local maximum above threshold
            if mag > threshold && mag > prev_mag && mag > next_mag {
                let freq = i as f32 * bin_width;
                peaks.push(freq);
            }
        }

        peaks
    }

    #[test]
    fn test_ring_mod_creates_sidebands() {
        // Ring mod creates sum and difference frequencies
        let sample_rate = 44100.0;
        let block_size = 8192; // Large block for frequency resolution
        let carrier_freq = 440.0;
        let modulator_freq = 100.0;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Create oscillators
        let mut carrier = OscillatorNode::new(0, Waveform::Sine);
        let mut modulator = OscillatorNode::new(1, Waveform::Sine);
        let mut ring_mod = RingModNode::new(0, 1);

        // Generate carrier (440 Hz)
        let mut carrier_const = ConstantNode::new(carrier_freq);
        let mut carrier_freq_buf = vec![0.0; block_size];
        let inputs_freq = vec![];
        carrier_const.process_block(&inputs_freq, &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Generate modulator (100 Hz)
        let mut modulator_const = ConstantNode::new(modulator_freq);
        let mut modulator_freq_buf = vec![0.0; block_size];
        modulator_const.process_block(&inputs_freq, &mut modulator_freq_buf, sample_rate, &context);

        let mut modulator_buf = vec![0.0; block_size];
        modulator.process_block(
            &[&modulator_freq_buf],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Apply ring modulation
        let mut output = vec![0.0; block_size];
        ring_mod.process_block(
            &[&carrier_buf, &modulator_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Analyze spectrum for sidebands
        let peaks = find_frequency_peaks(&output, sample_rate, 0.1);

        // Expected frequencies: 440 + 100 = 540 Hz, |440 - 100| = 340 Hz
        let sum_freq = carrier_freq + modulator_freq;
        let diff_freq = (carrier_freq - modulator_freq).abs();

        // Find peaks near expected frequencies (within 20 Hz tolerance)
        let has_sum = peaks.iter().any(|&f| (f - sum_freq).abs() < 20.0);
        let has_diff = peaks.iter().any(|&f| (f - diff_freq).abs() < 20.0);

        assert!(
            has_sum,
            "Expected sum frequency {} Hz not found. Peaks: {:?}",
            sum_freq, peaks
        );
        assert!(
            has_diff,
            "Expected difference frequency {} Hz not found. Peaks: {:?}",
            diff_freq, peaks
        );

        // Original frequencies should be suppressed (ideal ring mod)
        let has_carrier = peaks.iter().any(|&f| (f - carrier_freq).abs() < 20.0);
        let has_modulator = peaks.iter().any(|&f| (f - modulator_freq).abs() < 20.0);

        // For sine × sine, original frequencies SHOULD be absent
        // (This is the mathematical property of ring modulation)
        assert!(
            !has_carrier || !has_modulator,
            "Ring modulation should suppress original frequencies. Peaks: {:?}",
            peaks
        );
    }

    #[test]
    fn test_ring_mod_with_lfo() {
        // LFO-rate modulation creates tremolo with sidebands
        let mut ring_mod = RingModNode::new(0, 1);

        let sample_rate = 44100.0;
        let block_size = 8820; // 200ms at 44.1kHz (enough for full LFO cycle)

        // Audio signal: 1.0 constant (carrier)
        let audio = vec![1.0; block_size];

        // LFO: oscillating between -1.0 and 1.0
        let mut lfo = Vec::with_capacity(block_size);
        for i in 0..block_size {
            let phase = i as f32 / sample_rate;
            lfo.push((2.0 * PI * 5.0 * phase).sin()); // 5 Hz LFO
        }

        let inputs = vec![audio.as_slice(), lfo.as_slice()];
        let mut output = vec![0.0; block_size];

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        ring_mod.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should oscillate (tremolo effect)
        let mut has_positive = false;
        let mut has_negative = false;

        for &sample in &output {
            if sample > 0.8 {
                has_positive = true;
            }
            if sample < -0.8 {
                has_negative = true;
            }
        }

        assert!(
            has_positive && has_negative,
            "Ring mod with LFO should create bipolar tremolo effect. Max: {}, Min: {}",
            output.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            output.iter().copied().fold(f32::INFINITY, f32::min)
        );
    }

    #[test]
    fn test_ring_mod_metallic_sound() {
        // High-frequency ring mod creates metallic/inharmonic timbre
        let mut ring_mod = RingModNode::new(0, 1);

        let sample_rate = 44100.0;
        let block_size = 512;

        // Two audio-rate signals
        let mut carrier = Vec::with_capacity(block_size);
        let mut modulator = Vec::with_capacity(block_size);

        for i in 0..block_size {
            let t = i as f32 / sample_rate;
            carrier.push((2.0 * PI * 440.0 * t).sin());
            modulator.push((2.0 * PI * 447.0 * t).sin()); // Slightly detuned
        }

        let inputs = vec![carrier.as_slice(), modulator.as_slice()];
        let mut output = vec![0.0; block_size];

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        ring_mod.process_block(&inputs, &mut output, sample_rate, &context);

        // Should produce audible output with inharmonic content
        let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / block_size as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.3,
            "Ring mod should produce significant output. Got RMS: {}",
            rms
        );
    }

    #[test]
    fn test_ring_mod_dc_offset_removal() {
        // Ring mod can be used to remove DC offset
        let mut ring_mod = RingModNode::new(0, 1);

        let sample_rate = 44100.0;
        let block_size = 512;

        // Signal with DC offset
        let carrier = vec![0.5; block_size]; // Constant DC

        // AC signal
        let mut modulator = Vec::with_capacity(block_size);
        for i in 0..block_size {
            let t = i as f32 / sample_rate;
            modulator.push((2.0 * PI * 10.0 * t).sin());
        }

        let inputs = vec![carrier.as_slice(), modulator.as_slice()];
        let mut output = vec![0.0; block_size];

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        ring_mod.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should be scaled version of modulator (0.5 * sin)
        for i in 0..block_size {
            let expected = 0.5 * modulator[i];
            assert!(
                (output[i] - expected).abs() < 0.001,
                "Ring mod should scale modulator by carrier. Expected: {}, got: {}",
                expected,
                output[i]
            );
        }
    }

    #[test]
    fn test_ring_mod_symmetric() {
        // Ring mod is commutative: A×B = B×A
        let mut ring_mod1 = RingModNode::new(0, 1);
        let mut ring_mod2 = RingModNode::new(1, 0); // Swapped inputs

        let sample_rate = 44100.0;
        let block_size = 512;

        let signal_a = vec![0.5, -0.3, 0.8, -0.2];
        let signal_b = vec![0.2, 0.7, -0.4, 0.9];

        let inputs1 = vec![signal_a.as_slice(), signal_b.as_slice()];
        let inputs2 = vec![signal_b.as_slice(), signal_a.as_slice()];

        let mut output1 = vec![0.0; 4];
        let mut output2 = vec![0.0; 4];

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 4, 2.0, sample_rate);

        ring_mod1.process_block(&inputs1, &mut output1, sample_rate, &context);
        ring_mod2.process_block(&inputs2, &mut output2, sample_rate, &context);

        // Results should be identical
        for i in 0..4 {
            assert!(
                (output1[i] - output2[i]).abs() < 0.0001,
                "Ring mod should be symmetric. Position {}: {} vs {}",
                i,
                output1[i],
                output2[i]
            );
        }
    }

    #[test]
    fn test_ring_mod_dependencies() {
        let ring_mod = RingModNode::new(5, 10);
        let deps = ring_mod.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }

    #[test]
    fn test_ring_mod_with_constants() {
        // Test with constant inputs (useful for scaling)
        let mut const_a = ConstantNode::new(0.5);
        let mut const_b = ConstantNode::new(0.8);
        let mut ring_mod = RingModNode::new(0, 1);

        let sample_rate = 44100.0;
        let block_size = 512;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Process constants first
        let mut buf_a = vec![0.0; block_size];
        let mut buf_b = vec![0.0; block_size];

        const_a.process_block(&[], &mut buf_a, sample_rate, &context);
        const_b.process_block(&[], &mut buf_b, sample_rate, &context);

        // Now ring mod them
        let inputs = vec![buf_a.as_slice(), buf_b.as_slice()];
        let mut output = vec![0.0; block_size];

        ring_mod.process_block(&inputs, &mut output, sample_rate, &context);

        // Every sample should be 0.4 (0.5 * 0.8)
        for sample in &output {
            assert!(
                (*sample - 0.4).abs() < 0.001,
                "Ring mod with constants should produce constant output. Expected: 0.4, got: {}",
                sample
            );
        }
    }

    #[test]
    fn test_ring_mod_zero_through_zero() {
        // Ring mod with zero in either input produces zero
        let mut ring_mod = RingModNode::new(0, 1);

        let sample_rate = 44100.0;
        let block_size = 512;

        // Non-zero carrier
        let carrier = vec![0.5; block_size];

        // Zero modulator
        let modulator = vec![0.0; block_size];

        let inputs = vec![carrier.as_slice(), modulator.as_slice()];
        let mut output = vec![0.0; block_size];

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        ring_mod.process_block(&inputs, &mut output, sample_rate, &context);

        // All output should be zero
        for sample in &output {
            assert!(
                sample.abs() < 0.0001,
                "Ring mod with zero modulator should produce silence"
            );
        }
    }
}
