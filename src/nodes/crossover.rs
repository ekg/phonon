/// Crossover filter nodes for multi-band processing
///
/// Implements Linkwitz-Riley 24dB/oct crossover by cascading two 12dB/oct
/// Butterworth filters. This provides flat frequency response when bands
/// are summed and good phase coherency at crossover points.
///
/// Three separate nodes are provided:
/// - CrossoverLowNode: Extracts low frequencies
/// - CrossoverMidNode: Extracts mid frequencies
/// - CrossoverHighNode: Extracts high frequencies
///
/// # Theory
///
/// A Linkwitz-Riley crossover is designed so that when the outputs of adjacent
/// bands are summed, the result has flat magnitude and phase response (all-pass).
/// This is achieved by cascading two Butterworth filters of the same order.
///
/// For a 24dB/oct LR crossover:
/// - Low band: Two cascaded 12dB/oct lowpass filters at low_freq
/// - Mid band: 12dB/oct highpass at low_freq + 12dB/oct lowpass at high_freq
/// - High band: Two cascaded 12dB/oct highpass filters at high_freq
///
/// # References
///
/// - Siegfried Linkwitz (1976) "Active Crossover Networks for Noncoincident Drivers"
/// - Douglas Self (2009) "Audio Power Amplifier Design Handbook"
/// - https://en.wikipedia.org/wiki/Linkwitz%E2%80%93Riley_filter
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz, Q_BUTTERWORTH_F32};

/// Low band output of a 3-band Linkwitz-Riley crossover
///
/// Uses two cascaded 12dB/oct Butterworth lowpass filters for 24dB/oct rolloff.
///
/// # Example
/// ```ignore
/// // Split signal into low/mid/high bands
/// let signal = OscillatorNode::new(0, Waveform::Saw);      // NodeId 1
/// let low_freq = ConstantNode::new(250.0);                  // NodeId 2
/// let high_freq = ConstantNode::new(2000.0);                // NodeId 3
///
/// let low = CrossoverLowNode::new(1, 2, 3);                 // NodeId 4
/// let mid = CrossoverMidNode::new(1, 2, 3);                 // NodeId 5
/// let high = CrossoverHighNode::new(1, 2, 3);               // NodeId 6
///
/// // Process each band independently
/// let low_compressed = CompressorNode::new(4, ...);
/// let mid_eq = ParametricEQNode::new(5, ...);
/// let high_limited = LimiterNode::new(6, ...);
/// ```
pub struct CrossoverLowNode {
    /// Input signal to be split
    input: NodeId,
    /// Low/mid crossover frequency (Hz)
    low_freq_input: NodeId,
    /// Mid/high crossover frequency (Hz) - not used for low band but needed for API consistency
    high_freq_input: NodeId,
    /// First lowpass filter stage
    filter1: DirectForm2Transposed<f32>,
    /// Second lowpass filter stage (cascade)
    filter2: DirectForm2Transposed<f32>,
    /// Last low frequency value (for change detection)
    last_low_freq: f32,
}

impl CrossoverLowNode {
    /// CrossoverLow - Extracts low frequencies from 3-band Linkwitz-Riley crossover
    ///
    /// Passes frequencies below the low crossover point using cascaded
    /// 24 dB/octave Butterworth lowpass filters.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to split
    /// - `low_freq_input`: NodeId providing low/mid crossover frequency in Hz (default: 250)
    /// - `high_freq_input`: NodeId providing mid/high crossover frequency in Hz (not used for low band)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~low_band: ~signal # crossover_low 250 2000
    /// ```
    pub fn new(input: NodeId, low_freq_input: NodeId, high_freq_input: NodeId) -> Self {
        // Initialize with 250 Hz cutoff, Butterworth Q
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            250.0.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        Self {
            input,
            low_freq_input,
            high_freq_input,
            filter1: DirectForm2Transposed::<f32>::new(coeffs),
            filter2: DirectForm2Transposed::<f32>::new(coeffs),
            last_low_freq: 250.0,
        }
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            self.last_low_freq.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        self.filter1 = DirectForm2Transposed::<f32>::new(coeffs);
        self.filter2 = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for CrossoverLowNode {
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
            "CrossoverLowNode requires 3 inputs: signal, low_freq, high_freq"
        );

        let input_buffer = inputs[0];
        let low_freq_buffer = inputs[1];
        // high_freq_buffer not used for low band

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            let low_freq = low_freq_buffer[i].max(20.0).min(sample_rate * 0.45); // Clamp to valid range

            // Update filter coefficients if frequency changed significantly
            if (low_freq - self.last_low_freq).abs() > 0.1 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::LowPass,
                    sample_rate.hz(),
                    low_freq.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap();
                self.filter1 = DirectForm2Transposed::<f32>::new(coeffs);
                self.filter2 = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_low_freq = low_freq;
            }

