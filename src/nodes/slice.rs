/// Slice Node - Sample slicing with trigger control
///
/// This node accumulates incoming audio into a buffer and plays back
/// configurable slices when triggered. It's designed for triggering
/// portions of a buffer on demand, enabling techniques like:
/// - Beat slicing (play back 1/8th, 1/16th sections)
/// - Sample chopping (trigger random slices)
/// - Live re-sequencing (rearrange audio in real-time)
///
/// # Algorithm
/// 1. Accumulate input audio into internal buffer
/// 2. On trigger (rising edge >0.5), start slice playback
/// 3. Slice position defined by slice_start and slice_end (0.0-1.0)
/// 4. Use linear interpolation for smooth playback
/// 5. Retriggerable: new trigger restarts from slice_start
///
/// # Parameters
/// - `input` - Audio buffer to accumulate and slice
/// - `trigger` - Trigger signal (>0.5 = start slice, rising edge detection)
/// - `slice_start` - Start position in buffer (0.0-1.0)
/// - `slice_end` - End position in buffer (0.0-1.0)
///
/// # Example Use Cases
/// ```ignore
/// // Play first quarter of accumulated buffer on each trigger
/// slice(input, trigger, 0.0, 0.25)
///
/// // Play middle half
/// slice(input, trigger, 0.25, 0.75)
///
/// // Pattern-controlled slice positions (dynamic slicing)
/// slice(input, trigger, "0 0.25 0.5 0.75", "0.25 0.5 0.75 1.0")
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Maximum buffer size (10 seconds at 44100 Hz)
const MAX_BUFFER_SIZE: usize = 441000;

/// Slice node with trigger and position control
pub struct SliceNode {
    input: NodeId,              // Audio input to accumulate
    trigger_input: NodeId,      // Trigger signal input (>0.5 = start)
    slice_start_input: NodeId,  // Slice start position (0.0-1.0)
    slice_end_input: NodeId,    // Slice end position (0.0-1.0)

    // Internal buffer state
    buffer: Vec<f32>,           // Accumulated audio buffer
    write_position: usize,      // Current write position in buffer

    // Playback state
    playback_position: f32,     // Current read position (fractional samples)
    is_playing: bool,           // Whether slice playback is active
    last_trigger: f32,          // Last trigger value (for edge detection)

    // Current slice bounds (in samples)
    slice_start_samples: usize,
    slice_end_samples: usize,
}

impl SliceNode {
    /// Create a new slice node
    ///
    /// # Arguments
    /// * `input` - NodeId providing audio to accumulate
    /// * `trigger_input` - NodeId providing trigger signal (> 0.5 = start)
    /// * `slice_start_input` - NodeId providing slice start position (0.0-1.0)
    /// * `slice_end_input` - NodeId providing slice end position (0.0-1.0)
    pub fn new(
        input: NodeId,
        trigger_input: NodeId,
        slice_start_input: NodeId,
        slice_end_input: NodeId,
    ) -> Self {
        Self {
            input,
            trigger_input,
            slice_start_input,
            slice_end_input,
            buffer: Vec::new(),
            write_position: 0,
            playback_position: 0.0,
            is_playing: false,
            last_trigger: 0.0,
            slice_start_samples: 0,
            slice_end_samples: 0,
        }
    }

    /// Get current buffer length
    pub fn buffer_length(&self) -> usize {
        self.buffer.len()
    }

    /// Get current playback position
    pub fn position(&self) -> f32 {
        self.playback_position
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Reset internal state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.write_position = 0;
        self.playback_position = 0.0;
        self.is_playing = false;
        self.last_trigger = 0.0;
        self.slice_start_samples = 0;
        self.slice_end_samples = 0;
    }

    /// Clear buffer (keep playback state)
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.write_position = 0;
    }
}

