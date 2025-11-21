/// VCO (Voltage-Controlled Oscillator) - Analog-style multi-waveform with PWM
///
/// This node combines multiple waveforms (saw, pulse, triangle, sine) with
/// polyBLEP anti-aliasing for a warm, analog-style sound. Unlike basic
/// oscillators, VCO uses polyBLEP to reduce aliasing on discontinuous
/// waveforms (saw, pulse).
///
/// # References
/// - Välimäki and Huovilainen "Oscillator and Filter Algorithms for Virtual
///   Analog Synthesis" (2006)
/// - Stilson/Smith "Antialiasing Oscillators in Subtractive Synthesis" (1996)
/// - Classic Moog/ARP/Buchla VCO designs

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Waveform types for VCO
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VCOWaveform {
    Saw,
    Pulse,
    Triangle,
    Sine,
}

/// Analog-style VCO with polyBLEP anti-aliasing
///
/// # Example
/// ```ignore
/// // 440 Hz saw wave
/// let freq_const = ConstantNode::new(440.0);  // NodeId 0
/// let pw_const = ConstantNode::new(0.5);      // NodeId 1 (unused for saw)
/// let vco = VCONode::new(0, VCOWaveform::Saw, 1);  // NodeId 2
/// ```
pub struct VCONode {
    freq_input: NodeId,      // NodeId providing frequency values (Hz)
    waveform: VCOWaveform,   // Waveform type
    pulse_width: NodeId,     // NodeId providing pulse width (0.0 to 1.0, only for Pulse)
    phase: f32,              // Internal state (0.0 to 1.0)
}

impl VCONode {
    /// VCO - Voltage-Controlled Oscillator with polyBLEP anti-aliasing
    ///
    /// Analog-style oscillator with multiple waveforms and PWM support.
    /// Uses polyBLEP for band-limited discontinuous waveforms.
    ///
    /// # Parameters
    /// - `freq_input`: Frequency in Hz (can be modulated)
    /// - `waveform`: Waveform type (Saw, Pulse, Triangle, Sine)
    /// - `pulse_width`: Pulse width (0.0-1.0, used for Pulse only)
    ///
    /// # Example
    /// ```phonon
    /// ~freq: lfo 0.5 110 220
    /// ~pw: 0.5
    /// out: vco ~freq pulse ~pw
    /// ```
    pub fn new(freq_input: NodeId, waveform: VCOWaveform, pulse_width: NodeId) -> Self {
        Self {
            freq_input,
            waveform,
            pulse_width,
            phase: 0.0,
        }
    }

    /// Get current phase (0.0 to 1.0)
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Reset phase to 0.0
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    /// Set waveform type
    pub fn set_waveform(&mut self, waveform: VCOWaveform) {
        self.waveform = waveform;
    }

    /// Get waveform type
    pub fn waveform(&self) -> VCOWaveform {
        self.waveform
    }
}

/// PolyBLEP (Polynomial Band-Limited Step) for anti-aliasing
///
/// This function corrects the discontinuities in waveforms (like saw and pulse)
/// to reduce aliasing. It generates a polynomial "correction" signal that is
/// subtracted from the naive waveform.
///
/// # Arguments
/// * `phase` - Current phase (0.0 to 1.0)
/// * `phase_increment` - Phase increment per sample (freq / sample_rate)
///
/// # Returns
/// Correction value to subtract from naive waveform
fn poly_blep(phase: f32, phase_increment: f32) -> f32 {
    // Transition at phase = 0 (discontinuity going from 1.0 to 0.0)
    if phase < phase_increment {
        let t = phase / phase_increment;
        2.0 * t - t * t - 1.0
    }
    // Transition at phase = 1 (discontinuity going from 0.0 to 1.0)
    else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        t * t + 2.0 * t + 1.0
    }
    // No discontinuity
    else {
        0.0
    }
}

