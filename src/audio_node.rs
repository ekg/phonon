/// Block-based audio processing - core abstraction for DAW-style architecture
///
/// This module defines the AudioNode trait, which replaces sample-by-sample
/// SignalNode evaluation with efficient block-based buffer passing.

use crate::pattern::Fraction;
use std::collections::HashMap;

pub type NodeId = usize;

/// Context passed to all nodes during block processing
///
/// Contains timing and state information needed for audio processing.
#[derive(Debug, Clone)]
pub struct ProcessContext {
    /// Current position in the musical cycle (e.g., 0.0 to 1.0 for one cycle)
    pub cycle_position: Fraction,

    /// Sample offset within the current cycle
    pub sample_offset: usize,

    /// Number of samples to process in this block (usually 512)
    pub block_size: usize,

    /// Tempo in cycles per second
    pub tempo: f64,

    /// Sample rate (usually 44100.0 Hz)
    pub sample_rate: f32,

    /// Additional control parameters that may be needed
    pub controls: HashMap<String, f32>,
}

impl ProcessContext {
    /// Create a new process context
    pub fn new(
        cycle_position: Fraction,
        sample_offset: usize,
        block_size: usize,
        tempo: f64,
        sample_rate: f32,
    ) -> Self {
        Self {
            cycle_position,
            sample_offset,
            block_size,
            tempo,
            sample_rate,
            controls: HashMap::new(),
        }
    }

    /// Calculate the cycle position at a specific sample offset within the block
    pub fn cycle_position_at_offset(&self, offset: usize) -> Fraction {
        let samples_per_cycle = self.sample_rate as f64 / self.tempo;
        let cycle_delta = Fraction::from_float(offset as f64 / samples_per_cycle);
        self.cycle_position + cycle_delta
    }
}

/// Core trait for block-based audio processing
///
/// Every audio-producing entity implements this trait. Nodes process entire
/// 512-sample buffers at once (instead of sample-by-sample), enabling:
/// - Graph traversed ONCE per block (not 512 times)
/// - Zero-copy buffer passing (via Arc)
/// - Parallel execution of independent nodes
/// - Better cache locality and SIMD optimization
pub trait AudioNode: Send {
    /// Process an entire block of audio
    ///
    /// This is called ONCE per block (not per sample). The node should:
    /// 1. Read from input buffers (already computed by dependencies)
    /// 2. Process the entire block (512 samples at once)
    /// 3. Write results to output buffer
    ///
    /// # Arguments
    /// * `inputs` - Input buffers from dependent nodes (zero-copy via &[f32])
    /// * `output` - Output buffer to write to (length = block_size)
    /// * `sample_rate` - Current sample rate (44100.0 Hz)
    /// * `context` - Processing context (cycle position, tempo, etc.)
    ///
    /// # Performance Notes
    /// - Prefer vectorized operations over sample-by-sample loops
    /// - Use SIMD-friendly code patterns where possible
    /// - Avoid allocations inside process_block (use preallocated state)
    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        context: &ProcessContext,
    );

    /// Return list of input node IDs this node depends on
    ///
    /// Used for:
    /// - Topological sorting (execution order)
    /// - Identifying parallelizable nodes (no shared dependencies)
    /// - Buffer routing (which inputs to pass to process_block)
    ///
    /// # Returns
    /// Vec of NodeIds in the order they should appear in the `inputs` array
    /// passed to process_block. Empty vec for source nodes (no dependencies).
    fn input_nodes(&self) -> Vec<NodeId>;

    /// Called once per block before processing (optional)
    ///
    /// Use this for:
    /// - Pattern evaluation (query events for the block's time range)
    /// - Voice triggering (sample-accurate event scheduling)
    /// - Parameter updates that happen once per block
    ///
    /// Default implementation does nothing.
    fn prepare_block(&mut self, _context: &ProcessContext) {}

    /// Get a human-readable name for this node (for debugging)
    fn name(&self) -> &str {
        "AudioNode"
    }

    /// Returns true if this node provides delay (can break feedback cycles)
    ///
    /// Nodes that maintain internal delay buffers (DelayNode, CombFilterNode,
    /// FlangerNode, ReverbNode, etc.) should return true. This allows them
    /// to safely participate in feedback loops.
    ///
    /// The delay provides the "previous block" values needed for feedback,
    /// preventing instant (zero-delay) loops that would be undefined.
    ///
    /// Default implementation returns false (most nodes don't provide delay).
    fn provides_delay(&self) -> bool {
        false
    }
}

/// Helper trait for nodes that can be cloned (for multi-threading)
///
/// Nodes that implement this can be deep-cloned to give each thread
/// its own independent state (oscillator phase, filter memory, etc.)
pub trait CloneableAudioNode: AudioNode {
    fn clone_node(&self) -> Box<dyn AudioNode>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_context_cycle_position_at_offset() {
        let ctx = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,  // 2 cycles per second
            44100.0,
        );

        // At offset 0, should be at cycle position 0.0
        let pos_0 = ctx.cycle_position_at_offset(0);
        assert!((pos_0.to_float() - 0.0).abs() < 0.0001);

        // At offset 256 (half block), should advance by fraction of cycle
        let pos_256 = ctx.cycle_position_at_offset(256);
        let expected = 256.0 / (44100.0 / 2.0);  // samples / samples_per_cycle
        assert!((pos_256.to_float() - expected).abs() < 0.0001);
    }
}
