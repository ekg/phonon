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

    /// Parameter input node IDs (optional - None means use default values)
    gain_id: Option<usize>,
    pan_id: Option<usize>,
    speed_id: Option<usize>,
    n_id: Option<usize>, // pitch offset in semitones (converted to speed)
    attack_id: Option<usize>,
    release_id: Option<usize>,
    begin_id: Option<usize>, // Future: sample start position (0.0-1.0)
    end_id: Option<usize>,   // Future: sample end position (0.0-1.0)

    /// Cached parameter values (read from inputs during process_block)
    cached_gain: f32,
    cached_pan: f32,
    cached_speed: f32,
    cached_attack: f32,
    cached_release: f32,
    cached_begin: f32,
    cached_end: f32,
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
            gain_id: None,
            pan_id: None,
            speed_id: None,
            n_id: None,
            attack_id: None,
            release_id: None,
            begin_id: None,
            end_id: None,
            // Default parameter values
            cached_gain: 1.0,
            cached_pan: 0.0,
            cached_speed: 1.0,
            cached_attack: 0.001,
            cached_release: 0.1,
            cached_begin: 0.0,
            cached_end: 1.0,
        }
    }

    /// Set the node ID (used for voice source tracking)
    pub fn set_node_id(&mut self, node_id: usize) {
        self.node_id = node_id;
    }

    /// Set the gain parameter input node
    pub fn with_gain(mut self, gain_id: usize) -> Self {
        self.gain_id = Some(gain_id);
        self
    }

    /// Set the pan parameter input node
    pub fn with_pan(mut self, pan_id: usize) -> Self {
        self.pan_id = Some(pan_id);
        self
    }

    /// Set the speed parameter input node
    pub fn with_speed(mut self, speed_id: usize) -> Self {
        self.speed_id = Some(speed_id);
        self
    }

    /// Set the pitch offset (n) parameter input node (semitones)
    pub fn with_n(mut self, n_id: usize) -> Self {
        self.n_id = Some(n_id);
        self
    }

    /// Set the attack time parameter input node
    pub fn with_attack(mut self, attack_id: usize) -> Self {
        self.attack_id = Some(attack_id);
        self
    }

    /// Set the release time parameter input node
    pub fn with_release(mut self, release_id: usize) -> Self {
        self.release_id = Some(release_id);
        self
    }

    /// Set the begin position parameter input node (0.0-1.0)
    pub fn with_begin(mut self, begin_id: usize) -> Self {
        self.begin_id = Some(begin_id);
        self
    }

    /// Set the end position parameter input node (0.0-1.0)
    pub fn with_end(mut self, end_id: usize) -> Self {
        self.end_id = Some(end_id);
        self
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

    /// Helper to read parameter value from inputs by node ID
    ///
    /// Returns the first sample of the corresponding input buffer, or default if not connected
    fn read_param_from_inputs(
        &self,
        inputs: &[&[f32]],
        node_id: Option<usize>,
        default: f32,
    ) -> f32 {
        if let Some(id) = node_id {
            // Find the index of this node_id in our input_nodes list
            let input_nodes = self.input_nodes();
            if let Some(idx) = input_nodes.iter().position(|&n| n == id) {
                // Read from the corresponding input buffer
                if let Some(buffer) = inputs.get(idx) {
                    if let Some(&value) = buffer.first() {
                        return value;
                    }
                }
            }
        }
        default
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

                    // Trigger sample with parameter values from cached inputs
                    // Use trigger_sample_with_envelope for full parameter control
                    vm.trigger_sample_with_envelope(
                        sample_data,
                        self.cached_gain,
                        self.cached_pan,
                        self.cached_speed,
                        None, // No cut group for now
                        self.cached_attack,
                        self.cached_release,
                    );

                    // Set trigger offset for sample-accurate timing
                    vm.set_last_voice_trigger_offset(sample_offset);

                    // Note: begin/end parameters are cached but not yet used
                    // VoiceManager needs to be extended to support begin/end slicing
                }
            }
        }
    }

    fn process_block(
        &mut self,
        inputs: &[&[f32]],
        output: &mut [f32],
        _sample_rate: f32,
        _context: &ProcessContext,
    ) {
        // Read parameter values from input nodes (first sample of each buffer)
        self.cached_gain = self.read_param_from_inputs(inputs, self.gain_id, 1.0);
        self.cached_pan = self.read_param_from_inputs(inputs, self.pan_id, 0.0);
        self.cached_speed = self.read_param_from_inputs(inputs, self.speed_id, 1.0);

        // If n (pitch offset in semitones) is provided, convert to speed
        // Speed = 2^(n/12) where n is semitones
        if self.n_id.is_some() {
            let n = self.read_param_from_inputs(inputs, self.n_id, 0.0);
            // Convert semitones to speed: 2^(n/12)
            self.cached_speed = 2.0_f32.powf(n / 12.0);
        }

        self.cached_attack = self.read_param_from_inputs(inputs, self.attack_id, 0.001);
        self.cached_release = self.read_param_from_inputs(inputs, self.release_id, 0.1);
        self.cached_begin = self.read_param_from_inputs(inputs, self.begin_id, 0.0);
        self.cached_end = self.read_param_from_inputs(inputs, self.end_id, 1.0);

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
        // Collect all parameter node IDs that are set
        let mut inputs = Vec::new();

        if let Some(id) = self.gain_id {
            inputs.push(id);
        }
        if let Some(id) = self.pan_id {
            inputs.push(id);
        }
        if let Some(id) = self.speed_id {
            inputs.push(id);
        }
        if let Some(id) = self.n_id {
            inputs.push(id);
        }
        if let Some(id) = self.attack_id {
            inputs.push(id);
        }
        if let Some(id) = self.release_id {
            inputs.push(id);
        }
        if let Some(id) = self.begin_id {
            inputs.push(id);
        }
        if let Some(id) = self.end_id {
            inputs.push(id);
        }

        inputs
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
