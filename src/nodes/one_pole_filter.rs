/// One-pole filter node - simple lowpass or highpass filter
///
/// This node implements a 1st-order (6 dB/octave) filter with
/// pattern-controllable cutoff frequency. It's simpler and more
/// efficient than biquad filters, making it ideal for:
/// - Smoothing control signals
/// - Gentle filtering with analog character
/// - Low CPU usage scenarios
/// - DC blocking (highpass mode)
///
/// # Filter Characteristics
///
/// - **Rolloff**: 6 dB/octave (gentler than biquad's 12 dB/octave)
/// - **CPU**: Very efficient (~2-3 operations per sample)
/// - **Sound**: Classic analog filter character
/// - **Stability**: Excellent (simple one-pole design)
///
/// # Algorithm
///
/// **Lowpass**:
/// ```text
/// b1 = exp(-2π × cutoff / sample_rate)
/// a0 = 1.0 - b1
/// output = a0 × input + b1 × state
/// state = output
/// ```
///
/// **Highpass**:
/// ```text
/// // Subtract lowpass from original signal
/// b1 = exp(-2π × cutoff / sample_rate)
/// a0 = 1.0 - b1
/// lowpass = a0 × input + b1 × state
/// state = lowpass
/// output = input - lowpass
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Filter mode: lowpass or highpass
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OnePoleMode {
    /// Low-pass filter (attenuates high frequencies)
    LowPass,
    /// High-pass filter (attenuates low frequencies)
    HighPass,
}

/// One-pole filter node with pattern-controlled cutoff
///
/// # Example
/// ```ignore
/// // Smooth a control signal with gentle 500 Hz lowpass
/// let signal = OscillatorNode::new(0, Waveform::Saw);     // NodeId 1
/// let cutoff = ConstantNode::new(500.0);                  // NodeId 2
/// let lpf = OnePoleFilterNode::new(1, 2, OnePoleMode::LowPass);  // NodeId 3
///
/// // DC blocking with 20 Hz highpass
/// let hpf = OnePoleFilterNode::new(1, 4, OnePoleMode::HighPass);  // NodeId 5
/// ```
pub struct OnePoleFilterNode {
    /// Input signal to be filtered
    input: NodeId,

    /// Cutoff frequency input (Hz)
    cutoff_input: NodeId,

    /// Filter mode (lowpass or highpass)
    mode: OnePoleMode,

    /// Filter state (maintains memory between samples)
    state: f32,
}

impl OnePoleFilterNode {
    /// Create a new one-pole filter node
    ///
    /// # Arguments
    /// * `input` - NodeId providing signal to filter
    /// * `cutoff_input` - NodeId providing cutoff frequency in Hz
    /// * `mode` - OnePoleMode::LowPass or OnePoleMode::HighPass
    ///
    /// # Notes
    /// - Cutoff frequency should be below Nyquist (sample_rate / 2)
    /// - For lowpass: frequencies above cutoff are attenuated
    /// - For highpass: frequencies below cutoff are attenuated
    /// - 6 dB/octave rolloff (gentler than biquad filters)
    pub fn new(input: NodeId, cutoff_input: NodeId, mode: OnePoleMode) -> Self {
        Self {
            input,
            cutoff_input,
            mode,
            state: 0.0,
        }
    }

    /// Get current filter state (for debugging/testing)
    pub fn state(&self) -> f32 {
        self.state
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        self.state = 0.0;
    }

    /// Get filter mode
    pub fn mode(&self) -> OnePoleMode {
        self.mode
    }

    /// Set filter mode
    pub fn set_mode(&mut self, mode: OnePoleMode) {
        self.mode = mode;
    }
}

