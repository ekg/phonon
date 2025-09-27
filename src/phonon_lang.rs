//! The Complete Phonon Language
//! Unifies Strudel patterns with synthesis DSL
//!
//! This is the ultimate live coding language for music!

use crate::pattern::{Fraction, Pattern, State, TimeSpan};
// use crate::enhanced_parser::{Expression, Token};  // TODO: Use when integrating parser
use crate::signal_graph::{NodeId, SignalGraph};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A Phonon value can be a pattern, a signal, or a number
#[derive(Clone)]
pub enum PhononValue {
    Pattern(Pattern<f64>),
    Signal(NodeId),
    Number(f64),
    String(String),
    PatternString(Pattern<String>),
}

/// The complete Phonon environment
pub struct PhononEnv {
    /// Pattern bindings (like ~rhythm: "bd sn")
    pub patterns: HashMap<String, PhononValue>,

    /// Signal graph for synthesis
    pub signal_graph: Arc<RwLock<SignalGraph>>,

    /// Current time for pattern evaluation
    pub current_time: f64,

    /// Tempo in cycles per second
    pub cps: f64,
}

impl PhononEnv {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            patterns: HashMap::new(),
            signal_graph: Arc::new(RwLock::new(SignalGraph::new(sample_rate))),
            current_time: 0.0,
            cps: 0.5, // 120 BPM in 4/4
        }
    }

    /// Parse and evaluate a complete Phonon program
    pub fn eval(&mut self, code: &str) -> Result<(), String> {
        let lines: Vec<&str> = code
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with("//"))
            .collect();

        for line in lines {
            self.eval_line(line)?;
        }

        Ok(())
    }

    /// Evaluate a single line of Phonon code
    fn eval_line(&mut self, line: &str) -> Result<(), String> {
        // Parse assignment: ~name: expression
        if let Some((name, expr)) = line.split_once(':') {
            let name = name.trim();
            let expr = expr.trim();

            if name.starts_with('~') {
                // Pattern or signal binding
                let var_name = name[1..].to_string();
                let value = self.parse_expression(expr)?;
                self.patterns.insert(var_name, value);
            } else if name == "out" || name == "o" {
                // Output expression
                self.parse_output(expr)?;
            } else if name == "bpm" {
                // Set tempo in BPM
                let bpm: f64 = expr
                    .parse()
                    .map_err(|_| format!("Invalid BPM value: {expr}"))?;
                // Convert BPM to CPS (assuming 4 beats per cycle)
                self.cps = bpm / 240.0;
            } else if name == "cps" {
                // Set tempo in cycles per second
                self.cps = expr
                    .parse()
                    .map_err(|_| format!("Invalid CPS value: {expr}"))?;
            }
        }

        Ok(())
    }

    /// Parse an expression (can be pattern or synthesis)
    fn parse_expression(&mut self, expr: &str) -> Result<PhononValue, String> {
        // Check for >> operator chains (patterns or synthesis)
        if expr.contains(">>") {
            // Split by >> to get chain components
            let parts: Vec<&str> = expr.split(">>").map(|s| s.trim()).collect();

            // Check if first part is a pattern source
            if parts[0].starts_with("s ")
                || parts[0].starts_with("s\"")
                || parts[0].starts_with("stack")
                || parts[0].starts_with("cat")
            {
                return self.parse_pattern_transformation_chain(expr);
            } else {
                // It's a synthesis chain
                return self.parse_synthesis(expr);
            }
        }

        // Check if it's a string pattern (mini-notation)
        if expr.starts_with('"') && expr.ends_with('"') {
            let pattern_str = &expr[1..expr.len() - 1];
            return Ok(PhononValue::PatternString(Pattern::from_string(
                pattern_str,
            )));
        }

        // Check for pattern sources
        if expr.starts_with("s ") || expr.starts_with("s\"") {
            return self.parse_pattern_source(expr);
        }

        // Check if it's a pattern method chain (old dot syntax for compatibility)
        if expr.contains(".fast(")
            || expr.contains(".slow(")
            || expr.contains(".rev(")
            || expr.contains(".every(")
        {
            return self.parse_pattern_chain(expr);
        }

        // Check if it's synthesis (oscillators, filters)
        if expr.contains("sine(") || expr.contains("saw(") || expr.contains("lpf(") {
            return self.parse_synthesis(expr);
        }

        // Check if it's arithmetic
        if expr.contains('+') || expr.contains('*') || expr.contains('-') {
            return self.parse_arithmetic(expr);
        }

        // Variable reference
        if let Some(var_name) = expr.strip_prefix('~') {
            if let Some(value) = self.patterns.get(var_name) {
                return Ok(value.clone());
            }
        }

        // Try to parse as number
        if let Ok(n) = expr.parse::<f64>() {
            return Ok(PhononValue::Number(n));
        }

        // Default to string
        Ok(PhononValue::String(expr.to_string()))
    }

    /// Parse pattern source like s "bd sn"
    fn parse_pattern_source(&mut self, expr: &str) -> Result<PhononValue, String> {
        // Remove 's ' prefix
        let pattern_str = if expr.starts_with("s \"") && expr.ends_with('"') {
            &expr[3..expr.len() - 1]
        } else if expr.starts_with("s ") {
            &expr[2..]
        } else {
            expr
        };

        Ok(PhononValue::PatternString(Pattern::from_string(
            pattern_str,
        )))
    }

    /// Parse pattern transformation chains like s "bd sn" >> fast 2 >> rev
    fn parse_pattern_transformation_chain(&mut self, expr: &str) -> Result<PhononValue, String> {
        let parts: Vec<&str> = expr.split(">>").map(|s| s.trim()).collect();

        // Parse the initial pattern
        let mut pattern = match self.parse_expression(parts[0])? {
            PhononValue::PatternString(p) => p,
            _ => return Err("Expected pattern source".to_string()),
        };

        // Apply each transformation
        for transform in &parts[1..] {
            pattern = self.apply_pattern_transformation(pattern, transform)?;
        }

        Ok(PhononValue::PatternString(pattern))
    }

    /// Apply a pattern transformation like fast 2 or every 4 (slow 2)
    fn apply_pattern_transformation(
        &self,
        pattern: Pattern<String>,
        transform: &str,
    ) -> Result<Pattern<String>, String> {
        let transform = transform.trim();

        // Parse transformation with arguments
        if let Some((op, args)) = transform.split_once(' ') {
            match op {
                "fast" => {
                    let factor: f64 = args
                        .parse()
                        .map_err(|_| format!("Invalid fast factor: {args}"))?;
                    Ok(pattern.fast(factor))
                }
                "slow" => {
                    let factor: f64 = args
                        .parse()
                        .map_err(|_| format!("Invalid slow factor: {args}"))?;
                    Ok(pattern.slow(factor))
                }
                "every" => {
                    // Parse "every n (transformation)"
                    if let Some((n_str, rest)) = args.split_once(' ') {
                        let n: i32 = n_str
                            .parse()
                            .map_err(|_| format!("Invalid every count: {n_str}"))?;

                        // Extract transformation inside parentheses
                        let inner = rest.trim();
                        if inner.starts_with('(') && inner.ends_with(')') {
                            let inner_transform = &inner[1..inner.len() - 1];
                            // For now, we'll apply the transformation directly
                            // In a full implementation, this would recursively parse
                            match inner_transform {
                                "rev" => Ok(pattern.every(n, |p| p.rev())),
                                transform if transform.starts_with("fast ") => {
                                    let factor: f64 = transform[5..]
                                        .parse()
                                        .map_err(|_| "Invalid fast factor in every")?;
                                    Ok(pattern.every(n, move |p| p.fast(factor)))
                                }
                                transform if transform.starts_with("slow ") => {
                                    let factor: f64 = transform[5..]
                                        .parse()
                                        .map_err(|_| "Invalid slow factor in every")?;
                                    Ok(pattern.every(n, move |p| p.slow(factor)))
                                }
                                _ => Err(format!(
                                    "Unsupported transformation in every: {inner_transform}"
                                )),
                            }
                        } else {
                            Err("Expected (transformation) after every count".to_string())
                        }
                    } else {
                        Err("Invalid every syntax".to_string())
                    }
                }
                "degradeBy" => {
                    let prob: f64 = args
                        .parse()
                        .map_err(|_| format!("Invalid degrade probability: {args}"))?;
                    Ok(pattern.degrade_by(prob))
                }
                "pan" => {
                    // Pan would need to convert pattern type
                    // For now, keep as string pattern
                    Ok(pattern)
                }
                _ => Err(format!("Unknown transformation: {op}")),
            }
        } else {
            // Transformations without arguments
            match transform {
                "rev" => Ok(pattern.rev()),
                "palindrome" => Ok(pattern.palindrome()),
                "degrade" => Ok(pattern.degrade()),
                _ => {
                    // Check for function call syntax
                    if let Some(op) = transform.strip_suffix("()") {
                        match op {
                            "rev" => Ok(pattern.rev()),
                            "palindrome" => Ok(pattern.palindrome()),
                            "degrade" => Ok(pattern.degrade()),
                            _ => Err(format!("Unknown transformation: {op}")),
                        }
                    } else {
                        Err(format!("Unknown transformation: {transform}"))
                    }
                }
            }
        }
    }

    /// Parse pattern method chains like "bd sn".fast(2).every(3, rev)
    fn parse_pattern_chain(&mut self, expr: &str) -> Result<PhononValue, String> {
        // Split by dots to get method chain
        let parts: Vec<&str> = expr.split('.').collect();

        // Start with base pattern
        let base = self.parse_expression(parts[0])?;
        let mut pattern = match base {
            PhononValue::PatternString(p) => p,
            PhononValue::String(s) => Pattern::from_string(&s),
            _ => return Err("Expected pattern".to_string()),
        };

        // Apply each method
        for part in &parts[1..] {
            pattern = self.apply_pattern_method(pattern, part)?;
        }

        Ok(PhononValue::PatternString(pattern))
    }

    /// Apply a pattern method like fast(2) or rev()
    fn apply_pattern_method(
        &self,
        pattern: Pattern<String>,
        method: &str,
    ) -> Result<Pattern<String>, String> {
        if let Some(arg) = method
            .strip_prefix("fast(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let factor: f64 = arg.parse().map_err(|_| "Invalid fast factor")?;
            Ok(pattern.fast(factor))
        } else if let Some(arg) = method
            .strip_prefix("slow(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let factor: f64 = arg.parse().map_err(|_| "Invalid slow factor")?;
            Ok(pattern.slow(factor))
        } else if method == "rev()" || method == "rev" {
            Ok(pattern.rev())
        } else if method == "palindrome()" || method == "palindrome" {
            Ok(pattern.palindrome())
        } else if let Some(arg) = method
            .strip_prefix("degrade(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if arg.is_empty() {
                Ok(pattern.degrade())
            } else {
                let prob: f64 = arg.parse().map_err(|_| "Invalid degrade probability")?;
                Ok(pattern.degrade_by(prob))
            }
        } else if method == "s()" || method == "s" {
            // Convert to sample pattern
            Ok(pattern.s())
        } else if method == "note()" || method == "note" {
            // This would need type conversion
            Ok(pattern) // For now, keep as string
        } else {
            Err(format!("Unknown pattern method: {method}"))
        }
    }

    /// Parse synthesis expressions
    fn parse_synthesis(&mut self, expr: &str) -> Result<PhononValue, String> {
        // This would integrate with the existing enhanced_parser
        // For now, return a placeholder
        Ok(PhononValue::String(format!("synth:{expr}")))
    }

    /// Parse arithmetic expressions
    fn parse_arithmetic(&mut self, expr: &str) -> Result<PhononValue, String> {
        // Simple arithmetic parser
        // This would be more sophisticated in production
        if let Some((left, right)) = expr.split_once('+') {
            let l = self.parse_expression(left.trim())?;
            let r = self.parse_expression(right.trim())?;

            match (l, r) {
                (PhononValue::Number(a), PhononValue::Number(b)) => Ok(PhononValue::Number(a + b)),
                _ => Err("Type mismatch in addition".to_string()),
            }
        } else if let Some((left, right)) = expr.split_once('*') {
            let l = self.parse_expression(left.trim())?;
            let r = self.parse_expression(right.trim())?;

            match (l, r) {
                (PhononValue::Number(a), PhononValue::Number(b)) => Ok(PhononValue::Number(a * b)),
                _ => Err("Type mismatch in multiplication".to_string()),
            }
        } else {
            self.parse_expression(expr)
        }
    }

    /// Parse output expression
    fn parse_output(&mut self, expr: &str) -> Result<(), String> {
        // This would connect to the actual audio output
        let _output = self.parse_expression(expr)?;
        // TODO: Route to audio output
        Ok(())
    }
}

