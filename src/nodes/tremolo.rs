/// Tremolo node - amplitude modulation effect
///
/// This node applies tremolo (amplitude modulation) to an input signal using an
/// internal LFO (Low Frequency Oscillator). The LFO is a sine wave that ranges
/// from 0.0 to 1.0, which modulates the amplitude of the input signal.
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;

/// Tremolo effect with pattern-controlled rate and depth
///
/// # Algorithm
/// - LFO generates sine wave from 0.0 to 1.0
/// - Gain = 1.0 - (depth * (1.0 - lfo))
/// - Output = input * gain
///
/// # Example
/// ```ignore
/// // Tremolo at 4 Hz with 50% depth
/// let input = OscillatorNode::new(0, Waveform::Sine);     // NodeId 0
/// let rate = ConstantNode::new(4.0);                       // NodeId 1
/// let depth = ConstantNode::new(0.5);                      // NodeId 2
/// let tremolo = TremoloNode::new(0, 1, 2);                 // NodeId 3
/// ```
pub struct TremoloNode {
    input: NodeId,       // Audio input to modulate
    rate_input: NodeId,  // LFO rate in Hz
    depth_input: NodeId, // Modulation depth (0.0 to 1.0)
    phase: f32,          // LFO phase accumulator (0.0 to 1.0)
}

impl TremoloNode {
    /// Tremolo - Amplitude modulation effect with LFO control
    ///
    /// Modulates signal amplitude using internal sine-wave LFO.
    /// Creates pulsing, breathing effects at any rate.
    ///
    /// # Parameters
    /// - `input`: Audio signal to modulate
    /// - `rate_input`: LFO rate in Hz (0.5-20 typical)
    /// - `depth_input`: Modulation depth (0.0-1.0)
    ///
    /// # Example
    /// ```phonon
    /// ~synth: sine 440
    /// out: ~synth # tremolo 4.0 0.7
    /// ```
    pub fn new(input: NodeId, rate_input: NodeId, depth_input: NodeId) -> Self {
        Self {
            input,
            rate_input,
            depth_input,
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

impl AudioNode for TremoloNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "TremoloNode requires 3 inputs (input, rate, depth), got {}",
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
            let rate = rate_buffer[i];
            let depth = depth_buffer[i].clamp(0.0, 1.0);

            // Generate LFO (sine wave from 0.0 to 1.0)
            let lfo = (self.phase * 2.0 * PI).sin() * 0.5 + 0.5;

            // Apply amplitude modulation
            // When depth=0: gain=1.0 (no modulation)
            // When depth=1: gain ranges from 0.0 to 1.0 (full modulation)
            let gain = 1.0 - (depth * (1.0 - lfo));
            output[i] = sample * gain;

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
        "TremoloNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    fn create_test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_tremolo_zero_depth_no_effect() {
        // When depth=0, output should equal input (no modulation)
        let mut input_osc = OscillatorNode::new(0, Waveform::Sine);
        let mut rate = ConstantNode::new(4.0);
        let mut depth = ConstantNode::new(0.0); // Zero depth
        let mut tremolo = TremoloNode::new(0, 1, 2);

        let context = create_test_context(512);

        // Generate input signal (440 Hz sine)
        let mut freq_buf = vec![0.0; 512];
        let mut freq_const = ConstantNode::new(440.0);
        freq_const.process_block(&[], &mut freq_buf, 44100.0, &context);

        let freq_inputs = vec![freq_buf.as_slice()];
        let mut input_buf = vec![0.0; 512];
        input_osc.process_block(&freq_inputs, &mut input_buf, 44100.0, &context);

        // Generate rate and depth buffers
        let mut rate_buf = vec![0.0; 512];
        let mut depth_buf = vec![0.0; 512];
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        // Apply tremolo
        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        tremolo.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should match input when depth=0
        for i in 0..512 {
            assert!(
                (output[i] - input_buf[i]).abs() < 0.001,
                "Sample {} differs: output={}, input={}",
                i,
                output[i],
                input_buf[i]
            );
        }
    }

    #[test]
    fn test_tremolo_full_depth_modulation() {
        // When depth=1, gain should range from 0.0 to 1.0
        let mut input = ConstantNode::new(1.0); // Constant signal
        let mut rate = ConstantNode::new(4.0);
        let mut depth = ConstantNode::new(1.0); // Full depth
        let mut tremolo = TremoloNode::new(0, 1, 2);

        let context = create_test_context(44100); // 1 second at 44.1kHz

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output = vec![0.0; 44100];
        tremolo.process_block(&inputs, &mut output, 44100.0, &context);

        // Find min and max values
        let min = output.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // With full depth, gain should modulate from 0.0 to 1.0
        // So output should range from 0.0 to 1.0 (since input is 1.0)
        assert!(min < 0.1, "Min too high: {}", min);
        assert!(max > 0.9, "Max too low: {}", max);
    }

    #[test]
    fn test_tremolo_rate_affects_speed() {
        // Higher rate should produce more cycles in same time period
        let context = create_test_context(44100); // 1 second

        // Test with 2 Hz
        let mut input_2hz = ConstantNode::new(1.0);
        let mut rate_2hz = ConstantNode::new(2.0);
        let mut depth_2hz = ConstantNode::new(1.0);
        let mut tremolo_2hz = TremoloNode::new(0, 1, 2);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input_2hz.process_block(&[], &mut input_buf, 44100.0, &context);
        rate_2hz.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth_2hz.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output_2hz = vec![0.0; 44100];
        tremolo_2hz.process_block(&inputs, &mut output_2hz, 44100.0, &context);

        // Test with 8 Hz
        let mut input_8hz = ConstantNode::new(1.0);
        let mut rate_8hz = ConstantNode::new(8.0);
        let mut depth_8hz = ConstantNode::new(1.0);
        let mut tremolo_8hz = TremoloNode::new(0, 1, 2);

        input_8hz.process_block(&[], &mut input_buf, 44100.0, &context);
        rate_8hz.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth_8hz.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output_8hz = vec![0.0; 44100];
        tremolo_8hz.process_block(&inputs, &mut output_8hz, 44100.0, &context);

        // Count zero crossings (where signal crosses threshold)
        let threshold = 0.5;
        let crossings_2hz = count_crossings(&output_2hz, threshold);
        let crossings_8hz = count_crossings(&output_8hz, threshold);

        // 8 Hz should have ~4x more crossings than 2 Hz
        assert!(
            crossings_8hz > crossings_2hz * 3,
            "8 Hz should have more crossings: 2Hz={}, 8Hz={}",
            crossings_2hz,
            crossings_8hz
        );
    }

    #[test]
    fn test_tremolo_lfo_range() {
        // LFO should range from 0.0 to 1.0
        let mut input = ConstantNode::new(1.0);
        let mut rate = ConstantNode::new(4.0);
        let mut depth = ConstantNode::new(1.0);
        let mut tremolo = TremoloNode::new(0, 1, 2);

        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output = vec![0.0; 44100];
        tremolo.process_block(&inputs, &mut output, 44100.0, &context);

        // All output values should be in [0.0, 1.0]
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample >= 0.0 && sample <= 1.0,
                "Sample {} out of range: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_tremolo_phase_advances() {
        let tremolo = TremoloNode::new(0, 1, 2);
        assert_eq!(tremolo.phase(), 0.0);

        let mut tremolo = tremolo;

        // Process one sample at 4 Hz
        let input_buf = vec![1.0];
        let rate_buf = vec![4.0];
        let depth_buf = vec![0.5];

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output = vec![0.0; 1];

        let context = create_test_context(1);
        tremolo.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should have advanced by 4/44100
        let expected_phase = 4.0 / 44100.0;
        assert!(
            (tremolo.phase() - expected_phase).abs() < 0.0001,
            "Phase mismatch: got {}, expected {}",
            tremolo.phase(),
            expected_phase
        );
    }

    #[test]
    fn test_tremolo_dependencies() {
        let tremolo = TremoloNode::new(10, 20, 30);
        let deps = tremolo.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10);
        assert_eq!(deps[1], 20);
        assert_eq!(deps[2], 30);
    }

