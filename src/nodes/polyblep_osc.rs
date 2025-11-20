/// PolyBLEP Oscillator - Anti-aliased multi-waveform oscillator
///
/// PolyBLEP (Polynomial Bandlimited Step) is a technique for reducing aliasing
/// in non-sinusoidal waveforms by correcting the discontinuities. This produces
/// warmer, more analog-like sounds compared to naive waveform generation.
///
/// # References
/// - Välimäki and Huovilainen "Oscillator and Filter Algorithms for Virtual
///   Analog Synthesis" (2006)
/// - Stilson/Smith "Antialiasing Oscillators in Subtractive Synthesis" (1996)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Waveform types for PolyBLEP oscillator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PolyBLEPWaveform {
    Saw,
    Square,
    Triangle,
}

/// Anti-aliased oscillator using PolyBLEP technique
///
/// # Example
/// ```ignore
/// // 440 Hz saw wave
/// let freq_const = ConstantNode::new(440.0);  // NodeId 0
/// let osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);  // NodeId 1
/// ```
pub struct PolyBLEPOscNode {
    frequency: NodeId,          // NodeId providing frequency values (Hz)
    waveform: PolyBLEPWaveform, // Waveform type
    phase: f32,                 // Internal state (0.0 to 1.0)
}

impl PolyBLEPOscNode {
    /// Create a new PolyBLEP oscillator node
    ///
    /// # Arguments
    /// * `frequency` - NodeId that provides frequency (can be constant or pattern)
    /// * `waveform` - Waveform type (Saw, Square, Triangle)
    pub fn new(frequency: NodeId, waveform: PolyBLEPWaveform) -> Self {
        Self {
            frequency,
            waveform,
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
    pub fn set_waveform(&mut self, waveform: PolyBLEPWaveform) {
        self.waveform = waveform;
    }

    /// Get waveform type
    pub fn waveform(&self) -> PolyBLEPWaveform {
        self.waveform
    }
}

/// PolyBLEP (Polynomial Band-Limited Step) for anti-aliasing
///
/// This function corrects the discontinuities in waveforms (like saw and square)
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

impl AudioNode for PolyBLEPOscNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "PolyBLEPOscNode requires frequency input"
        );

