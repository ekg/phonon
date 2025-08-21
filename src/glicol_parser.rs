//! Glicol-style DSP parser
//! 
//! Parses Glicol syntax: 
//! - `o: sin 440 >> mul 0.5`
//! - `~amp: sin 1.0 >> mul 0.3 >> add 0.5`
//! - Integration with mini-notation patterns

use crate::glicol_dsp::{DspChain, DspNode, DspEnvironment, LfoShape};
use crate::signal_graph::SignalGraph;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Identifiers and literals
    Symbol(String),
    Number(f64),
    String(String),
    
    // Operators
    Chain,          // >>
    Colon,          // :
    Tilde,          // ~
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    
    // Keywords (node types)
    Sin, Saw, Square, Triangle, Noise,
    Mul, Add, Sub, Div,
    Lpf, Hpf, Bpf, Notch,
    Delay, Reverb, Chorus, Phaser,
    Seq, Speed, Choose, Sp,
    Adsr, Env, Lfo,
    Mix, Pan, Gain,
    Meta,
    
    // Special
    Newline,
    Eof,
}

pub struct GlicolParser {
    input: String,
    position: usize,
    tokens: Vec<Token>,
    current: usize,
}

impl GlicolParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            position: 0,
            tokens: Vec::new(),
            current: 0,
        }
    }
    
    /// Parse the entire input into a DSP environment
    pub fn parse(&mut self) -> Result<DspEnvironment, String> {
        self.tokenize()?;
        self.parse_environment()
    }
    
    /// Tokenize the input string
    fn tokenize(&mut self) -> Result<(), String> {
        let chars: Vec<char> = self.input.chars().collect();
        
        while self.position < chars.len() {
            match chars[self.position] {
                ' ' | '\t' | '\r' => {
                    self.position += 1;
                }
                '\n' => {
                    self.tokens.push(Token::Newline);
                    self.position += 1;
                }
                ':' => {
                    self.tokens.push(Token::Colon);
                    self.position += 1;
                }
                '~' => {
                    self.tokens.push(Token::Tilde);
                    self.position += 1;
                }
                '(' => {
                    self.tokens.push(Token::LeftParen);
                    self.position += 1;
                }
                ')' => {
                    self.tokens.push(Token::RightParen);
                    self.position += 1;
                }
                '[' => {
                    self.tokens.push(Token::LeftBracket);
                    self.position += 1;
                }
                ']' => {
                    self.tokens.push(Token::RightBracket);
                    self.position += 1;
                }
                '>' if self.position + 1 < chars.len() && chars[self.position + 1] == '>' => {
                    self.tokens.push(Token::Chain);
                    self.position += 2;
                }
                '"' => {
                    // Parse string literal
                    self.position += 1;
                    let start = self.position;
                    while self.position < chars.len() && chars[self.position] != '"' {
                        self.position += 1;
                    }
                    let string_content: String = chars[start..self.position].iter().collect();
                    self.tokens.push(Token::String(string_content));
                    self.position += 1; // Skip closing quote
                }
                '/' if self.position + 1 < chars.len() && chars[self.position + 1] == '/' => {
                    // Comment - skip to end of line
                    while self.position < chars.len() && chars[self.position] != '\n' {
                        self.position += 1;
                    }
                }
                '0'..='9' | '-' | '.' => {
                    // Parse number
                    let start = self.position;
                    if chars[self.position] == '-' {
                        self.position += 1;
                    }
                    while self.position < chars.len() && 
                          (chars[self.position].is_ascii_digit() || chars[self.position] == '.') {
                        self.position += 1;
                    }
                    let num_str: String = chars[start..self.position].iter().collect();
                    if let Ok(num) = num_str.parse::<f64>() {
                        self.tokens.push(Token::Number(num));
                    } else {
                        return Err(format!("Invalid number: {}", num_str));
                    }
                }
                _ if chars[self.position].is_ascii_alphabetic() || chars[self.position] == '_' => {
                    // Parse identifier or keyword
                    let start = self.position;
                    while self.position < chars.len() && 
                          (chars[self.position].is_ascii_alphanumeric() || 
                           chars[self.position] == '_' || 
                           chars[self.position] == '-') {
                        self.position += 1;
                    }
                    let ident: String = chars[start..self.position].iter().collect();
                    
                    // Check if it's a keyword
                    let token = match ident.as_str() {
                        "sin" => Token::Sin,
                        "saw" => Token::Saw,
                        "square" | "squ" => Token::Square,
                        "triangle" | "tri" => Token::Triangle,
                        "noise" | "noiz" => Token::Noise,
                        "mul" => Token::Mul,
                        "add" => Token::Add,
                        "sub" => Token::Sub,
                        "div" => Token::Div,
                        "lpf" => Token::Lpf,
                        "hpf" | "rhpf" => Token::Hpf,
                        "bpf" => Token::Bpf,
                        "notch" => Token::Notch,
                        "delay" | "delayn" => Token::Delay,
                        "reverb" | "rev" => Token::Reverb,
                        "chorus" => Token::Chorus,
                        "phaser" => Token::Phaser,
                        "seq" => Token::Seq,
                        "speed" => Token::Speed,
                        "choose" => Token::Choose,
                        "sp" | "sampler" => Token::Sp,
                        "adsr" => Token::Adsr,
                        "env" => Token::Env,
                        "lfo" => Token::Lfo,
                        "mix" => Token::Mix,
                        "pan" => Token::Pan,
                        "gain" => Token::Gain,
                        "meta" | "script" => Token::Meta,
                        _ => Token::Symbol(ident),
                    };
                    self.tokens.push(token);
                }
                _ => {
                    return Err(format!("Unexpected character: {}", chars[self.position]));
                }
            }
        }
        
        self.tokens.push(Token::Eof);
        Ok(())
    }
    
    /// Parse the DSP environment (multiple lines)
    fn parse_environment(&mut self) -> Result<DspEnvironment, String> {
        let mut env = DspEnvironment::new();
        
        while self.current_token() != &Token::Eof {
            // Skip newlines
            if self.current_token() == &Token::Newline {
                self.advance();
                continue;
            }
            
            // Parse a line
            self.parse_line(&mut env)?;
        }
        
        Ok(env)
    }
    
    /// Parse a single line (either output or reference chain)
    fn parse_line(&mut self, env: &mut DspEnvironment) -> Result<(), String> {
        // Check if it starts with ~
        let is_ref = if self.current_token() == &Token::Tilde {
            self.advance();
            true
        } else {
            false
        };
        
        // Get the name
        let name = if let Token::Symbol(s) = self.current_token() {
            let name = s.clone();
            self.advance();
            name
        } else {
            if !is_ref && self.current_token() == &Token::Sin {
                // Allow direct chain without name for output
                let chain = self.parse_chain()?;
                env.set_output(chain);
                return Ok(());
            }
            return Err("Expected identifier".to_string());
        };
        
        // Expect colon
        if self.current_token() != &Token::Colon {
            return Err("Expected ':'".to_string());
        }
        self.advance();
        
        // Parse the chain
        let chain = self.parse_chain()?;
        
        // Add to environment
        if is_ref {
            env.add_ref(name, chain);
        } else if name == "o" || name == "out" {
            env.set_output(chain);
        } else {
            env.add_ref(name, chain);
        }
        
        // Skip optional newline
        if self.current_token() == &Token::Newline {
            self.advance();
        }
        
        Ok(())
    }
    
    /// Parse a chain of nodes connected with >>
    fn parse_chain(&mut self) -> Result<DspChain, String> {
        let mut chain = DspChain::new();
        
        // Parse first node
        let node = self.parse_node()?;
        chain.nodes.push(node);
        
        // Parse additional nodes connected with >>
        while self.current_token() == &Token::Chain {
            self.advance();
            let node = self.parse_node()?;
            chain.nodes.push(node);
        }
        
        Ok(chain)
    }
    
    /// Parse a single node
    fn parse_node(&mut self) -> Result<DspNode, String> {
        match self.current_token() {
            Token::Sin => {
                self.advance();
                let freq = self.parse_number_or_ref()?;
                Ok(DspNode::Sin { freq })
            }
            Token::Saw => {
                self.advance();
                let freq = self.parse_number_or_ref()?;
                Ok(DspNode::Saw { freq })
            }
            Token::Square => {
                self.advance();
                let freq = self.parse_number_or_ref()?;
                Ok(DspNode::Square { freq })
            }
            Token::Triangle => {
                self.advance();
                let freq = self.parse_number_or_ref()?;
                Ok(DspNode::Triangle { freq })
            }
            Token::Noise => {
                self.advance();
                self.parse_number_or_ref()?; // Noise seed/type (ignored for now)
                Ok(DspNode::Noise)
            }
            Token::Mul => {
                self.advance();
                let value = self.parse_number_or_ref()?;
                Ok(DspNode::Mul { value })
            }
            Token::Add => {
                self.advance();
                let value = self.parse_number_or_ref()?;
                Ok(DspNode::Add { value })
            }
            Token::Lpf => {
                self.advance();
                let cutoff = self.parse_number_or_ref()?;
                let q = if let Token::Number(_) = self.peek_token() {
                    self.parse_number_or_ref()?
                } else {
                    1.0
                };
                Ok(DspNode::Lpf { cutoff, q })
            }
            Token::Hpf => {
                self.advance();
                let cutoff = self.parse_number_or_ref()?;
                let q = if let Token::Number(_) = self.peek_token() {
                    self.parse_number_or_ref()?
                } else {
                    1.0
                };
                Ok(DspNode::Hpf { cutoff, q })
            }
            Token::Delay => {
                self.advance();
                let time = self.parse_number_or_ref()?;
                let feedback = if let Token::Number(_) = self.peek_token() {
                    self.parse_number_or_ref()?
                } else {
                    0.5
                };
                Ok(DspNode::Delay { time, feedback })
            }
            Token::Reverb => {
                self.advance();
                let room = self.parse_number_or_ref()?;
                let damp = if let Token::Number(_) = self.peek_token() {
                    self.parse_number_or_ref()?
                } else {
                    0.5
                };
                Ok(DspNode::Reverb { room, damp })
            }
            Token::Seq => {
                self.advance();
                let pattern = self.parse_string_or_pattern()?;
                Ok(DspNode::Seq { pattern })
            }
            Token::Speed => {
                self.advance();
                let factor = self.parse_number_or_ref()?;
                Ok(DspNode::Speed { factor })
            }
            Token::Sp => {
                self.advance();
                let sample = if let Token::Symbol(s) = self.current_token() {
                    let sample = s.clone();
                    self.advance();
                    sample
                } else if let Token::String(s) = self.current_token() {
                    let sample = s.clone();
                    self.advance();
                    sample
                } else {
                    return Err("Expected sample name".to_string());
                };
                Ok(DspNode::Sp { sample })
            }
            Token::Tilde => {
                // Reference to another chain
                self.advance();
                if let Token::Symbol(name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    Ok(DspNode::Ref { name })
                } else {
                    Err("Expected reference name after ~".to_string())
                }
            }
            _ => Err(format!("Unexpected token in node: {:?}", self.current_token()))
        }
    }
    
    /// Parse a number or reference (~name)
    fn parse_number_or_ref(&mut self) -> Result<f64, String> {
        match self.current_token() {
            Token::Number(n) => {
                let value = *n;
                self.advance();
                Ok(value)
            }
            Token::Tilde => {
                // For now, return a placeholder value
                // In full implementation, would resolve reference
                self.advance();
                if let Token::Symbol(_) = self.current_token() {
                    self.advance();
                }
                Ok(0.0)
            }
            _ => Err("Expected number or reference".to_string())
        }
    }
    
    /// Parse a string or pattern
    fn parse_string_or_pattern(&mut self) -> Result<String, String> {
        if let Token::String(s) = self.current_token() {
            let pattern = s.clone();
            self.advance();
            Ok(pattern)
        } else {
            // Parse space-separated pattern elements until >> or newline
            let mut pattern = String::new();
            while !matches!(self.current_token(), Token::Chain | Token::Newline | Token::Eof) {
                match self.current_token() {
                    Token::Symbol(s) => {
                        if !pattern.is_empty() {
                            pattern.push(' ');
                        }
                        pattern.push_str(s);
                        self.advance();
                    }
                    Token::Number(n) => {
                        if !pattern.is_empty() {
                            pattern.push(' ');
                        }
                        pattern.push_str(&n.to_string());
                        self.advance();
                    }
                    _ => break,
                }
            }
            Ok(pattern)
        }
    }
    
    fn current_token(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::Eof)
    }
    
    fn peek_token(&self) -> &Token {
        self.tokens.get(self.current + 1).unwrap_or(&Token::Eof)
    }
    
    fn advance(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }
}

