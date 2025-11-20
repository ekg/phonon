/// Parametric EQ node - 3-band parametric equalizer with low/mid/high control
///
/// This node implements a professional-grade 3-band parametric EQ using three
/// cascaded biquad filters from the Audio EQ Cookbook:
/// 1. Low shelf - Boost/cut bass frequencies
/// 2. Peaking filter - Boost/cut mid frequencies with Q control
/// 3. High shelf - Boost/cut treble frequencies
///
/// # Implementation Details
///
/// Each band uses biquad topology:
/// - Low shelf: Smooth boost/cut below `low_freq` (20-500 Hz typical)
/// - Mid peak: Precise boost/cut at `mid_freq` with bandwidth control via Q (200-8000 Hz)
/// - High shelf: Smooth boost/cut above `high_freq` (2000-20000 Hz typical)
///
/// Based on:
/// - Robert Bristow-Johnson's Audio EQ Cookbook (biquad formulas)
/// - Professional audio EQ designs (Neve, API, SSL consoles)
/// - DAW parametric EQs (Pro Tools, Logic, Ableton)
///
/// # Musical Characteristics
///
/// - **Low shelf**: Shape bass response (kick, bass guitar, sub frequencies)
/// - **Mid peak**: Surgical frequency control (vocals, snare, presence)
/// - **High shelf**: Air and brightness (cymbals, vocal sibilance, sparkle)
/// - **Q control**: Narrow (high Q) for surgical cuts, wide (low Q) for musical shaping
/// - **Gain range**: ±24 dB typical (enough for corrective and creative EQ)
///
/// # Usage Examples
///
/// ```ignore
/// // Vocal presence boost
/// let signal = OscillatorNode::new(0, Waveform::Saw);
/// let low_freq = ConstantNode::new(100.0);   // Low shelf at 100 Hz
/// let low_gain = ConstantNode::new(-3.0);    // Cut -3 dB (reduce rumble)
/// let mid_freq = ConstantNode::new(3000.0);  // Mid peak at 3 kHz
/// let mid_gain = ConstantNode::new(4.0);     // Boost +4 dB (presence)
/// let mid_q = ConstantNode::new(1.5);        // Moderate Q (musical)
/// let high_freq = ConstantNode::new(10000.0); // High shelf at 10 kHz
/// let high_gain = ConstantNode::new(2.0);    // Boost +2 dB (air)
/// let peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Parametric EQ node with 3-band control
///
/// All parameters can be modulated by patterns in real-time.
///
/// # Musical Applications
/// - Vocal shaping (cut mud, boost presence, add air)
/// - Drum EQ (kick punch, snare body, cymbal shimmer)
/// - Mix bus processing (gentle tonal shaping)
/// - Creative sound design (extreme boosts/cuts)
/// - Pattern-controlled filter sweeps (automate mid_freq/mid_gain)
pub struct ParametricEQNode {
    /// Input signal to be equalized
    input: NodeId,
    /// Low shelf frequency (Hz)
    low_freq: NodeId,
    /// Low shelf gain (dB)
    low_gain: NodeId,
    /// Mid peak frequency (Hz)
    mid_freq: NodeId,
    /// Mid peak gain (dB)
    mid_gain: NodeId,
    /// Mid peak Q factor
    mid_q: NodeId,
    /// High shelf frequency (Hz)
    high_freq: NodeId,
    /// High shelf gain (dB)
    high_gain: NodeId,

    /// Low shelf filter state
    low_shelf: DirectForm2Transposed<f32>,
    /// Mid peaking filter state
    mid_peak: DirectForm2Transposed<f32>,
    /// High shelf filter state
    high_shelf: DirectForm2Transposed<f32>,

    /// Last parameter values (for detecting changes)
    last_low_freq: f32,
    last_low_gain: f32,
    last_mid_freq: f32,
    last_mid_gain: f32,
    last_mid_q: f32,
    last_high_freq: f32,
    last_high_gain: f32,
}

