#![allow(unused_variables)]
//! Error diagnostics for better user-facing error messages
//!
//! This module provides helpful error reporting for live coders, including:
//! - Line number tracking
//! - Detection of common syntax mistakes
//! - Actionable error messages

use std::fmt;

/// Diagnostic error with line number and context
#[derive(Debug, Clone)]
pub struct DiagnosticError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
    pub source_line: Option<String>,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "❌ Parse Error at line {}:{}", self.line, self.column)?;
        writeln!(f)?;

        if let Some(source) = &self.source_line {
            writeln!(f, "  {}", source)?;
            writeln!(f, "  {}^", " ".repeat(self.column.saturating_sub(1)))?;
        }

        writeln!(f)?;
        writeln!(f, "Error: {}", self.message)?;

        if let Some(hint) = &self.hint {
            writeln!(f)?;
            writeln!(f, "💡 Hint: {}", hint)?;
        }

        Ok(())
    }
}

/// Analyze unparsed input and provide helpful diagnostics
pub fn diagnose_parse_failure(original_input: &str, remaining: &str) -> DiagnosticError {
    // Calculate how much was successfully parsed
    let parsed_len = original_input.len() - remaining.len();
    let parsed = &original_input[..parsed_len];

    // Count lines and get position
    let lines: Vec<&str> = original_input.lines().collect();
    let mut current_len = 0;
    let mut error_line = 0;
    let mut error_column = 0;
    let mut source_line = String::new();

    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if current_len + line_len > parsed_len {
            error_line = i + 1; // 1-indexed
            error_column = parsed_len - current_len + 1;
            source_line = line.to_string();
            break;
        }
        current_len += line_len;
    }

    // If we didn't find the line, we're at EOF
    if error_line == 0 {
        error_line = lines.len();
        error_column = lines.last().map(|l| l.len()).unwrap_or(0) + 1;
        source_line = lines.last().map(|s| s.to_string()).unwrap_or_default();
    }

    // Get the problematic text
    let problem_text = remaining.trim();
    let problem_preview = if problem_text.len() > 50 {
        format!("{}...", &problem_text[..50])
    } else {
        problem_text.to_string()
    };

    // Detect common syntax errors
    let (message, hint) = detect_common_error(problem_text, &source_line);

    DiagnosticError {
        line: error_line,
        column: error_column,
        message: if message.is_empty() {
            format!("Could not parse: '{}'", problem_preview)
        } else {
            message
        },
        hint,
        source_line: Some(source_line),
    }
}

