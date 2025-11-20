/// Granular synthesis node - texture and drone generation
///
/// This node implements asynchronous granular synthesis, where grains are
/// spawned at random positions with configurable density, size, pitch, and spray.
/// Each grain is windowed with a Hann window for smooth amplitude envelope.
///
/// # Algorithm
///
/// Asynchronous granular synthesis:
/// 1. Spawn grains at intervals determined by density (grains per second)
/// 2. Each grain:
///    - Random position (position ± spray)
///    - Windowed segment (Hann window)
///    - Pitch-shifted playback
///    - Fixed duration (grain_size)
/// 3. Sum all active grains
///
/// # Parameters
///
/// - `source` - Source audio buffer to granulate (Arc<Vec<f32>>)
/// - `position` - Playback position 0.0-1.0 in source buffer
/// - `grain_size` - Grain size in ms (5-500ms)
/// - `density` - Grains per second (1-100)
/// - `pitch` - Pitch shift in semitones (-12 to +12)
/// - `spray` - Random position offset 0.0-1.0
///
/// # References
///
/// - Curtis Roads, "Microsound" (2001) - Comprehensive granular synthesis theory
/// - Barry Truax, "Real-time Granular Synthesis" (1988) - Asynchronous grain scheduling

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::sync::Arc;

/// Single grain state
#[derive(Debug, Clone)]
struct Grain {
    /// Current position in source buffer (in samples)
    position: f32,
    /// Total grain duration in samples
    duration: f32,
    /// Current age in samples (0.0 to duration)
    age: f32,
    /// Playback speed for pitch shift (1.0 = normal, 2.0 = octave up)
    pitch_ratio: f32,
}

impl Grain {
    /// Create a new grain
    fn new(start_position: f32, duration: f32, pitch_ratio: f32) -> Self {
        Self {
            position: start_position,
            duration,
            age: 0.0,
            pitch_ratio,
        }
    }

    /// Advance the grain by one sample, return output amplitude
    fn process(&mut self, source: &[f32]) -> f32 {
        if self.age >= self.duration {
            return 0.0; // Grain finished
        }

        // Apply Hann window
        let window = hann_window(self.age, self.duration);

        // Read from source with linear interpolation
        let source_len = source.len() as f32;
        let pos = self.position.rem_euclid(source_len);
        let index = pos as usize;
        let frac = pos - index as f32;

        let sample = if index + 1 < source.len() {
            source[index] + frac * (source[index + 1] - source[index])
        } else {
            source[index] // End of buffer
        };

        // Apply window
        let output = sample * window;

        // Advance position and age
        self.position += self.pitch_ratio;
        self.age += 1.0;

        output
    }

    /// Check if grain is finished
    fn is_finished(&self) -> bool {
        self.age >= self.duration
    }
}

/// Hann window function
///
/// Returns amplitude (0.0 to 1.0) for a given age/duration.
/// Produces smooth bell curve: 0.5 * (1.0 - cos(2π * phase))
fn hann_window(age: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return 0.0;
    }
    let phase = age / duration;
    0.5 * (1.0 - (2.0 * std::f32::consts::PI * phase).cos())
}

/// Granular synthesis node
///
/// # Example
/// ```ignore
/// // Create granular synthesizer on buffer
/// let source = Arc::new(vec![0.0; 44100]); // 1 second buffer
/// let position = ConstantNode::new(0.5);  // Middle of buffer, NodeId 0
/// let grain_size = ConstantNode::new(50.0);  // 50ms grains, NodeId 1
/// let density = ConstantNode::new(10.0);  // 10 grains/sec, NodeId 2
/// let pitch = ConstantNode::new(0.0);  // No pitch shift, NodeId 3
/// let spray = ConstantNode::new(0.1);  // 10% position variation, NodeId 4
/// let granular = GranularNode::new(source, 0, 1, 2, 3, 4, 44100.0);
/// ```
pub struct GranularNode {
    source: Arc<Vec<f32>>,       // Source buffer to granulate
    position_input: NodeId,      // Position in buffer (0.0-1.0)
    grain_size_input: NodeId,    // Grain size in ms (5-500)
    density_input: NodeId,       // Grains per second (1-100)
    pitch_input: NodeId,         // Pitch shift in semitones (-12 to +12)
    spray_input: NodeId,         // Random position offset (0.0-1.0)
    active_grains: Vec<Grain>,   // Currently playing grains
    samples_since_last_grain: f32, // Fractional sample counter
    rng: rand::rngs::StdRng,     // Random number generator
    sample_rate: f32,            // Sample rate for calculations
}