/// Mini-notation parser for pattern strings
pub fn parse_mini_notation(input: &str) -> Pattern<String> {
    // Handle groups [a b c]
    if input.starts_with('[') && input.ends_with(']') {
        let inner = &input[1..input.len() - 1];
        return parse_mini_notation(inner).fast(1.0); // Groups play faster
    }

    // Handle alternation <a b c>
    if input.starts_with('<') && input.ends_with('>') {
        let inner = &input[1..input.len() - 1];
        let parts: Vec<Pattern<String>> = inner
            .split_whitespace()
            .map(|s| Pattern::pure(s.to_string()))
            .collect();
        return Pattern::slowcat(parts);
    }

    // Handle repetition a*3
    if let Some((pattern, count)) = input.split_once('*') {
        if let Ok(n) = count.parse::<usize>() {
            let base = parse_mini_notation(pattern);
            return base.fast(n as f64);
        }
    }

    // Handle rest/silence ~
    if input == "~" {
        return Pattern::silence();
    }

    // Handle subdivision a/2
    if let Some((pattern, div)) = input.split_once('/') {
        if let Ok(n) = div.parse::<f64>() {
            let base = parse_mini_notation(pattern);
            return base.slow(n);
        }
    }

    // Default: parse as sequence
    Pattern::from_string(input)
}

