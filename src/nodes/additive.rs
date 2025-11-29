/// Additive synthesis node - sum of weighted harmonics
///
/// Additive synthesis creates complex timbres by summing sine wave harmonics.
/// This is the most flexible synthesis method, allowing precise control over
/// each harmonic's amplitude and detuning.
///
/// # Theory
///
/// Additive synthesis is based on Fourier's theorem: any periodic waveform
/// can be constructed by summing sine waves at harmonic frequencies.
///
/// Output = Σ (weight[i] × sin(2π × freq × (i+1) × t + detune[i]))
///
/// where i = 0 to num_harmonics-1
///
/// ## Classic Waveforms via Harmonics
///
/// - **Sine**: Only fundamental (1 harmonic)
/// - **Sawtooth**: All harmonics with 1/n amplitude falloff
/// - **Square**: Odd harmonics only with 1/n falloff
/// - **Triangle**: Odd harmonics with 1/n² falloff
///
/// ## Musical Applications
///
/// - **Pipe organ**: Static harmonic content (drawbars)
/// - **Hammond organ**: Adjustable harmonic "drawbars"
/// - **Bell tones**: Inharmonic partials (detuned harmonics)
/// - **Vocal synthesis**: Formant-based harmonic shaping
/// - **Evolving pads**: Time-varying harmonic weights
///
/// # References
///
/// - Jean-Baptiste Fourier (1822) "Théorie analytique de la chaleur"
/// - Hammond organ (1935) - popularized additive synthesis via drawbars
/// - Julius O. Smith III "Spectral Audio Signal Processing"
/// - Kawai K5 (1987) - digital additive synthesizer
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;
use std::sync::Arc;

/// Maximum number of harmonics to prevent excessive CPU usage
const MAX_HARMONICS: usize = 32;

/// Additive synthesis node with pattern-controlled parameters
///
/// # Example
/// ```ignore
/// // Sawtooth via additive synthesis (8 harmonics)
/// let freq = ConstantNode::new(110.0);           // NodeId 0
/// let num_harms = ConstantNode::new(8.0);         // NodeId 1
/// let weights = Arc::new(vec![1.0, 0.5, 0.33, 0.25, 0.2, 0.17, 0.14, 0.125]);
/// let detune = Arc::new(vec![0.0; 8]);
/// let additive = AdditiveNode::new(0, 1, weights, detune);
/// ```
pub struct AdditiveNode {
    frequency_input: NodeId,         // Fundamental frequency in Hz
    num_harmonics_input: NodeId,     // Number of harmonics to use (1-32)
    harmonic_weights: Arc<Vec<f32>>, // Amplitude of each harmonic (0.0-1.0)
    harmonic_detune: Arc<Vec<f32>>,  // Detune each harmonic in cents
    phases: Vec<f32>,                // Phase for each harmonic (0.0 to 1.0)
}

impl AdditiveNode {
    /// Additive - Sum of harmonics based on Fourier synthesis
    ///
    /// Generates complex timbres by summing sine waves at harmonic frequencies with
    /// individually controlled amplitude and detuning. Classic approach used in pipe organs,
    /// Hammond organs, and digital synthesizers.
    ///
    /// # Parameters
    /// - `frequency_input`: Fundamental frequency in Hz
    /// - `num_harmonics_input`: Number of harmonics to use (1-32)
    /// - `harmonic_weights`: Amplitude of each harmonic (0.0-1.0)
    /// - `harmonic_detune`: Detune each harmonic in cents (default: 0.0)
    ///
    /// # Example
    /// ```phonon
    /// ~freq: sine 0.25 * 1000 + 110
    /// ~additive: ~freq # additive 8 [1.0, 0.5, 0.33, 0.25, 0.2, 0.17, 0.14, 0.125]
    /// ```
    pub fn new(
        frequency_input: NodeId,
        num_harmonics_input: NodeId,
        harmonic_weights: Arc<Vec<f32>>,
        harmonic_detune: Arc<Vec<f32>>,
    ) -> Self {
        Self {
            frequency_input,
            num_harmonics_input,
            harmonic_weights,
            harmonic_detune,
            phases: vec![0.0; MAX_HARMONICS],
        }
    }

    /// Create a sawtooth wave using additive synthesis
    ///
    /// Sawtooth = Σ (1/n) × sin(2πnft) for n = 1, 2, 3, ...
    pub fn sawtooth(frequency_input: NodeId, num_harmonics_input: NodeId) -> Self {
        let weights: Vec<f32> = (1..=MAX_HARMONICS).map(|i| 1.0 / i as f32).collect();
        let detune = vec![0.0; MAX_HARMONICS];

        Self::new(
            frequency_input,
            num_harmonics_input,
            Arc::new(weights),
            Arc::new(detune),
        )
    }

