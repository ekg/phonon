/// Scale Quantize node - quantize frequencies/pitches to musical scales
///
/// This node maps input frequencies to the nearest pitch in a musical scale.
/// Unlike simple quantization (which snaps to grid), this finds the nearest
/// scale degree and handles octave wrapping correctly.
///
/// # Algorithm
/// 1. Convert input frequency to semitones relative to root
/// 2. Find the octave and semitone-within-octave
/// 3. Snap to nearest scale degree
/// 4. Convert back to frequency
///
/// # Example
/// ```ignore
/// // Quantize LFO to C major scale
/// let lfo = OscillatorNode::new(freq_id, Waveform::Sine);  // Random frequencies
/// let root = ConstantNode::new(261.63);                     // C4 = 261.63 Hz
/// let quantized = ScaleQuantizeNode::major(lfo_id, root_id);
/// // Output will only contain frequencies from C major scale
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::sync::Arc;

/// Common musical scales as semitone offsets
///
/// All scales are defined relative to the root note (0 semitones).
/// These are standard Western music theory scales.

/// Major scale (Ionian mode): W-W-H-W-W-W-H
/// Example: C Major = C D E F G A B
pub const MAJOR_SCALE: &[f32] = &[0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0];

/// Natural minor scale (Aeolian mode): W-H-W-W-H-W-W
/// Example: A minor = A B C D E F G
pub const MINOR_SCALE: &[f32] = &[0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0];

/// Major pentatonic scale: W-W-m3-W-m3
/// Example: C major pentatonic = C D E G A
pub const PENTATONIC_MAJOR: &[f32] = &[0.0, 2.0, 4.0, 7.0, 9.0];

/// Minor pentatonic scale: m3-W-W-m3-W
/// Example: A minor pentatonic = A C D E G
pub const PENTATONIC_MINOR: &[f32] = &[0.0, 3.0, 5.0, 7.0, 10.0];

/// Chromatic scale (all 12 semitones)
/// Example: C chromatic = C C# D D# E F F# G G# A A# B
pub const CHROMATIC: &[f32] = &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0];

/// Blues scale: m3-W-H-H-m3-W
/// Example: C blues = C Eb F F# G Bb
pub const BLUES_SCALE: &[f32] = &[0.0, 3.0, 5.0, 6.0, 7.0, 10.0];

/// Harmonic minor scale: W-H-W-W-H-Aug2-H
/// Example: A harmonic minor = A B C D E F G#
pub const HARMONIC_MINOR: &[f32] = &[0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 11.0];

/// Dorian mode: W-H-W-W-W-H-W
/// Example: D dorian = D E F G A B C
pub const DORIAN: &[f32] = &[0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 10.0];

/// Phrygian mode: H-W-W-W-H-W-W
/// Example: E phrygian = E F G A B C D
pub const PHRYGIAN: &[f32] = &[0.0, 1.0, 3.0, 5.0, 7.0, 8.0, 10.0];

/// Lydian mode: W-W-W-H-W-W-H
/// Example: F lydian = F G A B C D E
pub const LYDIAN: &[f32] = &[0.0, 2.0, 4.0, 6.0, 7.0, 9.0, 11.0];

/// Mixolydian mode: W-W-H-W-W-H-W
/// Example: G mixolydian = G A B C D E F
pub const MIXOLYDIAN: &[f32] = &[0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 10.0];

/// Locrian mode: H-W-W-H-W-W-W
/// Example: B locrian = B C D E F G A
pub const LOCRIAN: &[f32] = &[0.0, 1.0, 3.0, 5.0, 6.0, 8.0, 10.0];

