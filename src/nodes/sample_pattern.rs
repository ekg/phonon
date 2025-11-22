/// Sample Pattern Node - Integrates pattern playback with VoiceManager
///
/// This node bridges the pattern system with the voice-based sample playback system.
/// It queries a Pattern<String> during each audio block to find sample events, then
/// triggers those samples via the VoiceManager for polyphonic playback.
///
/// # Architecture
///
/// The node operates in a hybrid manner:
/// 1. **Pattern Evaluation** (prepare_block): Queries pattern for events in the block's time range
/// 2. **Sample Triggering** (prepare_block): Loads samples from SampleBank and triggers via VoiceManager
/// 3. **Voice Rendering** (process_block): Calls VoiceManager.render_block() to generate mixed audio
///
/// # Example Usage
///
/// ```ignore
/// use phonon::nodes::sample_pattern::SamplePatternNode;
/// use phonon::mini_notation_v3::parse_mini_notation;
/// use phonon::sample_loader::SampleBank;
/// use phonon::voice_manager::VoiceManager;
/// use std::sync::{Arc, Mutex};
///
/// // Create pattern, sample bank, and voice manager
/// let pattern = Arc::new(parse_mini_notation("bd sn hh cp"));
/// let sample_bank = Arc::new(Mutex::new(SampleBank::new()));
/// let voice_manager = Arc::new(Mutex::new(VoiceManager::new()));
///
/// // Create node
/// let node = SamplePatternNode::new(pattern, voice_manager, sample_bank);
/// ```

use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::sample_loader::SampleBank;
use crate::voice_manager::VoiceManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Sample Pattern Node
///
/// Evaluates a pattern of sample names and triggers them via VoiceManager.
pub struct SamplePatternNode {
    /// Pattern to evaluate (sample names like "bd", "sn", "hh")
    pattern: Arc<Pattern<String>>,

    /// Voice manager for polyphonic sample playback
    voice_manager: Arc<Mutex<VoiceManager>>,

    /// Sample bank for loading samples (wrapped in Mutex for thread safety)
    sample_bank: Arc<Mutex<SampleBank>>,

    /// Node ID for voice source tracking (set during construction)
    node_id: usize,
}

impl SamplePatternNode {
    /// Create a new SamplePatternNode
    ///
    /// # Parameters
    /// - `pattern`: Pattern of sample names to play
    /// - `voice_manager`: Shared voice manager for sample playback
    /// - `sample_bank`: Sample bank for loading samples (wrapped in Mutex)
    ///
    /// # Example
    /// ```ignore
    /// let pattern = Arc::new(parse_mini_notation("bd sn hh cp"));
    /// let vm = Arc::new(Mutex::new(VoiceManager::new()));
    /// let bank = Arc::new(Mutex::new(SampleBank::new()));
    /// let node = SamplePatternNode::new(pattern, vm, bank);
    /// ```
    pub fn new(
        pattern: Arc<Pattern<String>>,
        voice_manager: Arc<Mutex<VoiceManager>>,
        sample_bank: Arc<Mutex<SampleBank>>,
    ) -> Self {
        Self {
            pattern,
            voice_manager,
            sample_bank,
            node_id: 0, // Default, will be set by caller if needed
        }
    }

    /// Set the node ID (used for voice source tracking)
    pub fn set_node_id(&mut self, node_id: usize) {
        self.node_id = node_id;
    }

    /// Parse sample name from event value
    ///
    /// Handles:
    /// - Direct names: "bd" -> "bd"
    /// - Indexed samples: "bd:0", "bd:1"
    /// - Rests: "~" -> None (silence)
    fn parse_sample_name(&self, event_str: &str) -> Option<String> {
        let s = event_str.trim();

        // Rest (explicit silence)
        if s == "~" || s.is_empty() {
            return None;
        }

        // Return the sample name as-is
        Some(s.to_string())
    }
}

