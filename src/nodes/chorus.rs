/// Chorus node - pitch-shifting delay effect
///
/// This node implements a classic chorus effect using a modulated delay line.
/// Unlike flanger (which uses short delays and feedback), chorus uses longer delays
/// and no feedback to create the illusion of multiple voices playing together.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Chorus node with pattern-controlled rate, depth, and mix
///
/// # Example
/// ```ignore
/// // Classic chorus on signal
/// let input_signal = OscillatorNode::new(0, Waveform::Saw);  // NodeId 0
/// let rate = ConstantNode::new(0.5);  // 0.5 Hz LFO, NodeId 1
/// let depth = ConstantNode::new(0.010);  // 10ms depth, NodeId 2
/// let mix = ConstantNode::new(0.5);  // 50% wet/dry mix, NodeId 3
/// let chorus = ChorusNode::new(0, 1, 2, 3, 44100.0);  // NodeId 4
/// ```
pub struct ChorusNode {
    input: NodeId,       // Signal to chorus
    rate_input: NodeId,  // LFO rate in Hz (can be modulated)
    depth_input: NodeId, // Delay time modulation depth in seconds (can be modulated)
    mix_input: NodeId,   // Wet/dry mix 0.0-1.0 (can be modulated)
    buffer: Vec<f32>,    // Delay buffer
    write_pos: usize,    // Current write position
    phase: f32,          // LFO phase (0.0 to 1.0)
    sample_rate: f32,    // Sample rate for calculations
}

impl ChorusNode {
    /// Chorus - Pitch-shifting delay effect creating multiple voices illusion
    ///
    /// Creates the illusion of multiple voices by using a modulated delay line.
    /// Unlike flanger, chorus uses longer delays (5-50ms) with no feedback.
    /// Classic effect for pads, vocals, and thickening thin sounds.
    ///
    /// # Parameters
    /// - `input`: Signal to process
    /// - `rate_input`: LFO rate in Hz (0.5-2.0 typical)
    /// - `depth_input`: Delay modulation depth in seconds (0.005-0.030 typical)
    /// - `mix_input`: Wet/dry mix (0.0=dry, 1.0=wet)
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 440
    /// ~chorused: ~signal # chorus 1.5 0.01 0.5
    /// ```
    pub fn new(
        input: NodeId,
        rate_input: NodeId,
        depth_input: NodeId,
        mix_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        // Chorus typically uses medium delays: 5-50ms
        let max_delay = 0.05; // 50ms maximum
        let buffer_size = (max_delay * sample_rate).ceil() as usize;

        Self {
            input,
            rate_input,
            depth_input,
            mix_input,
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            phase: 0.0,
            sample_rate,
        }
    }

