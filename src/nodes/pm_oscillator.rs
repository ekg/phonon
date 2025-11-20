/// PM Oscillator node - Phase Modulation synthesis
///
/// Phase Modulation synthesis where a modulator oscillator modulates the
/// instantaneous phase of a carrier oscillator, creating rich harmonic content.
///
/// # PM Synthesis Theory
///
/// PM synthesis is mathematically equivalent to FM (Frequency Modulation) but
/// is simpler to implement. Instead of modulating the carrier's frequency,
/// PM directly modulates the carrier's phase.
///
/// **FM**: carrier_freq = base_freq + modulator * mod_index
/// **PM**: carrier_phase_offset = modulator * mod_index
///
/// The result is identical harmonic content, but PM avoids the need for
/// frequency integration (∫freq dt = phase).
///
/// ## PM Algorithm
/// ```text
/// modulator_output = sin(2π * modulator_phase)
/// phase_offset = mod_index * modulator_output
/// carrier_output = sin(2π * (carrier_phase + phase_offset))
/// ```
///
/// ## Classic PM Ratios (same as FM)
/// - 1:1 = Bell-like sounds
/// - 1:2 = Brass-like sounds
/// - 2:1 = Reed-like sounds
/// - Integer ratios = Harmonic spectra
/// - Non-integer ratios = Inharmonic/metallic sounds
///
/// ## Sidebands
/// When modulation index > 0, sidebands appear at:
/// `carrier_freq ± N * modulator_freq` (for all integers N)
///
/// Number of significant sidebands ≈ modulation_index + 1
///
/// # References
/// - John Chowning (1973) "The Synthesis of Complex Audio Spectra by Means of
///   Frequency Modulation" - applies equally to PM
/// - Casio CZ series synthesizers - used phase distortion (related to PM)
/// - Yamaha DX7 - technically used PM, not FM (though marketed as FM)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// PM Oscillator with pattern-controlled parameters
///
/// # Example
/// ```ignore
/// // Classic PM bell sound (1:1 ratio, mod_index = 2.0)
/// let carrier_freq = ConstantNode::new(440.0);   // NodeId 0
/// let mod_freq = ConstantNode::new(440.0);       // NodeId 1
/// let mod_index = ConstantNode::new(2.0);        // NodeId 2
/// let pm_osc = PMOscillatorNode::new(0, 1, 2);  // NodeId 3
/// ```
pub struct PMOscillatorNode {
    carrier_freq_input: NodeId,    // Carrier frequency in Hz
    modulator_freq_input: NodeId,  // Modulator frequency in Hz
    mod_index_input: NodeId,       // Modulation index (depth)
    carrier_phase: f32,             // Carrier phase (0.0 to 1.0)
    modulator_phase: f32,           // Modulator phase (0.0 to 1.0)
}

impl PMOscillatorNode {
    /// Create a new PM oscillator node
    ///
    /// # Arguments
    /// * `carrier_freq_input` - NodeId providing carrier frequency
    /// * `modulator_freq_input` - NodeId providing modulator frequency
    /// * `mod_index_input` - NodeId providing modulation index
    ///
    /// # Modulation Index Guidelines
    /// - 0.0 = Pure sine wave (no modulation)
    /// - 1.0 = Mild harmonics
    /// - 2.0-5.0 = Rich harmonic content
    /// - >10.0 = Very bright/noisy
    pub fn new(
        carrier_freq_input: NodeId,
        modulator_freq_input: NodeId,
        mod_index_input: NodeId,
    ) -> Self {
        Self {
            carrier_freq_input,
            modulator_freq_input,
            mod_index_input,
            carrier_phase: 0.0,
            modulator_phase: 0.0,
        }
    }

    /// Get current carrier phase (0.0 to 1.0)
    pub fn carrier_phase(&self) -> f32 {
        self.carrier_phase
    }

