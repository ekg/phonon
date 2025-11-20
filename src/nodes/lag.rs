/// Lag node - exponential slew limiter for portamento/glide effects
///
/// This node applies exponential smoothing to a signal, useful for:
/// - Portamento/glide effects on pitch
/// - Smooth parameter transitions
/// - Low-pass filter-like smoothing
/// - Attack-release envelope simulation
/// - Smoothing control voltages
///
/// Unlike linear slew limiting, lag uses exponential approach to the target,
/// which sounds more natural for musical portamento effects.

use crate::audio_node::{AudioNode, NodeId, ProcessContext};

/// Lag node: exponential smoothing with time constant control
///
/// The algorithm:
/// ```text
/// for each sample:
///     target = input[i]
///     lag_time = max(0.001, lag_time_input[i])  // Min 1ms
///
///     // Calculate coefficient for exponential approach
///     // Formula: output += (target - output) * coeff
///     // where coeff = 1 - exp(-1 / (lag_time * sample_rate))
///     coeff = 1.0 - exp(-1.0 / (lag_time * sample_rate))
///
///     // Exponential approach to target
///     current_value += (target - current_value) * coeff
///
///     output[i] = current_value
/// ```
///
/// # Theory
///
/// The lag time constant determines how quickly the output approaches the target:
/// - After 1 time constant: output reaches ~63% of target
/// - After 2 time constants: output reaches ~86% of target
/// - After 3 time constants: output reaches ~95% of target
/// - After 5 time constants: output reaches ~99.3% of target
///
/// # Example
/// ```ignore
/// // Smooth a step function with 0.1 second lag
/// let input = ConstantNode::new(1.0);           // NodeId 0
/// let lag_time = ConstantNode::new(0.1);        // NodeId 1 (100ms time constant)
/// let lag = LagNode::new(0, 1);                 // NodeId 2
/// // Output will exponentially approach 1.0, reaching 63% in 100ms
/// ```
pub struct LagNode {
    /// Input signal to smooth
    input: NodeId,

    /// Lag time constant in seconds (time to reach ~63% of target)
    lag_time_input: NodeId,

    /// Current output value (for exponential smoothing)
    current_value: f32,
}

impl LagNode {
    /// Create a new lag node
    ///
    /// # Arguments
    /// * `input` - NodeId of signal to smooth
    /// * `lag_time_input` - NodeId of lag time signal (in seconds)
    ///
    /// # Initial State
    /// - `current_value` starts at 0.0
    pub fn new(input: NodeId, lag_time_input: NodeId) -> Self {
        Self {
            input,
            lag_time_input,
            current_value: 0.0,
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the lag time input node ID
    pub fn lag_time_input(&self) -> NodeId {
        self.lag_time_input
    }

    /// Get the current output value (for debugging/testing)
    pub fn current_value(&self) -> f32 {
        self.current_value
    }
}

impl AudioNode for LagNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "LagNode requires 2 inputs, got {}",
            inputs.len()
        );

        let input_buf = inputs[0];
        let lag_time_buf = inputs[1];

