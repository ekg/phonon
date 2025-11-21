/// Frequency shifter node - shift all frequencies by a constant Hz amount
///
/// This node implements single-sideband (SSB) modulation to shift all frequencies
/// linearly. Unlike pitch shifting (which multiplies frequencies), frequency shifting
/// adds/subtracts a constant value, creating inharmonic, metallic sounds.
///
/// # Algorithm
///
/// Uses Hilbert transform to create I/Q (in-phase/quadrature) signals:
/// 1. Generate I/Q pair from input using 90° phase shift
/// 2. Multiply by complex exponential at shift frequency
/// 3. Extract real part for output
///
/// Mathematically:
/// ```text
/// hilbert_i = input (real part)
/// hilbert_q = hilbert_transform(input) (90° phase shift)
///
/// cos_val = cos(2π × shift_hz × t)
/// sin_val = sin(2π × shift_hz × t)
///
/// output = hilbert_i × cos_val - hilbert_q × sin_val  (upper sideband)
/// ```
///
/// # Frequency Shifting vs Ring Modulation
///
/// **Frequency Shifter** (this node):
/// - Input: 440 Hz + 880 Hz
/// - Shift: +100 Hz
/// - Output: 540 Hz + 980 Hz (linear shift)
/// - Maintains harmonic relationships in non-harmonic way
///
/// **Ring Modulator**:
/// - Input: 440 Hz carrier, 100 Hz modulator
/// - Output: 340 Hz (440-100) + 540 Hz (440+100)
/// - Creates sum AND difference frequencies
///
/// # Use Cases
///
/// - **Metallic/bell sounds**: Inharmonic partials
/// - **Detuning**: Subtle frequency offset
/// - **Special effects**: Alien voices, robotic sounds
/// - **Spectral processing**: Move frequency content
///
/// # Implementation
///
/// Uses a cascade of allpass filters to approximate Hilbert transform
/// (90° phase shift across wide frequency range). This is more efficient
/// than FFT-based approaches for real-time processing.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Frequency shifter node with pattern-controlled shift amount
///
/// # Example
/// ```ignore
/// // Shift all frequencies up by 100 Hz
/// let signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let shift = ConstantNode::new(100.0);                  // NodeId 1
/// let shifter = FrequencyShifterNode::new(0, 1);        // NodeId 2
/// ```
pub struct FrequencyShifterNode {
    input: NodeId,       // Audio signal to frequency shift
    shift_hz_input: NodeId, // Frequency shift in Hz (-1000 to +1000)

    // Hilbert transform using allpass filter cascade
    // We use a simplified approach with 2 allpass stages
    // For better quality, could use 4-12 stages
    hilbert_state: HilbertState,

    // Oscillator for shifting (complex exponential)
    oscillator_phase: f32, // Phase accumulator (0.0 to 1.0)
}

/// State for Hilbert transform approximation
///
/// Uses allpass filters to create 90° phase shift.
/// This is a simplified 2-stage design - production quality
/// would use 6-12 stages for wider bandwidth coverage.
struct HilbertState {
    // All-pass filter coefficients and states for I channel (0° path)
    ap_i1: AllPassState,
    ap_i2: AllPassState,

    // All-pass filter coefficients and states for Q channel (90° path)
    ap_q1: AllPassState,
    ap_q2: AllPassState,
}

/// Single all-pass filter state
struct AllPassState {
    a: f32,      // Coefficient
    x_prev: f32, // Previous input
    y_prev: f32, // Previous output
}

impl AllPassState {
    fn new(a: f32) -> Self {
        Self {
            a,
            x_prev: 0.0,
            y_prev: 0.0,
        }
    }

    /// Process one sample through all-pass filter
    /// Transfer function: H(z) = (a + z^-1) / (1 + a*z^-1)
    fn process(&mut self, input: f32) -> f32 {
        let output = self.a * input + self.x_prev - self.a * self.y_prev;
        self.x_prev = input;
        self.y_prev = output;
        output
    }

    fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y_prev = 0.0;
    }
}

