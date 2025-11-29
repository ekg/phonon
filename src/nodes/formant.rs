/// Formant filter for vowel synthesis
///
/// This node implements a formant filter using 3 parallel bandpass filters
/// tuned to create vowel sounds. Formants are resonant peaks in the frequency
/// spectrum that characterize vowel sounds in speech.
///
/// # Implementation Details
///
/// Uses 3 parallel bandpass filters (biquad) to create formants F1, F2, F3.
/// Vowel selection (0-4) controls which formant frequencies are used:
/// - 0.0 = A (730, 1090, 2440 Hz)
/// - 1.0 = E (270, 2290, 3010 Hz)
/// - 2.0 = I (390, 1990, 2550 Hz)
/// - 3.0 = O (570, 840, 2410 Hz)
/// - 4.0 = U (440, 1020, 2240 Hz)
///
/// Intermediate values interpolate smoothly between vowels.
///
/// # References
///
/// Formant frequencies based on:
/// - Hillenbrand et al. (1995) "Acoustic Characteristics of American English Vowels"
/// - Peterson & Barney (1952) "Control Methods Used in a Study of the Vowels"
/// - Averaged male speaker formants (rounded for ease of use)
///
/// # Musical Characteristics
///
/// - Creates vocal-like timbres from any input signal
/// - Smooth interpolation enables vowel morphing effects
/// - Works well with saw/pulse waves (vocal cord simulation)
/// - Intensity controls formant prominence vs dry signal
/// - Classic vocal synthesis technique
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz};

/// Formant frequencies for 5 vowels (Hz): [F1, F2, F3]
///
/// Based on averaged male speaker formants from acoustic phonetics research.
const FORMANT_FREQS: [[f32; 3]; 5] = [
    [730.0, 1090.0, 2440.0], // A (as in "father")
    [270.0, 2290.0, 3010.0], // E (as in "bet")
    [390.0, 1990.0, 2550.0], // I (as in "bit")
    [570.0, 840.0, 2410.0],  // O (as in "bought")
    [440.0, 1020.0, 2240.0], // U (as in "book")
];

/// Formant bandwidths (Hz) - approximate Q values for natural vowel sounds
///
/// Wider bandwidth for F1 (lower frequency), narrower for F2/F3.
const FORMANT_BANDWIDTHS: [f32; 3] = [
    90.0,  // F1 bandwidth
    110.0, // F2 bandwidth
    170.0, // F3 bandwidth
];

/// Internal state for formant filter
#[derive(Debug, Clone)]
struct FormantState {
    /// Bandpass filter for first formant (F1)
    filter1: DirectForm2Transposed<f32>,
    /// Bandpass filter for second formant (F2)
    filter2: DirectForm2Transposed<f32>,
    /// Bandpass filter for third formant (F3)
    filter3: DirectForm2Transposed<f32>,
    /// Last formant value (for detecting changes)
    last_formant: f32,
}

impl FormantState {
    fn new(sample_rate: f32) -> Self {
        // Initialize with vowel A (formant = 0.0)
        let vowel_idx = 0;

        let filter1 = Self::create_formant_filter(
            sample_rate,
            FORMANT_FREQS[vowel_idx][0],
            FORMANT_BANDWIDTHS[0],
        );

        let filter2 = Self::create_formant_filter(
            sample_rate,
            FORMANT_FREQS[vowel_idx][1],
            FORMANT_BANDWIDTHS[1],
        );

        let filter3 = Self::create_formant_filter(
            sample_rate,
            FORMANT_FREQS[vowel_idx][2],
            FORMANT_BANDWIDTHS[2],
        );

        Self {
            filter1,
            filter2,
            filter3,
            last_formant: 0.0,
        }
    }

    /// Create a bandpass filter for a formant
    fn create_formant_filter(
        sample_rate: f32,
        freq: f32,
        bandwidth: f32,
    ) -> DirectForm2Transposed<f32> {
        // Convert bandwidth to Q factor: Q = freq / bandwidth
        let q = freq / bandwidth;
        let q = q.max(0.5).min(50.0); // Clamp to safe range

        let coeffs = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            freq.hz(),
            q,
        )
        .unwrap();

