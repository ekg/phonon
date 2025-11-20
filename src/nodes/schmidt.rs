/// Schmidt trigger node - comparator with hysteresis for noise immunity
///
/// A Schmidt trigger converts an analog signal to a digital gate signal with
/// hysteresis, providing noise immunity. It has two thresholds:
/// - high_threshold: Level where gate turns ON
/// - low_threshold: Level where gate turns OFF
///
/// The hysteresis prevents rapid switching when the input hovers near a single threshold.
///
/// # Algorithm
/// ```text
/// If gate is currently LOW:
///   If input > high_threshold: gate = HIGH (1.0)
/// If gate is currently HIGH:
///   If input < low_threshold: gate = LOW (0.0)
/// Otherwise: maintain current state
/// ```
///
/// # Applications
/// - Convert noisy sensor signals to clean gates
/// - Noise gate with adjustable hysteresis
/// - Trigger detection with noise immunity
/// - Threshold detection for drums/transients
///
/// # Example
/// ```ignore
/// // Create a gate that turns on at 0.5, off at 0.3
/// let input = OscillatorNode::new(Waveform::Sine);  // NodeId 1
/// let high_thresh = ConstantNode::new(0.5);         // NodeId 2
/// let low_thresh = ConstantNode::new(0.3);          // NodeId 3
/// let gate = SchmidtNode::new(1, 2, 3);             // NodeId 4
/// // Output: 1.0 when input > 0.5, 0.0 when input < 0.3, held otherwise
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Schmidt trigger state
#[derive(Debug, Clone)]
struct SchmidtState {
    gate_high: bool,  // Current gate state (true = high/1.0, false = low/0.0)
}

impl Default for SchmidtState {
    fn default() -> Self {
        Self { gate_high: false }
    }
}

/// Schmidt trigger node: comparator with hysteresis
///
/// Provides noise-immune threshold detection by using two thresholds:
/// - Switches to HIGH when input exceeds high_threshold
/// - Switches to LOW when input falls below low_threshold
/// - Maintains state when between thresholds
pub struct SchmidtNode {
    input: NodeId,
    high_threshold_input: NodeId,  // Threshold to turn gate ON
    low_threshold_input: NodeId,   // Threshold to turn gate OFF
    state: SchmidtState,            // Current gate state
}

impl SchmidtNode {
    /// Create a new Schmidt trigger node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to threshold
    /// * `high_threshold_input` - NodeId of high threshold (gate turns on above this)
    /// * `low_threshold_input` - NodeId of low threshold (gate turns off below this)
    ///
    /// # Note
    /// For proper hysteresis, high_threshold should be greater than low_threshold.
    /// If they're equal, it behaves like a simple comparator.
    pub fn new(
        input: NodeId,
        high_threshold_input: NodeId,
        low_threshold_input: NodeId,
    ) -> Self {
        Self {
            input,
            high_threshold_input,
            low_threshold_input,
            state: SchmidtState::default(),
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the high threshold input node ID
    pub fn high_threshold_input(&self) -> NodeId {
        self.high_threshold_input
    }

    /// Get the low threshold input node ID
    pub fn low_threshold_input(&self) -> NodeId {
        self.low_threshold_input
    }

    /// Reset Schmidt trigger state to OFF
    pub fn reset(&mut self) {
        self.state = SchmidtState::default();
    }

    /// Get current gate state (for debugging/testing)
    pub fn gate_state(&self) -> bool {
        self.state.gate_high
    }
}

impl AudioNode for SchmidtNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 3,
            "SchmidtNode requires 3 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let high_thresh_buf = inputs[1];
        let low_thresh_buf = inputs[2];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        // Process each sample with hysteresis logic
        for i in 0..output.len() {
            let sample = input_buf[i];
            let high_threshold = high_thresh_buf[i];
            let low_threshold = low_thresh_buf[i];

            // Update gate state based on hysteresis
            if !self.state.gate_high {
                // Currently LOW: check if input exceeds high threshold
                if sample > high_threshold {
                    self.state.gate_high = true;
                }
            } else {
                // Currently HIGH: check if input falls below low threshold
                if sample < low_threshold {
                    self.state.gate_high = false;
                }
            }

            // Output 1.0 if gate is high, 0.0 if low
            output[i] = if self.state.gate_high { 1.0 } else { 0.0 };
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.high_threshold_input,
            self.low_threshold_input,
        ]
    }

