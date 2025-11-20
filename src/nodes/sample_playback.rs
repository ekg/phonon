/// Sample Playback Node - DAW-style buffer passing sample playback
///
/// This node plays back sample data with trigger detection and speed control.
/// It demonstrates sample-accurate triggering with rising edge detection,
/// linear interpolation for fractional playback positions, and shared sample
/// data using Arc for efficient memory usage.
///
/// # Algorithm
/// - Detect rising edge on trigger signal (0 -> 1 transition, threshold 0.5)
/// - Start playback from beginning on trigger
/// - Advance read position by speed factor each sample
/// - Use linear interpolation for fractional positions (smooth playback)
/// - Stop when reaching end of sample
///
/// # Features
/// - Retriggerable: New trigger restarts playback from beginning
/// - Variable speed: speed = 1.0 (normal), 2.0 (double speed), 0.5 (half speed)
/// - Shared data: Arc<Vec<f32>> allows multiple playback nodes to share same sample
/// - Sample-accurate: Triggers detected at sample precision (not block level)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::sync::Arc;

/// Sample playback node with trigger and speed control
///
/// # Inputs
/// 1. Trigger signal (> 0.5 = start playback, rising edge detection)
/// 2. Speed signal (playback rate, 1.0 = normal)
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
///
/// // Load sample data (normally from file)
/// let sample_data = Arc::new(vec![0.0, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5]);
///
/// // Create trigger and speed inputs
/// let trigger = ImpulseNode::new(/* freq */);  // NodeId 0
/// let speed = ConstantNode::new(1.0);          // NodeId 1
///
/// // Create sample playback node
/// let playback = SamplePlaybackNode::new(0, 1, sample_data);  // NodeId 2
/// ```
pub struct SamplePlaybackNode {
    trigger_input: NodeId,           // Trigger signal input (>0.5 = start)
    speed_input: NodeId,             // Playback speed input (1.0 = normal)
    sample_data: Arc<Vec<f32>>,     // Shared sample data

    // Internal playback state
    playback_position: f32,          // Current read position (fractional)
    is_playing: bool,                // Whether playback is active
    last_trigger: f32,               // Last trigger value (for edge detection)
}

impl SamplePlaybackNode {
    /// Create a new sample playback node
    ///
    /// # Arguments
    /// * `trigger_input` - NodeId providing trigger signal (> 0.5 = start)
    /// * `speed_input` - NodeId providing playback speed (1.0 = normal)
    /// * `sample_data` - Arc-wrapped sample data to play back
    pub fn new(
        trigger_input: NodeId,
        speed_input: NodeId,
        sample_data: Arc<Vec<f32>>,
    ) -> Self {
        Self {
            trigger_input,
            speed_input,
            sample_data,
            playback_position: 0.0,
            is_playing: false,
            last_trigger: 0.0,
        }
    }

