/// Wavetable oscillator node - efficient arbitrary waveform synthesis
///
/// Wavetable synthesis plays back pre-computed waveforms with linear interpolation
/// for smooth, click-free output. Much more efficient than computing waveforms
/// per-sample and enables arbitrary waveforms (not just sine/saw/square).
///
/// Based on:
/// - Classic PPG Wave, Waldorf Wave, and modern digital synthesizers
/// - Standard linear interpolation (cubic is overkill for audio)
/// - Used heavily in modern electronic music production
///
/// # Features
/// - Arc-based table sharing (multiple oscillators can share same wavetable)
/// - Linear interpolation between samples (smooth, no aliasing)
/// - Arbitrary waveforms (not limited to geometric shapes)
/// - Very efficient (table lookup vs trigonometric math)
///
/// # Common Table Sizes
/// - 64 samples: Lo-fi/retro sound
/// - 256 samples: Good for most uses (default)
/// - 1024 samples: High quality
/// - 4096 samples: Ultra-high quality (diminishing returns)

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use std::f32::consts::PI;
use std::sync::Arc;

/// Wavetable oscillator with linear interpolation
///
/// # Example
/// ```ignore
/// // Create a 256-sample sine wavetable oscillator at 440 Hz
/// let freq_const = ConstantNode::new(440.0);  // NodeId 0
/// let osc = WavetableNode::sine(0, 256);       // NodeId 1
/// ```
pub struct WavetableNode {
    freq_input: NodeId,              // NodeId providing frequency values
    wavetable: Arc<Vec<f32>>,        // Shared waveform data
    phase: f32,                      // Current phase (0.0 to 1.0)
}

impl WavetableNode {
    /// Wavetable - Efficient arbitrary waveform synthesis with linear interpolation
    ///
    /// Plays back pre-computed waveforms much faster than computing per-sample.
    /// Enables arbitrary waveforms and Arc-based table sharing.
    ///
    /// # Parameters
    /// - `freq_input`: Frequency in Hz (can be modulated)
    /// - `wavetable`: Pre-computed waveform data (Arc-wrapped for sharing)
    ///
    /// # Example
    /// ```phonon
    /// ~freq: lfo 0.5 110 220
    /// out: wavetable ~freq sine 256
    /// ```
    pub fn new(freq_input: NodeId, wavetable: Arc<Vec<f32>>) -> Self {
        Self {
            freq_input,
            wavetable,
            phase: 0.0,
        }
    }

    /// Create a sine wave wavetable oscillator
    ///
    /// # Arguments
    /// * `freq_input` - NodeId providing frequency
    /// * `table_size` - Number of samples in table (256 recommended)
    pub fn sine(freq_input: NodeId, table_size: usize) -> Self {
        Self::new(freq_input, generate_sine_table(table_size))
    }

    /// Create a sawtooth wave wavetable oscillator
    ///
    /// # Arguments
    /// * `freq_input` - NodeId providing frequency
    /// * `table_size` - Number of samples in table (256 recommended)
    pub fn saw(freq_input: NodeId, table_size: usize) -> Self {
        Self::new(freq_input, generate_saw_table(table_size))
    }

    /// Create a square wave wavetable oscillator
    ///
    /// # Arguments
    /// * `freq_input` - NodeId providing frequency
    /// * `table_size` - Number of samples in table (256 recommended)
    pub fn square(freq_input: NodeId, table_size: usize) -> Self {
        Self::new(freq_input, generate_square_table(table_size))
    }

    /// Get current phase (0.0 to 1.0)
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Reset phase to 0.0
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    /// Get reference to wavetable data
    pub fn wavetable(&self) -> &Arc<Vec<f32>> {
        &self.wavetable
    }
}

/// Generate a sine wave lookup table
///
/// # Arguments
/// * `size` - Number of samples in table
///
/// # Returns
/// Arc-wrapped vector of samples ranging from -1.0 to 1.0
fn generate_sine_table(size: usize) -> Arc<Vec<f32>> {
    let mut table = Vec::with_capacity(size);
    for i in 0..size {
        let phase = (i as f32) / (size as f32);
        table.push((phase * 2.0 * PI).sin());
    }
    Arc::new(table)
}

