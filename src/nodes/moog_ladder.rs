/// Moog Ladder filter node - classic 4-pole lowpass filter with resonance
///
/// This node implements the iconic Moog ladder filter, a 4-pole (24 dB/octave)
/// lowpass filter with resonance feedback. The design is based on Bob Moog's
/// original analog ladder circuit from 1965.
///
/// # Implementation Details
///
/// Uses four cascaded one-pole lowpass stages with resonance feedback from
/// the output. At high resonance values (> 3.5), the filter self-oscillates,
/// producing a sine wave at the cutoff frequency.
///
/// Based on:
/// - Bob Moog's original ladder filter design (1965)
/// - Antti Huovilainen's improved digital model (2004)
/// - Will Pirkle "Designing Software Synthesizer Plug-Ins in C++" (2015)
///
/// # Musical Characteristics
///
/// - 24 dB/octave rolloff (4-pole)
/// - Self-oscillation at high resonance (becomes sine oscillator)
/// - Warm, fat bass sound characteristic of analog synthesizers
/// - Slight resonance boost even at low settings
/// - Natural-sounding filter movement when modulated

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Internal state for the 4-pole ladder filter
#[derive(Debug, Clone)]
struct MoogLadderState {
    /// First filter stage output
    stage1: f32,
    /// Second filter stage output
    stage2: f32,
    /// Third filter stage output
    stage3: f32,
    /// Fourth filter stage output
    stage4: f32,
}

impl Default for MoogLadderState {
    fn default() -> Self {
        Self {
            stage1: 0.0,
            stage2: 0.0,
            stage3: 0.0,
            stage4: 0.0,
        }
    }
}

/// Moog Ladder filter node with pattern-controlled cutoff and resonance
///
/// # Example
/// ```ignore
/// // Classic Moog bass sound - 200 Hz lowpass with moderate resonance
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let cutoff = ConstantNode::new(200.0);                  // NodeId 2
/// let resonance = ConstantNode::new(2.5);                 // NodeId 3
/// let moog = MoogLadderNode::new(1, 2, 3);                // NodeId 4
/// ```
///
/// # Musical Applications
/// - Classic analog bass sounds (low cutoff, high resonance)
/// - Lead synth sounds (modulated cutoff)
/// - Self-oscillation as sine oscillator (resonance > 3.5)
/// - Filter sweeps and modulation
/// - Acid basslines (modulated cutoff + resonance)
pub struct MoogLadderNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Cutoff frequency input (Hz)
    cutoff: NodeId,
    /// Resonance input (0.0 to 4.0, self-oscillates above ~3.5)
    resonance: NodeId,
    /// Filter state (four stage values)
    state: MoogLadderState,
}

impl MoogLadderNode {
    /// Moog Ladder Filter - Classic analog-modeled lowpass with resonance
    ///
    /// 4-pole ladder filter based on Moog design with voltage-controlled resonance.
    /// Features self-oscillation at high resonance for subtractive synthesis.
    ///
    /// # Parameters
    /// - `input`: Audio signal to filter
    /// - `cutoff`: Cutoff frequency in Hz (20-20000)
    /// - `resonance`: Resonance amount (0.0-4.0, >3.5 = self-oscillating)
    ///
    /// # Example
    /// ```phonon
    /// ~saw: saw 110
    /// ~cutoff: sine 0.5 * 4000 + 2000
    /// ~filtered: ~saw # moog ~cutoff 2.5
    /// out: ~filtered * 0.5
    /// ```
    pub fn new(input: NodeId, cutoff: NodeId, resonance: NodeId) -> Self {
        Self {
            input,
            cutoff,
            resonance,
            state: MoogLadderState::default(),
        }
    }

    /// Get current filter state (for debugging)
    #[allow(dead_code)]
    fn state(&self) -> &MoogLadderState {
        &self.state
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        self.state = MoogLadderState::default();
    }
}

