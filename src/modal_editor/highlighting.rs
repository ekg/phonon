use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// All function names recognized by the Phonon language
pub const FUNCTIONS: &[&str] = &[
    // Pattern sources
    "s", "euclid", "stack", "cat",
    // Pattern transforms
    "fast", "slow", "rev", "every", "degrade", "degradeBy", "stutter",
    "palindrome", "iter", "jux", "chunk", "scramble", "spread", "spreadr",
    "when", "whenmod", "off", "superimpose",
    // Oscillators
    "sine", "saw", "square", "tri", "triangle", "noise", "pulse",
    // Filters
    "lpf", "hpf", "bpf", "notch",
    // Effects
    "reverb", "plate", "delay", "tapedelay", "tape", "multitap", "pingpong",
    "chorus", "bitcrush", "dist", "distort", "distortion",
    "comp", "compressor", "expand", "expander", "coarse", "djf", "vowel",
    // Envelopes
    "adsr", "ad", "ar",
    // DSP modifiers
    "gain", "pan", "speed", "cut", "note", "n",
    // Structure
    "tempo", "cps", "bpm", "outmix",
    // Outputs
    "out", "o1", "o2", "o3", "o4", "o5", "o6", "o7", "o8",
    "out1", "out2", "out3", "out4", "out5", "out6", "out7", "out8",
    "d1", "d2", "d3", "d4", "d5", "d6", "d7", "d8",
    // Commands
    "hush", "panic",
];

/// Syntax highlight a single line of Phonon code
///
/// Returns a vector of styled spans suitable for rendering in a terminal UI.
///
/// # Color scheme:
/// - Functions (s, fast, lpf, etc.): Blue
/// - Bus references (~name): Magenta
/// - Numbers (123, 45.6): Orange (RGB 255, 165, 0)
/// - Strings ("..."): White
/// - Operators # and $: Hot Pink (RGB 255, 20, 147)
/// - Other operators: Light Gray (RGB 150, 150, 150)
/// - Comments (--): Dark Gray (RGB 100, 100, 100)
/// - Default: White
pub fn highlight_line(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut in_comment = false;

    // Check if line starts with -- (comment)
    let line_trimmed = line.trim_start();
    if line_trimmed.starts_with("--") {
        // Entire line is a comment
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(Color::Rgb(100, 100, 100)),
        ));
        return spans;
    }

    for ch in line.chars() {
        if in_comment {
            current.push(ch);
            continue;
        }

        // String detection
        if ch == '"' {
            if in_string {
                current.push(ch);
                // Mininotation strings → White
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::White),
                ));
                current.clear();
                in_string = false;
            } else {
                // Flush current token
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), token_style(&current)));
                    current.clear();
                }
                current.push(ch);
                in_string = true;
            }
            continue;
        }

        if in_string {
            current.push(ch);
            continue;
        }

        // Operators and delimiters
        if "(){}[]:|#$<>=+*-/,".contains(ch) {
            // Flush current token
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), token_style(&current)));
                current.clear();
            }
            // # and $ → Hot Pink, others → Light Gray
            let color = if ch == '#' || ch == '$' {
                Color::Rgb(255, 20, 147) // Hot Pink
            } else {
                Color::Rgb(150, 150, 150) // Light Gray
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            continue;
        }

        // Whitespace
        if ch.is_whitespace() {
            // Flush current token
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), token_style(&current)));
                current.clear();
            }
            spans.push(Span::raw(ch.to_string()));
            continue;
        }

        current.push(ch);
    }

    // Flush remaining
    if !current.is_empty() {
        let style = if in_comment {
            Style::default().fg(Color::Rgb(100, 100, 100)) // Comments → Dark gray
        } else if in_string {
            Style::default().fg(Color::White) // Strings → White
        } else {
            token_style(&current)
        };
        spans.push(Span::styled(current, style));
    }

    if spans.is_empty() {
        spans.push(Span::raw(" "));
    }

    spans
}