/// Generate a sawtooth wave lookup table
///
/// # Arguments
/// * `size` - Number of samples in table
///
/// # Returns
/// Arc-wrapped vector of samples ranging from -1.0 to 1.0
fn generate_saw_table(size: usize) -> Arc<Vec<f32>> {
    let mut table = Vec::with_capacity(size);
    for i in 0..size {
        let phase = (i as f32) / (size as f32);
        table.push(2.0 * phase - 1.0);
    }
    Arc::new(table)
}

/// Generate a square wave lookup table
///
/// # Arguments
/// * `size` - Number of samples in table
///
/// # Returns
/// Arc-wrapped vector of samples (1.0 or -1.0)
fn generate_square_table(size: usize) -> Arc<Vec<f32>> {
    let mut table = Vec::with_capacity(size);
    for i in 0..size {
        let phase = (i as f32) / (size as f32);
        table.push(if phase < 0.5 { 1.0 } else { -1.0 });
    }
    Arc::new(table)
}

impl AudioNode for WavetableNode {
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        _context: &ProcessContext,
    ) {
        debug_assert!(
            !inputs.is_empty(),
            "WavetableNode requires frequency input"
        );

        let freq_buffer = inputs[0];

        debug_assert_eq!(
            freq_buffer.len(),
            output.len(),
            "Frequency buffer length mismatch"
        );

        let table_len = self.wavetable.len();

        // Edge case: empty table
        if table_len == 0 {
            output.fill(0.0);
            return;
        }

        // For interpolation, we need at least 2 samples
        if table_len == 1 {
            let value = self.wavetable[0];
            output.fill(value);
            return;
        }

        for i in 0..output.len() {
            let freq = freq_buffer[i];

            // Convert phase (0.0 to 1.0) to table position
            // We map to (table_len - 1) so we can safely interpolate with next sample
            let table_pos = self.phase * ((table_len - 1) as f32);
            let index = table_pos as usize;
            let frac = table_pos - (index as f32);

            // Linear interpolation between adjacent samples
            let sample1 = self.wavetable[index];
            let sample2 = self.wavetable[index + 1];
            output[i] = sample1 + frac * (sample2 - sample1);

            // Advance phase
            self.phase += freq / sample_rate;

            // Wrap phase to [0.0, 1.0)
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            while self.phase < 0.0 {
                self.phase += 1.0;
            }
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![self.freq_input]
    }

    fn name(&self) -> &str {
        "WavetableNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::constant::ConstantNode;
    use crate::pattern::Fraction;

    #[test]
    fn test_wavetable_sine_matches_sin_function() {
        // Sine wavetable should closely match std::f32::sin()
        let table_size = 1024;  // High resolution for accuracy
        let table = generate_sine_table(table_size);

        // Check multiple points around the table
        for i in 0..100 {
            let phase = (i as f32) / 100.0;
            let table_pos = phase * ((table_size - 1) as f32);
            let table_index = table_pos as usize;
            let frac = table_pos - (table_index as f32);

            // Use linear interpolation like the actual node does
            let sample1 = table[table_index];
            let sample2 = if table_index + 1 < table.len() {
                table[table_index + 1]
            } else {
                table[table_index]
            };
            let table_value = sample1 + frac * (sample2 - sample1);
            let expected = (phase * 2.0 * PI).sin();

            // Should be very close (within 0.01 for 1024-sample table with interpolation)
            assert!(
                (table_value - expected).abs() < 0.01,
                "Sine table mismatch at phase {}: got {}, expected {}",
                phase, table_value, expected
            );
        }
    }

    #[test]
    fn test_wavetable_saw_shape() {
        let table = generate_saw_table(256);

        // Saw should be linear ramp from -1.0 to ~1.0
        assert!((table[0] - (-1.0)).abs() < 0.01, "Saw should start at -1.0");
        assert!((table[127] - 0.0).abs() < 0.1, "Saw middle should be near 0.0");
        assert!(table[255] > 0.9, "Saw end should be near 1.0");

        // Should be monotonically increasing
        for i in 1..table.len() {
            assert!(
                table[i] >= table[i - 1],
                "Saw should be monotonically increasing"
            );
        }
    }

    #[test]
    fn test_wavetable_square_shape() {
        let table = generate_square_table(256);

        // First half should be 1.0, second half should be -1.0
        for i in 0..128 {
            assert_eq!(table[i], 1.0, "First half of square should be 1.0");
        }
        for i in 128..256 {
            assert_eq!(table[i], -1.0, "Second half of square should be -1.0");
        }
    }

    #[test]
    fn test_wavetable_interpolation_smoothness() {
        // Linear interpolation should produce smooth transitions
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = WavetableNode::sine(0, 256);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Generate frequency buffer
        let mut freq_buf = vec![0.0; 512];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        // Generate wavetable output
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Check for sudden jumps (shouldn't exist with interpolation)
        for i in 1..output.len() {
            let diff = (output[i] - output[i - 1]).abs();
            assert!(
                diff < 0.5,  // Sine at 440Hz shouldn't jump more than 0.5 between samples
                "Large discontinuity detected at sample {}: {} to {}",
                i, output[i - 1], output[i]
            );
        }
    }

    #[test]
    fn test_wavetable_frequency_modulation() {
        // Test with varying frequency
        let table_size = 256;
        let mut osc = WavetableNode::sine(0, table_size);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Create frequency ramp from 220 Hz to 880 Hz
        let mut freq_buf = vec![0.0; 512];
        for i in 0..512 {
            freq_buf[i] = 220.0 + (660.0 * (i as f32) / 512.0);
        }

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Wavetable with FM should produce signal");

        // All samples should be in valid range
        for sample in &output {
            assert!(
                sample.abs() <= 1.1,
                "Sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_wavetable_phase_wrapping() {
        let mut osc = WavetableNode::sine(0, 256);

        // Set phase close to 1.0
        osc.phase = 0.99;

        // Process one sample at high frequency
        let freq_buf = vec![4410.0];  // 10% of sample rate
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Phase should wrap back to [0.0, 1.0)
        assert!(
            osc.phase() >= 0.0 && osc.phase() < 1.0,
            "Phase didn't wrap: {}",
            osc.phase()
        );
    }

    #[test]
    fn test_wavetable_custom_data() {
        // Test with custom waveform data
        let custom_data = vec![0.5, 0.8, 1.0, 0.8, 0.5, 0.0, -0.5, -0.8, -1.0, -0.8];
        let custom_table = Arc::new(custom_data);

        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = WavetableNode::new(0, custom_table.clone());

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            256,
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 256];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 256];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should produce valid output
        let has_signal = output.iter().any(|&x| x.abs() > 0.1);
        assert!(has_signal, "Custom wavetable should produce signal");

        // Peak should not exceed custom data range
        let max = output.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        assert!(max <= 1.1, "Output should stay within custom data range");
    }

    #[test]
    fn test_wavetable_different_sizes() {
        let sizes = vec![64, 256, 1024, 4096];

        for size in sizes {
            let mut const_freq = ConstantNode::new(440.0);
            let mut osc = WavetableNode::sine(0, size);

            let context = ProcessContext::new(
                Fraction::from_float(0.0),
                0,
                512,
                2.0,
                44100.0,
            );

            let mut freq_buf = vec![0.0; 512];
            const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

            let inputs = vec![freq_buf.as_slice()];
            let mut output = vec![0.0; 512];
            osc.process_block(&inputs, &mut output, 44100.0, &context);

            // All sizes should produce valid sine output
            let has_signal = output.iter().any(|&x| x.abs() > 0.1);
            assert!(
                has_signal,
                "Table size {} should produce signal",
                size
            );

            // Check DC offset (sine should average to ~0)
            let sum: f32 = output.iter().sum();
            let avg = sum / output.len() as f32;
            assert!(
                avg.abs() < 0.2,
                "Table size {} has DC offset: {}",
                size, avg
            );
        }
    }

    #[test]
    fn test_wavetable_interpolation_at_fractional_positions() {
        // Test that interpolation works correctly at specific fractional positions
        let table = Arc::new(vec![0.0, 1.0, 0.0, -1.0]);  // Simple 4-sample table
        let mut osc = WavetableNode::new(0, table);

        // Manually set phase to 0.5 (halfway between samples 1 and 2)
        osc.phase = 0.5 / 3.0;  // Position 1.5 in table

        let freq_buf = vec![0.0];  // Don't advance phase
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 1];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            1,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should interpolate between table[1] = 1.0 and table[2] = 0.0
        // Position 1.5 means 50% between them: 1.0 + 0.5 * (0.0 - 1.0) = 0.5
        assert!(
            (output[0] - 0.5).abs() < 0.01,
            "Interpolation incorrect: got {}, expected 0.5",
            output[0]
        );
    }

    #[test]
    fn test_wavetable_dependencies() {
        let osc = WavetableNode::sine(42, 256);
        let deps = osc.input_nodes();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], 42);
    }

    #[test]
    fn test_wavetable_arc_sharing() {
        // Multiple oscillators can share the same wavetable data
        let shared_table = generate_sine_table(256);

        let osc1 = WavetableNode::new(0, shared_table.clone());
        let osc2 = WavetableNode::new(0, shared_table.clone());

        // Both should reference the same Arc
        assert_eq!(
            Arc::strong_count(osc1.wavetable()),
            3,  // shared_table + osc1 + osc2
            "Arc should be shared between instances"
        );

        // Both should have independent phase
        assert_eq!(osc1.phase(), 0.0);
        assert_eq!(osc2.phase(), 0.0);
    }

    #[test]
    fn test_wavetable_reset_phase() {
        let mut osc = WavetableNode::sine(0, 256);

        // Advance phase
        osc.phase = 0.7;
        assert_eq!(osc.phase(), 0.7);

        // Reset
        osc.reset_phase();
        assert_eq!(osc.phase(), 0.0);
    }

    #[test]
    fn test_wavetable_empty_table() {
        // Edge case: empty table should produce silence
        let empty_table = Arc::new(vec![]);
        let mut osc = WavetableNode::new(0, empty_table);

        let freq_buf = vec![440.0; 512];
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![1.0; 512];  // Pre-fill with non-zero

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Should be all zeros
        for sample in &output {
            assert_eq!(*sample, 0.0, "Empty table should produce silence");
        }
    }

    #[test]
    fn test_wavetable_single_sample_table() {
        // Edge case: single-sample table should produce constant output
        let single_sample = Arc::new(vec![0.42]);
        let mut osc = WavetableNode::new(0, single_sample);

        let freq_buf = vec![440.0; 512];
        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 512];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // All samples should be 0.42
        for sample in &output {
            assert_eq!(*sample, 0.42, "Single-sample table should produce constant");
        }
    }

    #[test]
    fn test_wavetable_dc_offset() {
        // Test that sine wavetable has zero DC offset over multiple cycles
        let mut const_freq = ConstantNode::new(440.0);
        let mut osc = WavetableNode::sine(0, 256);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4410,  // 0.1 seconds = ~44 cycles at 440 Hz
            2.0,
            44100.0,
        );

        let mut freq_buf = vec![0.0; 4410];
        const_freq.process_block(&[], &mut freq_buf, 44100.0, &context);

        let inputs = vec![freq_buf.as_slice()];
        let mut output = vec![0.0; 4410];
        osc.process_block(&inputs, &mut output, 44100.0, &context);

        // Calculate average (should be near 0 for sine wave)
        let sum: f32 = output.iter().sum();
        let avg = sum / output.len() as f32;

        assert!(
            avg.abs() < 0.05,
            "Sine wavetable DC offset too high: {}",
            avg
        );
    }
}