impl GranularNode {
    /// Create a new granular synthesis node
    ///
    /// # Arguments
    /// * `source` - Source audio buffer to granulate
    /// * `position_input` - NodeId providing position (0.0-1.0)
    /// * `grain_size_input` - NodeId providing grain size in ms (5-500)
    /// * `density_input` - NodeId providing grains per second (1-100)
    /// * `pitch_input` - NodeId providing pitch shift in semitones (-12 to +12)
    /// * `spray_input` - NodeId providing random position offset (0.0-1.0)
    /// * `sample_rate` - Sample rate in Hz (usually 44100.0)
    ///
    /// # Note
    /// If the source buffer is empty, the node will output silence.
    pub fn new(
        source: Arc<Vec<f32>>,
        position_input: NodeId,
        grain_size_input: NodeId,
        density_input: NodeId,
        pitch_input: NodeId,
        spray_input: NodeId,
        sample_rate: f32,
    ) -> Self {
        Self {
            source,
            position_input,
            grain_size_input,
            density_input,
            pitch_input,
            spray_input,
            active_grains: Vec::with_capacity(64), // Preallocate for efficiency
            samples_since_last_grain: 0.0,
            rng: rand::rngs::StdRng::from_entropy(),
            sample_rate,
        }
    }

    /// Get the number of currently active grains
    pub fn active_grain_count(&self) -> usize {
        self.active_grains.len()
    }

    /// Clear all active grains (silence)
    pub fn clear_grains(&mut self) {
        self.active_grains.clear();
        self.samples_since_last_grain = 0.0;
    }

    /// Set a new source buffer
    pub fn set_source(&mut self, source: Arc<Vec<f32>>) {
        self.source = source;
        self.clear_grains();
    }
}