    fn name(&self) -> &str {
        "SchmidtNode"
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
    fn test_schmidt_turns_on_at_high_threshold() {
        // Test that gate turns ON when input exceeds high threshold
        let size = 512;

        // Create ramping input from 0.0 to 1.0
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = i as f32 / size as f32;
        }

        // High threshold at 0.5, low at 0.3
        let high_thresh = vec![0.5; size];
        let low_thresh = vec![0.3; size];

        let inputs: Vec<&[f32]> = vec![&input, &high_thresh, &low_thresh];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut schmidt = SchmidtNode::new(0, 1, 2);
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be 0.0 before threshold, 1.0 after
        // Find transition point
        let first_half = &output[0..size / 2];  // Input 0.0-0.5
        let second_half = &output[size / 2..];  // Input 0.5-1.0

        // First half should be mostly 0.0 (before threshold)
        let low_count = first_half.iter().filter(|&&x| x < 0.5).count();
        assert!(
            low_count > size / 4,
            "Expected mostly low in first half, got {} low out of {}",
            low_count,
            first_half.len()
        );

        // Second half should be mostly 1.0 (after threshold)
        let high_count = second_half.iter().filter(|&&x| x > 0.5).count();
        assert!(
            high_count > size / 4,
            "Expected mostly high in second half, got {} high out of {}",
            high_count,
            second_half.len()
        );
    }

    #[test]
    fn test_schmidt_turns_off_at_low_threshold() {
        // Test that gate turns OFF when input falls below low threshold
        let size = 512;

        // Create ramping input from 1.0 down to 0.0
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = 1.0 - (i as f32 / size as f32);
        }

        // High threshold at 0.7, low at 0.3
        let high_thresh = vec![0.7; size];
        let low_thresh = vec![0.3; size];

        let inputs: Vec<&[f32]> = vec![&input, &high_thresh, &low_thresh];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut schmidt = SchmidtNode::new(0, 1, 2);

        // First, pre-charge the gate by processing high input
        let high_input = vec![1.0; size];
        let charge_inputs: Vec<&[f32]> = vec![&high_input, &high_thresh, &low_thresh];
        schmidt.process_block(&charge_inputs, &mut output, 44100.0, &context);

        // Gate should be high now
        assert!(schmidt.gate_state(), "Gate should be ON after high input");

        // Now process the falling ramp
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should be 1.0 at start (above low threshold), 0.0 at end (below)
        let first_quarter = &output[0..size / 4];  // Input 1.0-0.75 (above 0.3)
        let last_quarter = &output[3 * size / 4..]; // Input 0.25-0.0 (below 0.3)

        // First quarter should be mostly 1.0
        let high_count = first_quarter.iter().filter(|&&x| x > 0.5).count();
        assert!(
            high_count > size / 8,
            "Expected mostly high in first quarter, got {} high out of {}",
            high_count,
            first_quarter.len()
        );

