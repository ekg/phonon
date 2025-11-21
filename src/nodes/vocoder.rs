/// Vocoder node - channel vocoder for voice synthesis effects
///
/// This node implements a classic channel vocoder using parallel bandpass filters
/// to extract spectral envelopes from a modulator signal and apply them to a
/// carrier signal. This creates the characteristic "robotic voice" or "talking
/// instrument" effect.
///
/// # Implementation Details
///
/// Uses N parallel bandpass filters (typically 16) to divide both carrier and
/// modulator into frequency bands. Each modulator band's amplitude envelope is
/// extracted and used to control the amplitude of the corresponding carrier band.
/// The bands are then summed to produce the vocoded output.
///
/// # Algorithm
///
/// 1. Split carrier and modulator into N frequency bands (bandpass filters)
/// 2. Extract envelope from each modulator band (envelope follower)
/// 3. Apply modulator envelopes to corresponding carrier bands
/// 4. Sum all bands for final output
///
/// # Band Spacing
///
/// Bands are spaced logarithmically from 100 Hz to 8000 Hz (human hearing range).
/// This matches the perceptual frequency resolution of the human ear.
///
/// # References
///
/// - Dudley, H. (1939) "The Vocoder" - Original vocoder patent
/// - Flanagan, J. L. (1972) "Speech Analysis Synthesis and Perception"
/// - Julius O. Smith III "Spectral Audio Signal Processing" (vocoder chapter)
///
/// # Musical Characteristics
///
/// - Classic robotic voice effect (carrier = saw/pulse, modulator = voice)
/// - Talking drums/synths (carrier = oscillator, modulator = speech)
/// - Cross-synthesis effects (carrier = pad, modulator = drums)
/// - More bands = higher quality, more CPU
/// - Bandwidth controls clarity vs smoothness

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Internal state for a single vocoder band
#[derive(Debug, Clone)]
struct VocoderBand {
    /// Bandpass filter for carrier signal
    carrier_filter: DirectForm2Transposed<f32>,
    /// Bandpass filter for modulator signal
    modulator_filter: DirectForm2Transposed<f32>,
    /// Current envelope value (0.0 to 1.0+)
    envelope_state: f32,
}

impl VocoderBand {
    fn new(center_freq: f32, bandwidth: f32, sample_rate: f32) -> Self {
        let carrier_filter = Self::create_bandpass(center_freq, bandwidth, sample_rate);
        let modulator_filter = Self::create_bandpass(center_freq, bandwidth, sample_rate);

        Self {
            carrier_filter,
            modulator_filter,
            envelope_state: 0.0,
        }
    }

    /// Create a bandpass filter for a given center frequency and bandwidth
    fn create_bandpass(
        center_freq: f32,
        bandwidth: f32,
        sample_rate: f32,
    ) -> DirectForm2Transposed<f32> {
        // Calculate Q from bandwidth: Q = fc / BW
        let q = (center_freq / bandwidth).max(0.5).min(50.0);

        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            center_freq.hz(),
            q,
        )
        .unwrap();

        DirectForm2Transposed::<f32>::new(coeffs)
    }

    /// Update filter coefficients for new center frequency and bandwidth
    fn update_filters(&mut self, center_freq: f32, bandwidth: f32, sample_rate: f32) {
        let q = (center_freq / bandwidth).max(0.5).min(50.0);

        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            center_freq.hz(),
            q,
        )
        .unwrap();

        self.carrier_filter.update_coefficients(coeffs.clone());
        self.modulator_filter.update_coefficients(coeffs);
    }
}

