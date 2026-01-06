//! Context detection for tab completion
//!
//! Determines what type of completion to show based on cursor position

use super::function_metadata::FUNCTION_METADATA;

/// Token at cursor position
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The token text
    pub text: String,
    /// Start position in the line
    pub start: usize,
    /// End position in the line
    pub end: usize,
}

/// Completion context based on cursor position
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompletionContext {
    /// Inside a string, completing sample names or buses
    Sample,
    /// Outside strings, completing function names
    Function,
    /// Explicitly completing bus references (after ~)
    Bus,
    /// Completing keyword argument after : (function_name)
    Keyword(&'static str),
    /// No completion available
    None,
    /// After # operator - show Effects/Filters only
    AfterChain,
    /// After $ operator - show Transforms only
    AfterTransform,
    /// After : on bus assignment (~name: or out:) - show Generators/Oscillators/Synths
    AfterBusAssignment,
    /// After vst/au/clap/lv2/plugin - complete plugin names
    Plugin,
}

/// Extract the token at the cursor position
///
/// Returns the word being typed at the cursor, along with its boundaries.
/// Tokens are separated by whitespace or special characters.
pub fn get_token_at_cursor(line: &str, cursor_pos: usize) -> Option<Token> {
    if cursor_pos > line.len() {
        return None;
    }

    // Find the start of the current token
    let mut start = cursor_pos;
    while start > 0 {
        let ch = line.chars().nth(start - 1)?;
        if ch.is_whitespace() || "(){}[]:\",".contains(ch) {
            break;
        }
        start -= 1;
    }

    // Find the end of the current token
    let mut end = cursor_pos;
    while end < line.len() {
        let ch = line.chars().nth(end)?;
        if ch.is_whitespace() || "(){}[]:\",".contains(ch) {
            break;
        }
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(Token {
        text: line[start..end].to_string(),
        start,
        end,
    })
}

/// Determine the completion context based on cursor position
///
/// # Examples
///
/// ```ignore
/// // Function context (outside strings)
/// assert!(matches!(get_completion_context("lpf", 2), CompletionContext::Function));
///
/// // Sample context (inside strings)
/// assert!(matches!(get_completion_context("s \"bd", 5), CompletionContext::Sample));
///
/// // Bus context (after ~)
/// assert!(matches!(get_completion_context("s \"~b", 6), CompletionContext::Bus));
///
/// // Keyword context (after function name and :)
/// assert!(matches!(get_completion_context("lpf :", 5), CompletionContext::Keyword("lpf")));
/// ```
pub fn get_completion_context(line: &str, cursor_pos: usize) -> CompletionContext {
    if cursor_pos > line.len() {
        return CompletionContext::None;
    }

    // Check if we're inside a string
    let line_before_cursor = &line[..cursor_pos];
    let quote_count = line_before_cursor.matches('"').count();
    let in_string = quote_count % 2 == 1;

    if in_string {
        // Check if the token starts with ~
        if let Some(token) = get_token_at_cursor(line, cursor_pos) {
            if token.text.starts_with('~') {
                return CompletionContext::Bus;
            }
        }

        // Check if we're in a string after vst/au/clap/lv2/plugin command
        // e.g., `vst "Os<cursor>` should complete plugin names
        let before_quote = line_before_cursor.rfind('"').map(|i| &line_before_cursor[..i]);
        if let Some(before) = before_quote {
            let trimmed = before.trim_end();
            let last_word = trimmed.split_whitespace().last().unwrap_or("");
            if matches!(
                last_word.to_lowercase().as_str(),
                "vst" | "vst3" | "au" | "clap" | "lv2" | "plugin"
            ) {
                return CompletionContext::Plugin;
            }
        }

        // Inside string but not explicitly a bus reference
        return CompletionContext::Sample;
    }

    // Check for operator context (before checking Function context)
    // Only trigger if cursor is AFTER the operator with whitespace
    if !in_string && cursor_pos > 0 {
        let trimmed_before = line_before_cursor.trim_end();

        // Only detect operator context if we're at a word boundary after the operator
        // i.e., "# " (with space) or "#<cursor>" where cursor isn't in the middle of the operator itself
        if let Some(last_char) = trimmed_before.chars().last() {
            // Check if cursor position is past the operator (whitespace after it)
            let at_word_boundary = line_before_cursor.ends_with(' ')
                || (cursor_pos < line.len()
                    && line
                        .chars()
                        .nth(cursor_pos)
                        .map_or(false, |c| c.is_whitespace()));

            if at_word_boundary {
                match last_char {
                    '#' => return CompletionContext::AfterChain,
                    '$' => return CompletionContext::AfterTransform,
                    _ => {}
                }
            }

            // Handle colon for bus assignment separately
            // This requires checking the start of the line
            if last_char == ':' {
                // Check if this is a bus assignment (~name: or out:)
                let trimmed_start = trimmed_before.trim_start();
                // Only trigger AfterBusAssignment if:
                // 1. Line starts with ~ or "out"
                // 2. The colon is immediately after the bus name (no other content after it)
                // 3. We're at a word boundary (space or cursor at end)
                let is_bus_assignment = (trimmed_start.starts_with('~') || trimmed_start.starts_with("out"))
                    && !trimmed_before.contains('#')  // Not in a chain after #
                    && !trimmed_before.contains('$'); // Not after a transform

                if is_bus_assignment && at_word_boundary {
                    return CompletionContext::AfterBusAssignment;
                }
                // Otherwise fall through to keyword detection
            }
        }
    }

    // Check if we're typing a keyword argument (after : following a function name)
    if let Some(func_name) = detect_keyword_context(line, cursor_pos) {
        return CompletionContext::Keyword(func_name);
    }

    // Check if we're right after a function name with just whitespace
    // Example: "gain " or "lpf 800 " should show kwargs
    // We search backwards through tokens to find the function name
    // BUT we only search within the current "segment" - we stop at # or $ operators
    if !in_string && cursor_pos > 0 {
        // Look back for the last non-whitespace token
        let trimmed_before = line_before_cursor.trim_end();
        if trimmed_before != line_before_cursor {
            // Find the last # or $ to determine the current segment
            // We only search for functions within the current segment
            let last_hash = trimmed_before.rfind('#').map(|i| i + 1).unwrap_or(0);
            let last_dollar = trimmed_before.rfind('$').map(|i| i + 1).unwrap_or(0);
            let segment_start = last_hash.max(last_dollar);
            let current_segment = &trimmed_before[segment_start..];

            // We have trailing whitespace - search backwards for a function in current segment
            let tokens: Vec<&str> = current_segment
                .split(|c: char| c.is_whitespace() || "(){}[]".contains(c))
                .filter(|t| !t.is_empty())
                .collect();

            // Search backwards through tokens to find a known function
            for token in tokens.iter().rev() {
                if let Some(metadata) = FUNCTION_METADATA.get(*token) {
                    return CompletionContext::Keyword(metadata.name);
                }
            }
        }
    }

    // Outside strings - could be function or nothing
    // Make sure we're actually on a word
    if let Some(token) = get_token_at_cursor(line, cursor_pos) {
        // Skip if it's a number or operator
        if token.text.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return CompletionContext::None;
        }
        if token.text.chars().all(|c| "(){}[]:|#$<>=+*-/,".contains(c)) {
            return CompletionContext::None;
        }
        return CompletionContext::Function;
    }

    CompletionContext::None
}

/// Detect if we're in a keyword argument context and return the function name
///
/// Returns Some(function_name) if cursor is after `:` following a known function
fn detect_keyword_context(line: &str, cursor_pos: usize) -> Option<&'static str> {
    use super::function_metadata::FUNCTION_METADATA;

    if cursor_pos == 0 {
        return None;
    }

    let line_before_cursor = &line[..cursor_pos];

    // Look for the last `:` before cursor
    let last_colon = line_before_cursor.rfind(':')?;

    // Check if we're right after a colon or typing a parameter name
    let after_colon = &line[last_colon + 1..cursor_pos];
    let after_colon_trimmed = after_colon.trim_start(); // Allow leading whitespace

    // Only consider this a keyword context if:
    // 1. We're right after `:` (nothing after it, possibly with whitespace)
    // 2. Or we're typing something that looks like a parameter name (alphanumeric + _)
    if !after_colon_trimmed.is_empty()
        && !after_colon_trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }

    // Now look backwards to find the function name
    // Parse backwards to find tokens before the colon
    let before_colon = &line[..last_colon].trim_end();

    // Split by common delimiters and get the last token
    let tokens: Vec<&str> = before_colon
        .split(|c: char| c.is_whitespace() || "(){}[]#$".contains(c))
        .collect();

    // Get the last non-empty token (this should be the function name or a previous argument)
    for token in tokens.iter().rev() {
        if token.is_empty() {
            continue;
        }

        // Check if this is a known function in FUNCTION_METADATA
        if FUNCTION_METADATA.contains_key(token) {
            // Need to return a static string, so we look it up in the metadata
            // This works because FUNCTION_METADATA keys are 'static str
            return FUNCTION_METADATA.get(token).map(|meta| meta.name);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_at_start_of_line() {
        let token = get_token_at_cursor("fast", 2).unwrap();
        assert_eq!(token.text, "fast");
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 4);
    }

    #[test]
    fn test_token_in_middle() {
        let token = get_token_at_cursor("s \"bd sn\" fast", 12).unwrap();
        assert_eq!(token.text, "fast");
        assert_eq!(token.start, 10);
        assert_eq!(token.end, 14);
    }

    #[test]
    fn test_token_with_tilde() {
        let token = get_token_at_cursor("~bass", 3).unwrap();
        assert_eq!(token.text, "~bass");
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 5);
    }

    #[test]
    fn test_partial_token() {
        let token = get_token_at_cursor("fa", 2).unwrap();
        assert_eq!(token.text, "fa");
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 2);
    }

    #[test]
    fn test_empty_at_cursor() {
        let token = get_token_at_cursor("  ", 1);
        assert!(token.is_none());
    }

    #[test]
    fn test_context_function() {
        let context = get_completion_context("fa", 2);
        assert_eq!(context, CompletionContext::Function);
    }

    #[test]
    fn test_context_function_with_colon() {
        let context = get_completion_context("out: fa", 7);
        assert_eq!(context, CompletionContext::Function);
    }

    #[test]
    fn test_context_sample_in_string() {
        let context = get_completion_context("s \"bd", 5);
        assert_eq!(context, CompletionContext::Sample);
    }

    #[test]
    fn test_context_sample_partial() {
        let context = get_completion_context("s \"b", 4);
        assert_eq!(context, CompletionContext::Sample);
    }

    #[test]
    fn test_context_bus_with_tilde() {
        let context = get_completion_context("s \"~b", 5);
        assert_eq!(context, CompletionContext::Bus);
    }

    #[test]
    fn test_context_bus_complete() {
        let context = get_completion_context("s \"~bass", 8);
        assert_eq!(context, CompletionContext::Bus);
    }

    #[test]
    fn test_context_none_for_number() {
        let context = get_completion_context("123", 2);
        assert_eq!(context, CompletionContext::None);
    }

    #[test]
    fn test_context_none_for_operator() {
        let context = get_completion_context("#", 1);
        assert_eq!(context, CompletionContext::None);
    }

    #[test]
    fn test_context_after_closing_quote() {
        // After "s \"bd\" " we should show kwargs for "s" since the user
        // might want to type :gain, :pan, etc.
        let context = get_completion_context("s \"bd\" ", 7);
        assert_eq!(context, CompletionContext::Keyword("s"));
    }

    #[test]
    fn test_multiple_strings() {
        // First string
        let context = get_completion_context("s \"bd\" $ fast 2 \"sn", 18);
        assert_eq!(context, CompletionContext::Sample);
    }

    #[test]
    fn test_bus_reference_in_function_position() {
        // ~bass used as a signal source (outside string)
        let context = get_completion_context("out: ~ba", 8);
        assert_eq!(context, CompletionContext::Function);
    }

    #[test]
    fn test_keyword_context_after_colon() {
        let context = get_completion_context("lpf 1000 :", 10);
        assert_eq!(context, CompletionContext::Keyword("lpf"));
    }

    #[test]
    fn test_keyword_context_typing_param() {
        let context = get_completion_context("lpf 1000 :q", 11);
        assert_eq!(context, CompletionContext::Keyword("lpf"));
    }

    #[test]
    fn test_keyword_context_reverb() {
        let context = get_completion_context("reverb 0.8 0.5 :", 16);
        assert_eq!(context, CompletionContext::Keyword("reverb"));
    }

    #[test]
    fn test_keyword_context_in_chain() {
        let context = get_completion_context("~bass: saw 55 # lpf 800 :", 25);
        assert_eq!(context, CompletionContext::Keyword("lpf"));
    }

    #[test]
    fn test_keyword_context_adsr() {
        let context = get_completion_context("adsr 0.01 0.1 :", 15);
        assert_eq!(context, CompletionContext::Keyword("adsr"));
    }

    #[test]
    fn test_no_keyword_context_for_unknown_function() {
        let context = get_completion_context("unknown_func :", 14);
        // Should not detect keyword context for unknown functions
        assert_ne!(context, CompletionContext::Keyword("unknown_func"));
    }

    #[test]
    fn test_keyword_context_not_inside_string() {
        // Colon inside string should not trigger keyword context
        let context = get_completion_context("s \"bd:", 6);
        assert_eq!(context, CompletionContext::Sample);
    }

    #[test]
    fn test_keyword_context_with_space_after_colon() {
        // Should work with space after colon: "gain : <TAB>"
        let context = get_completion_context("gain : ", 7);
        assert_eq!(context, CompletionContext::Keyword("gain"));
    }

    #[test]
    fn test_keyword_context_with_multiple_spaces_after_colon() {
        // Should work with multiple spaces after colon
        let context = get_completion_context("lpf 1000 :   ", 13);
        assert_eq!(context, CompletionContext::Keyword("lpf"));
    }

    #[test]
    fn test_keyword_context_space_then_typing() {
        // Should work when typing after space: "gain : a<TAB>"
        let context = get_completion_context("gain : am", 9);
        assert_eq!(context, CompletionContext::Keyword("gain"));
    }

    #[test]
    fn test_plugin_context_vst() {
        // After vst " should complete plugin names
        let context = get_completion_context("vst \"Os", 7);
        assert_eq!(context, CompletionContext::Plugin);
    }

    #[test]
    fn test_plugin_context_au() {
        // After au " should complete plugin names
        let context = get_completion_context("au \"Alc", 7);
        assert_eq!(context, CompletionContext::Plugin);
    }

    #[test]
    fn test_plugin_context_clap() {
        // After clap " should complete plugin names
        let context = get_completion_context("clap \"Sur", 9);
        assert_eq!(context, CompletionContext::Plugin);
    }

    #[test]
    fn test_plugin_context_plugin() {
        // After plugin " should complete plugin names
        let context = get_completion_context("plugin \"Vi", 10);
        assert_eq!(context, CompletionContext::Plugin);
    }

    #[test]
    fn test_plugin_context_in_bus() {
        // Plugin in bus assignment
        let context = get_completion_context("~synth $ vst \"Osi", 17);
        assert_eq!(context, CompletionContext::Plugin);
    }

    #[test]
    fn test_sample_not_plugin() {
        // s " should be sample, not plugin
        let context = get_completion_context("s \"bd", 5);
        assert_eq!(context, CompletionContext::Sample);
    }

    #[test]
    fn test_keyword_context_function_with_args_space() {
        // "lpf 800 " should show kwargs for lpf
        let context = get_completion_context("lpf 800 ", 8);
        assert_eq!(context, CompletionContext::Keyword("lpf"));
    }

    #[test]
    fn test_keyword_context_function_with_multiple_args_space() {
        // "reverb 0.5 0.8 " should show kwargs for reverb
        let context = get_completion_context("reverb 0.5 0.8 ", 15);
        assert_eq!(context, CompletionContext::Keyword("reverb"));
    }

    #[test]
    fn test_keyword_context_gain_space() {
        // "gain " should show kwargs for gain
        let context = get_completion_context("gain ", 5);
        assert_eq!(context, CompletionContext::Keyword("gain"));
    }

    #[test]
    fn test_after_chain_shows_effects_not_kwargs() {
        // "s \"bd\" # " should show AfterChain (effects), not kwargs for "s"
        let context = get_completion_context("s \"bd\" # ", 9);
        assert_eq!(context, CompletionContext::AfterChain);
    }

    #[test]
    fn test_after_transform_shows_transforms_not_kwargs() {
        // "s \"bd\" $ " should show AfterTransform, not kwargs for "s"
        let context = get_completion_context("s \"bd\" $ ", 9);
        assert_eq!(context, CompletionContext::AfterTransform);
    }

    #[test]
    fn test_kwargs_after_chain_function() {
        // "s \"bd\" # lpf 800 " should show kwargs for lpf, not s
        let context = get_completion_context("s \"bd\" # lpf 800 ", 17);
        assert_eq!(context, CompletionContext::Keyword("lpf"));
    }

    #[test]
    fn test_kwargs_after_transform_function() {
        // "s \"bd\" $ fast 2 " should show kwargs for fast, not s
        let context = get_completion_context("s \"bd\" $ fast 2 ", 16);
        assert_eq!(context, CompletionContext::Keyword("fast"));
    }
}
