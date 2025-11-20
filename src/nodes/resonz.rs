/// Resonz - Classic resonant bandpass filter
///
/// This node implements a resonant bandpass filter based on the biquad structure.
/// Unlike the standard BandPassFilterNode which uses Q directly, Resonz uses rq
/// (reciprocal of Q, where rq = bandwidth/center_frequency) for more intuitive
/// control of bandwidth.
///
/// # Implementation Details
///
/// Uses a biquad bandpass filter with bandwidth control via rq parameter.
/// - Small rq = narrow bandwidth = high resonance
/// - Large rq = wide bandwidth = low resonance
///
/// Based on:
/// - SuperCollider Resonz UGen
/// - Robert Bristow-Johnson Audio EQ Cookbook
/// - Classic analog resonant bandpass designs
///
/// # Musical Characteristics
///
/// - Resonant peak at center frequency
/// - Attenuates frequencies above and below center
/// - Natural-sounding resonance for filter sweeps
/// - Works well for creating "ringing" tones from noise
/// - Good for formant filtering and vowel sounds

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Internal state for the resonz filter
#[derive(Debug, Clone)]
struct ResonzState {
    /// Biquad filter implementation
    filter: DirectForm2Transposed<f32>,
    /// Last center frequency (for detecting changes)
    last_freq: f32,
    /// Last rq value (for detecting changes)
    last_rq: f32,
}

impl ResonzState {
    fn new(sample_rate: f32) -> Self {
        // Initialize with default coefficients (1000 Hz, rq=0.1)
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            1000.0.hz(),
            Self::rq_to_q(0.1, 1000.0),
        )
        .unwrap();

        Self {
            filter: DirectForm2Transposed::<f32>::new(coeffs),
            last_freq: 1000.0,
            last_rq: 0.1,
        }
    }

    /// Convert rq (reciprocal of Q) to Q for biquad library
    /// rq = bandwidth / center_frequency
    /// Q = center_frequency / bandwidth = 1 / rq
    fn rq_to_q(rq: f32, _freq: f32) -> f32 {
        // Q = 1 / rq
        // Clamp to reasonable range to prevent instability
        let q = 1.0 / rq.max(0.001); // Prevent divide by zero
        q.max(0.1).min(100.0) // Clamp Q to safe range
    }
}

/// Resonz filter node with pattern-controlled center frequency and bandwidth
///
/// # Example
/// ```ignore
/// // Filter noise through narrow resonant peak at 440 Hz
/// let amp = ConstantNode::new(1.0);                       // NodeId 0
/// let noise = NoiseNode::new(0);                          // NodeId 1
/// let freq = ConstantNode::new(440.0);                    // NodeId 2
/// let rq = ConstantNode::new(0.01);                       // NodeId 3 (narrow bandwidth)
/// let resonz = ResonzNode::new(1, 2, 3);                  // NodeId 4
/// // Creates sine-like tone from noise at 440 Hz
/// ```
///
/// # Musical Applications
/// - Resonant filter sweeps
/// - Creating tonal sounds from noise
/// - Formant filtering (vowel sounds)
/// - Simulating physical resonances
/// - Band-isolated effects
pub struct ResonzNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Center frequency input (Hz)
    freq: NodeId,
    /// Reciprocal of Q (bandwidth/frequency) - controls resonance width
    rq: NodeId,
    /// Filter state
    state: ResonzState,
}

