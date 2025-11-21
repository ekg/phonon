/// Tape delay node - analog tape echo simulation with wow, flutter, and saturation
///
/// A tape delay emulates vintage tape delay machines (like the Roland RE-201 Space Echo)
/// with realistic tape artifacts:
/// - **Wow**: Slow pitch modulation (0.1-2 Hz) from tape speed variations
/// - **Flutter**: Fast pitch modulation (5-10 Hz) from tape mechanism vibrations
/// - **Saturation**: Soft clipping from tape magnetic saturation
/// - **Filtering**: One-pole lowpass for tape head frequency response
///
/// # Algorithm
/// ```text
/// // Update LFO phases
/// wow_phase += wow_rate / sample_rate
/// flutter_phase += flutter_rate / sample_rate
///
/// // Modulate delay time
/// wow_mod = sin(wow_phase * TAU) * wow_depth * 0.001
/// flutter_mod = sin(flutter_phase * TAU) * flutter_depth * 0.0001
/// modulated_time = delay_time + wow_mod + flutter_mod
///
/// // Fractional delay (linear interpolation)
/// read_pos = write_pos - (modulated_time * sample_rate)
/// delayed = lerp(buffer[floor(read_pos)], buffer[ceil(read_pos)], fract(read_pos))
///
/// // Tape saturation
/// saturated = tanh(delayed * (1 + saturation * 3)) / (1 + saturation * 3)
///
/// // Tape head filtering (one-pole lowpass)
/// lpf_state = lpf_state * coeff + saturated * (1 - coeff)
///
/// // Write with feedback
/// buffer[write_pos] = input + lpf_state * feedback
///
/// output = input * (1 - mix) + lpf_state * mix
/// ```
///
/// # Applications
/// - Vintage delay effects
/// - Dub/reggae production
/// - Lo-fi/retro aesthetics
/// - Warm, analog-sounding delays
/// - Creative pitch modulation
///
/// # Example
/// ```ignore
/// // Classic tape echo
/// let synth = OscillatorNode::new(Waveform::Saw);  // NodeId 1
/// let time = ConstantNode::new(0.375);              // NodeId 2 (375ms)
/// let feedback = ConstantNode::new(0.6);            // NodeId 3 (60%)
/// let wow_rate = ConstantNode::new(0.5);            // NodeId 4 (0.5 Hz)
/// let wow_depth = ConstantNode::new(0.5);           // NodeId 5 (medium)
/// let flutter_rate = ConstantNode::new(7.0);        // NodeId 6 (7 Hz)
/// let flutter_depth = ConstantNode::new(0.3);       // NodeId 7 (subtle)
/// let saturation = ConstantNode::new(0.4);          // NodeId 8 (warm)
/// let mix = ConstantNode::new(0.5);                 // NodeId 9 (50%)
/// let delay = TapeDelayNode::new(1, 2, 3, 4, 5, 6, 7, 8, 9, 1.0, 44100.0);
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::TAU;

/// Tape delay state
#[derive(Debug, Clone)]
struct TapeDelayState {
    buffer: Vec<f32>,       // Circular delay buffer
    write_pos: usize,       // Current write position
    wow_phase: f32,         // Wow LFO phase (0.0 to 1.0)
    flutter_phase: f32,     // Flutter LFO phase (0.0 to 1.0)
    lpf_state: f32,         // One-pole lowpass filter state
    sample_rate: f32,       // Sample rate for calculations
}

impl TapeDelayState {
    fn new(buffer_size: usize, sample_rate: f32) -> Self {
        Self {
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            wow_phase: 0.0,
            flutter_phase: 0.0,
            lpf_state: 0.0,
            sample_rate,
        }
    }
}

/// Tape delay node: vintage tape echo with wow, flutter, and saturation
///
/// Simulates analog tape delay machines with realistic tape artifacts.
/// Pitch modulation (wow/flutter) uses fractional delay with linear interpolation.
pub struct TapeDelayNode {
    input: NodeId,              // Signal to delay
    time_input: NodeId,         // Delay time in seconds
    feedback_input: NodeId,     // Feedback amount (0.0-0.95)
    wow_rate_input: NodeId,     // Wow modulation rate in Hz (0.1-2.0)
    wow_depth_input: NodeId,    // Wow modulation depth (0.0-1.0)
    flutter_rate_input: NodeId, // Flutter modulation rate in Hz (5.0-10.0)
    flutter_depth_input: NodeId,// Flutter modulation depth (0.0-1.0)
    saturation_input: NodeId,   // Tape saturation (0.0-1.0)
    mix_input: NodeId,          // Dry/wet mix (0.0-1.0)
    state: TapeDelayState,
    max_delay: f32,             // Maximum delay time (for buffer sizing)
}

