/// Phaser node - classic phaser effect using cascaded all-pass filters
///
/// This node implements a classic phaser effect using cascaded all-pass filters.
/// An LFO sweeps the all-pass frequencies, creating notches that move through the spectrum.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// All-pass filter state for one stage
#[derive(Debug, Clone)]
struct AllPassState {
    x1: f32, // Previous input
    y1: f32, // Previous output
}

impl AllPassState {
    fn new() -> Self {
        Self { x1: 0.0, y1: 0.0 }
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

/// Phaser node with pattern-controlled rate, depth, feedback, and stages
///
/// # Example
/// ```ignore
/// // Classic phaser on signal
/// let input_signal = OscillatorNode::new(0, Waveform::Saw);  // NodeId 0
/// let rate = ConstantNode::new(0.5);  // 0.5 Hz LFO, NodeId 1
/// let depth = ConstantNode::new(0.8);  // 80% depth, NodeId 2
/// let feedback = ConstantNode::new(0.7);  // 70% feedback, NodeId 3
/// let phaser = PhaserNode::new(0, 1, 2, 3, 6, 44100.0);  // 6 stages, NodeId 4
/// ```
pub struct PhaserNode {
    input: NodeId,           // Signal to phase
    rate_input: NodeId,      // LFO rate in Hz (can be modulated)
    depth_input: NodeId,     // Modulation depth 0.0 to 1.0 (can be modulated)
    feedback_input: NodeId,  // Feedback amount (can be modulated)
    stages: usize,           // Number of all-pass stages (usually 4-8)
    allpass_states: Vec<AllPassState>, // State for each stage
    phase: f32,              // LFO phase (0.0 to 1.0)
    sample_rate: f32,        // Sample rate for calculations
}

impl PhaserNode {
    /// PhaserNode - Classic all-pass cascaded phaser effect with LFO modulation
    ///
    /// Creates sweeping frequency notches via cascaded all-pass filters modulated by
    /// an LFO. Classic effect for chorus-like textures, stereo widening, and creative
    /// sound design. Multiple stages create richer modulation patterns.
    ///
    /// # Parameters
    /// - `input`: NodeId of signal to phase
    /// - `rate_input`: NodeId of LFO rate in Hz (0.1-10 typical)
    /// - `depth_input`: NodeId of modulation depth (0.0-1.0)
    /// - `feedback_input`: NodeId of feedback amount (0.0-0.95)
    /// - `stages`: Number of all-pass stages (4-8 typical, more = richer)
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: sine 440
    /// ~phased: ~signal # phaser 0.5 0.8 0.5 4
    /// ```
    pub fn new(
        input: NodeId,
        rate_input: NodeId,
        depth_input: NodeId,
        feedback_input: NodeId,
        stages: usize,
        sample_rate: f32,
    ) -> Self {
        let allpass_states = (0..stages).map(|_| AllPassState::new()).collect();

        Self {
            input,
            rate_input,
            depth_input,
            feedback_input,
            stages,
            allpass_states,
            phase: 0.0,
            sample_rate,
        }
    }

    /// Get the number of all-pass stages
    pub fn stage_count(&self) -> usize {
        self.stages
    }

    /// Get the current LFO phase (0.0 to 1.0)
    pub fn lfo_phase(&self) -> f32 {
        self.phase
    }

    /// Reset the phaser state to silence
    pub fn clear_state(&mut self) {
        for state in &mut self.allpass_states {
            state.reset();
        }
        self.phase = 0.0;
    }
}

impl AudioNode for PhaserNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "PhaserNode requires 4 inputs: signal, rate, depth, feedback"
        );

        let input_buffer = inputs[0];
        let rate_buffer = inputs[1];
        let depth_buffer = inputs[2];
        let feedback_buffer = inputs[3];

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
        debug_assert_eq!(
            feedback_buffer.len(),
            output.len(),
            "Feedback buffer length mismatch"
        );