/// Detect common syntax errors and provide helpful hints
fn detect_common_error(text: &str, source_line: &str) -> (String, Option<String>) {
    let text = text.trim();

    // Check for # used as comment (should use --)
    if source_line.trim_start().starts_with('#') && !source_line.contains(" # ") {
        return (
            "# is the chain operator, not for comments".to_string(),
            Some(
                "Use -- for comments instead of #\n\
                  ❌ Wrong: # This is a comment\n\
                  ✅ Correct: -- This is a comment\n\
                  Note: # is used for chaining: saw 110 # lpf 1000 0.8"
                    .to_string(),
            ),
        );
    }

    // Check for function call with parentheses and commas
    if text.contains("(") && text.contains(",") {
        // Look for patterns like s("bd", ...) or lpf(..., ...)
        if let Some(paren_pos) = text.find('(') {
            let func_name = text[..paren_pos].trim();
            if !func_name.is_empty() {
                return (
                    format!(
                        "Function '{}' called with parentheses and commas",
                        func_name
                    ),
                    Some(format!(
                        "Phonon uses space-separated syntax, not commas.\n\
                                  ❌ Wrong: {} \"bd sn\" 0.8\n\
                                  ✅ Correct: {} \"bd sn\" 0.8",
                        func_name, func_name
                    )),
                );
            }
        }
    }

    // Check for s(...) syntax
    if text.starts_with("s(") || text.contains(" s(") {
        return (
            "Sample function 's' should use space-separated syntax".to_string(),
            Some(
                "Use: s \"pattern\" instead of s(\"pattern\")\n\
                  Example: s \"bd sn hh cp\""
                    .to_string(),
            ),
        );
    }

    // Check for effect functions with parentheses
    let effect_funcs = [
        "lpf", "hpf", "reverb", "delay", "distort", "chorus", "compress",
    ];
    for func in &effect_funcs {
        let pattern = format!("{}(", func);
        if text.contains(&pattern) {
            return (
                format!("Effect '{}' should use space-separated syntax", func),
                Some(format!(
                    "❌ Wrong: {}(1000, 0.8)\n\
                              ✅ Correct: {} 1000 0.8",
                    func, func
                )),
            );
        }
    }

    // Check for synth functions with parentheses and commas
    let synth_funcs = ["supersaw", "superkick", "supersnare", "superfm", "superpwm"];
    for func in &synth_funcs {
        let pattern = format!("{}(", func);
        if text.contains(&pattern) && text.contains(",") {
            return (
                format!("Synth '{}' should use space-separated syntax", func),
                Some(format!(
                    "❌ Wrong: {}(55, 0.4, 5)\n\
                              ✅ Correct: {} 55 0.4 5",
                    func, func
                )),
            );
        }
    }

    // Check for 'bpm' instead of 'tempo'
    if text.starts_with("bpm ") {
        return (
            "'bpm' keyword is not supported".to_string(),
            Some(
                "Use 'tempo:' instead\n\
                  Example: tempo: 2.0"
                    .to_string(),
            ),
        );
    }

    // Check for assignment with = instead of :
    if source_line.contains("=") && !source_line.starts_with("out") {
        if let Some(eq_pos) = source_line.find('=') {
            let before = source_line[..eq_pos].trim();
            if before.starts_with('~') {
                return (
                    "Bus assignment should use ':' not '='".to_string(),
                    Some(format!(
                        "❌ Wrong: {} = ...\n\
                                  ✅ Correct: {} ...",
                        before,
                        before.replace('=', ":")
                    )),
                );
            }
        }
    }

    // Generic error
    (String::new(), None)
}

/// Check entire input for common mistakes and provide warnings
pub fn check_for_common_mistakes(input: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    for (i, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        // Check for # used as comment (should use --)
        if trimmed.starts_with('#') && !trimmed.contains(" # ") {
            warnings.push(format!(
                "Line {}: # is the chain operator. Use '--' for comments.",
                i + 1
            ));
        }

        // Check for parentheses syntax in common functions
        if trimmed.contains("s(")
            || trimmed.contains("lpf(")
            || trimmed.contains("hpf(")
            || trimmed.contains("reverb(")
            || trimmed.contains("supersaw(")
        {
            warnings.push(format!(
                "Line {}: Detected parentheses syntax. Phonon uses space-separated syntax.",
                i + 1
            ));
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hash_comments() {
        let input = "tempo: 2.0\n# This is a comment\nout: sine 440";
        let remaining = "# This is a comment\nout: sine 440";

        let diag = diagnose_parse_failure(input, remaining);
        assert_eq!(diag.line, 2);
        assert!(diag.message.contains("chain operator"));
        assert!(diag.hint.is_some());
    }

    #[test]
    fn test_detect_parentheses_syntax() {
        let input = "tempo: 2.0\n~kick: s(\"bd*4\")";
        let remaining = "~kick: s(\"bd*4\")";

        let diag = diagnose_parse_failure(input, remaining);
        assert!(diag.message.contains("space-separated"));
    }

    #[test]
    fn test_check_common_mistakes() {
        let input = "tempo: 2.0\n# comment\n~kick: s(\"bd\")";
        let warnings = check_for_common_mistakes(input);

        assert!(warnings.len() >= 2);
        assert!(warnings.iter().any(|w| w.contains("chain operator")));
        assert!(warnings.iter().any(|w| w.contains("parentheses")));
    }
}