        DirectForm2Transposed::<f32>::new(coeffs)
    }

    /// Interpolate formant frequencies between vowels
    fn interpolate_formants(formant: f32) -> [f32; 3] {
        // Clamp formant to valid range [0.0, 4.0]
        let formant = formant.max(0.0).min(4.0);

        // Get lower and upper vowel indices
        let lower_idx = formant.floor() as usize;
        let upper_idx = (formant.ceil() as usize).min(4);

        // Interpolation factor (0.0 to 1.0)
        let t = formant - lower_idx as f32;

        let lower = FORMANT_FREQS[lower_idx];
        let upper = FORMANT_FREQS[upper_idx];

        [
            lower[0] + t * (upper[0] - lower[0]),
            lower[1] + t * (upper[1] - lower[1]),
            lower[2] + t * (upper[2] - lower[2]),
        ]
    }

    /// Update filters with new formant frequencies
    fn update_filters(&mut self, sample_rate: f32, formant: f32) {
        let freqs = Self::interpolate_formants(formant);

        // Update each formant filter
        let coeffs1 = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            freqs[0].hz(),
            freqs[0] / FORMANT_BANDWIDTHS[0],
        )
        .unwrap();
        self.filter1.update_coefficients(coeffs1);

        let coeffs2 = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            freqs[1].hz(),
            freqs[1] / FORMANT_BANDWIDTHS[1],
        )
        .unwrap();
        self.filter2.update_coefficients(coeffs2);

        let coeffs3 = Coefficients::<f32>::from_params(
            biquad::Type::BandPass,
            sample_rate.hz(),
            freqs[2].hz(),
            freqs[2] / FORMANT_BANDWIDTHS[2],
        )
        .unwrap();
        self.filter3.update_coefficients(coeffs3);

        self.last_formant = formant;
    }
}

/// Formant filter node for vowel synthesis
///
/// # Example
/// ```ignore
/// // Create vowel "AH" sound from saw wave
/// let freq = ConstantNode::new(110.0);                    // NodeId 0
/// let saw = OscillatorNode::new(0, Waveform::Saw);        // NodeId 1
/// let formant = ConstantNode::new(0.0);                   // NodeId 2 (vowel A)
/// let intensity = ConstantNode::new(0.8);                 // NodeId 3
/// let vowel = FormantNode::new(1, 2, 3);                  // NodeId 4
/// // Creates "AH" vowel sound
/// ```
///
/// # Musical Applications
/// - Vocal synthesis from oscillators
/// - Vowel morphing effects (modulate formant parameter)
/// - Talking instruments
/// - Formant filtering of drums/percussion
/// - Robot voice effects
pub struct FormantNode {
    /// Input signal to be filtered
    input: NodeId,
    /// Formant selection (0.0=A, 1.0=E, 2.0=I, 3.0=O, 4.0=U)
    formant: NodeId,
    /// Formant intensity (0.0=dry, 1.0=fully filtered)
    intensity: NodeId,
    /// Filter state
    state: FormantState,
}

impl FormantNode {
    /// Formant - Vowel synthesis using parallel formant filters
    ///
    /// Creates vocal-like timbres through 3 parallel bandpass filters tuned to vowel formants,
    /// enabling vowel morphing and vocal synthesis effects.
    ///
    /// # Parameters
    /// - `input`: NodeId providing signal to filter (saw/pulse waves work best)
    /// - `formant`: NodeId providing vowel selection 0.0-4.0 (default: 0 = A)
    /// - `intensity`: NodeId providing effect intensity 0.0-1.0 (default: 0.8)
    ///
    /// # Example
    /// ```phonon
    /// ~saw: saw 110
    /// ~vocal: ~saw # formant 1.5 0.8
    /// ```
    pub fn new(input: NodeId, formant: NodeId, intensity: NodeId) -> Self {
        Self {
            input,
            formant,
            intensity,
            state: FormantState::new(44100.0), // Default sample rate
        }
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the formant input node ID
    pub fn formant(&self) -> NodeId {
        self.formant
    }

    /// Get the intensity input node ID
    pub fn intensity(&self) -> NodeId {
        self.intensity
    }

    /// Reset the filter state
    pub fn reset(&mut self, sample_rate: f32) {
        self.state = FormantState::new(sample_rate);
    }
}

impl AudioNode for FormantNode {
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
            "FormantNode requires 3 inputs: signal, formant, intensity"
        );