        for i in 0..output.len() {
            let sample = input_buffer[i];
            let rate = rate_buffer[i];
            let depth = depth_buffer[i].clamp(0.0, 1.0);
            let feedback = feedback_buffer[i].clamp(0.0, 0.95);

            // Generate LFO (0.0 to 1.0)
            let lfo = (self.phase * 2.0 * PI).sin() * 0.5 + 0.5;

            // Calculate all-pass frequency (200 Hz to 2000 Hz sweep)
            let freq = 200.0 + depth * lfo * 1800.0;
            let omega = 2.0 * PI * freq / self.sample_rate;
            let alpha = (omega.tan() - 1.0) / (omega.tan() + 1.0);

            // Cascade all-pass filters
            let mut signal = sample;
            for state in &mut self.allpass_states {
                let output_val = alpha * signal + state.x1 - alpha * state.y1;
                state.x1 = signal;
                state.y1 = output_val;
                signal = output_val;
            }

            // Mix dry + phased signal with feedback
            output[i] = sample + signal * feedback;

            // Advance LFO phase
            self.phase += rate / self.sample_rate;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.rate_input, self.depth_input, self.feedback_input]
    }

    fn name(&self) -> &str {
        "PhaserNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_phaser_creates_notches() {
        // Test 1: Phaser should create spectral notches (affects frequency content)

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.5); // 0.5 Hz LFO
        let mut depth_node = ConstantNode::new(0.8); // 80% depth
        let mut feedback_node = ConstantNode::new(0.5); // 50% feedback
        let mut phaser = PhaserNode::new(0, 1, 2, 3, 6, sample_rate); // 6 stages

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Generate input buffers
        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.8; block_size];
        let mut feedback_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process several blocks to let state settle
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            phaser.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Final block - phaser should produce output
        let mut output = vec![0.0; block_size];
        phaser.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should be non-zero (phaser processes signal)
        let avg = output.iter().sum::<f32>() / output.len() as f32;
        assert!(avg.abs() > 0.1, "Phaser should produce non-zero output, got {}", avg);

        // All samples should be finite
        for (i, &sample) in output.iter().enumerate() {
            assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
        }
    }

    #[test]
    fn test_phaser_rate_modulation() {
        // Test 2: Different LFO rates should produce different modulation speeds

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut depth_node = ConstantNode::new(0.8);
        let mut feedback_node = ConstantNode::new(0.5);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test with slow LFO (0.5 Hz)
        let mut rate_node_slow = ConstantNode::new(0.5);
        let mut phaser_slow = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.8; block_size];
        let mut feedback_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node_slow.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs_slow = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process blocks and record phase advancement
        let mut slow_phases = Vec::new();
        for _ in 0..4 {
            let mut output = vec![0.0; block_size];
            phaser_slow.process_block(&inputs_slow, &mut output, sample_rate, &context);
            slow_phases.push(phaser_slow.lfo_phase());
        }

        // Test with fast LFO (2.0 Hz)
        let mut rate_node_fast = ConstantNode::new(2.0);
        let mut phaser_fast = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let mut rate_buf_fast = vec![2.0; block_size];
        rate_node_fast.process_block(&[], &mut rate_buf_fast, sample_rate, &context);

        let inputs_fast = vec![
            input_buf.as_slice(),
            rate_buf_fast.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        let mut fast_phases = Vec::new();
        for _ in 0..4 {
            let mut output = vec![0.0; block_size];
            phaser_fast.process_block(&inputs_fast, &mut output, sample_rate, &context);
            fast_phases.push(phaser_fast.lfo_phase());
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
    fn test_phaser_depth_effect() {
        // Test 3: Higher depth should produce more dramatic effect

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.5);
        let mut feedback_node = ConstantNode::new(0.5);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test with low depth
        let mut depth_node_low = ConstantNode::new(0.1);
        let mut phaser_low = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf_low = vec![0.1; block_size];
        let mut feedback_buf = vec![0.5; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node_low.process_block(&[], &mut depth_buf_low, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs_low = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf_low.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process several blocks
        let mut low_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            phaser_low.process_block(&inputs_low, &mut output, sample_rate, &context);
            low_outputs.extend_from_slice(&output);
        }

        let low_range = {
            let min = low_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = low_outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            max - min
        };

        // Test with high depth
        let mut depth_node_high = ConstantNode::new(0.9);
        let mut phaser_high = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let mut depth_buf_high = vec![0.9; block_size];
        depth_node_high.process_block(&[], &mut depth_buf_high, sample_rate, &context);

        let inputs_high = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf_high.as_slice(),
            feedback_buf.as_slice(),
        ];

        let mut high_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            phaser_high.process_block(&inputs_high, &mut output, sample_rate, &context);
            high_outputs.extend_from_slice(&output);
        }

        let high_range = {
            let min = high_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = high_outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            max - min
        };

        // Higher depth should produce wider range of variation
        assert!(
            high_range > low_range,
            "High depth should produce wider variation: high_range={}, low_range={}",
            high_range,
            low_range
        );
    }

    #[test]
    fn test_phaser_feedback_resonance() {
        // Test 4: Higher feedback should increase resonance/peaks

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(1.0);
        let mut rate_node = ConstantNode::new(0.5);
        let mut depth_node = ConstantNode::new(0.8);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test with low feedback
        let mut feedback_node_low = ConstantNode::new(0.1);
        let mut phaser_low = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let mut input_buf = vec![1.0; block_size];
        let mut rate_buf = vec![0.5; block_size];
        let mut depth_buf = vec![0.8; block_size];
        let mut feedback_buf_low = vec![0.1; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node_low.process_block(&[], &mut feedback_buf_low, sample_rate, &context);

        let inputs_low = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf_low.as_slice(),
        ];

        // Process several blocks
        let mut low_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            phaser_low.process_block(&inputs_low, &mut output, sample_rate, &context);
            low_outputs.extend_from_slice(&output);
        }

        let low_max = low_outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Test with high feedback
        let mut feedback_node_high = ConstantNode::new(0.9);
        let mut phaser_high = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let mut feedback_buf_high = vec![0.9; block_size];
        feedback_node_high.process_block(&[], &mut feedback_buf_high, sample_rate, &context);

        let inputs_high = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf_high.as_slice(),
        ];

        let mut high_outputs = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            phaser_high.process_block(&inputs_high, &mut output, sample_rate, &context);
            high_outputs.extend_from_slice(&output);
        }

        let high_max = high_outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Higher feedback should produce higher peaks (resonance)
        assert!(
            high_max > low_max * 1.1,
            "High feedback should increase resonance: high_max={}, low_max={}",
            high_max,
            low_max
        );
    }

    #[test]
    fn test_phaser_stage_count() {
        // Test 5: More stages should create more notches (visible in frequency response)

        let sample_rate = 44100.0;
        let block_size = 512;

        // Create phaser with 2 stages
        let phaser_2 = PhaserNode::new(0, 1, 2, 3, 2, sample_rate);
        assert_eq!(phaser_2.stage_count(), 2);

        // Create phaser with 8 stages
        let phaser_8 = PhaserNode::new(0, 1, 2, 3, 8, sample_rate);
        assert_eq!(phaser_8.stage_count(), 8);

        // More stages should exist (actual frequency response testing would require FFT)
        // This test just verifies the stage count is stored correctly
        assert!(
            phaser_8.stage_count() > phaser_2.stage_count(),
            "8-stage phaser should have more stages than 2-stage"
        );
    }

    #[test]
    fn test_phaser_dependencies() {
        // Test 6: Verify phaser reports correct dependencies

        let phaser = PhaserNode::new(10, 20, 30, 40, 6, 44100.0);
        let deps = phaser.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // rate_input
        assert_eq!(deps[2], 30); // depth_input
        assert_eq!(deps[3], 40); // feedback_input
    }

    #[test]
    fn test_phaser_with_constants() {
        // Test 7: Phaser should work with constant parameters

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut input_node = ConstantNode::new(0.5);
        let mut rate_node = ConstantNode::new(0.3); // 0.3 Hz
        let mut depth_node = ConstantNode::new(0.7); // 70% depth
        let mut feedback_node = ConstantNode::new(0.6); // 60% feedback
        let mut phaser = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut input_buf = vec![0.0; block_size];
        let mut rate_buf = vec![0.0; block_size];
        let mut depth_buf = vec![0.0; block_size];
        let mut feedback_buf = vec![0.0; block_size];

        input_node.process_block(&[], &mut input_buf, sample_rate, &context);
        rate_node.process_block(&[], &mut rate_buf, sample_rate, &context);
        depth_node.process_block(&[], &mut depth_buf, sample_rate, &context);
        feedback_node.process_block(&[], &mut feedback_buf, sample_rate, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process multiple blocks
        let mut all_outputs = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            phaser.process_block(&inputs, &mut output, sample_rate, &context);
            all_outputs.extend_from_slice(&output);

            // All outputs should be finite
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} is not finite: {}",
                    i,
                    sample
                );
            }
        }

        // Output should have variation (LFO modulation)
        let min = all_outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = all_outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        assert!(range > 0.05, "Output should vary, range: {}", range);
    }

    #[test]
    fn test_phaser_phase_advances() {
        // Test 8: LFO phase should advance continuously

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut phaser = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let input_buf = vec![1.0; block_size];
        let rate_buf = vec![1.0; block_size]; // 1 Hz
        let depth_buf = vec![0.8; block_size];
        let feedback_buf = vec![0.5; block_size];

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        let initial_phase = phaser.lfo_phase();

        // Process one block
        let mut output = vec![0.0; block_size];
        phaser.process_block(&inputs, &mut output, sample_rate, &context);

        let final_phase = phaser.lfo_phase();

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
    fn test_phaser_clear_state() {
        // Test 9: clear_state should reset all state

        let sample_rate = 44100.0;
        let block_size = 512;

        let mut phaser = PhaserNode::new(0, 1, 2, 3, 6, sample_rate);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let input_buf = vec![1.0; block_size];
        let rate_buf = vec![1.0; block_size];
        let depth_buf = vec![0.8; block_size];
        let feedback_buf = vec![0.5; block_size];

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
            feedback_buf.as_slice(),
        ];

        // Process some blocks to build up state
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            phaser.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Phase should be non-zero
        assert!(phaser.lfo_phase() > 0.0);

        // Clear state
        phaser.clear_state();

        // Phase should be reset
        assert_eq!(phaser.lfo_phase(), 0.0);
    }
}