    /// Create a square wave using additive synthesis
    ///
    /// Square = Σ (1/n) × sin(2πnft) for n = 1, 3, 5, 7, ... (odd harmonics only)
    pub fn square(frequency_input: NodeId, num_harmonics_input: NodeId) -> Self {
        let weights: Vec<f32> = (0..MAX_HARMONICS)
            .map(|i| {
                if i % 2 == 0 {
                    // Even indices = odd harmonics (1st, 3rd, 5th, ...)
                    1.0 / (i * 2 + 1) as f32
                } else {
                    0.0
                }
            })
            .collect();
        let detune = vec![0.0; MAX_HARMONICS];

        Self::new(
            frequency_input,
            num_harmonics_input,
            Arc::new(weights),
            Arc::new(detune),
        )
    }

    /// Create a triangle wave using additive synthesis
    ///
    /// Triangle = Σ (1/n²) × sin(2πnft) for n = 1, 3, 5, 7, ... (odd harmonics only)
    pub fn triangle(frequency_input: NodeId, num_harmonics_input: NodeId) -> Self {
        let weights: Vec<f32> = (0..MAX_HARMONICS)
            .map(|i| {
                if i % 2 == 0 {
                    // Even indices = odd harmonics
                    let n = (i * 2 + 1) as f32;
                    1.0 / (n * n)
                } else {
                    0.0
                }
            })
            .collect();
        let detune = vec![0.0; MAX_HARMONICS];

        Self::new(
            frequency_input,
            num_harmonics_input,
            Arc::new(weights),
            Arc::new(detune),
        )
    }

    /// Create a pure sine wave (fundamental only)
    pub fn sine(frequency_input: NodeId, num_harmonics_input: NodeId) -> Self {
        let mut weights = vec![0.0; MAX_HARMONICS];
        weights[0] = 1.0; // Only fundamental
        let detune = vec![0.0; MAX_HARMONICS];

        Self::new(
            frequency_input,
            num_harmonics_input,
            Arc::new(weights),
            Arc::new(detune),
        )
    }

    /// Get current phases (for testing/debugging)
    pub fn phases(&self) -> &[f32] {
        &self.phases
    }

    /// Reset all phases to 0.0
    pub fn reset_phases(&mut self) {
        for phase in &mut self.phases {
            *phase = 0.0;
        }
    }
}