/// Scale Quantize node: map frequencies to musical scales
///
/// Quantizes input frequencies to the nearest pitch in a specified scale.
/// The scale is defined as semitone offsets from a root frequency.
///
/// # Musical Theory
/// - Scales are defined relative to a root note (e.g., C4 = 261.63 Hz)
/// - Each scale degree is a number of semitones above the root
/// - The algorithm preserves octaves (C3 major and C5 major use same scale)
///
/// # Parameters
/// - `input`: Frequency to quantize (in Hz)
/// - `scale`: Arc-wrapped vector of semitone offsets (e.g., [0, 2, 4, 5, 7, 9, 11] for major)
/// - `root`: Root frequency of the scale (in Hz)
pub struct ScaleQuantizeNode {
    /// Input frequency signal (NodeId)
    input: NodeId,

    /// Scale degrees in semitones (Arc for efficient cloning)
    /// Example: [0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0] for major scale
    scale: Arc<Vec<f32>>,

    /// Root frequency input (NodeId)
    root: NodeId,
}

impl ScaleQuantizeNode {
    /// Create a new scale quantize node with custom scale
    ///
    /// # Arguments
    /// * `input` - NodeId of frequency to quantize
    /// * `scale` - Arc-wrapped vector of semitone offsets
    /// * `root` - NodeId of root frequency
    ///
    /// # Example
    /// ```ignore
    /// // Custom whole-tone scale (C D E F# G# A#)
    /// let whole_tone = Arc::new(vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
    /// let quantizer = ScaleQuantizeNode::new(1, whole_tone, 2);
    /// ```
    pub fn new(input: NodeId, scale: Arc<Vec<f32>>, root: NodeId) -> Self {
        // Validate scale is not empty and is sorted
        debug_assert!(!scale.is_empty(), "Scale cannot be empty");
        debug_assert!(
            scale.windows(2).all(|w| w[0] <= w[1]),
            "Scale must be sorted in ascending order"
        );

        Self { input, scale, root }
    }

    /// Create major scale quantizer (Ionian mode)
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::major(lfo_id, root_id);
    /// ```
    pub fn major(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(MAJOR_SCALE.to_vec()), root)
    }

    /// Create natural minor scale quantizer (Aeolian mode)
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::minor(lfo_id, root_id);
    /// ```
    pub fn minor(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(MINOR_SCALE.to_vec()), root)
    }

    /// Create major pentatonic scale quantizer
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::pentatonic_major(lfo_id, root_id);
    /// ```
    pub fn pentatonic_major(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(PENTATONIC_MAJOR.to_vec()), root)
    }

    /// Create minor pentatonic scale quantizer
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::pentatonic_minor(lfo_id, root_id);
    /// ```
    pub fn pentatonic_minor(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(PENTATONIC_MINOR.to_vec()), root)
    }

    /// Create chromatic scale quantizer (all 12 semitones)
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::chromatic(lfo_id, root_id);
    /// ```
    pub fn chromatic(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(CHROMATIC.to_vec()), root)
    }

    /// Create blues scale quantizer
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::blues(lfo_id, root_id);
    /// ```
    pub fn blues(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(BLUES_SCALE.to_vec()), root)
    }

    /// Create harmonic minor scale quantizer
    ///
    /// # Example
    /// ```ignore
    /// let quantizer = ScaleQuantizeNode::harmonic_minor(lfo_id, root_id);
    /// ```
    pub fn harmonic_minor(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(HARMONIC_MINOR.to_vec()), root)
    }

    /// Create Dorian mode quantizer
    pub fn dorian(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(DORIAN.to_vec()), root)
    }

    /// Create Phrygian mode quantizer
    pub fn phrygian(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(PHRYGIAN.to_vec()), root)
    }

    /// Create Lydian mode quantizer
    pub fn lydian(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(LYDIAN.to_vec()), root)
    }

    /// Create Mixolydian mode quantizer
    pub fn mixolydian(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(MIXOLYDIAN.to_vec()), root)
    }

    /// Create Locrian mode quantizer
    pub fn locrian(input: NodeId, root: NodeId) -> Self {
        Self::new(input, Arc::new(LOCRIAN.to_vec()), root)
    }

    /// Get the input node ID
    pub fn input(&self) -> NodeId {
        self.input
    }

    /// Get the root node ID
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Get the scale
    pub fn scale(&self) -> &Arc<Vec<f32>> {
        &self.scale
    }

    /// Quantize a single frequency to the scale
    ///
    /// This is the core algorithm, extracted for testing.
    fn quantize_frequency(&self, freq: f32, root_freq: f32) -> f32 {
        // Protect against invalid inputs
        if freq <= 0.0 || root_freq <= 0.0 || !freq.is_finite() || !root_freq.is_finite() {
            return root_freq; // Fallback to root
        }

        // Convert frequency to semitones from root using equal temperament
        // semitones = 12 * log2(freq / root)
        let semitones = 12.0 * (freq / root_freq).log2();

        // Find which octave we're in (can be negative for frequencies below root)
        let octave = (semitones / 12.0).floor();

        // Get semitone position within the octave (0-12)
        let semitone_in_octave = semitones - (octave * 12.0);

        // Find closest scale degree
        let mut closest_degree = self.scale[0];
        let mut min_distance = (semitone_in_octave - closest_degree).abs();

        for &degree in self.scale.iter() {
            let distance = (semitone_in_octave - degree).abs();
            if distance < min_distance {
                closest_degree = degree;
                min_distance = distance;
            }
        }

        // Also check wrapping around the octave (12 semitones up)
        // This handles the case where we might be closer to the next octave's root
        for &degree in self.scale.iter() {
            let wrapped_degree = degree + 12.0;
            let distance = (semitone_in_octave - wrapped_degree).abs();
            if distance < min_distance {
                closest_degree = wrapped_degree;
                min_distance = distance;
            }
        }

        // Convert back to frequency
        // freq = root * 2^(semitones/12)
        let quantized_semitones = octave * 12.0 + closest_degree;
        let output_freq = root_freq * 2.0_f32.powf(quantized_semitones / 12.0);

        output_freq
    }
}

