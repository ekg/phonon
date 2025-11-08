//! Completion matching and filtering
//!
//! Filters available completions based on partial input and context

use super::context::CompletionContext;
use super::function_metadata::FUNCTION_METADATA;
use crate::modal_editor::highlighting::FUNCTIONS;

/// Type of completion item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionType {
    Function,
    Sample,
    Bus,
    Keyword,
}

impl CompletionType {
    /// Get the display label for this completion type
    pub fn label(&self) -> &'static str {
        match self {
            CompletionType::Function => "[function]",
            CompletionType::Sample => "[sample]",
            CompletionType::Bus => "[bus]",
            CompletionType::Keyword => "[param]",
        }
    }
}

/// A completion suggestion with type information
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    /// The text to insert
    pub text: String,
    /// The type of completion
    pub completion_type: CompletionType,
}

impl Completion {
    /// Create a new completion
    pub fn new(text: String, completion_type: CompletionType) -> Self {
        Self {
            text,
            completion_type,
        }
    }

    /// Get the display label
    pub fn label(&self) -> &str {
        self.completion_type.label()
    }
}

/// Filter and sort completions based on partial input and context
///
/// # Arguments
/// * `partial` - The partial text being completed
/// * `context` - The completion context (Function/Sample/Bus)
/// * `sample_names` - Available sample names from ~/dirt-samples/
/// * `bus_names` - Available bus names from current editor content
///
/// # Returns
/// A sorted list of matching completions
pub fn filter_completions(
    partial: &str,
    context: &CompletionContext,
    sample_names: &[String],
    bus_names: &[String],
) -> Vec<Completion> {
    let mut completions = Vec::new();

    match context {
        CompletionContext::Function => {
            // Only show functions
            for func in FUNCTIONS.iter() {
                if func.starts_with(partial) {
                    completions.push(Completion::new(
                        func.to_string(),
                        CompletionType::Function,
                    ));
                }
            }
        }

        CompletionContext::Sample => {
            // Show both samples and buses (with ~ prefix)
            if partial.starts_with('~') {
                // User typed ~, show only buses
                let partial_no_tilde = partial.trim_start_matches('~');
                for bus in bus_names {
                    if bus.starts_with(partial_no_tilde) {
                        completions.push(Completion::new(
                            format!("~{}", bus),
                            CompletionType::Bus,
                        ));
                    }
                }
            } else {
                // Show both samples and buses
                for sample in sample_names {
                    if sample.starts_with(partial) {
                        completions.push(Completion::new(
                            sample.clone(),
                            CompletionType::Sample,
                        ));
                    }
                }

                for bus in bus_names {
                    if bus.starts_with(partial) {
                        completions.push(Completion::new(
                            format!("~{}", bus),
                            CompletionType::Bus,
                        ));
                    }
                }
            }
        }

        CompletionContext::Bus => {
            // User explicitly typed ~, only show buses
            let partial_no_tilde = partial.trim_start_matches('~');
            for bus in bus_names {
                if bus.starts_with(partial_no_tilde) {
                    completions.push(Completion::new(
                        format!("~{}", bus),
                        CompletionType::Bus,
                    ));
                }
            }
        }

        CompletionContext::Keyword(func_name) => {
            // Show parameter names for this function
            if let Some(metadata) = FUNCTION_METADATA.get(func_name) {
                for param in &metadata.params {
                    let param_with_colon = format!(":{}", param.name);
                    if param_with_colon.starts_with(partial) || param.name.starts_with(partial.trim_start_matches(':')) {
                        completions.push(Completion::new(
                            param_with_colon,
                            CompletionType::Keyword,
                        ));
                    }
                }
            }
        }

        CompletionContext::None => {
            // No completions
        }
    }

    // Sort: samples first, then buses, then keywords, alphabetically within each group
    completions.sort_by(|a, b| {
        use std::cmp::Ordering;
        match (a.completion_type, b.completion_type) {
            (CompletionType::Sample, CompletionType::Bus) => Ordering::Less,
            (CompletionType::Sample, CompletionType::Keyword) => Ordering::Less,
            (CompletionType::Bus, CompletionType::Sample) => Ordering::Greater,
            (CompletionType::Bus, CompletionType::Keyword) => Ordering::Less,
            (CompletionType::Keyword, CompletionType::Sample) => Ordering::Greater,
            (CompletionType::Keyword, CompletionType::Bus) => Ordering::Greater,
            _ => a.text.cmp(&b.text),
        }
    });

    completions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_samples() -> Vec<String> {
        vec![
            "bd".to_string(),
            "bass".to_string(),
            "bend".to_string(),
            "sn".to_string(),
        ]
    }

    fn make_buses() -> Vec<String> {
        vec!["bass".to_string(), "drums".to_string()]
    }

    #[test]
    fn test_function_completions() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Function;

        let completions = filter_completions("fa", &context, &samples, &buses);

        // Should find "fast"
        assert!(completions.iter().any(|c| c.text == "fast"));
        assert!(completions
            .iter()
            .all(|c| c.completion_type == CompletionType::Function));
    }

    #[test]
    fn test_sample_completions_no_prefix() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Sample;

        let completions = filter_completions("b", &context, &samples, &buses);

        // Should find: bd, bass (samples) and ~bass (bus)
        assert!(completions.iter().any(|c| c.text == "bd"));
        assert!(completions.iter().any(|c| c.text == "bass"));
        assert!(completions.iter().any(|c| c.text == "~bass"));

        // Check types
        assert_eq!(
            completions
                .iter()
                .find(|c| c.text == "bd")
                .unwrap()
                .completion_type,
            CompletionType::Sample
        );
        assert_eq!(
            completions
                .iter()
                .find(|c| c.text == "~bass")
                .unwrap()
                .completion_type,
            CompletionType::Bus
        );
    }

    #[test]
    fn test_sample_completions_with_tilde() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Sample;

        let completions = filter_completions("~b", &context, &samples, &buses);

        // Should only show buses
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "~bass");
        assert_eq!(completions[0].completion_type, CompletionType::Bus);
    }

    #[test]
    fn test_bus_completions() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Bus;

        let completions = filter_completions("~d", &context, &samples, &buses);

        // Should only show buses
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "~drums");
        assert_eq!(completions[0].completion_type, CompletionType::Bus);
    }

    #[test]
    fn test_no_matches() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Function;

        let completions = filter_completions("xyz", &context, &samples, &buses);

        assert!(completions.is_empty());
    }

    #[test]
    fn test_empty_partial() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Sample;

        let completions = filter_completions("", &context, &samples, &buses);

        // Should show all samples and buses
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.text == "bd"));
        assert!(completions.iter().any(|c| c.text == "~bass"));
    }

    #[test]
    fn test_sorting_samples_before_buses() {
        let samples = vec!["bass".to_string()];
        let buses = vec!["bass".to_string()];
        let context = CompletionContext::Sample;

        let completions = filter_completions("b", &context, &samples, &buses);

        // Samples should come before buses
        let bass_sample_idx = completions
            .iter()
            .position(|c| c.text == "bass" && c.completion_type == CompletionType::Sample)
            .unwrap();
        let bass_bus_idx = completions
            .iter()
            .position(|c| c.text == "~bass" && c.completion_type == CompletionType::Bus)
            .unwrap();

        assert!(bass_sample_idx < bass_bus_idx);
    }

    #[test]
    fn test_alphabetical_within_type() {
        let samples = vec!["bend".to_string(), "bd".to_string(), "bass".to_string()];
        let buses = make_buses();
        let context = CompletionContext::Sample;

        let completions = filter_completions("b", &context, &samples, &buses);

        // Find sample completions
        let sample_completions: Vec<_> = completions
            .iter()
            .filter(|c| c.completion_type == CompletionType::Sample)
            .collect();

        // Should be alphabetically sorted
        assert_eq!(sample_completions[0].text, "bass");
        assert_eq!(sample_completions[1].text, "bd");
        assert_eq!(sample_completions[2].text, "bend");
    }

    #[test]
    fn test_completion_labels() {
        assert_eq!(CompletionType::Function.label(), "[function]");
        assert_eq!(CompletionType::Sample.label(), "[sample]");
        assert_eq!(CompletionType::Bus.label(), "[bus]");
        assert_eq!(CompletionType::Keyword.label(), "[param]");
    }

    #[test]
    fn test_keyword_completions_lpf() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Keyword("lpf");

        // Should show lpf parameters
        let completions = filter_completions("", &context, &samples, &buses);

        // lpf has cutoff and q parameters
        assert!(completions.iter().any(|c| c.text == ":cutoff"));
        assert!(completions.iter().any(|c| c.text == ":q"));
        assert!(completions
            .iter()
            .all(|c| c.completion_type == CompletionType::Keyword));
    }

    #[test]
    fn test_keyword_completions_reverb() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Keyword("reverb");

        let completions = filter_completions("", &context, &samples, &buses);

        // reverb has room_size, damping, mix parameters
        assert!(completions.iter().any(|c| c.text == ":room_size"));
        assert!(completions.iter().any(|c| c.text == ":damping"));
        assert!(completions.iter().any(|c| c.text == ":mix"));
    }

    #[test]
    fn test_keyword_completions_filtered() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Keyword("reverb");

        // Filter by partial match
        let completions = filter_completions(":m", &context, &samples, &buses);

        // Should only show :mix (starts with :m)
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, ":mix");
    }

    #[test]
    fn test_keyword_completions_no_metadata() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Keyword("unknown");

        // Unknown function should return no completions
        let completions = filter_completions("", &context, &samples, &buses);
        assert!(completions.is_empty());
    }
}
