#![allow(unused_variables)]
//! Parser for pattern transformation chains in Phonon DSL
//!
//! Supports syntax like:
//! - s "bd sn" >> fast 2 >> rev
//! - s "bd sn" >> every 4 (slow 2)

use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::Pattern;

#[derive(Debug, Clone, PartialEq)]
pub enum PatternExpr {
    /// Source pattern from mini notation
    MiniNotation(String),

    /// Reference to a named pattern
    Reference(String),

    /// Pattern transformation
    Transform {
        pattern: Box<PatternExpr>,
        op: TransformOp,
    },

    /// Stack multiple patterns
    Stack(Vec<PatternExpr>),

    /// Concatenate patterns
    Cat(Vec<PatternExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransformOp {
    // Time transformations
    Fast(f64),
    Slow(f64),
    Rev,
    Early(f64),
    Late(f64),
    Hurry(f64),

    // Structure
    Every(i32, Box<TransformOp>),
    Chunk(usize, Box<TransformOp>),
    Euclid {
        pulses: usize,
        steps: usize,
        rotation: Option<i32>,
    },

    // Probability
    Degrade,
    DegradeBy(f64),
    Sometimes(Box<TransformOp>),
    Often(Box<TransformOp>),
    Rarely(Box<TransformOp>),

    // Repetition
    Stutter(usize),
    Echo {
        times: usize,
        time: f64,
        feedback: f64,
    },
    Ply(usize),

    // Combination
    Overlay(Box<PatternExpr>),
    Append(Box<PatternExpr>),

    // Stereo
    Jux(Box<TransformOp>),
    JuxBy(f64, Box<TransformOp>),
    Pan(f64),

    // Values
    Add(f64),
    Mul(f64),
    Range(f64, f64),

    // Advanced
    Compress(f64, f64),
    Zoom(f64, f64),
    Inside(f64, Box<TransformOp>),
    Outside(f64, Box<TransformOp>),

    // Custom function (for extensibility)
    Custom(String, Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Pattern(Box<PatternExpr>),
    Function(Box<TransformOp>),
}

/// Parser for pattern expressions
pub struct PatternParser {
    input: String,
    position: usize,
}

impl PatternParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            position: 0,
        }
    }

    /// Parse a complete pattern expression
    pub fn parse(&mut self) -> Result<PatternExpr, String> {
        self.skip_whitespace();

        // Parse the source pattern
        let mut expr = self.parse_source()?;

        // Parse any chained transformations
        while self.consume_str(">>") {
            let transform = self.parse_transform()?;
            expr = PatternExpr::Transform {
                pattern: Box::new(expr),
                op: transform,
            };
        }

        Ok(expr)
    }

    /// Parse a pattern source
    fn parse_source(&mut self) -> Result<PatternExpr, String> {
        self.skip_whitespace();

        if self.peek_char() == Some('s') && self.peek_ahead(1) == Some(' ') {
            // Mini notation: s "pattern"
            self.consume_char(); // consume 's'
            self.skip_whitespace();
            let pattern = self.parse_string()?;
            Ok(PatternExpr::MiniNotation(pattern))
        } else if self.consume_str("stack") {
            // Stack: stack [pat1, pat2, ...]
            self.skip_whitespace();
            self.expect_char('[')?;
            let patterns = self.parse_pattern_list()?;
            self.expect_char(']')?;
            Ok(PatternExpr::Stack(patterns))
        } else if self.consume_str("cat") {
            // Cat: cat [pat1, pat2, ...]
            self.skip_whitespace();
            self.expect_char('[')?;
            let patterns = self.parse_pattern_list()?;
            self.expect_char(']')?;
            Ok(PatternExpr::Cat(patterns))
        } else if self.peek_char() == Some('~') {
            // Pattern reference: ~name
            self.consume_char();
            let name = self.parse_identifier()?;
            Ok(PatternExpr::Reference(name))
        } else {
            Err(
                "Expected pattern source (s \"...\", stack [...], cat [...], or ~reference)"
                    .to_string(),
            )
        }
    }