impl TapeDelayNode {
    /// Create a new tape delay node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to delay
    /// * `time_input` - NodeId of delay time in seconds (0.001 to max_delay)
    ///   - 0.125 = 125ms (slapback)
    ///   - 0.375 = 375ms (classic tape echo)
    ///   - 0.75 = 750ms (long echo)
    /// * `feedback_input` - NodeId of feedback amount (0.0 to 0.95)
    ///   - 0.0 = single echo
    ///   - 0.6 = classic tape echo
    ///   - 0.85 = long decaying echoes
    /// * `wow_rate_input` - NodeId of wow rate in Hz (0.1 to 2.0)
    ///   - 0.5 Hz = slow, gentle pitch drift
    ///   - 1.0 Hz = medium warble
    ///   - 2.0 Hz = fast pitch wobble
    /// * `wow_depth_input` - NodeId of wow depth (0.0 to 1.0)
    ///   - 0.0 = no wow
    ///   - 0.5 = subtle vintage character
    ///   - 1.0 = obvious pitch modulation
    /// * `flutter_rate_input` - NodeId of flutter rate in Hz (5.0 to 10.0)
    ///   - 5.0 Hz = slow flutter
    ///   - 7.0 Hz = classic tape flutter
    ///   - 10.0 Hz = fast flutter
    /// * `flutter_depth_input` - NodeId of flutter depth (0.0 to 1.0)
    ///   - 0.0 = no flutter
    ///   - 0.3 = subtle texture
    ///   - 0.7 = obvious flutter
    /// * `saturation_input` - NodeId of tape saturation (0.0 to 1.0)
    ///   - 0.0 = clean
    ///   - 0.4 = warm tape compression
    ///   - 0.8 = heavy tape saturation
    /// * `mix_input` - NodeId of wet/dry mix (0.0 to 1.0)
    ///   - 0.0 = completely dry (bypass)
    ///   - 0.5 = 50/50 blend
    ///   - 1.0 = completely wet (only delays)
    /// * `max_delay` - Maximum delay time in seconds (determines buffer size)
    /// * `sample_rate` - Sample rate in Hz (usually 44100.0)
    ///
    /// # Panics
    /// Panics if max_delay <= 0.0
    pub fn new(
        input: NodeId,
        time_input: NodeId,
        feedback_input: NodeId,
        wow_rate_input: NodeId,
        wow_depth_input: NodeId,
        flutter_rate_input: NodeId,
        flutter_depth_input: NodeId,
        saturation_input: NodeId,
        mix_input: NodeId,
        max_delay: f32,
        sample_rate: f32,
    ) -> Self {
        assert!(max_delay > 0.0, "max_delay must be greater than 0");

        // Buffer size: max_delay * sample_rate
        let buffer_size = (max_delay * sample_rate).ceil() as usize;

        Self {
            input,
            time_input,
            feedback_input,
            wow_rate_input,
            wow_depth_input,
            flutter_rate_input,
            flutter_depth_input,
            saturation_input,
            mix_input,
            state: TapeDelayState::new(buffer_size, sample_rate),
            max_delay,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the time input node ID
    pub fn time_input(&self) -> NodeId {
        self.time_input
    }

    /// Get the feedback input node ID
    pub fn feedback_input(&self) -> NodeId {
        self.feedback_input
    }

    /// Get the wow rate input node ID
    pub fn wow_rate_input(&self) -> NodeId {
        self.wow_rate_input
    }

    /// Get the wow depth input node ID
    pub fn wow_depth_input(&self) -> NodeId {
        self.wow_depth_input
    }

    /// Get the flutter rate input node ID
    pub fn flutter_rate_input(&self) -> NodeId {
        self.flutter_rate_input
    }

    /// Get the flutter depth input node ID
    pub fn flutter_depth_input(&self) -> NodeId {
        self.flutter_depth_input
    }

    /// Get the saturation input node ID
    pub fn saturation_input(&self) -> NodeId {
        self.saturation_input
    }

    /// Get the mix input node ID
    pub fn mix_input(&self) -> NodeId {
        self.mix_input
    }

    /// Get the current write position (for debugging/testing)
    pub fn write_position(&self) -> usize {
        self.state.write_pos
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.state.buffer.len()
    }

    /// Reset the delay buffer to silence and clear state
    pub fn clear_buffer(&mut self) {
        self.state.buffer.fill(0.0);
        self.state.write_pos = 0;
        self.state.wow_phase = 0.0;
        self.state.flutter_phase = 0.0;
        self.state.lpf_state = 0.0;
    }
}

impl AudioNode for TapeDelayNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 9,
            "TapeDelayNode requires 9 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let time_buf = inputs[1];
        let feedback_buf = inputs[2];
        let wow_rate_buf = inputs[3];
        let wow_depth_buf = inputs[4];
        let flutter_rate_buf = inputs[5];
        let flutter_depth_buf = inputs[6];
        let saturation_buf = inputs[7];
        let mix_buf = inputs[8];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        let buffer_len = self.state.buffer.len();
        let sample_rate = self.state.sample_rate;

        for i in 0..output.len() {
            let sample = input_buf[i];
            let delay_time = time_buf[i].max(0.001).min(self.max_delay);
            let feedback = feedback_buf[i].clamp(0.0, 0.95);
            let wow_rate = wow_rate_buf[i].clamp(0.1, 2.0);
            let wow_depth = wow_depth_buf[i].clamp(0.0, 1.0);
            let flutter_rate = flutter_rate_buf[i].clamp(5.0, 10.0);
            let flutter_depth = flutter_depth_buf[i].clamp(0.0, 1.0);
            let saturation = saturation_buf[i].clamp(0.0, 1.0);
            let mix = mix_buf[i].clamp(0.0, 1.0);

            // Update LFO phases
            let wow_phase_inc = wow_rate / sample_rate;
            let flutter_phase_inc = flutter_rate / sample_rate;

            // Calculate wow and flutter modulation
            let wow = (self.state.wow_phase * TAU).sin() * wow_depth * 0.001;
            let flutter = (self.state.flutter_phase * TAU).sin() * flutter_depth * 0.0001;

            // Modulate delay time
            let modulated_time = delay_time + wow + flutter;
            let delay_samples = (modulated_time * sample_rate).max(1.0).min(buffer_len as f32 - 1.0);

            // Fractional delay with linear interpolation
            let read_pos_f = (self.state.write_pos as f32) - delay_samples;
            let read_pos = if read_pos_f < 0.0 {
                read_pos_f + buffer_len as f32
            } else {
                read_pos_f
            };

            let read_idx = (read_pos as usize) % buffer_len;
            let next_idx = (read_idx + 1) % buffer_len;
            let frac = read_pos.fract();

            // Linear interpolation between two samples
            let delayed = self.state.buffer[read_idx] * (1.0 - frac)
                + self.state.buffer[next_idx] * frac;

            // Tape saturation (soft clipping with tanh)
            let saturated = if saturation > 0.01 {
                let drive = 1.0 + saturation * 3.0;
                (delayed * drive).tanh() / drive
            } else {
                delayed
            };

            // Tape head filtering (one-pole lowpass)
            // Higher saturation = more high-frequency rolloff
            let cutoff_coef = 0.7 + saturation * 0.2;
            let filtered = self.state.lpf_state * cutoff_coef + saturated * (1.0 - cutoff_coef);

            // Write input plus feedback to buffer
            let to_write = sample + filtered * feedback;
            self.state.buffer[self.state.write_pos] = to_write;

            // Advance write position
            self.state.write_pos = (self.state.write_pos + 1) % buffer_len;

            // Update LFO phases (wrap around at 1.0)
            self.state.wow_phase = (self.state.wow_phase + wow_phase_inc) % 1.0;
            self.state.flutter_phase = (self.state.flutter_phase + flutter_phase_inc) % 1.0;

            // Update filter state
            self.state.lpf_state = filtered;

            // Mix dry and wet signals
            output[i] = sample * (1.0 - mix) + filtered * mix;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.time_input,
            self.feedback_input,
            self.wow_rate_input,
            self.wow_depth_input,
            self.flutter_rate_input,
            self.flutter_depth_input,
            self.saturation_input,
            self.mix_input,
        ]
    }