/// Determine the style for a token based on its content
fn token_style(token: &str) -> Style {
    if FUNCTIONS.contains(&token) {
        Style::default().fg(Color::Blue) // Functions → Blue
    } else if token.starts_with('~') {
        Style::default().fg(Color::Magenta) // Buses → Magenta
    } else if token.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Style::default().fg(Color::Rgb(255, 165, 0)) // Numbers → Orange
    } else {
        Style::default().fg(Color::White) // Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span_text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn span_colors(spans: &[Span]) -> Vec<Option<Color>> {
        spans.iter().map(|s| s.style.fg).collect()
    }

    #[test]
    fn test_empty_line() {
        let spans = highlight_line("");
        assert_eq!(spans.len(), 1);
        assert_eq!(span_text(&spans), " ");
    }

    #[test]
    fn test_comment_line() {
        let spans = highlight_line("-- This is a comment");
        assert_eq!(spans.len(), 1);
        assert_eq!(span_text(&spans), "-- This is a comment");
        assert_eq!(span_colors(&spans), vec![Some(Color::Rgb(100, 100, 100))]);
    }

    #[test]
    fn test_function_highlighting() {
        let spans = highlight_line("fast");
        assert!(spans.iter().any(|s| s.content == "fast"));
        let fast_span = spans.iter().find(|s| s.content == "fast").unwrap();
        assert_eq!(fast_span.style.fg, Some(Color::Blue));
    }

    #[test]
    fn test_bus_highlighting() {
        let spans = highlight_line("~bass");
        assert!(spans.iter().any(|s| s.content == "~bass"));
        let bus_span = spans.iter().find(|s| s.content == "~bass").unwrap();
        assert_eq!(bus_span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn test_number_highlighting() {
        let spans = highlight_line("123");
        assert!(spans.iter().any(|s| s.content == "123"));
        let num_span = spans.iter().find(|s| s.content == "123").unwrap();
        assert_eq!(num_span.style.fg, Some(Color::Rgb(255, 165, 0)));
    }

    #[test]
    fn test_float_highlighting() {
        let spans = highlight_line("12.34");
        assert!(spans.iter().any(|s| s.content == "12.34"));
        let num_span = spans.iter().find(|s| s.content == "12.34").unwrap();
        assert_eq!(num_span.style.fg, Some(Color::Rgb(255, 165, 0)));
    }

    #[test]
    fn test_string_highlighting() {
        let spans = highlight_line("\"bd sn hh\"");
        assert!(spans.iter().any(|s| s.content == "\"bd sn hh\""));
        let str_span = spans.iter().find(|s| s.content == "\"bd sn hh\"").unwrap();
        assert_eq!(str_span.style.fg, Some(Color::White));
    }

    #[test]
    fn test_chain_operator_highlighting() {
        let spans = highlight_line("#");
        assert!(spans.iter().any(|s| s.content == "#"));
        let op_span = spans.iter().find(|s| s.content == "#").unwrap();
        assert_eq!(op_span.style.fg, Some(Color::Rgb(255, 20, 147))); // Hot Pink
    }

    #[test]
    fn test_apply_operator_highlighting() {
        let spans = highlight_line("$");
        assert!(spans.iter().any(|s| s.content == "$"));
        let op_span = spans.iter().find(|s| s.content == "$").unwrap();
        assert_eq!(op_span.style.fg, Some(Color::Rgb(255, 20, 147))); // Hot Pink
    }

    #[test]
    fn test_other_operator_highlighting() {
        let spans = highlight_line("(");
        assert!(spans.iter().any(|s| s.content == "("));
        let op_span = spans.iter().find(|s| s.content == "(").unwrap();
        assert_eq!(op_span.style.fg, Some(Color::Rgb(150, 150, 150))); // Light Gray
    }

    #[test]
    fn test_complete_statement() {
        let line = "out: s \"bd sn\" # lpf 1000 0.8";
        let spans = highlight_line(line);

        // Reconstruct the line
        assert_eq!(span_text(&spans), line);

        // Check specific tokens
        assert!(spans.iter().any(|s| s.content == "out"));
        assert!(spans.iter().any(|s| s.content == "s" && s.style.fg == Some(Color::Blue)));
        assert!(spans.iter().any(|s| s.content == "\"bd sn\"" && s.style.fg == Some(Color::White)));
        assert!(spans.iter().any(|s| s.content == "#" && s.style.fg == Some(Color::Rgb(255, 20, 147))));
        assert!(spans.iter().any(|s| s.content == "lpf" && s.style.fg == Some(Color::Blue)));
        assert!(spans.iter().any(|s| s.content == "1000" && s.style.fg == Some(Color::Rgb(255, 165, 0))));
        assert!(spans.iter().any(|s| s.content == "0.8" && s.style.fg == Some(Color::Rgb(255, 165, 0))));
    }

    #[test]
    fn test_bus_definition() {
        let line = "~bass: saw 55 # lpf 800 0.8";
        let spans = highlight_line(line);

        assert_eq!(span_text(&spans), line);

        // Check that ~bass is highlighted as a bus
        assert!(spans.iter().any(|s| s.content == "~bass" && s.style.fg == Some(Color::Magenta)));
        assert!(spans.iter().any(|s| s.content == "saw" && s.style.fg == Some(Color::Blue)));
        assert!(spans.iter().any(|s| s.content == "55" && s.style.fg == Some(Color::Rgb(255, 165, 0))));
    }

    #[test]
    fn test_pattern_transform() {
        let line = "s \"bd sn\" $ fast 2";
        let spans = highlight_line(line);

        assert_eq!(span_text(&spans), line);

        // Check that $ is hot pink
        assert!(spans.iter().any(|s| s.content == "$" && s.style.fg == Some(Color::Rgb(255, 20, 147))));
        // Check that fast is blue
        assert!(spans.iter().any(|s| s.content == "fast" && s.style.fg == Some(Color::Blue)));
        // Check that 2 is orange
        assert!(spans.iter().any(|s| s.content == "2" && s.style.fg == Some(Color::Rgb(255, 165, 0))));
    }

    #[test]
    fn test_multi_output() {
        let line = "o1: s \"bd(4,4)\"";
        let spans = highlight_line(line);

        assert_eq!(span_text(&spans), line);

        // Check that o1 is recognized as a function (output)
        assert!(spans.iter().any(|s| s.content == "o1" && s.style.fg == Some(Color::Blue)));
        assert!(spans.iter().any(|s| s.content == "s" && s.style.fg == Some(Color::Blue)));
    }

    #[test]
    fn test_effects_chain() {
        let line = "s \"bd\" # reverb 0.85 0.4 # delay 0.5 0.3";
        let spans = highlight_line(line);

        assert_eq!(span_text(&spans), line);

        // Check all # operators are hot pink
        let chain_ops: Vec<_> = spans.iter()
            .filter(|s| s.content == "#")
            .collect();
        assert_eq!(chain_ops.len(), 2);
        assert!(chain_ops.iter().all(|s| s.style.fg == Some(Color::Rgb(255, 20, 147))));

        // Check effects are blue
        assert!(spans.iter().any(|s| s.content == "reverb" && s.style.fg == Some(Color::Blue)));
        assert!(spans.iter().any(|s| s.content == "delay" && s.style.fg == Some(Color::Blue)));
    }

    #[test]
    fn test_new_functions() {
        // Test functions that were recently added to the list
        let test_cases = vec![
            ("gain", Color::Blue),
            ("stack", Color::Blue),
            ("dist", Color::Blue),
            ("comp", Color::Blue),
            ("adsr", Color::Blue),
            ("coarse", Color::Blue),
            ("djf", Color::Blue),
            ("vowel", Color::Blue),
        ];

        for (func, expected_color) in test_cases {
            let spans = highlight_line(func);
            let func_span = spans.iter().find(|s| s.content == func)
                .unwrap_or_else(|| panic!("Function {} not found in spans", func));
            assert_eq!(func_span.style.fg, Some(expected_color),
                "Function {} should be colored {:?}", func, expected_color);
        }
    }

    #[test]
    fn test_tempo_line() {
        let line = "tempo: 2.0";
        let spans = highlight_line(line);

        assert_eq!(span_text(&spans), line);

        // tempo is a function
        assert!(spans.iter().any(|s| s.content == "tempo" && s.style.fg == Some(Color::Blue)));
        // 2.0 is a number
        assert!(spans.iter().any(|s| s.content == "2.0" && s.style.fg == Some(Color::Rgb(255, 165, 0))));
    }

    #[test]
    fn test_whitespace_preservation() {
        let line = "s  \"bd\"   #  lpf";
        let spans = highlight_line(line);

        // Reconstruct should match original exactly
        assert_eq!(span_text(&spans), line);
    }

    #[test]
    fn test_unknown_identifier() {
        let line = "unknown_thing";
        let spans = highlight_line(line);

        // Unknown identifiers should be white (default)
        assert!(spans.iter().any(|s| s.content == "unknown_thing" && s.style.fg == Some(Color::White)));
    }
}