        // Last quarter should be mostly 0.0
        let low_count = last_quarter.iter().filter(|&&x| x < 0.5).count();
        assert!(
            low_count > size / 8,
            "Expected mostly low in last quarter, got {} low out of {}",
            low_count,
            last_quarter.len()
        );
    }

    #[test]
    fn test_schmidt_hysteresis() {
        // Test that hysteresis prevents rapid switching
        let size = 512;

        // Create input that oscillates between low and high thresholds
        // Input: 0.4 (between thresholds)
        let input = vec![0.4; size];
        let high_thresh = vec![0.5; size];  // Turn on at 0.5
        let low_thresh = vec![0.3; size];   // Turn off at 0.3

        let inputs: Vec<&[f32]> = vec![&input, &high_thresh, &low_thresh];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut schmidt = SchmidtNode::new(0, 1, 2);

        // Process with gate starting LOW
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should remain constant (all 0.0) because input never crosses threshold
        for &val in &output {
            assert_eq!(val, 0.0, "Output should remain low when between thresholds");
        }

        // Now pre-charge gate to HIGH
        schmidt.reset();
        let high_input = vec![1.0; size];
        let charge_inputs: Vec<&[f32]> = vec![&high_input, &high_thresh, &low_thresh];
        schmidt.process_block(&charge_inputs, &mut output, 44100.0, &context);

        // Process same input again (0.4, between thresholds)
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should remain constant (all 1.0) because input never crosses low threshold
        for &val in &output {
            assert_eq!(
                val, 1.0,
                "Output should remain high when between thresholds"
            );
        }
    }

    #[test]
    fn test_schmidt_with_noisy_signal() {
        // Test noise immunity with a signal that crosses threshold multiple times
        let size = 512;

        // Create noisy signal around threshold
        let mut input = vec![0.0; size];
        for i in 0..size {
            let base = 0.5 + 0.1 * ((i as f32 * 0.1).sin());
            let noise = (i as f32 * 0.7).sin() * 0.02; // Small noise
            input[i] = base + noise;
        }

        // Narrow hysteresis (more switching)
        let high_thresh = vec![0.52; size];
        let low_thresh = vec![0.48; size];

        let inputs: Vec<&[f32]> = vec![&input, &high_thresh, &low_thresh];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut schmidt = SchmidtNode::new(0, 1, 2);
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);

        // Count transitions
        let mut transitions = 0;
        for i in 1..output.len() {
            if output[i] != output[i - 1] {
                transitions += 1;
            }
        }

        // Should have some transitions but not excessive (hysteresis limits them)
        assert!(
            transitions > 0,
            "Should have at least one transition with varying input"
        );
        assert!(
            transitions < 50,
            "Hysteresis should limit transitions, got {}",
            transitions
        );
    }

    #[test]
    fn test_schmidt_node_interface() {
        // Test node getters
        let schmidt = SchmidtNode::new(1, 2, 3);

        assert_eq!(schmidt.input(), 1);
        assert_eq!(schmidt.high_threshold_input(), 2);
        assert_eq!(schmidt.low_threshold_input(), 3);

        let inputs = schmidt.input_nodes();
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0], 1);
        assert_eq!(inputs[1], 2);
        assert_eq!(inputs[2], 3);

        assert_eq!(schmidt.name(), "SchmidtNode");
    }

    #[test]
    fn test_schmidt_reset() {
        // Test that reset clears gate state
        let size = 512;

        // High input to turn gate ON
        let input = vec![1.0; size];
        let high_thresh = vec![0.5; size];
        let low_thresh = vec![0.3; size];

        let inputs: Vec<&[f32]> = vec![&input, &high_thresh, &low_thresh];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut schmidt = SchmidtNode::new(0, 1, 2);

        // Turn gate ON
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);
        assert!(schmidt.gate_state(), "Gate should be ON after processing high input");

        // Reset
        schmidt.reset();
        assert!(
            !schmidt.gate_state(),
            "Gate should be OFF after reset"
        );
    }

    #[test]
    fn test_schmidt_equal_thresholds() {
        // Test behavior when high and low thresholds are equal (simple comparator)
        let size = 512;

        // Create ramping input
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = i as f32 / size as f32;
        }

        // Equal thresholds at 0.5
        let high_thresh = vec![0.5; size];
        let low_thresh = vec![0.5; size];

        let inputs: Vec<&[f32]> = vec![&input, &high_thresh, &low_thresh];
        let mut output = vec![0.0; size];
        let context = create_context(size);

        let mut schmidt = SchmidtNode::new(0, 1, 2);
        schmidt.process_block(&inputs, &mut output, 44100.0, &context);

        // Should behave like simple comparator
        // Before 0.5: output = 0.0, after 0.5: output = 1.0
        let threshold_idx = size / 2;

        // Check samples before and after threshold
        assert!(
            output[threshold_idx - 10] < 0.5,
            "Output should be low before threshold"
        );
        assert!(
            output[threshold_idx + 10] > 0.5,
            "Output should be high after threshold"
        );
    }
}