        debug_assert_eq!(
            input_buf.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            lag_time_buf.len(),
            output.len(),
            "Lag time buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let target = input_buf[i];
            let lag_time = lag_time_buf[i].max(0.001); // Prevent too-fast changes (1ms min)

            // Calculate coefficient for exponential approach
            // Formula: output += (target - output) * coeff
            // where coeff = 1 - exp(-1 / (lag_time * sample_rate))
            let coeff = 1.0 - (-1.0 / (lag_time * sample_rate)).exp();

            // Exponential approach to target
            self.current_value += (target - self.current_value) * coeff;

            output[i] = self.current_value;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.lag_time_input]
    }

    fn name(&self) -> &str {
        "LagNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_lag_smooths_step_change() {
        let mut lag = LagNode::new(0, 1);

        // Instant step from 0 to 1
        let input = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        let lag_time = vec![0.1; 5]; // 100ms time constant
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; 5];
        let context = create_context(5);

        lag.process_block(&inputs, &mut output, 44100.0, &context);

        // With 100ms time constant at 44100 Hz:
        // coeff = 1 - exp(-1 / (0.1 * 44100)) = 1 - exp(-0.0002267) ≈ 0.0002267
        // Sample 0: 0.0 + (1.0 - 0.0) * 0.0002267 = 0.0002267
        // Should be gradually rising, not instant
        assert!(output[0] > 0.0);
        assert!(output[0] < 0.01, "output[0] = {}", output[0]);
        assert!(output[1] > output[0]);
        assert!(output[2] > output[1]);
        assert!(output[3] > output[2]);
        assert!(output[4] > output[3]);

        // Should not reach target yet (exponential approach)
        assert!(output[4] < 0.01, "output[4] = {}", output[4]);
    }

    #[test]
    fn test_lag_exponential_approach() {
        let mut lag = LagNode::new(0, 1);

        // Calculate how many samples for one time constant (0.1 seconds)
        let lag_time_seconds = 0.1;
        let sample_rate = 44100.0;
        let samples_per_time_constant = (lag_time_seconds * sample_rate) as usize;

        // Create buffer with enough samples for one time constant
        let input = vec![1.0; samples_per_time_constant];
        let lag_time = vec![lag_time_seconds; samples_per_time_constant];
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; samples_per_time_constant];
        let context = create_context(samples_per_time_constant);

        lag.process_block(&inputs, &mut output, sample_rate, &context);

        // After one time constant, should reach approximately 63% of target
        // (1 - 1/e ≈ 0.632)
        let final_value = output[samples_per_time_constant - 1];
        assert!(final_value > 0.60, "final_value = {}", final_value);
        assert!(final_value < 0.66, "final_value = {}", final_value);
    }

    #[test]
    fn test_lag_time_constant_effect() {
        let mut lag_fast = LagNode::new(0, 1);
        let mut lag_slow = LagNode::new(0, 1);

        let samples = 100;
        let input = vec![1.0; samples];

        // Fast lag: 10ms
        let fast_time = vec![0.01; samples];
        let inputs_fast = vec![input.as_slice(), fast_time.as_slice()];
        let mut output_fast = vec![0.0; samples];
        let context = create_context(samples);

        lag_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);

        // Slow lag: 100ms (10x slower)
        let slow_time = vec![0.1; samples];
        let inputs_slow = vec![input.as_slice(), slow_time.as_slice()];
        let mut output_slow = vec![0.0; samples];

        lag_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

        // After same number of samples, fast lag should be much closer to target
        assert!(output_fast[99] > output_slow[99] * 5.0,
                "fast = {}, slow = {}",
                output_fast[99], output_slow[99]);
    }

    #[test]
    fn test_lag_fast_vs_slow() {
        let mut lag_fast = LagNode::new(0, 1);
        let mut lag_slow = LagNode::new(0, 1);

        // Step input
        let input = vec![1.0; 10];

        // Fast lag: 1ms
        let fast_time = vec![0.001; 10];
        let inputs_fast = vec![input.as_slice(), fast_time.as_slice()];
        let mut output_fast = vec![0.0; 10];
        let context = create_context(10);

        lag_fast.process_block(&inputs_fast, &mut output_fast, 44100.0, &context);

        // Slow lag: 100ms
        let slow_time = vec![0.1; 10];
        let inputs_slow = vec![input.as_slice(), slow_time.as_slice()];
        let mut output_slow = vec![0.0; 10];

        lag_slow.process_block(&inputs_slow, &mut output_slow, 44100.0, &context);

        // Fast lag should approach target much quicker
        assert!(output_fast[9] > 0.1, "output_fast[9] = {}", output_fast[9]);
        assert!(output_slow[9] < 0.01, "output_slow[9] = {}", output_slow[9]);
        assert!(output_fast[9] > output_slow[9] * 10.0);
    }

    #[test]
    fn test_lag_continuous_input() {
        let mut lag = LagNode::new(0, 1);

        // Slowly increasing input (linear ramp)
        let input: Vec<f32> = (0..10).map(|i| i as f32 / 10.0).collect();
        let lag_time = vec![0.01; 10]; // 10ms lag
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; 10];
        let context = create_context(10);

        lag.process_block(&inputs, &mut output, 44100.0, &context);

        // Output should lag behind input
        for i in 1..10 {
            // Output should be less than input (lagging)
            assert!(output[i] < input[i],
                    "output[{}] = {}, input[{}] = {}",
                    i, output[i], i, input[i]);

            // Output should be increasing
            assert!(output[i] > output[i-1],
                    "output[{}] = {}, output[{}] = {}",
                    i, output[i], i-1, output[i-1]);
        }
    }

    #[test]
    fn test_lag_state_persistence() {
        // Test that lag state persists across multiple process_block calls
        let mut lag = LagNode::new(0, 1);

        // First block: start approaching 1.0
        let input1 = vec![1.0; 5];
        let lag_time1 = vec![0.1; 5]; // 100ms
        let inputs1 = vec![input1.as_slice(), lag_time1.as_slice()];
        let mut output1 = vec![0.0; 5];
        let context = create_context(5);

        lag.process_block(&inputs1, &mut output1, 44100.0, &context);

        let end_of_block1 = output1[4];
        assert!(end_of_block1 > 0.0);
        assert!(end_of_block1 < 1.0);

        // Second block: continue approaching from where we left off
        let input2 = vec![1.0; 5];
        let lag_time2 = vec![0.1; 5];
        let inputs2 = vec![input2.as_slice(), lag_time2.as_slice()];
        let mut output2 = vec![0.0; 5];

        lag.process_block(&inputs2, &mut output2, 44100.0, &context);

        // Should start where previous block ended and continue rising
        // Due to exponential approach, there will be a small step
        assert!((output2[0] - end_of_block1).abs() < 0.001,
                "output2[0] = {}, end_of_block1 = {}",
                output2[0], end_of_block1);
        assert!(output2[4] > output2[0]);
    }

    #[test]
    fn test_lag_dependencies() {
        let lag = LagNode::new(3, 7);
        let deps = lag.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 3);
        assert_eq!(deps[1], 7);
    }

    #[test]
    fn test_lag_with_constants() {
        let mut lag = LagNode::new(0, 1);

        // Constant input and lag time
        let input = vec![1.0; 20];
        let lag_time = vec![0.01; 20]; // 10ms lag
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; 20];
        let context = create_context(20);

        lag.process_block(&inputs, &mut output, 44100.0, &context);

        // Should exponentially approach 1.0
        // Each sample should be higher than the last
        for i in 1..20 {
            assert!(output[i] >= output[i-1],
                    "output[{}] = {}, output[{}] = {}",
                    i, output[i], i-1, output[i-1]);
        }

        // Should be getting closer to target
        // With 10ms lag time and 20 samples, we're only 20/441 = 4.5% through one time constant
        assert!(output[19] > 0.04, "output[19] = {}", output[19]);
    }

    #[test]
    fn test_lag_reaches_target_eventually() {
        let mut lag = LagNode::new(0, 1);

        // Very long buffer with enough samples to reach target
        // For 10ms lag, need ~5 time constants = 50ms = 2205 samples at 44.1kHz
        let samples = 5000;
        let input = vec![1.0; samples];
        let lag_time = vec![0.01; samples]; // 10ms lag
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; samples];
        let context = create_context(samples);

        lag.process_block(&inputs, &mut output, 44100.0, &context);

        // After 5 time constants (~50ms = 2205 samples), should reach ~99.3% of target
        // After 5000 samples, should be very close to 1.0
        assert!((output[4999] - 1.0).abs() < 0.001, "output[4999] = {}", output[4999]);
    }

    #[test]
    fn test_lag_no_change_instant() {
        let mut lag = LagNode::new(0, 1);
        lag.current_value = 0.5;

        // Input stays at current value
        let input = vec![0.5, 0.5, 0.5, 0.5];
        let lag_time = vec![0.1; 4];
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; 4];
        let context = create_context(4);

        lag.process_block(&inputs, &mut output, 44100.0, &context);

        // No change - should stay at 0.5
        for &val in &output {
            assert!((val - 0.5).abs() < 0.0001);
        }
    }

    #[test]
    fn test_lag_portamento_effect() {
        let mut lag = LagNode::new(0, 1);
        // Pre-initialize to simulate previous state at 220 Hz
        lag.current_value = 220.0;

        // Pitch change: 220 Hz -> 440 Hz (one octave)
        // Simulate portamento/glide effect
        let input = vec![220.0, 220.0, 220.0, 440.0, 440.0, 440.0, 440.0, 440.0];
        let lag_time = vec![0.05; 8]; // 50ms portamento time
        let inputs = vec![input.as_slice(), lag_time.as_slice()];

        let mut output = vec![0.0; 8];
        let context = create_context(8);

        lag.process_block(&inputs, &mut output, 44100.0, &context);

        // Should start near 220 (pre-initialized state)
        assert!((output[0] - 220.0).abs() < 1.0, "output[0] = {}", output[0]);

        // Should gradually move toward 440 after step
        assert!(output[4] > 220.0, "output[4] = {}", output[4]);
        assert!(output[5] > output[4]);
        assert!(output[6] > output[5]);

        // Should not reach 440 immediately (portamento effect)
        assert!(output[7] < 300.0, "output[7] = {}", output[7]);
    }
}