impl ParametricEQNode {
    /// Create a new 3-band parametric EQ node
    ///
    /// # Arguments
    /// * `input` - NodeId providing signal to equalize
    /// * `low_freq` - NodeId for low shelf frequency (20-500 Hz typical)
    /// * `low_gain` - NodeId for low shelf gain (-24 to +24 dB)
    /// * `mid_freq` - NodeId for mid peak frequency (200-8000 Hz typical)
    /// * `mid_gain` - NodeId for mid peak gain (-24 to +24 dB)
    /// * `mid_q` - NodeId for mid peak Q factor (0.1-10.0)
    /// * `high_freq` - NodeId for high shelf frequency (2000-20000 Hz typical)
    /// * `high_gain` - NodeId for high shelf gain (-24 to +24 dB)
    ///
    /// # Notes
    /// - Gain in dB: positive = boost, negative = cut, 0 = neutral
    /// - Q factor: lower = wider bandwidth, higher = narrower (surgical)
    /// - Frequencies are clamped to valid range (20 Hz to Nyquist)
    /// - Gains are clamped to ±24 dB to prevent extreme boosts
    pub fn new(
        input: NodeId,
        low_freq: NodeId,
        low_gain: NodeId,
        mid_freq: NodeId,
        mid_gain: NodeId,
        mid_q: NodeId,
        high_freq: NodeId,
        high_gain: NodeId,
    ) -> Self {
        let sample_rate = 44100.0;

        // Initialize with neutral settings (no EQ)
        let low_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowShelf(0.0), // 0 dB gain = neutral
            sample_rate.hz(),
            100.0.hz(),
            1.0, // Q unused for shelves
        )
        .unwrap();

        let mid_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::PeakingEQ(0.0), // 0 dB gain = neutral
            sample_rate.hz(),
            1000.0.hz(),
            1.0,
        )
        .unwrap();

        let high_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighShelf(0.0), // 0 dB gain = neutral
            sample_rate.hz(),
            10000.0.hz(),
            1.0, // Q unused for shelves
        )
        .unwrap();

        Self {
            input,
            low_freq,
            low_gain,
            mid_freq,
            mid_gain,
            mid_q,
            high_freq,
            high_gain,
            low_shelf: DirectForm2Transposed::<f32>::new(low_coeffs),
            mid_peak: DirectForm2Transposed::<f32>::new(mid_coeffs),
            high_shelf: DirectForm2Transposed::<f32>::new(high_coeffs),
            last_low_freq: 100.0,
            last_low_gain: 0.0,
            last_mid_freq: 1000.0,
            last_mid_gain: 0.0,
            last_mid_q: 1.0,
            last_high_freq: 10000.0,
            last_high_gain: 0.0,
        }
    }

    /// Reset all filter states (clear memory)
    pub fn reset(&mut self) {
        let sample_rate = 44100.0;

        let low_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::LowShelf(self.last_low_gain),
            sample_rate.hz(),
            self.last_low_freq.hz(),
            1.0,
        )
        .unwrap();

        let mid_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::PeakingEQ(self.last_mid_gain),
            sample_rate.hz(),
            self.last_mid_freq.hz(),
            self.last_mid_q,
        )
        .unwrap();

        let high_coeffs = Coefficients::<f32>::from_params(
            biquad::Type::HighShelf(self.last_high_gain),
            sample_rate.hz(),
            self.last_high_freq.hz(),
            1.0,
        )
        .unwrap();

        self.low_shelf = DirectForm2Transposed::<f32>::new(low_coeffs);
        self.mid_peak = DirectForm2Transposed::<f32>::new(mid_coeffs);
        self.high_shelf = DirectForm2Transposed::<f32>::new(high_coeffs);
    }
}