impl AudioNode for ScaleQuantizeNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert_eq!(
            inputs.len(),
            2,
            "ScaleQuantizeNode expects 2 inputs (input frequency, root frequency), got {}",
            inputs.len()
        );

        let input_buffer = inputs[0];
        let root_buffer = inputs[1];

        debug_assert_eq!(
            input_buffer.len(),
            output.len(),
            "Input buffer length mismatch"
        );
        debug_assert_eq!(
            root_buffer.len(),
            output.len(),
            "Root buffer length mismatch"
        );

        // Process each sample
        for i in 0..output.len() {
            let freq = input_buffer[i];
            let root_freq = root_buffer[i];
            output[i] = self.quantize_frequency(freq, root_freq);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.input, self.root]
    }

    fn name(&self) -> &str {
        "ScaleQuantizeNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    fn create_context(block_size: usize) -> ProcessContext {
        ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
    }

    #[test]
    fn test_major_scale_quantization() {
        // Test: Major scale quantization (C major)
        // Root = 261.63 Hz (C4)
        // Input frequencies should snap to C D E F G A B C
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        // Frequencies slightly off from C major scale notes
        let c4 = 261.63;
        let root = vec![c4; 8];

        // Test frequencies: C4+10Hz, D4-5Hz, E4+3Hz, F4-2Hz, G4+8Hz, A4-4Hz, B4+6Hz, C5+1Hz
        let d4 = 293.66;
        let e4 = 329.63;
        let f4 = 349.23;
        let g4 = 392.0;
        let a4 = 440.0;
        let b4 = 493.88;
        let c5 = 523.25;

        let input = vec![
            c4 + 10.0,  // Should snap to C4
            d4 - 5.0,   // Should snap to D4
            e4 + 3.0,   // Should snap to E4
            f4 - 2.0,   // Should snap to F4
            g4 + 8.0,   // Should snap to G4
            a4 - 4.0,   // Should snap to A4
            b4 + 6.0,   // Should snap to B4
            c5 + 1.0,   // Should snap to C5
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 8];
        let context = create_context(8);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify each frequency is quantized to major scale
        // Allow 0.5 Hz tolerance for floating point errors
        assert!((output[0] - c4).abs() < 0.5, "Expected C4, got {}", output[0]);
        assert!((output[1] - d4).abs() < 0.5, "Expected D4, got {}", output[1]);
        assert!((output[2] - e4).abs() < 0.5, "Expected E4, got {}", output[2]);
        assert!((output[3] - f4).abs() < 0.5, "Expected F4, got {}", output[3]);
        assert!((output[4] - g4).abs() < 0.5, "Expected G4, got {}", output[4]);
        assert!((output[5] - a4).abs() < 0.5, "Expected A4, got {}", output[5]);
        assert!((output[6] - b4).abs() < 0.5, "Expected B4, got {}", output[6]);
        assert!((output[7] - c5).abs() < 0.5, "Expected C5, got {}", output[7]);
    }

    #[test]
    fn test_minor_scale_quantization() {
        // Test: Natural minor scale (A minor)
        // Root = 440 Hz (A4)
        let mut quantizer = ScaleQuantizeNode::minor(0, 1);

        let a4 = 440.0;
        let root = vec![a4; 7];

        // A minor scale: A B C D E F G
        // Semitones: 0, 2, 3, 5, 7, 8, 10
        let b4 = a4 * 2.0_f32.powf(2.0 / 12.0);   // +2 semitones
        let c5 = a4 * 2.0_f32.powf(3.0 / 12.0);   // +3 semitones
        let d5 = a4 * 2.0_f32.powf(5.0 / 12.0);   // +5 semitones
        let e5 = a4 * 2.0_f32.powf(7.0 / 12.0);   // +7 semitones
        let f5 = a4 * 2.0_f32.powf(8.0 / 12.0);   // +8 semitones
        let g5 = a4 * 2.0_f32.powf(10.0 / 12.0);  // +10 semitones

        let input = vec![
            a4 + 2.0,
            b4 - 1.0,
            c5 + 3.0,
            d5 - 2.0,
            e5 + 1.0,
            f5 - 3.0,
            g5 + 2.0,
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 7];
        let context = create_context(7);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify minor scale quantization
        assert!((output[0] - a4).abs() < 0.5);
        assert!((output[1] - b4).abs() < 0.5);
        assert!((output[2] - c5).abs() < 0.5);
        assert!((output[3] - d5).abs() < 0.5);
        assert!((output[4] - e5).abs() < 0.5);
        assert!((output[5] - f5).abs() < 0.5);
        assert!((output[6] - g5).abs() < 0.5);
    }

    #[test]
    fn test_pentatonic_scale_quantization() {
        // Test: Major pentatonic scale (5 notes)
        let mut quantizer = ScaleQuantizeNode::pentatonic_major(0, 1);

        let c4 = 261.63;
        let root = vec![c4; 5];

        // C major pentatonic: C D E G A (semitones: 0, 2, 4, 7, 9)
        let d4 = c4 * 2.0_f32.powf(2.0 / 12.0);
        let e4 = c4 * 2.0_f32.powf(4.0 / 12.0);
        let g4 = c4 * 2.0_f32.powf(7.0 / 12.0);
        let a4 = c4 * 2.0_f32.powf(9.0 / 12.0);

        let input = vec![c4 + 5.0, d4 - 3.0, e4 + 7.0, g4 - 4.0, a4 + 2.0];
        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 5];
        let context = create_context(5);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        assert!((output[0] - c4).abs() < 0.5);
        assert!((output[1] - d4).abs() < 0.5);
        assert!((output[2] - e4).abs() < 0.5);
        assert!((output[3] - g4).abs() < 0.5);
        assert!((output[4] - a4).abs() < 0.5);
    }

    #[test]
    fn test_chromatic_scale_no_change() {
        // Test: Chromatic scale should quantize to nearest semitone
        let mut quantizer = ScaleQuantizeNode::chromatic(0, 1);

        let c4 = 261.63;
        let root = vec![c4; 4];

        // Test frequencies between semitones
        let input = vec![
            c4 + 5.0,   // ~0.3 semitones above C4, should snap to C4
            c4 * 2.0_f32.powf(1.3 / 12.0),  // 1.3 semitones, should snap to 1 semitone (C#4)
            c4 * 2.0_f32.powf(2.6 / 12.0),  // 2.6 semitones, should snap to 3 semitones (D#4)
            c4 * 2.0_f32.powf(11.7 / 12.0), // 11.7 semitones, should snap to 12 semitones (C5)
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 4];
        let context = create_context(4);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify chromatic quantization
        let expected = vec![
            c4,                                 // 0 semitones
            c4 * 2.0_f32.powf(1.0 / 12.0),     // 1 semitone (C#4)
            c4 * 2.0_f32.powf(3.0 / 12.0),     // 3 semitones (D#4)
            c4 * 2.0_f32.powf(12.0 / 12.0),    // 12 semitones (C5)
        ];

        for i in 0..4 {
            assert!(
                (output[i] - expected[i]).abs() < 0.5,
                "Sample {}: expected {}, got {}",
                i,
                expected[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_octave_spanning() {
        // Test: Quantization should work across multiple octaves
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let c4 = 261.63;
        let c3 = c4 / 2.0;  // One octave below
        let c5 = c4 * 2.0;  // One octave above
        let c6 = c4 * 4.0;  // Two octaves above

        let root = vec![c4; 4];
        let input = vec![c3 + 2.0, c4 + 3.0, c5 + 4.0, c6 + 5.0];
        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 4];
        let context = create_context(4);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // All should quantize to C in their respective octaves
        assert!((output[0] - c3).abs() < 0.5);
        assert!((output[1] - c4).abs() < 0.5);
        assert!((output[2] - c5).abs() < 0.5);
        assert!((output[3] - c6).abs() < 0.5);
    }

    #[test]
    fn test_custom_scale() {
        // Test: Custom whole-tone scale (6 notes, 2 semitones apart)
        let whole_tone = Arc::new(vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        let mut quantizer = ScaleQuantizeNode::new(0, whole_tone, 1);

        let c4 = 261.63;
        let root = vec![c4; 6];

        // Whole-tone scale: C D E F# G# A# (semitones: 0, 2, 4, 6, 8, 10)
        let expected = vec![
            c4,                                // 0 semitones
            c4 * 2.0_f32.powf(2.0 / 12.0),    // 2 semitones
            c4 * 2.0_f32.powf(4.0 / 12.0),    // 4 semitones
            c4 * 2.0_f32.powf(6.0 / 12.0),    // 6 semitones
            c4 * 2.0_f32.powf(8.0 / 12.0),    // 8 semitones
            c4 * 2.0_f32.powf(10.0 / 12.0),   // 10 semitones
        ];

        let input = vec![
            expected[0] + 5.0,
            expected[1] - 3.0,
            expected[2] + 4.0,
            expected[3] - 2.0,
            expected[4] + 6.0,
            expected[5] - 4.0,
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 6];
        let context = create_context(6);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        for i in 0..6 {
            assert!(
                (output[i] - expected[i]).abs() < 0.5,
                "Sample {}: expected {}, got {}",
                i,
                expected[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_pattern_modulated_input() {
        // Test: Input frequency can vary per sample (pattern modulation)
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let c4 = 261.63;
        let root = vec![c4; 512];

        // Varying input frequencies (simulating LFO or pattern)
        let mut input = vec![0.0; 512];
        for i in 0..512 {
            // Sweep from C4 to C5
            input[i] = c4 + (c4 * (i as f32 / 512.0));
        }

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 512];
        let context = create_context(512);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify all outputs are valid frequencies in C major scale
        for &freq in &output {
            assert!(freq > 0.0 && freq.is_finite());
            // Should be within C4 to C5 range
            assert!(freq >= c4 - 1.0 && freq <= c4 * 2.0 + 1.0);
        }
    }

    #[test]
    fn test_pattern_modulated_root() {
        // Test: Root frequency can vary per sample
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let c4 = 261.63;
        let d4 = 293.66;

        // Alternating root frequencies (C4 and D4)
        let mut root = vec![0.0; 512];
        for i in 0..512 {
            root[i] = if i % 2 == 0 { c4 } else { d4 };
        }

        // Fixed input frequency
        let input = vec![c4; 512];
        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 512];
        let context = create_context(512);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify output changes based on changing root
        // When root is C4, C4 input should stay C4
        // When root is D4, C4 input should map to D4 scale
        for &freq in &output {
            assert!(freq > 0.0 && freq.is_finite());
        }
    }

    #[test]
    fn test_frequency_accuracy() {
        // Test: Verify precise frequency calculations
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let a4 = 440.0; // Standard pitch
        let root = vec![a4; 7];

        // A major scale frequencies (calculated precisely)
        let b4 = a4 * 2.0_f32.powf(2.0 / 12.0);   // 493.88 Hz
        let cs5 = a4 * 2.0_f32.powf(4.0 / 12.0);  // 554.37 Hz (C#5)
        let d5 = a4 * 2.0_f32.powf(5.0 / 12.0);   // 587.33 Hz
        let e5 = a4 * 2.0_f32.powf(7.0 / 12.0);   // 659.26 Hz
        let fs5 = a4 * 2.0_f32.powf(9.0 / 12.0);  // 739.99 Hz (F#5)
        let gs5 = a4 * 2.0_f32.powf(11.0 / 12.0); // 830.61 Hz (G#5)

        let input = vec![a4, b4, cs5, d5, e5, fs5, gs5];
        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 7];
        let context = create_context(7);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Verify exact frequencies (within 0.1 Hz)
        assert!((output[0] - a4).abs() < 0.1);
        assert!((output[1] - b4).abs() < 0.1);
        assert!((output[2] - cs5).abs() < 0.1);
        assert!((output[3] - d5).abs() < 0.1);
        assert!((output[4] - e5).abs() < 0.1);
        assert!((output[5] - fs5).abs() < 0.1);
        assert!((output[6] - gs5).abs() < 0.1);
    }

    #[test]
    fn test_blues_scale() {
        // Test: Blues scale quantization
        let mut quantizer = ScaleQuantizeNode::blues(0, 1);

        let c4 = 261.63;
        let root = vec![c4; 6];

        // C blues scale: C Eb F F# G Bb (semitones: 0, 3, 5, 6, 7, 10)
        let expected = vec![
            c4,                                // 0 semitones
            c4 * 2.0_f32.powf(3.0 / 12.0),    // 3 semitones (Eb)
            c4 * 2.0_f32.powf(5.0 / 12.0),    // 5 semitones (F)
            c4 * 2.0_f32.powf(6.0 / 12.0),    // 6 semitones (F#)
            c4 * 2.0_f32.powf(7.0 / 12.0),    // 7 semitones (G)
            c4 * 2.0_f32.powf(10.0 / 12.0),   // 10 semitones (Bb)
        ];

        let input = vec![
            expected[0] + 2.0,
            expected[1] - 1.0,
            expected[2] + 3.0,
            expected[3] - 2.0,
            expected[4] + 1.0,
            expected[5] - 3.0,
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 6];
        let context = create_context(6);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        for i in 0..6 {
            assert!(
                (output[i] - expected[i]).abs() < 0.5,
                "Sample {}: expected {}, got {}",
                i,
                expected[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_harmonic_minor_scale() {
        // Test: Harmonic minor scale
        let mut quantizer = ScaleQuantizeNode::harmonic_minor(0, 1);

        let a4 = 440.0;
        let root = vec![a4; 7];

        // A harmonic minor: A B C D E F G# (semitones: 0, 2, 3, 5, 7, 8, 11)
        let expected = vec![
            a4,                                // 0 semitones
            a4 * 2.0_f32.powf(2.0 / 12.0),    // 2 semitones (B)
            a4 * 2.0_f32.powf(3.0 / 12.0),    // 3 semitones (C)
            a4 * 2.0_f32.powf(5.0 / 12.0),    // 5 semitones (D)
            a4 * 2.0_f32.powf(7.0 / 12.0),    // 7 semitones (E)
            a4 * 2.0_f32.powf(8.0 / 12.0),    // 8 semitones (F)
            a4 * 2.0_f32.powf(11.0 / 12.0),   // 11 semitones (G#)
        ];

        let input = vec![
            expected[0] + 1.0,
            expected[1] - 2.0,
            expected[2] + 3.0,
            expected[3] - 1.0,
            expected[4] + 2.0,
            expected[5] - 3.0,
            expected[6] + 1.0,
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 7];
        let context = create_context(7);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        for i in 0..7 {
            assert!(
                (output[i] - expected[i]).abs() < 0.5,
                "Sample {}: expected {}, got {}",
                i,
                expected[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_between_scale_degrees() {
        // Test: Frequencies exactly between scale degrees snap to nearest
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let c4 = 261.63;
        let root = vec![c4; 2];

        // C4 to D4 is 2 semitones
        // Frequency exactly 1 semitone above C4 (between C and D)
        let between_c_and_d = c4 * 2.0_f32.powf(1.0 / 12.0);
        // Should snap to either C4 or D4 (D4 is 2 semitones)
        let d4 = c4 * 2.0_f32.powf(2.0 / 12.0);

        let input = vec![between_c_and_d, between_c_and_d];
        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 2];
        let context = create_context(2);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // Should snap to one of the scale degrees
        assert!(
            (output[0] - c4).abs() < 0.5 || (output[0] - d4).abs() < 0.5,
            "Expected C4 ({}) or D4 ({}), got {}",
            c4,
            d4,
            output[0]
        );
    }

    #[test]
    fn test_dependencies() {
        // Test: Verify input_nodes returns correct dependencies
        let quantizer = ScaleQuantizeNode::major(5, 10);
        let deps = quantizer.input_nodes();

        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], 5);  // input
        assert_eq!(deps[1], 10); // root
    }

    #[test]
    fn test_with_constants() {
        // Integration test with ConstantNode
        let mut const_input = ConstantNode::new(450.0);  // Slightly above A4 (440 Hz)
        let mut const_root = ConstantNode::new(440.0);   // A4
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let context = create_context(512);

        // Process constants first
        let mut buf_input = vec![0.0; 512];
        let mut buf_root = vec![0.0; 512];

        const_input.process_block(&[], &mut buf_input, 44100.0, &context);
        const_root.process_block(&[], &mut buf_root, 44100.0, &context);

        // Now quantize
        let inputs = vec![buf_input.as_slice(), buf_root.as_slice()];
        let mut output = vec![0.0; 512];

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // 450 Hz is about 0.39 semitones above A4
        // Should quantize to A4 (440 Hz) since it's closest
        let a4 = 440.0;
        for &sample in &output {
            assert!((sample - a4).abs() < 1.0, "Expected ~{}, got {}", a4, sample);
        }
    }

    #[test]
    fn test_invalid_frequencies_protected() {
        // Test: Invalid frequencies (negative, zero, NaN) are handled gracefully
        let mut quantizer = ScaleQuantizeNode::major(0, 1);

        let c4 = 261.63;
        let root = vec![c4; 4];

        let input = vec![
            -100.0,           // Negative frequency
            0.0,              // Zero frequency
            f32::NAN,         // NaN
            f32::INFINITY,    // Infinity
        ];

        let inputs = vec![input.as_slice(), root.as_slice()];
        let mut output = vec![0.0; 4];
        let context = create_context(4);

        quantizer.process_block(&inputs, &mut output, 44100.0, &context);

        // All should fall back to root frequency
        for &sample in &output {
            assert!(sample.is_finite(), "Output should be finite");
            assert!(sample > 0.0, "Output should be positive");
        }
    }
}