    /// Get current playback position (fractional samples)
    pub fn position(&self) -> f32 {
        self.playback_position
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Reset playback state (stop and return to beginning)
    pub fn reset(&mut self) {
        self.playback_position = 0.0;
        self.is_playing = false;
        self.last_trigger = 0.0;
    }

    /// Get sample data length
    pub fn sample_length(&self) -> usize {
        self.sample_data.len()
    }
}

impl AudioNode for SamplePlaybackNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "SamplePlaybackNode requires 2 inputs: trigger, speed"
        );

        let trigger_buffer = inputs[0];
        let speed_buffer = inputs[1];

        debug_assert_eq!(
            trigger_buffer.len(),
            output.len(),
            "Trigger buffer length mismatch"
        );
        debug_assert_eq!(
            speed_buffer.len(),
            output.len(),
            "Speed buffer length mismatch"
        );

        for i in 0..output.len() {
            let trigger = trigger_buffer[i];
            let speed = speed_buffer[i];

            // Detect rising edge (0 -> 1 transition)
            // Trigger is high (>0.5) AND was previously low (<=0.5)
            let trigger_high = trigger > 0.5;
            let was_low = self.last_trigger <= 0.5;

            if trigger_high && was_low {
                // Rising edge detected: start playback from beginning
                self.playback_position = 0.0;
                self.is_playing = true;
            }

            // Update trigger state for next sample
            self.last_trigger = trigger;

            // Playback
            if self.is_playing {
                let pos = self.playback_position as usize;

                // Check if we're still within sample bounds
                if pos < self.sample_data.len() {
                    // Linear interpolation for smooth playback
                    let frac = self.playback_position.fract();  // Fractional part
                    let s1 = self.sample_data[pos];

                    // Get next sample for interpolation (or use current if at end)
                    let s2 = self.sample_data
                        .get(pos + 1)
                        .copied()
                        .unwrap_or(s1);

                    // Linear interpolation: s1 * (1 - frac) + s2 * frac
                    output[i] = s1 * (1.0 - frac) + s2 * frac;

                    // Advance playback position by speed
                    self.playback_position += speed;
                } else {
                    // Reached end of sample: stop playback and output silence
                    self.is_playing = false;
                    output[i] = 0.0;
                }
            } else {
                // Not playing: output silence
                output[i] = 0.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.trigger_input, self.speed_input]
    }

    fn name(&self) -> &str {
        "SamplePlaybackNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    /// Helper function to create a simple test sample (sine-ish wave)
    fn create_test_sample(length: usize) -> Arc<Vec<f32>> {
        let mut data = Vec::with_capacity(length);
        for i in 0..length {
            let phase = (i as f32) / (length as f32);
            data.push((phase * 2.0 * std::f32::consts::PI).sin());
        }
        Arc::new(data)
    }

    /// Helper function to create a ramp sample (0.0 to 1.0)
    fn create_ramp_sample(length: usize) -> Arc<Vec<f32>> {
        let mut data = Vec::with_capacity(length);
        for i in 0..length {
            data.push(i as f32 / (length - 1) as f32);
        }
        Arc::new(data)
    }

    #[test]
    fn test_sample_playback_trigger_starts_playback() {
        // Test 1: Trigger starts playback
        let sample_data = create_test_sample(1000);  // Longer sample (1000 samples)
        let mut trigger = ConstantNode::new(1.0);  // High trigger
        let mut speed = ConstantNode::new(1.0);    // Normal speed
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate input buffers
        let mut trigger_buf = vec![0.0; 512];
        let mut speed_buf = vec![0.0; 512];
        trigger.process_block(&[], &mut trigger_buf, 44100.0, &context);
        speed.process_block(&[], &mut speed_buf, 44100.0, &context);

        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 512];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // Should be playing (sample is 1000 samples, only played 512)
        assert!(playback.is_playing(), "Playback should start on trigger");

        // Should have non-zero output (sample is playing)
        let has_signal = output.iter().take(100).any(|&x| x.abs() > 0.01);
        assert!(has_signal, "Should produce audio when playing");
    }

    #[test]
    fn test_sample_playback_no_trigger_no_sound() {
        // Test 2: No trigger = no sound
        let sample_data = create_test_sample(100);
        let mut trigger = ConstantNode::new(0.0);  // Low trigger
        let mut speed = ConstantNode::new(1.0);
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let mut trigger_buf = vec![0.0; 512];
        let mut speed_buf = vec![0.0; 512];
        trigger.process_block(&[], &mut trigger_buf, 44100.0, &context);
        speed.process_block(&[], &mut speed_buf, 44100.0, &context);

        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 512];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // Should not be playing
        assert!(!playback.is_playing(), "Should not start without trigger");

        // Output should be all zeros
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(sample, 0.0, "Sample {} should be silent without trigger", i);
        }
    }

    #[test]
    fn test_sample_playback_rising_edge_detection() {
        // Test 3: Rising edge detection (0 -> 1 transition)
        let sample_data = create_test_sample(1000);  // Longer sample
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // First 256 samples: trigger low (0.0)
        // Last 256 samples: trigger high (1.0)
        let mut trigger_buf = vec![0.0; 512];
        for i in 256..512 {
            trigger_buf[i] = 1.0;
        }
        let speed_buf = vec![1.0; 512];

        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 512];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // First 256 samples should be silent (trigger low)
        let first_half_silent = output.iter().take(256).all(|&x| x == 0.0);
        assert!(first_half_silent, "First half should be silent (trigger low)");

        // After sample 256, should start playing (rising edge)
        let second_half_has_signal = output.iter().skip(257).take(50).any(|&x| x.abs() > 0.01);
        assert!(second_half_has_signal, "Should start playing on rising edge");

        // Should be playing by the end (sample is 1000 long, only played 256 samples)
        assert!(playback.is_playing(), "Should be playing after rising edge");
    }

    #[test]
    fn test_sample_playback_retrigger() {
        // Test 4: Retrigger restarts from beginning
        let sample_data = create_ramp_sample(100);  // 0.0 to 1.0 ramp
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            64,
            2.0,
            44100.0,
        );

        // First trigger: start playback
        let trigger_buf = vec![1.0; 64];
        let speed_buf = vec![1.0; 64];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output1 = vec![0.0; 64];
        playback.process_block(&inputs, &mut output1, 44100.0, &context);

        // Should be playing and advanced
        assert!(playback.is_playing());
        let pos_after_first = playback.position();
        assert!(pos_after_first > 0.0, "Position should advance");

        // Trigger goes low, then high again (retrigger)
        let mut trigger_buf = vec![0.0; 64];
        trigger_buf[32] = 1.0;  // Rising edge at sample 32
        for i in 33..64 {
            trigger_buf[i] = 1.0;
        }
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output2 = vec![0.0; 64];
        playback.process_block(&inputs, &mut output2, 44100.0, &context);

        // After retrigger, position should reset and restart
        // The sample is a ramp, so early samples should be smaller values
        let value_after_retrigger = output2[33];  // Just after retrigger
        assert!(
            value_after_retrigger < 0.1,
            "Should restart from beginning on retrigger, got {}",
            value_after_retrigger
        );
    }

    #[test]
    fn test_sample_playback_speed_control() {
        // Test 5: Speed affects playback rate
        let sample_data = create_test_sample(2000);  // Long sample (2000 samples)

        // Normal speed (1.0)
        let mut playback_normal = SamplePlaybackNode::new(0, 1, sample_data.clone());
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let trigger_buf = vec![1.0; 512];
        let speed_buf = vec![1.0; 512];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output_normal = vec![0.0; 512];
        playback_normal.process_block(&inputs, &mut output_normal, 44100.0, &context);

        let pos_normal = playback_normal.position();

        // Double speed (2.0)
        let mut playback_fast = SamplePlaybackNode::new(0, 1, sample_data.clone());
        let trigger_buf = vec![1.0; 512];
        let speed_buf = vec![2.0; 512];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output_fast = vec![0.0; 512];
        playback_fast.process_block(&inputs, &mut output_fast, 44100.0, &context);

        let pos_fast = playback_fast.position();

        // Double speed should advance twice as far
        assert!(
            (pos_fast - 2.0 * pos_normal).abs() < 1.0,
            "Double speed should advance twice as far: normal={}, fast={} (expected ~{})",
            pos_normal,
            pos_fast,
            2.0 * pos_normal
        );
    }

    #[test]
    fn test_sample_playback_half_speed() {
        // Test 6: Half speed playback
        let sample_data = create_test_sample(1000);
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let trigger_buf = vec![1.0; 512];
        let speed_buf = vec![0.5; 512];  // Half speed
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 512];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        let pos_half = playback.position();

        // At half speed, should advance by ~256 samples
        assert!(
            (pos_half - 256.0).abs() < 1.0,
            "Half speed should advance by ~256 samples, got {}",
            pos_half
        );
    }

    #[test]
    fn test_sample_playback_stops_at_end() {
        // Test 7: Playback stops at end of sample
        let sample_data = create_test_sample(100);  // Short sample
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Trigger playback
        let trigger_buf = vec![1.0; 512];
        let speed_buf = vec![1.0; 512];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 512];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have stopped (sample is only 100 samples long)
        assert!(!playback.is_playing(), "Should stop after reaching end of sample");

        // Later samples should be silent
        let later_samples_silent = output.iter().skip(150).all(|&x| x == 0.0);
        assert!(later_samples_silent, "Should be silent after sample ends");
    }

    #[test]
    fn test_sample_playback_linear_interpolation() {
        // Test 8: Linear interpolation for fractional positions
        // Create a simple two-sample test: [0.0, 1.0]
        let sample_data = Arc::new(vec![0.0, 1.0]);
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        // Start playback, then manually set position to 0.5
        let trigger_buf = vec![1.0; 1];
        let speed_buf = vec![0.0; 1];  // Zero speed (don't advance)
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 1];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // Manually set fractional position
        playback.playback_position = 0.5;
        playback.is_playing = true;

        // Process one sample
        let trigger_buf = vec![0.0; 1];  // Keep trigger low
        let speed_buf = vec![0.0; 1];    // Don't advance
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 1];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // At position 0.5, should interpolate between 0.0 and 1.0
        // Result should be 0.0 * 0.5 + 1.0 * 0.5 = 0.5
        assert!(
            (output[0] - 0.5).abs() < 0.01,
            "Linear interpolation should produce 0.5, got {}",
            output[0]
        );
    }

    #[test]
    fn test_sample_playback_correct_sample_output() {
        // Test 9: Verify correct sample data is output
        let sample_data = Arc::new(vec![0.1, 0.2, 0.3, 0.4, 0.5]);
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            5,
            2.0,
            44100.0,
        );

        // Trigger and play at normal speed
        let trigger_buf = vec![1.0; 5];
        let speed_buf = vec![1.0; 5];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 5];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // First sample should be close to 0.1 (may have slight interpolation)
        assert!(
            (output[0] - 0.1).abs() < 0.05,
            "First sample should be ~0.1, got {}",
            output[0]
        );

        // Samples should generally increase (ramp upward)
        assert!(output[1] > output[0], "Samples should increase");
        assert!(output[2] > output[1], "Samples should increase");
    }

    #[test]
    fn test_sample_playback_multiple_triggers() {
        // Test 10: Multiple triggers in one block
        let sample_data = create_ramp_sample(100);
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            128,
            2.0,
            44100.0,
        );

        // Create trigger pattern: high, then low, then high again
        let mut trigger_buf = vec![0.0; 128];
        trigger_buf[0] = 1.0;  // First trigger
        trigger_buf[1] = 1.0;
        trigger_buf[2] = 0.0;  // Goes low
        trigger_buf[64] = 1.0; // Second trigger (rising edge)
        for i in 65..128 {
            trigger_buf[i] = 1.0;
        }

        let speed_buf = vec![1.0; 128];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 128];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // Should have retriggered at sample 64
        // The ramp should restart, so value at 65 should be small
        assert!(
            output[65] < 0.1,
            "Should restart from beginning on second trigger, got {}",
            output[65]
        );
    }

    #[test]
    fn test_sample_playback_dependencies() {
        // Test 11: Verify input_nodes returns correct dependencies
        let sample_data = create_test_sample(100);
        let playback = SamplePlaybackNode::new(10, 20, sample_data);
        let deps = playback.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 10);  // trigger_input
        assert_eq!(deps[1], 20);  // speed_input
    }

    #[test]
    fn test_sample_playback_reset() {
        // Test 12: Reset functionality
        let sample_data = create_test_sample(100);
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        // Start playback
        playback.playback_position = 50.0;
        playback.is_playing = true;
        playback.last_trigger = 1.0;

        assert!(playback.is_playing());
        assert_eq!(playback.position(), 50.0);

        // Reset
        playback.reset();

        assert!(!playback.is_playing());
        assert_eq!(playback.position(), 0.0);
        assert_eq!(playback.last_trigger, 0.0);
    }

    #[test]
    fn test_sample_playback_empty_sample() {
        // Test 13: Empty sample handling
        let sample_data = Arc::new(Vec::new());
        let mut playback = SamplePlaybackNode::new(0, 1, sample_data.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let trigger_buf = vec![1.0; 512];
        let speed_buf = vec![1.0; 512];
        let inputs = vec![trigger_buf.as_slice(), speed_buf.as_slice()];
        let mut output = vec![0.0; 512];
        playback.process_block(&inputs, &mut output, 44100.0, &context);

        // Should not crash, should output silence
        let all_silent = output.iter().all(|&x| x == 0.0);
        assert!(all_silent, "Empty sample should produce silence");
    }

    #[test]
    fn test_sample_playback_shared_data() {
        // Test 14: Arc allows data sharing between multiple nodes
        let sample_data = create_test_sample(100);

        // Create two playback nodes sharing same data
        let playback1 = SamplePlaybackNode::new(0, 1, sample_data.clone());
        let playback2 = SamplePlaybackNode::new(2, 3, sample_data.clone());

        // Verify both reference the same data
        assert_eq!(playback1.sample_length(), 100);
        assert_eq!(playback2.sample_length(), 100);

        // Verify Arc refcount increased (data is shared)
        assert_eq!(Arc::strong_count(&sample_data), 3);  // Original + 2 nodes
    }
}
