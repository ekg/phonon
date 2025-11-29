/// Segments Envelope Generator - Multi-segment envelope with configurable breakpoints
///
/// Creates arbitrary envelope shapes using a sequence of segments, each with:
/// - Target value (end point of segment)
/// - Duration (time to reach target)
/// - Curve type (linear or exponential interpolation)
///
/// Triggered by a gate/trigger signal (rising edge detection).
/// Holds at final segment value after completion.
///
/// Key features:
/// - Flexible: any number of segments (unlike fixed ADSR)
/// - Retriggerable: can restart during envelope
/// - Multiple curve types: Linear, Exponential
/// - Pattern-controllable trigger input
use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Interpolation curve type for a segment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveType {
    /// Linear interpolation (constant rate of change)
    Linear,
    /// Exponential interpolation (natural for pitch/amplitude)
    Exponential,
}

/// A single segment in the envelope
#[derive(Debug, Clone)]
pub struct Segment {
    /// Target value at end of this segment
    pub target_value: f32,
    /// Duration in seconds to reach target
    pub duration: f32,
    /// Interpolation curve type
    pub curve: CurveType,
}

impl Segment {
    /// Create a new segment with linear curve
    pub fn linear(target_value: f32, duration: f32) -> Self {
        Self {
            target_value,
            duration,
            curve: CurveType::Linear,
        }
    }

    /// Create a new segment with exponential curve
    pub fn exponential(target_value: f32, duration: f32) -> Self {
        Self {
            target_value,
            duration,
            curve: CurveType::Exponential,
        }
    }
}

/// Internal state for envelope tracking
#[derive(Debug, Clone)]
struct SegmentsState {
    /// Current segment index (0..segments.len())
    current_segment: usize,
    /// Elapsed time within current segment (seconds)
    elapsed_in_segment: f32,
    /// Current envelope output value
    current_value: f32,
    /// Value at start of current segment (for interpolation)
    start_value: f32,
    /// Is envelope currently active?
    is_active: bool,
    /// Previous trigger value (for edge detection)
    last_trigger: f32,
}

impl Default for SegmentsState {
    fn default() -> Self {
        Self {
            current_segment: 0,
            elapsed_in_segment: 0.0,
            current_value: 0.0,
            start_value: 0.0,
            is_active: false,
            last_trigger: 0.0,
        }
    }
}

/// Multi-Segment Envelope Generator Node
///
/// # Inputs
/// 1. Trigger input (> 0.5 = trigger, rising edge detection)
///
/// # Example
/// ```ignore
/// use phonon::nodes::{SegmentsNode, Segment, CurveType};
///
/// // Create ADSR-equivalent envelope
/// let segments = vec![
///     Segment::linear(1.0, 0.01),        // Attack to 1.0 over 10ms
///     Segment::exponential(0.7, 0.1),    // Decay to 0.7 over 100ms
///     Segment::linear(0.7, 1.0),         // Sustain at 0.7 for 1 second
///     Segment::exponential(0.0, 0.2),    // Release to 0.0 over 200ms
/// ];
/// let trigger = ConstantNode::new(1.0);  // NodeId 0
/// let env = SegmentsNode::new(0, segments);
/// ```
pub struct SegmentsNode {
    trigger_input: NodeId,  // Trigger to restart envelope
    segments: Vec<Segment>, // Breakpoint sequence
    state: SegmentsState,   // Internal state machine
}

impl SegmentsNode {
    /// SegmentsNode - Multi-segment envelope generator with linear and exponential curves
    ///
    /// Creates complex envelopes from multiple segments with independent curve types.
    /// Each segment defines a target value and duration with optional exponential shaping.
    ///
    /// # Parameters
    /// - `trigger_input`: NodeId providing trigger signal (>0.5 = trigger, rising edge)
    /// - `segments`: Vector of segments defining the envelope shape
    ///
    /// # Example
    /// ```phonon
    /// ~trigger: square 4.0
    /// ~env: trigger # segments [(1.0, 0.01), (0.7, 0.1), (0.0, 0.2)]
    /// ```
    pub fn new(trigger_input: NodeId, segments: Vec<Segment>) -> Self {
        Self {
            trigger_input,
            segments,
            state: SegmentsState::default(),
        }
    }

