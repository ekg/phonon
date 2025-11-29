/// FM Cross-Modulation node - Uses any audio signal as FM modulator
///
/// Unlike classic FM synthesis (which uses internal oscillators), FM cross-modulation
/// allows ANY audio signal to modulate the phase/timbre of a carrier signal.
/// This creates rhythmic timbral changes, complex inharmonic tones, and experimental textures.
///
/// Mathematically: output[i] = carrier[i] * cos(2π * mod_depth * modulator[i])
///
/// # Audio Theory
///
/// Phase modulation varies the instantaneous phase of the carrier based on the modulator:
/// - Low mod_depth: Subtle vibrato/chorus effects
/// - Medium mod_depth: Rich harmonic content, bell-like tones
/// - High mod_depth: Complex inharmonic sidebands, metallic textures
///
/// # Use Cases
///
/// - **Rhythmic timbral modulation**: Drums modulating bass for pulsing textures
/// - **Audio-rate vibrato**: LFO modulating pad for deep modulation
/// - **Vocoder-like effects**: Voice modulating synth for talking instruments
/// - **Experimental sound design**: Any signal modulating any other signal
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// FM Cross-Modulation node: carrier modulated by any audio signal
///
/// # Example
/// ```ignore
/// // Drums modulating bass
/// let kick = SamplePlaybackNode::new(...);    // NodeId 0
/// let bass = OscillatorNode::new(55.0);       // NodeId 1
/// let mod_depth = ConstantNode::new(2.0);     // NodeId 2
/// let fm_cross = FMCrossModNode::new(1, 0, 2); // NodeId 3
/// // Bass timbre changes rhythmically with kick pattern
/// ```
pub struct FMCrossModNode {
    carrier_input: NodeId,
    modulator_input: NodeId,
    mod_depth_input: NodeId,
}

impl FMCrossModNode {
    /// FMCrossModNode - Phase modulation using any audio signal as modulator
    ///
    /// Varies the carrier's phase/timbre based on modulator signal. Creates rich
    /// harmonic content, rhythmic timbral changes, and experimental textures.
    ///
    /// # Parameters
    /// - `carrier_input`: NodeId of carrier signal (signal to be modulated)
    /// - `modulator_input`: NodeId of modulator signal (controls modulation)
    /// - `mod_depth_input`: NodeId for modulation depth (0.0 = no effect, higher = more modulation)
    ///
    /// # Example
    /// ```phonon
    /// ~kick: s "bd*4"
    /// ~bass: saw 55
    /// ~modulated: fmcrossmod ~bass ~kick 2.0
    /// ```
    pub fn new(carrier_input: NodeId, modulator_input: NodeId, mod_depth_input: NodeId) -> Self {
        Self {
            carrier_input,
            modulator_input,
            mod_depth_input,
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

    /// Get the modulation depth input node ID
    pub fn mod_depth_input(&self) -> NodeId {
        self.mod_depth_input
    }
}

impl AudioNode for FMCrossModNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "FMCrossModNode requires 3 inputs (carrier, modulator, mod_depth), got {}",
            inputs.len()
        );

        let carrier = inputs[0];
        let modulator = inputs[1];
        let mod_depth = inputs[2];

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
        debug_assert_eq!(
            mod_depth.len(),
            output.len(),
            "Mod depth buffer length mismatch"
        );