            // Apply cascaded lowpass filters (24 dB/oct)
            let stage1 = self.filter1.run(input_buffer[i]);
            output[i] = self.filter2.run(stage1);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.low_freq_input, self.high_freq_input]
    }

    fn name(&self) -> &str {
        "CrossoverLowNode"
    }
}

/// Mid band output of a 3-band Linkwitz-Riley crossover
///
/// Combines a 12dB/oct highpass (at low_freq) with a 12dB/oct lowpass (at high_freq)
/// to create a bandpass characteristic.
///
/// # Example
/// ```ignore
/// // Extract mid frequencies between 250 Hz and 2000 Hz
/// let mid = CrossoverMidNode::new(signal_id, low_freq_id, high_freq_id);
/// ```
pub struct CrossoverMidNode {
    /// Input signal to be split
    input: NodeId,
    /// Low/mid crossover frequency (Hz)
    low_freq_input: NodeId,
    /// Mid/high crossover frequency (Hz)
    high_freq_input: NodeId,
    /// Highpass filter at low_freq (removes lows)
    hp_filter: DirectForm2Transposed<f32>,
    /// Lowpass filter at high_freq (removes highs)
    lp_filter: DirectForm2Transposed<f32>,
    /// Last low frequency value
    last_low_freq: f32,
    /// Last high frequency value
    last_high_freq: f32,
}

impl CrossoverMidNode {
    /// CrossoverMid - Extracts mid frequencies from 3-band Linkwitz-Riley crossover
    ///
    /// Passes frequencies between the low and high crossover points using
    /// Butterworth highpass and lowpass filters in series.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to split
    /// - `low_freq_input`: NodeId providing low/mid crossover frequency in Hz (default: 250)
    /// - `high_freq_input`: NodeId providing mid/high crossover frequency in Hz (default: 2000)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~mid_band: ~signal # crossover_mid 250 2000
    /// ```
    pub fn new(input: NodeId, low_freq_input: NodeId, high_freq_input: NodeId) -> Self {
        // Initialize highpass at 250 Hz
        let hp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            250.0.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        // Initialize lowpass at 2000 Hz
        let lp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            2000.0.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        Self {
            input,
            low_freq_input,
            high_freq_input,
            hp_filter: DirectForm2Transposed::<f32>::new(hp_coeffs),
            lp_filter: DirectForm2Transposed::<f32>::new(lp_coeffs),
            last_low_freq: 250.0,
            last_high_freq: 2000.0,
        }
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        let hp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            self.last_low_freq.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        let lp_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowPass,
            44100.0.hz(),
            self.last_high_freq.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        self.hp_filter = DirectForm2Transposed::<f32>::new(hp_coeffs);
        self.lp_filter = DirectForm2Transposed::<f32>::new(lp_coeffs);
    }
}