/// Parse Glicol-style DSP code
pub fn parse_glicol(input: &str) -> Result<DspEnvironment, String> {
    let mut parser = GlicolParser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_chain() {
        let input = "o: sin 440 >> mul 0.5";
        let env = parse_glicol(input).unwrap();
        assert!(env.output_chain.is_some());
        let chain = env.output_chain.unwrap();
        assert_eq!(chain.nodes.len(), 2);
    }
    
    #[test]
    fn test_reference_chain() {
        let input = r#"
            ~amp: sin 1.0 >> mul 0.3 >> add 0.5
            o: sin 440 >> mul ~amp
        "#;
        let env = parse_glicol(input).unwrap();
        assert!(env.output_chain.is_some());
        assert_eq!(env.ref_chains.len(), 1);
        assert!(env.ref_chains.contains_key("amp"));
    }
    
    #[test]
    fn test_pattern_integration() {
        let input = r#"o: seq "bd sn bd sn" >> sp"#;
        let env = parse_glicol(input).unwrap();
        assert!(env.output_chain.is_some());
    }
    
    #[test]
    fn test_complex_chain() {
        let input = r#"
            ~lfo: sin 0.5 >> mul 0.5 >> add 0.5
            ~bass: saw 55 >> lpf ~lfo*2000+500 0.8
            o: ~bass >> reverb 0.8 0.5 >> mul 0.4
        "#;
        let env = parse_glicol(input).unwrap();
        assert!(env.output_chain.is_some());
        assert_eq!(env.ref_chains.len(), 2);
    }
}