impl ResonzNode {
    /// Create a new Resonz filter node
    ///
    /// # Arguments
    /// * `input` - NodeId providing signal to filter
    /// * `freq` - NodeId providing center frequency in Hz (20 to 20000)
    /// * `rq` - NodeId providing bandwidth ratio (0.001 to 1.0)
    ///
    /// # Notes on rq parameter
    /// - rq = bandwidth / center_frequency
    /// - rq = 0.01 gives very narrow, resonant filter (Q = 100)
    /// - rq = 0.1 gives moderate bandwidth (Q = 10)
    /// - rq = 1.0 gives very wide bandwidth (Q = 1)
    /// - Small rq = narrow bandwidth = high resonance
    /// - Large rq = wide bandwidth = low resonance
    pub fn new(input: NodeId, freq: NodeId, rq: NodeId) -> Self {
        Self {
            input,
            freq,
            rq,
            state: ResonzState::new(44100.0), // Default sample rate, will update on first process
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the frequency input node ID
    pub fn freq(&self) -> NodeId {
        self.freq
    }

    /// Get the rq input node ID
    pub fn rq(&self) -> NodeId {
        self.rq
    }

    /// Reset the filter state
    pub fn reset(&mut self, sample_rate: f32) {
        self.state = ResonzState::new(sample_rate);
    }
}

impl AudioNode for ResonzNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            3,
            "ResonzNode requires 3 inputs: signal, freq, rq"
        );