    /// Helper: Create segments envelope with all linear curves
    ///
    /// # Arguments
    /// * `trigger_input` - NodeId providing trigger signal
    /// * `targets` - Vector of (target_value, duration) pairs
    pub fn linear_segments(trigger_input: NodeId, targets: Vec<(f32, f32)>) -> Self {
        let segments = targets
            .into_iter()
            .map(|(target, duration)| Segment::linear(target, duration))
            .collect();
        Self::new(trigger_input, segments)
    }

    /// Get current envelope value
    pub fn value(&self) -> f32 {
        self.state.current_value
    }

    /// Check if envelope is currently active
    pub fn is_active(&self) -> bool {
        self.state.is_active
    }

    /// Get current segment index
    pub fn current_segment(&self) -> usize {
        self.state.current_segment
    }

    /// Reset envelope to initial state
    pub fn reset(&mut self) {
        self.state = SegmentsState::default();
    }
}

impl AudioNode for SegmentsNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(inputs.len() >= 1, "SegmentsNode requires 1 input: trigger");

        let trigger_buffer = inputs[0];

        debug_assert_eq!(
            trigger_buffer.len(),
            output.len(),
            "Trigger buffer length mismatch"
        );

        for i in 0..output.len() {
            let trigger = trigger_buffer[i];

            // Detect trigger (rising edge: trigger > 0.5 and was previously <= 0.5)
            let trigger_rising = trigger > 0.5 && self.state.last_trigger <= 0.5;
            self.state.last_trigger = trigger;

            if trigger_rising {
                // Start new envelope from beginning
                self.state.current_segment = 0;
                self.state.elapsed_in_segment = 0.0;
                self.state.start_value = self.state.current_value; // Smooth retrigger
                self.state.is_active = !self.segments.is_empty();
            }

            if self.state.is_active && self.state.current_segment < self.segments.len() {
                let seg = &self.segments[self.state.current_segment];
                let duration = seg.duration.max(0.0001); // Minimum 0.1ms
                let progress = self.state.elapsed_in_segment / duration;

                if progress >= 1.0 {
                    // Segment complete: move to target value and advance
                    self.state.current_value = seg.target_value;
                    self.state.current_segment += 1;

                    if self.state.current_segment < self.segments.len() {
                        // Start next segment
                        self.state.start_value = self.state.current_value;
                        self.state.elapsed_in_segment = 0.0;
                    } else {
                        // All segments complete: deactivate but hold final value
                        self.state.is_active = false;
                    }
                } else {
                    // Segment in progress: interpolate
                    self.state.current_value = match seg.curve {
                        CurveType::Linear => {
                            // Linear interpolation: y = start + (target - start) * progress
                            self.state.start_value
                                + (seg.target_value - self.state.start_value) * progress
                        }
                        CurveType::Exponential => {
                            // Exponential interpolation: y = start * (target/start)^progress
                            // Handle edge cases:
                            // - If start is 0, use linear to avoid division by zero
                            // - If values have opposite signs, use linear to avoid negative base
                            let start = self.state.start_value;
                            let target = seg.target_value;

                            if start.abs() < 0.0001
                                || target.abs() < 0.0001
                                || (start * target) < 0.0
                            {
                                // Fall back to linear for edge cases
                                start + (target - start) * progress
                            } else {
                                // True exponential curve
                                start * (target / start).powf(progress)
                            }
                        }
                    };

                    // Advance time by one sample period
                    self.state.elapsed_in_segment += 1.0 / sample_rate;
                }
            }

            // Output current value (holds at final value after completion)
            output[i] = self.state.current_value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.trigger_input]
    }

    fn name(&self) -> &str {
        "SegmentsNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_segments_two_segment_envelope() {
        // Test 1: Simple 2-segment envelope (attack + decay)
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![
            Segment::linear(1.0, 0.01), // Attack to 1.0 over 10ms
            Segment::linear(0.0, 0.02), // Decay to 0.0 over 20ms
        ];

        let mut trigger = ConstantNode::new(1.0); // Trigger on
        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let mut trigger_buf = vec![0.0; block_size];
        trigger.process_block(&[], &mut trigger_buf, sample_rate, &context);

        let inputs = vec![trigger_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should start rising immediately
        assert!(output[0] >= 0.0, "Should start at 0.0 or above");
        assert!(output[100] > output[0], "Attack should be rising");

        // Should be active
        assert!(env.is_active() || env.current_segment() > 0);
    }

    #[test]
    fn test_segments_four_segment_adsr() {
        // Test 2: 4-segment ADSR-style envelope
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![
            Segment::linear(1.0, 0.001), // Fast attack (1ms)
            Segment::linear(0.7, 0.001), // Fast decay (1ms)
            Segment::linear(0.7, 0.01),  // Sustain hold (10ms)
            Segment::linear(0.0, 0.001), // Fast release (1ms)
        ];

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        // Process multiple blocks to complete envelope
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            env.process_block(&inputs, &mut output, sample_rate, &context);
        }

        // Should eventually complete all segments
        assert!(
            !env.is_active() || env.current_segment() >= 4,
            "Should complete all 4 segments"
        );
    }

    #[test]
    fn test_segments_exponential_curve() {
        // Test 3: Verify exponential curve differs from linear
        // NOTE: Must start from non-zero value for true exponential curve
        let sample_rate = 44100.0;
        let block_size = 1024; // Longer block to see curve

        let linear_seg = vec![Segment::linear(2.0, 0.01)];
        let exp_seg = vec![Segment::exponential(2.0, 0.01)];

        let mut env_linear = SegmentsNode::new(0, linear_seg);
        let mut env_exp = SegmentsNode::new(0, exp_seg);

        // Pre-set start values to 0.1 (non-zero for exponential)
        env_linear.state.current_value = 0.1;
        env_linear.state.start_value = 0.1;
        env_exp.state.current_value = 0.1;
        env_exp.state.start_value = 0.1;

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output_linear = vec![0.0; block_size];
        let mut output_exp = vec![0.0; block_size];

        env_linear.process_block(&inputs, &mut output_linear, sample_rate, &context);
        env_exp.process_block(&inputs, &mut output_exp, sample_rate, &context);

        // Exponential curve (0.1 -> 2.0) should differ from linear
        // At midpoint (50% progress), linear is at ~1.05, exponential should be different
        let midpoint = 220; // ~5ms at 44100 Hz
        if midpoint < block_size {
            let diff = (output_linear[midpoint] - output_exp[midpoint]).abs();
            assert!(
                diff > 0.1,
                "Exponential curve should differ from linear at midpoint, got linear={}, exp={}",
                output_linear[midpoint],
                output_exp[midpoint]
            );
        }
    }

    #[test]
    fn test_segments_linear_curve() {
        // Test 4: Verify linear curve has constant rate of change
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![Segment::linear(1.0, 0.01)]; // 10ms to reach 1.0

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Check for constant increment (linear)
        if block_size > 100 {
            let increment_1 = output[51] - output[50];
            let increment_2 = output[101] - output[100];

            assert!(
                (increment_1 - increment_2).abs() < 0.001,
                "Linear curve should have constant increment, got {} and {}",
                increment_1,
                increment_2
            );
        }
    }

    #[test]
    fn test_segments_retrigger() {
        // Test 5: Retriggering should restart envelope
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![
            Segment::linear(1.0, 0.1), // Slow attack (100ms)
        ];

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // First trigger
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        let value_before_retrigger = output[block_size - 1];
        assert!(value_before_retrigger > 0.0, "Should have progressed");

        // Trigger off
        let trigger_buf = vec![0.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Retrigger (rising edge)
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should restart from current value (smooth retrigger)
        assert_eq!(env.current_segment(), 0, "Should restart at segment 0");
    }

    #[test]
    fn test_segments_completion_holds() {
        // Test 6: After completion, should hold at final value
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![
            Segment::linear(1.0, 0.001), // Very fast (1ms)
        ];

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should complete within first block
        let completion_sample = (0.001 * sample_rate) as usize + 10;
        if completion_sample < block_size {
            assert!(
                (output[completion_sample] - 1.0).abs() < 0.1,
                "Should reach target value, got {}",
                output[completion_sample]
            );

            // Should hold at final value
            assert!(
                (output[block_size - 1] - 1.0).abs() < 0.1,
                "Should hold at final value, got {}",
                output[block_size - 1]
            );

            assert!(!env.is_active(), "Should be inactive after completion");
        }
    }

    #[test]
    fn test_segments_multiple_triggers() {
        // Test 7: Multiple triggers should restart envelope each time
        let sample_rate = 44100.0;
        let block_size = 64; // Small blocks

        let segments = vec![Segment::linear(1.0, 0.1)]; // 100ms

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        // First trigger
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Trigger off
        let trigger_buf = vec![0.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Second trigger
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        assert_eq!(
            env.current_segment(),
            0,
            "Should restart at segment 0 on second trigger"
        );

        // Trigger off
        let trigger_buf = vec![0.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Third trigger
        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        assert_eq!(
            env.current_segment(),
            0,
            "Should restart at segment 0 on third trigger"
        );
    }

    #[test]
    fn test_segments_timing_accuracy() {
        // Test 8: Verify segment timing is accurate
        let sample_rate = 44100.0;
        let block_size = 512;
        let duration = 0.01; // 10ms = 441 samples

        let segments = vec![Segment::linear(1.0, duration)];

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        let expected_completion = (duration * sample_rate) as usize;

        // Should be near 1.0 at expected completion time
        if expected_completion < block_size {
            assert!(
                output[expected_completion] > 0.9,
                "Should be near completion at sample {}, got {}",
                expected_completion,
                output[expected_completion]
            );
        }
    }

    #[test]
    fn test_segments_very_short() {
        // Test 9: Very short segments (1 sample)
        let sample_rate = 44100.0;
        let block_size = 512;
        let duration = 1.0 / sample_rate; // 1 sample

        let segments = vec![
            Segment::linear(0.5, duration),
            Segment::linear(1.0, duration),
        ];

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should complete very quickly
        assert!(
            output[10] > 0.0,
            "Should have progressed in first 10 samples"
        );
    }

    #[test]
    fn test_segments_very_long() {
        // Test 10: Very long segments (1 second)
        let sample_rate = 44100.0;
        let block_size = 512;
        let duration = 1.0; // 1 second

        let segments = vec![Segment::linear(1.0, duration)];

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        // Process first block
        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should still be in first segment
        assert_eq!(env.current_segment(), 0, "Should still be in segment 0");
        assert!(env.is_active(), "Should still be active");

        // Should be progressing slowly
        let first_value = output[block_size - 1];
        assert!(
            first_value < 0.02,
            "Should progress slowly, got {}",
            first_value
        );
    }

    #[test]
    fn test_segments_single_segment() {
        // Test 11: Single segment envelope
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![Segment::linear(1.0, 0.01)]; // Just attack

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should progress through single segment
        assert!(output[0] >= 0.0, "Should start at 0");
        assert!(output[100] > output[0], "Should be rising");
    }

    #[test]
    fn test_segments_empty() {
        // Test 12: Empty segments should output 0.0
        let sample_rate = 44100.0;
        let block_size = 512;

        let segments = vec![]; // No segments

        let mut env = SegmentsNode::new(0, segments);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // All output should be 0.0
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(
                sample, 0.0,
                "Sample {} should be 0.0 with empty segments",
                i
            );
        }

        assert!(!env.is_active(), "Should not be active with empty segments");
    }

    #[test]
    fn test_segments_input_nodes() {
        // Test 13: Verify input_nodes dependencies
        let segments = vec![Segment::linear(1.0, 0.1)];
        let env = SegmentsNode::new(42, segments);

        let deps = env.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42); // trigger_input
    }

    #[test]
    fn test_segments_linear_helper() {
        // Test 14: Verify linear_segments helper constructor
        let sample_rate = 44100.0;
        let block_size = 512;

        let targets = vec![(0.5, 0.01), (1.0, 0.01), (0.0, 0.02)];

        let mut env = SegmentsNode::linear_segments(0, targets);

        let context =
            ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

        let trigger_buf = vec![1.0; block_size];
        let inputs = vec![trigger_buf.as_slice()];

        let mut output = vec![0.0; block_size];
        env.process_block(&inputs, &mut output, sample_rate, &context);

        // Should create 3 linear segments
        assert!(output[0] >= 0.0, "Should start");
        assert!(output[100] > 0.0, "Should be progressing");
    }
}
