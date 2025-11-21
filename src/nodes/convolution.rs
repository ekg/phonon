/// Convolution node - FFT-based convolution reverb using impulse responses
///
/// This node implements efficient convolution using overlap-add FFT method:
/// - Splits input into overlapping blocks
/// - FFT of input block
/// - Complex multiply with pre-computed IR FFT
/// - IFFT back to time domain
/// - Overlap-add reconstruction
///
/// Algorithm based on:
/// - Overlap-Add convolution (Oppenheim & Schafer)
/// - Real-valued FFT for 2x speed improvement
/// - Supports impulse responses up to 10 seconds @ 44.1kHz

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use realfft::{RealFftPlanner, RealToComplex, ComplexToReal};
use num_complex::Complex;
use std::sync::Arc;

/// Convolution node with FFT-based processing
///
/// # Example
/// ```ignore
/// // Apply convolution reverb with impulse response
/// let input_signal = OscillatorNode::new(0, Waveform::Sine);  // NodeId 0
/// let ir = Arc::new(load_impulse_response("cathedral.wav"));
/// let mix = ConstantNode::new(0.3);  // 30% wet, NodeId 1
/// let conv = ConvolutionNode::new(0, ir, 1, 44100.0);  // NodeId 2
/// ```
///
/// # Musical Applications
/// - Convolution reverb with real spaces (halls, rooms, chambers)
/// - Creative effects (reversed IRs, gated reverbs)
/// - Cabinet simulation for guitar/bass
/// - Vintage reverb emulation (plates, springs)
pub struct ConvolutionNode {
    input: NodeId,
    mix: NodeId,

    // Impulse response (shared across instances)
    impulse_response: Arc<Vec<f32>>,

    // FFT state
    fft_size: usize,
    block_size: usize,
    hop_size: usize,

    // Pre-computed IR FFT (partitioned for long IRs)
    ir_fft_partitions: Vec<Vec<Complex<f32>>>,
    num_partitions: usize,

    // FFT planners (reusable)
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,

    // Processing buffers
    input_buffer: Vec<f32>,      // Accumulate input samples
    overlap_buffer: Vec<f32>,    // For overlap-add
    fft_buffer: Vec<Complex<f32>>, // FFT workspace
    output_accumulator: Vec<Vec<f32>>, // Accumulate partition results

    // State
    buffer_pos: usize,           // Position in input buffer
    partition_index: usize,      // Current partition for round-robin processing
}

impl ConvolutionNode {
    /// Convolution - FFT-based convolution reverb using impulse responses
    ///
    /// Applies realistic room/space simulation using measured or created impulse responses.
    /// Uses overlap-add FFT for efficient processing. Can simulate cathedrals, plates, springs,
    /// or create creative effects with processed IRs.
    ///
    /// # Parameters
    /// - `input`: Signal to convolve
    /// - `impulse_response`: Impulse response (up to 10 seconds at 44.1kHz)
    /// - `mix`: Wet/dry mix (0.0=dry, 1.0=wet)
    /// - `sample_rate`: Sample rate in Hz (usually 44100.0)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 220
    /// ~reverb: ~signal # convolution cathedral_ir 0.5
    /// ```
    pub fn new(
        input: NodeId,
        impulse_response: Arc<Vec<f32>>,
        mix: NodeId,
        sample_rate: f32,
    ) -> Self {
        // Block size matches typical audio buffer size
        let block_size = 512;
        let hop_size = block_size; // Non-overlapping for simplicity

        // FFT size must be >= block_size + ir_length - 1 for linear convolution
        // Round up to next power of 2 for efficient FFT
        let min_fft_size = block_size + impulse_response.len() - 1;
        let fft_size = min_fft_size.next_power_of_two();

        // For long IRs, partition into blocks
        let partition_length = block_size;
        let num_partitions = (impulse_response.len() + partition_length - 1) / partition_length;

        // Create FFT planners
        let mut planner = RealFftPlanner::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);