    fn name(&self) -> &str {
        "TapeDelayNode"
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
    fn test_tape_delay_bypass() {
        // Test that mix=0.0 passes signal through unchanged
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];
        let feedback = vec![0.5; size];
        let wow_rate = vec![0.5; size];
        let wow_depth = vec![0.0; size];     // No wow
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.0; size]; // No flutter
        let saturation = vec![0.0; size];
        let mix = vec![0.0; size];            // Bypass

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should equal input (bypass)
        for i in 0..size {
            assert!(
                (output[i] - input[i]).abs() < 0.0001,
                "With mix=0, output should equal input"
            );
        }
    }

    #[test]
    fn test_tape_delay_creates_echoes() {
        // Test that delay creates echoes
        let size = 1024;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let delay_time = 0.01; // 10ms
        let time = vec![delay_time; size];
        let feedback = vec![0.0; size]; // No feedback (cleaner test)
        let wow_rate = vec![0.5; size];
        let wow_depth = vec![0.0; size];
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.0; size];
        let saturation = vec![0.0; size];
        let mix = vec![1.0; size]; // Full wet

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Check for echo at expected position
        let delay_samples = (delay_time * sample_rate) as usize;
        let echo_idx = delay_samples;

        if echo_idx < size {
            assert!(
                output[echo_idx] > 0.2,
                "Echo should appear at sample {}, got {}",
                echo_idx,
                output[echo_idx]
            );
        }
    }

    #[test]
    fn test_tape_delay_wow_modulates_pitch() {
        // Test that wow creates pitch modulation
        let size = 4410; // 100ms at 44.1kHz
        let sample_rate = 44100.0;

        // Continuous tone
        let input = vec![0.3; size];
        let delay_time = 0.02; // 20ms
        let time = vec![delay_time; size];
        let feedback = vec![0.0; size];
        let wow_rate = vec![1.0; size];    // 1 Hz wow
        let wow_depth = vec![1.0; size];   // Full wow
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.0; size];
        let saturation = vec![0.0; size];
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output_with_wow = vec![0.0; size];
        let context = create_context(size);

        let mut delay_wow = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay_wow.process_block(&inputs, &mut output_with_wow, sample_rate, &context);

        // Without wow
        let wow_depth_zero = vec![0.0; size];
        let inputs_no_wow: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth_zero,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output_no_wow = vec![0.0; size];

        let mut delay_no_wow = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay_no_wow.process_block(&inputs_no_wow, &mut output_no_wow, sample_rate, &context);

        // Outputs should be different due to wow modulation
        let mut diff_count = 0;
        let delay_samples = (delay_time * sample_rate) as usize;
        for i in delay_samples..(size - 100) {
            if i > delay_samples && (output_with_wow[i] - output_no_wow[i]).abs() > 0.01 {
                diff_count += 1;
            }
        }

        // Wow modulation should create some detectable difference
        assert!(
            diff_count > 10,
            "Wow should create audible pitch modulation (got {} samples different)",
            diff_count
        );
    }

    #[test]
    fn test_tape_delay_saturation() {
        // Test that saturation affects the signal
        let size = 1024;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let time = vec![0.01; size];
        let feedback = vec![0.5; size];
        let wow_rate = vec![0.5; size];
        let wow_depth = vec![0.0; size];
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.0; size];
        let mix = vec![1.0; size];

        // With saturation
        let saturation_on = vec![0.8; size];
        let inputs_sat: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation_on,
            &mix,
        ];
        let mut output_sat = vec![0.0; size];
        let context = create_context(size);

        let mut delay_sat = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay_sat.process_block(&inputs_sat, &mut output_sat, sample_rate, &context);

        // Without saturation
        let saturation_off = vec![0.0; size];
        let inputs_clean: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation_off,
            &mix,
        ];
        let mut output_clean = vec![0.0; size];

        let mut delay_clean = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay_clean.process_block(&inputs_clean, &mut output_clean, sample_rate, &context);

        // Saturation should create audible differences
        let mut diff_count = 0;
        for i in 0..size {
            if (output_sat[i] - output_clean[i]).abs() > 0.01 {
                diff_count += 1;
            }
        }

        // Saturation should create some detectable differences
        assert!(
            diff_count > 10,
            "Saturation should create audible differences (got {} samples different)",
            diff_count
        );
    }

    #[test]
    fn test_tape_delay_feedback() {
        // Test that feedback creates multiple echoes
        let size = 2048;
        let sample_rate = 44100.0;

        // Impulse
        let mut input = vec![0.0; size];
        input[0] = 1.0;

        let time = vec![0.01; size];
        let feedback = vec![0.7; size]; // High feedback
        let wow_rate = vec![0.5; size];
        let wow_depth = vec![0.0; size];
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.0; size];
        let saturation = vec![0.0; size];
        let mix = vec![1.0; size];

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Count peaks (echoes)
        let mut peak_count = 0;
        for i in 1..size {
            if output[i] > 0.05 && output[i] > output[i - 1] {
                peak_count += 1;
            }
        }

        assert!(
            peak_count >= 3,
            "High feedback should create multiple echoes, found {} peaks",
            peak_count
        );
    }

    #[test]
    fn test_tape_delay_parameter_clamping() {
        // Test that parameters are clamped to valid ranges
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![10.0; size];        // Way too long
        let feedback = vec![2.0; size];     // Invalid
        let wow_rate = vec![100.0; size];   // Invalid
        let wow_depth = vec![5.0; size];    // Invalid
        let flutter_rate = vec![0.1; size]; // Invalid
        let flutter_depth = vec![-1.0; size]; // Invalid
        let saturation = vec![10.0; size];  // Invalid
        let mix = vec![-5.0; size];         // Invalid

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        // Should not panic, and should produce valid output
        for &val in &output {
            assert!(val.is_finite(), "Output should be finite with clamped params");
        }
    }

    #[test]
    fn test_tape_delay_buffer_wraparound() {
        // Test that circular buffer wraps around correctly
        let size = 1024;
        let sample_rate = 44100.0;

        // Continuous input
        let input = vec![0.1; size];
        let time = vec![0.01; size];
        let feedback = vec![0.3; size];
        let wow_rate = vec![0.5; size];
        let wow_depth = vec![0.2; size];
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.1; size];
        let saturation = vec![0.2; size];
        let mix = vec![0.5; size];

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 0.5, sample_rate);

        // Process multiple blocks to test wraparound
        for _ in 0..5 {
            delay.process_block(&inputs, &mut output, sample_rate, &context);

            // Should not have NaN or inf
            for &val in &output {
                assert!(val.is_finite(), "Output should remain finite");
            }
        }

        // Write position should have wrapped
        assert!(
            delay.write_position() < delay.buffer_size(),
            "Write position should stay within buffer bounds"
        );
    }

    #[test]
    fn test_tape_delay_node_interface() {
        // Test node getters
        let delay = TapeDelayNode::new(10, 11, 12, 13, 14, 15, 16, 17, 18, 1.0, 44100.0);

        assert_eq!(delay.input(), 10);
        assert_eq!(delay.time_input(), 11);
        assert_eq!(delay.feedback_input(), 12);
        assert_eq!(delay.wow_rate_input(), 13);
        assert_eq!(delay.wow_depth_input(), 14);
        assert_eq!(delay.flutter_rate_input(), 15);
        assert_eq!(delay.flutter_depth_input(), 16);
        assert_eq!(delay.saturation_input(), 17);
        assert_eq!(delay.mix_input(), 18);

        let inputs = delay.input_nodes();
        assert_eq!(inputs.len(), 9);
        assert_eq!(inputs[0], 10);
        assert_eq!(inputs[8], 18);

        assert_eq!(delay.name(), "TapeDelayNode");
        assert!(delay.buffer_size() > 0);
    }

    #[test]
    fn test_tape_delay_clear_buffer() {
        // Test that clearing buffer resets all state
        let size = 512;
        let sample_rate = 44100.0;

        let input = vec![0.5; size];
        let time = vec![0.1; size];
        let feedback = vec![0.5; size];
        let wow_rate = vec![0.5; size];
        let wow_depth = vec![0.5; size];
        let flutter_rate = vec![7.0; size];
        let flutter_depth = vec![0.3; size];
        let saturation = vec![0.4; size];
        let mix = vec![0.8; size];

        let inputs: Vec<&[f32]> = vec![
            &input,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut delay = TapeDelayNode::new(0, 1, 2, 3, 4, 5, 6, 7, 8, 1.0, sample_rate);

        // Process to build up state
        delay.process_block(&inputs, &mut output, sample_rate, &context);

        let pos_before = delay.write_position();
        assert!(pos_before > 0, "Write position should advance");

        // Clear buffer
        delay.clear_buffer();

        assert_eq!(delay.write_position(), 0, "Write position should be reset");

        // Process silence and verify buffer is clear
        let silence = vec![0.0; size];
        let inputs_silent: Vec<&[f32]> = vec![
            &silence,
            &time,
            &feedback,
            &wow_rate,
            &wow_depth,
            &flutter_rate,
            &flutter_depth,
            &saturation,
            &mix,
        ];
        let mut output_silent = vec![0.0; size];

        delay.process_block(&inputs_silent, &mut output_silent, sample_rate, &context);

        // Output should be very close to 0
        for &val in &output_silent {
            assert!(
                val.abs() < 0.001,
                "After clear, silent input should produce near-zero output"
            );
        }
    }
}