        // FM cross-modulation: phase modulate carrier by modulator
        for i in 0..output.len() {
            // Phase modulation formula: carrier * cos(2π * depth * modulator)
            let phase_offset = 2.0 * PI * mod_depth[i] * modulator[i];
            output[i] = carrier[i] * phase_offset.cos();
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.carrier_input,
            self.modulator_input,
            self.mod_depth_input,
        ]
    }

    fn name(&self) -> &str {
        "FMCrossModNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    /// Helper: Calculate RMS of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

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
    fn test_fmcrossmod_zero_depth_passes_carrier() {
        // With zero modulation depth, output should equal carrier
        let sample_rate = 44100.0;
        let block_size = 512;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Create carrier (440 Hz sine)
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq = ConstantNode::new(440.0);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Create modulator (any signal - doesn't matter with zero depth)
        let mut modulator_osc = OscillatorNode::new(1, Waveform::Sine);
        let mut modulator_freq = ConstantNode::new(100.0);
        let mut modulator_freq_buf = vec![0.0; block_size];
        modulator_freq.process_block(&[], &mut modulator_freq_buf, sample_rate, &context);

        let mut modulator_buf = vec![0.0; block_size];
        modulator_osc.process_block(
            &[&modulator_freq_buf],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Zero modulation depth
        let mod_depth_buf = vec![0.0; block_size];

        // Apply FM cross-modulation
        let mut fm_cross = FMCrossModNode::new(0, 1, 2);
        let mut output = vec![0.0; block_size];
        fm_cross.process_block(
            &[&carrier_buf, &modulator_buf, &mod_depth_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Output should equal carrier (cos(0) = 1, so carrier * 1 = carrier)
        for i in 0..block_size {
            assert!(
                (output[i] - carrier_buf[i]).abs() < 0.001,
                "Zero depth should pass carrier unchanged. Position {}: expected {}, got {}",
                i,
                carrier_buf[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_fmcrossmod_creates_sidebands() {
        // FM should create sidebands at carrier ± n*modulator frequencies
        let sample_rate = 44100.0;
        let block_size = 8192; // Large block for frequency resolution
        let carrier_freq = 440.0;
        let modulator_freq = 100.0;
        let mod_depth = 1.0;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Create carrier oscillator
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq_const = ConstantNode::new(carrier_freq);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq_const.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Create modulator oscillator
        let mut modulator_osc = OscillatorNode::new(1, Waveform::Sine);
        let mut modulator_freq_const = ConstantNode::new(modulator_freq);
        let mut modulator_freq_buf = vec![0.0; block_size];
        modulator_freq_const.process_block(&[], &mut modulator_freq_buf, sample_rate, &context);

        let mut modulator_buf = vec![0.0; block_size];
        modulator_osc.process_block(
            &[&modulator_freq_buf],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Constant mod depth
        let mod_depth_buf = vec![mod_depth; block_size];

        // Apply FM cross-modulation
        let mut fm_cross = FMCrossModNode::new(0, 1, 2);
        let mut output = vec![0.0; block_size];
        fm_cross.process_block(
            &[&carrier_buf, &modulator_buf, &mod_depth_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Analyze spectrum
        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // FM creates sidebands at carrier ± n*modulator
        // With sine×sine FM, we expect frequencies around:
        // - Carrier frequency (440 Hz)
        // - Carrier ± modulator (340 Hz, 540 Hz)
        let has_carrier = peaks.iter().any(|&f| (f - carrier_freq).abs() < 30.0);
        let has_lower_sideband = peaks
            .iter()
            .any(|&f| (f - (carrier_freq - modulator_freq)).abs() < 30.0);
        let has_upper_sideband = peaks
            .iter()
            .any(|&f| (f - (carrier_freq + modulator_freq)).abs() < 30.0);

        assert!(
            has_carrier,
            "Expected carrier frequency {} Hz. Peaks: {:?}",
            carrier_freq, peaks
        );
        assert!(
            has_lower_sideband || has_upper_sideband,
            "Expected sidebands around {} Hz and {} Hz. Peaks: {:?}",
            carrier_freq - modulator_freq,
            carrier_freq + modulator_freq,
            peaks
        );
    }

    #[test]
    fn test_fmcrossmod_depth_affects_intensity() {
        // Higher modulation depth should create more prominent sidebands
        let sample_rate = 44100.0;
        let block_size = 2048;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Setup carrier and modulator
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq = ConstantNode::new(440.0);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        let mut modulator_osc = OscillatorNode::new(1, Waveform::Sine);
        let mut modulator_freq = ConstantNode::new(100.0);
        let mut modulator_freq_buf = vec![0.0; block_size];
        modulator_freq.process_block(&[], &mut modulator_freq_buf, sample_rate, &context);

        let mut modulator_buf = vec![0.0; block_size];
        modulator_osc.process_block(
            &[&modulator_freq_buf],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Test with low depth
        let low_depth_buf = vec![0.1; block_size];
        let mut fm_low = FMCrossModNode::new(0, 1, 2);
        let mut output_low = vec![0.0; block_size];
        fm_low.process_block(
            &[&carrier_buf, &modulator_buf, &low_depth_buf],
            &mut output_low,
            sample_rate,
            &context,
        );

        // Test with high depth
        let high_depth_buf = vec![2.0; block_size];
        let mut fm_high = FMCrossModNode::new(0, 1, 2);
        let mut output_high = vec![0.0; block_size];
        fm_high.process_block(
            &[&carrier_buf, &modulator_buf, &high_depth_buf],
            &mut output_high,
            sample_rate,
            &context,
        );

        // Higher depth should create more variation from original carrier
        let carrier_rms = calculate_rms(&carrier_buf);
        let low_rms = calculate_rms(&output_low);
        let high_rms = calculate_rms(&output_high);

        // Both should have energy
        assert!(
            low_rms > 0.05,
            "Low depth FM should produce audible output. Got RMS: {}",
            low_rms
        );
        assert!(
            high_rms > 0.05,
            "High depth FM should produce audible output. Got RMS: {}",
            high_rms
        );

        // The outputs should differ (different modulation depths create different timbres)
        let mut diff_sum = 0.0;
        for i in 0..block_size {
            diff_sum += (output_high[i] - output_low[i]).abs();
        }
        let avg_diff = diff_sum / block_size as f32;

        assert!(
            avg_diff > 0.01,
            "Different modulation depths should produce different outputs. Avg diff: {}",
            avg_diff
        );
    }

    #[test]
    fn test_fmcrossmod_with_lfo() {
        // LFO modulation should create vibrato-like effect
        let sample_rate = 44100.0;
        let block_size = 4410; // 100ms

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Carrier: constant sine at 440 Hz
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq = ConstantNode::new(440.0);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Modulator: 5 Hz LFO
        let mut lfo = Vec::with_capacity(block_size);
        for i in 0..block_size {
            let t = i as f32 / sample_rate;
            lfo.push((2.0 * PI * 5.0 * t).sin());
        }

        // Moderate mod depth
        let mod_depth_buf = vec![0.5; block_size];

        // Apply FM
        let mut fm_cross = FMCrossModNode::new(0, 1, 2);
        let mut output = vec![0.0; block_size];
        fm_cross.process_block(
            &[&carrier_buf, lfo.as_slice(), &mod_depth_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Output should have variation (vibrato effect)
        let rms = calculate_rms(&output);
        assert!(
            rms > 0.2,
            "FM with LFO should produce audible output. Got RMS: {}",
            rms
        );

        // Should see amplitude variation over time
        let first_quarter_rms = calculate_rms(&output[0..block_size / 4]);
        let third_quarter_rms =
            calculate_rms(&output[block_size / 2..block_size / 2 + block_size / 4]);

        // RMS should vary across the buffer due to LFO modulation
        assert!(
            (first_quarter_rms - third_quarter_rms).abs() > 0.01,
            "LFO modulation should create varying amplitude. First quarter RMS: {}, third quarter RMS: {}",
            first_quarter_rms,
            third_quarter_rms
        );
    }

    #[test]
    fn test_fmcrossmod_with_constant_modulator() {
        // Constant modulator should create a constant phase offset
        let sample_rate = 44100.0;
        let block_size = 512;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Carrier
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq = ConstantNode::new(440.0);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Constant modulator (0.5)
        let modulator_buf = vec![0.5; block_size];

        // Mod depth
        let mod_depth_buf = vec![1.0; block_size];

        // Apply FM
        let mut fm_cross = FMCrossModNode::new(0, 1, 2);
        let mut output = vec![0.0; block_size];
        fm_cross.process_block(
            &[&carrier_buf, &modulator_buf, &mod_depth_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // With constant modulator, output should be carrier scaled by constant
        // output[i] = carrier[i] * cos(2π * 1.0 * 0.5) = carrier[i] * cos(π)
        let scale_factor = (PI).cos(); // cos(π) = -1

        for i in 0..block_size {
            let expected = carrier_buf[i] * scale_factor;
            assert!(
                (output[i] - expected).abs() < 0.01,
                "Constant modulator should scale carrier. Position {}: expected {}, got {}",
                i,
                expected,
                output[i]
            );
        }
    }

    #[test]
    fn test_fmcrossmod_zero_modulator() {
        // Zero modulator should pass carrier through (cos(0) = 1)
        let sample_rate = 44100.0;
        let block_size = 512;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Carrier
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq = ConstantNode::new(440.0);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Zero modulator
        let modulator_buf = vec![0.0; block_size];

        // Any mod depth
        let mod_depth_buf = vec![2.0; block_size];

        // Apply FM
        let mut fm_cross = FMCrossModNode::new(0, 1, 2);
        let mut output = vec![0.0; block_size];
        fm_cross.process_block(
            &[&carrier_buf, &modulator_buf, &mod_depth_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Output should equal carrier (cos(0) = 1)
        for i in 0..block_size {
            assert!(
                (output[i] - carrier_buf[i]).abs() < 0.001,
                "Zero modulator should pass carrier. Position {}: expected {}, got {}",
                i,
                carrier_buf[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_fmcrossmod_dependencies() {
        let fm_cross = FMCrossModNode::new(5, 10, 15);
        let deps = fm_cross.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 5); // carrier
        assert_eq!(deps[1], 10); // modulator
        assert_eq!(deps[2], 15); // mod_depth
    }

    #[test]
    fn test_fmcrossmod_produces_output() {
        // Basic sanity check: FM should produce audible output
        let sample_rate = 44100.0;
        let block_size = 1024;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Carrier: 220 Hz sine
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier_freq_buf = vec![0.0; block_size];
        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);

        let mut carrier_buf = vec![0.0; block_size];
        carrier_osc.process_block(
            &[&carrier_freq_buf],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Modulator: 50 Hz sine
        let mut modulator_osc = OscillatorNode::new(1, Waveform::Sine);
        let mut modulator_freq = ConstantNode::new(50.0);
        let mut modulator_freq_buf = vec![0.0; block_size];
        modulator_freq.process_block(&[], &mut modulator_freq_buf, sample_rate, &context);

        let mut modulator_buf = vec![0.0; block_size];
        modulator_osc.process_block(
            &[&modulator_freq_buf],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Mod depth: 1.5
        let mod_depth_buf = vec![1.5; block_size];

        // Apply FM
        let mut fm_cross = FMCrossModNode::new(0, 1, 2);
        let mut output = vec![0.0; block_size];
        fm_cross.process_block(
            &[&carrier_buf, &modulator_buf, &mod_depth_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Should produce significant output
        let rms = calculate_rms(&output);
        assert!(
            rms > 0.2,
            "FM should produce significant output. Got RMS: {}",
            rms
        );

        // Output should have samples with varying amplitude
        let max = output.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let min = output.iter().copied().fold(f32::INFINITY, f32::min);

        assert!(
            max > 0.3,
            "FM output should have positive peaks. Max: {}",
            max
        );
        assert!(
            min < -0.3,
            "FM output should have negative peaks. Min: {}",
            min
        );
    }
}