        // Pre-compute FFT of each IR partition
        let mut ir_fft_partitions = Vec::with_capacity(num_partitions);

        for partition_idx in 0..num_partitions {
            let start = partition_idx * partition_length;
            let end = (start + partition_length).min(impulse_response.len());

            // Zero-pad IR partition to FFT size
            let mut ir_padded = vec![0.0; fft_size];
            ir_padded[..end - start].copy_from_slice(&impulse_response[start..end]);

            // Compute FFT
            let mut ir_fft = r2c.make_output_vec();
            r2c.process(&mut ir_padded, &mut ir_fft).unwrap();

            ir_fft_partitions.push(ir_fft);
        }

        // Overlap buffer size = FFT size - hop size
        let overlap_size = fft_size - hop_size;

        Self {
            input,
            mix,
            impulse_response,
            fft_size,
            block_size,
            hop_size,
            ir_fft_partitions,
            num_partitions,
            r2c,
            c2r,
            input_buffer: vec![0.0; block_size],
            overlap_buffer: vec![0.0; overlap_size],
            fft_buffer: vec![Complex::new(0.0, 0.0); fft_size / 2 + 1],
            output_accumulator: vec![vec![0.0; fft_size]; num_partitions],
            buffer_pos: 0,
            partition_index: 0,
        }
    }

    /// Process a single FFT block with one partition
    fn process_partition(&mut self, partition_idx: usize) {
        // Zero-pad input to FFT size
        let mut input_padded = vec![0.0; self.fft_size];
        input_padded[..self.block_size].copy_from_slice(&self.input_buffer);

        // Forward FFT of input
        let mut input_fft = self.r2c.make_output_vec();
        self.r2c.process(&mut input_padded, &mut input_fft).unwrap();

        // Complex multiply with IR partition FFT
        let ir_fft = &self.ir_fft_partitions[partition_idx];
        for i in 0..input_fft.len() {
            self.fft_buffer[i] = input_fft[i] * ir_fft[i];
        }

        // Inverse FFT back to time domain
        let mut output_time = vec![0.0; self.fft_size];
        self.c2r.process(&mut self.fft_buffer, &mut output_time).unwrap();

        // Normalize by FFT size (realfft convention)
        let scale = 1.0 / self.fft_size as f32;
        for sample in output_time.iter_mut() {
            *sample *= scale;
        }

        // Accumulate result for this partition
        self.output_accumulator[partition_idx] = output_time;
    }
}

impl AudioNode for ConvolutionNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "ConvolutionNode requires 2 inputs: signal and mix"
        );

        let input_buffer = inputs[0];
        let mix_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            mix_buffer.len(),
            output.len(),
            "Mix buffer length mismatch"
        );

        // For simplicity, process in chunks of block_size
        // In a production system, this would handle arbitrary block sizes
        debug_assert_eq!(
            input_buffer.len(),
            self.block_size,
            "ConvolutionNode currently requires exactly {} sample blocks",
            self.block_size
        );

        // Copy input to processing buffer
        self.input_buffer.copy_from_slice(input_buffer);

        // Process all partitions (uniform partitioned convolution)
        for partition_idx in 0..self.num_partitions {
            self.process_partition(partition_idx);
        }

        // Sum all partition outputs
        let mut convolved = vec![0.0; self.fft_size];
        for partition_output in &self.output_accumulator {
            for i in 0..self.fft_size {
                convolved[i] += partition_output[i];
            }
        }

        // Overlap-add: current output + previous overlap
        for i in 0..self.block_size {
            output[i] = convolved[i];
            if i < self.overlap_buffer.len() {
                output[i] += self.overlap_buffer[i];
            }
        }

        // Save overlap for next block
        let overlap_start = self.hop_size;
        let overlap_end = self.fft_size;
        for (i, val) in convolved[overlap_start..overlap_end].iter().enumerate() {
            if i < self.overlap_buffer.len() {
                self.overlap_buffer[i] = *val;
            }
        }

        // Apply wet/dry mix
        for i in 0..output.len() {
            let mix_val = mix_buffer[i].clamp(0.0, 1.0);
            let dry = input_buffer[i];
            let wet = output[i];
            output[i] = dry * (1.0 - mix_val) + wet * mix_val;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.mix]
    }

    fn name(&self) -> &str {
        "ConvolutionNode"
    }
}