    /// Parse a transformation operator
    fn parse_transform(&mut self) -> Result<TransformOp, String> {
        self.skip_whitespace();
        let name = self.parse_identifier()?;

        match name.as_str() {
            // Simple transformations (no args)
            "rev" => Ok(TransformOp::Rev),
            "degrade" => Ok(TransformOp::Degrade),

            // Numeric argument
            "fast" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Fast(n))
            }
            "slow" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Slow(n))
            }
            "early" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Early(n))
            }
            "late" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Late(n))
            }
            "hurry" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Hurry(n))
            }
            "degradeBy" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::DegradeBy(n))
            }
            "pan" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Pan(n))
            }
            "add" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Add(n))
            }
            "mul" => {
                let n = self.parse_number_arg()?;
                Ok(TransformOp::Mul(n))
            }

            // Integer argument
            "stutter" => {
                let n = self.parse_number_arg()? as usize;
                Ok(TransformOp::Stutter(n))
            }
            "ply" => {
                let n = self.parse_number_arg()? as usize;
                Ok(TransformOp::Ply(n))
            }

            // Complex arguments
            "every" => {
                let n = self.parse_number_arg()? as i32;
                let func = self.parse_nested_transform()?;
                Ok(TransformOp::Every(n, Box::new(func)))
            }
            "chunk" => {
                let n = self.parse_number_arg()? as usize;
                let func = self.parse_nested_transform()?;
                Ok(TransformOp::Chunk(n, Box::new(func)))
            }
            "sometimes" => {
                let func = self.parse_nested_transform()?;
                Ok(TransformOp::Sometimes(Box::new(func)))
            }
            "often" => {
                let func = self.parse_nested_transform()?;
                Ok(TransformOp::Often(Box::new(func)))
            }
            "rarely" => {
                let func = self.parse_nested_transform()?;
                Ok(TransformOp::Rarely(Box::new(func)))
            }
            "jux" => {
                let func = self.parse_nested_transform()?;
                Ok(TransformOp::Jux(Box::new(func)))
            }

            // Multiple numeric arguments
            "range" => {
                self.skip_whitespace();
                let min = self.parse_number()?;
                self.skip_whitespace();
                let max = self.parse_number()?;
                Ok(TransformOp::Range(min, max))
            }
            "compress" => {
                self.skip_whitespace();
                let start = self.parse_number()?;
                self.skip_whitespace();
                let end = self.parse_number()?;
                Ok(TransformOp::Compress(start, end))
            }
            "zoom" => {
                self.skip_whitespace();
                let start = self.parse_number()?;
                self.skip_whitespace();
                let end = self.parse_number()?;
                Ok(TransformOp::Zoom(start, end))
            }
            "euclid" => {
                self.skip_whitespace();
                let pulses = self.parse_number()? as usize;
                self.skip_whitespace();
                let steps = self.parse_number()? as usize;
                let rotation = if self.peek_char().map(|c| c.is_numeric()).unwrap_or(false) {
                    self.skip_whitespace();
                    Some(self.parse_number()? as i32)
                } else {
                    None
                };
                Ok(TransformOp::Euclid {
                    pulses,
                    steps,
                    rotation,
                })
            }
            "echo" => {
                self.skip_whitespace();
                let times = self.parse_number()? as usize;
                self.skip_whitespace();
                let time = self.parse_number()?;
                self.skip_whitespace();
                let feedback = self.parse_number()?;
                Ok(TransformOp::Echo {
                    times,
                    time,
                    feedback,
                })
            }

            // Unknown - treat as custom
            _ => {
                let args = self.parse_arguments()?;
                Ok(TransformOp::Custom(name, args))
            }
        }
    }

    /// Parse a nested transformation (in parentheses or following whitespace)
    fn parse_nested_transform(&mut self) -> Result<TransformOp, String> {
        self.skip_whitespace();
        if self.peek_char() == Some('(') {
            self.consume_char();
            let transform = self.parse_transform()?;
            self.expect_char(')')?;
            Ok(transform)
        } else {
            self.parse_transform()
        }
    }

    /// Parse a number argument (with or without parens)
    fn parse_number_arg(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        if self.peek_char() == Some('(') {
            self.consume_char();
            let n = self.parse_number()?;
            self.expect_char(')')?;
            Ok(n)
        } else {
            self.parse_number()
        }
    }

    /// Parse function arguments in parentheses
    fn parse_arguments(&mut self) -> Result<Vec<Value>, String> {
        let mut args = Vec::new();

        if self.peek_char() == Some('(') {
            self.consume_char();

            if self.peek_char() != Some(')') {
                loop {
                    args.push(self.parse_value()?);

                    if self.peek_char() == Some(',') {
                        self.consume_char();
                        self.skip_whitespace();
                    } else {
                        break;
                    }
                }
            }

            self.expect_char(')')?;
        }

        Ok(args)
    }

    /// Parse a value (number, string, pattern, or function)
    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_whitespace();

        if self.peek_char() == Some('"') {
            Ok(Value::String(self.parse_string()?))
        } else if self
            .peek_char()
            .map(|c| c.is_numeric() || c == '-')
            .unwrap_or(false)
        {
            Ok(Value::Number(self.parse_number()?))
        } else {
            // Could be a pattern or function - for now treat as string
            Ok(Value::String(self.parse_identifier()?))
        }
    }

    /// Parse a list of pattern expressions
    fn parse_pattern_list(&mut self) -> Result<Vec<PatternExpr>, String> {
        let mut patterns = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek_char() == Some(']') {
                break;
            }

            patterns.push(self.parse()?);

            self.skip_whitespace();
            if self.peek_char() == Some(',') {
                self.consume_char();
            } else if self.peek_char() != Some(']') {
                return Err("Expected ',' or ']' in pattern list".to_string());
            }
        }

        Ok(patterns)
    }

    // Utility parsing methods

    fn parse_identifier(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        let start = self.position;

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.consume_char();
            } else {
                break;
            }
        }

        if self.position == start {
            return Err("Expected identifier".to_string());
        }

        Ok(self.input[start..self.position].to_string())
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        self.expect_char('"')?;

        let start = self.position;
        while let Some(c) = self.peek_char() {
            if c == '"' {
                let s = self.input[start..self.position].to_string();
                self.consume_char();
                return Ok(s);
            }
            self.consume_char();
        }

        Err("Unterminated string".to_string())
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.position;

        // Handle negative numbers
        if self.peek_char() == Some('-') {
            self.consume_char();
        }

        // Parse integer part
        while let Some(c) = self.peek_char() {
            if c.is_numeric() {
                self.consume_char();
            } else {
                break;
            }
        }

        // Parse decimal part
        if self.peek_char() == Some('.') {
            self.consume_char();
            while let Some(c) = self.peek_char() {
                if c.is_numeric() {
                    self.consume_char();
                } else {
                    break;
                }
            }
        }

        if self.position == start {
            return Err("Expected number".to_string());
        }

        self.input[start..self.position]
            .parse()
            .map_err(|_| "Invalid number".to_string())
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.consume_char();
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.input.chars().nth(self.position + n)
    }

    fn consume_char(&mut self) -> Option<char> {
        let c = self.peek_char();
        if c.is_some() {
            self.position += 1;
        }
        c
    }

    fn consume_str(&mut self, s: &str) -> bool {
        self.skip_whitespace();
        if self.input[self.position..].starts_with(s) {
            self.position += s.len();
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        self.skip_whitespace();
        if self.peek_char() == Some(expected) {
            self.consume_char();
            Ok(())
        } else {
            Err(format!("Expected '{expected}'"))
        }
    }
}

