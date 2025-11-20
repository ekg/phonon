/// Auto Pan node - automatic stereo panning with LFO modulation
///
/// This node applies automatic panning that sweeps the signal across the stereo field
/// using an internal LFO (Low Frequency Oscillator). Unlike manual panning which uses
/// a static position, auto-pan creates movement and space.
///
/// The LFO generates a panning signal that varies from -1.0 (full left) to +1.0 (full right),
/// modulated by the depth parameter. Different waveforms create different panning characters:
/// - **Sine**: Smooth, natural sweeping motion (classic auto-pan)
/// - **Triangle**: Linear movement with direction changes
/// - **Square**: Hard switching between left and right (tremolo-like)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Waveform shapes for the auto-pan LFO
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoPanWaveform {
    /// Smooth sine wave - natural sweeping motion
    Sine,
    /// Triangle wave - linear movement with direction changes
    Triangle,
    /// Square wave - hard left/right switching
    Square,
}

/// Auto-pan node with pattern-controlled rate, depth, and waveform selection
///
/// # Algorithm
/// ```text
/// // Generate LFO (-1 to +1)
/// lfo = waveform_function(phase)
///
/// // Map to pan position
/// pan = lfo * depth  // -depth to +depth range
///
/// // Apply equal-power panning
/// pan_angle = (pan + 1.0) * PI / 4.0  // 0 to PI/2
/// left = input * cos(pan_angle)
/// right = input * sin(pan_angle)
///
/// // Output (mono for now, stereo in future)
/// output = (left + right) / 2.0
/// ```
///
/// # Stereo Output Note
/// Like other spatial nodes, true stereo output requires multi-channel buffer support.
/// For now, outputs mono mix of left+right channels. The panning calculation is
/// correct and ready for stereo when the architecture supports it.
///
/// # Example
/// ```ignore
/// // Smooth auto-pan at 0.5 Hz
/// let input = OscillatorNode::new(0, Waveform::Saw);  // NodeId 0
/// let rate = ConstantNode::new(0.5);                   // NodeId 1
/// let depth = ConstantNode::new(1.0);                  // NodeId 2 (full L-R)
/// let auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);  // NodeId 3
/// ```
pub struct AutoPanNode {
    input: NodeId,           // Mono audio signal
    rate_input: NodeId,      // LFO rate in Hz (can be modulated)
    depth_input: NodeId,     // Modulation depth 0.0-1.0 (can be modulated)
    waveform: AutoPanWaveform, // LFO waveform shape
    phase: f32,              // LFO phase accumulator (0.0 to 1.0)
}

impl AutoPanNode {
    /// Create a new auto-pan node
    ///
    /// # Arguments
    /// * `input` - NodeId providing the mono audio signal
    /// * `rate_input` - NodeId providing LFO rate in Hz (typically 0.01-20 Hz)
    /// * `depth_input` - NodeId providing modulation depth 0.0-1.0 (0=center, 1=full L-R)
    /// * `waveform` - LFO waveform shape (Sine, Triangle, or Square)
    pub fn new(
        input: NodeId,
        rate_input: NodeId,
        depth_input: NodeId,
        waveform: AutoPanWaveform,
    ) -> Self {
        Self {
            input,
            rate_input,
            depth_input,
            waveform,
            phase: 0.0,
        }
    }

    /// Get current LFO phase (0.0 to 1.0)
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Reset LFO phase to 0.0
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    /// Get the waveform type
    pub fn waveform(&self) -> AutoPanWaveform {
        self.waveform
    }

    /// Get input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get rate input node ID
    pub fn rate_input(&self) -> NodeId {
        self.rate_input
    }

    /// Get depth input node ID
    pub fn depth_input(&self) -> NodeId {
        self.depth_input
    }
}