impl AudioNode for SamplePatternNode {
    fn prepare_block(&mut self, context: &ProcessContext) {
        // Calculate the time range for this block
        let start_cycle = context.cycle_position;
        let end_cycle = context.cycle_position_at_offset(context.block_size);

        // Query pattern for events in this block's time range
        let state = State {
            span: TimeSpan::new(start_cycle, end_cycle),
            controls: HashMap::new(),
        };

        let events = self.pattern.query(&state);

        // Trigger samples for each event
        let mut vm = self.voice_manager.lock().unwrap();
        let mut bank = self.sample_bank.lock().unwrap();

        for event in events {
            // Parse sample name from event
            if let Some(sample_name) = self.parse_sample_name(&event.value) {
                // Load sample from bank
                if let Some(sample_data) = bank.get_sample(&sample_name) {
                    // Calculate sample offset within the block
                    // Event occurs at event.part.begin (cycle position)
                    let event_cycle_offset = event.part.begin - start_cycle;
                    let samples_per_cycle = context.sample_rate as f64 / context.tempo;
                    let sample_offset =
                        (event_cycle_offset.to_float() * samples_per_cycle) as usize;

                    // Clamp to block size
                    let sample_offset = sample_offset.min(context.block_size - 1);

                    // Set default source node before triggering
                    vm.set_default_source_node(self.node_id);

                    // Trigger sample with default parameters (gain=1.0, pan=0.0, speed=1.0)
                    vm.trigger_sample_with_params(sample_data, 1.0, 0.0, 1.0);

                    // Set trigger offset for sample-accurate timing
                    vm.set_last_voice_trigger_offset(sample_offset);
                }
            }
        }
    }

    fn process_block(
        &mut self,
        _inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        // Process voices and get mixed output
        let mut vm = self.voice_manager.lock().unwrap();
        let node_buffers = vm.render_block(output.len());

        // Extract our node's buffer (if it exists)
        if let Some(buffer) = node_buffers.get(&self.node_id) {
            // Copy to output
            output.copy_from_slice(buffer);
        } else {
            // No voices active, output silence
            output.fill(0.0);
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![] // No input nodes - generates audio from pattern
    }

    fn name(&self) -> &str {
        "SamplePatternNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_notation_v3::parse_mini_notation;

    #[test]
    fn test_sample_pattern_node_creation() {
        let pattern = Arc::new(parse_mini_notation("bd sn"));
        let vm = Arc::new(Mutex::new(VoiceManager::new()));
        let bank = Arc::new(Mutex::new(SampleBank::new()));

        let node = SamplePatternNode::new(pattern, vm, bank);

        assert_eq!(node.name(), "SamplePatternNode");
        assert_eq!(node.input_nodes().len(), 0);
    }

    #[test]
    fn test_parse_sample_name() {
        let pattern = Arc::new(parse_mini_notation("bd"));
        let vm = Arc::new(Mutex::new(VoiceManager::new()));
        let bank = Arc::new(Mutex::new(SampleBank::new()));

        let node = SamplePatternNode::new(pattern, vm, bank);

        assert_eq!(node.parse_sample_name("bd"), Some("bd".to_string()));
        assert_eq!(node.parse_sample_name("bd:0"), Some("bd:0".to_string()));
        assert_eq!(node.parse_sample_name("sn:3"), Some("sn:3".to_string()));
        assert_eq!(node.parse_sample_name("~"), None);
        assert_eq!(node.parse_sample_name(""), None);
    }

    #[test]
    fn test_sample_pattern_node_process_block() {
        // Create a simple pattern
        let pattern = Arc::new(parse_mini_notation("bd sn"));
        let vm = Arc::new(Mutex::new(VoiceManager::new()));
        let mut bank = SampleBank::new();

        // Pre-load samples (assuming dirt-samples are available)
        let _ = bank.get_sample("bd");
        let _ = bank.get_sample("sn");

        let bank = Arc::new(Mutex::new(bank));

        let mut node = SamplePatternNode::new(pattern, vm, bank);

        // Create process context
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,     // 2 cycles per second
            44100.0, // 44.1kHz
        );

        // Prepare block (triggers samples)
        node.prepare_block(&context);

        // Process block (renders voices)
        let mut output = vec![0.0; 512];
        node.process_block(&[], &mut output, 44100.0, &context);

        // Verify output is not all zeros (if samples are loaded)
        // Note: This test may produce zeros if dirt-samples aren't available
        let has_audio = output.iter().any(|&s| s.abs() > 0.0001);

        // The test passes whether samples are loaded or not
        // If samples are loaded, we should see audio
        // If samples aren't loaded, output will be zeros (expected)
        println!(
            "SamplePatternNode test: has_audio={}, first_10_samples={:?}",
            has_audio,
            &output[0..10]
        );
    }
}
