/// Tap tempo node - converts musical beats to time in seconds
///
/// This utility node converts musical time (beats) to real time (seconds)
/// based on tempo in BPM. Essential for delay times, envelope durations, etc.
///
/// Formula: time_seconds = (60.0 / bpm) * beats
///
/// # Example
/// ```ignore
/// // Create a delay time of 2 beats at 120 BPM
/// let beats = ConstantNode::new(2.0);           // NodeId 0
/// let bpm = ConstantNode::new(120.0);           // NodeId 1
/// let time = TapNode::new(0, 1);                // NodeId 2
/// // Output will be 1.0 second (60.0/120.0 * 2.0 = 1.0)
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Tap tempo converter: converts beats to seconds
///
/// Useful for:
/// - Delay times synchronized to tempo
/// - Envelope durations in musical time
/// - LFO rates matching song tempo
/// - Any time-based parameter that should follow BPM
pub struct TapNode {
    beats: NodeId,
    bpm: NodeId,
}

impl TapNode {
    /// Tap - Converts musical beats to real time (seconds)
    ///
    /// Converts beat count to seconds based on BPM tempo.
    /// Essential for tempo-sync'd delays, modulation, and timing.
    ///
    /// # Parameters
    /// - `beats`: Beat count (1.0=quarter note, 2.0=half note, etc.)
    /// - `bpm`: Tempo in beats per minute (20-300)
    ///
    /// # Example
    /// ```phonon
    /// ~beats: 1.0
    /// ~bpm: 120
    /// ~delay_time: tap ~beats ~bpm
    /// out: sine 440 # delay ~delay_time 0.5
    /// ```
    pub fn new(beats: NodeId, bpm: NodeId) -> Self {
        Self { beats, bpm }
    }

    /// Get the beats input node ID
    pub fn beats(&self) -> NodeId {
        self.beats
    }

    /// Get the BPM input node ID
    pub fn bpm(&self) -> NodeId {
        self.bpm
    }
}