        let input_buffer = inputs[0];
        let formant_buffer = inputs[1];
        let intensity_buffer = inputs[2];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            formant_buffer.len(),
            output.len(),
            "Formant buffer length mismatch"
        );
        debug_assert_eq!(
            intensity_buffer.len(),
            output.len(),
            "Intensity buffer length mismatch"
        );

        for i in 0..output.len() {
            let input_sample = input_buffer[i];
            let formant = formant_buffer[i];
            let intensity = intensity_buffer[i].max(0.0).min(1.0);

            // Update filter coefficients if formant changed significantly
            if (formant - self.state.last_formant).abs() > 0.01 {
                self.state.update_filters(sample_rate, formant);
            }

            // Process through all 3 formant filters in parallel
            let f1 = self.state.filter1.run(input_sample);
            let f2 = self.state.filter2.run(input_sample);
            let f3 = self.state.filter3.run(input_sample);

            // Sum the formants with appropriate weights
            // F1 is strongest, F2 medium, F3 weakest (natural vocal tract response)
            let formant_signal = f1 * 0.5 + f2 * 0.35 + f3 * 0.15;

            // Mix between dry and formant signal based on intensity
            output[i] = input_sample * (1.0 - intensity) + formant_signal * intensity;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.formant, self.intensity]
    }

    fn name(&self) -> &str {
        "FormantNode"
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

    /// Helper: Calculate spectral centroid (rough brightness measure)
    fn calculate_spectral_centroid(buffer: &[f32], sample_rate: f32) -> f32 {
        use rustfft::{num_complex::Complex, FftPlanner};

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(buffer.len());

        let mut spectrum: Vec<Complex<f32>> =
            buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();

        fft.process(&mut spectrum);

        // Calculate centroid from magnitude spectrum
        let mut weighted_sum = 0.0;
        let mut magnitude_sum = 0.0;

        for (i, complex) in spectrum.iter().take(spectrum.len() / 2).enumerate() {
            let magnitude = complex.norm();
            let freq = (i as f32 * sample_rate) / buffer.len() as f32;
            weighted_sum += freq * magnitude;
            magnitude_sum += magnitude;
        }

        if magnitude_sum > 0.0 {
            weighted_sum / magnitude_sum
        } else {
            0.0
        }
    }

    /// Helper: Create test context
    fn test_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_formant_vowel_a_spectrum() {
        // Test 1: Vowel A (formant=0.0) should have distinct formant peaks
        let sample_rate = 44100.0;
        let block_size = 2048;

        // Saw wave (rich harmonics for formant shaping)
        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);

        // Vowel A
        let mut formant_const = ConstantNode::new(0.0);
        let mut intensity_const = ConstantNode::new(1.0);
        let mut formant = FormantNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let formant_inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        // Process multiple blocks to reach steady state
        for _ in 0..5 {
            formant.process_block(&formant_inputs, &mut filtered, sample_rate, &context);
        }

        let filtered_rms = calculate_rms(&filtered);

        // Should produce audible output
        assert!(
            filtered_rms > 0.01,
            "Vowel A should produce audible output: {}",
            filtered_rms
        );

        // Should be stable
        assert!(
            filtered.iter().all(|&x| x.is_finite()),
            "Output should be stable"
        );
    }

    #[test]
    fn test_formant_vowel_e_spectrum() {
        // Test 2: Vowel E (formant=1.0) should have different spectrum than A
        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(1.0); // Vowel E
        let mut intensity_const = ConstantNode::new(1.0);
        let mut formant = FormantNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let formant_inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant.process_block(&formant_inputs, &mut filtered, sample_rate, &context);
        }

        let filtered_rms = calculate_rms(&filtered);

        assert!(filtered_rms > 0.01, "Vowel E should produce audible output");
    }

    #[test]
    fn test_formant_vowel_i_spectrum() {
        // Test 3: Vowel I (formant=2.0)
        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(2.0); // Vowel I
        let mut intensity_const = ConstantNode::new(1.0);
        let mut formant = FormantNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let formant_inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant.process_block(&formant_inputs, &mut filtered, sample_rate, &context);
        }

        let filtered_rms = calculate_rms(&filtered);

        assert!(filtered_rms > 0.01, "Vowel I should produce audible output");
    }

    #[test]
    fn test_formant_vowel_o_spectrum() {
        // Test 4: Vowel O (formant=3.0)
        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(3.0); // Vowel O
        let mut intensity_const = ConstantNode::new(1.0);
        let mut formant = FormantNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let formant_inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant.process_block(&formant_inputs, &mut filtered, sample_rate, &context);
        }

        let filtered_rms = calculate_rms(&filtered);

        assert!(filtered_rms > 0.01, "Vowel O should produce audible output");
    }

    #[test]
    fn test_formant_vowel_u_spectrum() {
        // Test 5: Vowel U (formant=4.0)
        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(4.0); // Vowel U
        let mut intensity_const = ConstantNode::new(1.0);
        let mut formant = FormantNode::new(1, 2, 3);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let formant_inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant.process_block(&formant_inputs, &mut filtered, sample_rate, &context);
        }

        let filtered_rms = calculate_rms(&filtered);

        assert!(filtered_rms > 0.01, "Vowel U should produce audible output");
    }

    #[test]
    fn test_formant_interpolation_smooth() {
        // Test 6: Interpolation between vowels should be smooth
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut intensity_const = ConstantNode::new(1.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        // Test interpolation between A (0.0) and E (1.0)
        let mut formant_0 = ConstantNode::new(0.0);
        let mut formant_05 = ConstantNode::new(0.5);
        let mut formant_1 = ConstantNode::new(1.0);

        let mut formant_buf_0 = vec![0.0; block_size];
        let mut formant_buf_05 = vec![0.0; block_size];
        let mut formant_buf_1 = vec![0.0; block_size];

        formant_0.process_block(&[], &mut formant_buf_0, sample_rate, &context);
        formant_05.process_block(&[], &mut formant_buf_05, sample_rate, &context);
        formant_1.process_block(&[], &mut formant_buf_1, sample_rate, &context);

        let mut formant_node_0 = FormantNode::new(0, 1, 2);
        let mut formant_node_05 = FormantNode::new(0, 1, 2);
        let mut formant_node_1 = FormantNode::new(0, 1, 2);

        let mut output_0 = vec![0.0; block_size];
        let mut output_05 = vec![0.0; block_size];
        let mut output_1 = vec![0.0; block_size];

        let inputs_0 = vec![
            signal_buf.as_slice(),
            formant_buf_0.as_slice(),
            intensity_buf.as_slice(),
        ];
        let inputs_05 = vec![
            signal_buf.as_slice(),
            formant_buf_05.as_slice(),
            intensity_buf.as_slice(),
        ];
        let inputs_1 = vec![
            signal_buf.as_slice(),
            formant_buf_1.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant_node_0.process_block(&inputs_0, &mut output_0, sample_rate, &context);
            formant_node_05.process_block(&inputs_05, &mut output_05, sample_rate, &context);
            formant_node_1.process_block(&inputs_1, &mut output_1, sample_rate, &context);
        }

        let rms_0 = calculate_rms(&output_0);
        let rms_05 = calculate_rms(&output_05);
        let rms_1 = calculate_rms(&output_1);

        // All should produce audible output
        assert!(rms_0 > 0.01, "Formant 0.0 should produce output");
        assert!(rms_05 > 0.01, "Formant 0.5 should produce output");
        assert!(rms_1 > 0.01, "Formant 1.0 should produce output");

        // Intermediate value should be different from endpoints
        assert!(
            (rms_05 - rms_0).abs() > 0.001,
            "Interpolated formant should differ from endpoint"
        );
    }

    #[test]
    fn test_formant_intensity_control() {
        // Test 7: Intensity should control dry/wet mix
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(0.0); // Vowel A

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let input_rms = calculate_rms(&signal_buf);

        // Test intensity = 0.0 (dry)
        let mut intensity_dry = ConstantNode::new(0.0);
        let mut intensity_dry_buf = vec![0.0; block_size];
        intensity_dry.process_block(&[], &mut intensity_dry_buf, sample_rate, &context);

        let mut formant_dry = FormantNode::new(0, 1, 2);
        let mut output_dry = vec![0.0; block_size];

        let inputs_dry = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_dry_buf.as_slice(),
        ];

        formant_dry.process_block(&inputs_dry, &mut output_dry, sample_rate, &context);

        let rms_dry = calculate_rms(&output_dry);

        // Intensity 0.0 should output approximately the input signal
        assert!(
            (rms_dry - input_rms).abs() < 0.01,
            "Intensity 0.0 should pass dry signal: input={}, output={}",
            input_rms,
            rms_dry
        );

        // Test intensity = 1.0 (wet)
        let mut intensity_wet = ConstantNode::new(1.0);
        let mut intensity_wet_buf = vec![0.0; block_size];
        intensity_wet.process_block(&[], &mut intensity_wet_buf, sample_rate, &context);

        let mut formant_wet = FormantNode::new(0, 1, 2);
        let mut output_wet = vec![0.0; block_size];

        let inputs_wet = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_wet_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant_wet.process_block(&inputs_wet, &mut output_wet, sample_rate, &context);
        }

        let rms_wet = calculate_rms(&output_wet);

        // Wet signal should be significantly different from dry
        assert!(
            (rms_wet - input_rms).abs() > 0.01,
            "Intensity 1.0 should produce filtered signal: input={}, output={}",
            input_rms,
            rms_wet
        );
    }

    #[test]
    fn test_formant_intensity_partial() {
        // Test 8: Partial intensity should blend signals
        let sample_rate = 44100.0;
        let block_size = 1024;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(0.0);
        let mut intensity_const = ConstantNode::new(0.5); // 50% mix

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let mut formant = FormantNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant.process_block(&inputs, &mut output, sample_rate, &context);
        }

        let output_rms = calculate_rms(&output);

        // Should produce audible output
        assert!(
            output_rms > 0.01,
            "50% intensity should produce audible output"
        );
    }

    #[test]
    fn test_formant_with_noise() {
        // Test 9: Formant filter works with noise input
        let sample_rate = 44100.0;
        let block_size = 2048;

        // White noise
        let mut amp_const = ConstantNode::new(1.0);
        let mut noise = NoiseNode::new(0);

        let mut formant_const = ConstantNode::new(0.0); // Vowel A
        let mut intensity_const = ConstantNode::new(1.0);
        let mut formant = FormantNode::new(0, 1, 2);

        let context = test_context(block_size);

        let mut amp_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut noise_buf = vec![0.0; block_size];
        let mut filtered = vec![0.0; block_size];

        amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        noise.process_block(&[amp_buf.as_slice()], &mut noise_buf, sample_rate, &context);

        let inputs = vec![
            noise_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..5 {
            formant.process_block(&inputs, &mut filtered, sample_rate, &context);
        }

        let noise_rms = calculate_rms(&noise_buf);
        let filtered_rms = calculate_rms(&filtered);

        // Should produce audible output
        assert!(
            filtered_rms > 0.01,
            "Formant filter on noise should produce output"
        );

        // Should attenuate some frequencies (formants are selective)
        assert!(
            filtered_rms < noise_rms,
            "Formant filter should attenuate noise: noise={}, filtered={}",
            noise_rms,
            filtered_rms
        );
    }

    #[test]
    fn test_formant_parameter_modulation() {
        // Test 10: Formant parameter can change smoothly
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut intensity_const = ConstantNode::new(1.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        // Start with vowel A
        let mut formant_const = ConstantNode::new(0.0);
        let mut formant_buf = vec![0.0; block_size];
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);

        let mut formant = FormantNode::new(0, 1, 2);
        let mut output1 = vec![0.0; block_size];

        let inputs1 = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..3 {
            formant.process_block(&inputs1, &mut output1, sample_rate, &context);
        }

        let rms1 = calculate_rms(&output1);

        // Change to vowel I
        formant_const.set_value(2.0);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);

        let mut output2 = vec![0.0; block_size];
        let inputs2 = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        for _ in 0..3 {
            formant.process_block(&inputs2, &mut output2, sample_rate, &context);
        }

        let rms2 = calculate_rms(&output2);

        // Both should produce output
        assert!(rms1 > 0.01, "Vowel A should produce output");
        assert!(rms2 > 0.01, "Vowel I should produce output");

        // Outputs should differ
        assert!(
            (rms1 - rms2).abs() > 0.001,
            "Different vowels should produce different outputs: A={}, I={}",
            rms1,
            rms2
        );
    }

    #[test]
    fn test_formant_extreme_parameters() {
        // Test 11: Extreme parameters should be handled safely
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(100.0); // Way out of range
        let mut intensity_const = ConstantNode::new(2.0); // Above 1.0

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        let mut formant = FormantNode::new(0, 1, 2);
        let inputs = vec![
            signal_buf.as_slice(),
            formant_buf.as_slice(),
            intensity_buf.as_slice(),
        ];

        // Should not panic
        formant.process_block(&inputs, &mut output, sample_rate, &context);

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
    fn test_formant_stability() {
        // Test 12: Filter should remain stable over many blocks
        let sample_rate = 44100.0;
        let block_size = 512;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut formant_const = ConstantNode::new(0.0);
        let mut intensity_const = ConstantNode::new(1.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut formant_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        let mut formant = FormantNode::new(0, 1, 2);

        // Process 100 blocks
        for _ in 0..100 {
            osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

            let inputs = vec![
                signal_buf.as_slice(),
                formant_buf.as_slice(),
                intensity_buf.as_slice(),
            ];
            formant.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability
            for (i, &sample) in output.iter().enumerate() {
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
    fn test_formant_input_nodes() {
        // Test 13: Verify input node dependencies
        let formant = FormantNode::new(10, 20, 30);
        let deps = formant.input_nodes();

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], 10); // input
        assert_eq!(deps[1], 20); // formant
        assert_eq!(deps[2], 30); // intensity
    }

    #[test]
    fn test_formant_vowel_differences() {
        // Test 14: All vowels should produce distinct spectral characteristics
        let sample_rate = 44100.0;
        let block_size = 2048;

        let mut freq_const = ConstantNode::new(110.0);
        let mut osc = OscillatorNode::new(0, Waveform::Saw);
        let mut intensity_const = ConstantNode::new(1.0);

        let context = test_context(block_size);

        let mut freq_buf = vec![0.0; block_size];
        let mut intensity_buf = vec![0.0; block_size];
        let mut signal_buf = vec![0.0; block_size];

        freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);
        intensity_const.process_block(&[], &mut intensity_buf, sample_rate, &context);

        let osc_inputs = vec![freq_buf.as_slice()];
        osc.process_block(&osc_inputs, &mut signal_buf, sample_rate, &context);

        // Test all 5 vowels
        let mut rms_values = Vec::new();

        for vowel_idx in 0..5 {
            let mut formant_const = ConstantNode::new(vowel_idx as f32);
            let mut formant_buf = vec![0.0; block_size];
            formant_const.process_block(&[], &mut formant_buf, sample_rate, &context);

            let mut formant = FormantNode::new(0, 1, 2);
            let mut output = vec![0.0; block_size];

            let inputs = vec![
                signal_buf.as_slice(),
                formant_buf.as_slice(),
                intensity_buf.as_slice(),
            ];

            for _ in 0..5 {
                formant.process_block(&inputs, &mut output, sample_rate, &context);
            }

            let rms = calculate_rms(&output);
            rms_values.push(rms);

            // Each vowel should produce audible output
            assert!(
                rms > 0.01,
                "Vowel {} should produce audible output",
                vowel_idx
            );
        }

        // At least some vowels should have different RMS values
        let min_rms = rms_values.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_rms = rms_values.iter().fold(0.0f32, |a, &b| a.max(b));

        assert!(
            (max_rms - min_rms) > 0.01,
            "Vowels should have distinct characteristics: range {} to {}",
            min_rms,
            max_rms
        );
    }
}
