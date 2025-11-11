#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Mini-notation parser for TidalCycles/Strudel pattern syntax
//!
//! Parses strings like "bd sn [bd bd] sn" into Pattern structures

use crate::pattern::Pattern;

/// Token types in mini-notation
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Symbol(String), // bd, sn, etc.
    Number(f64),    // 1, 2.5, etc.
    Rest,           // ~
    OpenBracket,    // [
    CloseBracket,   // ]
    OpenAngle,      // <
    CloseAngle,     // >
    OpenParen,      // (
    CloseParen,     // )
    Comma,          // ,
    Star,           // *
    Slash,          // /
    Colon,          // :
    At,             // @
    Percent,        // %
    Question,       // ?
    Exclamation,    // !
    Dot,            // .
}

/// Tokenizer for mini-notation
struct Tokenizer {
    input: String,
    position: usize,
}

impl Tokenizer {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_symbol(&mut self) -> String {
        let mut symbol = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                symbol.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        symbol
    }

    fn read_number(&mut self) -> Option<f64> {
        let mut num_str = String::new();
        let mut has_dot = false;

        while let Some(ch) = self.peek() {
            if ch.is_numeric() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        num_str.parse().ok()
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while self.position < self.input.len() {
            self.skip_whitespace();

            if let Some(ch) = self.peek() {
                let token = match ch {
                    '~' => {
                        self.advance();
                        Token::Rest
                    }
                    '[' => {
                        self.advance();
                        Token::OpenBracket
                    }
                    ']' => {
                        self.advance();
                        Token::CloseBracket
                    }
                    '<' => {
                        self.advance();
                        Token::OpenAngle
                    }
                    '>' => {
                        self.advance();
                        Token::CloseAngle
                    }
                    '(' => {
                        self.advance();
                        Token::OpenParen
                    }
                    ')' => {
                        self.advance();
                        Token::CloseParen
                    }
                    ',' => {
                        self.advance();
                        Token::Comma
                    }
                    '*' => {
                        self.advance();
                        Token::Star
                    }
                    '/' => {
                        self.advance();
                        Token::Slash
                    }
                    ':' => {
                        self.advance();
                        Token::Colon
                    }
                    '@' => {
                        self.advance();
                        Token::At
                    }
                    '%' => {
                        self.advance();
                        Token::Percent
                    }
                    '?' => {
                        self.advance();
                        Token::Question
                    }
                    '!' => {
                        self.advance();
                        Token::Exclamation
                    }
                    '.' => {
                        self.advance();
                        Token::Dot
                    }
                    _ if ch.is_numeric() => {
                        if let Some(num) = self.read_number() {
                            Token::Number(num)
                        } else {
                            self.advance();
                            continue;
                        }
                    }
                    _ if ch.is_alphabetic() => {
                        let symbol = self.read_symbol();
                        Token::Symbol(symbol)
                    }
                    _ => {
                        self.advance();
                        continue;
                    }
                };
                tokens.push(token);
            } else {
                break;
            }
        }

        tokens
    }
}

/// Parser for mini-notation
pub struct MiniNotationParser {
    tokens: Vec<Token>,
    position: usize,
}

impl MiniNotationParser {
    pub fn new(input: &str) -> Self {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        Self {
            tokens,
            position: 0,
        }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position);
        self.position += 1;
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1)
    }

    /// Parse the entire pattern
    pub fn parse(&mut self) -> Pattern<String> {
        self.parse_sequence()
    }

    /// Parse a sequence of patterns
    fn parse_sequence(&mut self) -> Pattern<String> {
        let patterns: Vec<Pattern<String>> = Vec::new();
        let mut current_group: Vec<Pattern<String>> = Vec::new();

        while let Some(token) = self.current() {
            match token {
                Token::OpenBracket => {
                    self.advance();
                    let group = self.parse_group();
                    current_group.push(group);
                }
                Token::OpenAngle => {
                    self.advance();
                    let alt = self.parse_alternation();
                    current_group.push(alt);
                }
                Token::OpenParen => {
                    self.advance();
                    let poly = self.parse_polyrhythm();
                    current_group.push(poly);
                }
                Token::Symbol(s) => {
                    let s = s.clone();
                    self.advance();

                    // Check for Euclidean rhythm syntax: sample(pulses,steps)
                    if let Some(Token::OpenParen) = self.current() {
                        self.advance(); // consume (

                        // Parse pulses
                        if let Some(Token::Number(pulses)) = self.current() {
                            let pulses = *pulses as usize;
                            self.advance();

                            // Expect comma
                            if let Some(Token::Comma) = self.current() {
                                self.advance();

                                // Parse steps
                                if let Some(Token::Number(steps)) = self.current() {
                                    let steps = *steps as usize;
                                    self.advance();

                                    // Optional rotation parameter
                                    let rotation = if let Some(Token::Comma) = self.current() {
                                        self.advance();
                                        if let Some(Token::Number(rot)) = self.current() {
                                            let rot = *rot as i32;
                                            self.advance();
                                            rot
                                        } else {
                                            0
                                        }
                                    } else {
                                        0
                                    };

                                    // Expect closing paren
                                    if let Some(Token::CloseParen) = self.current() {
                                        self.advance();

                                        // Create Euclidean pattern as boolean, then map to sample
                                        let euclid_bool =
                                            Pattern::<bool>::euclid(pulses, steps, rotation);
                                        // Convert boolean pattern to sample pattern
                                        let sample_pattern = euclid_bool.fmap(move |hit| {
                                            if hit {
                                                s.clone()
                                            } else {
                                                "~".to_string()
                                            }
                                        });
                                        current_group.push(sample_pattern);
                                        continue;
                                    }
                                }
                            }
                        }
                        // If parsing failed, we might have a polyrhythm instead
                        // Rewind would be nice here, but let's just handle it
                    }

                    // Check for operators
                    if let Some(op_pattern) = self.parse_operators(Pattern::pure(s.clone())) {
                        current_group.push(op_pattern);
                    } else {
                        current_group.push(Pattern::pure(s));
                    }
                }
                Token::Number(n) => {
                    let n = *n;
                    self.advance();

                    if let Some(op_pattern) = self.parse_operators(Pattern::pure(n.to_string())) {
                        current_group.push(op_pattern);
                    } else {
                        current_group.push(Pattern::pure(n.to_string()));
                    }
                }
                Token::Rest => {
                    self.advance();
                    current_group.push(Pattern::silence());
                }
                _ => {
                    self.advance();
                }
            }
        }

        // Convert current_group into a pattern
        if current_group.is_empty() {
            Pattern::silence()
        } else if current_group.len() == 1 {
            current_group.into_iter().next().unwrap()
        } else {
            // Create a sequence where each element takes equal time
            self.fast_cat(current_group)
        }
    }

    /// Parse a bracketed group [a b c] or polyrhythm [a, b, c]
    fn parse_group(&mut self) -> Pattern<String> {
        // First, check if this is a polyrhythm (contains commas)
        let start_pos = self.position;
        let mut has_comma = false;
        let mut depth = 1;

        // Scan ahead to check for commas at this bracket level
        while let Some(token) = self.current() {
            match token {
                Token::CloseBracket => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Token::OpenBracket => depth += 1,
                Token::Comma if depth == 1 => {
                    has_comma = true;
                    break;
                }
                _ => {}
            }
            self.advance();
        }

        // Reset position
        self.position = start_pos;

        if has_comma {
            // Parse as polyrhythm - multiple patterns separated by commas
            let mut patterns = Vec::new();
            let mut current_elements = Vec::new();

            while let Some(token) = self.current() {
                match token {
                    Token::CloseBracket => {
                        self.advance();
                        if !current_elements.is_empty() {
                            patterns.push(self.fast_cat(current_elements));
                        }
                        break;
                    }
                    Token::Comma => {
                        self.advance();
                        if !current_elements.is_empty() {
                            patterns.push(self.fast_cat(current_elements));
                            current_elements = Vec::new();
                        }
                    }
                    _ => {
                        if let Some(elem) = self.parse_element() {
                            current_elements.push(elem);
                        }
                    }
                }
            }

            // Stack all patterns to play simultaneously
            Pattern::stack(patterns)
        } else {
            // Parse as regular group - sequence of elements
            let mut elements = Vec::new();

            while let Some(token) = self.current() {
                match token {
                    Token::CloseBracket => {
                        self.advance();
                        break;
                    }
                    _ => {
                        if let Some(elem) = self.parse_element() {
                            elements.push(elem);
                        }
                    }
                }
            }

            // Groups are played faster
            self.fast_cat(elements)
        }
    }

    /// Parse a single element (could be a symbol with Euclidean notation, etc.)
    fn parse_element(&mut self) -> Option<Pattern<String>> {
        match self.current()? {
            Token::Symbol(s) => {
                let s = s.clone();
                self.advance();

                // Check for Euclidean rhythm syntax: sample(pulses,steps)
                if let Some(Token::OpenParen) = self.current() {
                    // We need to handle alternation in arguments like bd(<3,4>,8)
                    // For now, let's just parse the simple case
                    // TODO: Implement full alternation support in function arguments

                    self.advance(); // consume (

                    // Parse pulses (could be a number or alternation)
                    if let Some(Token::Number(pulses)) = self.current() {
                        let pulses = *pulses as usize;
                        self.advance();

                        // Expect comma
                        if let Some(Token::Comma) = self.current() {
                            self.advance();

                            // Parse steps
                            if let Some(Token::Number(steps)) = self.current() {
                                let steps = *steps as usize;
                                self.advance();

                                // Optional rotation parameter
                                let rotation = if let Some(Token::Comma) = self.current() {
                                    self.advance();
                                    if let Some(Token::Number(rot)) = self.current() {
                                        let rot = *rot as i32;
                                        self.advance();
                                        rot
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                };

                                // Expect closing paren
                                if let Some(Token::CloseParen) = self.current() {
                                    self.advance();

                                    // Create Euclidean pattern
                                    let euclid_bool =
                                        Pattern::<bool>::euclid(pulses, steps, rotation);
                                    let sample_pattern = euclid_bool.fmap(move |hit| {
                                        if hit {
                                            s.clone()
                                        } else {
                                            "~".to_string()
                                        }
                                    });
                                    return Some(sample_pattern);
                                }
                            }
                        }
                    }
                    // If we see <, it's an alternation for the first argument
                    // This is complex to implement properly - would need to create
                    // alternating euclidean patterns
                    // For now, just treat it as a symbol
                    return Some(Pattern::pure(s));
                }

                // Check for operators
                if let Some(op_pattern) = self.parse_operators(Pattern::pure(s.clone())) {
                    Some(op_pattern)
                } else {
                    Some(Pattern::pure(s))
                }
            }
            Token::Number(n) => {
                let n = *n;
                self.advance();

                if let Some(op_pattern) = self.parse_operators(Pattern::pure(n.to_string())) {
                    Some(op_pattern)
                } else {
                    Some(Pattern::pure(n.to_string()))
                }
            }
            Token::Rest => {
                self.advance();
                Some(Pattern::silence())
            }
            Token::OpenBracket => {
                self.advance();
                Some(self.parse_group())
            }
            Token::OpenAngle => {
                self.advance();
                Some(self.parse_alternation())
            }
            _ => {
                self.advance();
                None
            }
        }
    }

    /// Parse alternation <a b c>
    fn parse_alternation(&mut self) -> Pattern<String> {
        let mut elements = Vec::new();

        while let Some(token) = self.current() {
            match token {
                Token::CloseAngle => {
                    self.advance();
                    break;
                }
                Token::Symbol(s) => {
                    let s = s.clone();
                    self.advance();
                    elements.push(Pattern::pure(s));
                }
                Token::Number(n) => {
                    let n = *n;
                    self.advance();
                    elements.push(Pattern::pure(n.to_string()));
                }
                Token::Rest => {
                    self.advance();
                    elements.push(Pattern::silence());
                }
                _ => {
                    self.advance();
                }
            }
        }

        // Alternation plays one element per cycle
        Pattern::slowcat(elements)
    }

    /// Parse polyrhythm (a,b,c)
    fn parse_polyrhythm(&mut self) -> Pattern<String> {
        let mut patterns = Vec::new();
        let mut current = Vec::new();

        while let Some(token) = self.current() {
            match token {
                Token::CloseParen => {
                    self.advance();
                    if !current.is_empty() {
                        patterns.push(self.fast_cat(current));
                    }
                    break;
                }
                Token::Comma => {
                    self.advance();
                    if !current.is_empty() {
                        patterns.push(self.fast_cat(current));
                        current = Vec::new();
                    }
                }
                Token::Symbol(s) => {
                    let s = s.clone();
                    self.advance();
                    current.push(Pattern::pure(s));
                }
                Token::Number(n) => {
                    let n = *n;
                    self.advance();
                    current.push(Pattern::pure(n.to_string()));
                }
                Token::Rest => {
                    self.advance();
                    current.push(Pattern::silence());
                }
                _ => {
                    self.advance();
                }
            }
        }

        // Stack all patterns to play simultaneously
        Pattern::stack(patterns)
    }

    /// Parse operators like *, /, @, ?, !
    fn parse_operators(&mut self, pattern: Pattern<String>) -> Option<Pattern<String>> {
        if let Some(token) = self.current() {
            match token {
                Token::Star => {
                    self.advance();
                    if let Some(Token::Number(n)) = self.current() {
                        let n = *n as usize;
                        self.advance();
                        // For mini-notation, x*n means repeat x n times fast
                        // Create n copies and concatenate them
                        let patterns = vec![pattern; n];
                        return Some(self.fast_cat(patterns));
                    }
                }
                Token::Slash => {
                    self.advance();
                    if let Some(Token::Number(n)) = self.current() {
                        let n = *n;
                        self.advance();
                        return Some(pattern.slow(Pattern::pure(n)));
                    }
                }
                Token::At => {
                    self.advance();
                    if let Some(Token::Number(n)) = self.current() {
                        let n = *n;
                        self.advance();
                        return Some(pattern.late(Pattern::pure(n)));
                    }
                }
                Token::Question => {
                    self.advance();
                    return Some(pattern.degrade());
                }
                Token::Exclamation => {
                    self.advance();
                    return Some(pattern.dup(2));
                }
                Token::Colon => {
                    self.advance();
                    if let Some(Token::Number(n)) = self.current() {
                        let n = *n as usize;
                        self.advance();
                        // This would select the nth element in multi-sample patterns
                        return Some(pattern);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Helper to create a fast concatenation
    fn fast_cat(&self, patterns: Vec<Pattern<String>>) -> Pattern<String> {
        // Use the built-in Pattern::cat which properly sequences patterns
        Pattern::cat(patterns)
    }
}

/// Parse mini-notation string into a Pattern
pub fn parse_mini_notation(input: &str) -> Pattern<String> {
    let mut parser = MiniNotationParser::new(input);
    parser.parse()
}

/// Extended mini-notation with more features
pub fn parse_extended_notation(input: &str) -> Pattern<String> {
    // First handle pattern stacking with |
    if input.contains('|') {
        let parts: Vec<&str> = input.split('|').collect();
        let patterns: Vec<Pattern<String>> = parts
            .iter()
            .map(|part| parse_mini_notation(part.trim()))
            .collect();
        return Pattern::stack(patterns);
    }

    // Handle pattern layering with +
    if input.contains('+') {
        let parts: Vec<&str> = input.split('+').collect();
        let patterns: Vec<Pattern<String>> = parts
            .iter()
            .map(|part| parse_mini_notation(part.trim()))
            .collect();
        return Pattern::stack(patterns);
    }

    parse_mini_notation(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    #[test]
    fn test_tokenizer() {
        let mut tokenizer = Tokenizer::new("bd sn [hh hh] <kick snare>");
        let tokens = tokenizer.tokenize();

        assert_eq!(tokens[0], Token::Symbol("bd".to_string()));
        assert_eq!(tokens[1], Token::Symbol("sn".to_string()));
        assert_eq!(tokens[2], Token::OpenBracket);
        assert_eq!(tokens[3], Token::Symbol("hh".to_string()));
    }

    #[test]
    fn test_simple_pattern() {
        let pattern = parse_mini_notation("bd sn hh cp");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = pattern.query(&state);

        assert_eq!(haps.len(), 4);
        assert_eq!(haps[0].value, "bd");
        assert_eq!(haps[1].value, "sn");
        assert_eq!(haps[2].value, "hh");
        assert_eq!(haps[3].value, "cp");
    }

    #[test]
    fn test_rest_pattern() {
        let pattern = parse_mini_notation("bd ~ sn ~");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = pattern.query(&state);

        // Should have 2 events (rests are silence)
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, "bd");
        assert_eq!(haps[1].value, "sn");
    }

    #[test]
    fn test_group_pattern() {
        let pattern = parse_mini_notation("bd [sn sn] hh");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = pattern.query(&state);

        // Should have 4 events total (bd, sn, sn, hh)
        // The [sn sn] group takes the same time as a single element
        assert!(haps.len() >= 3);
    }

    #[test]
    fn test_alternation_pattern() {
        let pattern = parse_mini_notation("<bd sn cp>");

        // First cycle should have bd
        let state1 = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps1 = pattern.query(&state1);
        assert_eq!(haps1[0].value, "bd");

        // Second cycle should have sn
        let state2 = State {
            span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
            controls: HashMap::new(),
        };
        let haps2 = pattern.query(&state2);
        assert_eq!(haps2[0].value, "sn");
    }

    #[test]
    fn test_operators() {
        // Test repeat operator
        let pattern = parse_mini_notation("bd*3");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = pattern.query(&state);
        assert_eq!(haps.len(), 3);

        // Test degrade operator
        let pattern2 = parse_mini_notation("bd?");
        // Pattern should exist but may be degraded
        assert!(pattern2.query(&state).len() <= 1);
    }

    #[test]
    fn test_polyrhythm() {
        let pattern = parse_mini_notation("(bd,sn cp,hh hh hh)");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = pattern.query(&state);

        // Should have events from all three patterns playing simultaneously
        assert!(haps.len() >= 3);
    }

    #[test]
    fn test_extended_notation() {
        // Test stacking with |
        let pattern = parse_extended_notation("bd sn | cp hh");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let haps = pattern.query(&state);

        // Should have events from both patterns
        assert!(haps.len() >= 4);
    }
}