/// Vocoder node with pattern-controlled carrier, modulator, bands, and bandwidth
///
/// # Example
/// ```ignore
/// // Voice modulating synth (talking synth effect)
/// let voice_input = SamplePlaybackNode::new(...);    // NodeId 0 (modulator)
/// let synth_carrier = OscillatorNode::new(...);      // NodeId 1 (carrier)
/// let num_bands = ConstantNode::new(16.0);           // NodeId 2
/// let bandwidth = ConstantNode::new(1.2);            // NodeId 3
/// let vocoder = VocoderNode::new(1, 0, 2, 3, 44100.0); // NodeId 4
/// // Voice controls synth pitch/timbre
/// ```
///
/// # Musical Applications
/// - Robotic voice effects
/// - Talking instruments (voice modulates synth)
/// - Cross-synthesis (drums modulate pads)
/// - Spectral morphing
/// - Classic vocoder sounds from 70s/80s
pub struct VocoderNode {
    /// Carrier signal (usually synthesizer)
    carrier: NodeId,
    /// Modulator signal (usually voice/audio)
    modulator: NodeId,
    /// Number of frequency bands (8-32, typically 16)
    num_bands_input: NodeId,
    /// Band overlap/bandwidth multiplier (0.5-2.0)
    bandwidth_input: NodeId,
    /// Vocoder bands
    bands: Vec<VocoderBand>,
    /// Sample rate for calculations
    sample_rate: f32,
    /// Last number of bands (for detecting changes)
    last_num_bands: usize,
    /// Last bandwidth multiplier (for detecting changes)
    last_bandwidth: f32,
}

impl VocoderNode {
    /// Create a new vocoder node
    ///
    /// # Arguments
    /// * `carrier` - NodeId providing carrier signal (usually synth/oscillator)
    /// * `modulator` - NodeId providing modulator signal (usually voice/audio)
    /// * `num_bands_input` - NodeId providing number of bands (8-32, typically 16)
    /// * `bandwidth_input` - NodeId providing bandwidth multiplier (0.5-2.0)
    /// * `sample_rate` - Sample rate in Hz
    pub fn new(
        carrier: NodeId,
        modulator: NodeId,
        num_bands_input: NodeId,
        bandwidth_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        // Initialize with default 16 bands
        let num_bands = 16;
        let bandwidth = 1.0;
        let bands = Self::create_bands(num_bands, bandwidth, sample_rate);

        Self {
            carrier,
            modulator,
            num_bands_input,
            bandwidth_input,
            bands,
            sample_rate,
            last_num_bands: num_bands,
            last_bandwidth: bandwidth,
        }
    }

    /// Calculate logarithmically-spaced band frequencies
    ///
    /// Human hearing is logarithmic, so bands are spaced exponentially
    /// from min_freq to max_freq.
    fn calculate_band_frequencies(num_bands: usize) -> Vec<f32> {
        let min_freq = 100.0_f32; // Hz
        let max_freq = 8000.0_f32; // Hz (upper limit of speech)

        (0..num_bands)
            .map(|i| {
                let t = i as f32 / (num_bands - 1) as f32;
                min_freq * (max_freq / min_freq).powf(t)
            })
            .collect()
    }

    /// Create vocoder bands for given parameters
    fn create_bands(num_bands: usize, bandwidth_mult: f32, sample_rate: f32) -> Vec<VocoderBand> {
        let freqs = Self::calculate_band_frequencies(num_bands);

        freqs
            .iter()
            .map(|&freq| {
                // Calculate bandwidth as a fraction of center frequency
                // Bandwidth ~= freq / 4 gives good separation
                let bandwidth = (freq / 4.0) * bandwidth_mult;
                VocoderBand::new(freq, bandwidth, sample_rate)
            })
            .collect()
    }

    /// Update bands if parameters changed
    fn update_bands_if_needed(&mut self, num_bands: usize, bandwidth_mult: f32) {
        let num_bands = num_bands.clamp(8, 32);
        let bandwidth_mult = bandwidth_mult.clamp(0.5, 2.0);

        // Check if we need to recreate bands (number changed)
        if num_bands != self.last_num_bands {
            self.bands = Self::create_bands(num_bands, bandwidth_mult, self.sample_rate);
            self.last_num_bands = num_bands;
            self.last_bandwidth = bandwidth_mult;
            return;
        }

        // Check if we need to update coefficients (bandwidth changed)
        if (bandwidth_mult - self.last_bandwidth).abs() > 0.01 {
            let freqs = Self::calculate_band_frequencies(num_bands);
            for (band, &freq) in self.bands.iter_mut().zip(freqs.iter()) {
                let bandwidth = (freq / 4.0) * bandwidth_mult;
                band.update_filters(freq, bandwidth, self.sample_rate);
            }
            self.last_bandwidth = bandwidth_mult;
        }
    }

    /// Get the carrier input node ID
    pub fn carrier(&self) -> NodeId {
        self.carrier
    }

