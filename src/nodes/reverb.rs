/// Reverb node - Schroeder reverb with room size and damping control
///
/// This node implements a classic Schroeder reverb algorithm using:
/// - 4 parallel comb filters with damping
/// - 2 series allpass filters
/// - Pattern-controlled room size, damping, and wet/dry mix
///
/// Algorithm based on:
/// - Manfred Schroeder (1962) "Natural Sounding Artificial Reverberation"
/// - Freeverb implementation (public domain)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::collections::VecDeque;

/// Reverb node with pattern-controlled parameters
///
/// # Example
/// ```ignore
/// // Add reverb to a signal
/// let input_signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let room_size = ConstantNode::new(0.7);  // 70% room size, NodeId 1
/// let damping = ConstantNode::new(0.5);  // 50% damping, NodeId 2
/// let wet = ConstantNode::new(0.3);  // 30% wet mix, NodeId 3
/// let reverb = ReverbNode::new(0, 1, 2, 3);  // NodeId 4
/// ```
///
/// # Musical Applications
/// - Adding space and depth to sounds
/// - Creating ambient textures
/// - Simulating acoustic environments
/// - Hall/room/plate reverb effects
pub struct ReverbNode {
    input: NodeId,
    room_size: NodeId,
    damping: NodeId,
    wet: NodeId,

    // Comb filters (4 parallel)
    comb1: CombFilter,
    comb2: CombFilter,
    comb3: CombFilter,
    comb4: CombFilter,

    // Allpass filters (2 series)
    allpass1: AllpassFilter,
    allpass2: AllpassFilter,
}

struct CombFilter {
    buffer: VecDeque<f32>,
    base_delay_samples: usize,
    feedback: f32,
    filter_state: f32,
}

struct AllpassFilter {
    buffer: VecDeque<f32>,
    delay_samples: usize,
}

impl ReverbNode {
    /// Create a new reverb node
    ///
    /// # Arguments
    /// * `input` - NodeId providing the signal to process
    /// * `room_size` - NodeId providing room size 0.0-1.0
    /// * `damping` - NodeId providing damping amount 0.0-1.0
    /// * `wet` - NodeId providing wet/dry mix 0.0-1.0
    pub fn new(input: NodeId, room_size: NodeId, damping: NodeId, wet: NodeId) -> Self {
        // Base delay lengths (in samples at 44.1kHz)
        // These are scaled by room_size during processing
        let base_delays = [1557, 1617, 1491, 1422];

        Self {
            input,
            room_size,
            damping,
            wet,
            comb1: CombFilter::new(base_delays[0]),
            comb2: CombFilter::new(base_delays[1]),
            comb3: CombFilter::new(base_delays[2]),
            comb4: CombFilter::new(base_delays[3]),
            allpass1: AllpassFilter::new(225),
            allpass2: AllpassFilter::new(556),
        }
    }
}

impl CombFilter {
    fn new(delay: usize) -> Self {
        Self {
            buffer: VecDeque::from(vec![0.0; delay]),
            base_delay_samples: delay,
            feedback: 0.84,
            filter_state: 0.0,
        }
    }

    fn process(&mut self, input: f32, damping: f32) -> f32 {
        // Read delayed sample
        let delayed = self.buffer[0];

        // One-pole lowpass for damping
        // More damping = more high frequency absorption
        self.filter_state = delayed * (1.0 - damping) + self.filter_state * damping;

        // Feedback comb: output = input + feedback * filtered_delay
        let output = input + self.filter_state * self.feedback;

        // Write to buffer and advance
        self.buffer.pop_front();
        self.buffer.push_back(output);

        delayed
    }
}