impl AudioNode for AutoPanNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "AutoPanNode requires 3 inputs (input, rate, depth), got {}",
            inputs.len()
        );

        let input_buffer = inputs[0];
        let rate_buffer = inputs[1];
        let depth_buffer = inputs[2];

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

        for i in 0..output.len() {
            let sample = input_buffer[i];
            let rate = rate_buffer[i].clamp(0.01, 20.0); // LFO rate: 0.01-20 Hz
            let depth = depth_buffer[i].clamp(0.0, 1.0); // Depth: 0.0-1.0

            // Generate LFO (-1.0 to +1.0) based on waveform
            let lfo = match self.waveform {
                AutoPanWaveform::Sine => {
                    // Smooth sine wave
                    (self.phase * 2.0 * PI).sin()
                }
                AutoPanWaveform::Triangle => {
                    // Triangle wave: rises 0->1->0->-1->0
                    // phase: 0.0 -> 0.25 -> 0.5 -> 0.75 -> 1.0
                    // out:   0.0 -> 1.0  -> 0.0 -> -1.0 -> 0.0
                    let p = self.phase * 4.0; // Scale to 0-4
                    if p < 1.0 {
                        p // Rising: 0 to 1
                    } else if p < 2.0 {
                        2.0 - p // Falling: 1 to 0
                    } else if p < 3.0 {
                        2.0 - p // Continuing: 0 to -1
                    } else {
                        p - 4.0 // Rising: -1 to 0
                    }
                }
                AutoPanWaveform::Square => {
                    // Square wave: hard left/right switching
                    if self.phase < 0.5 {
                        -1.0
                    } else {
                        1.0
                    }
                }
            };

            // Map LFO to pan position (-depth to +depth)
            let pan = lfo * depth;

            // Apply equal-power panning law
            // pan_angle ranges from 0 (left) to PI/2 (right)
            let pan_angle = (pan + 1.0) * PI / 4.0;

            let left = sample * pan_angle.cos();
            let right = sample * pan_angle.sin();

            // For mono output, return average of left and right
            // TODO: When stereo support is added, output left and right separately
            output[i] = (left + right) / 2.0;

            // Advance phase
            self.phase += rate / sample_rate;

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
        vec![self.input, self.rate_input, self.depth_input]
    }

    fn name(&self) -> &str {
        "AutoPanNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    fn create_test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            44100.0,
        )
    }

    #[test]
    fn test_auto_pan_zero_depth_no_effect() {
        // Test 1: With zero depth, output should be centered (no panning)
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(1.0);
        let mut depth = ConstantNode::new(0.0); // Zero depth
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let context = create_test_context(512);

        let mut input_buf = vec![0.0; 512];
        let mut rate_buf = vec![0.0; 512];
        let mut depth_buf = vec![0.0; 512];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 512];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // With zero depth, pan=0 always (center)
        // Center pan_angle = PI/4, left=right=cos(PI/4), output=(2*cos(PI/4))/2 = cos(PI/4)
        let expected = (PI / 4.0).cos();

        for (i, &sample) in output.iter().enumerate() {
            assert!(
                (sample - expected).abs() < 0.001,
                "Sample {} should be centered: got {}, expected {}",
                i,
                sample,
                expected
            );
        }
    }

    #[test]
    fn test_auto_pan_full_depth_modulation() {
        // Test 2: Full depth should create panning from left to right
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(1.0); // 1 Hz
        let mut depth = ConstantNode::new(1.0); // Full depth
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let context = create_test_context(44100); // 1 second

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should vary (panning modulation)
        let min = output.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(
            range > 0.2,
            "Full depth should create significant variation: range={}",
            range
        );
    }

    #[test]
    fn test_auto_pan_rate_affects_speed() {
        // Test 3: Different rates should produce different panning speeds
        let context = create_test_context(44100); // 1 second

        // Test with slow rate (0.5 Hz)
        let mut input_slow = ConstantNode::new(1.0);
        let mut rate_slow = ConstantNode::new(0.5);
        let mut depth_slow = ConstantNode::new(1.0);
        let mut auto_pan_slow = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input_slow.process_block(&[], &mut input_buf, 44100.0, &context);
        rate_slow.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth_slow.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs_slow = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output_slow = vec![0.0; 44100];
        auto_pan_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

        // Test with fast rate (4.0 Hz)
        let mut input_fast = ConstantNode::new(1.0);
        let mut rate_fast = ConstantNode::new(4.0);
        let mut depth_fast = ConstantNode::new(1.0);
        let mut auto_pan_fast = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let mut rate_buf_fast = vec![0.0; 44100];
        rate_fast.process_block(&[], &mut rate_buf_fast, 44100.0, &context);

        let inputs_fast = vec![
            input_buf.as_slice(),
            rate_buf_fast.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output_fast = vec![0.0; 44100];
        auto_pan_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);

        // Count zero crossings (how many times signal crosses a threshold)
        let threshold = 0.5;
        let crossings_slow = count_crossings(&output_slow, threshold);
        let crossings_fast = count_crossings(&output_fast, threshold);

        // Fast rate should have ~8x more crossings than slow rate (4.0 / 0.5 = 8)
        assert!(
            crossings_fast > crossings_slow * 6,
            "Fast rate should have more crossings: slow={}, fast={}",
            crossings_slow,
            crossings_fast
        );
    }

    #[test]
    fn test_auto_pan_sine_waveform_smooth() {
        // Test 4: Sine waveform should produce smooth, continuous panning
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(1.0);
        let mut depth = ConstantNode::new(1.0);
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        assert_eq!(auto_pan.waveform(), AutoPanWaveform::Sine);

        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Check smoothness: calculate maximum sample-to-sample difference
        let mut max_delta = 0.0_f32;
        for i in 1..output.len() {
            let delta = (output[i] - output[i - 1]).abs();
            max_delta = max_delta.max(delta);
        }

        // Sine should be very smooth (small deltas)
        assert!(
            max_delta < 0.01,
            "Sine waveform should be smooth, max_delta={}",
            max_delta
        );
    }

    #[test]
    fn test_auto_pan_triangle_waveform_linear() {
        // Test 5: Triangle waveform should produce linear ramps
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(0.1); // Very slow to observe linearity
        let mut depth = ConstantNode::new(1.0);
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Triangle);

        assert_eq!(auto_pan.waveform(), AutoPanWaveform::Triangle);

        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Triangle should have more variation in deltas than sine (due to direction changes)
        // But should still be relatively smooth
        let mut max_delta = 0.0_f32;
        for i in 1..output.len() {
            let delta = (output[i] - output[i - 1]).abs();
            max_delta = max_delta.max(delta);
        }

        // Should be reasonably smooth
        assert!(
            max_delta < 0.02,
            "Triangle waveform should be reasonably smooth, max_delta={}",
            max_delta
        );
    }

    #[test]
    fn test_auto_pan_square_waveform_hard_switching() {
        // Test 6: Square waveform should produce hard left/right switching
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(1.0);
        let mut depth = ConstantNode::new(1.0);
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Square);

        assert_eq!(auto_pan.waveform(), AutoPanWaveform::Square);

        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Square wave should have two distinct levels (hard left and hard right)
        // Find the two most common value ranges
        let mut value_counts = std::collections::HashMap::new();
        for &sample in &output {
            let bucket = (sample * 100.0).round() as i32; // Round to 0.01 precision
            *value_counts.entry(bucket).or_insert(0) += 1;
        }

        // Should have at most a few distinct values (quantized by square wave)
        assert!(
            value_counts.len() < 20,
            "Square wave should have few distinct values, got {}",
            value_counts.len()
        );
    }

    #[test]
    fn test_auto_pan_depth_controls_width() {
        // Test 7: Higher depth should create wider panning range
        let context = create_test_context(44100);

        // Test with low depth (0.3)
        let mut input_low = ConstantNode::new(1.0);
        let mut rate_low = ConstantNode::new(1.0);
        let mut depth_low = ConstantNode::new(0.3);
        let mut auto_pan_low = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input_low.process_block(&[], &mut input_buf, 44100.0, &context);
        rate_low.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth_low.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs_low = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output_low = vec![0.0; 44100];
        auto_pan_low.process_block(&inputs_low, &mut output_low, 44100.0, &context);

        let low_min = output_low.iter().cloned().fold(f32::INFINITY, f32::min);
        let low_max = output_low.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let low_range = low_max - low_min;

        // Test with high depth (1.0)
        let mut input_high = ConstantNode::new(1.0);
        let mut rate_high = ConstantNode::new(1.0);
        let mut depth_high = ConstantNode::new(1.0);
        let mut auto_pan_high = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let mut depth_buf_high = vec![0.0; 44100];
        depth_high.process_block(&[], &mut depth_buf_high, 44100.0, &context);

        let inputs_high = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf_high.as_slice(),
        ];
        let mut output_high = vec![0.0; 44100];
        auto_pan_high.process_block(&inputs_high, &mut output_high, 44100.0, &context);

        let high_min = output_high.iter().cloned().fold(f32::INFINITY, f32::min);
        let high_max = output_high.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let high_range = high_max - high_min;

        // Higher depth should produce wider range
        assert!(
            high_range > low_range * 1.5,
            "High depth should have wider range: low={}, high={}",
            low_range,
            high_range
        );
    }

    #[test]
    fn test_auto_pan_equal_power_panning() {
        // Test 8: Verify equal-power panning law (L² + R² = 1)
        // This tests the underlying panning algorithm, not the mono output
        let pan_positions = vec![-1.0, -0.5, 0.0, 0.5, 1.0];

        for &pan_pos in &pan_positions {
            let pan_angle = (pan_pos + 1.0) * PI / 4.0;
            let left_gain = pan_angle.cos();
            let right_gain = pan_angle.sin();

            let stereo_power = left_gain * left_gain + right_gain * right_gain;

            assert!(
                (stereo_power - 1.0).abs() < 0.001,
                "Pan position {} should have equal power: {:.6} (expected 1.0)",
                pan_pos,
                stereo_power
            );
        }
    }

    #[test]
    fn test_auto_pan_phase_advances() {
        // Test 9: Phase should advance correctly based on rate
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);
        assert_eq!(auto_pan.phase(), 0.0);

        let context = create_test_context(512);

        let input_buf = vec![1.0; 512];
        let rate_buf = vec![2.0; 512]; // 2 Hz
        let depth_buf = vec![1.0; 512];

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 512];

        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Expected phase advancement: 512 samples * (2 Hz / 44100 Hz)
        let expected_phase = (512.0 * 2.0) / 44100.0;
        let actual_phase = auto_pan.phase();

        assert!(
            (actual_phase - expected_phase).abs() < 0.001,
            "Phase advancement mismatch: expected {}, got {}",
            expected_phase,
            actual_phase
        );
    }

    #[test]
    fn test_auto_pan_reset_phase() {
        // Test 10: reset_phase() should reset to 0.0
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        // Manually advance phase
        auto_pan.phase = 0.75;
        assert_eq!(auto_pan.phase(), 0.75);

        // Reset
        auto_pan.reset_phase();
        assert_eq!(auto_pan.phase(), 0.0);
    }

    #[test]
    fn test_auto_pan_dependencies() {
        // Test 11: Verify correct dependency reporting
        let auto_pan = AutoPanNode::new(10, 20, 30, AutoPanWaveform::Sine);
        let deps = auto_pan.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // rate_input
        assert_eq!(deps[2], 30); // depth_input
    }

    #[test]
    fn test_auto_pan_pattern_modulated_rate() {
        // Test 12: Rate can be pattern-modulated (varying over time)
        let mut input = ConstantNode::new(1.0);
        let mut depth = ConstantNode::new(1.0);
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        // Create varying rate buffer (0.5 Hz -> 4.0 Hz over 1 second)
        let mut rate_buf = vec![0.0; 44100];
        for i in 0..44100 {
            rate_buf[i] = 0.5 + (i as f32 / 44100.0) * 3.5; // Linear ramp
        }

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should vary (panning with changing rate)
        let min = output.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(
            range > 0.2,
            "Pattern-modulated rate should create variation: range={}",
            range
        );
    }

    #[test]
    fn test_auto_pan_pattern_modulated_depth() {
        // Test 13: Depth can be pattern-modulated (varying over time)
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(2.0);
        let mut auto_pan = AutoPanNode::new(0, 1, 2, AutoPanWaveform::Sine);

        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);

        // Create varying depth buffer (0.0 -> 1.0 over 1 second)
        let mut depth_buf = vec![0.0; 44100];
        for i in 0..44100 {
            depth_buf[i] = i as f32 / 44100.0; // Linear ramp
        }

        let inputs = vec![input_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Early samples (low depth) should have less variation than late samples (high depth)
        let early_range = calc_range(&output[0..1000]);
        let late_range = calc_range(&output[43100..44100]);

        assert!(
            late_range > early_range * 2.0,
            "Pattern-modulated depth should increase variation: early={}, late={}",
            early_range,
            late_range
        );
    }

    #[test]
    fn test_auto_pan_with_audio_signal() {
        // Test 14: Auto-pan works with real audio (oscillator)
        let mut freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut rate = ConstantNode::new(0.5);
        let mut depth = ConstantNode::new(1.0);
        let mut auto_pan = AutoPanNode::new(1, 2, 3, AutoPanWaveform::Sine);

        let context = create_test_context(44100);

        // Generate frequency buffer
        let mut freq_buf = vec![0.0; 44100];
        freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Generate audio signal
        let freq_inputs = vec![freq_buf.as_slice()];
        let mut audio_buf = vec![0.0; 44100];
        osc.process_block(&freq_inputs, &mut audio_buf, 44100.0, &context);

        // Generate rate and depth buffers
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        // Apply auto-pan
        let inputs = vec![audio_buf.as_slice(), rate_buf.as_slice(), depth_buf.as_slice()];
        let mut output = vec![0.0; 44100];
        auto_pan.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should have audio energy (not silent)
        let rms = calc_rms(&output);
        assert!(rms > 0.01, "Auto-panned audio should have energy: rms={}", rms);

        // Output should vary (panning modulation)
        let range = calc_range(&output);
        assert!(
            range > 0.5,
            "Auto-panned audio should have variation: range={}",
            range
        );
    }

    // Helper functions

    fn count_crossings(buffer: &[f32], threshold: f32) -> usize {
        let mut count = 0;
        let mut prev_above = buffer[0] > threshold;

        for &sample in buffer.iter().skip(1) {
            let curr_above = sample > threshold;
            if curr_above != prev_above {
                count += 1;
                prev_above = curr_above;
            }
        }

        count
    }

    fn calc_range(buffer: &[f32]) -> f32 {
        let min = buffer.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = buffer.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        max - min
    }

    fn calc_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }
}