impl AudioNode for TapNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "TapNode requires 2 inputs (beats + bpm), got {}",
            inputs.len()
        );

        let beats = inputs[0];
        let bpm = inputs[1];

        debug_assert_eq!(
            beats.len(),
            output.len(),
            "Beats input length mismatch"
        );
        debug_assert_eq!(
            bpm.len(),
            output.len(),
            "BPM input length mismatch"
        );

        // Formula: time = (60.0 / bpm) * beats
        // Clamp BPM to reasonable range to avoid division issues
        for i in 0..output.len() {
            let b = beats[i].max(0.0);  // Non-negative beats
            let tempo = bpm[i].clamp(20.0, 300.0);  // Reasonable BPM range

            output[i] = (60.0 / tempo) * b;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.beats, self.bpm]
    }

    fn name(&self) -> &str {
        "TapNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_tap_quarter_note_120bpm() {
        // 1 beat at 120 BPM should be 0.5 seconds
        let mut tap = TapNode::new(0, 1);

        let beats = vec![1.0; 4];
        let bpm = vec![120.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // 60.0 / 120.0 * 1.0 = 0.5 seconds
        for &sample in &output {
            assert_eq!(sample, 0.5);
        }
    }

    #[test]
    fn test_tap_half_note_120bpm() {
        // 2 beats at 120 BPM should be 1.0 second
        let mut tap = TapNode::new(0, 1);

        let beats = vec![2.0; 4];
        let bpm = vec![120.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // 60.0 / 120.0 * 2.0 = 1.0 second
        for &sample in &output {
            assert_eq!(sample, 1.0);
        }
    }

    #[test]
    fn test_tap_eighth_note_120bpm() {
        // 0.5 beats at 120 BPM should be 0.25 seconds
        let mut tap = TapNode::new(0, 1);

        let beats = vec![0.5; 4];
        let bpm = vec![120.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // 60.0 / 120.0 * 0.5 = 0.25 seconds
        for &sample in &output {
            assert_eq!(sample, 0.25);
        }
    }

    #[test]
    fn test_tap_60bpm_whole_note() {
        // 4 beats at 60 BPM should be 4.0 seconds
        let mut tap = TapNode::new(0, 1);

        let beats = vec![4.0; 4];
        let bpm = vec![60.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // 60.0 / 60.0 * 4.0 = 4.0 seconds
        for &sample in &output {
            assert_eq!(sample, 4.0);
        }
    }

    #[test]
    fn test_tap_fast_tempo_240bpm() {
        // 1 beat at 240 BPM should be 0.25 seconds
        let mut tap = TapNode::new(0, 1);

        let beats = vec![1.0; 4];
        let bpm = vec![240.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // 60.0 / 240.0 * 1.0 = 0.25 seconds
        for &sample in &output {
            assert_eq!(sample, 0.25);
        }
    }

    #[test]
    fn test_tap_varying_beats() {
        // Test with varying beat values
        let mut tap = TapNode::new(0, 1);

        let beats = vec![1.0, 2.0, 0.5, 4.0];
        let bpm = vec![120.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // 60.0 / 120.0 = 0.5 (beat duration)
        assert_eq!(output[0], 0.5);   // 1.0 beats
        assert_eq!(output[1], 1.0);   // 2.0 beats
        assert_eq!(output[2], 0.25);  // 0.5 beats
        assert_eq!(output[3], 2.0);   // 4.0 beats
    }

    #[test]
    fn test_tap_varying_tempo() {
        // Test with varying BPM values
        let mut tap = TapNode::new(0, 1);

        let beats = vec![1.0; 4];
        let bpm = vec![60.0, 120.0, 240.0, 180.0];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // Different tempos produce different times
        assert_eq!(output[0], 1.0);         // 60.0 / 60.0
        assert_eq!(output[1], 0.5);         // 60.0 / 120.0
        assert_eq!(output[2], 0.25);        // 60.0 / 240.0
        assert!((output[3] - 0.3333333).abs() < 0.0001);  // 60.0 / 180.0
    }

    #[test]
    fn test_tap_clamps_very_low_bpm() {
        // BPM below 20 should be clamped to 20
        let mut tap = TapNode::new(0, 1);

        let beats = vec![1.0; 4];
        let bpm = vec![10.0; 4];  // Below minimum
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // Should use clamped BPM of 20
        // 60.0 / 20.0 * 1.0 = 3.0 seconds
        for &sample in &output {
            assert_eq!(sample, 3.0);
        }
    }

    #[test]
    fn test_tap_clamps_very_high_bpm() {
        // BPM above 300 should be clamped to 300
        let mut tap = TapNode::new(0, 1);

        let beats = vec![1.0; 4];
        let bpm = vec![500.0; 4];  // Above maximum
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // Should use clamped BPM of 300
        // 60.0 / 300.0 * 1.0 = 0.2 seconds
        for &sample in &output {
            assert_eq!(sample, 0.2);
        }
    }

    #[test]
    fn test_tap_negative_beats_clamped() {
        // Negative beats should be clamped to 0
        let mut tap = TapNode::new(0, 1);

        let beats = vec![-1.0, -2.5, 0.0, 1.0];
        let bpm = vec![120.0; 4];
        let inputs = vec![beats.as_slice(), bpm.as_slice()];

        let mut output = vec![0.0; 4];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // Negative beats become 0
        assert_eq!(output[0], 0.0);   // -1.0 clamped to 0.0
        assert_eq!(output[1], 0.0);   // -2.5 clamped to 0.0
        assert_eq!(output[2], 0.0);   // 0.0 * 0.5 = 0.0
        assert_eq!(output[3], 0.5);   // 1.0 * 0.5 = 0.5
    }

    #[test]
    fn test_tap_with_constants() {
        // Integration test with ConstantNode
        let mut beats_node = ConstantNode::new(2.0);
        let mut bpm_node = ConstantNode::new(120.0);
        let mut tap = TapNode::new(0, 1);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process constants first
        let mut beats_buf = vec![0.0; 512];
        let mut bpm_buf = vec![0.0; 512];

        beats_node.process_block(&[], &mut beats_buf, 44100.0, &context);
        bpm_node.process_block(&[], &mut bpm_buf, 44100.0, &context);

        // Apply tap conversion
        let inputs = vec![beats_buf.as_slice(), bpm_buf.as_slice()];
        let mut output = vec![0.0; 512];

        tap.process_block(&inputs, &mut output, 44100.0, &context);

        // Every sample should be 1.0 second (60.0/120.0 * 2.0)
        for sample in &output {
            assert_eq!(*sample, 1.0);
        }
    }

    #[test]
    fn test_tap_dependencies() {
        let tap = TapNode::new(5, 10);
        let deps = tap.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);
        assert_eq!(deps[1], 10);
    }
}