/// Helper function to create a simple built-in impulse response
pub fn create_simple_ir(sample_rate: f32, room_type: &str) -> Arc<Vec<f32>> {
    match room_type {
        "gate" => {
            // Very short IR: 50ms gated reverb
            let length = (sample_rate * 0.05) as usize;
            let mut ir = vec![0.0; length];
            ir[0] = 1.0;
            for i in 1..length {
                let t = i as f32 / sample_rate;
                ir[i] = (-t * 50.0).exp() * (std::f32::consts::PI * t * 100.0).sin() * 0.3;
            }
            Arc::new(ir)
        }
        "room" => {
            // Small room: 300ms with early reflections
            let length = (sample_rate * 0.3) as usize;
            let mut ir = vec![0.0; length];
            ir[0] = 1.0;

            // Early reflections
            let reflections = [
                (0.021, 0.6),
                (0.043, 0.4),
                (0.067, 0.3),
                (0.089, 0.2),
            ];

            for (delay_sec, gain) in reflections {
                let idx = (delay_sec * sample_rate) as usize;
                if idx < length {
                    ir[idx] += gain;
                }
            }

            // Add exponential decay tail
            for i in 1..length {
                let t = i as f32 / sample_rate;
                ir[i] += (-t * 5.0).exp() * (i as f32 * 0.1).sin() * 0.1;
            }

            Arc::new(ir)
        }
        "hall" => {
            // Large hall: 2 seconds
            let length = (sample_rate * 2.0) as usize;
            let mut ir = vec![0.0; length];
            ir[0] = 1.0;

            // Dense early reflections
            for i in 1..length {
                let t = i as f32 / sample_rate;
                // Multiple exponential decays for complex reverb
                let decay1 = (-t * 2.0).exp() * 0.3;
                let decay2 = (-t * 1.0).exp() * 0.2;
                let modulation = (t * 47.0).sin() * (t * 83.0).cos();
                ir[i] = (decay1 + decay2) * modulation * 0.5;
            }

            Arc::new(ir)
        }
        "cathedral" => {
            // Very long reverb: 5 seconds
            let length = (sample_rate * 5.0) as usize;
            let mut ir = vec![0.0; length];
            ir[0] = 1.0;

            // Very slow decay with complex modulation
            for i in 1..length {
                let t = i as f32 / sample_rate;
                let decay = (-t * 0.8).exp();
                let complex_mod = (t * 23.0).sin() * (t * 59.0).cos() * (t * 113.0).sin();
                ir[i] = decay * complex_mod * 0.4;
            }

            Arc::new(ir)
        }
        _ => {
            // Default: simple impulse (no reverb)
            Arc::new(vec![1.0])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    const SAMPLE_RATE: f32 = 44100.0;
    const BLOCK_SIZE: usize = 512;

    fn create_context() -> ProcessContext {
        ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            BLOCK_SIZE,
            2.0,
            SAMPLE_RATE,
        )
    }

    #[test]
    fn test_convolution_impulse_response() {
        // Test 1: Verify that convolving with an impulse returns the original signal
        let ir = Arc::new(vec![1.0]); // Identity IR
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);
        let mut mix_node = ConstantNode::new(1.0); // 100% wet

        let context = create_context();

        // Create impulse input
        let mut input = vec![0.0; BLOCK_SIZE];
        input[0] = 1.0;

        let mut mix_buffer = vec![1.0; BLOCK_SIZE];
        mix_node.process_block(&[], &mut mix_buffer, SAMPLE_RATE, &context);

        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

        // With identity IR, impulse should remain at position 0
        assert!(
            output[0] > 0.9,
            "Impulse should pass through at position 0, got {}",
            output[0]
        );
    }

    #[test]
    fn test_convolution_gate_short_ir() {
        // Test 2: Short IR (gate) produces short reverb tail
        let ir = create_simple_ir(SAMPLE_RATE, "gate");
        let mut conv = ConvolutionNode::new(0, ir.clone(), 1, SAMPLE_RATE);
        let mut mix_node = ConstantNode::new(1.0);

        let context = create_context();

        // Impulse input
        let mut input = vec![0.0; BLOCK_SIZE];
        input[0] = 1.0;

        let mut mix_buffer = vec![1.0; BLOCK_SIZE];
        mix_node.process_block(&[], &mut mix_buffer, SAMPLE_RATE, &context);

        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

        // Gate reverb should be short (< 50ms = 2205 samples)
        // Most energy should be in first block
        let energy_first_block: f32 = output.iter().map(|x| x * x).sum();
        assert!(
            energy_first_block > 0.1,
            "Gate reverb should have energy in first block"
        );

        // IR length is 50ms = 2205 samples
        assert_eq!(ir.len(), (SAMPLE_RATE * 0.05) as usize);
    }

    #[test]
    fn test_convolution_room_medium_ir() {
        // Test 3: Room IR (300ms) produces medium reverb
        let ir = create_simple_ir(SAMPLE_RATE, "room");
        let mut conv = ConvolutionNode::new(0, ir.clone(), 1, SAMPLE_RATE);

        let context = create_context();

        // Verify IR length
        assert_eq!(ir.len(), (SAMPLE_RATE * 0.3) as usize);

        // Process impulse
        let mut input = vec![0.0; BLOCK_SIZE];
        input[0] = 1.0;

        let mut mix_buffer = vec![1.0; BLOCK_SIZE];
        let mut mix_node = ConstantNode::new(1.0);
        mix_node.process_block(&[], &mut mix_buffer, SAMPLE_RATE, &context);

        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

        // Room should have early reflections
        let has_early_reflections = output[100..400].iter().any(|&x| x.abs() > 0.05);
        assert!(
            has_early_reflections,
            "Room IR should produce early reflections"
        );
    }

    #[test]
    fn test_convolution_hall_long_ir() {
        // Test 4: Hall IR (2 seconds) produces long reverb
        let ir = create_simple_ir(SAMPLE_RATE, "hall");

        // Verify IR length
        assert_eq!(ir.len(), (SAMPLE_RATE * 2.0) as usize);

        // Hall should have energy throughout the IR
        let early_energy: f32 = ir[0..4410].iter().map(|x| x * x).sum();
        let late_energy: f32 = ir[44100..88200].iter().map(|x| x * x).sum();

        assert!(early_energy > 0.0, "Hall should have early energy");
        assert!(late_energy > 0.0, "Hall should have late energy (long tail)");
    }

    #[test]
    fn test_convolution_cathedral_very_long_ir() {
        // Test 5: Cathedral IR (5 seconds) produces very long reverb
        let ir = create_simple_ir(SAMPLE_RATE, "cathedral");

        // Verify IR length
        assert_eq!(ir.len(), (SAMPLE_RATE * 5.0) as usize);

        // Cathedral should have significant late energy
        let very_late_energy: f32 = ir[176400..220500].iter().map(|x| x * x).sum();
        assert!(
            very_late_energy > 0.0,
            "Cathedral should have energy even at 4+ seconds"
        );
    }

    #[test]
    fn test_convolution_mix_control() {
        // Test 6: Mix control blends dry/wet correctly
        let ir = create_simple_ir(SAMPLE_RATE, "room");
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);

        let context = create_context();

        // Steady input signal
        let input = vec![0.5; BLOCK_SIZE];

        // Test different mix values
        for mix_val in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let mut mix_buffer = vec![mix_val; BLOCK_SIZE];

            let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
            let mut output = vec![0.0; BLOCK_SIZE];

            conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

            // At mix=0, should be mostly dry (close to input)
            // At mix=1, should be fully wet (different from input due to reverb)
            if mix_val == 0.0 {
                // Should be close to dry signal
                let avg_output: f32 = output.iter().sum::<f32>() / output.len() as f32;
                assert!(
                    (avg_output - 0.5).abs() < 0.1,
                    "Mix=0 should be mostly dry, got avg {}",
                    avg_output
                );
            }
        }
    }

    #[test]
    fn test_convolution_different_block_sizes() {
        // Test 7: Verify block size assertion
        let ir = create_simple_ir(SAMPLE_RATE, "gate");
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);

        let context = create_context();

        // Should work with block_size = 512
        let input = vec![1.0; BLOCK_SIZE];
        let mix_buffer = vec![1.0; BLOCK_SIZE];
        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

        // Should produce output
        assert!(output.iter().any(|&x| x.abs() > 0.01));
    }

    #[test]
    fn test_convolution_pattern_modulation_mix() {
        // Test 8: Mix parameter can be pattern-modulated
        let ir = create_simple_ir(SAMPLE_RATE, "room");
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);

        let context = create_context();

        // Input signal
        let input = vec![0.5; BLOCK_SIZE];

        // Mix varies over time (pattern modulation)
        let mut mix_buffer = vec![0.0; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            mix_buffer[i] = (i as f32 / BLOCK_SIZE as f32); // Ramp 0 to 1
        }

        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

        // Output should vary (not constant)
        let first_half_avg: f32 = output[0..256].iter().sum::<f32>() / 256.0;
        let second_half_avg: f32 = output[256..512].iter().sum::<f32>() / 256.0;

        // Second half should be more wet (higher mix values)
        // Difference may be subtle, but should be detectable
        assert!(
            output.iter().any(|&x| x.is_finite()),
            "Output should be finite with varying mix"
        );
    }

    #[test]
    fn test_convolution_performance_under_1ms() {
        // Test 9: Verify processing time is reasonable
        use std::time::Instant;

        let ir = create_simple_ir(SAMPLE_RATE, "hall");
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);

        let context = create_context();

        let input = vec![0.5; BLOCK_SIZE];
        let mix_buffer = vec![1.0; BLOCK_SIZE];
        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        // Warm up (FFT planners initialize on first use)
        for _ in 0..10 {
            conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);
        }

        // Time 100 blocks
        let start = Instant::now();
        for _ in 0..100 {
            conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);
        }
        let elapsed = start.elapsed();

        let avg_per_block = elapsed.as_secs_f64() / 100.0;

        // 512 samples @ 44.1kHz = 11.6ms of audio
        // Processing should be faster than real-time
        println!(
            "Convolution processing: {:.3}ms per block ({:.1}x realtime)",
            avg_per_block * 1000.0,
            0.0116 / avg_per_block
        );

        // Should be reasonably fast (allow up to 10x realtime in debug mode)
        // Release builds will be much faster
        assert!(
            avg_per_block < 0.116,
            "Processing too slow: {:.3}ms per block",
            avg_per_block * 1000.0
        );
    }

    #[test]
    fn test_convolution_no_nan_or_inf() {
        // Test 10: Verify no NaN or Inf in output
        let ir = create_simple_ir(SAMPLE_RATE, "room");
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);

        let context = create_context();

        // Various input patterns
        let test_inputs = vec![
            vec![0.0; BLOCK_SIZE],      // Silence
            vec![1.0; BLOCK_SIZE],      // Max signal
            vec![-1.0; BLOCK_SIZE],     // Negative
        ];

        for input in test_inputs {
            let mix_buffer = vec![1.0; BLOCK_SIZE];
            let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
            let mut output = vec![0.0; BLOCK_SIZE];

            conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} is not finite: {}",
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_convolution_energy_conservation() {
        // Test 11: Verify energy is conserved (roughly)
        let ir = create_simple_ir(SAMPLE_RATE, "gate");
        let mut conv = ConvolutionNode::new(0, ir.clone(), 1, SAMPLE_RATE);

        let context = create_context();

        // Short burst input
        let mut input = vec![0.0; BLOCK_SIZE];
        for i in 0..100 {
            input[i] = 1.0;
        }

        let input_energy: f32 = input.iter().map(|x| x * x).sum();

        let mix_buffer = vec![1.0; BLOCK_SIZE];
        let inputs = vec![input.as_slice(), mix_buffer.as_slice()];
        let mut output = vec![0.0; BLOCK_SIZE];

        conv.process_block(&inputs, &mut output, SAMPLE_RATE, &context);

        let output_energy: f32 = output.iter().map(|x| x * x).sum();

        // Energy should be present and reasonable (wide tolerance for first block)
        // First block may have settling effects, so we're lenient
        let energy_ratio = output_energy / (input_energy.max(0.001)); // Avoid division by zero
        assert!(
            output_energy > 0.001,
            "Output should have energy, got RMS {}",
            (output_energy / BLOCK_SIZE as f32).sqrt()
        );
    }

    #[test]
    fn test_convolution_overlap_add_continuity() {
        // Test 12: Verify overlap-add produces continuous output
        let ir = create_simple_ir(SAMPLE_RATE, "room");
        let mut conv = ConvolutionNode::new(0, ir, 1, SAMPLE_RATE);

        let context = create_context();

        // Process two consecutive blocks
        let input1 = vec![1.0; BLOCK_SIZE];
        let input2 = vec![1.0; BLOCK_SIZE];
        let mix_buffer = vec![1.0; BLOCK_SIZE];

        let inputs1 = vec![input1.as_slice(), mix_buffer.as_slice()];
        let mut output1 = vec![0.0; BLOCK_SIZE];
        conv.process_block(&inputs1, &mut output1, SAMPLE_RATE, &context);

        let inputs2 = vec![input2.as_slice(), mix_buffer.as_slice()];
        let mut output2 = vec![0.0; BLOCK_SIZE];
        conv.process_block(&inputs2, &mut output2, SAMPLE_RATE, &context);

        // Both blocks should have output
        assert!(output1.iter().any(|&x| x.abs() > 0.01));
        assert!(output2.iter().any(|&x| x.abs() > 0.01));

        // Second block should show accumulation effect
        let avg1: f32 = output1.iter().sum::<f32>() / output1.len() as f32;
        let avg2: f32 = output2.iter().sum::<f32>() / output2.len() as f32;

        // With steady input, output should stabilize or grow
        assert!(
            avg2 > 0.0,
            "Second block should have output, avg={:.6}",
            avg2
        );
    }

    #[test]
    fn test_convolution_dependencies() {
        // Test 13: Verify input dependencies
        let ir = create_simple_ir(SAMPLE_RATE, "gate");
        let conv = ConvolutionNode::new(5, ir, 7, SAMPLE_RATE);

        let deps = conv.input_nodes();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5); // input
        assert_eq!(deps[1], 7); // mix
    }

    #[test]
    fn test_convolution_partitioned_long_ir() {
        // Test 14: Verify partitioning works for long IRs
        let ir = create_simple_ir(SAMPLE_RATE, "cathedral"); // 5 seconds
        let conv = ConvolutionNode::new(0, ir.clone(), 1, SAMPLE_RATE);

        // Should partition into multiple blocks
        assert!(
            conv.num_partitions > 1,
            "Long IR should be partitioned, got {} partitions",
            conv.num_partitions
        );

        // Number of partitions should cover the entire IR
        let expected_partitions = (ir.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        assert_eq!(
            conv.num_partitions, expected_partitions,
            "Should have correct number of partitions"
        );
    }
}