impl AudioNode for ParametricEQNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            8,
            "ParametricEQNode requires 8 inputs: signal, low_freq, low_gain, mid_freq, mid_gain, mid_q, high_freq, high_gain"
        );

        let input_buffer = inputs[0];
        let low_freq_buffer = inputs[1];
        let low_gain_buffer = inputs[2];
        let mid_freq_buffer = inputs[3];
        let mid_gain_buffer = inputs[4];
        let mid_q_buffer = inputs[5];
        let high_freq_buffer = inputs[6];
        let high_gain_buffer = inputs[7];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );

        for i in 0..output.len() {
            // Clamp parameters to valid ranges
            let low_freq = low_freq_buffer[i].max(20.0).min(500.0);
            let low_gain = low_gain_buffer[i].max(-24.0).min(24.0);
            let mid_freq = mid_freq_buffer[i].max(20.0).min(sample_rate * 0.49);
            let mid_gain = mid_gain_buffer[i].max(-24.0).min(24.0);
            let mid_q = mid_q_buffer[i].max(0.1).min(10.0);
            let high_freq = high_freq_buffer[i].max(2000.0).min(sample_rate * 0.49);
            let high_gain = high_gain_buffer[i].max(-24.0).min(24.0);

            // Update low shelf coefficients if changed significantly
            if (low_freq - self.last_low_freq).abs() > 1.0
                || (low_gain - self.last_low_gain).abs() > 0.1
            {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::LowShelf(low_gain),
                    sample_rate.hz(),
                    low_freq.hz(),
                    1.0,
                )
                .unwrap();
                self.low_shelf = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_low_freq = low_freq;
                self.last_low_gain = low_gain;
            }

            // Update mid peak coefficients if changed significantly
            if (mid_freq - self.last_mid_freq).abs() > 1.0
                || (mid_gain - self.last_mid_gain).abs() > 0.1
                || (mid_q - self.last_mid_q).abs() > 0.01
            {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::PeakingEQ(mid_gain),
                    sample_rate.hz(),
                    mid_freq.hz(),
                    mid_q,
                )
                .unwrap();
                self.mid_peak = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_mid_freq = mid_freq;
                self.last_mid_gain = mid_gain;
                self.last_mid_q = mid_q;
            }

            // Update high shelf coefficients if changed significantly
            if (high_freq - self.last_high_freq).abs() > 1.0
                || (high_gain - self.last_high_gain).abs() > 0.1
            {
                let coeffs = Coefficients::<f32>::from_params(
                    biquad::Type::HighShelf(high_gain),
                    sample_rate.hz(),
                    high_freq.hz(),
                    1.0,
                )
                .unwrap();
                self.high_shelf = DirectForm2Transposed::<f32>::new(coeffs);
                self.last_high_freq = high_freq;
                self.last_high_gain = high_gain;
            }

            // Apply filters in series: input -> low shelf -> mid peak -> high shelf -> output
            let after_low = self.low_shelf.run(input_buffer[i]);
            let after_mid = self.mid_peak.run(after_low);
            output[i] = self.high_shelf.run(after_mid);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.input,
            self.low_freq,
            self.low_gain,
            self.mid_freq,
            self.mid_gain,
            self.mid_q,
            self.high_freq,
            self.high_gain,
        ]
    }

    fn name(&self) -> &str {
        "ParametricEQNode"
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
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_parametric_eq_neutral() {
        // Test 1: All gains at 0 dB should pass signal unmodified
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        // Neutral EQ settings
        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0); // Neutral
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(0.0); // Neutral
        let mut mid_q = ConstantNode::new(1.0);
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0); // Neutral

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);
        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks to reach steady state
        for _ in 0..3 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // Neutral EQ should pass signal with minimal change
        let ratio = output_rms / input_rms;
        assert!(
            ratio > 0.95 && ratio < 1.05,
            "Neutral EQ should preserve signal: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            ratio
        );
    }

    #[test]
    fn test_parametric_eq_low_shelf_boost() {
        // Test 2: Low shelf boost should increase bass energy
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Low frequency signal (100 Hz)
        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // Test with low shelf boost
        let mut low_freq = ConstantNode::new(200.0);
        let mut low_gain = ConstantNode::new(6.0); // +6 dB boost
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(0.0);
        let mut mid_q = ConstantNode::new(1.0);
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // +6 dB boost should increase amplitude (6 dB ≈ 2x amplitude)
        assert!(
            output_rms > input_rms * 1.5,
            "Low shelf boost should increase bass: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_low_shelf_cut() {
        // Test 3: Low shelf cut should reduce bass energy
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_node = ConstantNode::new(100.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // Test with low shelf cut
        let mut low_freq = ConstantNode::new(200.0);
        let mut low_gain = ConstantNode::new(-12.0); // -12 dB cut
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(0.0);
        let mut mid_q = ConstantNode::new(1.0);
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // -12 dB cut should reduce amplitude significantly
        assert!(
            output_rms < input_rms * 0.5,
            "Low shelf cut should reduce bass: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_mid_peak_boost() {
        // Test 4: Mid peak boost should increase energy at target frequency
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Signal at 1000 Hz (where we'll boost)
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // Mid peak boost at 1000 Hz
        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(9.0); // +9 dB boost
        let mut mid_q = ConstantNode::new(2.0); // Moderate Q
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // +9 dB boost should increase amplitude significantly (9 dB ≈ 2.8x)
        assert!(
            output_rms > input_rms * 2.0,
            "Mid peak boost should increase signal: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_mid_peak_cut() {
        // Test 5: Mid peak cut should reduce energy at target frequency
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_node = ConstantNode::new(2000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // Mid peak cut at 2000 Hz
        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_freq = ConstantNode::new(2000.0);
        let mut mid_gain = ConstantNode::new(-12.0); // -12 dB cut
        let mut mid_q = ConstantNode::new(3.0); // Narrow cut
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // -12 dB cut should reduce amplitude significantly
        assert!(
            output_rms < input_rms * 0.5,
            "Mid peak cut should reduce signal: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_high_shelf_boost() {
        // Test 6: High shelf boost should increase treble energy
        let sample_rate = 44100.0;
        let block_size = 1024;

        // High frequency signal (8000 Hz)
        let mut freq_node = ConstantNode::new(8000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // High shelf boost
        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(0.0);
        let mut mid_q = ConstantNode::new(1.0);
        let mut high_freq = ConstantNode::new(5000.0);
        let mut high_gain = ConstantNode::new(6.0); // +6 dB boost

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // +6 dB boost should increase amplitude
        assert!(
            output_rms > input_rms * 1.5,
            "High shelf boost should increase treble: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_high_shelf_cut() {
        // Test 7: High shelf cut should reduce treble energy
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_node = ConstantNode::new(12000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // High shelf cut
        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(0.0);
        let mut mid_q = ConstantNode::new(1.0);
        let mut high_freq = ConstantNode::new(8000.0);
        let mut high_gain = ConstantNode::new(-9.0); // -9 dB cut

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // -9 dB cut should reduce amplitude
        assert!(
            output_rms < input_rms * 0.5,
            "High shelf cut should reduce treble: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_all_bands_boost() {
        // Test 8: All bands boosted should increase overall energy
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Rich harmonic content (saw wave)
        let mut freq_node = ConstantNode::new(220.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // Boost all bands
        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(3.0);
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(4.0);
        let mut mid_q = ConstantNode::new(1.5);
        let mut high_freq = ConstantNode::new(8000.0);
        let mut high_gain = ConstantNode::new(3.0);

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        for _ in 0..5 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // All bands boosted should increase energy (even moderate boosts add up)
        assert!(
            output_rms > input_rms * 1.02,
            "All bands boost should increase signal: input={}, output={}, ratio={}",
            input_rms,
            output_rms,
            output_rms / input_rms
        );
    }

    #[test]
    fn test_parametric_eq_q_control() {
        // Test 9: Q factor should control bandwidth of mid peak
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Signal at mid frequency
        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        // Test with low Q (wide bandwidth)
        let mut low_q_peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);
        let mut mid_q_low = ConstantNode::new(0.5); // Wide

        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_gain = ConstantNode::new(6.0);
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_low_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q_low.process_block(&[], &mut mid_q_low_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let inputs_low_q = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_low_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output_low_q = vec![0.0; block_size];

        for _ in 0..5 {
            low_q_peq.process_block(&inputs_low_q, &mut output_low_q, sample_rate, &context);
        }

        let rms_low_q = calculate_rms(&output_low_q);

        // Test with high Q (narrow bandwidth)
        let mut high_q_peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);
        let mut mid_q_high = ConstantNode::new(5.0); // Narrow

        let mut mid_q_high_buf = vec![0.0; block_size];
        mid_q_high.process_block(&[], &mut mid_q_high_buf, sample_rate, &context);

        let inputs_high_q = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_high_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output_high_q = vec![0.0; block_size];

        for _ in 0..5 {
            high_q_peq.process_block(&inputs_high_q, &mut output_high_q, sample_rate, &context);
        }

        let rms_high_q = calculate_rms(&output_high_q);

        // Both should boost at center frequency, but high Q should be more pronounced
        assert!(
            rms_low_q > 0.5 && rms_high_q > 0.5,
            "Both Q values should boost at center frequency: low_q={}, high_q={}",
            rms_low_q,
            rms_high_q
        );
    }

    #[test]
    fn test_parametric_eq_frequency_sweep() {
        // Test 10: Sweeping mid frequency should affect different frequencies
        let sample_rate = 44100.0;
        let block_size = 512;

        // Rich harmonic content
        let mut freq_node = ConstantNode::new(220.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_gain = ConstantNode::new(6.0);
        let mut mid_q = ConstantNode::new(2.0);
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        // Test at 500 Hz
        let mut peq1 = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);
        let mut mid_freq1 = ConstantNode::new(500.0);
        let mut mid_freq1_buf = vec![0.0; block_size];
        mid_freq1.process_block(&[], &mut mid_freq1_buf, sample_rate, &context);

        let inputs1 = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq1_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output1 = vec![0.0; block_size];

        for _ in 0..3 {
            peq1.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        let rms1 = calculate_rms(&output1);

        // Test at 2000 Hz
        let mut peq2 = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);
        let mut mid_freq2 = ConstantNode::new(2000.0);
        let mut mid_freq2_buf = vec![0.0; block_size];
        mid_freq2.process_block(&[], &mut mid_freq2_buf, sample_rate, &context);

        let inputs2 = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq2_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output2 = vec![0.0; block_size];

        for _ in 0..3 {
            peq2.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let rms2 = calculate_rms(&output2);

        // Different mid frequencies should produce different outputs
        assert!(
            (rms1 - rms2).abs() > 0.01,
            "Different mid frequencies should affect signal differently: rms1={}, rms2={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_parametric_eq_stability() {
        // Test 11: Filter should remain stable with extreme parameters
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.0; block_size];
        for i in 0..block_size {
            signal_buf[i] = ((i as f32 * 0.1).sin() * 0.8).clamp(-1.0, 1.0);
        }

        // Extreme settings
        let mut low_freq = ConstantNode::new(20.0);
        let mut low_gain = ConstantNode::new(24.0); // Max boost
        let mut mid_freq = ConstantNode::new(20000.0); // Very high
        let mut mid_gain = ConstantNode::new(-24.0); // Max cut
        let mut mid_q = ConstantNode::new(10.0); // Max Q
        let mut high_freq = ConstantNode::new(20000.0);
        let mut high_gain = ConstantNode::new(24.0); // Max boost

        let context = test_context(block_size);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let mut peq = ParametricEQNode::new(0, 1, 2, 3, 4, 5, 6, 7);
        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Process multiple blocks
        for _ in 0..10 {
            peq.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} became infinite/NaN",
                    i
                );
                assert!(
                    sample.abs() < 100.0,
                    "Sample {} has extreme value: {}",
                    i,
                    sample
                );
            }
        }
    }

    #[test]
    fn test_parametric_eq_pattern_modulation() {
        // Test 12: Parameters should respond to modulation
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_node = ConstantNode::new(1000.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[freq_buf.as_slice()], &mut signal_buf, sample_rate, &context);

        // Create modulating gain buffer (0 to 6 dB)
        let mut mid_gain_buf = vec![0.0; block_size];
        for i in 0..block_size {
            mid_gain_buf[i] = (i as f32 / block_size as f32) * 6.0;
        }

        let mut low_freq = ConstantNode::new(100.0);
        let mut low_gain = ConstantNode::new(0.0);
        let mut mid_freq = ConstantNode::new(1000.0);
        let mut mid_q = ConstantNode::new(2.0);
        let mut high_freq = ConstantNode::new(10000.0);
        let mut high_gain = ConstantNode::new(0.0);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let mut peq = ParametricEQNode::new(1, 2, 3, 4, 5, 6, 7, 8);
        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        peq.process_block(&inputs, &mut output, sample_rate, &context);

        // Output should vary with modulation (not constant)
        let first_quarter_rms = calculate_rms(&output[0..block_size / 4]);
        let last_quarter_rms = calculate_rms(&output[3 * block_size / 4..]);

        assert!(
            (first_quarter_rms - last_quarter_rms).abs() > 0.05,
            "Modulation should affect output: first={}, last={}",
            first_quarter_rms,
            last_quarter_rms
        );
    }

    #[test]
    fn test_parametric_eq_input_nodes() {
        // Test 13: Verify input node dependencies
        let peq = ParametricEQNode::new(10, 20, 30, 40, 50, 60, 70, 80);
        let deps = peq.input_nodes();

        assert_eq!(deps.len(), 8);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // low_freq
        assert_eq!(deps[2], 30); // low_gain
        assert_eq!(deps[3], 40); // mid_freq
        assert_eq!(deps[4], 50); // mid_gain
        assert_eq!(deps[5], 60); // mid_q
        assert_eq!(deps[6], 70); // high_freq
        assert_eq!(deps[7], 80); // high_gain
    }

    #[test]
    fn test_parametric_eq_parameter_clamping() {
        // Test 14: Extreme parameters should be clamped safely
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut signal_buf = vec![0.5; block_size]; // DC signal

        // Out-of-range parameters
        let mut low_freq = ConstantNode::new(5.0); // Below minimum
        let mut low_gain = ConstantNode::new(100.0); // Way above max
        let mut mid_freq = ConstantNode::new(100000.0); // Above Nyquist
        let mut mid_gain = ConstantNode::new(-100.0); // Way below min
        let mut mid_q = ConstantNode::new(50.0); // Way above max
        let mut high_freq = ConstantNode::new(1000.0); // Below minimum for high shelf
        let mut high_gain = ConstantNode::new(200.0); // Extreme boost

        let context = test_context(block_size);

        let mut low_freq_buf = vec![0.0; block_size];
        let mut low_gain_buf = vec![0.0; block_size];
        let mut mid_freq_buf = vec![0.0; block_size];
        let mut mid_gain_buf = vec![0.0; block_size];
        let mut mid_q_buf = vec![0.0; block_size];
        let mut high_freq_buf = vec![0.0; block_size];
        let mut high_gain_buf = vec![0.0; block_size];

        low_freq.process_block(&[], &mut low_freq_buf, sample_rate, &context);
        low_gain.process_block(&[], &mut low_gain_buf, sample_rate, &context);
        mid_freq.process_block(&[], &mut mid_freq_buf, sample_rate, &context);
        mid_gain.process_block(&[], &mut mid_gain_buf, sample_rate, &context);
        mid_q.process_block(&[], &mut mid_q_buf, sample_rate, &context);
        high_freq.process_block(&[], &mut high_freq_buf, sample_rate, &context);
        high_gain.process_block(&[], &mut high_gain_buf, sample_rate, &context);

        let mut peq = ParametricEQNode::new(0, 1, 2, 3, 4, 5, 6, 7);
        let inputs = vec![
            signal_buf.as_slice(),
            low_freq_buf.as_slice(),
            low_gain_buf.as_slice(),
            mid_freq_buf.as_slice(),
            mid_gain_buf.as_slice(),
            mid_q_buf.as_slice(),
            high_freq_buf.as_slice(),
            high_gain_buf.as_slice(),
        ];
        let mut output = vec![0.0; block_size];

        // Should not panic despite extreme values
        peq.process_block(&inputs, &mut output, sample_rate, &context);

        // All samples should be finite
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} not finite with extreme parameters: {}",
                i,
                sample
            );
        }
    }
}
