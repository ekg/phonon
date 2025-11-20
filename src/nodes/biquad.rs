/// Biquad filter node - high-quality second-order IIR filter with multiple modes
///
/// Implements the classic RBJ (Robert Bristow-Johnson) Audio EQ Cookbook biquad
/// filter formulas. Biquad filters are the foundation of most digital audio filtering,
/// providing excellent frequency response with minimal CPU usage.
///
/// # Filter Modes
/// - **Lowpass** (mode 0): Passes low frequencies, attenuates high frequencies
/// - **Highpass** (mode 1): Passes high frequencies, attenuates low frequencies
/// - **Bandpass** (mode 2): Passes a band around center frequency
/// - **Notch** (mode 3): Rejects a narrow band around center frequency
///
/// # Algorithm (RBJ Cookbook)
/// ```text
/// 1. Calculate normalized frequency: ω = 2π * freq / sample_rate
/// 2. Calculate filter coefficients based on mode (LP/HP/BP/Notch)
/// 3. Apply biquad difference equation:
///    y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
/// ```
///
/// # Applications
/// - Synthesizer filters (lowpass/highpass sweeps)
/// - EQ sections in mixing consoles
/// - Notch filters for feedback elimination
/// - Bandpass filters for formant synthesis
///
/// # Example
/// ```ignore
/// // Lowpass filter at 1000 Hz with Q=0.707 (Butterworth)
/// let audio = OscillatorNode::new(Waveform::Saw);  // NodeId 1
/// let freq = ConstantNode::new(1000.0);             // NodeId 2
/// let q = ConstantNode::new(0.707);                 // NodeId 3
/// let filt = BiquadNode::new(1, 2, 3, FilterMode::Lowpass); // NodeId 4
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Biquad filter modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Lowpass = 0,
    Highpass = 1,
    Bandpass = 2,
    Notch = 3,
}

/// Biquad filter state (history buffers for IIR)
#[derive(Debug, Clone)]
struct BiquadState {
    x1: f32, // Input delayed by 1 sample
    x2: f32, // Input delayed by 2 samples
    y1: f32, // Output delayed by 1 sample
    y2: f32, // Output delayed by 2 samples
}