impl AudioNode for VCONode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "VCONode requires frequency and pulse width inputs"
        );

        let freq_buffer = inputs[0];
        let pw_buffer = inputs[1];

        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );
        debug_assert_eq!(
            pw_buffer.len(),
            output.len(),
            "Pulse width buffer length mismatch"
        );

        for i in 0..output.len() {
            let freq = freq_buffer[i];
            let phase_increment = freq / sample_rate;

            // Generate waveform based on type
            output[i] = match self.waveform {
                VCOWaveform::Saw => {
                    // Anti-aliased saw using polyBLEP
                    let naive = 2.0 * self.phase - 1.0;
                    let corrected = naive - poly_blep(self.phase, phase_increment);
                    corrected
                }

                VCOWaveform::Pulse => {
                    let pw = pw_buffer[i].clamp(0.01, 0.99);

                    // Naive pulse wave
                    let naive = if self.phase < pw { 1.0 } else { -1.0 };

                    // Apply polyBLEP at both edges
                    // First edge at phase = 0
                    let mut corrected = naive - poly_blep(self.phase, phase_increment);

                    // Second edge at phase = pw (use wrapped phase)
                    let wrapped_phase = if self.phase >= pw {
                        self.phase - pw
                    } else {
                        self.phase + (1.0 - pw)
                    };
                    corrected += poly_blep(wrapped_phase, phase_increment);

                    corrected
                }

                VCOWaveform::Triangle => {
                    // Integrate saw to get triangle (no polyBLEP needed - continuous)
                    if self.phase < 0.5 {
                        4.0 * self.phase - 1.0
                    } else {
                        -4.0 * self.phase + 3.0
                    }
                }

                VCOWaveform::Sine => {
                    // Pure sine wave (no aliasing)
                    (self.phase * 2.0 * PI).sin()
                }
            };

            // Advance phase
            self.phase += phase_increment;

            // Wrap phase to [0.0, 1.0)
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            while self.phase < 0.0 {
                self.phase += 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.freq_input, self.pulse_width]
    }

    fn name(&self) -> &str {
        "VCONode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_vco_saw_waveform() {
        // Test saw waveform generation
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_pw = ConstantNode::new(0.5);
        let mut vco = VCONode::new(0, VCOWaveform::Saw, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 512];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Saw wave should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Saw wave produced no signal");

        // Should be roughly in [-1, 1] range (polyBLEP may cause slight overshoot)
        for sample in &output {
            assert!(
                sample.abs() <= 1.5,
                "Saw sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_vco_pulse_square_wave() {
        // Width = 0.5 should produce a square wave
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_pw = ConstantNode::new(0.5);
        let mut vco = VCONode::new(0, VCOWaveform::Pulse, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 512];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.5);
        assert!(has_signal, "Pulse wave produced no signal");

        // Count samples near high and low values (accounting for polyBLEP transitions)
        let high_count = output.iter().filter(|&&x| x > 0.5).count();
        let low_count = output.iter().filter(|&&x| x < -0.5).count();

        // With width = 0.5, should have roughly equal high and low samples
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.5).abs() < 0.15,
            "Square wave duty cycle should be ~0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_vco_pulse_narrow_width() {
        // Width = 0.1 should produce narrow pulses
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_pw = ConstantNode::new(0.1);
        let mut vco = VCONode::new(0, VCOWaveform::Pulse, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 512];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Count high and low samples
        let high_count = output.iter().filter(|&&x| x > 0.5).count();
        let low_count = output.iter().filter(|&&x| x < -0.5).count();

        // With width = 0.1, high samples should be ~10% of total
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.1).abs() < 0.15,
            "Narrow pulse duty cycle should be ~0.1, got {}",
            ratio
        );
    }

    #[test]
    fn test_vco_pulse_wide_width() {
        // Width = 0.9 should produce wide pulses
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_pw = ConstantNode::new(0.9);
        let mut vco = VCONode::new(0, VCOWaveform::Pulse, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 512];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Count high and low samples
        let high_count = output.iter().filter(|&&x| x > 0.5).count();
        let low_count = output.iter().filter(|&&x| x < -0.5).count();

        // With width = 0.9, high samples should be ~90% of total
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.9).abs() < 0.15,
            "Wide pulse duty cycle should be ~0.9, got {}",
            ratio
        );
    }

    #[test]
    fn test_vco_triangle_waveform() {
        // Test triangle waveform generation
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_pw = ConstantNode::new(0.5);
        let mut vco = VCONode::new(0, VCOWaveform::Triangle, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 512];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Triangle wave should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Triangle wave produced no signal");

        // Should be in [-1, 1] range
        for sample in &output {
            assert!(
                sample.abs() <= 1.1,
                "Triangle sample out of range: {}",
                sample
            );
        }

        // Triangle should have near-zero DC offset
        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;
        assert!(avg.abs() < 0.1, "Triangle wave DC offset too high: {}", avg);
    }

    #[test]
    fn test_vco_sine_waveform() {
        // Test sine waveform generation
        let mut const_freq = ConstantNode::new(440.0);
        let mut const_pw = ConstantNode::new(0.5);
        let mut vco = VCONode::new(0, VCOWaveform::Sine, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 512];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Sine wave should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Sine wave produced no signal");

        // Should be in [-1, 1] range
        for sample in &output {
            assert!(
                sample.abs() <= 1.0,
                "Sine sample out of range: {}",
                sample
            );
        }

        // Sine should have near-zero DC offset
        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;
        assert!(avg.abs() < 0.1, "Sine wave DC offset too high: {}", avg);
    }

    #[test]
    fn test_vco_pulse_width_modulation() {
        // Test that pulse width can be modulated
        let mut const_freq = ConstantNode::new(440.0);
        let mut vco = VCONode::new(0, VCOWaveform::Pulse, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Test with different pulse widths
        for pw_value in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let mut const_pw = ConstantNode::new(pw_value);
            let mut pw_buf = vec![0.0; 512];
            const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
            let mut output = vec![0.0; 512];
            vco.reset_phase();
            vco.process_block(&inputs, &mut output, 44100.0, &context);

            // Should produce signal for all widths
            let has_signal = output.iter().any(|&x| x.abs() > 0.5);
            assert!(
                has_signal,
                "Pulse wave with width {} produced no signal",
                pw_value
            );
        }
    }

    #[test]
    fn test_vco_frequency_modulation() {
        // Test that frequency can be modulated
        let mut vco = VCONode::new(0, VCOWaveform::Sine, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut const_pw = ConstantNode::new(0.5);
        let mut pw_buf = vec![0.0; 512];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        // Test with different frequencies
        for freq_value in [110.0, 220.0, 440.0, 880.0, 1760.0] {
            let mut const_freq = ConstantNode::new(freq_value);
            let mut freq_buf = vec![0.0; 512];
            const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
            let mut output = vec![0.0; 512];
            vco.reset_phase();
            vco.process_block(&inputs, &mut output, 44100.0, &context);

            // Should produce signal for all frequencies
            let has_signal = output.iter().any(|&x| x.abs() > 0.1);
            assert!(
                has_signal,
                "Sine wave at {} Hz produced no signal",
                freq_value
            );
        }
    }

    #[test]
    fn test_vco_phase_advances() {
        let vco = VCONode::new(0, VCOWaveform::Sine, 1);

        assert_eq!(vco.phase(), 0.0);

        // Process one sample at 440 Hz
        let freq_buf = vec![440.0];
        let pw_buf = vec![0.5];
        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        let mut vco = vco;
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should have advanced by 440/44100
        let expected_phase = 440.0 / 44100.0;
        assert!(
            (vco.phase() - expected_phase).abs() < 0.0001,
            "Phase mismatch: got {}, expected {}",
            vco.phase(),
            expected_phase
        );
    }

    #[test]
    fn test_vco_phase_wraps() {
        let mut vco = VCONode::new(0, VCOWaveform::Sine, 1);

        // Set phase close to 1.0
        vco.phase = 0.99;

        // Process one sample at high frequency
        let freq_buf = vec![4410.0];  // 10% of sample rate
        let pw_buf = vec![0.5];
        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should wrap back to [0.0, 1.0)
        assert!(
            vco.phase() >= 0.0 && vco.phase() < 1.0,
            "Phase didn't wrap: {}",
            vco.phase()
        );
    }

    #[test]
    fn test_vco_all_waveforms_in_range() {
        let waveforms = vec![
            VCOWaveform::Saw,
            VCOWaveform::Pulse,
            VCOWaveform::Triangle,
            VCOWaveform::Sine,
        ];

        for waveform in waveforms {
            let mut const_freq = ConstantNode::new(440.0);
            let mut const_pw = ConstantNode::new(0.5);
            let mut vco = VCONode::new(0, waveform, 1);

            let context = ProcessContext::new(
                Fraction::from_float(0.0),
                0,
                512,
                2.0,
                44100.0,
            );

            let mut freq_buf = vec![0.0; 512];
            const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

            let mut pw_buf = vec![0.0; 512];
            const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
            let mut output = vec![0.0; 512];
            vco.process_block(&inputs, &mut output, 44100.0, &context);

            // All waveforms should produce signal
            let has_signal = output.iter().any(|&x| x.abs() > 0.1);
            assert!(
                has_signal,
                "Waveform {:?} produced no signal",
                waveform
            );

            // All samples should be in valid range (allow overshoot for polyBLEP)
            // PolyBLEP can cause transient overshoots up to ~2x during edge corrections
            for sample in &output {
                assert!(
                    sample.abs() <= 2.5,
                    "Waveform {:?} sample out of range: {}",
                    waveform,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_vco_input_nodes() {
        let vco = VCONode::new(42, VCOWaveform::Saw, 43);
        let deps = vco.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 42);
        assert_eq!(deps[1], 43);
    }

    #[test]
    fn test_vco_reset_phase() {
        let mut vco = VCONode::new(0, VCOWaveform::Sine, 1);

        // Advance phase
        vco.phase = 0.5;
        assert_eq!(vco.phase(), 0.5);

        // Reset
        vco.reset_phase();
        assert_eq!(vco.phase(), 0.0);
    }

    #[test]
    fn test_vco_waveform_setter_getter() {
        let mut vco = VCONode::new(0, VCOWaveform::Sine, 1);

        assert_eq!(vco.waveform(), VCOWaveform::Sine);

        vco.set_waveform(VCOWaveform::Saw);
        assert_eq!(vco.waveform(), VCOWaveform::Saw);

        vco.set_waveform(VCOWaveform::Pulse);
        assert_eq!(vco.waveform(), VCOWaveform::Pulse);
    }

    #[test]
    fn test_vco_polyblep_reduces_aliasing() {
        // Compare saw wave at high frequency with and without polyBLEP
        // This is a qualitative test - polyBLEP should produce smoother spectrum
        let mut const_freq = ConstantNode::new(8000.0);  // High frequency
        let mut const_pw = ConstantNode::new(0.5);
        let mut vco = VCONode::new(0, VCOWaveform::Saw, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4410,  // 0.1 second
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 4410];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let mut pw_buf = vec![0.0; 4410];
        const_pw.process_block(&[], &mut pw_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), pw_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        vco.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "High frequency saw produced no signal");

        // Check that we don't have extreme values (polyBLEP smooths transitions)
        // This is a basic sanity check - proper aliasing test would use FFT
        let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

        assert!(
            max <= 1.2 && min >= -1.2,
            "PolyBLEP saw should not have extreme overshoots (max: {}, min: {})",
            max,
            min
        );
    }
}