    #[test]
    fn test_tremolo_with_constants() {
        // Test with all constant inputs
        let mut input = ConstantNode::new(0.8);
        let mut rate = ConstantNode::new(5.0);
        let mut depth = ConstantNode::new(0.5);
        let mut tremolo = TremoloNode::new(0, 1, 2);

        // Use 44100 samples (1 second) to ensure full cycles
        let context = create_test_context(44100);

        let mut input_buf = vec![0.0; 44100];
        let mut rate_buf = vec![0.0; 44100];
        let mut depth_buf = vec![0.0; 44100];

        input.process_block(&[], &mut input_buf, 44100.0, &context);
        rate.process_block(&[], &mut rate_buf, 44100.0, &context);
        depth.process_block(&[], &mut depth_buf, 44100.0, &context);

        let inputs = vec![
            input_buf.as_slice(),
            rate_buf.as_slice(),
            depth_buf.as_slice(),
        ];
        let mut output = vec![0.0; 44100];
        tremolo.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should have modulation (varying values)
        let min = output.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        assert!(
            max - min > 0.2,
            "Not enough modulation: min={}, max={}",
            min,
            max
        );
    }

    #[test]
    fn test_tremolo_reset_phase() {
        let mut tremolo = TremoloNode::new(0, 1, 2);

        // Advance phase
        tremolo.phase = 0.5;
        assert_eq!(tremolo.phase(), 0.5);

        // Reset
        tremolo.reset_phase();
        assert_eq!(tremolo.phase(), 0.0);
    }

    // Helper function to count threshold crossings
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
}