impl Default for BiquadState {
    fn default() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

/// Biquad filter node: high-quality second-order IIR filter
///
/// Implements RBJ Audio EQ Cookbook biquad filters with multiple modes.
/// Provides excellent frequency response and minimal CPU usage.
pub struct BiquadNode {
    input: NodeId,
    frequency_input: NodeId, // Cutoff/center frequency in Hz
    q_input: NodeId,         // Quality factor (0.1 to ~20.0)
    mode: FilterMode,        // Filter mode (LP/HP/BP/Notch)
    state: BiquadState,      // Filter state (history)
}

impl BiquadNode {
    /// Create a new biquad filter node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to filter
    /// * `frequency_input` - NodeId of cutoff/center frequency in Hz (10 Hz to Nyquist)
    /// * `q_input` - NodeId of quality factor (0.1 to 20.0)
    ///   - 0.5 = wide, gentle slope
    ///   - 0.707 = Butterworth (maximally flat)
    ///   - 1.0 = moderate resonance
    ///   - 10.0+ = very sharp, resonant peak
    /// * `mode` - Filter mode (Lowpass, Highpass, Bandpass, Notch)
    pub fn new(
        input: NodeId,
        frequency_input: NodeId,
        q_input: NodeId,
        mode: FilterMode,
    ) -> Self {
        Self {
            input,
            frequency_input,
            q_input,
            mode,
            state: BiquadState::default(),
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the frequency input node ID
    pub fn frequency_input(&self) -> NodeId {
        self.frequency_input
    }

    /// Get the Q input node ID
    pub fn q_input(&self) -> NodeId {
        self.q_input
    }

    /// Get the filter mode
    pub fn mode(&self) -> FilterMode {
        self.mode
    }

    /// Reset filter state (clear history)
    pub fn reset(&mut self) {
        self.state = BiquadState::default();
    }
}

impl AudioNode for BiquadNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "BiquadNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let freq_buf = inputs[1];
        let q_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let sample = input_buf[i];
            let freq = freq_buf[i].clamp(10.0, sample_rate * 0.45); // Prevent aliasing
            let q = q_buf[i].clamp(0.1, 20.0); // Prevent instability

            // Calculate normalized frequency (RBJ Cookbook)
            let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
            let sin_omega = omega.sin();
            let cos_omega = omega.cos();
            let alpha = sin_omega / (2.0 * q);

            // Calculate coefficients based on mode (RBJ formulas)
            let (b0, b1, b2, a0, a1, a2) = match self.mode {
                FilterMode::Lowpass => {
                    // Lowpass: H(s) = 1 / (s^2 + s/Q + 1)
                    let b1_temp = 1.0 - cos_omega;
                    let b0_temp = b1_temp / 2.0;
                    let b2_temp = b0_temp;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
                FilterMode::Highpass => {
                    // Highpass: H(s) = s^2 / (s^2 + s/Q + 1)
                    let b0_temp = (1.0 + cos_omega) / 2.0;
                    let b1_temp = -(1.0 + cos_omega);
                    let b2_temp = b0_temp;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
                FilterMode::Bandpass => {
                    // Bandpass (constant skirt gain): H(s) = s / (s^2 + s/Q + 1)
                    let b0_temp = alpha;
                    let b1_temp = 0.0;
                    let b2_temp = -alpha;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
                FilterMode::Notch => {
                    // Notch: H(s) = (s^2 + 1) / (s^2 + s/Q + 1)
                    let b0_temp = 1.0;
                    let b1_temp = -2.0 * cos_omega;
                    let b2_temp = 1.0;
                    let a0_temp = 1.0 + alpha;
                    let a1_temp = -2.0 * cos_omega;
                    let a2_temp = 1.0 - alpha;
                    (b0_temp, b1_temp, b2_temp, a0_temp, a1_temp, a2_temp)
                }
            };

            // Normalize coefficients (divide by a0)
            let b0_norm = b0 / a0;
            let b1_norm = b1 / a0;
            let b2_norm = b2 / a0;
            let a1_norm = a1 / a0;
            let a2_norm = a2 / a0;

            // Apply biquad difference equation
            let y = b0_norm * sample
                + b1_norm * self.state.x1
                + b2_norm * self.state.x2
                - a1_norm * self.state.y1
                - a2_norm * self.state.y2;

            // Clamp output and check for stability
            let y_clamped = y.clamp(-10.0, 10.0);
            let final_output = if y_clamped.is_finite() {
                y_clamped
            } else {
                0.0 // Reset on instability
            };

            // Update state (shift history buffers)
            self.state.x2 = self.state.x1;
            self.state.x1 = sample;
            self.state.y2 = self.state.y1;
            self.state.y1 = final_output;

            output[i] = final_output;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.frequency_input, self.q_input]
    }

    fn name(&self) -> &str {
        "BiquadNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
    }

    #[test]
    fn test_biquad_lowpass_attenuates_highs() {
        // Test that lowpass filter attenuates high frequencies
        let size = 4410; // 0.1 seconds at 44.1kHz
        let sample_rate = 44100.0;

        // Generate high frequency tone (5000 Hz)
        let mut input = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input[i] = (2.0 * std::f32::consts::PI * 5000.0 * t).sin();
        }

        // Lowpass at 1000 Hz (should attenuate 5kHz tone)
        let freq = vec![1000.0; size];
        let q = vec![0.707; size]; // Butterworth

        let inputs: Vec<&[f32]> = vec![&input, &freq, &q];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut filter = BiquadNode::new(0, 1, 2, FilterMode::Lowpass);
        filter.process_block(&inputs, &mut output, sample_rate, &context);

        // Calculate RMS of input and output
        let input_rms: f32 = input.iter().skip(100).map(|x| x * x).sum::<f32>() / (size - 100) as f32;
        let output_rms: f32 = output.iter().skip(100).map(|x| x * x).sum::<f32>() / (size - 100) as f32;

        // Output should be significantly attenuated
        assert!(
            output_rms < input_rms * 0.3,
            "Lowpass should attenuate high frequencies: input_rms={:.4}, output_rms={:.4}",
            input_rms.sqrt(),
            output_rms.sqrt()
        );
    }

    #[test]
    fn test_biquad_highpass_attenuates_lows() {
        // Test that highpass filter attenuates low frequencies
        let size = 4410;
        let sample_rate = 44100.0;

        // Generate low frequency tone (100 Hz)
        let mut input = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input[i] = (2.0 * std::f32::consts::PI * 100.0 * t).sin();
        }

        // Highpass at 1000 Hz (should attenuate 100Hz tone)
        let freq = vec![1000.0; size];
        let q = vec![0.707; size];

        let inputs: Vec<&[f32]> = vec![&input, &freq, &q];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut filter = BiquadNode::new(0, 1, 2, FilterMode::Highpass);
        filter.process_block(&inputs, &mut output, sample_rate, &context);

        let input_rms: f32 = input.iter().skip(100).map(|x| x * x).sum::<f32>() / (size - 100) as f32;
        let output_rms: f32 = output.iter().skip(100).map(|x| x * x).sum::<f32>() / (size - 100) as f32;

        assert!(
            output_rms < input_rms * 0.1,
            "Highpass should attenuate low frequencies: input_rms={:.4}, output_rms={:.4}",
            input_rms.sqrt(),
            output_rms.sqrt()
        );
    }