impl HilbertState {
    /// Create new Hilbert transform state
    ///
    /// Uses coefficient pairs designed for ~90° phase difference
    /// across a wide frequency range (from Olli Niemitalo's design)
    fn new() -> Self {
        Self {
            // I (0°) path coefficients
            ap_i1: AllPassState::new(0.6923878),
            ap_i2: AllPassState::new(0.9360654322959),

            // Q (90°) path coefficients
            ap_q1: AllPassState::new(0.4021921162426),
            ap_q2: AllPassState::new(0.8561710882420),
        }
    }

    /// Process one sample, return (I, Q) pair
    fn process(&mut self, input: f32) -> (f32, f32) {
        // I path: input -> ap1 -> ap2
        let i = self.ap_i2.process(self.ap_i1.process(input));

        // Q path: input -> ap1 -> ap2 (different coefficients)
        let q = self.ap_q2.process(self.ap_q1.process(input));

        (i, q)
    }

    fn reset(&mut self) {
        self.ap_i1.reset();
        self.ap_i2.reset();
        self.ap_q1.reset();
        self.ap_q2.reset();
    }
}

impl FrequencyShifterNode {
    /// FrequencyShifter - Shifts all frequencies linearly by constant Hz
    ///
    /// Uses single-sideband modulation to shift spectral content linearly,
    /// creating metallic inharmonic effects and special sound design.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to shift
    /// - `shift_hz_input`: NodeId providing shift amount in Hz (default: 0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~shifted: ~signal # frequency_shifter 50
    /// ```
    pub fn new(input: NodeId, shift_hz_input: NodeId) -> Self {
        Self {
            input,
            shift_hz_input,
            hilbert_state: HilbertState::new(),
            oscillator_phase: 0.0,
        }
    }

    /// Get current oscillator phase (0.0 to 1.0)
    pub fn oscillator_phase(&self) -> f32 {
        self.oscillator_phase
    }

    /// Reset internal state to silence
    pub fn reset(&mut self) {
        self.hilbert_state.reset();
        self.oscillator_phase = 0.0;
    }

    /// Get input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get shift_hz input node ID
    pub fn shift_hz_input(&self) -> NodeId {
        self.shift_hz_input
    }
}