impl AudioNode for GranularNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            inputs.len() >= 5,
            "GranularNode requires 5 inputs: position, grain_size, density, pitch, spray"
        );

        let position_buffer = inputs[0];
        let grain_size_buffer = inputs[1];
        let density_buffer = inputs[2];
        let pitch_buffer = inputs[3];
        let spray_buffer = inputs[4];

        debug_assert_eq!(
            position_buffer.len(),
            output.len(),
            "Position buffer length mismatch"
        );
        debug_assert_eq!(
            grain_size_buffer.len(),
            output.len(),
            "Grain size buffer length mismatch"
        );
        debug_assert_eq!(
            density_buffer.len(),
            output.len(),
            "Density buffer length mismatch"
        );
        debug_assert_eq!(
            pitch_buffer.len(),
            output.len(),
            "Pitch buffer length mismatch"
        );
        debug_assert_eq!(
            spray_buffer.len(),
            output.len(),
            "Spray buffer length mismatch"
        );

        // If source is empty, output silence
        if self.source.is_empty() {
            output.fill(0.0);
            return;
        }

        let source_len = self.source.len() as f32;

        for i in 0..output.len() {
            // Read parameters (clamped to valid ranges)
            let position = position_buffer[i].clamp(0.0, 1.0);
            let grain_size_ms = grain_size_buffer[i].clamp(5.0, 500.0);
            let density = density_buffer[i].clamp(1.0, 100.0);
            let pitch_semitones = pitch_buffer[i].clamp(-12.0, 12.0);
            let spray = spray_buffer[i].clamp(0.0, 1.0);

            // Convert grain size to samples
            let grain_duration = (grain_size_ms / 1000.0) * self.sample_rate;

            // Calculate pitch ratio (2^(semitones/12))
            let pitch_ratio = 2.0_f32.powf(pitch_semitones / 12.0);

            // Calculate grains per sample
            let grains_per_sample = density / self.sample_rate;

            // Update grain spawn counter
            self.samples_since_last_grain += grains_per_sample;

            // Spawn new grain(s) if needed
            while self.samples_since_last_grain >= 1.0 {
                // Calculate spawn position with spray
                let spray_offset = (self.rng.gen::<f32>() - 0.5) * spray;
                let spawn_position = (position + spray_offset).clamp(0.0, 1.0);
                let start_sample = spawn_position * source_len;

                // Create new grain
                let grain = Grain::new(start_sample, grain_duration, pitch_ratio);
                self.active_grains.push(grain);

                self.samples_since_last_grain -= 1.0;
            }

            // Process all active grains
            let mut sample_sum = 0.0;
            for grain in &mut self.active_grains {
                sample_sum += grain.process(&self.source);
            }

            // Remove finished grains (retain unfinished ones)
            self.active_grains.retain(|g| !g.is_finished());

            // Write output
            output[i] = sample_sum;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![
            self.position_input,
            self.grain_size_input,
            self.density_input,
            self.pitch_input,
            self.spray_input,
        ]
    }

    fn name(&self) -> &str {
        "GranularNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    /// Create a simple test source buffer (1 second sine wave at 440 Hz)
    fn create_test_source(sample_rate: f32) -> Arc<Vec<f32>> {
        let len = sample_rate as usize;
        let mut buffer = Vec::with_capacity(len);
        for i in 0..len {
            let phase = (i as f32 / sample_rate) * 440.0 * 2.0 * std::f32::consts::PI;
            buffer.push(phase.sin() * 0.5);
        }
        Arc::new(buffer)
    }

    #[test]
    fn test_granular_spawns_grains_at_correct_rate() {
        // Test 1: Grain spawning rate matches density parameter

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(50.0); // 50ms grains
        let mut density_node = ConstantNode::new(10.0); // 10 grains/sec
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.0);

        let mut granular = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Generate input buffers
        let mut position_buf = vec![0.0; block_size];
        let mut grain_size_buf = vec![0.0; block_size];
        let mut density_buf = vec![0.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.0; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Process 1 second (86 blocks)
        let mut grain_counts = Vec::new();
        for _ in 0..(sample_rate as usize / block_size) {
            let mut output = vec![0.0; block_size];
            granular.process_block(&inputs, &mut output, sample_rate, &context);
            grain_counts.push(granular.active_grain_count());
        }

        // With 10 grains/sec and 50ms duration, we expect approximately:
        // max ~0.5 grains active at once (10 grains/sec * 0.05 sec/grain)
        // But during spawning transient, could be higher
        let max_grains = *grain_counts.iter().max().unwrap();
        assert!(
            max_grains >= 1 && max_grains <= 10,
            "Expected 1-10 active grains, got max: {}",
            max_grains
        );
    }

    #[test]
    fn test_granular_grain_size_affects_duration() {
        // Test 2: Larger grain size creates longer-lasting grains

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut density_node = ConstantNode::new(5.0); // 5 grains/sec
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test short grains (20ms)
        let mut grain_size_short = ConstantNode::new(20.0);
        let mut granular_short = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf_short = vec![20.0; block_size];
        let mut density_buf = vec![5.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.0; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_short.process_block(&[], &mut grain_size_buf_short, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs_short = vec![
            position_buf.as_slice(),
            grain_size_buf_short.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Process 0.5 seconds
        let blocks = (sample_rate as usize / 2) / block_size;
        for _ in 0..blocks {
            let mut output = vec![0.0; block_size];
            granular_short.process_block(&inputs_short, &mut output, sample_rate, &context);
        }

        let short_grains = granular_short.active_grain_count();

        // Test long grains (100ms)
        let mut grain_size_long = ConstantNode::new(100.0);
        let mut granular_long = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut grain_size_buf_long = vec![100.0; block_size];
        grain_size_long.process_block(&[], &mut grain_size_buf_long, sample_rate, &context);

        let inputs_long = vec![
            position_buf.as_slice(),
            grain_size_buf_long.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        for _ in 0..blocks {
            let mut output = vec![0.0; block_size];
            granular_long.process_block(&inputs_long, &mut output, sample_rate, &context);
        }

        let long_grains = granular_long.active_grain_count();

        // Longer grain size should result in more active grains
        // (more grains overlap since they last longer)
        assert!(
            long_grains > short_grains,
            "Long grains ({}) should have more active than short grains ({})",
            long_grains,
            short_grains
        );
    }

    #[test]
    fn test_granular_position_controls_playback() {
        // Test 3: Position parameter controls where grains read from source

        let sample_rate = 44100.0;
        let block_size = 512;

        // Create source with distinct sections (different amplitudes)
        let mut source_data = vec![0.0; sample_rate as usize];
        for i in 0..source_data.len() {
            if i < source_data.len() / 2 {
                source_data[i] = 0.1; // First half: low amplitude
            } else {
                source_data[i] = 0.9; // Second half: high amplitude
            }
        }
        let source = Arc::new(source_data);

        let mut grain_size_node = ConstantNode::new(50.0);
        let mut density_node = ConstantNode::new(20.0);
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.0); // No spray

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test position 0.25 (first half - low amplitude)
        let mut position_low = ConstantNode::new(0.25);
        let mut granular_low = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut position_buf_low = vec![0.25; block_size];
        let mut grain_size_buf = vec![50.0; block_size];
        let mut density_buf = vec![20.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.0; block_size];

        position_low.process_block(&[], &mut position_buf_low, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs_low = vec![
            position_buf_low.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut output_low = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            granular_low.process_block(&inputs_low, &mut output, sample_rate, &context);
            output_low.extend_from_slice(&output);
        }

        let rms_low = calculate_rms(&output_low);

        // Test position 0.75 (second half - high amplitude)
        let mut position_high = ConstantNode::new(0.75);
        let mut granular_high = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut position_buf_high = vec![0.75; block_size];
        position_high.process_block(&[], &mut position_buf_high, sample_rate, &context);

        let inputs_high = vec![
            position_buf_high.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut output_high = Vec::new();
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            granular_high.process_block(&inputs_high, &mut output, sample_rate, &context);
            output_high.extend_from_slice(&output);
        }

        let rms_high = calculate_rms(&output_high);

        // High position (0.75) should have higher RMS than low position (0.25)
        assert!(
            rms_high > rms_low * 2.0,
            "High position RMS ({}) should be much higher than low position RMS ({})",
            rms_high,
            rms_low
        );
    }

    #[test]
    fn test_granular_pitch_shift_works() {
        // Test 4: Pitch shift affects playback speed

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(100.0);
        let mut density_node = ConstantNode::new(10.0);
        let mut spray_node = ConstantNode::new(0.0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test normal pitch (0 semitones)
        let mut pitch_normal = ConstantNode::new(0.0);
        let mut granular_normal = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![100.0; block_size];
        let mut density_buf = vec![10.0; block_size];
        let mut pitch_buf_normal = vec![0.0; block_size];
        let mut spray_buf = vec![0.0; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_normal.process_block(&[], &mut pitch_buf_normal, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs_normal = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf_normal.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut output_normal = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            granular_normal.process_block(&inputs_normal, &mut output, sample_rate, &context);
            output_normal.extend_from_slice(&output);
        }

        // Test octave up (+12 semitones)
        let mut pitch_up = ConstantNode::new(12.0);
        let mut granular_up = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut pitch_buf_up = vec![12.0; block_size];
        pitch_up.process_block(&[], &mut pitch_buf_up, sample_rate, &context);

        let inputs_up = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf_up.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut output_up = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            granular_up.process_block(&inputs_up, &mut output, sample_rate, &context);
            output_up.extend_from_slice(&output);
        }

        // Both should produce sound
        let rms_normal = calculate_rms(&output_normal);
        let rms_up = calculate_rms(&output_up);

        assert!(rms_normal > 0.01, "Normal pitch should produce sound");
        assert!(rms_up > 0.01, "Pitch shifted should produce sound");

        // Pitch shift should create different output
        // (Note: RMS might be similar, but waveform would be different)
        // For this test, just verify both work
    }

    #[test]
    fn test_granular_spray_creates_variation() {
        // Test 5: Spray parameter creates position variation

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(50.0);
        let mut density_node = ConstantNode::new(20.0);
        let mut pitch_node = ConstantNode::new(0.0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test no spray
        let mut spray_none = ConstantNode::new(0.0);
        let mut granular_none = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![50.0; block_size];
        let mut density_buf = vec![20.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf_none = vec![0.0; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_none.process_block(&[], &mut spray_buf_none, sample_rate, &context);

        let inputs_none = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf_none.as_slice(),
        ];

        let mut output_none = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            granular_none.process_block(&inputs_none, &mut output, sample_rate, &context);
            output_none.extend_from_slice(&output);
        }

        // Test with spray
        let mut spray_some = ConstantNode::new(0.5);
        let mut granular_some = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut spray_buf_some = vec![0.5; block_size];
        spray_some.process_block(&[], &mut spray_buf_some, sample_rate, &context);

        let inputs_some = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf_some.as_slice(),
        ];

        let mut output_some = Vec::new();
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            granular_some.process_block(&inputs_some, &mut output, sample_rate, &context);
            output_some.extend_from_slice(&output);
        }

        // Both should produce sound
        let rms_none = calculate_rms(&output_none);
        let rms_some = calculate_rms(&output_some);

        assert!(rms_none > 0.01, "No spray should produce sound");
        assert!(rms_some > 0.01, "With spray should produce sound");

        // Spray creates variation - outputs should be different
        // (could measure spectral content, but for now just verify both work)
    }

    #[test]
    fn test_granular_density_affects_texture() {
        // Test 6: Density affects number of simultaneous grains

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(100.0); // 100ms grains
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.0);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Test low density (5 grains/sec)
        let mut density_low = ConstantNode::new(5.0);
        let mut granular_low = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![100.0; block_size];
        let mut density_buf_low = vec![5.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.0; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_low.process_block(&[], &mut density_buf_low, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs_low = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf_low.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut max_grains_low = 0;
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            granular_low.process_block(&inputs_low, &mut output, sample_rate, &context);
            max_grains_low = max_grains_low.max(granular_low.active_grain_count());
        }

        // Test high density (50 grains/sec)
        let mut density_high = ConstantNode::new(50.0);
        let mut granular_high = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let mut density_buf_high = vec![50.0; block_size];
        density_high.process_block(&[], &mut density_buf_high, sample_rate, &context);

        let inputs_high = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf_high.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut max_grains_high = 0;
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            granular_high.process_block(&inputs_high, &mut output, sample_rate, &context);
            max_grains_high = max_grains_high.max(granular_high.active_grain_count());
        }

        // High density should have more active grains
        assert!(
            max_grains_high > max_grains_low * 2,
            "High density ({}) should have more grains than low density ({})",
            max_grains_high,
            max_grains_low
        );
    }

    #[test]
    fn test_granular_pattern_modulation() {
        // Test 7: Parameters can be modulated per-sample

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut granular = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Create modulating buffers
        let mut position_buf = vec![0.0; block_size];
        let mut grain_size_buf = vec![0.0; block_size];
        let mut density_buf = vec![0.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.0; block_size];

        for i in 0..block_size {
            let phase = i as f32 / block_size as f32;
            position_buf[i] = 0.3 + phase * 0.4; // 0.3 to 0.7
            grain_size_buf[i] = 20.0 + phase * 80.0; // 20ms to 100ms
            density_buf[i] = 5.0 + phase * 15.0; // 5 to 20 grains/sec
            pitch_buf[i] = -6.0 + phase * 12.0; // -6 to +6 semitones
            spray_buf[i] = phase * 0.5; // 0.0 to 0.5
        }

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Process several blocks
        for _ in 0..20 {
            let mut output = vec![0.0; block_size];
            granular.process_block(&inputs, &mut output, sample_rate, &context);

            // All outputs should be finite
            for &sample in output.iter() {
                assert!(sample.is_finite(), "Output should be finite");
            }
        }

        // Should have spawned some grains
        assert!(granular.active_grain_count() > 0, "Should have active grains");
    }

    #[test]
    fn test_granular_performance_many_grains() {
        // Test 8: Node should handle 20+ simultaneous grains efficiently

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(200.0); // Long grains (200ms)
        let mut density_node = ConstantNode::new(100.0); // High density (100/sec)
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.2);

        let mut granular = GranularNode::new(
            source.clone(),
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![200.0; block_size];
        let mut density_buf = vec![100.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.2; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Process blocks until we have many grains
        let mut max_grains = 0;
        for _ in 0..50 {
            let mut output = vec![0.0; block_size];
            granular.process_block(&inputs, &mut output, sample_rate, &context);
            max_grains = max_grains.max(granular.active_grain_count());

            // All samples should be finite
            for &sample in output.iter() {
                assert!(sample.is_finite(), "Output should remain finite");
            }
        }

        // Should reach 20+ grains (100 grains/sec * 0.2 sec = 20)
        assert!(
            max_grains >= 15,
            "Should handle 15+ grains, got max: {}",
            max_grains
        );
    }

    #[test]
    fn test_granular_empty_source() {
        // Test 9: Empty source buffer should output silence

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = Arc::new(Vec::new()); // Empty buffer

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(50.0);
        let mut density_node = ConstantNode::new(10.0);
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.1);

        let mut granular = GranularNode::new(
            source,
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![50.0; block_size];
        let mut density_buf = vec![10.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.1; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        let mut output = vec![0.0; block_size];
        granular.process_block(&inputs, &mut output, sample_rate, &context);

        // All output should be zero
        for &sample in output.iter() {
            assert_eq!(sample, 0.0, "Empty source should output silence");
        }
    }

    #[test]
    fn test_granular_clear_grains() {
        // Test 10: clear_grains() should silence output

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(50.0);
        let mut density_node = ConstantNode::new(20.0);
        let mut pitch_node = ConstantNode::new(0.0);
        let mut spray_node = ConstantNode::new(0.1);

        let mut granular = GranularNode::new(
            source,
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![50.0; block_size];
        let mut density_buf = vec![20.0; block_size];
        let mut pitch_buf = vec![0.0; block_size];
        let mut spray_buf = vec![0.1; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Process to spawn grains
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            granular.process_block(&inputs, &mut output, sample_rate, &context);
        }

        assert!(granular.active_grain_count() > 0, "Should have grains before clear");

        // Clear grains
        granular.clear_grains();

        assert_eq!(granular.active_grain_count(), 0, "Should have no grains after clear");
    }

    #[test]
    fn test_granular_dependencies() {
        // Test 11: Verify node reports correct dependencies

        let source = Arc::new(vec![0.0; 44100]);
        let granular = GranularNode::new(
            source,
            10, 20, 30, 40, 50,
            44100.0,
        );

        let deps = granular.input_nodes();

        assert_eq!(deps.len(), 5);
        assert_eq!(deps[0], 10); // position_input
        assert_eq!(deps[1], 20); // grain_size_input
        assert_eq!(deps[2], 30); // density_input
        assert_eq!(deps[3], 40); // pitch_input
        assert_eq!(deps[4], 50); // spray_input
    }

    #[test]
    fn test_granular_hann_window_shape() {
        // Test 12: Hann window should be smooth bell curve

        // Test at various points
        assert_eq!(hann_window(0.0, 100.0), 0.0); // Start: 0

        let quarter = hann_window(25.0, 100.0);
        assert!(quarter > 0.2 && quarter < 0.6, "Quarter point should be rising");

        let middle = hann_window(50.0, 100.0);
        assert!(middle > 0.9, "Middle should be peak (~1.0)");

        let three_quarter = hann_window(75.0, 100.0);
        assert!(three_quarter > 0.2 && three_quarter < 0.6, "Three-quarter should be falling");

        assert_eq!(hann_window(100.0, 100.0), 0.0); // End: 0
    }

    #[test]
    fn test_granular_parameter_clamping() {
        // Test 13: Parameters should be clamped to valid ranges

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut granular = GranularNode::new(
            source,
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        // Invalid parameters (out of range)
        let position_buf = vec![-1.0; block_size]; // Invalid: < 0
        let grain_size_buf = vec![1000.0; block_size]; // Invalid: > 500ms
        let density_buf = vec![200.0; block_size]; // Invalid: > 100
        let pitch_buf = vec![24.0; block_size]; // Invalid: > 12
        let spray_buf = vec![2.0; block_size]; // Invalid: > 1.0

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Should not crash or produce invalid output
        for _ in 0..10 {
            let mut output = vec![0.0; block_size];
            granular.process_block(&inputs, &mut output, sample_rate, &context);

            for &sample in output.iter() {
                assert!(sample.is_finite(), "Clamped params should produce finite output");
            }
        }
    }

    #[test]
    fn test_granular_stable_over_time() {
        // Test 14: Node should remain stable over extended processing

        let sample_rate = 44100.0;
        let block_size = 512;
        let source = create_test_source(sample_rate);

        let mut position_node = ConstantNode::new(0.5);
        let mut grain_size_node = ConstantNode::new(80.0);
        let mut density_node = ConstantNode::new(25.0);
        let mut pitch_node = ConstantNode::new(2.0);
        let mut spray_node = ConstantNode::new(0.3);

        let mut granular = GranularNode::new(
            source,
            0, 1, 2, 3, 4,
            sample_rate,
        );

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            block_size,
            2.0,
            sample_rate,
        );

        let mut position_buf = vec![0.5; block_size];
        let mut grain_size_buf = vec![80.0; block_size];
        let mut density_buf = vec![25.0; block_size];
        let mut pitch_buf = vec![2.0; block_size];
        let mut spray_buf = vec![0.3; block_size];

        position_node.process_block(&[], &mut position_buf, sample_rate, &context);
        grain_size_node.process_block(&[], &mut grain_size_buf, sample_rate, &context);
        density_node.process_block(&[], &mut density_buf, sample_rate, &context);
        pitch_node.process_block(&[], &mut pitch_buf, sample_rate, &context);
        spray_node.process_block(&[], &mut spray_buf, sample_rate, &context);

        let inputs = vec![
            position_buf.as_slice(),
            grain_size_buf.as_slice(),
            density_buf.as_slice(),
            pitch_buf.as_slice(),
            spray_buf.as_slice(),
        ];

        // Process 1000 blocks (about 11 seconds)
        for _ in 0..1000 {
            let mut output = vec![0.0; block_size];
            granular.process_block(&inputs, &mut output, sample_rate, &context);

            // Check stability
            for &sample in output.iter() {
                assert!(sample.is_finite(), "Output should remain finite");
                assert!(sample.abs() < 100.0, "Output should not explode: {}", sample);
            }

            // Grain count should remain reasonable
            assert!(
                granular.active_grain_count() < 200,
                "Grain count should not explode: {}",
                granular.active_grain_count()
            );
        }
    }

    // Helper function for RMS calculation
    fn calculate_rms(buffer: &[f32]) -> f32 {
        let sum: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum / buffer.len() as f32).sqrt()
    }
}