    #[test]
    fn test_biquad_bandpass_passes_center() {
        // Test that bandpass passes frequencies near center, attenuates others
        let size = 4410;
        let sample_rate = 44100.0;

        // Test at center frequency (1000 Hz)
        let mut input_center = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input_center[i] = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        }

        let freq = vec![1000.0; size];
        let q = vec![5.0; size]; // Narrow bandpass

        let inputs_center: Vec<&[f32]> = vec![&input_center, &freq, &q];
        let mut output_center = vec![0.0; size];
        let context = create_context(size);

        let mut filter_center = BiquadNode::new(0, 1, 2, FilterMode::Bandpass);
        filter_center.process_block(&inputs_center, &mut output_center, sample_rate, &context);

        // Test far from center (5000 Hz)
        let mut input_far = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input_far[i] = (2.0 * std::f32::consts::PI * 5000.0 * t).sin();
        }

        let inputs_far: Vec<&[f32]> = vec![&input_far, &freq, &q];
        let mut output_far = vec![0.0; size];

        let mut filter_far = BiquadNode::new(0, 1, 2, FilterMode::Bandpass);
        filter_far.process_block(&inputs_far, &mut output_far, sample_rate, &context);

        let rms_center: f32 = output_center.iter().skip(100).map(|x| x * x).sum::<f32>();
        let rms_far: f32 = output_far.iter().skip(100).map(|x| x * x).sum::<f32>();

