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
    /// Indices of matched characters in the text (for highlighting)
    pub matched_indices: Vec<usize>,
}

impl Completion {
    /// Create a new completion
    pub fn new(text: String, completion_type: CompletionType, description: Option<String>) -> Self {
        Self {
            text,
            completion_type,
            description,
            matched_indices: vec![],
        }
    }

    /// Create a new completion with matched indices
    pub fn with_match(
        text: String,
        completion_type: CompletionType,
        description: Option<String>,
        matched_indices: Vec<usize>,
    ) -> Self {
        Self {
            text,
            completion_type,
            description,
            matched_indices,
        }
    }

    /// Get the display label
    pub fn label(&self) -> &str {
        self.completion_type.label()
    }
}

/// Result of fzf-style fuzzy matching
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FzfMatch {
    /// Match score (higher is better)
    pub score: i32,
    /// Indices of matched characters in the candidate (for highlighting)
    pub matched_indices: Vec<usize>,
}

impl FzfMatch {
    fn new(score: i32, matched_indices: Vec<usize>) -> Self {
        Self {
            score,
            matched_indices,
        }
    }
}

/// Score for fuzzy matching (internal, for backwards compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FuzzyScore(usize);

// fzf-style scoring constants
const SCORE_MATCH: i32 = 16;
const SCORE_GAP_START: i32 = -3;
const SCORE_GAP_EXTENSION: i32 = -1;
const BONUS_BOUNDARY: i32 = 8;
const BONUS_CAMEL: i32 = 7;
const BONUS_FIRST_CHAR: i32 = 8;
const BONUS_CONSECUTIVE: i32 = 4;

/// Classify a character for word boundary detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Lower,
    Upper,
    Digit,
    Delimiter,
    Other,
}

fn char_class(c: char) -> CharClass {
    if c.is_lowercase() {
        CharClass::Lower
    } else if c.is_uppercase() {
        CharClass::Upper
    } else if c.is_ascii_digit() {
        CharClass::Digit
    } else if c == '_' || c == '-' || c == ' ' || c == '/' || c == '.' {
        CharClass::Delimiter
    } else {
        CharClass::Other
    }
}

/// Calculate word boundary bonus based on previous and current character
fn boundary_bonus(prev_class: CharClass, curr_class: CharClass) -> i32 {
    match (prev_class, curr_class) {
        // After delimiter is a strong boundary
        (CharClass::Delimiter, _) => BONUS_BOUNDARY,
        // Start of string is like after delimiter
        (CharClass::Other, _) if prev_class == CharClass::Other => BONUS_BOUNDARY,
        // camelCase: lowercase followed by uppercase
        (CharClass::Lower, CharClass::Upper) => BONUS_CAMEL,
        // After digit transitioning to letter
        (CharClass::Digit, CharClass::Lower) | (CharClass::Digit, CharClass::Upper) => BONUS_BOUNDARY / 2,
        _ => 0,
    }
}