impl AudioNode for SliceNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 4,
            "SliceNode requires 4 inputs: input, trigger, slice_start, slice_end"
        );

        let input_buffer = inputs[0];
        let trigger_buffer = inputs[1];
        let slice_start_buffer = inputs[2];
        let slice_end_buffer = inputs[3];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            trigger_buffer.len(),
            output.len(),
            "Trigger buffer length mismatch"
        );
        debug_assert_eq!(
            slice_start_buffer.len(),
            output.len(),
            "Slice start buffer length mismatch"
        );
        debug_assert_eq!(
            slice_end_buffer.len(),
            output.len(),
            "Slice end buffer length mismatch"
        );

        for i in 0..output.len() {
            // Accumulate input into buffer (if not full)
            if self.buffer.len() < MAX_BUFFER_SIZE {
                self.buffer.push(input_buffer[i]);
                self.write_position += 1;
            }

            let trigger = trigger_buffer[i];
            let slice_start = slice_start_buffer[i].clamp(0.0, 1.0);
            let slice_end = slice_end_buffer[i].clamp(0.0, 1.0);

            let buffer_len = self.buffer.len();

            // Detect rising edge (0 -> 1 transition)
            let trigger_high = trigger > 0.5;
            let was_low = self.last_trigger <= 0.5;

            if trigger_high && was_low && buffer_len > 0 {
                // Rising edge detected: Calculate slice bounds and start playback
                // Bounds are calculated at trigger time based on current buffer size

                // Ensure start <= end
                let (start_norm, end_norm) = if slice_start <= slice_end {
                    (slice_start, slice_end)
                } else {
                    (slice_end, slice_start)
                };

                self.slice_start_samples = (start_norm * buffer_len as f32) as usize;
                self.slice_end_samples = (end_norm * buffer_len as f32) as usize;

                // Start playback from slice start
                self.playback_position = self.slice_start_samples as f32;
                self.is_playing = true;
            }

            // Update trigger state for next sample
            self.last_trigger = trigger;

            // Playback
            if self.is_playing && buffer_len > 0 {
                let pos = self.playback_position as usize;

                // Check if we're still within slice bounds (strict check - stop at slice_end)
                if pos < self.slice_end_samples && pos < buffer_len {
                    // Linear interpolation for smooth playback
                    let frac = self.playback_position.fract();
                    let s1 = self.buffer[pos];

                    // Get next sample for interpolation (or use current if at end)
                    let s2 = if pos + 1 < buffer_len && pos + 1 < self.slice_end_samples {
                        self.buffer[pos + 1]
                    } else {
                        s1
                    };

                    // Linear interpolation: s1 * (1 - frac) + s2 * frac
                    output[i] = s1 * (1.0 - frac) + s2 * frac;

                    // Advance playback position by 1 sample
                    self.playback_position += 1.0;

                    // Check if we've reached the end after advancing
                    if self.playback_position as usize > self.slice_end_samples {
                        self.is_playing = false;
                    }
                } else {
                    // Reached end of slice: stop playback
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
        vec![
            self.input,
            self.trigger_input,
            self.slice_start_input,
            self.slice_end_input,
        ]
    }

    fn name(&self) -> &str {
        "SliceNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    /// Helper function to create a ramp sample (0.0 to 1.0)
    fn create_ramp(length: usize) -> Vec<f32> {
        (0..length)
            .map(|i| i as f32 / (length - 1) as f32)
            .collect()
    }

    /// Helper to process a block with slice node
    fn process_slice(
        slice: &mut SliceNode,
        input: &[f32],
        trigger: &[f32],
        slice_start: &[f32],
        slice_end: &[f32],
    ) -> Vec<f32> {
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            input.len(),
            2.0,
            44100.0,
        );

        let inputs = vec![input, trigger, slice_start, slice_end];
        let mut output = vec![0.0; input.len()];
        slice.process_block(&inputs, &mut output, 44100.0, &context);
        output
    }

    #[test]
    fn test_slice_no_trigger_no_output() {
        // Test 1: No trigger = no output (even with buffer)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        let input = create_ramp(100);
        let trigger = vec![0.0; 100];  // No trigger
        let slice_start = vec![0.0; 100];
        let slice_end = vec![1.0; 100];

        let output = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Buffer should accumulate but no playback
        assert_eq!(slice.buffer_length(), 100);
        assert!(!slice.is_playing());

        // Output should be all zeros
        for (i, &sample) in output.iter().enumerate() {
            assert_eq!(sample, 0.0, "Sample {} should be silent without trigger", i);
        }
    }

    #[test]
    fn test_slice_trigger_plays_full_buffer() {
        // Test 2: Trigger plays full buffer (slice_start=0, slice_end=1)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // First, accumulate some data
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![1.0; 100];

        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);
        assert_eq!(slice.buffer_length(), 100);

        // Now trigger playback
        let silence_input = vec![0.0; 100];  // No new input
        let trigger = vec![1.0; 100];  // High trigger
        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // Should be playing
        assert!(slice.is_playing(), "Should start playing on trigger");

        // Should have non-zero output (playing back ramp)
        let has_signal = output.iter().take(50).any(|&x| x > 0.01);
        assert!(has_signal, "Should produce audio when playing slice");

        // Should be increasing (ramp)
        assert!(output[10] < output[20], "Ramp should increase");
        assert!(output[20] < output[30], "Ramp should increase");
    }

    #[test]
    fn test_slice_rising_edge_detection() {
        // Test 3: Rising edge detection (0 -> 1 transition)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate buffer
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![1.0; 100];
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Create rising edge at sample 50
        let silence_input = vec![0.0; 100];
        let mut trigger = vec![0.0; 100];
        for i in 50..100 {
            trigger[i] = 1.0;
        }

        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // First 50 samples should be silent (trigger low)
        let first_half_silent = output.iter().take(50).all(|&x| x == 0.0);
        assert!(first_half_silent, "First half should be silent (trigger low)");

        // After sample 50, should start playing
        let second_half_has_signal = output.iter().skip(51).take(20).any(|&x| x > 0.01);
        assert!(second_half_has_signal, "Should start playing on rising edge");
    }

    #[test]
    fn test_slice_first_quarter() {
        // Test 4: Slice first quarter (0.0 to 0.25)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate 100 sample ramp (0.0 to 1.0)
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![0.25; 100];  // First quarter
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Trigger playback
        let silence_input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // Should play only first quarter (samples 0-25)
        // Values should be in range 0.0 to 0.25
        let max_value = output.iter().take(30).fold(0.0f32, |a, &b| a.max(b));
        assert!(
            max_value < 0.35,
            "First quarter slice should have max value ~0.25, got {}",
            max_value
        );

        // Should stop after ~25 samples
        let later_samples_silent = output.iter().skip(30).all(|&x| x == 0.0);
        assert!(later_samples_silent, "Should stop after first quarter ends");
    }

    #[test]
    fn test_slice_middle_half() {
        // Test 5: Slice middle half (0.25 to 0.75)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate 100 sample ramp
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.25; 100];  // Start at 25%
        let slice_end = vec![0.75; 100];    // End at 75%
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Trigger playback
        let silence_input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // First value should be ~0.25 (start of slice)
        assert!(
            (output[0] - 0.25).abs() < 0.05,
            "First value should be ~0.25, got {}",
            output[0]
        );

        // Should play ~50 samples (25 to 75)
        let has_signal_early = output.iter().take(50).any(|&x| x > 0.3);
        assert!(has_signal_early, "Should have signal in middle range");

        // Should stop after ~50 samples
        let later_silent = output.iter().skip(55).all(|&x| x == 0.0);
        assert!(later_silent, "Should stop after slice ends");
    }

    #[test]
    fn test_slice_last_eighth() {
        // Test 6: Slice last eighth (0.875 to 1.0)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate 100 sample ramp
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.875; 100];  // Start at 87.5%
        let slice_end = vec![1.0; 100];      // End at 100%
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Trigger playback
        let silence_input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // First value should be ~0.875
        assert!(
            (output[0] - 0.875).abs() < 0.1,
            "First value should be ~0.875, got {}",
            output[0]
        );

        // Should play only ~12-13 samples
        let signal_count = output.iter().filter(|&&x| x > 0.0).count();
        assert!(
            signal_count < 20,
            "Last eighth should play ~12 samples, got {}",
            signal_count
        );
    }

    #[test]
    fn test_slice_retrigger_restarts() {
        // Test 7: Retrigger restarts from slice_start
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate buffer
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![0.5; 100];
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // First trigger
        let silence_input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let _ = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // Should be playing and advanced
        assert!(slice.is_playing() || slice.position() > 0.0);

        // Retrigger: go low then high again
        let mut trigger = vec![0.0; 100];
        trigger[50] = 1.0;  // Rising edge at 50
        for i in 51..100 {
            trigger[i] = 1.0;
        }

        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // At sample 51 (after retrigger), should restart from beginning
        let value_after_retrigger = output[51];
        assert!(
            value_after_retrigger < 0.1,
            "Should restart from beginning on retrigger, got {}",
            value_after_retrigger
        );
    }

    #[test]
    fn test_slice_empty_buffer_no_crash() {
        // Test 8: Empty buffer doesn't crash
        let mut slice = SliceNode::new(0, 1, 2, 3);

        let input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![1.0; 100];

        let output = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Should not crash, should output silence
        let all_silent = output.iter().all(|&x| x == 0.0);
        assert!(all_silent, "Empty buffer should produce silence");
    }

    #[test]
    fn test_slice_swapped_start_end() {
        // Test 9: Swapped start/end are corrected
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate buffer
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.75; 100];  // Start > End (swapped)
        let slice_end = vec![0.25; 100];
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Trigger playback
        let silence_input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // Should play from 0.25 to 0.75 (corrected order)
        let first_value = output[0];
        assert!(
            (first_value - 0.25).abs() < 0.05,
            "Should start at 0.25 (corrected), got {}",
            first_value
        );
    }

    #[test]
    fn test_slice_zero_length_slice() {
        // Test 10: Zero-length slice (start == end)
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate buffer
        let input = create_ramp(100);
        let trigger = vec![0.0; 100];
        let slice_start = vec![0.5; 100];
        let slice_end = vec![0.5; 100];  // Same as start
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Trigger playback
        let silence_input = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let output = process_slice(&mut slice, &silence_input, &trigger, &slice_start, &slice_end);

        // Should output single sample then stop
        let signal_count = output.iter().filter(|&&x| x > 0.0).count();
        assert!(
            signal_count <= 2,
            "Zero-length slice should output ≤2 samples, got {}",
            signal_count
        );
    }

    #[test]
    fn test_slice_accumulates_over_multiple_blocks() {
        // Test 11: Buffer accumulates over multiple blocks
        let mut slice = SliceNode::new(0, 1, 2, 3);

        let trigger = vec![0.0; 50];
        let slice_start = vec![0.0; 50];
        let slice_end = vec![1.0; 50];

        // Block 1: Accumulate 0.0 to 0.5
        let input1 = create_ramp(50);
        let _ = process_slice(&mut slice, &input1, &trigger, &slice_start, &slice_end);
        assert_eq!(slice.buffer_length(), 50);

        // Block 2: Accumulate 0.5 to 1.0
        let input2: Vec<f32> = (50..100)
            .map(|i| i as f32 / 99.0)
            .collect();
        let _ = process_slice(&mut slice, &input2, &trigger, &slice_start, &slice_end);
        assert_eq!(slice.buffer_length(), 100);

        // Trigger playback of full accumulated buffer
        let silence = vec![0.0; 100];
        let trigger = vec![1.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![1.0; 100];
        let output = process_slice(&mut slice, &silence, &trigger, &slice_start, &slice_end);

        // Should play entire 100 samples
        let has_high_values = output.iter().any(|&x| x > 0.8);
        assert!(has_high_values, "Should contain high values from second block");
    }

    #[test]
    fn test_slice_max_buffer_size() {
        // Test 12: Buffer respects MAX_BUFFER_SIZE limit
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Try to accumulate more than MAX_BUFFER_SIZE
        let block_size = 1024;
        let trigger = vec![0.0; block_size];
        let slice_start = vec![0.0; block_size];
        let slice_end = vec![1.0; block_size];

        // Accumulate many blocks
        for _ in 0..500 {
            let input = vec![1.0; block_size];
            let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);
        }

        // Buffer should not exceed MAX_BUFFER_SIZE
        assert!(
            slice.buffer_length() <= MAX_BUFFER_SIZE,
            "Buffer should not exceed MAX_BUFFER_SIZE, got {}",
            slice.buffer_length()
        );
    }

    #[test]
    fn test_slice_linear_interpolation() {
        // Test 13: Linear interpolation at fractional positions
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Create simple two-value buffer: [0.0, 1.0]
        let input = vec![0.0, 1.0];
        let trigger = vec![0.0; 2];
        let slice_start = vec![0.0; 2];
        let slice_end = vec![1.0; 2];
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        // Trigger to initialize slice bounds
        let silence = vec![0.0; 1];
        let trigger_high = vec![1.0; 1];
        let slice_start_1 = vec![0.0; 1];
        let slice_end_1 = vec![1.0; 1];
        let _ = process_slice(&mut slice, &silence, &trigger_high, &slice_start_1, &slice_end_1);

        // Manually set playback position to 0.5 for interpolation test
        slice.playback_position = 0.5;
        slice.is_playing = true;

        // Process one sample with low trigger (no retrigger)
        let trigger_low = vec![0.0; 1];
        let output = process_slice(&mut slice, &silence, &trigger_low, &slice_start_1, &slice_end_1);

        // At position 0.5, should interpolate: 0.0 * 0.5 + 1.0 * 0.5 = 0.5
        assert!(
            (output[0] - 0.5).abs() < 0.01,
            "Linear interpolation should produce 0.5, got {}",
            output[0]
        );
    }

    #[test]
    fn test_slice_dependencies() {
        // Test 14: Verify input_nodes returns correct dependencies
        let slice = SliceNode::new(10, 20, 30, 40);
        let deps = slice.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10);  // input
        assert_eq!(deps[1], 20);  // trigger_input
        assert_eq!(deps[2], 30);  // slice_start_input
        assert_eq!(deps[3], 40);  // slice_end_input
    }

    #[test]
    fn test_slice_reset() {
        // Test 15: Reset clears all state
        let mut slice = SliceNode::new(0, 1, 2, 3);

        // Accumulate some data and start playback
        let input = create_ramp(100);
        let trigger = vec![1.0; 100];
        let slice_start = vec![0.0; 100];
        let slice_end = vec![1.0; 100];
        let _ = process_slice(&mut slice, &input, &trigger, &slice_start, &slice_end);

        assert!(slice.buffer_length() > 0);

        // Reset
        slice.reset();

        assert_eq!(slice.buffer_length(), 0);
        assert!(!slice.is_playing());
        assert_eq!(slice.position(), 0.0);
    }
}