/// Convert a PatternExpr to an actual Pattern
pub fn eval_pattern_expr<T>(expr: &PatternExpr) -> Result<Pattern<T>, String>
where
    T: Clone + Send + Sync + 'static + From<String>,
{
    match expr {
        PatternExpr::MiniNotation(s) => {
            // Parse mini notation and convert
            let pattern = parse_mini_notation(s);
            // This is a simplification - need proper type conversion
            Ok(pattern.fmap(|s| T::from(s)))
        }
        PatternExpr::Reference(name) => {
            Err(format!("Pattern references not yet implemented: {name}"))
        }
        PatternExpr::Transform { pattern, op } => {
            let base = eval_pattern_expr(pattern)?;
            apply_transform(base, op)
        }
        PatternExpr::Stack(patterns) => {
            let mut pats = Vec::new();
            for p in patterns {
                pats.push(eval_pattern_expr(p)?);
            }
            Ok(Pattern::stack(pats))
        }
        PatternExpr::Cat(patterns) => {
            let mut pats = Vec::new();
            for p in patterns {
                pats.push(eval_pattern_expr(p)?);
            }
            Ok(Pattern::cat(pats))
        }
    }
}

/// Apply a transformation to a pattern
fn apply_transform<T>(pattern: Pattern<T>, op: &TransformOp) -> Result<Pattern<T>, String>
where
    T: Clone + Send + Sync + 'static,
{
    match op {
        TransformOp::Fast(n) => Ok(pattern.fast(Pattern::pure(*n))),
        TransformOp::Slow(n) => Ok(pattern.slow(Pattern::pure(*n))),
        TransformOp::Rev => Ok(pattern.rev()),
        TransformOp::Early(n) => Ok(pattern.early(Pattern::pure(*n))),
        TransformOp::Late(n) => Ok(pattern.late(Pattern::pure(*n))),
        TransformOp::Degrade => Ok(pattern.degrade()),
        TransformOp::DegradeBy(n) => Ok(pattern.degrade_by(Pattern::pure(*n))),
        TransformOp::Stutter(n) => Ok(pattern.stutter(*n)),
        TransformOp::Every(n, f) => {
            // This needs special handling for the nested function
            Err("Every not yet fully implemented".to_string())
        }
        _ => Err(format!("Transform {op:?} not yet implemented")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mini_notation() {
        let mut parser = PatternParser::new("s \"bd sn hh cp\"");
        let expr = parser.parse().unwrap();

        assert_eq!(expr, PatternExpr::MiniNotation("bd sn hh cp".to_string()));
    }

    #[test]
    fn test_parse_simple_transform() {
        let mut parser = PatternParser::new("s \"bd sn\" >> fast 2");
        let expr = parser.parse().unwrap();

        match expr {
            PatternExpr::Transform { pattern, op } => {
                assert_eq!(*pattern, PatternExpr::MiniNotation("bd sn".to_string()));
                assert_eq!(op, TransformOp::Fast(2.0));
            }
            _ => panic!("Expected Transform"),
        }
    }

    #[test]
    fn test_parse_chained_transforms() {
        let mut parser = PatternParser::new("s \"bd sn\" >> fast 2 >> rev");
        let expr = parser.parse().unwrap();

        // Should parse as Transform(Transform(MiniNotation, Fast), Rev)
        match expr {
            PatternExpr::Transform { op, .. } => {
                assert_eq!(op, TransformOp::Rev);
            }
            _ => panic!("Expected Transform"),
        }
    }

    #[test]
    fn test_parse_every() {
        let mut parser = PatternParser::new("s \"bd sn\" >> every 4 (slow 2)");
        let expr = parser.parse().unwrap();

        match expr {
            PatternExpr::Transform { op, .. } => match op {
                TransformOp::Every(n, func) => {
                    assert_eq!(n, 4);
                    if let TransformOp::Slow(n) = func.as_ref() {
                        assert_eq!(*n, 2.0);
                    } else {
                        panic!("Expected Slow transform");
                    }
                }
                _ => panic!("Expected Every"),
            },
            _ => panic!("Expected Transform"),
        }
    }
}