        let freq_buffer = inputs[0];

        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );

        for i in 0..output.len() {
            let freq = freq_buffer[i];
            let phase_inc = freq / sample_rate;

            // Generate waveform based on type
            output[i] = match self.waveform {
                PolyBLEPWaveform::Saw => {
                    // Naive saw wave: ramp from -1 to 1
                    let mut value = 2.0 * self.phase - 1.0;

                    // Apply PolyBLEP correction at discontinuity (phase wrapping)
                    value -= poly_blep(self.phase, phase_inc);

                    value
                }

                PolyBLEPWaveform::Square => {
                    // Naive square wave: -1 or 1
                    let mut value = if self.phase < 0.5 { 1.0 } else { -1.0 };

                    // Apply PolyBLEP at both edges
                    // Edge at phase = 0
                    value += poly_blep(self.phase, phase_inc);

                    // Edge at phase = 0.5
                    value -= poly_blep((self.phase - 0.5).abs(), phase_inc);

                    value
                }

                PolyBLEPWaveform::Triangle => {
                    // Triangle is integrated from saw, so we need to apply PolyBLEP to saw first
                    // For now, use basic triangle (no aliasing since it's continuous)
                    if self.phase < 0.5 {
                        4.0 * self.phase - 1.0
                    } else {
                        -4.0 * self.phase + 3.0
                    }
                }
            };

            // Advance phase
            self.phase += phase_inc;

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
        vec![self.frequency]
    }

    fn name(&self) -> &str {
        "PolyBLEPOscNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_polyblep_saw_generates_signal() {
        // Test basic saw waveform generation
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Saw wave produced no signal");
    }

    #[test]
    fn test_polyblep_saw_in_range() {
        // Saw wave should be roughly in [-1, 1] range (polyBLEP may cause slight overshoot)
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                sample.abs() <= 1.5,
                "Saw sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_polyblep_saw_reduced_aliasing() {
        // Compare high frequency saw - should not have extreme overshoots
        let mut const_freq = ConstantNode::new(8000.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4410,  // 0.1 second
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 4410];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "High frequency saw produced no signal");

        // Check that we don't have extreme values (polyBLEP smooths transitions)
        let max = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));

        assert!(
            max <= 1.2 && min >= -1.2,
            "PolyBLEP saw should not have extreme overshoots (max: {}, min: {})",
            max,
            min
        );
    }

    #[test]
    fn test_polyblep_square_generates_signal() {
        // Test square waveform generation
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Square);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.5);
        assert!(has_signal, "Square wave produced no signal");
    }

    #[test]
    fn test_polyblep_square_duty_cycle() {
        // Square wave should have ~50% duty cycle
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Square);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Count samples near high and low values (accounting for PolyBLEP transitions)
        let high_count = output.iter().filter(|&&x| x > 0.5).count();
        let low_count = output.iter().filter(|&&x| x < -0.5).count();

        // Should have roughly equal high and low samples
        let ratio = high_count as f32 / (high_count + low_count) as f32;
        assert!(
            (ratio - 0.5).abs() < 0.15,
            "Square wave duty cycle should be ~0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_polyblep_triangle_generates_signal() {
        // Test triangle waveform generation
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Triangle);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have signal
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Triangle wave produced no signal");
    }

    #[test]
    fn test_polyblep_triangle_in_range() {
        // Triangle wave should be in [-1, 1] range
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Triangle);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        for sample in &output {
            assert!(
                sample.abs() <= 1.1,
                "Triangle sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_polyblep_triangle_dc_offset() {
        // Triangle wave should have near-zero DC offset
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Triangle);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;
        assert!(avg.abs() < 0.1, "Triangle wave DC offset too high: {}", avg);
    }

    #[test]
    fn test_polyblep_frequency_sweep() {
        // Test frequency modulation across a range
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        for freq_value in [110.0, 220.0, 440.0, 880.0, 1760.0, 3520.0] {
            let mut const_freq = ConstantNode::new(freq_value);
            let mut freq_buf = vec![0.0; 512];
            const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice()];
            let mut output = vec![0.0; 512];
            osc.reset_phase();
            osc.process_block(&inputs, &mut output, 44100.0, &context);

            // Should produce signal for all frequencies
            let has_signal = output.iter().any(|&x| x.abs() > 0.1);
            assert!(
                has_signal,
                "Saw wave at {} Hz produced no signal",
                freq_value
            );
        }
    }

    #[test]
    fn test_polyblep_phase_continuity() {
        // Verify phase advances correctly and continuously
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        assert_eq!(osc.phase(), 0.0);

        // Process one sample at 440 Hz
        let freq_buf = vec![440.0];
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should have advanced by 440/44100
        let expected_phase = 440.0 / 44100.0;
        assert!(
            (osc.phase() - expected_phase).abs() < 0.0001,
            "Phase mismatch: got {}, expected {}",
            osc.phase(),
            expected_phase
        );
    }

    #[test]
    fn test_polyblep_phase_wraps() {
        // Verify phase wraps correctly at 1.0
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        // Set phase close to 1.0
        osc.phase = 0.99;

        // Process one sample at high frequency
        let freq_buf = vec![4410.0];  // 10% of sample rate
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should wrap back to [0.0, 1.0)
        assert!(
            osc.phase() >= 0.0 && osc.phase() < 1.0,
            "Phase didn't wrap: {}",
            osc.phase()
        );
    }

    #[test]
    fn test_polyblep_all_waveforms() {
        // Test all waveforms produce signal
        let waveforms = vec![
            PolyBLEPWaveform::Saw,
            PolyBLEPWaveform::Square,
            PolyBLEPWaveform::Triangle,
        ];

        for waveform in waveforms {
            let mut const_freq = ConstantNode::new(440.0);
            let mut osc = PolyBLEPOscNode::new(0, waveform);

            let context = ProcessContext::new(
                Fraction::from_float(0.0),
                0,
                512,
                2.0,
                44100.0,
            );

            let mut freq_buf = vec![0.0; 512];
            const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice()];
            let mut output = vec![0.0; 512];
            osc.process_block(&inputs, &mut output, 44100.0, &context);

            // All waveforms should produce signal
            let has_signal = output.iter().any(|&x| x.abs() > 0.1);
            assert!(
                has_signal,
                "Waveform {:?} produced no signal",
                waveform
            );
        }
    }

    #[test]
    fn test_polyblep_reset_phase() {
        // Test phase reset functionality
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        // Advance phase
        osc.phase = 0.5;
        assert_eq!(osc.phase(), 0.5);

        // Reset
        osc.reset_phase();
        assert_eq!(osc.phase(), 0.0);
    }

    #[test]
    fn test_polyblep_waveform_setter_getter() {
        // Test waveform setter/getter
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        assert_eq!(osc.waveform(), PolyBLEPWaveform::Saw);

        osc.set_waveform(PolyBLEPWaveform::Square);
        assert_eq!(osc.waveform(), PolyBLEPWaveform::Square);

        osc.set_waveform(PolyBLEPWaveform::Triangle);
        assert_eq!(osc.waveform(), PolyBLEPWaveform::Triangle);
    }

    #[test]
    fn test_polyblep_input_nodes() {
        // Test dependency tracking
        let osc = PolyBLEPOscNode::new(42, PolyBLEPWaveform::Saw);
        let deps = osc.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }

    #[test]
    fn test_polyblep_saw_dc_offset() {
        // Saw wave should have near-zero DC offset
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Saw);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4410,  // 0.1 second for more accurate average
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 4410];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;
        assert!(avg.abs() < 0.1, "Saw wave DC offset too high: {}", avg);
    }

    #[test]
    fn test_polyblep_square_dc_offset() {
        // Square wave should have near-zero DC offset
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = PolyBLEPOscNode::new(0, PolyBLEPWaveform::Square);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4410,  // 0.1 second for more accurate average
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 4410];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;
        assert!(avg.abs() < 0.1, "Square wave DC offset too high: {}", avg);
    }
}