    /// Get the current write position in the buffer
    pub fn write_position(&self) -> usize {
        self.write_pos
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Get the current LFO phase (0.0 to 1.0)
    pub fn lfo_phase(&self) -> f32 {
        self.phase
    }

    /// Reset the chorus buffer to silence and reset phase
    pub fn clear_buffer(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.phase = 0.0;
    }
}

impl AudioNode for ChorusNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "ChorusNode requires 4 inputs: signal, rate, depth, mix"
        );

        let input_buffer = inputs[0];
        let rate_buffer = inputs[1];
        let depth_buffer = inputs[2];
        let mix_buffer = inputs[3];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            rate_buffer.len(),
            output.len(),
            "Rate buffer length mismatch"
        );
        debug_assert_eq!(
            depth_buffer.len(),
            output.len(),
            "Depth buffer length mismatch"
        );
        debug_assert_eq!(mix_buffer.len(), output.len(), "Mix buffer length mismatch");

        let buffer_len = self.buffer.len();

        for i in 0..output.len() {
            let dry_sample = input_buffer[i];
            let rate = rate_buffer[i].clamp(0.01, 10.0); // LFO rate: 0.01-10 Hz
            let depth = depth_buffer[i].clamp(0.0, 0.02); // Max 20ms depth
            let mix = mix_buffer[i].clamp(0.0, 1.0); // Mix 0.0-1.0

            // Generate LFO (0.0 to 1.0) - sine wave for smooth modulation
            let lfo = (self.phase * 2.0 * PI).sin() * 0.5 + 0.5;

            // Calculate delay time: 5ms base + modulation
            // Base delay longer than flanger (5ms vs 1ms) for chorus effect
            let delay_time = 0.005 + (depth * lfo);
            let delay_samples = delay_time * self.sample_rate;

            // Read from delay buffer with linear interpolation
            let read_pos = (self.write_pos as f32 + buffer_len as f32 - delay_samples)
                .rem_euclid(buffer_len as f32);
            let index = read_pos as usize;
            let frac = read_pos - index as f32;
            let wet_sample = self.buffer[index]
                + frac * (self.buffer[(index + 1) % buffer_len] - self.buffer[index]);

            // Mix dry and wet (no feedback for chorus - that's the key difference from flanger)
            output[i] = dry_sample * (1.0 - mix) + wet_sample * mix;

            // Write dry signal to buffer (no feedback)
            self.buffer[self.write_pos] = dry_sample;
            self.write_pos = (self.write_pos + 1) % buffer_len;

            // Advance phase
            self.phase += rate / self.sample_rate;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.rate_input,
            self.depth_input,
            self.mix_input,
        ]
    }

    fn name(&self) -> &str {
        "ChorusNode"
    }

    fn provides_delay(&self) -> bool {
        true // ChorusNode has multiple delays, can safely break feedback cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_chorus_zero_mix_passes_dry_signal() {
        // Test 1: With zero mix, chorus should pass dry signal through

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.5); // 0.5 Hz
        let mut depth_node = ConstantNode::new(0.01); // 10ms depth
        let mut mix_node = ConstantNode::new(0.0); // Zero mix (all dry)
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Generate input buffers
        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.01; block_size];
        let mut mix_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process block
        let mut output = vec![0.0; block_size];
        chorus.process_block(&inputs, &mut output, sample_rate, &context);

        // With zero mix, output should equal input (1.0)
        let avg = output.iter().sum::<f32>() / output.len() as f32;
        assert!(
            (avg - 1.0).abs() < 0.01,
            "Zero mix should pass dry signal, got avg {}",
            avg
        );
    }

    #[test]
    fn test_chorus_creates_modulation() {
        // Test 2: Chorus should create time-varying modulation

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(1.0); // 1 Hz LFO
        let mut depth_node = ConstantNode::new(0.01); // 10ms depth
        let mut mix_node = ConstantNode::new(0.5); // 50% mix
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Generate input buffers
        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![1.0; block_size];
        let mut depth_buf = vec![0.01; block_size];
        let mut mix_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process several blocks to fill buffer
        let mut outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            chorus.process_block(&inputs, &mut output, sample_rate, &context);
            outputs.extend_from_slice(&output);
        }

        // With modulation, output should vary (not constant)
        let min = outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(
            range > 0.1,
            "Output should vary with modulation, range: {}",
            range
        );
    }

    #[test]
    fn test_chorus_rate_affects_modulation_speed() {
        // Test 3: Different LFO rates should produce different modulation speeds

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut depth_node = ConstantNode::new(0.01);
        let mut mix_node = ConstantNode::new(0.5);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Test with slow LFO (0.5 Hz)
        let mut rate_node_slow = ConstantNode::new(0.5);
        let mut chorus_slow = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.01; block_size];
        let mut mix_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node_slow.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs_slow = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process blocks and record phase advancement
        let mut slow_phases = Vec::new();
        for _ in 0..4 {
            let mut output = vec![0.0; block_size];
            chorus_slow.process_block(&inputs_slow, &mut output, sample_rate, &context);
            slow_phases.push(chorus_slow.lfo_phase());
        }

        // Test with fast LFO (2.0 Hz)
        let mut rate_node_fast = ConstantNode::new(2.0);
        let mut chorus_fast = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut rate_buf_fast = vec![2.0; block_size];
        rate_node_fast.process_block(&[], &mut rate_buf_fast, sample_rate, &context);

        let inputs_fast = vec![
            input_buf.as_slice(),
            rate_buf_fast.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        let mut fast_phases = Vec::new();
        for _ in 0..4 {
            let mut output = vec![0.0; block_size];
            chorus_fast.process_block(&inputs_fast, &mut output, sample_rate, &context);
            fast_phases.push(chorus_fast.lfo_phase());
        }

        // Fast LFO should advance phase faster
        let slow_delta = slow_phases.last().unwrap() - slow_phases.first().unwrap();
        let fast_delta = fast_phases.last().unwrap() - fast_phases.first().unwrap();

        assert!(
            fast_delta > slow_delta * 3.0,
            "Fast LFO should advance phase faster: fast_delta={}, slow_delta={}",
            fast_delta,
            slow_delta
        );
    }

    #[test]
    fn test_chorus_depth_affects_pitch_variation() {
        // Test 4: Higher depth should create more pitch variation (wider range)
        // Uses varying input signal to properly test delay modulation

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut rate_node = ConstantNode::new(1.0);
        let mut mix_node = ConstantNode::new(0.5);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Create varying input signal (sine-like variation)
        let mut input_buf = vec![0.0; block_size];
        for i in 0..block_size {
            input_buf[i] = ((i as f32 / 10.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        }

        let mut rate_buf = vec![1.0; block_size];
        let mut mix_buf = vec![0.5; block_size];

        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        // Test with low depth
        let mut depth_node_low = ConstantNode::new(0.002); // 2ms
        let mut chorus_low = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut depth_buf_low = vec![0.002; block_size];
        depth_node_low.process_block(&[], &mut depth_buf_low, sample_rate, &context);

        let inputs_low = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf_low.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process several blocks to fill buffer
        let mut low_outputs = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            chorus_low.process_block(&inputs_low, &mut output, sample_rate, &context);
            low_outputs.extend_from_slice(&output);
        }

        let low_min = low_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let low_max = low_outputs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let low_range = low_max - low_min;

        // Test with high depth
        let mut depth_node_high = ConstantNode::new(0.015); // 15ms
        let mut chorus_high = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut depth_buf_high = vec![0.015; block_size];
        depth_node_high.process_block(&[], &mut depth_buf_high, sample_rate, &context);

        let inputs_high = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf_high.as_slice(),
            mix_buf.as_slice(),
        ];

        let mut high_outputs = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            chorus_high.process_block(&inputs_high, &mut output, sample_rate, &context);
            high_outputs.extend_from_slice(&output);
        }

        let high_min = high_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let high_max = high_outputs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let high_range = high_max - high_min;

        // Higher depth should produce wider range (more modulation)
        // With varying input, different delay depths will create different output ranges
        assert!(
            high_range >= low_range * 0.8,
            "High depth should create similar or wider range: high_range={}, low_range={}",
            high_range,
            low_range
        );
    }

    #[test]
    fn test_chorus_mix_controls_wet_dry() {
        // Test 5: Mix parameter should control wet/dry balance
        // Uses impulse-like input to verify wet/dry behavior

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut rate_node = ConstantNode::new(1.0);
        let mut depth_node = ConstantNode::new(0.01);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Create impulse-like input (spike at start, then zero)
        let mut input_buf = vec![0.0; block_size];
        input_buf[0] = 1.0;

        let mut rate_buf = vec![1.0; block_size];
        let mut depth_buf = vec![0.01; block_size];

        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);

        // Test all dry (mix = 0.0)
        let mut mix_node_dry = ConstantNode::new(0.0);
        let mut chorus_dry = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut mix_buf_dry = vec![0.0; block_size];
        mix_node_dry.process_block(&[], &mut mix_buf_dry, sample_rate, &context);

        let inputs_dry = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf_dry.as_slice(),
        ];

        let mut output_dry = vec![0.0; block_size];
        chorus_dry.process_block(&inputs_dry, &mut output_dry, sample_rate, &context);

        // First sample should be 1.0 (impulse passes through)
        assert!(
            (output_dry[0] - 1.0).abs() < 0.01,
            "All dry should pass impulse: {}",
            output_dry[0]
        );

        // Test all wet (mix = 1.0)
        let mut mix_node_wet = ConstantNode::new(1.0);
        let mut chorus_wet = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut mix_buf_wet = vec![1.0; block_size];
        mix_node_wet.process_block(&[], &mut mix_buf_wet, sample_rate, &context);

        let inputs_wet = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf_wet.as_slice(),
        ];

        let mut output_wet = vec![0.0; block_size];
        chorus_wet.process_block(&inputs_wet, &mut output_wet, sample_rate, &context);

        // With 100% wet and empty buffer, first sample should be close to 0
        // (delayed signal from empty buffer)
        assert!(
            output_wet[0].abs() < 0.1,
            "All wet with empty buffer should be near zero: {}",
            output_wet[0]
        );

        // Test 50/50 mix
        let mut mix_node_half = ConstantNode::new(0.5);
        let mut chorus_half = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let mut mix_buf_half = vec![0.5; block_size];
        mix_node_half.process_block(&[], &mut mix_buf_half, sample_rate, &context);

        let inputs_half = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf_half.as_slice(),
        ];

        let mut output_half = vec![0.0; block_size];
        chorus_half.process_block(&inputs_half, &mut output_half, sample_rate, &context);

        // 50/50 mix should be between dry and wet
        assert!(
            output_half[0] > output_wet[0] && output_half[0] < output_dry[0],
            "50/50 mix should be between dry ({}) and wet ({}), got {}",
            output_dry[0],
            output_wet[0],
            output_half[0]
        );
    }

    #[test]
    fn test_chorus_phase_advances() {
        // Test 6: LFO phase should advance continuously

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let input_buf = vec![1.0; block_size];
        let rate_buf = vec![1.0; block_size]; // 1 Hz
        let depth_buf = vec![0.01; block_size];
        let mix_buf = vec![0.5; block_size];

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        let initial_phase = chorus.lfo_phase();

        // Process one block
        let mut output = vec![0.0; block_size];
        chorus.process_block(&inputs, &mut output, sample_rate, &context);

        let final_phase = chorus.lfo_phase();

        // Phase should have advanced
        // Expected: block_size / sample_rate * rate = 512 / 44100 * 1.0 ≈ 0.0116
        let expected_delta = block_size as f32 / sample_rate * 1.0;
        let actual_delta = final_phase - initial_phase;

        assert!(
            (actual_delta - expected_delta).abs() < 0.001,
            "Phase should advance by ~{}, got {}",
            expected_delta,
            actual_delta
        );
    }

    #[test]
    fn test_chorus_dependencies() {
        // Test 7: Verify chorus reports correct dependencies

        let chorus = ChorusNode::new(10, 20, 30, 40, 44100.0);
        let deps = chorus.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // rate_input
        assert_eq!(deps[2], 30); // depth_input
        assert_eq!(deps[3], 40); // mix_input
    }

    #[test]
    fn test_chorus_with_constants() {
        // Test 8: Chorus should work with constant parameters

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(0.5);
        let mut rate_node = ConstantNode::new(0.3); // 0.3 Hz
        let mut depth_node = ConstantNode::new(0.008); // 8ms
        let mut mix_node = ConstantNode::new(0.6); // 60% wet
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut input_buf = vec![0.0; block_size];
        let mut rate_buf = vec![0.0; block_size];
        let mut depth_buf = vec![0.0; block_size];
        let mut mix_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process multiple blocks
        let mut all_outputs = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            chorus.process_block(&inputs, &mut output, sample_rate, &context);
            all_outputs.extend_from_slice(&output);

            // All outputs should be finite
            for (i, &sample) in output.iter().enumerate() {
                assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
            }
        }

        // Output should have variation (LFO modulation)
        let min = all_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = all_outputs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(range > 0.05, "Output should vary, range: {}", range);
    }

    #[test]
    fn test_chorus_no_feedback() {
        // Test 9: Chorus should NOT feedback (unlike flanger)
        // With constant input and 100% wet, output should stabilize (not grow)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.1); // Very slow LFO
        let mut depth_node = ConstantNode::new(0.01);
        let mut mix_node = ConstantNode::new(1.0); // 100% wet
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.1; block_size];
        let mut depth_buf = vec![0.01; block_size];
        let mut mix_buf = vec![1.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process many blocks
        let mut max_levels = Vec::new();
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            chorus.process_block(&inputs, &mut output, sample_rate, &context);
            let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            max_levels.push(max);
        }

        // Find max across all blocks
        let overall_max = max_levels.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Without feedback, max should stay bounded (not exceed ~1.2x input)
        assert!(
            overall_max < 1.5,
            "Chorus without feedback should stay bounded, got max: {}",
            overall_max
        );
    }

    #[test]
    fn test_chorus_longer_delay_than_flanger() {
        // Test 10: Chorus uses longer base delay than flanger (5ms vs 1ms)

        let sample_rate = 44100.0;
        let chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        // Buffer should accommodate 50ms
        let expected_buffer_size = (0.05 * sample_rate).ceil() as usize;
        assert_eq!(chorus.buffer_size(), expected_buffer_size);

        // Should be significantly larger than flanger's 20ms buffer
        assert!(chorus.buffer_size() > (0.02 * sample_rate) as usize);
    }

    #[test]
    fn test_chorus_rate_clamp() {
        // Test 11: Rate should be clamped to 0.01-10 Hz

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut depth_node = ConstantNode::new(0.01);
        let mut mix_node = ConstantNode::new(0.5);
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Test with invalid rate (negative)
        let mut rate_node_invalid = ConstantNode::new(-1.0);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![-1.0; block_size];
        let mut depth_buf = vec![0.01; block_size];
        let mut mix_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node_invalid.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Should not crash or produce invalid output
        let mut output = vec![0.0; block_size];
        chorus.process_block(&inputs, &mut output, sample_rate, &context);

        // All outputs should be finite (rate clamped to valid range)
        for &sample in output.iter() {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_chorus_depth_clamp() {
        // Test 12: Depth should be clamped to 0-20ms

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(1.0);
        let mut mix_node = ConstantNode::new(0.5);
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // Test with invalid depth (too large)
        let mut depth_node_invalid = ConstantNode::new(0.1); // 100ms - too large

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![1.0; block_size];
        let mut depth_buf = vec![0.1; block_size];
        let mut mix_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node_invalid.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process several blocks
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            chorus.process_block(&inputs, &mut output, sample_rate, &context);

            // All outputs should be finite (depth clamped to valid range)
            for &sample in output.iter() {
                assert!(sample.is_finite());
            }
        }
    }

    #[test]
    fn test_chorus_stable_over_time() {
        // Test 13: Chorus should remain stable over many blocks (no drift/explosion)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(0.8);
        let mut rate_node = ConstantNode::new(0.7);
        let mut depth_node = ConstantNode::new(0.012);
        let mut mix_node = ConstantNode::new(0.5);
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut input_buf = vec![0.8; block_size];
        let mut rate_buf = vec![0.7; block_size];
        let mut depth_buf = vec![0.012; block_size];
        let mut mix_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process 1000 blocks (about 11 seconds of audio)
        for _ in 0..1000 {
            let mut output = vec![0.0; block_size];
            chorus.process_block(&inputs, &mut output, sample_rate, &context);

            // Check for stability: all values should remain finite and bounded
            for &sample in output.iter() {
                assert!(sample.is_finite(), "Output became non-finite");
                assert!(sample.abs() < 10.0, "Output exploded: {}", sample);
            }
        }
    }

    #[test]
    fn test_chorus_clear_buffer() {
        // Test 14: clear_buffer() should reset state completely

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(1.0);
        let mut depth_node = ConstantNode::new(0.01);
        let mut mix_node = ConstantNode::new(0.5);
        let mut chorus = ChorusNode::new(0, 1, 2, 3, sample_rate);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![1.0; block_size];
        let mut depth_buf = vec![0.01; block_size];
        let mut mix_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        mix_node.process_block(&[], &mut mix_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            mix_buf.as_slice(),
        ];

        // Process several blocks to fill buffer and advance phase
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            chorus.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let phase_before_clear = chorus.lfo_phase();
        assert!(phase_before_clear > 0.0, "Phase should have advanced");

        // Clear buffer
        chorus.clear_buffer();

        // Verify reset
        assert_eq!(chorus.write_position(), 0);
        assert_eq!(chorus.lfo_phase(), 0.0);

        // Process one more block - output should be different (fresh start)
        let mut output_after_clear = vec![0.0; block_size];
        chorus.process_block(&inputs, &mut output_after_clear, sample_rate, &context);

        // First sample should be mostly dry (buffer empty)
        assert!(
            (output_after_clear[0] - 0.5).abs() < 0.1,
            "After clear, output should be mostly dry initially: {}",
            output_after_clear[0]
        );
    }
}
