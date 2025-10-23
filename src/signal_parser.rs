#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Parser for the modular synthesis DSL
//!
//! Parses text-based signal flow definitions into signal graphs

use crate::signal_graph::{AnalysisType, Node, NodeId, ProcessorType, SignalGraph, SourceType};

/// Token types for the DSL
#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Identifiers and literals
    BusName(String),    // ~name
    Identifier(String), // name
    Number(f64),        // 123.45
    String(String),     // "string"

    // Operators
    Colon, // :
    Arrow, // >>
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Comma,        // ,

    // Keywords
    Route, // route
    When,  // when
    If,    // if (for conditionals)

    // Special
    Newline,
    EOF,
}

/// Tokenizer for the DSL
struct Tokenizer {
    input: Vec<char>,
    position: usize,
}

impl Tokenizer {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.current() == Some('/') && self.peek() == Some('/') {
            while self.current().is_some() && self.current() != Some('\n') {
                self.advance();
            }
        }
    }

    fn read_number(&mut self) -> f64 {
        let mut num_str = String::new();

        while let Some(ch) = self.current() {
            if ch.is_numeric() || ch == '.' {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        num_str.parse().unwrap_or(0.0)
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();

        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        ident
    }

    fn read_string(&mut self) -> String {
        let mut string = String::new();
        self.advance(); // Skip opening quote

        while let Some(ch) = self.current() {
            if ch == '"' {
                self.advance(); // Skip closing quote
                break;
            }
            string.push(ch);
            self.advance();
        }

        string
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        self.skip_comment();

        match self.current() {
            None => Token::EOF,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('~') => {
                self.advance();
                let name = self.read_identifier();
                Token::BusName(name)
            }
            Some('"') => {
                let string = self.read_string();
                Token::String(string)
            }
            Some(':') => {
                self.advance();
                Token::Colon
            }
            Some('>') if self.peek() == Some('>') => {
                self.advance();
                self.advance();
                Token::Arrow
            }
            Some('+') => {
                self.advance();
                Token::Plus
            }
            Some('-') => {
                self.advance();
                Token::Minus
            }
            Some('*') => {
                self.advance();
                Token::Star
            }
            Some('/') => {
                self.advance();
                Token::Slash
            }
            Some('(') => {
                self.advance();
                Token::LeftParen
            }
            Some(')') => {
                self.advance();
                Token::RightParen
            }
            Some('[') => {
                self.advance();
                Token::LeftBracket
            }
            Some(']') => {
                self.advance();
                Token::RightBracket
            }
            Some('{') => {
                self.advance();
                Token::LeftBrace
            }
            Some('}') => {
                self.advance();
                Token::RightBrace
            }
            Some(',') => {
                self.advance();
                Token::Comma
            }
            Some(ch) if ch.is_numeric() => {
                let num = self.read_number();
                Token::Number(num)
            }
            Some(ch) if ch.is_alphabetic() => {
                let ident = self.read_identifier();
                match ident.as_str() {
                    "route" => Token::Route,
                    "when" => Token::When,
                    "if" => Token::If,
                    _ => Token::Identifier(ident),
                }
            }
            Some(ch) => {
                self.advance();
                Token::Identifier(ch.to_string())
            }
        }
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            if token == Token::EOF {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        tokens
    }
}

/// Parser for the DSL
pub struct SignalParser {
    tokens: Vec<Token>,
    position: usize,
    graph: SignalGraph,
    node_counter: usize,
}

impl SignalParser {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            tokens: Vec::new(),
            position: 0,
            graph: SignalGraph::new(sample_rate),
            node_counter: 0,
        }
    }

    fn generate_node_id(&mut self) -> NodeId {
        self.node_counter += 1;
        NodeId(format!("node_{}", self.node_counter))
    }

    fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if *self.current_token() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.current_token()
            ))
        }
    }

    /// Parse a bus definition: ~name: expression
    fn parse_bus_definition(&mut self) -> Result<(), String> {
        if let Token::BusName(name) = self.current_token().clone() {
            self.advance();
            self.expect(Token::Colon)?;

            let _bus_id = self.graph.add_bus(format!("~{name}"), 0.0);
            let _expr_node = self.parse_expression()?;

            // Connect expression to bus
            // For now, we'll just store the bus reference

            Ok(())
        } else {
            Err("Expected bus name".to_string())
        }
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> Result<NodeId, String> {
        self.parse_signal_chain()
    }

    /// Parse a signal chain: source >> processor >> ...
    fn parse_signal_chain(&mut self) -> Result<NodeId, String> {
        let mut current = self.parse_primary()?;

        while let Token::Arrow = self.current_token() {
            self.advance();
            let next = self.parse_primary()?;
            self.graph.connect(current.clone(), next.clone(), 1.0);
            current = next;
        }

        Ok(current)
    }

    /// Parse a primary expression (source, processor, etc.)
    fn parse_primary(&mut self) -> Result<NodeId, String> {
        match self.current_token().clone() {
            Token::Identifier(name) => {
                self.advance();

                // Check if it's a function call
                if let Token::LeftParen = self.current_token() {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.expect(Token::RightParen)?;

                    // Create appropriate node based on function name
                    self.create_function_node(&name, args)
                } else {
                    // It's a reference to something
                    Ok(NodeId(name))
                }
            }
            Token::BusName(name) => {
                self.advance();
                Ok(NodeId(format!("~{name}")))
            }
            Token::Number(_num) => {
                self.advance();
                // Create a constant node
                let node_id = self.generate_node_id();
                // For now, we'll treat numbers as constant sources
                Ok(node_id)
            }
            Token::String(s) => {
                self.advance();
                // Pattern string
                let node_id = self.generate_node_id();
                let node = Node::Pattern {
                    id: node_id.clone(),
                    pattern: s,
                };
                self.graph.add_node(node);
                Ok(node_id)
            }
            _ => Err(format!("Unexpected token: {:?}", self.current_token())),
        }
    }

    /// Parse function arguments
    fn parse_arguments(&mut self) -> Result<Vec<f64>, String> {
        let mut args = Vec::new();

        if let Token::RightParen = self.current_token() {
            return Ok(args);
        }

        loop {
            if let Token::Number(num) = self.current_token() {
                args.push(*num);
                self.advance();
            } else {
                return Err("Expected number in arguments".to_string());
            }

            if let Token::Comma = self.current_token() {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }

    /// Create a node based on function name and arguments
    fn create_function_node(&mut self, name: &str, args: Vec<f64>) -> Result<NodeId, String> {
        let node_id = self.generate_node_id();

        let node = match name {
            // Sources
            "sine" | "sin" => {
                if args.is_empty() {
                    return Err("sine() requires frequency argument".to_string());
                }
                Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sine { freq: args[0] },
                }
            }
            "saw" => {
                if args.is_empty() {
                    return Err("saw() requires frequency argument".to_string());
                }
                Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Saw { freq: args[0] },
                }
            }
            "square" => {
                if args.is_empty() {
                    return Err("square() requires frequency argument".to_string());
                }
                Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Square { freq: args[0] },
                }
            }
            "noise" | "white" => Node::Source {
                id: node_id.clone(),
                source_type: SourceType::Noise,
            },

            // Processors
            "lpf" | "lowpass" => {
                if args.len() < 2 {
                    return Err("lpf() requires cutoff and Q arguments".to_string());
                }
                Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::LowPass {
                        cutoff: args[0],
                        q: args[1],
                    },
                }
            }
            "hpf" | "highpass" => {
                if args.len() < 2 {
                    return Err("hpf() requires cutoff and Q arguments".to_string());
                }
                Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::HighPass {
                        cutoff: args[0],
                        q: args[1],
                    },
                }
            }
            "delay" => {
                if args.is_empty() {
                    return Err("delay() requires time argument".to_string());
                }
                let feedback = if args.len() > 1 { args[1] } else { 0.3 };
                Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::Delay {
                        time: args[0],
                        feedback,
                    },
                }
            }
            "reverb" => {
                let mix = if !args.is_empty() { args[0] } else { 0.3 };
                Node::Processor {
                    id: node_id.clone(),
                    processor_type: ProcessorType::Reverb { mix },
                }
            }

            // Analysis
            "rms" => {
                let window = if !args.is_empty() { args[0] } else { 0.05 };
                Node::Analysis {
                    id: node_id.clone(),
                    analysis_type: AnalysisType::RMS {
                        window_size: window,
                    },
                }
            }
            "pitch" => Node::Analysis {
                id: node_id.clone(),
                analysis_type: AnalysisType::Pitch,
            },
            "transient" => Node::Analysis {
                id: node_id.clone(),
                analysis_type: AnalysisType::Transient,
            },

            _ => return Err(format!("Unknown function: {name}")),
        };

        self.graph.add_node(node);
        Ok(node_id)
    }

    /// Parse a complete DSL string
    pub fn parse(&mut self, input: &str) -> Result<SignalGraph, String> {
        let mut tokenizer = Tokenizer::new(input);
        self.tokens = tokenizer.tokenize();
        self.position = 0;

        // Parse all statements
        while self.current_token() != &Token::EOF {
            // Skip newlines
            if let Token::Newline = self.current_token() {
                self.advance();
                continue;
            }

            // Parse bus definition or expression
            if let Token::BusName(_) = self.current_token() {
                self.parse_bus_definition()?;
            } else {
                // For now, skip other statements
                self.advance();
            }
        }

        // Compute execution order
        self.graph.compute_execution_order()?;

        Ok(std::mem::replace(
            &mut self.graph,
            SignalGraph::new(44100.0),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let mut tokenizer = Tokenizer::new("~lfo: sine(2) >> lpf(1000, 0.7)");
        let tokens = tokenizer.tokenize();

        assert_eq!(tokens[0], Token::BusName("lfo".to_string()));
        assert_eq!(tokens[1], Token::Colon);
        assert_eq!(tokens[2], Token::Identifier("sine".to_string()));
        assert_eq!(tokens[3], Token::LeftParen);
        assert_eq!(tokens[4], Token::Number(2.0));
        assert_eq!(tokens[5], Token::RightParen);
        assert_eq!(tokens[6], Token::Arrow);
        assert_eq!(tokens[7], Token::Identifier("lpf".to_string()));
    }

    #[test]
    fn test_parse_simple_bus() {
        let mut parser = SignalParser::new(44100.0);
        let result = parser.parse("~lfo: sine(2)");

        assert!(result.is_ok());
        let graph = result.unwrap();
        // The graph should contain nodes
        // We'll add more detailed assertions once the implementation is complete
    }

    #[test]
    fn test_parse_signal_chain() {
        let mut parser = SignalParser::new(44100.0);
        let result = parser.parse("~filtered: saw(220) >> lpf(1000, 0.7) >> delay(0.25, 0.3)");

        assert!(result.is_ok());
        let graph = result.unwrap();
        // Verify the chain was created correctly
    }

    #[test]
    fn test_parse_multiple_buses() {
        let mut parser = SignalParser::new(44100.0);
        let input = r#"
            ~lfo: sine(2)
            ~bass: saw(110) >> lpf(2000, 0.8)
            ~lead: square(440) >> delay(0.125)
        "#;

        let result = parser.parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_comments() {
        let mut tokenizer = Tokenizer::new("// This is a comment\n~lfo: sine(2)");
        let tokens = tokenizer.tokenize();

        // Comment should be skipped
        assert_eq!(tokens[0], Token::Newline);
        assert_eq!(tokens[1], Token::BusName("lfo".to_string()));
    }
}