impl AudioNode for CrossoverMidNode {
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
            "CrossoverMidNode requires 3 inputs: signal, low_freq, high_freq"
        );

        let input_buffer = inputs[0];
        let low_freq_buffer = inputs[1];
        let high_freq_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            let low_freq = low_freq_buffer[i].max(20.0).min(sample_rate * 0.45);
            let high_freq = high_freq_buffer[i].max(20.0).min(sample_rate * 0.45);

            // Ensure high_freq > low_freq
            let high_freq = high_freq.max(low_freq + 10.0);

            // Update HP filter if low frequency changed
            if (low_freq - self.last_low_freq).abs() > 0.1 {
                let hp_coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::HighPass,
                    sample_rate.hz(),
                    low_freq.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap();
                self.hp_filter = DirectForm2Transposed::<f32>::new(hp_coeffs);
                self.last_low_freq = low_freq;
            }

            // Update LP filter if high frequency changed
            if (high_freq - self.last_high_freq).abs() > 0.1 {
                let lp_coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::LowPass,
                    sample_rate.hz(),
                    high_freq.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap();
                self.lp_filter = DirectForm2Transposed::<f32>::new(lp_coeffs);
                self.last_high_freq = high_freq;
            }

            // Apply highpass then lowpass (creates bandpass)
            let hp_out = self.hp_filter.run(input_buffer[i]);
            output[i] = self.lp_filter.run(hp_out);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.low_freq_input, self.high_freq_input]
    }

    fn name(&self) -> &str {
        "CrossoverMidNode"
    }
}

/// High band output of a 3-band Linkwitz-Riley crossover
///
/// Uses two cascaded 12dB/oct Butterworth highpass filters for 24dB/oct rolloff.
///
/// # Example
/// ```ignore
/// // Extract high frequencies above 2000 Hz
/// let high = CrossoverHighNode::new(signal_id, low_freq_id, high_freq_id);
/// ```
pub struct CrossoverHighNode {
    /// Input signal to be split
    input: NodeId,
    /// Low/mid crossover frequency (Hz) - not used for high band but needed for API consistency
    low_freq_input: NodeId,
    /// Mid/high crossover frequency (Hz)
    high_freq_input: NodeId,
    /// First highpass filter stage
    filter1: DirectForm2Transposed<f32>,
    /// Second highpass filter stage (cascade)
    filter2: DirectForm2Transposed<f32>,
    /// Last high frequency value (for change detection)
    last_high_freq: f32,
}

impl CrossoverHighNode {
    /// CrossoverHigh - Extracts high frequencies from 3-band Linkwitz-Riley crossover
    ///
    /// Passes frequencies above the high crossover point using cascaded
    /// 24 dB/octave Butterworth highpass filters.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to split
    /// - `low_freq_input`: NodeId providing low/mid crossover frequency in Hz (not used for high band)
    /// - `high_freq_input`: NodeId providing mid/high crossover frequency in Hz (default: 2000)
    ///
    /// # Example
    /// ```phonon
    /// ~signal: saw 110
    /// ~high_band: ~signal # crossover_high 250 2000
    /// ```
    pub fn new(input: NodeId, low_freq_input: NodeId, high_freq_input: NodeId) -> Self {
        // Initialize with 2000 Hz cutoff, Butterworth Q
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            2000.0.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        Self {
            input,
            low_freq_input,
            high_freq_input,
            filter1: DirectForm2Transposed::<f32>::new(coeffs),
            filter2: DirectForm2Transposed::<f32>::new(coeffs),
            last_high_freq: 2000.0,
        }
    }

    /// Reset filter state (clear memory)
    pub fn reset(&mut self) {
        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighPass,
            44100.0.hz(),
            self.last_high_freq.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        self.filter1 = DirectForm2Transposed::<f32>::new(coeffs);
        self.filter2 = DirectForm2Transposed::<f32>::new(coeffs);
    }
}