/// fzf-style fuzzy matching algorithm
///
/// Returns None if no match, or Some(FzfMatch) with:
/// - score: Higher is better, based on:
///   - Consecutive match bonus
///   - Word boundary bonus (after _, -, space)
///   - Camel case boundary bonus (lowercase -> uppercase)
///   - First character match bonus
///   - Gap penalty (unmatched chars between matches)
/// - matched_indices: Positions of matched chars for highlighting
///
/// # Arguments
/// * `query` - The search query (what user typed)
/// * `candidate` - The candidate text to match against
pub fn fzf_match(query: &str, candidate: &str) -> Option<FzfMatch> {
    if query.is_empty() {
        // Empty query matches everything with base score
        return Some(FzfMatch::new(50, vec![]));
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let candidate_chars: Vec<char> = candidate.chars().collect();
    let candidate_lower: Vec<char> = candidate.to_lowercase().chars().collect();

    if query_lower.len() > candidate_lower.len() {
        return None;
    }

    // First pass: check if all query chars exist in candidate (in order)
    let mut query_idx = 0;
    let mut first_match_positions: Vec<usize> = Vec::new();

    for (i, &c) in candidate_lower.iter().enumerate() {
        if query_idx < query_lower.len() && c == query_lower[query_idx] {
            first_match_positions.push(i);
            query_idx += 1;
        }
    }

    if query_idx != query_lower.len() {
        return None; // Not all query chars found
    }

    // Use dynamic programming to find optimal match positions
    // We want to maximize: match bonuses - gap penalties
    let (score, matched_indices) =
        find_best_match(&query_lower, &candidate_chars, &candidate_lower);

    Some(FzfMatch::new(score, matched_indices))
}

/// Find the best matching positions using a greedy algorithm with lookahead
fn find_best_match(
    query: &[char],
    candidate: &[char],
    candidate_lower: &[char],
) -> (i32, Vec<usize>) {
    let n = query.len();
    let m = candidate_lower.len();

    if n == 0 {
        return (50, vec![]);
    }

    // For each query char, find all possible positions in candidate
    let mut possible_positions: Vec<Vec<usize>> = vec![vec![]; n];
    for (i, &qc) in query.iter().enumerate() {
        let start = if i == 0 {
            0
        } else {
            possible_positions[i - 1].first().copied().unwrap_or(0) + 1
        };
        for j in start..m {
            if candidate_lower[j] == qc {
                possible_positions[i].push(j);
            }
        }
    }

    // Check if matching is possible
    for (i, positions) in possible_positions.iter().enumerate() {
        if positions.is_empty() {
            return (0, vec![]); // Can't match
        }
        // Ensure positions are after previous query char's first position
        if i > 0 {
            let prev_min = possible_positions[i - 1].first().copied().unwrap_or(0);
            if positions.iter().all(|&p| p <= prev_min) {
                return (0, vec![]);
            }
        }
    }

    // Greedy matching with scoring
    let mut matched_indices = Vec::with_capacity(n);
    let mut total_score = 0i32;
    let mut prev_match_idx: Option<usize> = None;
    let mut in_gap = false;

    for (qi, &qc) in query.iter().enumerate() {
        let start_pos = prev_match_idx.map(|p| p + 1).unwrap_or(0);

        // Find best position for this query char
        let mut best_pos = None;
        let mut best_pos_score = i32::MIN;

        for ci in start_pos..m {
            if candidate_lower[ci] != qc {
                continue;
            }

            // Calculate score for matching at this position
            let mut pos_score = SCORE_MATCH;

            // First character bonus
            if ci == 0 {
                pos_score += BONUS_FIRST_CHAR;
            }

            // Consecutive match bonus
            if let Some(prev) = prev_match_idx {
                if ci == prev + 1 {
                    pos_score += BONUS_CONSECUTIVE;
                }
            }

            // Word boundary bonus
            if ci > 0 {
                let prev_class = char_class(candidate[ci - 1]);
                let curr_class = char_class(candidate[ci]);
                pos_score += boundary_bonus(prev_class, curr_class);
            } else {
                // First char is like after a boundary
                pos_score += BONUS_BOUNDARY;
            }

            // Case match bonus (exact case match)
            if candidate[ci] == query[qi] {
                pos_score += 1;
            }

            // Prefer earlier matches (tie-breaker)
            if best_pos.is_none() || pos_score > best_pos_score {
                best_pos = Some(ci);
                best_pos_score = pos_score;
            }

            // If we found a consecutive match, that's usually best
            if let Some(prev) = prev_match_idx {
                if ci == prev + 1 {
                    break;
                }
            }
        }

        if let Some(pos) = best_pos {
            // Apply gap penalty if there's a gap
            if let Some(prev) = prev_match_idx {
                let gap = pos - prev - 1;
                if gap > 0 {
                    if !in_gap {
                        total_score += SCORE_GAP_START;
                        in_gap = true;
                    }
                    total_score += SCORE_GAP_EXTENSION * (gap as i32 - 1).max(0);
                } else {
                    in_gap = false;
                }
            }

            total_score += best_pos_score;
            matched_indices.push(pos);
            prev_match_idx = Some(pos);
        } else {
            // Should not happen if first pass succeeded
            return (0, vec![]);
        }
    }

    // Bonus for shorter candidates (prefer exact or close matches)
    let length_bonus = ((20 - (m as i32 - n as i32)).max(0)) / 2;
    total_score += length_bonus;

    // Extra bonus for prefix match
    if matched_indices.first() == Some(&0) {
        total_score += 10;
    }

    // Extra bonus for exact match
    if n == m && matched_indices == (0..n).collect::<Vec<_>>() {
        total_score += 100;
    }

    (total_score, matched_indices)
}

/// Wrapper for backwards compatibility - returns FuzzyScore
fn fuzzy_score(input: &str, candidate: &str) -> Option<FuzzyScore> {
    fzf_match(input, candidate).map(|m| FuzzyScore(m.score.max(0) as usize))
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
/// A sorted list of matching completions with match indices for highlighting
pub fn filter_completions(
    partial: &str,
    context: &CompletionContext,
    sample_names: &[String],
    bus_names: &[String],
) -> Vec<Completion> {
    let mut completions: Vec<(Completion, i32)> = Vec::new();

    match context {
        CompletionContext::Function => {
            // Show functions with fuzzy matching on name and description
            for func in FUNCTIONS.iter() {
                // Try matching on function name
                if let Some(fzf) = fzf_match(partial, func) {
                    let description = FUNCTION_METADATA
                        .get(func)
                        .map(|meta| meta.description.to_string());

                    completions.push((
                        Completion::with_match(
                            func.to_string(),
                            CompletionType::Function,
                            description,
                            fzf.matched_indices,
                        ),
                        fzf.score,
                    ));
                } else if let Some(metadata) = FUNCTION_METADATA.get(func) {
                    // Try matching on description
                    if let Some(fzf) = fzf_match(partial, metadata.description) {
                        // Description matches get lower score than name matches
                        let adjusted_score = fzf.score / 2;
                        // Note: matched_indices here apply to description, not name
                        // We leave matched_indices empty since we can't highlight the name
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
                    if let Some(fzf) = fzf_match(partial_no_tilde, bus) {
                        // Shift indices by 1 to account for ~ prefix
                        let shifted_indices: Vec<usize> =
                            fzf.matched_indices.iter().map(|&i| i + 1).collect();
                        completions.push((
                            Completion::with_match(
                                format!("~{}", bus),
                                CompletionType::Bus,
                                Some(format!("Bus reference: ~{}", bus)),
                                shifted_indices,
                            ),
                            fzf.score,
                        ));
                    }
                }
            } else {
                // Show both samples and buses
                for sample in sample_names {
                    if let Some(fzf) = fzf_match(partial, sample) {
                        completions.push((
                            Completion::with_match(
                                sample.clone(),
                                CompletionType::Sample,
                                Some(format!("Sample: {}", sample)),
                                fzf.matched_indices,
                            ),
                            fzf.score,
                        ));
                    }
                }

                for bus in bus_names {
                    if let Some(fzf) = fzf_match(partial, bus) {
                        // Shift indices by 1 to account for ~ prefix
                        let shifted_indices: Vec<usize> =
                            fzf.matched_indices.iter().map(|&i| i + 1).collect();
                        completions.push((
                            Completion::with_match(
                                format!("~{}", bus),
                                CompletionType::Bus,
                                Some(format!("Bus reference: ~{}", bus)),
                                shifted_indices,
                            ),
                            fzf.score,
                        ));
                    }
                }
            }
        }

        CompletionContext::Bus => {
            // User explicitly typed ~, only show buses
            let partial_no_tilde = partial.trim_start_matches('~');
            for bus in bus_names {
                if let Some(fzf) = fzf_match(partial_no_tilde, bus) {
                    // Shift indices by 1 to account for ~ prefix
                    let shifted_indices: Vec<usize> =
                        fzf.matched_indices.iter().map(|&i| i + 1).collect();
                    completions.push((
                        Completion::with_match(
                            format!("~{}", bus),
                            CompletionType::Bus,
                            Some(format!("Bus reference: ~{}", bus)),
                            shifted_indices,
                        ),
                        fzf.score,
                    ));
                }
            }
        }

        CompletionContext::Keyword(func_name) => {
            // Show parameter names for this function
            if let Some(metadata) = FUNCTION_METADATA.get(func_name) {
                for param in &metadata.params {
                    let search_term = partial.trim_start_matches(':');

                    if let Some(fzf) = fzf_match(search_term, param.name) {
                        // Include ':' prefix if user hasn't typed it yet
                        // "gain <tab>" → show ":amount"
                        // "gain :a<tab>" → show "amount" (: already typed)
                        let (completion_text, indices) = if partial.starts_with(':') {
                            (param.name.to_string(), fzf.matched_indices)
                        } else {
                            // Shift indices by 1 to account for : prefix
                            let shifted: Vec<usize> =
                                fzf.matched_indices.iter().map(|&i| i + 1).collect();
                            (format!(":{}", param.name), shifted)
                        };

                        completions.push((
                            Completion::with_match(
                                completion_text,
                                CompletionType::Keyword,
                                Some(param.description.to_string()),
                                indices,
                            ),
                            fzf.score,
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
                        if let Some(fzf) = fzf_match(partial, func) {
                            let completion = Completion::with_match(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                                fzf.matched_indices,
                            );
                            completions.push((completion, fzf.score));
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
                        if let Some(fzf) = fzf_match(partial, func) {
                            let completion = Completion::with_match(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                                fzf.matched_indices,
                            );
                            completions.push((completion, fzf.score));
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
                        if let Some(fzf) = fzf_match(partial, func) {
                            let completion = Completion::with_match(
                                func.to_string(),
                                CompletionType::Function,
                                Some(metadata.description.to_string()),
                                fzf.matched_indices,
                            );
                            completions.push((completion, fzf.score));
                        }
                    }
                }
            }
        }

        CompletionContext::None => {
            // No completions
        }
    }

    // Sort by score (descending), then by type priority, then alphabetically
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
        assert!(score.is_some());
        // Exact match should score very high
        assert!(score.unwrap().0 > 100);
    }

    #[test]
    fn test_fuzzy_score_prefix_match() {
        let score = fuzzy_score("rev", "reverb");
        assert!(score.is_some());
        // Prefix matches should score positively
        assert!(score.unwrap().0 > 0);

        // Prefix match should score less than exact match
        let exact_score = fuzzy_score("reverb", "reverb");
        assert!(exact_score.unwrap().0 > score.unwrap().0);
    }

    #[test]
    fn test_fuzzy_score_substring_match() {
        let score = fuzzy_score("verb", "reverb");
        assert!(score.is_some());
        assert!(score.unwrap().0 > 0);

        // Substring match should score less than prefix match
        let prefix_score = fuzzy_score("rev", "reverb");
        assert!(prefix_score.unwrap().0 > score.unwrap().0);
    }

    #[test]
    fn test_fuzzy_score_character_match() {
        let score = fuzzy_score("lpf", "lowpass_filter");
        assert!(score.is_some());
    }

    // ===== FZF-SPECIFIC TESTS =====

    #[test]
    fn test_fzf_match_returns_matched_indices() {
        // Matching "lpf" in "lowpass_filter" should return the positions of l, p, f
        let result = fzf_match("lpf", "lowpass_filter");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.matched_indices.len(), 3); // 3 chars matched
        assert_eq!(m.matched_indices[0], 0); // 'l' at position 0
        // 'p' at position 3 (lowPass)
        assert_eq!(m.matched_indices[1], 3);
        // 'f' at position 8 (lowpass_Filter)
        assert_eq!(m.matched_indices[2], 8);
    }

    #[test]
    fn test_fzf_match_exact_match_indices() {
        let result = fzf_match("rev", "rev");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.matched_indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_fzf_match_word_boundary_bonus() {
        // "lf" matching "lowpass_filter" should prefer matching at word boundary
        // l at 0, f at 8 (after underscore) should score higher than f at some other position
        let result = fzf_match("lf", "lowpass_filter");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.matched_indices[0], 0); // 'l' at start
        assert_eq!(m.matched_indices[1], 8); // 'f' at word boundary after _
    }

    #[test]
    fn test_fzf_match_camel_case_bonus() {
        // Matching "gn" in "getNodeId" should match g at 0 and N (word boundary)
        let result = fzf_match("gn", "getNodeId");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.matched_indices[0], 0); // 'g' at start
        assert_eq!(m.matched_indices[1], 3); // 'N' at camelCase boundary
    }

    #[test]
    fn test_fzf_match_consecutive_bonus() {
        // "rev" in "reverb" - consecutive matches should score higher than spread out
        let result_consecutive = fzf_match("rev", "reverb");
        let result_spread = fzf_match("rvb", "reverb");

        assert!(result_consecutive.is_some());
        assert!(result_spread.is_some());

        // Consecutive should score higher
        assert!(result_consecutive.unwrap().score > result_spread.unwrap().score);
    }

    #[test]
    fn test_fzf_match_prefers_shorter_candidates() {
        // "rev" should score higher for "rev" than for "reverb"
        let result_short = fzf_match("rev", "rev");
        let result_long = fzf_match("rev", "reverb");

        assert!(result_short.is_some());
        assert!(result_long.is_some());

        assert!(result_short.unwrap().score > result_long.unwrap().score);
    }

    #[test]
    fn test_fzf_match_case_insensitive() {
        // Should match regardless of case
        let result = fzf_match("REV", "reverb");
        assert!(result.is_some());

        let result2 = fzf_match("rev", "REVERB");
        assert!(result2.is_some());
    }

    #[test]
    fn test_fzf_match_case_bonus() {
        // Exact case match should score slightly higher
        let result_exact_case = fzf_match("Rev", "Reverb");
        let result_diff_case = fzf_match("rev", "Reverb");

        assert!(result_exact_case.is_some());
        assert!(result_diff_case.is_some());

        // Exact case gets a small bonus
        assert!(result_exact_case.unwrap().score >= result_diff_case.unwrap().score);
    }

    #[test]
    fn test_fzf_match_no_match() {
        let result = fzf_match("xyz", "reverb");
        assert!(result.is_none());
    }

    #[test]
    fn test_fzf_match_empty_query() {
        let result = fzf_match("", "reverb");
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_indices, Vec::<usize>::new());
    }

    #[test]
    fn test_fzf_match_query_longer_than_candidate() {
        let result = fzf_match("reverb_long", "rev");
        assert!(result.is_none());
    }

    #[test]
    fn test_fzf_match_first_char_bonus() {
        // Matching at first char should score higher than later
        let result_start = fzf_match("r", "reverb");
        let result_middle = fzf_match("v", "reverb");

        assert!(result_start.is_some());
        assert!(result_middle.is_some());

        // First char match has bonus
        assert!(result_start.unwrap().score > result_middle.unwrap().score);
    }

    #[test]
    fn test_completion_has_matched_indices() {
        // Verify that filter_completions returns completions with matched_indices set
        let samples = vec!["bass".to_string()];
        let buses = vec![];
        let context = CompletionContext::Sample;

        let completions = filter_completions("bs", &context, &samples, &buses);

        // Should find "bass" with b and s highlighted
        assert_eq!(completions.len(), 1);
        let bass = &completions[0];
        assert_eq!(bass.text, "bass");
        // "bs" matches "bass" at indices 0 (b) and 2 (s)
        assert_eq!(bass.matched_indices, vec![0, 2]);
    }

    #[test]
    fn test_completion_empty_query_no_indices() {
        // Empty query should have no matched indices
        let samples = vec!["bass".to_string()];
        let buses = vec![];
        let context = CompletionContext::Sample;

        let completions = filter_completions("", &context, &samples, &buses);

        assert!(!completions.is_empty());
        // Empty query = no matches to highlight
        assert!(completions[0].matched_indices.is_empty());
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