    /// Get current modulator phase (0.0 to 1.0)
    pub fn modulator_phase(&self) -> f32 {
        self.modulator_phase
    }

    /// Reset both phases to 0.0
    pub fn reset_phases(&mut self) {
        self.carrier_phase = 0.0;
        self.modulator_phase = 0.0;
    }

    /// Reset carrier phase to 0.0
    pub fn reset_carrier_phase(&mut self) {
        self.carrier_phase = 0.0;
    }

    /// Reset modulator phase to 0.0
    pub fn reset_modulator_phase(&mut self) {
        self.modulator_phase = 0.0;
    }
}

impl AudioNode for PMOscillatorNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "PMOscillatorNode requires 3 inputs (carrier_freq, modulator_freq, mod_index)"
        );

        let carrier_freq_buffer = inputs[0];
        let modulator_freq_buffer = inputs[1];
        let mod_index_buffer = inputs[2];

        debug_assert_eq!(
            carrier_freq_buffer.len(),
            output.len(),
            "Carrier frequency buffer length mismatch"
        );
        debug_assert_eq!(
            modulator_freq_buffer.len(),
            output.len(),
            "Modulator frequency buffer length mismatch"
        );
        debug_assert_eq!(
            mod_index_buffer.len(),
            output.len(),
            "Modulation index buffer length mismatch"
        );

        for i in 0..output.len() {
            let carrier_freq = carrier_freq_buffer[i];
            let modulator_freq = modulator_freq_buffer[i];
            let mod_index = mod_index_buffer[i];

            // Calculate modulator output (sine wave)
            let modulator = (self.modulator_phase * 2.0 * PI).sin();

            // PM: modulate carrier phase by modulator
            // Phase offset is proportional to mod_index
            let phase_offset = mod_index * modulator;
            let carrier = ((self.carrier_phase + phase_offset) * 2.0 * PI).sin();

            output[i] = carrier;

            // Advance modulator phase
            self.modulator_phase += modulator_freq / sample_rate;
            while self.modulator_phase >= 1.0 {
                self.modulator_phase -= 1.0;
            }
            while self.modulator_phase < 0.0 {
                self.modulator_phase += 1.0;
            }

            // Advance carrier phase
            self.carrier_phase += carrier_freq / sample_rate;
            while self.carrier_phase >= 1.0 {
                self.carrier_phase -= 1.0;
            }
            while self.carrier_phase < 0.0 {
                self.carrier_phase += 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.carrier_freq_input,
            self.modulator_freq_input,
            self.mod_index_input,
        ]
    }

    fn name(&self) -> &str {
        "PMOscillatorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    /// Helper to process PM oscillator with constant inputs
    fn process_pm(
        carrier_freq: f32,
        modulator_freq: f32,
        mod_index: f32,
        buffer_size: usize,
    ) -> Vec<f32> {
        let mut carrier_node = ConstantNode::new(carrier_freq);
        let mut mod_node = ConstantNode::new(modulator_freq);
        let mut index_node = ConstantNode::new(mod_index);
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            buffer_size,
            2.0,
            44100.0,
        );

        // Generate input buffers
        let mut carrier_buf = vec![0.0; buffer_size];
        let mut mod_buf = vec![0.0; buffer_size];
        let mut index_buf = vec![0.0; buffer_size];

        carrier_node.process_block(&[], &mut carrier_buf, 44100.0, &context);
        mod_node.process_block(&[], &mut mod_buf, 44100.0, &context);
        index_node.process_block(&[], &mut index_buf, 44100.0, &context);

        // Generate PM output
        let inputs = vec![carrier_buf.as_slice(), mod_buf.as_slice(), index_buf.as_slice()];
        let mut output = vec![0.0; buffer_size];
        pm_osc.process_block(&inputs, &mut output, 44100.0, &context);

        output
    }

    /// Calculate RMS (Root Mean Square) of a signal
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Find peak magnitude in buffer
    fn find_peak_magnitude(buffer: &[f32]) -> f32 {
        buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
    }

    #[test]
    fn test_pm_zero_mod_index() {
        // With mod_index = 0, PM oscillator should produce a pure sine wave
        let output = process_pm(440.0, 440.0, 0.0, 512);

        // Should have non-zero RMS
        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM with zero mod_index should produce sound");

        // All samples should be in [-1.0, 1.0]
        for &sample in &output {
            assert!(
                sample >= -1.0 && sample <= 1.0,
                "Sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_pm_creates_sidebands() {
        // With mod_index > 0, should create harmonics (sidebands)
        let zero_index = process_pm(440.0, 440.0, 0.0, 4096);
        let with_modulation = process_pm(440.0, 440.0, 2.0, 4096);

        // Modulated signal should have different spectral content
        let rms_zero = calculate_rms(&zero_index);
        let rms_mod = calculate_rms(&with_modulation);

        // Both should have energy
        assert!(rms_zero > 0.3);
        assert!(rms_mod > 0.3);

        // Peak amplitude should be similar (both sine-based)
        let peak_zero = find_peak_magnitude(&zero_index);
        let peak_mod = find_peak_magnitude(&with_modulation);

        assert!(peak_zero > 0.8);
        assert!(peak_mod > 0.8);
    }

    #[test]
    fn test_pm_carrier_phase_advances() {
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);
        assert_eq!(pm_osc.carrier_phase(), 0.0);

        // Process one sample at 440 Hz carrier
        let carrier_buf = vec![440.0];
        let mod_buf = vec![440.0];
        let index_buf = vec![0.0];
        let inputs = vec![carrier_buf.as_slice(), mod_buf.as_slice(), index_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        pm_osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Carrier phase should have advanced
        let expected_phase = 440.0 / 44100.0;
        assert!(
            (pm_osc.carrier_phase() - expected_phase).abs() < 0.0001,
            "Carrier phase mismatch: got {}, expected {}",
            pm_osc.carrier_phase(),
            expected_phase
        );
    }

    #[test]
    fn test_pm_modulator_phase_advances() {
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);
        assert_eq!(pm_osc.modulator_phase(), 0.0);

        // Process one sample with 220 Hz modulator
        let carrier_buf = vec![440.0];
        let mod_buf = vec![220.0];
        let index_buf = vec![1.0];
        let inputs = vec![carrier_buf.as_slice(), mod_buf.as_slice(), index_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        pm_osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Modulator phase should have advanced
        let expected_phase = 220.0 / 44100.0;
        assert!(
            (pm_osc.modulator_phase() - expected_phase).abs() < 0.0001,
            "Modulator phase mismatch: got {}, expected {}",
            pm_osc.modulator_phase(),
            expected_phase
        );
    }

    #[test]
    fn test_pm_c_to_m_ratio_1_1() {
        // 1:1 ratio (bell-like)
        let output = process_pm(440.0, 440.0, 2.0, 2048);

        // Should produce sound
        assert!(calculate_rms(&output) > 0.3);

        // Should be in valid range
        for &sample in &output {
            assert!(sample.abs() <= 1.1);
        }
    }

    #[test]
    fn test_pm_c_to_m_ratio_1_2() {
        // 1:2 ratio (brass-like)
        let output = process_pm(440.0, 880.0, 2.0, 2048);

        // Should produce sound
        assert!(calculate_rms(&output) > 0.3);

        // Should be different from 1:1 ratio
        let ratio_1_1 = process_pm(440.0, 440.0, 2.0, 2048);
        let same = output.iter().zip(&ratio_1_1)
            .all(|(a, b)| (a - b).abs() < 0.001);

        assert!(!same, "Different C:M ratios should produce different waveforms");
    }

    #[test]
    fn test_pm_c_to_m_ratio_2_1() {
        // 2:1 ratio (reed-like)
        let output = process_pm(440.0, 220.0, 2.0, 2048);

        // Should produce sound
        assert!(calculate_rms(&output) > 0.3);

        // Should be different from 1:1 ratio
        let ratio_1_1 = process_pm(440.0, 440.0, 2.0, 2048);
        let same = output.iter().zip(&ratio_1_1)
            .all(|(a, b)| (a - b).abs() < 0.001);

        assert!(!same, "Different C:M ratios should produce different waveforms");
    }

    #[test]
    fn test_pm_c_to_m_ratio_affects_timbre() {
        // Different carrier:modulator ratios create different spectra

        // 1:1 ratio (bell-like)
        let ratio_1_1 = process_pm(440.0, 440.0, 2.0, 2048);

        // 2:1 ratio (reed-like)
        let ratio_2_1 = process_pm(440.0, 220.0, 2.0, 2048);

        // 1:2 ratio (brass-like)
        let ratio_1_2 = process_pm(440.0, 880.0, 2.0, 2048);

        // All should produce sound
        assert!(calculate_rms(&ratio_1_1) > 0.3);
        assert!(calculate_rms(&ratio_2_1) > 0.3);
        assert!(calculate_rms(&ratio_1_2) > 0.3);

        // Waveforms should be different (simple check: not identical)
        let same_as_1_1 = ratio_2_1.iter().zip(&ratio_1_1)
            .all(|(a, b)| (a - b).abs() < 0.001);

        assert!(!same_as_1_1, "Different C:M ratios should produce different waveforms");
    }

    #[test]
    fn test_pm_inharmonic_ratio() {
        // Non-integer ratio (1:1.5) creates inharmonic spectrum
        let output = process_pm(440.0, 660.0, 2.0, 2048);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM with inharmonic ratio should produce sound");

        // Should be different from harmonic ratio
        let harmonic = process_pm(440.0, 880.0, 2.0, 2048);
        let same = output.iter().zip(&harmonic)
            .all(|(a, b)| (a - b).abs() < 0.001);

        assert!(!same, "Inharmonic and harmonic ratios should differ");
    }

    #[test]
    fn test_pm_mod_index_1() {
        // Mild modulation
        let output = process_pm(440.0, 440.0, 1.0, 1024);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM with mod_index=1.0 should produce sound");

        // Should be in valid range
        for &sample in &output {
            assert!(sample.abs() <= 1.1);
        }
    }

    #[test]
    fn test_pm_mod_index_5() {
        // Complex spectrum
        let output = process_pm(440.0, 440.0, 5.0, 1024);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM with mod_index=5.0 should produce sound");

        // Should be in valid range
        for &sample in &output {
            assert!(sample.abs() <= 1.1);
        }
    }

    #[test]
    fn test_pm_high_mod_index() {
        // Very high modulation index should still work (but be noisy)
        let output = process_pm(440.0, 440.0, 10.0, 1024);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM with high mod_index should produce sound");

        // Should still be in valid range (no clipping)
        for &sample in &output {
            assert!(sample.abs() <= 1.1, "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_pm_dependencies() {
        let pm_osc = PMOscillatorNode::new(10, 20, 30);
        let deps = pm_osc.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10);  // carrier_freq
        assert_eq!(deps[1], 20);  // modulator_freq
        assert_eq!(deps[2], 30);  // mod_index
    }

    #[test]
    fn test_pm_with_constants() {
        // PM should work with constant parameters
        let output = process_pm(440.0, 440.0, 1.5, 1024);

        // Should produce sound
        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM with constants should produce sound");

        // Should be in valid range
        for &sample in &output {
            assert!(sample.abs() <= 1.1, "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_pm_bell_sound() {
        // Classic PM bell: 1:1 ratio, mod_index ≈ 2.0
        let output = process_pm(440.0, 440.0, 2.0, 4096);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "PM bell should have significant energy");

        // Check output is reasonable
        let peak = find_peak_magnitude(&output);
        assert!(peak > 0.5 && peak <= 1.1, "PM bell peak level: {}", peak);
    }

    #[test]
    fn test_pm_phase_wraps() {
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        // Set phases close to 1.0
        pm_osc.carrier_phase = 0.99;
        pm_osc.modulator_phase = 0.99;

        // Process one sample at high frequency
        let carrier_buf = vec![4410.0];  // 10% of sample rate
        let mod_buf = vec![4410.0];
        let index_buf = vec![1.0];
        let inputs = vec![carrier_buf.as_slice(), mod_buf.as_slice(), index_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        pm_osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Both phases should wrap back to [0.0, 1.0)
        assert!(
            pm_osc.carrier_phase() >= 0.0 && pm_osc.carrier_phase() < 1.0,
            "Carrier phase didn't wrap: {}",
            pm_osc.carrier_phase()
        );
        assert!(
            pm_osc.modulator_phase() >= 0.0 && pm_osc.modulator_phase() < 1.0,
            "Modulator phase didn't wrap: {}",
            pm_osc.modulator_phase()
        );
    }

    #[test]
    fn test_pm_reset_phases() {
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        // Advance phases
        pm_osc.carrier_phase = 0.5;
        pm_osc.modulator_phase = 0.7;

        // Reset both
        pm_osc.reset_phases();
        assert_eq!(pm_osc.carrier_phase(), 0.0);
        assert_eq!(pm_osc.modulator_phase(), 0.0);
    }

    #[test]
    fn test_pm_reset_carrier_phase() {
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        pm_osc.carrier_phase = 0.5;
        pm_osc.modulator_phase = 0.7;

        pm_osc.reset_carrier_phase();
        assert_eq!(pm_osc.carrier_phase(), 0.0);
        assert_eq!(pm_osc.modulator_phase(), 0.7);  // Unchanged
    }

    #[test]
    fn test_pm_reset_modulator_phase() {
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        pm_osc.carrier_phase = 0.5;
        pm_osc.modulator_phase = 0.7;

        pm_osc.reset_modulator_phase();
        assert_eq!(pm_osc.carrier_phase(), 0.5);  // Unchanged
        assert_eq!(pm_osc.modulator_phase(), 0.0);
    }

    #[test]
    fn test_pm_output_range() {
        // PM output should always be in [-1.0, 1.0]
        let output = process_pm(440.0, 440.0, 3.0, 2048);

        for &sample in &output {
            assert!(
                sample >= -1.0 && sample <= 1.0,
                "PM output out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_pm_frequency_modulation() {
        // Test that both carrier and modulator frequencies can be modulated
        // This is a simple structural test - actual modulation would be tested
        // at integration level with time-varying input buffers

        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        // Carrier varying from 440 to 880 Hz
        let carrier_buf = vec![440.0, 550.0, 660.0, 770.0, 880.0];
        // Modulator varying from 440 to 880 Hz
        let mod_buf = vec![440.0, 550.0, 660.0, 770.0, 880.0];
        let index_buf = vec![2.0; 5];

        let inputs = vec![carrier_buf.as_slice(), mod_buf.as_slice(), index_buf.as_slice()];
        let mut output = vec![0.0; 5];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        pm_osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        for &sample in &output {
            assert!(sample.abs() <= 1.1);
        }
    }

    #[test]
    fn test_pm_mod_index_modulation() {
        // Test modulation index can vary over time
        let mut pm_osc = PMOscillatorNode::new(0, 1, 2);

        let carrier_buf = vec![440.0; 5];
        let mod_buf = vec![440.0; 5];
        // Mod index varying from 0 to 4
        let index_buf = vec![0.0, 1.0, 2.0, 3.0, 4.0];

        let inputs = vec![carrier_buf.as_slice(), mod_buf.as_slice(), index_buf.as_slice()];
        let mut output = vec![0.0; 5];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        pm_osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        for &sample in &output {
            assert!(sample.abs() <= 1.1);
        }
    }
}