impl AudioNode for MoogLadderNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            3,
            "MoogLadderNode requires 3 inputs: signal, cutoff, resonance"
        );

        let input_buffer = inputs[0];
        let cutoff_buffer = inputs[1];
        let resonance_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            cutoff_buffer.len(),
            output.len(),
            "Cutoff buffer length mismatch"
        );
        debug_assert_eq!(
            resonance_buffer.len(),
            output.len(),
            "Resonance buffer length mismatch"
        );

        for i in 0..output.len() {
            let input_sample = input_buffer[i];

            // Clamp cutoff to valid range (20 Hz to 20 kHz)
            let cutoff = cutoff_buffer[i].max(20.0).min(20000.0);

            // Clamp resonance to prevent instability (0.0 to 4.0)
            let resonance = resonance_buffer[i].max(0.0).min(4.0);

            // Calculate one-pole coefficient from cutoff frequency
            // g = 1 - e^(-2π * fc / fs)
            // This is the discrete-time approximation of a one-pole lowpass
            let g = 1.0 - (-2.0 * PI * cutoff / sample_rate).exp();

            // Resonance feedback: take output from stage 4 and feed back to input
            // Scale resonance to create the characteristic Moog sound
            let feedback = self.state.stage4 * resonance;

            // Subtract feedback from input (negative feedback)
            let input_compensated = input_sample - feedback;

            // Apply soft saturation to input for warmth and stability
            // tanh provides gentle limiting that prevents filter blow-up
            let input_saturated = input_compensated.tanh();

            // Four cascaded one-pole lowpass stages
            // Each stage: output += g * (input - output)
            self.state.stage1 += g * (input_saturated - self.state.stage1);
            self.state.stage2 += g * (self.state.stage1 - self.state.stage2);
            self.state.stage3 += g * (self.state.stage2 - self.state.stage3);
            self.state.stage4 += g * (self.state.stage3 - self.state.stage4);

            // Output is the final stage
            output[i] = self.state.stage4;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.cutoff, self.resonance]
    }

    fn name(&self) -> &str {
        "MoogLadderNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{ConstantNode, OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    /// Helper to calculate RMS (root mean square) of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Helper to create a test context
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_moog_basic_lowpass() {
        // Test 1: Verify basic lowpass filtering - low frequency passes, high attenuated
        let sample_rate = 44100.0;
        let block_size = 512;

        // Create 440 Hz sine wave (should pass through 1000 Hz filter)
        let mut freq_low = ConstantNode::new(440.0);
        let mut osc_low = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut resonance = ConstantNode::new(0.5); // Low resonance
        let mut moog = MoogLadderNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        freq_low.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc_low.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        moog.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);

        // 440 Hz should pass through 1000 Hz lowpass mostly unchanged
        assert!(
            output_rms > input_rms * 0.5,
            "Low frequency should pass: input={}, output={}",
            input_rms,
            output_rms
        );
    }

    #[test]
    fn test_moog_high_frequency_attenuation() {
        // Test 2: High frequency should be heavily attenuated
        let sample_rate = 44100.0;
        let block_size = 512;

        // Create 8000 Hz sine wave (should be attenuated by 1000 Hz filter)
        let mut freq_high = ConstantNode::new(8000.0);
        let mut osc_high = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut resonance = ConstantNode::new(0.5);
        let mut moog = MoogLadderNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        freq_high.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc_high.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];
        moog.process_block(&inputs, &mut output, sample_rate, &context);

        let output_rms = calculate_rms(&output);

        // 8000 Hz should be heavily attenuated by 1000 Hz lowpass
        // 24 dB/octave = 72 dB attenuation at 3 octaves above cutoff
        assert!(
            output_rms < input_rms * 0.05,
            "High frequency should be attenuated: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_moog_resonance_boost() {
        // Test 3: Higher resonance should increase output level at cutoff
        let sample_rate = 44100.0;
        let block_size = 1024; // Longer block for stable measurements

        // Test signal at cutoff frequency (1000 Hz)
        let mut freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);

        // Test with low resonance
        let mut resonance_low = ConstantNode::new(0.5);
        let mut resonance_low_buf = vec![0.0; block_size];
        resonance_low.process_block(&[], &mut resonance_low_buf, sample_rate, &context);

        let mut moog_low = MoogLadderNode::new(1, 2, 3);
        let inputs_low = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_low_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            moog_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);
        }

        let rms_low = calculate_rms(&output_low);

        // Test with high resonance
        let mut resonance_high = ConstantNode::new(3.0);
        let mut resonance_high_buf = vec![0.0; block_size];
        resonance_high.process_block(&[], &mut resonance_high_buf, sample_rate, &context);

        let mut moog_high = MoogLadderNode::new(1, 2, 3);
        let inputs_high = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_high_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            moog_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);
        }

        let rms_high = calculate_rms(&output_high);

        // High resonance should boost signal at cutoff frequency
        assert!(
            rms_high > rms_low * 1.2,
            "High resonance should boost signal: low={}, high={}, ratio={}",
            rms_low,
            rms_high,
            rms_high / rms_low
        );
    }

    #[test]
    fn test_moog_high_resonance_characteristics() {
        // Test 4: Very high resonance creates distinctive Moog characteristics
        // At extreme resonance, tanh() saturation prevents blow-up but creates compression
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Compare moderate vs very high resonance at cutoff frequency
        let mut freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);

        // Test with moderate resonance
        let mut resonance_mod = ConstantNode::new(1.5);
        let mut resonance_mod_buf = vec![0.0; block_size];
        resonance_mod.process_block(&[], &mut resonance_mod_buf, sample_rate, &context);

        let mut moog_mod = MoogLadderNode::new(1, 2, 3);
        let inputs_mod = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_mod_buf.as_slice(),
        ];
        let mut output_mod = vec![0.0; block_size];

        for _ in 0..5 {
            moog_mod.process_block(&inputs_mod, &mut output_mod, sample_rate, &context);
        }

        let rms_mod = calculate_rms(&output_mod);

        // Test with very high resonance
        let mut resonance_high = ConstantNode::new(3.8);
        let mut resonance_high_buf = vec![0.0; block_size];
        resonance_high.process_block(&[], &mut resonance_high_buf, sample_rate, &context);

        let mut moog_high = MoogLadderNode::new(1, 2, 3);
        let inputs_high = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_high_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];

        for _ in 0..5 {
            moog_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);
        }

        let rms_high = calculate_rms(&output_high);

        // Both should produce significant output (filter is working)
        assert!(
            rms_mod > 0.01 && rms_high > 0.01,
            "Both resonance settings should produce output: mod={}, high={}",
            rms_mod,
            rms_high
        );

        // The outputs should differ (different resonance values have different effect)
        assert!(
            (rms_mod - rms_high).abs() > 0.01,
            "Different resonance should produce different outputs: mod={}, high={}",
            rms_mod,
            rms_high
        );
    }

    #[test]
    fn test_moog_cutoff_modulation() {
        // Test 5: Cutoff frequency changes should affect filtering
        let sample_rate = 44100.0;
        let block_size = 512;

        // Test signal: saw wave (rich harmonics)
        let mut freq = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut resonance = ConstantNode::new(1.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        // Test with low cutoff (200 Hz)
        let mut cutoff_low = ConstantNode::new(200.0);
        let mut cutoff_low_buf = vec![0.0; block_size];
        cutoff_low.process_block(&[], &mut cutoff_low_buf, sample_rate, &context);

        let mut moog_low = MoogLadderNode::new(1, 2, 3);
        let inputs_low = vec![
            signal_buf.as_slice(),
            cutoff_low_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output_low = vec![0.0; block_size];
        moog_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

        let rms_low = calculate_rms(&output_low);

        // Test with high cutoff (2000 Hz)
        let mut cutoff_high = ConstantNode::new(2000.0);
        let mut cutoff_high_buf = vec![0.0; block_size];
        cutoff_high.process_block(&[], &mut cutoff_high_buf, sample_rate, &context);

        let mut moog_high = MoogLadderNode::new(1, 2, 3);
        let inputs_high = vec![
            signal_buf.as_slice(),
            cutoff_high_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output_high = vec![0.0; block_size];
        moog_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

        let rms_high = calculate_rms(&output_high);

        // Higher cutoff should pass more harmonics = higher RMS
        assert!(
            rms_high > rms_low * 1.2,
            "Higher cutoff should pass more harmonics: low={}, high={}",
            rms_low,
            rms_high
        );
    }

    #[test]
    fn test_moog_resonance_modulation() {
        // Test 6: Resonance changes should affect output
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Signal at cutoff frequency
        let mut freq = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);

        // Test different resonance values
        let resonance_values = [0.5, 1.5, 2.5];
        let mut rms_values = Vec::new();

        for &res in &resonance_values {
            let mut resonance = ConstantNode::new(res);
            let mut resonance_buf = vec![0.0; block_size];
            resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

            let mut moog = MoogLadderNode::new(1, 2, 3);
            let inputs = vec![
                signal_buf.as_slice(),
                cutoff_buf.as_slice(),
                resonance_buf.as_slice(),
            ];
            let mut output = vec![0.0; block_size];

            // Multiple blocks to reach steady state
            for _ in 0..3 {
                moog.process_block(&inputs, &mut output, sample_rate, &context);
            }

            rms_values.push(calculate_rms(&output));
        }

        // RMS should increase with resonance
        assert!(
            rms_values[1] > rms_values[0],
            "RMS should increase with resonance: {} -> {}",
            rms_values[0],
            rms_values[1]
        );
        assert!(
            rms_values[2] > rms_values[1],
            "RMS should increase with resonance: {} -> {}",
            rms_values[1],
            rms_values[2]
        );
    }

    #[test]
    fn test_moog_24db_rolloff() {
        // Test 7: Verify 24 dB/octave rolloff characteristic
        let sample_rate = 44100.0;
        let block_size = 1024;
        let cutoff_freq = 1000.0;

        // Test frequencies: 1 octave and 2 octaves above cutoff
        let freq_1oct = cutoff_freq * 2.0; // 2000 Hz
        let freq_2oct = cutoff_freq * 4.0; // 4000 Hz

        let mut cutoff = ConstantNode::new(cutoff_freq);
        let mut resonance = ConstantNode::new(0.1); // Minimal resonance

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        // Test at 1 octave above cutoff
        let mut freq_node1 = ConstantNode::new(freq_1oct);
        let mut osc1 = OscillatorNode::new(0, Waveform::Sine);
        let mut freq_buf1 = vec![0.0; block_size];
        let mut signal_buf1 = vec![0.0; block_size];

        freq_node1.process_block(&[], &mut freq_buf1, sample_rate, &context);
        osc1.process_block(&[freq_buf1.as_slice()], &mut signal_buf1, sample_rate, &context);

        let mut moog1 = MoogLadderNode::new(1, 2, 3);
        let inputs1 = vec![
            signal_buf1.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; block_size];

        for _ in 0..3 {
            moog1.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        let input_rms1 = calculate_rms(&signal_buf1);
        let output_rms1 = calculate_rms(&output1);
        let attenuation_1oct = output_rms1 / input_rms1;

        // Test at 2 octaves above cutoff
        let mut freq_node2 = ConstantNode::new(freq_2oct);
        let mut osc2 = OscillatorNode::new(0, Waveform::Sine);
        let mut freq_buf2 = vec![0.0; block_size];
        let mut signal_buf2 = vec![0.0; block_size];

        freq_node2.process_block(&[], &mut freq_buf2, sample_rate, &context);
        osc2.process_block(&[freq_buf2.as_slice()], &mut signal_buf2, sample_rate, &context);

        let mut moog2 = MoogLadderNode::new(1, 2, 3);
        let inputs2 = vec![
            signal_buf2.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; block_size];

        for _ in 0..3 {
            moog2.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let input_rms2 = calculate_rms(&signal_buf2);
        let output_rms2 = calculate_rms(&output2);
        let attenuation_2oct = output_rms2 / input_rms2;

        // 24 dB/octave means:
        // 1 octave above = -24 dB ≈ 0.063x amplitude
        // 2 octaves above = -48 dB ≈ 0.004x amplitude
        // Ratio should be roughly 2^2 = 4 (doubling every octave in dB space)
        let attenuation_ratio = attenuation_1oct / attenuation_2oct;

        assert!(
            attenuation_ratio > 3.0 && attenuation_ratio < 20.0,
            "24dB/oct rolloff: 1oct={:.4}, 2oct={:.4}, ratio={:.2} (expected ~4-16)",
            attenuation_1oct,
            attenuation_2oct,
            attenuation_ratio
        );
    }

    #[test]
    fn test_moog_stability() {
        // Test 8: Filter should remain stable with extreme parameters
        let sample_rate = 44100.0;
        let block_size = 512;

        // Extreme inputs
        let mut signal_buf = vec![0.0; block_size];
        for i in 0..block_size {
            signal_buf[i] = ((i as f32 * 0.1).sin() * 2.0).clamp(-1.0, 1.0);
        }

        let mut cutoff = ConstantNode::new(19000.0); // Very high cutoff
        let mut resonance = ConstantNode::new(3.9); // Near maximum resonance

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let mut moog = MoogLadderNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks
        for _ in 0..10 {
            moog.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} became infinite/NaN",
                    i
                );
                assert!(
                    sample.abs() < 10.0,
                    "Sample {} has extreme value: {}",
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_moog_input_nodes() {
        // Test 9: Verify input node dependencies
        let moog = MoogLadderNode::new(10, 20, 30);
        let deps = moog.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // cutoff
        assert_eq!(deps[2], 30); // resonance
    }

    #[test]
    fn test_moog_state_isolation() {
        // Test 10: Two instances should have independent state
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        signal_buf[0] = 1.0; // Impulse

        let mut cutoff = ConstantNode::new(1000.0);
        let mut resonance = ConstantNode::new(3.0); // Higher resonance for more difference

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let mut moog1 = MoogLadderNode::new(0, 1, 2);
        let mut moog2 = MoogLadderNode::new(0, 1, 2);

        let mut output1 = vec![0.0; block_size];
        let mut output2 = vec![0.0; block_size];

        // Process first filter multiple times to build up different state
        for i in 0..10 {
            let inputs = vec![
                signal_buf.as_slice(),
                cutoff_buf.as_slice(),
                resonance_buf.as_slice(),
            ];
            moog1.process_block(&inputs, &mut output1, sample_rate, &context);
            if i == 0 {
                signal_buf.fill(0.0); // Silent after first impulse
            }
        }

        // Reset signal for second filter
        signal_buf[0] = 1.0;

        // Process second filter once (different history)
        let inputs2 = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        moog2.process_block(&inputs2, &mut output2, sample_rate, &context);

        // Outputs should differ significantly (different states)
        let rms1 = calculate_rms(&output1);
        let rms2 = calculate_rms(&output2);

        assert!(
            (rms1 - rms2).abs() > 0.001,
            "Filters should have independent state: rms1={}, rms2={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_moog_dc_response() {
        // Test 11: DC signal should pass through (with some attenuation from resonance feedback)
        let sample_rate = 44100.0;
        let block_size = 512;

        let signal_buf = vec![0.5; block_size]; // DC at 0.5

        let mut cutoff = ConstantNode::new(1000.0);
        let mut resonance = ConstantNode::new(0.1); // Minimal resonance for less feedback

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let mut moog = MoogLadderNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..20 {
            moog.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // DC should pass through (lowpass passes DC)
        // Some attenuation is expected due to resonance feedback and saturation
        let output_mean: f32 = output.iter().sum::<f32>() / output.len() as f32;

        assert!(
            output_mean > 0.3,
            "DC should pass through with minimal attenuation: got {}",
            output_mean
        );
    }

    #[test]
    fn test_moog_parameter_clamping() {
        // Test 12: Extreme parameters should be clamped safely
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        signal_buf[0] = 1.0;

        let mut cutoff = ConstantNode::new(100000.0); // Way above Nyquist
        let mut resonance = ConstantNode::new(100.0); // Way above safe range

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let mut moog = MoogLadderNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Should not panic or produce NaN/Inf
        moog.process_block(&inputs, &mut output, sample_rate, &context);

        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} not finite with extreme parameters: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_moog_reset() {
        // Test 13: Reset should clear state
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        signal_buf[0] = 1.0; // Impulse

        let mut cutoff = ConstantNode::new(1000.0);
        let mut resonance = ConstantNode::new(2.5);

        let context = test_context(block_size);

        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let mut moog = MoogLadderNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process to build up state
        for _ in 0..5 {
            moog.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Reset
        moog.reset();

        // State should be zeroed
        let state = moog.state();
        assert_eq!(state.stage1, 0.0);
        assert_eq!(state.stage2, 0.0);
        assert_eq!(state.stage3, 0.0);
        assert_eq!(state.stage4, 0.0);
    }

    #[test]
    fn test_moog_classic_sound() {
        // Test 14: Classic Moog sound characteristics
        // Low cutoff + high resonance should create characteristic "squelchy" bass
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Saw wave bass note (55 Hz = A1)
        let mut freq = ConstantNode::new(55.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut cutoff = ConstantNode::new(250.0); // Low cutoff
        let mut resonance = ConstantNode::new(3.0); // High resonance

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut cutoff_buf = vec![0.0; block_size];
        let mut resonance_buf = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        cutoff.process_block(&[], &mut cutoff_buf, sample_rate, &context);
        resonance.process_block(&[], &mut resonance_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let mut moog = MoogLadderNode::new(1, 2, 3);
        let inputs = vec![
            signal_buf.as_slice(),
            cutoff_buf.as_slice(),
            resonance_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks
        for _ in 0..3 {
            moog.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // Should have significant output (resonance boost at cutoff)
        assert!(
            output_rms > 0.01,
            "Classic Moog sound should have strong output: {}",
            output_rms
        );

        // Should be somewhat filtered (not full input)
        assert!(
            output_rms < input_rms,
            "Should be filtered: input={}, output={}",
            input_rms,
            output_rms
        );

        // Should be finite and bounded
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite() && sample.abs() < 5.0,
                "Sample {} should be reasonable: {}",
                i,
                sample
            );
        }
    }
}