impl AudioNode for AdditiveNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 2,
            "AdditiveNode requires 2 inputs (frequency, num_harmonics)"
        );

        let frequency_buffer = inputs[0];
        let num_harmonics_buffer = inputs[1];

        debug_assert_eq!(
            frequency_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );
        debug_assert_eq!(
            num_harmonics_buffer.len(),
            output.len(),
            "Num harmonics buffer length mismatch"
        );

        // Zero output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        for i in 0..output.len() {
            let fundamental_freq = frequency_buffer[i];
            let num_harms_raw = num_harmonics_buffer[i];

            // Clamp num_harmonics to [1, MAX_HARMONICS]
            let num_harms = num_harms_raw.max(1.0).min(MAX_HARMONICS as f32).round() as usize;

            // Clamp to actual weights length
            let num_harms = num_harms.min(self.harmonic_weights.len());

            // Sum all harmonics
            for h in 0..num_harms {
                let harmonic_num = (h + 1) as f32; // 1st, 2nd, 3rd, ... harmonic
                let harmonic_freq = fundamental_freq * harmonic_num;

                // Apply detuning (cents to frequency ratio)
                // Formula: ratio = 2^(cents/1200)
                let detune_cents = if h < self.harmonic_detune.len() {
                    self.harmonic_detune[h]
                } else {
                    0.0
                };
                let detune_ratio = 2.0_f32.powf(detune_cents / 1200.0);
                let actual_freq = harmonic_freq * detune_ratio;

                // Prevent aliasing: skip harmonics above Nyquist frequency
                if actual_freq >= sample_rate * 0.5 {
                    continue;
                }

                // Generate sine wave for this harmonic
                let phase_radians = self.phases[h] * 2.0 * PI;
                let amplitude = self.harmonic_weights[h];
                output[i] += amplitude * phase_radians.sin();

                // Advance phase
                let phase_increment = actual_freq / sample_rate;
                self.phases[h] += phase_increment;

                // Wrap phase to [0.0, 1.0)
                while self.phases[h] >= 1.0 {
                    self.phases[h] -= 1.0;
                }
                while self.phases[h] < 0.0 {
                    self.phases[h] += 1.0;
                }
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.frequency_input, self.num_harmonics_input]
    }

    fn name(&self) -> &str {
        "AdditiveNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    /// Helper to process additive synth with constant inputs
    fn process_additive(
        fundamental: f32,
        num_harmonics: f32,
        weights: Vec<f32>,
        detune: Vec<f32>,
        buffer_size: usize,
    ) -> Vec<f32> {
        let mut freq_node = ConstantNode::new(fundamental);
        let mut num_node = ConstantNode::new(num_harmonics);
        let mut additive = AdditiveNode::new(0, 1, Arc::new(weights), Arc::new(detune));

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, buffer_size, 2.0, 44100.0);

        // Generate input buffers
        let mut freq_buf = vec![0.0; buffer_size];
        let mut num_buf = vec![0.0; buffer_size];

        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        num_node.process_block(&[], &mut num_buf, 44100.0, &context);

        // Generate additive output
        let inputs = vec![freq_buf.as_slice(), num_buf.as_slice()];
        let mut output = vec![0.0; buffer_size];
        additive.process_block(&inputs, &mut output, 44100.0, &context);

        output
    }

    /// Calculate RMS (Root Mean Square)
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    #[test]
    fn test_additive_single_harmonic() {
        // Single harmonic = pure sine wave
        let weights = vec![1.0];
        let detune = vec![0.0];
        let output = process_additive(440.0, 1.0, weights, detune, 512);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.5,
            "Single harmonic should produce strong signal, RMS: {}",
            rms
        );

        // Check output is in valid range
        for &sample in &output {
            assert!(sample.abs() <= 1.1, "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_additive_multiple_harmonics() {
        // Multiple harmonics create richer sound
        let weights = vec![1.0, 0.5, 0.33, 0.25];
        let detune = vec![0.0; 4];
        let output = process_additive(220.0, 4.0, weights, detune, 1024);

        let rms = calculate_rms(&output);
        assert!(
            rms > 0.5,
            "Multiple harmonics should produce signal, RMS: {}",
            rms
        );
    }

    #[test]
    fn test_additive_sawtooth() {
        // Sawtooth approximation
        let weights: Vec<f32> = (1..=8).map(|i| 1.0 / i as f32).collect();
        let detune = vec![0.0; 8];
        let output = process_additive(110.0, 8.0, weights, detune, 2048);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "Sawtooth should produce signal, RMS: {}", rms);
    }

    #[test]
    fn test_additive_square() {
        // Square approximation (odd harmonics only)
        let weights: Vec<f32> = (0..8)
            .map(|i| {
                if i % 2 == 0 {
                    1.0 / (i * 2 + 1) as f32
                } else {
                    0.0
                }
            })
            .collect();
        let detune = vec![0.0; 8];
        let output = process_additive(110.0, 8.0, weights, detune, 2048);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "Square should produce signal, RMS: {}", rms);
    }

    #[test]
    fn test_additive_triangle() {
        // Triangle approximation (odd harmonics, 1/n² falloff)
        let weights: Vec<f32> = (0..8)
            .map(|i| {
                if i % 2 == 0 {
                    let n = (i * 2 + 1) as f32;
                    1.0 / (n * n)
                } else {
                    0.0
                }
            })
            .collect();
        let detune = vec![0.0; 8];
        let output = process_additive(110.0, 8.0, weights, detune, 2048);

        let rms = calculate_rms(&output);
        assert!(rms > 0.1, "Triangle should produce signal, RMS: {}", rms);
    }

    #[test]
    fn test_additive_detune_works() {
        // Detuning should create beating/richness
        let weights = vec![1.0, 1.0]; // Two equal harmonics
        let detune_none = vec![0.0, 0.0];
        let detune_some = vec![0.0, 10.0]; // Second harmonic detuned +10 cents

        let output_normal = process_additive(220.0, 2.0, weights.clone(), detune_none, 4096);
        let output_detuned = process_additive(220.0, 2.0, weights, detune_some, 4096);

        // Should produce different waveforms
        let different = output_normal
            .iter()
            .zip(&output_detuned)
            .any(|(a, b)| (a - b).abs() > 0.01);

        assert!(different, "Detuning should change the waveform");
    }

    #[test]
    fn test_additive_more_harmonics_brighter() {
        // More harmonics = brighter sound (higher RMS typically)
        let weights_few = vec![1.0, 0.5];
        let weights_many = vec![1.0, 0.5, 0.33, 0.25, 0.2, 0.17, 0.14, 0.125];
        let detune = vec![0.0; 8];

        let output_few = process_additive(220.0, 2.0, weights_few, detune.clone(), 2048);
        let output_many = process_additive(220.0, 8.0, weights_many, detune, 2048);

        let rms_few = calculate_rms(&output_few);
        let rms_many = calculate_rms(&output_many);

        // More harmonics should add energy
        assert!(
            rms_many > rms_few * 0.8,
            "More harmonics should produce more energy: few={}, many={}",
            rms_few,
            rms_many
        );
    }

    #[test]
    fn test_additive_nyquist_protection() {
        // High frequency fundamental with many harmonics
        // Should not produce harmonics above Nyquist (22050 Hz at 44.1kHz SR)
        let weights = vec![1.0; 32];
        let detune = vec![0.0; 32];

        // 10kHz fundamental × 32 harmonics would go to 320kHz
        // Should auto-skip harmonics above Nyquist
        let output = process_additive(10000.0, 32.0, weights, detune, 1024);

        // Should still produce sound (from lower harmonics)
        let rms = calculate_rms(&output);
        assert!(
            rms > 0.3,
            "Should produce sound despite high frequency, RMS: {}",
            rms
        );

        // Should not clip or blow up
        for &sample in &output {
            assert!(sample.abs() <= 2.0, "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_additive_factory_sawtooth() {
        let mut freq_node = ConstantNode::new(110.0);
        let mut num_node = ConstantNode::new(8.0);
        let mut additive = AdditiveNode::sawtooth(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 2048, 2.0, 44100.0);

        let mut freq_buf = vec![0.0; 2048];
        let mut num_buf = vec![0.0; 2048];
        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        num_node.process_block(&[], &mut num_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), num_buf.as_slice()];
        let mut output = vec![0.0; 2048];
        additive.process_block(&inputs, &mut output, 44100.0, &context);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "Factory sawtooth should work, RMS: {}", rms);
    }

    #[test]
    fn test_additive_factory_square() {
        let mut freq_node = ConstantNode::new(110.0);
        let mut num_node = ConstantNode::new(8.0);
        let mut additive = AdditiveNode::square(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 2048, 2.0, 44100.0);

        let mut freq_buf = vec![0.0; 2048];
        let mut num_buf = vec![0.0; 2048];
        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        num_node.process_block(&[], &mut num_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), num_buf.as_slice()];
        let mut output = vec![0.0; 2048];
        additive.process_block(&inputs, &mut output, 44100.0, &context);

        let rms = calculate_rms(&output);
        assert!(rms > 0.3, "Factory square should work, RMS: {}", rms);
    }

    #[test]
    fn test_additive_factory_sine() {
        let mut freq_node = ConstantNode::new(440.0);
        let mut num_node = ConstantNode::new(1.0);
        let mut additive = AdditiveNode::sine(0, 1);

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, 44100.0);

        let mut freq_buf = vec![0.0; 512];
        let mut num_buf = vec![0.0; 512];
        freq_node.process_block(&[], &mut freq_buf, 44100.0, &context);
        num_node.process_block(&[], &mut num_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice(), num_buf.as_slice()];
        let mut output = vec![0.0; 512];
        additive.process_block(&inputs, &mut output, 44100.0, &context);

        let rms = calculate_rms(&output);
        assert!(rms > 0.5, "Factory sine should work, RMS: {}", rms);
    }

    #[test]
    fn test_additive_dependencies() {
        let weights = vec![1.0];
        let detune = vec![0.0];
        let additive = AdditiveNode::new(42, 99, Arc::new(weights), Arc::new(detune));
        let deps = additive.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 42); // frequency
        assert_eq!(deps[1], 99); // num_harmonics
    }

    #[test]
    fn test_additive_phase_advances() {
        let mut additive = AdditiveNode::sine(0, 1);
        assert_eq!(additive.phases()[0], 0.0);

        // Process one sample at 440 Hz
        let freq_buf = vec![440.0];
        let num_buf = vec![1.0];
        let inputs = vec![freq_buf.as_slice(), num_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(Fraction::from_float(0.0), 0, 1, 2.0, 44100.0);

        additive.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should have advanced
        let expected_phase = 440.0 / 44100.0;
        assert!(
            (additive.phases()[0] - expected_phase).abs() < 0.0001,
            "Phase mismatch: got {}, expected {}",
            additive.phases()[0],
            expected_phase
        );
    }
}