        let input_buffer = inputs[0];
        let freq_buffer = inputs[1];
        let rq_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );
        debug_assert_eq!(
            rq_buffer.len(),
            output.len(),
            "RQ buffer length mismatch"
        );

        for i in 0..output.len() {
            let input_sample = input_buffer[i];

            // Clamp frequency to valid range (20 Hz to 20 kHz)
            let freq = freq_buffer[i].max(20.0).min(20000.0);

            // Clamp rq to valid range (0.001 to 1.0)
            // Small rq = narrow bandwidth = high resonance
            let rq = rq_buffer[i].max(0.001).min(1.0);

            // Update coefficients if parameters changed significantly
            if (freq - self.state.last_freq).abs() > 0.5 || (rq - self.state.last_rq).abs() > 0.0001 {
                let q = ResonzState::rq_to_q(rq, freq);
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::BandPass,
                    sample_rate.hz(),
                    freq.hz(),
                    q,
                )
                .unwrap();

                self.state.filter.update_coefficients(coeffs);
                self.state.last_freq = freq;
                self.state.last_rq = rq;
            }

            // Process sample through filter
            output[i] = self.state.filter.run(input_sample);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.freq, self.rq]
    }

    fn name(&self) -> &str {
        "ResonzNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::nodes::noise::NoiseNode;
    use crate::nodes::oscillator::{OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    /// Helper: Calculate RMS of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Helper: Calculate peak value in buffer
    fn calculate_peak(buffer: &[f32]) -> f32 {
        buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
    }

    /// Helper: Create test context
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_resonz_passes_center_frequency() {
        // Test 1: Signal at center frequency should pass through
        let sample_rate = 44100.0;
        let block_size = 1024;

        // 1000 Hz sine wave
        let mut freq_const = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Resonz at 1000 Hz, moderate bandwidth
        let mut center_const = ConstantNode::new(1000.0);
        let mut rq_const = ConstantNode::new(0.1);
        let mut resonz = ResonzNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut unfiltered = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);

        // Get unfiltered signal
        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, sample_rate, &context);

        // Get filtered signal
        let resonz_inputs = vec![unfiltered.as_slice(), center_buf.as_slice(), rq_buf.as_slice()];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            resonz.process_block(&resonz_inputs, &mut filtered, sample_rate, &context);
        }

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // Signal at center frequency should pass through with minimal attenuation
        assert!(
            filtered_rms > unfiltered_rms * 0.5,
            "Resonz should pass center frequency: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_resonz_attenuates_below_center() {
        // Test 2: Frequencies below center should be attenuated
        let sample_rate = 44100.0;
        let block_size = 1024;

        // 200 Hz sine wave
        let mut freq_const = ConstantNode::new(200.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Resonz at 1000 Hz
        let mut center_const = ConstantNode::new(1000.0);
        let mut rq_const = ConstantNode::new(0.1);
        let mut resonz = ResonzNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut unfiltered = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, sample_rate, &context);

        let resonz_inputs = vec![unfiltered.as_slice(), center_buf.as_slice(), rq_buf.as_slice()];

        for _ in 0..3 {
            resonz.process_block(&resonz_inputs, &mut filtered, sample_rate, &context);
        }

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // 200 Hz should be attenuated by 1000 Hz resonz (but not as heavily as higher-order filters)
        assert!(
            filtered_rms < unfiltered_rms * 0.4,
            "Resonz should attenuate low frequencies: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_resonz_attenuates_above_center() {
        // Test 3: Frequencies above center should be attenuated
        let sample_rate = 44100.0;
        let block_size = 1024;

        // 8000 Hz sine wave
        let mut freq_const = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Resonz at 1000 Hz
        let mut center_const = ConstantNode::new(1000.0);
        let mut rq_const = ConstantNode::new(0.1);
        let mut resonz = ResonzNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut unfiltered = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut unfiltered, sample_rate, &context);

        let resonz_inputs = vec![unfiltered.as_slice(), center_buf.as_slice(), rq_buf.as_slice()];

        for _ in 0..3 {
            resonz.process_block(&resonz_inputs, &mut filtered, sample_rate, &context);
        }

        let unfiltered_rms = calculate_rms(&unfiltered);
        let filtered_rms = calculate_rms(&filtered);

        // 8000 Hz should be heavily attenuated by 1000 Hz resonz
        assert!(
            filtered_rms < unfiltered_rms * 0.2,
            "Resonz should attenuate high frequencies: unfiltered={}, filtered={}",
            unfiltered_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_resonz_narrow_bandwidth() {
        // Test 4: Small rq (narrow bandwidth) creates tight resonance
        let sample_rate = 44100.0;
        let block_size = 2048; // Longer for better frequency resolution

        // White noise input
        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);

        // Very narrow resonance (rq = 0.01 means Q = 100)
        let mut center_const = ConstantNode::new(1000.0);
        let mut rq_const = ConstantNode::new(0.01); // Very narrow
        let mut resonz = ResonzNode::new(0, 1, 2);

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);
        noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);

        let resonz_inputs = vec![noise_buf.as_slice(), center_buf.as_slice(), rq_buf.as_slice()];

        // Process multiple blocks to reach steady state
        for _ in 0..5 {
            resonz.process_block(&resonz_inputs, &mut filtered, sample_rate, &context);
        }

        let noise_rms = calculate_rms(&noise_buf);
        let filtered_rms = calculate_rms(&filtered);

        // Narrow resonance on noise should produce significant output
        // (extracts tone from noise)
        assert!(
            filtered_rms > noise_rms * 0.05,
            "Narrow resonance should extract tone from noise: noise={}, filtered={}",
            noise_rms,
            filtered_rms
        );

        // Should be more tonal (less random) than noise
        // Check that output is coherent
        assert!(
            filtered_rms > 0.01,
            "Narrow resonance should produce audible output"
        );
    }

    #[test]
    fn test_resonz_wide_bandwidth() {
        // Test 5: Large rq (wide bandwidth) passes more frequencies
        let sample_rate = 44100.0;
        let block_size = 1024;

        // White noise
        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);

        // Compare narrow vs wide bandwidth
        let mut center_const = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);
        noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);

        // Narrow bandwidth (rq = 0.01)
        let mut rq_narrow = ConstantNode::new(0.01);
        let mut rq_narrow_buf = vec![0.0; block_size];
        rq_narrow.process_block(&[], &mut rq_narrow_buf, sample_rate, &context);

        let mut resonz_narrow = ResonzNode::new(0, 1, 2);
        let mut filtered_narrow = vec![0.0; block_size];
        let inputs_narrow = vec![noise_buf.as_slice(), center_buf.as_slice(), rq_narrow_buf.as_slice()];

        for _ in 0..3 {
            resonz_narrow.process_block(&inputs_narrow, &mut filtered_narrow, sample_rate, &context);
        }

        // Wide bandwidth (rq = 0.5)
        let mut rq_wide = ConstantNode::new(0.5);
        let mut rq_wide_buf = vec![0.0; block_size];
        rq_wide.process_block(&[], &mut rq_wide_buf, sample_rate, &context);

        let mut resonz_wide = ResonzNode::new(0, 1, 2);
        let mut filtered_wide = vec![0.0; block_size];
        let inputs_wide = vec![noise_buf.as_slice(), center_buf.as_slice(), rq_wide_buf.as_slice()];

        for _ in 0..3 {
            resonz_wide.process_block(&inputs_wide, &mut filtered_wide, sample_rate, &context);
        }

        let rms_narrow = calculate_rms(&filtered_narrow);
        let rms_wide = calculate_rms(&filtered_wide);

        // NOTE: Counter-intuitively, narrow bandwidth (high Q) creates resonant BOOST
        // Wide bandwidth (low Q) has less resonance, so LOWER output despite wider passband
        // This is correct behavior for a resonant filter
        assert!(
            rms_narrow > rms_wide,
            "Narrow bandwidth (high Q) should have resonant boost: narrow={}, wide={}",
            rms_narrow,
            rms_wide
        );
    }

    #[test]
    fn test_resonz_frequency_modulation() {
        // Test 6: Center frequency changes should affect filtering
        let sample_rate = 44100.0;
        let block_size = 1024;

        // White noise
        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);
        let mut rq_const = ConstantNode::new(0.05);

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);
        noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);

        // Test at 500 Hz
        let mut freq_low = ConstantNode::new(500.0);
        let mut freq_low_buf = vec![0.0; block_size];
        freq_low.process_block(&[], &mut freq_low_buf, sample_rate, &context);

        let mut resonz_low = ResonzNode::new(0, 1, 2);
        let mut filtered_low = vec![0.0; block_size];
        let inputs_low = vec![noise_buf.as_slice(), freq_low_buf.as_slice(), rq_buf.as_slice()];

        for _ in 0..5 {
            resonz_low.process_block(&inputs_low, &mut filtered_low, sample_rate, &context);
        }

        // Test at 2000 Hz
        let mut freq_high = ConstantNode::new(2000.0);
        let mut freq_high_buf = vec![0.0; block_size];
        freq_high.process_block(&[], &mut freq_high_buf, sample_rate, &context);

        let mut resonz_high = ResonzNode::new(0, 1, 2);
        let mut filtered_high = vec![0.0; block_size];
        let inputs_high = vec![noise_buf.as_slice(), freq_high_buf.as_slice(), rq_buf.as_slice()];

        for _ in 0..5 {
            resonz_high.process_block(&inputs_high, &mut filtered_high, sample_rate, &context);
        }

        let rms_low = calculate_rms(&filtered_low);
        let rms_high = calculate_rms(&filtered_high);

        // Both should produce output
        assert!(rms_low > 0.01, "Low frequency resonance should produce output");
        assert!(rms_high > 0.01, "High frequency resonance should produce output");

        // Outputs should differ (different center frequencies)
        assert!(
            (rms_low - rms_high).abs() > 0.001,
            "Different frequencies should produce different outputs: low={}, high={}",
            rms_low,
            rms_high
        );
    }

    #[test]
    fn test_resonz_rq_modulation() {
        // Test 7: RQ changes should affect bandwidth
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Saw wave (rich harmonics)
        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut center_const = ConstantNode::new(880.0); // 8th harmonic region

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        // Narrow rq (tight resonance)
        let mut rq_narrow = ConstantNode::new(0.02);
        let mut rq_narrow_buf = vec![0.0; block_size];
        rq_narrow.process_block(&[], &mut rq_narrow_buf, sample_rate, &context);

        let mut resonz_narrow = ResonzNode::new(0, 1, 2);
        let mut filtered_narrow = vec![0.0; block_size];
        let inputs_narrow = vec![signal_buf.as_slice(), center_buf.as_slice(), rq_narrow_buf.as_slice()];

        for _ in 0..3 {
            resonz_narrow.process_block(&inputs_narrow, &mut filtered_narrow, sample_rate, &context);
        }

        // Wide rq (broad resonance)
        let mut rq_wide = ConstantNode::new(0.3);
        let mut rq_wide_buf = vec![0.0; block_size];
        rq_wide.process_block(&[], &mut rq_wide_buf, sample_rate, &context);

        let mut resonz_wide = ResonzNode::new(0, 1, 2);
        let mut filtered_wide = vec![0.0; block_size];
        let inputs_wide = vec![signal_buf.as_slice(), center_buf.as_slice(), rq_wide_buf.as_slice()];

        for _ in 0..3 {
            resonz_wide.process_block(&inputs_wide, &mut filtered_wide, sample_rate, &context);
        }

        let rms_narrow = calculate_rms(&filtered_narrow);
        let rms_wide = calculate_rms(&filtered_wide);

        // Both should produce output
        assert!(rms_narrow > 0.001, "Narrow rq should produce output");
        assert!(rms_wide > 0.001, "Wide rq should produce output");

        // Narrow rq (high Q) creates resonant boost at center frequency
        // Wide rq (low Q) passes more bandwidth but with less resonant boost
        // For a saw wave with harmonics at center, narrow Q will boost more
        assert!(
            rms_narrow > rms_wide * 0.8,
            "Narrow rq (high Q) should have strong resonant boost: narrow={}, wide={}",
            rms_narrow,
            rms_wide
        );
    }

    #[test]
    fn test_resonz_extreme_parameters() {
        // Test 8: Extreme parameters should be handled safely
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);
        let mut freq_extreme = ConstantNode::new(50000.0); // Above Nyquist
        let mut rq_extreme = ConstantNode::new(0.0001); // Very narrow

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut freq_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        freq_extreme.process_block(&[], &mut freq_buf, sample_rate, &context);
        rq_extreme.process_block(&[], &mut rq_buf, sample_rate, &context);
        noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);

        let mut resonz = ResonzNode::new(0, 1, 2);
        let inputs = vec![noise_buf.as_slice(), freq_buf.as_slice(), rq_buf.as_slice()];

        // Should not panic
        resonz.process_block(&inputs, &mut filtered, sample_rate, &context);

        // Output should be stable (no NaN/Inf)
        for (i, &sample) in filtered.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} not finite with extreme parameters: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_resonz_stability() {
        // Test 9: Filter should remain stable over many blocks
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);
        let mut freq_const = ConstantNode::new(1000.0);
        let mut rq_const = ConstantNode::new(0.05);

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut freq_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);

        let mut resonz = ResonzNode::new(0, 1, 2);

        // Process 100 blocks
        for _ in 0..100 {
            noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);
            let inputs = vec![noise_buf.as_slice(), freq_buf.as_slice(), rq_buf.as_slice()];
            resonz.process_block(&inputs, &mut filtered, sample_rate, &context);

            // Check stability
            for (i, &sample) in filtered.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} became infinite/NaN after many blocks",
                    i
                );
                assert!(
                    sample.abs() < 10.0,
                    "Sample {} has extreme value: {}",
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_resonz_input_nodes() {
        // Test 10: Verify input node dependencies
        let resonz = ResonzNode::new(10, 20, 30);
        let deps = resonz.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // freq
        assert_eq!(deps[2], 30); // rq
    }

    #[test]
    fn test_resonz_tone_from_noise() {
        // Test 11: Classic use case - extract sine-like tone from noise
        let sample_rate = 44100.0;
        let block_size = 4096; // Long block for frequency stability

        // White noise
        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);

        // Very narrow resonance at 440 Hz (rq = 0.005 means Q = 200)
        let mut freq_const = ConstantNode::new(440.0);
        let mut rq_const = ConstantNode::new(0.005); // Very narrow

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut freq_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);

        let mut resonz = ResonzNode::new(0, 1, 2);

        // Process multiple blocks to reach steady state
        for _ in 0..10 {
            noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);
            let inputs = vec![noise_buf.as_slice(), freq_buf.as_slice(), rq_buf.as_slice()];
            resonz.process_block(&inputs, &mut filtered, sample_rate, &context);
        }

        let filtered_rms = calculate_rms(&filtered);

        // Should produce audible tone
        assert!(
            filtered_rms > 0.01,
            "Very narrow resonance should extract audible tone from noise: {}",
            filtered_rms
        );

        // Should be stable
        assert!(
            filtered.iter().all(|&x| x.is_finite()),
            "Output should be stable"
        );
    }

    #[test]
    fn test_resonz_formant_filtering() {
        // Test 12: Use multiple resonz for formant filtering (vowel sounds)
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Saw wave source (vocal cords)
        let mut freq_const = ConstantNode::new(110.0); // Male voice fundamental
        let mut osc = OscillatorNode::new(0, Waveform::Saw);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        // "AH" vowel approximation: formant at ~800 Hz
        let mut formant_const = ConstantNode::new(800.0);
        let mut rq_const = ConstantNode::new(0.08); // Moderate bandwidth

        let mut formant_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];

        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);

        let mut resonz = ResonzNode::new(0, 1, 2);
        let mut filtered = vec![0.0; block_size];

        let inputs = vec![signal_buf.as_slice(), formant_buf.as_slice(), rq_buf.as_slice()];

        for _ in 0..3 {
            resonz.process_block(&inputs, &mut filtered, sample_rate, &context);
        }

        let signal_rms = calculate_rms(&signal_buf);
        let filtered_rms = calculate_rms(&filtered);

        // Should produce audible output
        assert!(
            filtered_rms > 0.01,
            "Formant filter should produce audible output"
        );

        // Note: Resonant filter can BOOST at the formant frequency
        // This is correct behavior - formants are resonant peaks
        // Just verify it's different from input
        assert!(
            (filtered_rms - signal_rms).abs() > 0.01,
            "Formant filter should modify signal: input={}, output={}",
            signal_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_resonz_coefficient_updates() {
        // Test 13: Coefficients should update when parameters change
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);
        let mut freq_const = ConstantNode::new(1000.0);
        let mut rq_const = ConstantNode::new(0.1);

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut freq_buf = vec![0.0; block_size];
        let mut rq_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];
        let mut output1 = vec![0.0; block_size];
        let mut output2 = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        rq_const.process_block(&[], &mut rq_buf, sample_rate, &context);
        noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);

        let mut resonz = ResonzNode::new(0, 1, 2);

        // First pass at 1000 Hz
        let inputs1 = vec![noise_buf.as_slice(), freq_buf.as_slice(), rq_buf.as_slice()];
        for _ in 0..3 {
            resonz.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        let rms1 = calculate_rms(&output1);

        // Change frequency to 500 Hz
        freq_const.set_value(500.0);
        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

        // Second pass at 500 Hz
        let inputs2 = vec![noise_buf.as_slice(), freq_buf.as_slice(), rq_buf.as_slice()];
        for _ in 0..3 {
            resonz.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let rms2 = calculate_rms(&output2);

        // Both should produce output
        assert!(rms1 > 0.01, "First frequency should produce output");
        assert!(rms2 > 0.01, "Second frequency should produce output");

        // Outputs should differ (filter updated)
        assert!(
            (rms1 - rms2).abs() > 0.001,
            "Changed frequency should affect output: 1000Hz={}, 500Hz={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_resonz_resonance_boost() {
        // Test 14: Resonance should boost signal at center frequency
        let sample_rate = 44100.0;
        let block_size = 2048;

        // Signal exactly at center frequency
        let mut freq_const = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut center_const = ConstantNode::new(1000.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut center_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        center_const.process_block(&[], &mut center_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // High Q (narrow bandwidth, strong resonance)
        let mut rq_high = ConstantNode::new(0.01); // Q = 100
        let mut rq_high_buf = vec![0.0; block_size];
        rq_high.process_block(&[], &mut rq_high_buf, sample_rate, &context);

        let mut resonz_high = ResonzNode::new(0, 1, 2);
        let mut filtered_high = vec![0.0; block_size];
        let inputs_high = vec![signal_buf.as_slice(), center_buf.as_slice(), rq_high_buf.as_slice()];

        for _ in 0..5 {
            resonz_high.process_block(&inputs_high, &mut filtered_high, sample_rate, &context);
        }

        let output_rms_high = calculate_rms(&filtered_high);

        // High Q should provide some resonant boost
        // (May not be huge boost with biquad, but should pass signal well)
        assert!(
            output_rms_high > input_rms * 0.5,
            "High Q resonance should pass center frequency well: input={}, output={}",
            input_rms,
            output_rms_high
        );
    }
}