    /// Get the modulator input node ID
    pub fn modulator(&self) -> NodeId {
        self.modulator
    }

    /// Get the number of bands input node ID
    pub fn num_bands_input(&self) -> NodeId {
        self.num_bands_input
    }

    /// Get the bandwidth input node ID
    pub fn bandwidth_input(&self) -> NodeId {
        self.bandwidth_input
    }

    /// Get the current number of bands
    pub fn num_bands(&self) -> usize {
        self.bands.len()
    }

    /// Reset the vocoder state
    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.envelope_state = 0.0;
        }
    }
}

impl AudioNode for VocoderNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            4,
            "VocoderNode requires 4 inputs: carrier, modulator, num_bands, bandwidth"
        );

        let carrier_buffer = inputs[0];
        let modulator_buffer = inputs[1];
        let num_bands_buffer = inputs[2];
        let bandwidth_buffer = inputs[3];

        debug_assert_eq!(
            carrier_buffer.len(),
            output.len(),
            "Carrier buffer length mismatch"
        );
        debug_assert_eq!(
            modulator_buffer.len(),
            output.len(),
            "Modulator buffer length mismatch"
        );
        debug_assert_eq!(
            num_bands_buffer.len(),
            output.len(),
            "Num bands buffer length mismatch"
        );
        debug_assert_eq!(
            bandwidth_buffer.len(),
            output.len(),
            "Bandwidth buffer length mismatch"
        );

        // Envelope follower time constants (in samples)
        let attack_coeff = (-1.0 / (0.005 * sample_rate)).exp(); // 5ms attack
        let release_coeff = (-1.0 / (0.05 * sample_rate)).exp(); // 50ms release

        // Update bands if parameters changed (check first sample)
        let num_bands = num_bands_buffer[0].round() as usize;
        let bandwidth_mult = bandwidth_buffer[0];
        self.update_bands_if_needed(num_bands, bandwidth_mult);

        // Process each sample
        for i in 0..output.len() {
            let carrier_sample = carrier_buffer[i];
            let modulator_sample = modulator_buffer[i];

            // Sum of all vocoded bands
            let mut vocoded = 0.0;

            // Process each frequency band
            for band in &mut self.bands {
                // Filter carrier and modulator through band
                let carrier_band = band.carrier_filter.run(carrier_sample);
                let modulator_band = band.modulator_filter.run(modulator_sample);

                // Extract envelope from modulator band (full-wave rectification)
                let rectified = modulator_band.abs();

                // Envelope follower with attack/release
                band.envelope_state = if rectified > band.envelope_state {
                    // Attack (fast)
                    attack_coeff * band.envelope_state + (1.0 - attack_coeff) * rectified
                } else {
                    // Release (slow)
                    release_coeff * band.envelope_state + (1.0 - release_coeff) * rectified
                };

                // Apply modulator envelope to carrier band
                vocoded += carrier_band * band.envelope_state;
            }

            output[i] = vocoded;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.carrier,
            self.modulator,
            self.num_bands_input,
            self.bandwidth_input,
        ]
    }

    fn name(&self) -> &str {
        "VocoderNode"
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

    /// Helper: Create test context
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_vocoder_voice_modulating_synth() {
        // Test 1: Voice modulating synth creates vocoder effect
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Carrier: Saw wave at 220 Hz (synth)
        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Saw);

        // Modulator: White noise (simulating voice-like signal)
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);

        // Vocoder parameters
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(1, 2, 3, 4, sample_rate);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier_osc.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        let inputs = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        // Process several blocks to let envelope followers settle
        for _ in 0..5 {
            vocoder.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let rms = calculate_rms(&output);

        // Should produce audible output
        assert!(
            rms > 0.01,
            "Vocoder should produce audible output, got RMS: {}",
            rms
        );

        // Output should be stable (no NaN/inf)
        assert!(
            output.iter().all(|&x| x.is_finite()),
            "Output should be stable"
        );
    }

    #[test]
    fn test_vocoder_num_bands_affects_quality() {
        // Test 2: Number of bands affects quality/resolution
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut bandwidth = ConstantNode::new(1.0);

        let context = test_context(block_size);

        // Test with 8 bands (low quality)
        let mut num_bands_8 = ConstantNode::new(8.0);
        let mut vocoder_8 = VocoderNode::new(0, 1, 2, 3, sample_rate);

        // Test with 32 bands (high quality)
        let mut num_bands_32 = ConstantNode::new(32.0);
        let mut vocoder_32 = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier_osc.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Test 8 bands
        let mut num_bands_8_buf = vec![8.0; block_size];
        num_bands_8.process_block(&[], &mut num_bands_8_buf, sample_rate, &context);

        let inputs_8 = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_8_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_8 = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_8.process_block(&inputs_8, &mut output_8, sample_rate, &context);
        }

        assert_eq!(vocoder_8.num_bands(), 8, "Should have 8 bands");

        // Test 32 bands
        let mut num_bands_32_buf = vec![32.0; block_size];
        num_bands_32.process_block(&[], &mut num_bands_32_buf, sample_rate, &context);

        let inputs_32 = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_32_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_32 = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_32.process_block(&inputs_32, &mut output_32, sample_rate, &context);
        }

        assert_eq!(vocoder_32.num_bands(), 32, "Should have 32 bands");

        // Both should produce output
        let rms_8 = calculate_rms(&output_8);
        let rms_32 = calculate_rms(&output_32);

        assert!(rms_8 > 0.01, "8 bands should produce output");
        assert!(rms_32 > 0.01, "32 bands should produce output");
    }

    #[test]
    fn test_vocoder_bandwidth_affects_clarity() {
        // Test 3: Bandwidth affects clarity vs smoothness
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut num_bands = ConstantNode::new(16.0);

        let context = test_context(block_size);

        // Narrow bandwidth (0.5) - sharper, more precise
        let mut bandwidth_narrow = ConstantNode::new(0.5);
        let mut vocoder_narrow = VocoderNode::new(0, 1, 2, 3, sample_rate);

        // Wide bandwidth (2.0) - smoother, more overlap
        let mut bandwidth_wide = ConstantNode::new(2.0);
        let mut vocoder_wide = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);

        carrier_osc.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        // Test narrow bandwidth
        let mut bandwidth_narrow_buf = vec![0.5; block_size];
        bandwidth_narrow.process_block(&[], &mut bandwidth_narrow_buf, sample_rate, &context);

        let inputs_narrow = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_narrow_buf.as_slice(),
        ];

        let mut output_narrow = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_narrow.process_block(&inputs_narrow, &mut output_narrow, sample_rate, &context);
        }

        // Test wide bandwidth
        let mut bandwidth_wide_buf = vec![2.0; block_size];
        bandwidth_wide.process_block(&[], &mut bandwidth_wide_buf, sample_rate, &context);

        let inputs_wide = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_wide_buf.as_slice(),
        ];

        let mut output_wide = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_wide.process_block(&inputs_wide, &mut output_wide, sample_rate, &context);
        }

        // Both should produce output
        let rms_narrow = calculate_rms(&output_narrow);
        let rms_wide = calculate_rms(&output_wide);

        assert!(rms_narrow > 0.01, "Narrow bandwidth should produce output");
        assert!(rms_wide > 0.01, "Wide bandwidth should produce output");
    }

    #[test]
    fn test_vocoder_envelope_tracking() {
        // Test 4: Envelope follower should track modulator amplitude
        let sample_rate = 44100.0;
        let block_size = 1024;

        // Carrier: Constant amplitude
        let mut carrier = ConstantNode::new(1.0);

        // Modulator: Varying amplitude (impulse pattern)
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let context = test_context(block_size);

        let mut carrier_buf = vec![1.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];

        carrier.process_block(&[], &mut carrier_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        // Silent modulator -> should produce minimal output
        let modulator_silent = vec![0.0; block_size];
        let inputs_silent = vec![
            carrier_buf.as_slice(),
            modulator_silent.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_silent = vec![0.0; block_size];
        for _ in 0..10 {
            // Let envelope followers decay
            vocoder.process_block(&inputs_silent, &mut output_silent, sample_rate, &context);
        }

        let rms_silent = calculate_rms(&output_silent);

        // Loud modulator -> should produce louder output
        let modulator_loud = vec![1.0; block_size];
        let inputs_loud = vec![
            carrier_buf.as_slice(),
            modulator_loud.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_loud = vec![0.0; block_size];
        for _ in 0..10 {
            // Let envelope followers rise
            vocoder.process_block(&inputs_loud, &mut output_loud, sample_rate, &context);
        }

        let rms_loud = calculate_rms(&output_loud);

        // Loud modulator should produce more output than silent
        assert!(
            rms_loud > rms_silent * 5.0,
            "Loud modulator should produce more output: loud={}, silent={}",
            rms_loud,
            rms_silent
        );
    }

    #[test]
    fn test_vocoder_pattern_modulation() {
        // Test 5: Vocoder parameters can be modulated by patterns
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier_osc = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);

        let context = test_context(block_size);

        // Start with 16 bands
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];
        let mut output1 = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier_osc.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        let inputs1 = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        for _ in 0..3 {
            vocoder.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        assert_eq!(vocoder.num_bands(), 16);

        // Change to 24 bands
        num_bands.set_value(24.0);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);

        let inputs2 = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output2 = vec![0.0; block_size];
        vocoder.process_block(&inputs2, &mut output2, sample_rate, &context);

        assert_eq!(vocoder.num_bands(), 24, "Bands should update to 24");
    }

    #[test]
    fn test_vocoder_different_carriers() {
        // Test 6: Vocoder works with different carrier types
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);

        let context = test_context(block_size);

        // Test with saw wave carrier
        let mut carrier_freq_saw = ConstantNode::new(220.0);
        let mut carrier_saw = OscillatorNode::new(0, Waveform::Saw);
        let mut vocoder_saw = VocoderNode::new(1, 2, 3, 4, sample_rate);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_saw_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];

        carrier_freq_saw.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier_saw.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_saw_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        let inputs_saw = vec![
            carrier_saw_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_saw = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_saw.process_block(&inputs_saw, &mut output_saw, sample_rate, &context);
        }

        let rms_saw = calculate_rms(&output_saw);
        assert!(rms_saw > 0.01, "Saw carrier should work");

        // Test with pulse wave carrier
        let mut carrier_freq_pulse = ConstantNode::new(220.0);
        let mut carrier_pulse = OscillatorNode::new(0, Waveform::Square);
        let mut vocoder_pulse = VocoderNode::new(1, 2, 3, 4, sample_rate);

        let mut carrier_pulse_buf = vec![0.0; block_size];
        carrier_freq_pulse.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        carrier_pulse.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_pulse_buf,
            sample_rate,
            &context,
        );

        let inputs_pulse = vec![
            carrier_pulse_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_pulse = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_pulse.process_block(&inputs_pulse, &mut output_pulse, sample_rate, &context);
        }

        let rms_pulse = calculate_rms(&output_pulse);
        assert!(rms_pulse > 0.01, "Pulse carrier should work");
    }

    #[test]
    fn test_vocoder_different_modulators() {
        // Test 7: Vocoder works with different modulator types
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier = OscillatorNode::new(0, Waveform::Saw);
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );

        // Test with noise modulator
        let mut mod_amp_noise = ConstantNode::new(1.0);
        let mut modulator_noise = NoiseNode::new(0);
        let mut vocoder_noise = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_noise_buf = vec![0.0; block_size];

        mod_amp_noise.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        modulator_noise.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_noise_buf,
            sample_rate,
            &context,
        );

        let inputs_noise = vec![
            carrier_buf.as_slice(),
            modulator_noise_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_noise = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_noise.process_block(&inputs_noise, &mut output_noise, sample_rate, &context);
        }

        let rms_noise = calculate_rms(&output_noise);
        assert!(rms_noise > 0.01, "Noise modulator should work");

        // Test with sine modulator (voice-like tone)
        let mut mod_freq = ConstantNode::new(440.0);
        let mut modulator_sine = OscillatorNode::new(0, Waveform::Sine);
        let mut vocoder_sine = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let mut mod_freq_buf = vec![0.0; block_size];
        let mut modulator_sine_buf = vec![0.0; block_size];

        mod_freq.process_block(&[], &mut mod_freq_buf, sample_rate, &context);
        modulator_sine.process_block(
            &[mod_freq_buf.as_slice()],
            &mut modulator_sine_buf,
            sample_rate,
            &context,
        );

        let inputs_sine = vec![
            carrier_buf.as_slice(),
            modulator_sine_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output_sine = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder_sine.process_block(&inputs_sine, &mut output_sine, sample_rate, &context);
        }

        let rms_sine = calculate_rms(&output_sine);
        assert!(rms_sine > 0.01, "Sine modulator should work");
    }

    #[test]
    fn test_vocoder_extreme_parameters() {
        // Test 8: Extreme parameters should be handled safely
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);

        // Extreme parameters: 100 bands (clamped to 32), bandwidth 10.0 (clamped to 2.0)
        let mut num_bands = ConstantNode::new(100.0);
        let mut bandwidth = ConstantNode::new(10.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        let inputs = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        // Should not panic
        vocoder.process_block(&inputs, &mut output, sample_rate, &context);

        // Should clamp to max 32 bands
        assert_eq!(vocoder.num_bands(), 32, "Should clamp to 32 bands");

        // Output should be stable
        for (i, &sample) in output.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} not finite with extreme parameters: {}",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_vocoder_stability() {
        // Test 9: Vocoder should remain stable over many blocks
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        // Process 100 blocks
        for _ in 0..100 {
            carrier.process_block(
                &[carrier_freq_buf.as_slice()],
                &mut carrier_buf,
                sample_rate,
                &context,
            );
            modulator.process_block(
                &[mod_amp_buf.as_slice()],
                &mut modulator_buf,
                sample_rate,
                &context,
            );

            let inputs = vec![
                carrier_buf.as_slice(),
                modulator_buf.as_slice(),
                num_bands_buf.as_slice(),
                bandwidth_buf.as_slice(),
            ];

            vocoder.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability
            for (i, &sample) in output.iter().enumerate() {
                assert!(
                    sample.is_finite(),
                    "Sample {} became infinite/NaN after many blocks",
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
    fn test_vocoder_input_nodes() {
        // Test 10: Verify input node dependencies
        let vocoder = VocoderNode::new(10, 20, 30, 40, 44100.0);
        let deps = vocoder.input_nodes();

        assert_eq!(deps.len(), 4);
        assert_eq!(deps[0], 10); // carrier
        assert_eq!(deps[1], 20); // modulator
        assert_eq!(deps[2], 30); // num_bands
        assert_eq!(deps[3], 40); // bandwidth
    }

    #[test]
    fn test_vocoder_logarithmic_bands() {
        // Test 11: Band frequencies should be logarithmically spaced
        let freqs = VocoderNode::calculate_band_frequencies(16);

        assert_eq!(freqs.len(), 16);

        // First frequency should be near 100 Hz
        assert!((freqs[0] - 100.0).abs() < 1.0);

        // Last frequency should be near 8000 Hz
        assert!((freqs[15] - 8000.0).abs() < 1.0);

        // Frequency ratios should be approximately equal (logarithmic spacing)
        let ratio1 = freqs[1] / freqs[0];
        let ratio2 = freqs[2] / freqs[1];
        assert!(
            (ratio1 - ratio2).abs() < 0.01,
            "Bands should be logarithmically spaced: ratio1={}, ratio2={}",
            ratio1,
            ratio2
        );
    }

    #[test]
    fn test_vocoder_reset() {
        // Test 12: Reset should clear envelope states
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        // Process several blocks to build up envelope state
        for _ in 0..10 {
            carrier.process_block(
                &[carrier_freq_buf.as_slice()],
                &mut carrier_buf,
                sample_rate,
                &context,
            );
            modulator.process_block(
                &[mod_amp_buf.as_slice()],
                &mut modulator_buf,
                sample_rate,
                &context,
            );

            let inputs = vec![
                carrier_buf.as_slice(),
                modulator_buf.as_slice(),
                num_bands_buf.as_slice(),
                bandwidth_buf.as_slice(),
            ];

            vocoder.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let rms_before_reset = calculate_rms(&output);

        // Reset
        vocoder.reset();

        // Process with silent modulator - should produce minimal output
        let modulator_silent = vec![0.0; block_size];
        let inputs_silent = vec![
            carrier_buf.as_slice(),
            modulator_silent.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        vocoder.process_block(&inputs_silent, &mut output, sample_rate, &context);

        let rms_after_reset = calculate_rms(&output);

        // After reset with silent modulator, output should be lower
        // Allow for some settling - the key is that silent input produces less output
        assert!(
            rms_after_reset < rms_before_reset * 0.5,
            "After reset with silent modulator, output should be lower: before={}, after={}",
            rms_before_reset,
            rms_after_reset
        );
    }

    #[test]
    fn test_vocoder_band_update() {
        // Test 13: Changing band count should update internal structure
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![16.0; block_size];
        let mut bandwidth_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        vocoder.process_block(
            &[
                carrier_buf.as_slice(),
                modulator_buf.as_slice(),
                num_bands_buf.as_slice(),
                bandwidth_buf.as_slice(),
            ],
            &mut output,
            sample_rate,
            &context,
        );
        assert_eq!(vocoder.num_bands(), 16);

        // Change to 24 bands
        num_bands_buf.fill(24.0);
        vocoder.process_block(
            &[
                carrier_buf.as_slice(),
                modulator_buf.as_slice(),
                num_bands_buf.as_slice(),
                bandwidth_buf.as_slice(),
            ],
            &mut output,
            sample_rate,
            &context,
        );
        assert_eq!(vocoder.num_bands(), 24, "Should update to 24 bands");

        // Change to 12 bands
        num_bands_buf.fill(12.0);
        vocoder.process_block(
            &[
                carrier_buf.as_slice(),
                modulator_buf.as_slice(),
                num_bands_buf.as_slice(),
                bandwidth_buf.as_slice(),
            ],
            &mut output,
            sample_rate,
            &context,
        );
        assert_eq!(vocoder.num_bands(), 12, "Should update to 12 bands");
    }

    #[test]
    fn test_vocoder_bandwidth_update() {
        // Test 14: Changing bandwidth should update filter coefficients
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut carrier_freq = ConstantNode::new(220.0);
        let mut carrier = OscillatorNode::new(0, Waveform::Saw);
        let mut mod_amp = ConstantNode::new(1.0);
        let mut modulator = NoiseNode::new(0);
        let mut num_bands = ConstantNode::new(16.0);
        let mut bandwidth = ConstantNode::new(1.0);
        let mut vocoder = VocoderNode::new(0, 1, 2, 3, sample_rate);

        let context = test_context(block_size);

        let mut carrier_freq_buf = vec![0.0; block_size];
        let mut carrier_buf = vec![0.0; block_size];
        let mut mod_amp_buf = vec![0.0; block_size];
        let mut modulator_buf = vec![0.0; block_size];
        let mut num_bands_buf = vec![0.0; block_size];
        let mut bandwidth_buf = vec![1.0; block_size];
        let mut output1 = vec![0.0; block_size];

        carrier_freq.process_block(&[], &mut carrier_freq_buf, sample_rate, &context);
        mod_amp.process_block(&[], &mut mod_amp_buf, sample_rate, &context);
        num_bands.process_block(&[], &mut num_bands_buf, sample_rate, &context);
        bandwidth.process_block(&[], &mut bandwidth_buf, sample_rate, &context);

        carrier.process_block(
            &[carrier_freq_buf.as_slice()],
            &mut carrier_buf,
            sample_rate,
            &context,
        );
        modulator.process_block(
            &[mod_amp_buf.as_slice()],
            &mut modulator_buf,
            sample_rate,
            &context,
        );

        let inputs1 = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        // Process with bandwidth 1.0
        for _ in 0..5 {
            vocoder.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        let rms1 = calculate_rms(&output1);

        // Change to bandwidth 1.5
        bandwidth.set_value(1.5);
        bandwidth_buf.fill(1.5);

        let inputs2 = vec![
            carrier_buf.as_slice(),
            modulator_buf.as_slice(),
            num_bands_buf.as_slice(),
            bandwidth_buf.as_slice(),
        ];

        let mut output2 = vec![0.0; block_size];
        for _ in 0..5 {
            vocoder.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let rms2 = calculate_rms(&output2);

        // Both should produce output
        assert!(rms1 > 0.01, "Bandwidth 1.0 should produce output");
        assert!(rms2 > 0.01, "Bandwidth 1.5 should produce output");

        // Different bandwidths may produce different RMS levels
        // (not testing for specific relationship, just that both work)
    }
}