impl AudioNode for OnePoleFilterNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            2,
            "OnePoleFilterNode requires 2 inputs: signal, cutoff"
        );

        let input_buffer = inputs[0];
        let cutoff_buffer = inputs[1];

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

        match self.mode {
            OnePoleMode::LowPass => {
                for i in 0..output.len() {
                    let input = input_buffer[i];
                    let cutoff = cutoff_buffer[i].max(1.0).min(sample_rate * 0.49); // Clamp to valid range

                    // Calculate filter coefficient
                    // b1 = exp(-2π × cutoff / sample_rate)
                    let b1 = (-2.0 * PI * cutoff / sample_rate).exp();
                    let a0 = 1.0 - b1;

                    // Apply lowpass filter
                    // output = a0 × input + b1 × state
                    let filtered = a0 * input + b1 * self.state;
                    self.state = filtered;
                    output[i] = filtered;
                }
            }
            OnePoleMode::HighPass => {
                for i in 0..output.len() {
                    let input = input_buffer[i];
                    let cutoff = cutoff_buffer[i].max(1.0).min(sample_rate * 0.49); // Clamp to valid range

                    // Calculate filter coefficient
                    let b1 = (-2.0 * PI * cutoff / sample_rate).exp();
                    let a0 = 1.0 - b1;

                    // Apply lowpass to get state
                    let lowpass = a0 * input + b1 * self.state;
                    self.state = lowpass;

                    // Highpass = input - lowpass
                    output[i] = input - lowpass;
                }
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.cutoff_input]
    }

    fn name(&self) -> &str {
        match self.mode {
            OnePoleMode::LowPass => "OnePoleFilterNode(LowPass)",
            OnePoleMode::HighPass => "OnePoleFilterNode(HighPass)",
        }
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
    fn test_context(size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, size, 2.0, 44100.0)
    }

    #[test]
    fn test_lowpass_dc_passes() {
        // DC (0 Hz) should pass through lowpass unchanged
        let mut dc_input = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut lpf = OnePoleFilterNode::new(0, 1, OnePoleMode::LowPass);

        let context = test_context(512);

        let mut dc_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];

        dc_input.process_block(&[], &mut dc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let inputs = vec![dc_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 512];
        lpf.process_block(&inputs, &mut output, 44100.0, &context);

        // DC should pass through with minimal attenuation (allow for startup transient)
        let output_rms = calculate_rms(&output[100..]);
        assert!(
            output_rms > 0.9,
            "DC signal attenuated too much: RMS = {}",
            output_rms
        );
    }

    #[test]
    fn test_lowpass_high_freq_attenuation() {
        // High frequency (8000 Hz) should be attenuated by 1000 Hz lowpass
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut lpf = OnePoleFilterNode::new(1, 2, OnePoleMode::LowPass);

        let context = test_context(4410);  // 0.1 second to stabilize

        let mut freq_buf = vec![0.0; 4410];
        let mut osc_buf = vec![0.0; 4410];
        let mut cutoff_buf = vec![0.0; 4410];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_lpf = vec![osc_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        lpf.process_block(&inputs_lpf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 8000 Hz is 3 octaves above 1000 Hz
        // 6 dB/octave × 3 octaves = 18 dB attenuation
        // 18 dB ≈ 0.126 amplitude ratio
        // One-pole filters have gentler rolloff, so expect > 0.1 ratio
        assert!(
            output_rms < input_rms * 0.2,
            "High frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_lowpass_passband() {
        // 440 Hz should pass through 2000 Hz lowpass relatively unchanged
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(2000.0);
        let mut lpf = OnePoleFilterNode::new(1, 2, OnePoleMode::LowPass);

        let context = test_context(4410);

        let mut freq_buf = vec![0.0; 4410];
        let mut osc_buf = vec![0.0; 4410];
        let mut cutoff_buf = vec![0.0; 4410];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_lpf = vec![osc_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        lpf.process_block(&inputs_lpf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 440 Hz should pass through 2000 Hz lowpass with minimal attenuation
        assert!(
            output_rms > input_rms * 0.85,
            "Passband signal attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_highpass_dc_blocking() {
        // DC (0 Hz) should be blocked by highpass
        let mut dc_input = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(100.0);
        let mut hpf = OnePoleFilterNode::new(0, 1, OnePoleMode::HighPass);

        let context = test_context(4410);  // 0.1 second to stabilize

        let mut dc_buf = vec![0.0; 4410];
        let mut cutoff_buf = vec![0.0; 4410];

        dc_input.process_block(&[], &mut dc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let inputs = vec![dc_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        hpf.process_block(&inputs, &mut output, 44100.0, &context);

        // After settling, DC should be nearly blocked (check last 100 samples)
        let output_rms = calculate_rms(&output[4310..]);
        assert!(
            output_rms < 0.05,
            "DC signal not blocked by highpass: RMS = {}",
            output_rms
        );
    }

    #[test]
    fn test_highpass_high_freq_passes() {
        // High frequency (8000 Hz) should pass through 1000 Hz highpass
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut hpf = OnePoleFilterNode::new(1, 2, OnePoleMode::HighPass);

        let context = test_context(4410);

        let mut freq_buf = vec![0.0; 4410];
        let mut osc_buf = vec![0.0; 4410];
        let mut cutoff_buf = vec![0.0; 4410];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_hpf = vec![osc_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        hpf.process_block(&inputs_hpf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 8000 Hz should pass through 1000 Hz highpass with minimal attenuation
        assert!(
            output_rms > input_rms * 0.85,
            "High frequency attenuated too much: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_highpass_low_freq_attenuation() {
        // 100 Hz should be attenuated by 1000 Hz highpass
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut hpf = OnePoleFilterNode::new(1, 2, OnePoleMode::HighPass);

        let context = test_context(4410);

        let mut freq_buf = vec![0.0; 4410];
        let mut osc_buf = vec![0.0; 4410];
        let mut cutoff_buf = vec![0.0; 4410];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs_hpf = vec![osc_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        hpf.process_block(&inputs_hpf, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 100 Hz is ~3.3 octaves below 1000 Hz
        // 6 dB/octave × 3.3 octaves ≈ 20 dB attenuation
        // 20 dB ≈ 0.1 amplitude ratio
        assert!(
            output_rms < input_rms * 0.2,
            "Low frequency not attenuated enough: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_6db_octave_rolloff_lowpass() {
        // Verify 6 dB/octave rolloff by measuring at cutoff and one octave above
        let cutoff_freq = 1000.0;
        let test_freq_at_cutoff = 1000.0;
        let test_freq_octave_above = 2000.0;

        // Measure at cutoff frequency
        let mut freq_node = ConstantNode::new(test_freq_at_cutoff);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(cutoff_freq);
        let mut lpf = OnePoleFilterNode::new(1, 2, OnePoleMode::LowPass);

        let context = test_context(4410);

        let mut freq_buf = vec![0.0; 4410];
        let mut osc_buf = vec![0.0; 4410];
        let mut cutoff_buf = vec![0.0; 4410];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let input_rms_at_cutoff = calculate_rms(&osc_buf);
        let mut output = vec![0.0; 4410];
        lpf.process_block(&[osc_buf.as_slice(), cutoff_buf.as_slice()], &mut output, 44100.0, &context);
        let output_rms_at_cutoff = calculate_rms(&output);

        // Measure one octave above cutoff
        freq_node.set_value(test_freq_octave_above);
        osc.reset_phase();
        lpf.reset();

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut osc_buf, 44100.0, &context);

        let input_rms_octave = calculate_rms(&osc_buf);
        lpf.process_block(&[osc_buf.as_slice(), cutoff_buf.as_slice()], &mut output, 44100.0, &context);
        let output_rms_octave = calculate_rms(&output);

        // Calculate attenuation at cutoff and one octave above
        let attn_at_cutoff = 20.0 * (output_rms_at_cutoff / input_rms_at_cutoff).log10();
        let attn_octave = 20.0 * (output_rms_octave / input_rms_octave).log10();

        // Difference should be approximately 6 dB
        // Note: At exactly the cutoff frequency, a one-pole filter gives -3dB attenuation
        // The rolloff per octave approaches 6 dB/octave asymptotically in the stopband
        let rolloff = attn_at_cutoff - attn_octave;

        // One-pole filter should give approximately 4-6 dB/octave at cutoff region
        // (exact 6 dB/octave is achieved further into the stopband)
        assert!(
            rolloff > 3.5 && rolloff < 7.0,
            "Rolloff should be ~4-6 dB/octave at cutoff region, got {} dB/octave (attn@cutoff={} dB, attn@octave={} dB)",
            rolloff, attn_at_cutoff, attn_octave
        );
    }

    #[test]
    fn test_cutoff_pattern_modulation() {
        // Test that cutoff can be modulated sample-by-sample
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut lpf = OnePoleFilterNode::new(1, 2, OnePoleMode::LowPass);

        let context = test_context(512);

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];

        // Modulate cutoff from 100 Hz to 10000 Hz across the buffer
        let mut cutoff_buf: Vec<f32> = (0..512)
            .map(|i| 100.0 + (i as f32 / 512.0) * 9900.0)
            .collect();

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut osc_buf, 44100.0, &context);

        let mut output = vec![0.0; 512];
        lpf.process_block(&[osc_buf.as_slice(), cutoff_buf.as_slice()], &mut output, 44100.0, &context);

        // Should produce output without crashing
        let output_rms = calculate_rms(&output);
        assert!(output_rms > 0.0, "Pattern-modulated filter produced no output");
    }

    #[test]
    fn test_smooth_control_signal() {
        // One-pole lowpass is excellent for smoothing step changes in control signals
        let step_input = vec![0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let cutoff = vec![100.0; 10];  // Low cutoff for heavy smoothing

        let mut lpf = OnePoleFilterNode::new(0, 1, OnePoleMode::LowPass);

        let context = test_context(10);
        let mut output = vec![0.0; 10];

        lpf.process_block(&[step_input.as_slice(), cutoff.as_slice()], &mut output, 44100.0, &context);

        // Output should gradually rise, not jump instantly
        assert!(output[2] > 0.0, "Filter didn't respond to step: output[2] = {}", output[2]);
        assert!(output[2] < 0.5, "Filter didn't smooth step: output[2] = {}", output[2]);
        assert!(output[3] > output[2], "Output not rising smoothly");
        assert!(output[9] < 0.99, "Not enough smoothing: output[9] = {}", output[9]);
    }

    #[test]
    fn test_very_efficient_cpu() {
        // One-pole filters should be extremely fast (just a few operations per sample)
        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut cutoff = ConstantNode::new(1000.0);
        let mut lpf = OnePoleFilterNode::new(1, 2, OnePoleMode::LowPass);

        let context = test_context(44100);  // 1 second at 44.1kHz

        let mut freq_buf = vec![0.0; 44100];
        let mut osc_buf = vec![0.0; 44100];
        let mut cutoff_buf = vec![0.0; 44100];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut osc_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        // Process 1 second of audio
        let mut output = vec![0.0; 44100];

        let start = std::time::Instant::now();
        lpf.process_block(&[osc_buf.as_slice(), cutoff_buf.as_slice()], &mut output, 44100.0, &context);
        let elapsed = start.elapsed();

        // Should complete in under 2ms on any modern CPU (1 second of audio)
        // This is extremely fast - one-pole filters are among the cheapest operations
        assert!(
            elapsed.as_micros() < 2000,
            "One-pole filter too slow: {} μs for 44100 samples (should be < 2000 μs)",
            elapsed.as_micros()
        );
    }

    #[test]
    fn test_state_persistence() {
        // Filter state should persist across process_block calls
        let mut lpf = OnePoleFilterNode::new(0, 1, OnePoleMode::LowPass);

        let input1 = vec![1.0; 10];
        let cutoff1 = vec![1000.0; 10];

        let context = test_context(10);
        let mut output1 = vec![0.0; 10];

        lpf.process_block(&[input1.as_slice(), cutoff1.as_slice()], &mut output1, 44100.0, &context);

        let last_output = output1[9];

        // Process another block - should continue from previous state
        let input2 = vec![1.0; 10];
        let cutoff2 = vec![1000.0; 10];
        let mut output2 = vec![0.0; 10];

        lpf.process_block(&[input2.as_slice(), cutoff2.as_slice()], &mut output2, 44100.0, &context);

        // First sample of second block should be close to last sample of first block
        // (continuing exponential approach - allow for one sample of advancement)
        assert!(
            (output2[0] - last_output).abs() < 0.05,
            "State not persisting: output1[9]={}, output2[0]={}",
            last_output, output2[0]
        );
    }

    #[test]
    fn test_reset() {
        let mut lpf = OnePoleFilterNode::new(0, 1, OnePoleMode::LowPass);

        // Process some audio to build up state
        let input = vec![1.0; 100];
        let cutoff = vec![1000.0; 100];
        let context = test_context(100);
        let mut output = vec![0.0; 100];

        lpf.process_block(&[input.as_slice(), cutoff.as_slice()], &mut output, 44100.0, &context);

        assert!(lpf.state() > 0.5, "State should have built up");

        // Reset should clear state
        lpf.reset();
        assert_eq!(lpf.state(), 0.0, "Reset didn't clear state");
    }

    #[test]
    fn test_mode_switching() {
        let mut filter = OnePoleFilterNode::new(0, 1, OnePoleMode::LowPass);

        assert_eq!(filter.mode(), OnePoleMode::LowPass);

        filter.set_mode(OnePoleMode::HighPass);
        assert_eq!(filter.mode(), OnePoleMode::HighPass);

        filter.set_mode(OnePoleMode::LowPass);
        assert_eq!(filter.mode(), OnePoleMode::LowPass);
    }

    #[test]
    fn test_dependencies() {
        let lpf = OnePoleFilterNode::new(10, 20, OnePoleMode::LowPass);
        let deps = lpf.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10); // signal input
        assert_eq!(deps[1], 20); // cutoff input
    }

    #[test]
    fn test_parameter_clamping() {
        // Test that extreme parameter values are clamped
        let mut signal = ConstantNode::new(1.0);
        let mut cutoff = ConstantNode::new(100000.0); // Way above Nyquist
        let mut lpf = OnePoleFilterNode::new(0, 1, OnePoleMode::LowPass);

        let context = test_context(512);

        let mut signal_buf = vec![0.0; 512];
        let mut cutoff_buf = vec![0.0; 512];

        signal.process_block(&[], &mut signal_buf, 44100.0, &context);
        cutoff.process_block(&[], &mut cutoff_buf, 44100.0, &context);

        let inputs = vec![signal_buf.as_slice(), cutoff_buf.as_slice()];
        let mut output = vec![0.0; 512];

        // Should not panic despite extreme values
        lpf.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        assert!(output.iter().all(|&x| x.is_finite()));
    }
}
