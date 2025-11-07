//! Context detection for tab completion
//!
//! Determines what type of completion to show based on cursor position

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
    /// No completion available
    None,
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
        // Inside string but not explicitly a bus reference
        return CompletionContext::Sample;
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
        let context = get_completion_context("s \"bd\" ", 7);
        assert_eq!(context, CompletionContext::None);
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
}