        assert!(
            rms_center > rms_far * 10.0,
            "Bandpass should pass center freq much more than far freq: center={:.4}, far={:.4}",
            rms_center.sqrt(),
            rms_far.sqrt()
        );
    }

    #[test]
    fn test_biquad_notch_rejects_center() {
        // Test that notch filter rejects frequencies at center
        let size = 4410;
        let sample_rate = 44100.0;

        // Generate tone at notch frequency (1000 Hz)
        let mut input = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input[i] = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        }

        // Notch at 1000 Hz (should reject this frequency)
        let freq = vec![1000.0; size];
        let q = vec![10.0; size]; // Narrow notch

        let inputs: Vec<&[f32]> = vec![&input, &freq, &q];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut filter = BiquadNode::new(0, 1, 2, FilterMode::Notch);
        filter.process_block(&inputs, &mut output, sample_rate, &context);

        let input_rms: f32 = input.iter().skip(100).map(|x| x * x).sum::<f32>();
        let output_rms: f32 = output.iter().skip(100).map(|x| x * x).sum::<f32>();

        assert!(
            output_rms < input_rms * 0.01,
            "Notch should reject center frequency: input={:.4}, output={:.4}",
            input_rms.sqrt(),
            output_rms.sqrt()
        );
    }

    #[test]
    fn test_biquad_q_affects_slope() {
        // Test that Q parameter affects filter slope/resonance
        let size = 4410;
        let sample_rate = 44100.0;

        let mut input = vec![0.0; size];
        for i in 0..size {
            let t = i as f32 / sample_rate;
            input[i] = (2.0 * std::f32::consts::PI * 1500.0 * t).sin();
        }

        let freq = vec![1000.0; size];

        // Low Q (gentle slope)
        let q_low = vec![0.5; size];
        let inputs_low: Vec<&[f32]> = vec![&input, &freq, &q_low];
        let mut output_low = vec![0.0; size];
        let context = create_context(size);

        let mut filter_low = BiquadNode::new(0, 1, 2, FilterMode::Lowpass);
        filter_low.process_block(&inputs_low, &mut output_low, sample_rate, &context);

        // High Q (sharp slope, resonant)
        let q_high = vec![10.0; size];
        let inputs_high: Vec<&[f32]> = vec![&input, &freq, &q_high];
        let mut output_high = vec![0.0; size];

        let mut filter_high = BiquadNode::new(0, 1, 2, FilterMode::Lowpass);
        filter_high.process_block(&inputs_high, &mut output_high, sample_rate, &context);

        let rms_low: f32 = output_low.iter().skip(100).map(|x| x * x).sum::<f32>();
        let rms_high: f32 = output_high.iter().skip(100).map(|x| x * x).sum::<f32>();

        // High Q should have more attenuation at this frequency (steeper slope)
        assert!(
            rms_low > rms_high,
            "Low Q should attenuate less than high Q: low={:.4}, high={:.4}",
            rms_low.sqrt(),
            rms_high.sqrt()
        );
    }

    #[test]
    fn test_biquad_stability() {
        // Test that filter remains stable with extreme parameters
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![1.0; size];
        let freq = vec![20000.0; size]; // Very high frequency
        let q = vec![20.0; size];        // Very high Q

        let inputs: Vec<&[f32]> = vec![&input, &freq, &q];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut filter = BiquadNode::new(0, 1, 2, FilterMode::Lowpass);
        filter.process_block(&inputs, &mut output, sample_rate, &context);

        // All outputs should be finite
        for &val in &output {
            assert!(val.is_finite(), "Filter should remain stable, got non-finite: {}", val);
            assert!(val.abs() <= 10.0, "Filter should clamp output, got: {}", val);
        }
    }

    #[test]
    fn test_biquad_state_carries_over() {
        // Test that filter state persists across blocks
        let size = 256;
        let sample_rate = 44100.0;

        let mut input = vec![0.0; size];
        input[0] = 1.0; // Impulse

        let freq = vec![1000.0; size];
        let q = vec![0.707; size];

        let inputs: Vec<&[f32]> = vec![&input, &freq, &q];
        let mut output1 = vec![0.0; size];
        let context = create_context(size);

        let mut filter = BiquadNode::new(0, 1, 2, FilterMode::Lowpass);
        filter.process_block(&inputs, &mut output1, sample_rate, &context);

        // Process second block (all zeros)
        let input2 = vec![0.0; size];
        let inputs2: Vec<&[f32]> = vec![&input2, &freq, &q];
        let mut output2 = vec![0.0; size];

        filter.process_block(&inputs2, &mut output2, sample_rate, &context);

        // Second block should have non-zero output (impulse response continues)
        let rms2: f32 = output2.iter().map(|x| x * x).sum::<f32>() / size as f32;
        assert!(
            rms2 > 0.0001,
            "Filter state should carry over between blocks, got rms: {:.6}",
            rms2.sqrt()
        );
    }

    #[test]
    fn test_biquad_reset() {
        // Test that reset clears filter state
        let size = 512;
        let sample_rate = 44100.0;

        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let freq = vec![1000.0; size];
        let q = vec![0.707; size];
        let inputs: Vec<&[f32]> = vec![&input, &freq, &q];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut filter = BiquadNode::new(0, 1, 2, FilterMode::Lowpass);
        filter.process_block(&inputs, &mut output, sample_rate, &context);

        // State should be non-zero
        assert!(filter.state.y1.abs() > 0.0001, "State should be non-zero after processing");

        // Reset
        filter.reset();
        assert_eq!(filter.state.x1, 0.0, "x1 should be cleared");
        assert_eq!(filter.state.x2, 0.0, "x2 should be cleared");
        assert_eq!(filter.state.y1, 0.0, "y1 should be cleared");
        assert_eq!(filter.state.y2, 0.0, "y2 should be cleared");
    }

    #[test]
    fn test_biquad_node_interface() {
        // Test node getters
        let filter = BiquadNode::new(20, 21, 22, FilterMode::Bandpass);

        assert_eq!(filter.input(), 20);
        assert_eq!(filter.frequency_input(), 21);
        assert_eq!(filter.q_input(), 22);
        assert_eq!(filter.mode(), FilterMode::Bandpass);

        let inputs = filter.input_nodes();
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0], 20);
        assert_eq!(inputs[1], 21);
        assert_eq!(inputs[2], 22);

        assert_eq!(filter.name(), "BiquadNode");
    }
}
