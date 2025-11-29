/// Pattern Evaluator Node - Converts Phonon patterns to control signals
///
/// This node evaluates a Pattern at each sample, querying for events and
/// outputting their numeric values. It uses sample-and-hold behavior, meaning
/// it holds the last value until a new event occurs.
///
/// # Pattern Evaluation Algorithm
///
/// For each sample:
/// 1. Calculate current cycle position from sample index and tempo (CPS)
/// 2. Query pattern for events at that cycle position (tiny query window)
/// 3. If event found, parse and output its numeric value
/// 4. If no event, hold last value (sample-and-hold)
///
/// # Numeric Value Parsing
///
/// Pattern events (strings) are parsed as:
/// - Direct numbers: "440" -> 440.0, "0.5" -> 0.5
/// - Note names: "c4" -> 261.63 Hz, "a4" -> 440.0 Hz
/// - Rests: "~" -> 0.0 (silence)
///
/// # Example Usage
///
/// ```ignore
/// // Pattern that alternates between 110 Hz and 220 Hz
/// let pattern = parse_mini_notation("110 220");
/// let pattern_node = PatternEvaluatorNode::new(Arc::new(pattern), 2.0);
///
/// // Use as frequency control for oscillator
/// let freq_node_id = 0;  // PatternEvaluator as NodeId 0
/// let osc = OscillatorNode::new(freq_node_id, Waveform::Sine);
/// ```
use crate::audio_node::{AudioNode, NodeId, ProcessContext};
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;
use std::sync::Arc;

/// Pattern Evaluator Node
///
/// Converts a Phonon pattern into a continuous control signal by querying
/// the pattern at each sample position and holding values between events.
pub struct PatternEvaluatorNode {
    /// Pattern to evaluate (shared ownership)
    pattern: Arc<Pattern<String>>,

    /// Last output value (for sample-and-hold)
    last_value: f32,

    /// Total samples processed (for cycle position calculation)
    sample_index: usize,
}

impl PatternEvaluatorNode {
    /// PatternEvaluatorNode - Evaluates numeric patterns at control rate
    ///
    /// Samples pattern values at regular intervals to generate control signals.
    /// Used for functions like `run`, `scan`, `count` that generate numeric sequences
    /// for modulating synthesis parameters.
    ///
    /// # Parameters
    /// - `pattern`: Pattern to evaluate (numeric values)
    ///
    /// # Example
    /// ```phonon
    /// ~freq: run 8
    /// ~osc: sine ~freq
    /// ```
    pub fn new(pattern: Arc<Pattern<String>>) -> Self {
        Self {
            pattern,
            last_value: 0.0,
            sample_index: 0,
        }
    }

    /// Get the current pattern (for inspection/testing)
    pub fn pattern(&self) -> &Arc<Pattern<String>> {
        &self.pattern
    }

    /// Get the last held value
    pub fn last_value(&self) -> f32 {
        self.last_value
    }

    /// Reset the sample index (useful for testing)
    pub fn reset(&mut self) {
        self.sample_index = 0;
        self.last_value = 0.0;
    }

    /// Parse a pattern event string to a numeric value
    ///
    /// Handles:
    /// - Numbers: "440" -> 440.0
    /// - Note names: "c4" -> 261.63 Hz, "a4" -> 440.0 Hz
    /// - Rests: "~" -> 0.0
    fn parse_event_value(&self, event_str: &str) -> Option<f32> {
        let s = event_str.trim();

        // Rest (explicit silence)
        if s == "~" {
            return Some(0.0);
        }

        // Empty string (shouldn't happen, but handle gracefully)
        if s.is_empty() {
            return None;
        }

        // Try parsing as number first
        if let Ok(value) = s.parse::<f32>() {
            return Some(value);
        }

        // Try parsing as note name (c4, a4, cs5, etc.)
        use crate::pattern_tonal::{midi_to_freq, note_to_midi};
        if let Some(midi) = note_to_midi(s) {
            return Some(midi_to_freq(midi) as f32);
        }

        // Unparseable - return None to keep last value
        None
    }
}

impl AudioNode for PatternEvaluatorNode {
    fn process_block(
        &mut self,
        _inputs: &[&[f32]],
        output: &mut [f32],
        sample_rate: f32,
        context: &ProcessContext,
    ) {
        let cps = context.tempo;

        for i in 0..output.len() {
            // Calculate cycle position for this sample
            // cycle_position = (total_samples / sample_rate) * cps
            let time_seconds = self.sample_index as f64 / sample_rate as f64;
            let cycle_pos = time_seconds * cps;

            // Query pattern at this cycle position
            // Use a tiny query window (one sample width)
            let sample_width = 1.0 / sample_rate as f64 / cps;
            let state = State {
                span: TimeSpan::new(
                    Fraction::from_float(cycle_pos),
                    Fraction::from_float(cycle_pos + sample_width),
                ),
                controls: HashMap::new(),
            };

            let events = self.pattern.query(&state);

            // If we have an event, parse and use its value
            if let Some(event) = events.first() {
                if let Some(value) = self.parse_event_value(&event.value) {
                    self.last_value = value;
                }
            }

            // Output (either new value or held value)
            output[i] = self.last_value;
            self.sample_index += 1;
        }
    }

    fn input_nodes(&self) -> Vec<NodeId> {
        vec![] // No inputs - generates from pattern
    }

    fn name(&self) -> &str {
        "PatternEvaluatorNode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_notation_v3::parse_mini_notation;

    #[test]
    fn test_pattern_evaluator_creation() {
        let pattern = parse_mini_notation("110 220 440");
        let node = PatternEvaluatorNode::new(Arc::new(pattern));

        assert_eq!(node.last_value(), 0.0);
        assert_eq!(node.sample_index, 0);
    }

    #[test]
    fn test_pattern_evaluator_parse_numbers() {
        let pattern = parse_mini_notation("110");
        let node = PatternEvaluatorNode::new(Arc::new(pattern));

        assert_eq!(node.parse_event_value("110"), Some(110.0));
        assert_eq!(node.parse_event_value("440.5"), Some(440.5));
        assert_eq!(node.parse_event_value("0"), Some(0.0));
    }

    #[test]
    fn test_pattern_evaluator_parse_rests() {
        let pattern = parse_mini_notation("110");
        let node = PatternEvaluatorNode::new(Arc::new(pattern));

        assert_eq!(node.parse_event_value("~"), Some(0.0));
    }

    #[test]
    fn test_pattern_evaluator_no_inputs() {
        let pattern = parse_mini_notation("110");
        let node = PatternEvaluatorNode::new(Arc::new(pattern));

        assert_eq!(node.input_nodes().len(), 0);
    }
}