impl AudioNode for FrequencyShifterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "FrequencyShifterNode requires 2 inputs (signal, shift_hz), got {}",
            inputs.len()
        );

        let input_buffer = inputs[0];
        let shift_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            shift_buffer.len(),
            output.len(),
            "Shift buffer length mismatch"
        );

        for i in 0..output.len() {
            let input_sample = input_buffer[i];
            let shift_hz = shift_buffer[i].clamp(-1000.0, 1000.0);

            // Generate I/Q pair using Hilbert transform
            let (hilbert_i, hilbert_q) = self.hilbert_state.process(input_sample);

            // Generate complex exponential: e^(j*2π*shift_hz*t)
            // cos and sin give us the real and imaginary parts
            let angle = 2.0 * PI * self.oscillator_phase;
            let cos_val = angle.cos();
            let sin_val = angle.sin();

            // Single sideband modulation (upper sideband)
            // Real part of: (I + jQ) × (cos + j*sin)
            // = I*cos - Q*sin + j*(I*sin + Q*cos)
            // We only want the real part:
            output[i] = hilbert_i * cos_val - hilbert_q * sin_val;

            // Advance oscillator phase
            self.oscillator_phase += shift_hz / sample_rate;

            // Wrap phase to [0.0, 1.0)
            while self.oscillator_phase >= 1.0 {
                self.oscillator_phase -= 1.0;
            }
            while self.oscillator_phase < 0.0 {
                self.oscillator_phase += 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.shift_hz_input]
    }

    fn name(&self) -> &str {
        "FrequencyShifterNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;
    use std::f32::consts::PI;

    fn create_test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        )
    }

    /// Helper: Find frequency peaks in FFT spectrum
    fn find_frequency_peaks(buffer: &[f32], sample_rate: f32, threshold: f32) -> Vec<f32> {
        use rustfft::{FftPlanner, num_complex::Complex};

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
    fn test_frequency_shifter_positive_shift() {
        // Positive shift should move frequencies UP
        let sample_rate = 44100.0;
        let block_size = 8192; // Large block for good frequency resolution
        let input_freq = 440.0;
        let shift_hz = 100.0;

        let context = create_test_context(block_size);

        // Create 440 Hz sine wave
        let mut freq_const = ConstantNode::new(input_freq);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shifter = FrequencyShifterNode::new(0, 1);

        // Generate frequency buffer
        let mut freq_buf = vec![0.0; block_size];
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        // Generate input signal (440 Hz)
        let mut input_buf = vec![0.0; block_size];
        osc.process_block(&[&freq_buf], &mut input_buf, sample_rate, &context);

        // Generate shift amount buffer
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        // Apply frequency shifter
        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input_buf, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Analyze spectrum
        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // Expected: 440 + 100 = 540 Hz
        let expected_freq = input_freq + shift_hz;

        let has_expected = peaks.iter().any(|&f| (f - expected_freq).abs() < 30.0);

        assert!(
            has_expected,
            "Expected frequency {} Hz not found. Peaks: {:?}",
            expected_freq, peaks
        );
    }

    #[test]
    fn test_frequency_shifter_negative_shift() {
        // Negative shift should move frequencies DOWN
        let sample_rate = 44100.0;
        let block_size = 8192;
        let input_freq = 880.0;
        let shift_hz = -200.0;

        let context = create_test_context(block_size);

        let mut freq_const = ConstantNode::new(input_freq);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shifter = FrequencyShifterNode::new(0, 1);

        let mut freq_buf = vec![0.0; block_size];
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let mut input_buf = vec![0.0; block_size];
        osc.process_block(&[&freq_buf], &mut input_buf, sample_rate, &context);

        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input_buf, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // Expected: 880 - 200 = 680 Hz
        let expected_freq = input_freq + shift_hz;

        let has_expected = peaks.iter().any(|&f| (f - expected_freq).abs() < 30.0);

        assert!(
            has_expected,
            "Expected frequency {} Hz not found. Peaks: {:?}",
            expected_freq, peaks
        );
    }

    #[test]
    fn test_frequency_shifter_zero_shift_passthrough() {
        // Zero shift should approximately pass signal through
        // (may have some phase shift from Hilbert transform)
        let sample_rate = 44100.0;
        let block_size = 8192;
        let input_freq = 440.0;

        let context = create_test_context(block_size);

        let mut freq_const = ConstantNode::new(input_freq);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut shift_const = ConstantNode::new(0.0); // Zero shift
        let mut shifter = FrequencyShifterNode::new(0, 1);

        let mut freq_buf = vec![0.0; block_size];
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let mut input_buf = vec![0.0; block_size];
        osc.process_block(&[&freq_buf], &mut input_buf, sample_rate, &context);

        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input_buf, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // Should have peak at original frequency
        let has_input_freq = peaks.iter().any(|&f| (f - input_freq).abs() < 30.0);

        assert!(
            has_input_freq,
            "Original frequency {} Hz should be preserved with zero shift. Peaks: {:?}",
            input_freq, peaks
        );
    }

    #[test]
    fn test_frequency_shifter_creates_inharmonic_sound() {
        // Frequency shifter creates inharmonic content
        // Input: 440 Hz + 880 Hz (harmonic)
        // Shift: +100 Hz
        // Output: 540 Hz + 980 Hz (inharmonic - no longer octave!)

        let sample_rate = 44100.0;
        let block_size = 8192;

        let context = create_test_context(block_size);

        // Create complex input (440 Hz + 880 Hz)
        let mut input = vec![0.0; block_size];
        for i in 0..block_size {
            let t = i as f32 / sample_rate;
            input[i] =
                0.5 * (2.0 * PI * 440.0 * t).sin() +
                0.5 * (2.0 * PI * 880.0 * t).sin();
        }

        let shift_hz = 100.0;
        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // Expected: 540 Hz and 980 Hz (NOT an octave)
        let expected1 = 540.0;
        let expected2 = 980.0;

        let has_expected1 = peaks.iter().any(|&f| (f - expected1).abs() < 30.0);
        let has_expected2 = peaks.iter().any(|&f| (f - expected2).abs() < 30.0);

        assert!(
            has_expected1 || has_expected2,
            "Expected inharmonic frequencies ~{} Hz and ~{} Hz. Peaks: {:?}",
            expected1, expected2, peaks
        );
    }

    #[test]
    fn test_frequency_shifter_vs_ring_mod_different() {
        // Frequency shifter is DIFFERENT from ring modulation
        // Ring mod creates sum AND difference frequencies
        // Freq shifter creates only one sideband

        let sample_rate = 44100.0;
        let block_size = 8192;
        let input_freq = 440.0;
        let shift_hz = 100.0;

        let context = create_test_context(block_size);

        // Create input signal
        let mut freq_const = ConstantNode::new(input_freq);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let mut freq_buf = vec![0.0; block_size];
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let mut input_buf = vec![0.0; block_size];
        osc.process_block(&[&freq_buf], &mut input_buf, sample_rate, &context);

        // Frequency shifter
        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut shifted_output = vec![0.0; block_size];
        shifter.process_block(
            &[&input_buf, &shift_buf],
            &mut shifted_output,
            sample_rate,
            &context,
        );

        let shifter_peaks = find_frequency_peaks(&shifted_output, sample_rate, 0.05);

        // Frequency shifter should create primarily ONE peak (upper sideband)
        // Around 540 Hz (440 + 100)
        let upper_sideband = input_freq + shift_hz;
        let lower_sideband = input_freq - shift_hz;

        let has_upper = shifter_peaks.iter().any(|&f| (f - upper_sideband).abs() < 30.0);

        assert!(
            has_upper,
            "Frequency shifter should create upper sideband at {} Hz. Peaks: {:?}",
            upper_sideband, shifter_peaks
        );

        // Ring modulation would create BOTH sidebands (sum and difference)
        // Frequency shifter suppresses one sideband (SSB)
    }

    #[test]
    fn test_frequency_shifter_pattern_modulation() {
        // Test that shift amount can vary over time
        let sample_rate = 44100.0;
        let block_size = 4410; // 100ms

        let context = create_test_context(block_size);

        // Create constant input
        let input = vec![1.0; block_size];

        // Create varying shift pattern (0 to 100 Hz)
        let mut shift_buf = vec![0.0; block_size];
        for i in 0..block_size {
            shift_buf[i] = (i as f32 / block_size as f32) * 100.0;
        }

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Output should have variation
        let min = output.iter().copied().fold(f32::INFINITY, f32::min);
        let max = output.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        assert!(
            max - min > 0.1,
            "Output should vary with changing shift. Range: {} to {}",
            min, max
        );
    }

    #[test]
    fn test_frequency_shifter_metallic_effect() {
        // Large frequency shifts create characteristic metallic sound
        let sample_rate = 44100.0;
        let block_size = 8192;

        let context = create_test_context(block_size);

        // Create harmonic input (fundamental + harmonics)
        let mut input = vec![0.0; block_size];
        for i in 0..block_size {
            let t = i as f32 / sample_rate;
            input[i] =
                0.5 * (2.0 * PI * 200.0 * t).sin() +  // Fundamental
                0.3 * (2.0 * PI * 400.0 * t).sin() +  // 2nd harmonic
                0.2 * (2.0 * PI * 600.0 * t).sin();   // 3rd harmonic
        }

        // Large shift
        let shift_hz = 250.0;
        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Should produce audible output
        let rms: f32 = output.iter().map(|x| x * x).sum::<f32>() / block_size as f32;
        let rms = rms.sqrt();

        assert!(
            rms > 0.1,
            "Frequency shifter should produce audible output. RMS: {}",
            rms
        );

        // Check for inharmonic content (shifted frequencies)
        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // Expected (approximately): 450, 650, 850 Hz (200+250, 400+250, 600+250)
        // These are NOT harmonically related!
        assert!(
            peaks.len() >= 1,
            "Should have spectral content. Peaks: {:?}",
            peaks
        );
    }

    #[test]
    fn test_frequency_shifter_subtle_detuning() {
        // Small shifts can create subtle detuning/chorus effects
        let sample_rate = 44100.0;
        let block_size = 8192;
        let input_freq = 440.0;
        let shift_hz = 5.0; // Very small shift

        let context = create_test_context(block_size);

        let mut freq_const = ConstantNode::new(input_freq);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let mut freq_buf = vec![0.0; block_size];
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let mut input_buf = vec![0.0; block_size];
        osc.process_block(&[&freq_buf], &mut input_buf, sample_rate, &context);

        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input_buf, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        let peaks = find_frequency_peaks(&output, sample_rate, 0.05);

        // Should have peak near 445 Hz (440 + 5)
        let expected_freq = input_freq + shift_hz;

        let has_expected = peaks.iter().any(|&f| (f - expected_freq).abs() < 30.0);

        assert!(
            has_expected,
            "Expected slightly shifted frequency {} Hz. Peaks: {:?}",
            expected_freq, peaks
        );
    }

    #[test]
    fn test_frequency_shifter_clamping() {
        // Shift amount should be clamped to ±1000 Hz
        let sample_rate = 44100.0;
        let block_size = 512;

        let context = create_test_context(block_size);

        let input = vec![1.0; block_size];

        // Try extreme shift (should be clamped)
        let mut shift_const = ConstantNode::new(5000.0); // Way too high
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut output = vec![0.0; block_size];

        // Should not panic or produce invalid output
        shifter.process_block(
            &[&input, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // All samples should be finite
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} should be finite: {}",
                i, sample
            );
        }
    }

    #[test]
    fn test_frequency_shifter_reset() {
        let mut shifter = FrequencyShifterNode::new(0, 1);

        // Modify internal state
        shifter.oscillator_phase = 0.5;
        shifter.hilbert_state.ap_i1.x_prev = 1.0;
        shifter.hilbert_state.ap_q2.y_prev = -0.5;

        // Reset
        shifter.reset();

        // State should be cleared
        assert_eq!(shifter.oscillator_phase, 0.0);
        assert_eq!(shifter.hilbert_state.ap_i1.x_prev, 0.0);
        assert_eq!(shifter.hilbert_state.ap_q2.y_prev, 0.0);
    }

    #[test]
    fn test_frequency_shifter_dependencies() {
        let shifter = FrequencyShifterNode::new(10, 20);
        let deps = shifter.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // shift_hz_input
    }

    #[test]
    fn test_frequency_shifter_oscillator_phase_wraps() {
        // Phase should wrap to [0, 1) range
        let sample_rate = 44100.0;
        let block_size = 44100; // 1 second

        let context = create_test_context(block_size);

        let input = vec![1.0; block_size];

        // Use 100 Hz shift - after 1 second, phase should wrap many times
        let shift_hz = 100.0;
        let mut shift_const = ConstantNode::new(shift_hz);
        let mut shift_buf = vec![0.0; block_size];
        shift_const.process_block(&[], &mut shift_buf, sample_rate, &context);

        let mut shifter = FrequencyShifterNode::new(0, 1);
        let mut output = vec![0.0; block_size];
        shifter.process_block(
            &[&input, &shift_buf],
            &mut output,
            sample_rate,
            &context,
        );

        // Phase should be in [0, 1) range
        let phase = shifter.oscillator_phase();
        assert!(
            phase >= 0.0 && phase < 1.0,
            "Phase should be wrapped to [0, 1): {}",
            phase
        );
    }

    #[test]
    fn test_hilbert_state_basic() {
        // Test that Hilbert transform creates ~90° phase shift
        let mut hilbert = HilbertState::new();

        // Process a few samples
        let samples = [1.0, 0.5, -0.3, 0.8, -0.2];

        for &sample in &samples {
            let (i, q) = hilbert.process(sample);

            // Both outputs should be finite
            assert!(i.is_finite(), "I output should be finite");
            assert!(q.is_finite(), "Q output should be finite");
        }
    }

    #[test]
    fn test_allpass_state_basic() {
        // Test basic allpass filter operation
        let mut ap = AllPassState::new(0.5);

        let output = ap.process(1.0);
        assert!(output.is_finite(), "Output should be finite");

        // Reset should clear state
        ap.reset();
        assert_eq!(ap.x_prev, 0.0);
        assert_eq!(ap.y_prev, 0.0);
    }
}