impl AudioNode for CrossoverHighNode {
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
            "CrossoverHighNode requires 3 inputs: signal, low_freq, high_freq"
        );

        let input_buffer = inputs[0];
        // low_freq_buffer not used for high band
        let high_freq_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            let high_freq = high_freq_buffer[i].max(20.0).min(sample_rate * 0.45); // Clamp to valid range

            // Update filter coefficients if frequency changed significantly
            if (high_freq - self.last_high_freq).abs() > 0.1 {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::HighPass,
                    sample_rate.hz(),
                    high_freq.hz(),
                    Q_BUTTERWORTH_F32,
                )
                .unwrap();
                self.filter1 = DirectForm2Transposed::<f32>::new(coeffs);
                self.filter2 = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_high_freq = high_freq;
            }

            // Apply cascaded highpass filters (24 dB/oct)
            let stage1 = self.filter1.run(input_buffer[i]);
            output[i] = self.filter2.run(stage1);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.low_freq_input, self.high_freq_input]
    }

    fn name(&self) -> &str {
        "CrossoverHighNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::{ConstantNode, OscillatorNode, Waveform};
    use crate::pattern::Fraction;

    /// Helper to calculate RMS (root mean square) of a buffer
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Helper to create a test context
    fn test_context() -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0)
    }

    // ===== CrossoverLowNode Tests =====

    #[test]
    fn test_crossover_low_passes_low_frequencies() {
        // 100 Hz sine wave (well below 250 Hz crossover)
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverLowNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 100 Hz should pass through 250 Hz lowpass with minimal attenuation
        assert!(
            output_rms > input_rms * 0.85,
            "Low band should pass 100 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_low_attenuates_high_frequencies() {
        // 2000 Hz sine wave (well above 250 Hz crossover)
        let mut freq_node = ConstantNode::new(2000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverLowNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 2000 Hz should be heavily attenuated by 250 Hz lowpass (24 dB/oct)
        // 3 octaves above = ~72 dB attenuation, but in practice we see ~-33dB due to filter settling
        assert!(
            output_rms < input_rms * 0.03,
            "Low band should heavily attenuate 2000 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_low_frequency_response_at_crossover() {
        // Test response at crossover frequency (250 Hz)
        let mut freq_node = ConstantNode::new(250.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverLowNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // At crossover frequency, Linkwitz-Riley 24dB/oct is -6dB = ~0.5 amplitude
        let ratio = output_rms / input_rms;
        assert!(
            ratio > 0.4 && ratio < 0.6,
            "At crossover (250 Hz), should be ~-6dB: ratio={}",
            ratio
        );
    }

    #[test]
    fn test_crossover_low_dependencies() {
        let crossover = CrossoverLowNode::new(10, 20, 30);
        let deps = crossover.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // signal input
        assert_eq!(deps[1], 20); // low_freq input
        assert_eq!(deps[2], 30); // high_freq input
    }

    #[test]
    fn test_crossover_low_reset() {
        let mut crossover = CrossoverLowNode::new(0, 1, 2);
        crossover.reset();

        // Should not panic and should still work
        let signal = vec![0.5; 512];
        let low_freq_buf = vec![250.0; 512];
        let high_freq_buf = vec![2000.0; 512];
        let mut output = vec![0.0; 512];

        let context = test_context();
        let inputs = vec![
            signal.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        assert!(output.iter().all(|&x| x.is_finite()));
    }

    // ===== CrossoverMidNode Tests =====

    #[test]
    fn test_crossover_mid_passes_mid_frequencies() {
        // 500 Hz sine wave (between 250 Hz and 2000 Hz)
        let mut freq_node = ConstantNode::new(500.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverMidNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 500 Hz should pass through mid band relatively well
        assert!(
            output_rms > input_rms * 0.7,
            "Mid band should pass 500 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_mid_attenuates_low_frequencies() {
        // 50 Hz sine wave (well below 250 Hz)
        let mut freq_node = ConstantNode::new(50.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverMidNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 50 Hz should be heavily attenuated by 250 Hz highpass
        assert!(
            output_rms < input_rms * 0.1,
            "Mid band should attenuate 50 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_mid_attenuates_high_frequencies() {
        // 8000 Hz sine wave (well above 2000 Hz)
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverMidNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 8000 Hz should be heavily attenuated by 2000 Hz lowpass
        assert!(
            output_rms < input_rms * 0.1,
            "Mid band should attenuate 8000 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_mid_dependencies() {
        let crossover = CrossoverMidNode::new(10, 20, 30);
        let deps = crossover.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10);
        assert_eq!(deps[1], 20);
        assert_eq!(deps[2], 30);
    }

    #[test]
    fn test_crossover_mid_reset() {
        let mut crossover = CrossoverMidNode::new(0, 1, 2);
        crossover.reset();

        let signal = vec![0.5; 512];
        let low_freq_buf = vec![250.0; 512];
        let high_freq_buf = vec![2000.0; 512];
        let mut output = vec![0.0; 512];

        let context = test_context();
        let inputs = vec![
            signal.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        assert!(output.iter().all(|&x| x.is_finite()));
    }

    // ===== CrossoverHighNode Tests =====

    #[test]
    fn test_crossover_high_passes_high_frequencies() {
        // 5000 Hz sine wave (well above 2000 Hz crossover)
        let mut freq_node = ConstantNode::new(5000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverHighNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 5000 Hz should pass through 2000 Hz highpass with minimal attenuation
        assert!(
            output_rms > input_rms * 0.85,
            "High band should pass 5000 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_high_attenuates_low_frequencies() {
        // 250 Hz sine wave (well below 2000 Hz crossover)
        let mut freq_node = ConstantNode::new(250.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverHighNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // 250 Hz should be heavily attenuated by 2000 Hz highpass (24 dB/oct)
        // 3 octaves below = ~72 dB attenuation = ~1/4000 ratio
        assert!(
            output_rms < input_rms * 0.01,
            "High band should heavily attenuate 250 Hz: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_crossover_high_frequency_response_at_crossover() {
        // Test response at crossover frequency (2000 Hz)
        let mut freq_node = ConstantNode::new(2000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut crossover = CrossoverHighNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut output = vec![0.0; 512];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        let output_rms = calculate_rms(&output);

        // At crossover frequency, Linkwitz-Riley 24dB/oct is -6dB = ~0.5 amplitude
        let ratio = output_rms / input_rms;
        assert!(
            ratio > 0.4 && ratio < 0.6,
            "At crossover (2000 Hz), should be ~-6dB: ratio={}",
            ratio
        );
    }

    #[test]
    fn test_crossover_high_dependencies() {
        let crossover = CrossoverHighNode::new(10, 20, 30);
        let deps = crossover.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10);
        assert_eq!(deps[1], 20);
        assert_eq!(deps[2], 30);
    }

    #[test]
    fn test_crossover_high_reset() {
        let mut crossover = CrossoverHighNode::new(0, 1, 2);
        crossover.reset();

        let signal = vec![0.5; 512];
        let low_freq_buf = vec![250.0; 512];
        let high_freq_buf = vec![2000.0; 512];
        let mut output = vec![0.0; 512];

        let context = test_context();
        let inputs = vec![
            signal.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        crossover.process_block(&inputs, &mut output, 44100.0, &context);

        assert!(output.iter().all(|&x| x.is_finite()));
    }

    // ===== Phase Coherency Tests =====

    #[test]
    fn test_crossover_sum_approximates_flat_response_low_freq() {
        // Test that low + mid + high ≈ original signal at 100 Hz
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut low_cross = CrossoverLowNode::new(1, 2, 3);
        let mut mid_cross = CrossoverMidNode::new(1, 2, 3);
        let mut high_cross = CrossoverHighNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut low_out = vec![0.0; 512];
        let mut mid_out = vec![0.0; 512];
        let mut high_out = vec![0.0; 512];

        low_cross.process_block(&inputs, &mut low_out, 44100.0, &context);
        mid_cross.process_block(&inputs, &mut mid_out, 44100.0, &context);
        high_cross.process_block(&inputs, &mut high_out, 44100.0, &context);

        // Sum all three bands
        let mut summed = vec![0.0; 512];
        for i in 0..512 {
            summed[i] = low_out[i] + mid_out[i] + high_out[i];
        }

        let summed_rms = calculate_rms(&summed);

        // At 100 Hz (low band dominates), sum should be close to original
        // Allow some variation due to filter phase response and transient settling
        let ratio = summed_rms / input_rms;
        assert!(
            ratio > 0.75 && ratio < 1.25,
            "Sum should approximate flat response at 100 Hz: ratio={}",
            ratio
        );
    }

    #[test]
    fn test_crossover_sum_approximates_flat_response_mid_freq() {
        // Test that low + mid + high ≈ original signal at 1000 Hz
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut low_cross = CrossoverLowNode::new(1, 2, 3);
        let mut mid_cross = CrossoverMidNode::new(1, 2, 3);
        let mut high_cross = CrossoverHighNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut low_out = vec![0.0; 512];
        let mut mid_out = vec![0.0; 512];
        let mut high_out = vec![0.0; 512];

        low_cross.process_block(&inputs, &mut low_out, 44100.0, &context);
        mid_cross.process_block(&inputs, &mut mid_out, 44100.0, &context);
        high_cross.process_block(&inputs, &mut high_out, 44100.0, &context);

        // Sum all three bands
        let mut summed = vec![0.0; 512];
        for i in 0..512 {
            summed[i] = low_out[i] + mid_out[i] + high_out[i];
        }

        let summed_rms = calculate_rms(&summed);

        // At 1000 Hz (mid band), sum should be close to original
        let ratio = summed_rms / input_rms;
        assert!(
            ratio > 0.8 && ratio < 1.2,
            "Sum should approximate flat response at 1000 Hz: ratio={}",
            ratio
        );
    }

    #[test]
    fn test_crossover_sum_approximates_flat_response_high_freq() {
        // Test that low + mid + high ≈ original signal at 5000 Hz
        let mut freq_node = ConstantNode::new(5000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut low_freq = ConstantNode::new(250.0);
        let mut high_freq = ConstantNode::new(2000.0);
        let mut low_cross = CrossoverLowNode::new(1, 2, 3);
        let mut mid_cross = CrossoverMidNode::new(1, 2, 3);
        let mut high_cross = CrossoverHighNode::new(1, 2, 3);

        let context = test_context();

        let mut freq_buf = vec![0.0; 512];
        let mut osc_buf = vec![0.0; 512];
        let mut low_freq_buf = vec![0.0; 512];
        let mut high_freq_buf = vec![0.0; 512];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        let inputs_osc = vec![freq_buf.as_slice()];
        osc.process_block(&inputs_osc, &mut osc_buf, 44100.0, &context);
        low_freq.process_block(&[], &mut low_freq_buf, 44100.0, &context);
        high_freq.process_block(&[], &mut high_freq_buf, 44100.0, &context);

        let input_rms = calculate_rms(&osc_buf);

        let inputs = vec![
            osc_buf.as_slice(),
            low_freq_buf.as_slice(),
            high_freq_buf.as_slice(),
        ];
        let mut low_out = vec![0.0; 512];
        let mut mid_out = vec![0.0; 512];
        let mut high_out = vec![0.0; 512];

        low_cross.process_block(&inputs, &mut low_out, 44100.0, &context);
        mid_cross.process_block(&inputs, &mut mid_out, 44100.0, &context);
        high_cross.process_block(&inputs, &mut high_out, 44100.0, &context);

        // Sum all three bands
        let mut summed = vec![0.0; 512];
        for i in 0..512 {
            summed[i] = low_out[i] + mid_out[i] + high_out[i];
        }

        let summed_rms = calculate_rms(&summed);

        // At 5000 Hz (high band dominates), sum should be close to original
        let ratio = summed_rms / input_rms;
        assert!(
            ratio > 0.8 && ratio < 1.2,
            "Sum should approximate flat response at 5000 Hz: ratio={}",
            ratio
        );
    }
}
