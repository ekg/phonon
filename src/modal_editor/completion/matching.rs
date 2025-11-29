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
    /// Optional description for the completion
    pub description: Option<String>,
}

impl Completion {
    /// Create a new completion
    pub fn new(text: String, completion_type: CompletionType, description: Option<String>) -> Self {
        Self {
            text,
            completion_type,
            description,
        }
    }

    /// Get the display label
    pub fn label(&self) -> &str {
        self.completion_type.label()
    }
}

/// Score for fuzzy matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FuzzyScore(usize);

/// Calculate fuzzy match score between input and candidate
///
/// Returns None if no match, or Some(score) where higher is better:
/// - Exact match: 1000
/// - Prefix match: 500 + (1000 - input_len)
/// - Substring match: 100 + position bonus
/// - Character-by-character fuzzy: sum of position bonuses
///
/// # Arguments
/// * `input` - The partial text being completed
/// * `candidate` - The candidate text to match against
fn fuzzy_score(input: &str, candidate: &str) -> Option<FuzzyScore> {
    if input.is_empty() {
        // Empty input matches everything with base score
        return Some(FuzzyScore(50));
    }

    let input_lower = input.to_lowercase();
    let candidate_lower = candidate.to_lowercase();

    // Exact match: highest score
    if candidate_lower == input_lower {
        return Some(FuzzyScore(1000));
    }

    // Prefix match: high score
    // Bonus for shorter candidates (closer to exact match)
    if candidate_lower.starts_with(&input_lower) {
        let length_penalty = (candidate_lower.len() - input_lower.len()).min(400);
        return Some(FuzzyScore(900 - length_penalty));
    }

    // Substring match: medium score (bonus for position)
    if let Some(pos) = candidate_lower.find(&input_lower) {
        // Earlier position gets higher score
        let position_bonus = 100 - pos.min(100);
        return Some(FuzzyScore(100 + position_bonus));
    }

    // Character-by-character fuzzy matching
    let mut input_chars = input_lower.chars().peekable();
    let mut score = 0;
    let mut last_match_pos = 0;

    for (i, ch) in candidate_lower.chars().enumerate() {
        if let Some(&input_ch) = input_chars.peek() {
            if ch == input_ch {
                input_chars.next();
                // Bonus for consecutive matches
                let consecutive_bonus = if i == last_match_pos + 1 { 5 } else { 0 };
                score += 10 + consecutive_bonus;
                last_match_pos = i;
            }
        } else {
            // All input chars matched
            break;
        }
    }

    // Only count as match if all input characters were found
    if input_chars.peek().is_none() {
        Some(FuzzyScore(score))
    } else {
        None
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
    let mut completions: Vec<(Completion, FuzzyScore)> = Vec::new();

    match context {
        CompletionContext::Function => {
            // Show functions with fuzzy matching on name and description
            for func in FUNCTIONS.iter() {
                // Try matching on function name
                if let Some(name_score) = fuzzy_score(partial, func) {
                    let description = FUNCTION_METADATA
                        .get(func)
                        .map(|meta| meta.description.to_string());

                    completions.push((
                        Completion::new(func.to_string(), CompletionType::Function, description),
                        name_score,
                    ));
                } else if let Some(metadata) = FUNCTION_METADATA.get(func) {
                    // Try matching on description
                    if let Some(desc_score) = fuzzy_score(partial, metadata.description) {
                        // Description matches get lower score than name matches
                        let adjusted_score = FuzzyScore(desc_score.0 / 2);
                        completions.push((
                            Completion::new(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                            ),
                            adjusted_score,
                        ));
                    }
                }
            }
        }

        CompletionContext::Sample => {
            // Show both samples and buses (with ~ prefix)
            if partial.starts_with('~') {
                // User typed ~, show only buses
                let partial_no_tilde = partial.trim_start_matches('~');
                for bus in bus_names {
                    if let Some(score) = fuzzy_score(partial_no_tilde, bus) {
                        completions.push((
                            Completion::new(
                                format!("~{}", bus),
                                CompletionType::Bus,
                                Some(format!("Bus reference: ~{}", bus)),
                            ),
                            score,
                        ));
                    }
                }
            } else {
                // Show both samples and buses
                for sample in sample_names {
                    if let Some(score) = fuzzy_score(partial, sample) {
                        completions.push((
                            Completion::new(
                                sample.clone(),
                                CompletionType::Sample,
                                Some(format!("Sample: {}", sample)),
                            ),
                            score,
                        ));
                    }
                }

                for bus in bus_names {
                    if let Some(score) = fuzzy_score(partial, bus) {
                        completions.push((
                            Completion::new(
                                format!("~{}", bus),
                                CompletionType::Bus,
                                Some(format!("Bus reference: ~{}", bus)),
                            ),
                            score,
                        ));
                    }
                }
            }
        }

        CompletionContext::Bus => {
            // User explicitly typed ~, only show buses
            let partial_no_tilde = partial.trim_start_matches('~');
            for bus in bus_names {
                if let Some(score) = fuzzy_score(partial_no_tilde, bus) {
                    completions.push((
                        Completion::new(
                            format!("~{}", bus),
                            CompletionType::Bus,
                            Some(format!("Bus reference: ~{}", bus)),
                        ),
                        score,
                    ));
                }
            }
        }

        CompletionContext::Keyword(func_name) => {
            // Show parameter names for this function
            if let Some(metadata) = FUNCTION_METADATA.get(func_name) {
                for param in &metadata.params {
                    let search_term = partial.trim_start_matches(':');

                    if let Some(score) = fuzzy_score(search_term, param.name) {
                        // Include ':' prefix if user hasn't typed it yet
                        // "gain <tab>" → show ":amount"
                        // "gain :a<tab>" → show "amount" (: already typed)
                        let completion_text = if partial.starts_with(':') {
                            param.name.to_string() // Just "amount"
                        } else {
                            format!(":{}", param.name) // ":amount"
                        };

                        completions.push((
                            Completion::new(
                                completion_text,
                                CompletionType::Keyword,
                                Some(param.description.to_string()),
                            ),
                            score,
                        ));
                    }
                }
            }
        }

        CompletionContext::AfterChain => {
            // Filter to Effects + Filters only
            for func in FUNCTIONS.iter() {
                if let Some(metadata) = FUNCTION_METADATA.get(func) {
                    if metadata.category == "Effects" || metadata.category == "Filters" {
                        if let Some(name_score) = fuzzy_score(partial, func) {
                            let completion = Completion::new(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                            );
                            completions.push((completion, name_score));
                        }
                    }
                }
            }
        }

        CompletionContext::AfterTransform => {
            // Filter to Transforms only
            for func in FUNCTIONS.iter() {
                if let Some(metadata) = FUNCTION_METADATA.get(func) {
                    if metadata.category == "Transforms" {
                        if let Some(name_score) = fuzzy_score(partial, func) {
                            let completion = Completion::new(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                            );
                            completions.push((completion, name_score));
                        }
                    }
                }
            }
        }

        CompletionContext::AfterBusAssignment => {
            // Filter to Generators + Oscillators + Synths + Patterns
            for func in FUNCTIONS.iter() {
                if let Some(metadata) = FUNCTION_METADATA.get(func) {
                    let valid = matches!(
                        metadata.category,
                        "Generators" | "Oscillators" | "Synths" | "Patterns"
                    );
                    if valid {
                        if let Some(name_score) = fuzzy_score(partial, func) {
                            let completion = Completion::new(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                            );
                            completions.push((completion, name_score));
                        }
                    }
                }
            }
        }

        CompletionContext::None => {
            // No completions
        }
    }

    // Sort by fuzzy score (descending), then by type priority, then alphabetically
    completions.sort_by(|(a_completion, a_score), (b_completion, b_score)| {
        use std::cmp::Ordering;

        // First: sort by score (higher is better)
        match b_score.cmp(a_score) {
            Ordering::Equal => {
                // Then: sort by type priority (Function > Sample > Bus > Keyword)
                let type_order = |t: CompletionType| match t {
                    CompletionType::Function => 0,
                    CompletionType::Sample => 1,
                    CompletionType::Bus => 2,
                    CompletionType::Keyword => 3,
                };

                match type_order(a_completion.completion_type)
                    .cmp(&type_order(b_completion.completion_type))
                {
                    Ordering::Equal => {
                        // Finally: sort alphabetically
                        a_completion.text.cmp(&b_completion.text)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    // Extract just the completions (drop scores)
    completions.into_iter().map(|(c, _)| c).collect()
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

        // With fuzzy matching, shorter prefix matches score higher
        // "bd" (2 chars) should come before "bass" and "bend" (4 chars)
        assert_eq!(sample_completions[0].text, "bd");

        // "bass" and "bend" have same length, so they're alphabetically sorted
        assert!(sample_completions.iter().any(|c| c.text == "bass"));
        assert!(sample_completions.iter().any(|c| c.text == "bend"));

        // Check that they appear in alphabetical order when scores are equal
        let bass_idx = sample_completions
            .iter()
            .position(|c| c.text == "bass")
            .unwrap();
        let bend_idx = sample_completions
            .iter()
            .position(|c| c.text == "bend")
            .unwrap();
        assert!(bass_idx < bend_idx);
    }

    #[test]
    fn test_completion_labels() {
        assert_eq!(CompletionType::Function.label(), "[function]");
        assert_eq!(CompletionType::Sample.label(), "[sample]");
        assert_eq!(CompletionType::Bus.label(), "[bus]");
        assert_eq!(CompletionType::Keyword.label(), "[param]");
    }

    #[test]
    fn test_fuzzy_score_exact_match() {
        let score = fuzzy_score("test", "test");
        assert_eq!(score, Some(FuzzyScore(1000)));
    }

    #[test]
    fn test_fuzzy_score_prefix_match() {
        let score = fuzzy_score("rev", "reverb");
        assert!(score.is_some());
        // Prefix matches score high (900 minus length penalty)
        assert!(score.unwrap().0 >= 800);
        assert!(score.unwrap().0 < 1000); // But less than exact match
    }

    #[test]
    fn test_fuzzy_score_substring_match() {
        let score = fuzzy_score("verb", "reverb");
        assert!(score.is_some());
        assert!(score.unwrap().0 >= 100);
        assert!(score.unwrap().0 < 500);
    }

    #[test]
    fn test_fuzzy_score_character_match() {
        let score = fuzzy_score("lpf", "lowpass_filter");
        assert!(score.is_some());
    }

    #[test]
    fn test_fuzzy_score_no_match() {
        let score = fuzzy_score("xyz", "reverb");
        assert_eq!(score, None);
    }

    #[test]
    fn test_fuzzy_score_empty_input() {
        let score = fuzzy_score("", "test");
        assert_eq!(score, Some(FuzzyScore(50)));
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

        // With fuzzy matching, should match all parameters containing 'm'
        // When user types ":m", the colon is already present, so completions
        // return just the param name (to avoid ":m" + ":mix" = ":m:mix")
        assert!(!completions.is_empty());
        assert_eq!(completions[0].text, "mix");

        // Also verify fuzzy matching works - should find room_size and damping
        assert!(completions.iter().any(|c| c.text == "room_size"));
        assert!(completions.iter().any(|c| c.text == "damping"));
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

    #[test]
    fn test_fuzzy_matching_functions() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Function;

        // Test fuzzy matching: "rev" should match "reverb" and "rev"
        let completions = filter_completions("rev", &context, &samples, &buses);

        // Should find both rev and reverb
        assert!(completions.iter().any(|c| c.text == "rev"));
        assert!(completions.iter().any(|c| c.text == "reverb"));

        // "rev" exact match should come before "reverb" prefix match
        let rev_pos = completions.iter().position(|c| c.text == "rev").unwrap();
        let reverb_pos = completions.iter().position(|c| c.text == "reverb").unwrap();
        assert!(rev_pos < reverb_pos);
    }

    #[test]
    fn test_fuzzy_matching_description_search() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Function;

        // Test description search: "echo" should match "delay" (description mentions echo)
        let completions = filter_completions("echo", &context, &samples, &buses);

        // Should find delay via description match
        assert!(completions.iter().any(|c| c.text == "delay"));
    }

    #[test]
    fn test_fuzzy_matching_samples() {
        let samples = vec!["bass".to_string(), "bass3".to_string(), "casio".to_string()];
        let buses = make_buses();
        let context = CompletionContext::Sample;

        // Test fuzzy: "as" should match samples containing "as"
        let completions = filter_completions("as", &context, &samples, &buses);

        // Should find bass, bass3, casio (all contain "as")
        assert!(completions.iter().any(|c| c.text == "bass"));
        assert!(completions.iter().any(|c| c.text == "bass3"));
        assert!(completions.iter().any(|c| c.text == "casio"));

        // Verify prefix matches come first (bass, bass3 start with "bas")
        let first_completion = &completions[0];
        assert!(first_completion.text == "bass" || first_completion.text == "bass3");
    }

    #[test]
    fn test_completion_descriptions() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Function;

        let completions = filter_completions("lpf", &context, &samples, &buses);

        // Find lpf completion
        let lpf = completions.iter().find(|c| c.text == "lpf").unwrap();

        // Should have a description
        assert!(lpf.description.is_some());
        assert!(lpf.description.as_ref().unwrap().contains("Low-pass"));
    }

    #[test]
    fn test_keyword_descriptions() {
        let samples = make_samples();
        let buses = make_buses();
        let context = CompletionContext::Keyword("lpf");

        let completions = filter_completions("", &context, &samples, &buses);

        // Find cutoff parameter
        let cutoff = completions.iter().find(|c| c.text == ":cutoff").unwrap();

        // Should have a description
        assert!(cutoff.description.is_some());
        assert!(cutoff
            .description
            .as_ref()
            .unwrap()
            .contains("cutoff frequency"));
    }

    #[test]
    fn test_sample_and_bus_descriptions() {
        let samples = vec!["bd".to_string()];
        let buses = vec!["drums".to_string()];
        let context = CompletionContext::Sample;

        let completions = filter_completions("", &context, &samples, &buses);

        // Find sample and bus
        let bd = completions.iter().find(|c| c.text == "bd").unwrap();
        let drums = completions.iter().find(|c| c.text == "~drums").unwrap();

        // Should have descriptions
        assert!(bd.description.is_some());
        assert!(bd.description.as_ref().unwrap().contains("Sample:"));

        assert!(drums.description.is_some());
        assert!(drums
            .description
            .as_ref()
            .unwrap()
            .contains("Bus reference:"));
    }
}