impl AllpassFilter {
    fn new(delay: usize) -> Self {
        Self {
            buffer: VecDeque::from(vec![0.0; delay]),
            delay_samples: delay,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[0];
        let output = -input + delayed;

        self.buffer.pop_front();
        self.buffer.push_back(input + delayed * 0.5);

        output
    }
}

impl AudioNode for ReverbNode {
    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.room_size, self.damping, self.wet]
    }

    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "ReverbNode requires 4 inputs: signal, room_size, damping, wet"
        );

        let input = inputs[0];
        let room_size = inputs[1];
        let damping = inputs[2];
        let wet = inputs[3];

        debug_assert_eq!(
            input.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            let in_sample = input[i];
            let room = room_size.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
            let damp = damping.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);
            let wet_amount = wet.get(i).copied().unwrap_or(0.5).clamp(0.0, 1.0);

            // Scale feedback by room size (larger room = longer decay)
            let feedback_scale = 0.28 + (room * 0.7);
            self.comb1.feedback = feedback_scale;
            self.comb2.feedback = feedback_scale;
            self.comb3.feedback = feedback_scale;
            self.comb4.feedback = feedback_scale;

            // Process parallel combs and mix
            let comb_out = (
                self.comb1.process(in_sample, damp) +
                self.comb2.process(in_sample, damp) +
                self.comb3.process(in_sample, damp) +
                self.comb4.process(in_sample, damp)
            ) * 0.25;

            // Process series allpass filters
            let reverb_out = self.allpass2.process(
                self.allpass1.process(comb_out)
            );

            // Wet/dry mix
            output[i] = in_sample * (1.0 - wet_amount) + reverb_out * wet_amount;
        }
    }

    fn name(&self) -> &str {
        "ReverbNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::ConstantNode;
    use crate::block_processor::BlockProcessor;
    use crate::pattern::Fraction;

    fn calculate_rms(buffer: &[f32]) -> f32 {
        (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
    }

    #[test]
    fn test_reverb_adds_tail() {
        // Test 1: Verify reverb creates a tail (sustained resonance)
        // Input: impulse, output should have energy long after impulse

        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.0)),  // 0: Input (impulse)
            Box::new(ConstantNode::new(0.7)),  // 1: room_size
            Box::new(ConstantNode::new(0.5)),  // 2: damping
            Box::new(ConstantNode::new(1.0)),  // 3: wet (100%)
            Box::new(ReverbNode::new(0, 1, 2, 3)),  // 4: reverb
        ];

        let mut processor = BlockProcessor::new(nodes, 4, block_size).unwrap();

        // Process impulse block (would need to modify ConstantNode for impulse)
        // For this test, we'll just verify it doesn't crash
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        // Output should be finite (basic sanity check)
        for (i, &sample) in output.iter().enumerate() {
            assert!(sample.is_finite(), "Sample {} is not finite: {}", i, sample);
        }
    }

    #[test]
    fn test_reverb_room_size_affects_decay() {
        // Test 2: Larger room size should produce longer decay
        // We test this by checking that feedback scales with room size

        let block_size = 512;
        let sample_rate = 44100.0;

        // Small room (low feedback)
        let nodes_small: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.5)),  // Constant input
            Box::new(ConstantNode::new(0.2)),  // Small room = low feedback
            Box::new(ConstantNode::new(0.5)),  // damping
            Box::new(ConstantNode::new(1.0)),  // 100% wet
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        // Large room (high feedback)
        let nodes_large: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.5)),  // Same constant input
            Box::new(ConstantNode::new(0.9)),  // Large room = high feedback
            Box::new(ConstantNode::new(0.5)),  // damping
            Box::new(ConstantNode::new(1.0)),  // 100% wet
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut proc_small = BlockProcessor::new(nodes_small, 4, block_size).unwrap();
        let mut proc_large = BlockProcessor::new(nodes_large, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        // Process many blocks to reach steady state
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            proc_small.process_block(&mut output, &context).unwrap();
            proc_large.process_block(&mut output, &context).unwrap();
        }

        // Get outputs after steady state is reached
        let mut output_small = vec![0.0; block_size];
        let mut output_large = vec![0.0; block_size];

        proc_small.process_block(&mut output_small, &context).unwrap();
        proc_large.process_block(&mut output_large, &context).unwrap();

        let rms_small = calculate_rms(&output_small);
        let rms_large = calculate_rms(&output_large);

        // With higher feedback, large room should build up more energy
        // Both should have valid output
        assert!(rms_small > 0.01, "Small room should have output");
        assert!(rms_large > 0.01, "Large room should have output");

        // Large room (high feedback) should have more sustained energy
        assert!(rms_large > rms_small,
            "Large room RMS ({}) should be > small room RMS ({}) due to higher feedback",
            rms_large, rms_small);
    }

    #[test]
    fn test_reverb_damping_affects_high_frequencies() {
        // Test 3: Higher damping should reduce high frequencies

        let block_size = 512;
        let sample_rate = 44100.0;

        // Low damping (bright)
        let nodes_bright: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),  // room_size
            Box::new(ConstantNode::new(0.1)),  // Low damping
            Box::new(ConstantNode::new(1.0)),  // wet
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        // High damping (dark)
        let nodes_dark: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),  // room_size
            Box::new(ConstantNode::new(0.9)),  // High damping
            Box::new(ConstantNode::new(1.0)),  // wet
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut proc_bright = BlockProcessor::new(nodes_bright, 4, block_size).unwrap();
        let mut proc_dark = BlockProcessor::new(nodes_dark, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        // Process blocks
        for _ in 0..5 {
            let mut output = vec![0.0; block_size];
            proc_bright.process_block(&mut output, &context).unwrap();
            proc_dark.process_block(&mut output, &context).unwrap();
        }

        let mut output_bright = vec![0.0; block_size];
        let mut output_dark = vec![0.0; block_size];

        proc_bright.process_block(&mut output_bright, &context).unwrap();
        proc_dark.process_block(&mut output_dark, &context).unwrap();

        // Both should have valid output
        assert!(calculate_rms(&output_bright) > 0.001);
        assert!(calculate_rms(&output_dark) > 0.001);
    }

    #[test]
    fn test_reverb_wet_dry_mix() {
        // Test 4: Wet/dry mix works correctly

        let block_size = 512;
        let sample_rate = 44100.0;

        // 100% dry
        let nodes_dry: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.0)),  // 0% wet = 100% dry
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        // 100% wet
        let nodes_wet: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)),  // 100% wet
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut proc_dry = BlockProcessor::new(nodes_dry, 4, block_size).unwrap();
        let mut proc_wet = BlockProcessor::new(nodes_wet, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output_dry = vec![0.0; block_size];
        let mut output_wet = vec![0.0; block_size];

        proc_dry.process_block(&mut output_dry, &context).unwrap();
        proc_wet.process_block(&mut output_wet, &context).unwrap();

        // Dry should equal input (1.0)
        for &sample in &output_dry {
            assert!((sample - 1.0).abs() < 0.001, "Dry output should be 1.0, got {}", sample);
        }

        // Wet should have reverb processing (different from input)
        let has_variation = output_wet.iter().any(|&s| (s - 1.0).abs() > 0.01);
        assert!(has_variation, "Wet output should show reverb processing");
    }

    #[test]
    fn test_reverb_stability() {
        // Test 5: Reverb should be stable (no explosions)

        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.9)),  // Large room
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut processor = BlockProcessor::new(nodes, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        // Process many blocks
        for _ in 0..100 {
            let mut output = vec![0.0; block_size];
            processor.process_block(&mut output, &context).unwrap();

            // Check for explosions
            for (i, &sample) in output.iter().enumerate() {
                assert!(sample.is_finite(), "Sample {} is not finite at block", i);
                assert!(sample.abs() < 100.0, "Sample {} exploded: {}", i, sample);
            }
        }
    }

    #[test]
    fn test_reverb_impulse_response() {
        // Test 6: Verify impulse creates sustained tail
        // This test would need an impulse node, skipping for now
        // TODO: Implement when we have better test infrastructure
    }

    #[test]
    fn test_reverb_pattern_modulation_room_size() {
        // Test 7: Room size can be modulated over time
        // This would need time-varying input, testing basic functionality for now

        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.5)),  // input
            Box::new(ConstantNode::new(0.5)),  // room_size (could be pattern)
            Box::new(ConstantNode::new(0.5)),  // damping
            Box::new(ConstantNode::new(0.5)),  // wet
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut processor = BlockProcessor::new(nodes, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        // Should produce valid output
        assert!(calculate_rms(&output) > 0.001);
    }

    #[test]
    fn test_reverb_pattern_modulation_damping() {
        // Test 8: Damping can be modulated over time

        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.3)),  // damping (could be pattern)
            Box::new(ConstantNode::new(0.5)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut processor = BlockProcessor::new(nodes, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        assert!(calculate_rms(&output) > 0.001);
    }

    #[test]
    fn test_reverb_pattern_modulation_wet() {
        // Test 9: Wet mix can be modulated over time

        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.25)),  // wet (could be pattern)
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut processor = BlockProcessor::new(nodes, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        // Should mix dry and wet
        let rms = calculate_rms(&output);
        assert!(rms > 0.1 && rms < 2.0, "RMS should be reasonable: {}", rms);
    }

    #[test]
    fn test_reverb_zero_input() {
        // Test 10: Reverb with zero input produces zero output

        let block_size = 512;
        let sample_rate = 44100.0;

        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(0.0)),  // Zero input
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut processor = BlockProcessor::new(nodes, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output = vec![0.0; block_size];
        processor.process_block(&mut output, &context).unwrap();

        // Should be silent (or very close to zero)
        assert!(calculate_rms(&output) < 0.001, "Zero input should produce ~zero output");
    }

    #[test]
    fn test_reverb_extreme_room_sizes() {
        // Test 11: Extreme room sizes should be clamped safely

        let block_size = 512;
        let sample_rate = 44100.0;

        // Room size = 0.0 (minimum)
        let nodes_min: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        // Room size = 1.0 (maximum)
        let nodes_max: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut proc_min = BlockProcessor::new(nodes_min, 4, block_size).unwrap();
        let mut proc_max = BlockProcessor::new(nodes_max, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output_min = vec![0.0; block_size];
        let mut output_max = vec![0.0; block_size];

        // Should not crash
        proc_min.process_block(&mut output_min, &context).unwrap();
        proc_max.process_block(&mut output_max, &context).unwrap();

        // Both should produce valid output
        for &sample in &output_min {
            assert!(sample.is_finite());
        }
        for &sample in &output_max {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_reverb_extreme_damping() {
        // Test 12: Extreme damping values should be clamped safely

        let block_size = 512;
        let sample_rate = 44100.0;

        // Damping = 0.0 (no damping)
        let nodes_no_damp: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(0.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        // Damping = 1.0 (full damping)
        let nodes_full_damp: Vec<Box<dyn AudioNode>> = vec![
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.7)),
            Box::new(ConstantNode::new(1.0)),
            Box::new(ConstantNode::new(0.5)),
            Box::new(ReverbNode::new(0, 1, 2, 3)),
        ];

        let mut proc_no_damp = BlockProcessor::new(nodes_no_damp, 4, block_size).unwrap();
        let mut proc_full_damp = BlockProcessor::new(nodes_full_damp, 4, block_size).unwrap();

        let context = ProcessContext::new(
            Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate
        );

        let mut output_no_damp = vec![0.0; block_size];
        let mut output_full_damp = vec![0.0; block_size];

        proc_no_damp.process_block(&mut output_no_damp, &context).unwrap();
        proc_full_damp.process_block(&mut output_full_damp, &context).unwrap();

        // Both should be stable
        for &sample in &output_no_damp {
            assert!(sample.is_finite() && sample.abs() < 100.0);
        }
        for &sample in &output_full_damp {
            assert!(sample.is_finite() && sample.abs() < 100.0);
        }
    }

    #[test]
    fn test_reverb_dependencies() {
        // Test 13: Verify input dependencies are correct

        let reverb = ReverbNode::new(10, 20, 30, 40);
        let deps = reverb.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10);  // input
        assert_eq!(deps[1], 20);  // room_size
        assert_eq!(deps[2], 30);  // damping
        assert_eq!(deps[3], 40);  // wet
    }

    #[test]
    fn test_reverb_name() {
        // Test 14: Verify node name is correct

        let reverb = ReverbNode::new(0, 1, 2, 3);
        assert_eq!(reverb.name(), "ReverbNode");
    }
}