/// The unified Phonon language parser
pub struct PhononParser {
    env: PhononEnv,
}

impl PhononParser {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            env: PhononEnv::new(sample_rate),
        }
    }

    /// Parse a complete Phonon program
    pub fn parse(&mut self, code: &str) -> Result<PhononProgram, String> {
        self.env.eval(code)?;

        Ok(PhononProgram {
            patterns: self.env.patterns.clone(),
            signal_graph: self.env.signal_graph.clone(),
        })
    }
}

/// A compiled Phonon program ready for execution
pub struct PhononProgram {
    pub patterns: HashMap<String, PhononValue>,
    pub signal_graph: Arc<RwLock<SignalGraph>>,
}

impl PhononProgram {
    /// Execute the program for one cycle
    pub fn execute_cycle(&self, cycle: f64) -> Vec<f32> {
        // Query all patterns for this cycle
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle),
                Fraction::from_float(cycle + 1.0),
            ),
            controls: HashMap::new(),
        };

        // Collect events from patterns
        for (name, value) in &self.patterns {
            if let PhononValue::PatternString(p) = value {
                let events = p.query(&state);
                // TODO: Trigger samples/synths based on events
                for event in events {
                    println!("Event: {} at {}", event.value, event.part.begin.to_float());
                }
            }
        }

        // Generate audio from signal graph
        // TODO: Connect to actual synthesis
        vec![0.0; 512]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phonon_pattern_parsing() {
        let mut parser = PhononParser::new(44100.0);
        let code = r#"
~rhythm: "bd sn bd sn"
~bass: "c2 c2 g2 c3"
out: ~rhythm
"#;

        let program = parser.parse(code).unwrap();
        assert!(program.patterns.contains_key("rhythm"));
        assert!(program.patterns.contains_key("bass"));
    }

    #[test]
    fn test_pattern_methods() {
        let mut parser = PhononParser::new(44100.0);
        let code = r#"
~fast_rhythm: "bd sn".fast(2)
~reversed: "1 2 3 4".rev()
out: ~fast_rhythm
"#;

        let program = parser.parse(code).unwrap();
        assert!(program.patterns.contains_key("fast_rhythm"));
        assert!(program.patterns.contains_key("reversed"));
    }

    #[test]
    fn test_mini_notation_groups() {
        let pattern = parse_mini_notation("[bd sn] cp");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        assert!(!events.is_empty());
    }
}
