//! Enhanced Glicol parser with pattern parameter support
//!
//! This parser allows patterns as parameters: `lpf "1000 2000 500" 0.8`

use crate::dsp_parameter::DspParameter;
use crate::glicol_dsp_v2::{DspChain, DspEnvironment, DspNode};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Identifiers and literals
    Symbol(String),
    Number(f64),
    String(String),

    // Operators
    Chain, // >>
    Colon, // :
    Tilde, // ~

    // Delimiters
    LeftParen,
    RightParen,

    // Keywords (node types)
    Sin,
    Saw,
    Square,
    Triangle,
    Noise,
    Impulse,
    Pink,
    Brown,
    Mul,
    Add,
    Sub,
    Div,
    Lpf,
    Hpf,
    Bpf,
    Notch,
    Delay,
    Reverb,
    Chorus,
    Phaser,
    Seq,
    Speed,
    S,
    Adsr,
    Env,
    Lfo,
    Mix,
    Pan,
    Gain,
    Clip,
    Distortion,
    Compressor,

    // Arithmetic operators for signal math
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /

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
                '+' => {
                    self.tokens.push(Token::Plus);
                    self.position += 1;
                }
                '-' if self.position + 1 < chars.len() && chars[self.position + 1] != '>' => {
                    self.tokens.push(Token::Minus);
                    self.position += 1;
                }
                '*' => {
                    self.tokens.push(Token::Star);
                    self.position += 1;
                }
                '/' if self.position + 1 < chars.len() && chars[self.position + 1] != '/' => {
                    self.tokens.push(Token::Slash);
                    self.position += 1;
                }
                '>' if self.position > 0 && chars[self.position - 1] == '>' => {
                    // Already handled as >>
                    self.position += 1;
                }
                '>' if self.position + 1 < chars.len() && chars[self.position + 1] == '>' => {
                    self.tokens.push(Token::Chain);
                    self.position += 2;
                }
                '"' => {
                    // Parse string literal (pattern)
                    self.position += 1;
                    let start = self.position;
                    while self.position < chars.len() && chars[self.position] != '"' {
                        self.position += 1;
                    }
                    if self.position >= chars.len() {
                        return Err("Unterminated string".to_string());
                    }
                    let s = chars[start..self.position].iter().collect();
                    self.tokens.push(Token::String(s));
                    self.position += 1;
                }
                '0'..='9' | '.' => {
                    // Parse number
                    let start = self.position;
                    while self.position < chars.len()
                        && (chars[self.position].is_numeric() || chars[self.position] == '.')
                    {
                        self.position += 1;
                    }
                    let num_str: String = chars[start..self.position].iter().collect();
                    match num_str.parse::<f64>() {
                        Ok(n) => self.tokens.push(Token::Number(n)),
                        Err(_) => return Err(format!("Invalid number: {num_str}")),
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    // Parse identifier/keyword
                    let start = self.position;
                    while self.position < chars.len()
                        && (chars[self.position].is_alphanumeric() || chars[self.position] == '_')
                    {
                        self.position += 1;
                    }
                    let ident: String = chars[start..self.position].iter().collect();

                    // Check for keywords
                    let token = match ident.as_str() {
                        "sin" => Token::Sin,
                        "saw" => Token::Saw,
                        "square" => Token::Square,
                        "triangle" => Token::Triangle,
                        "noise" => Token::Noise,
                        "impulse" => Token::Impulse,
                        "pink" => Token::Pink,
                        "brown" => Token::Brown,
                        "mul" => Token::Mul,
                        "add" => Token::Add,
                        "sub" => Token::Sub,
                        "div" => Token::Div,
                        "lpf" => Token::Lpf,
                        "hpf" => Token::Hpf,
                        "bpf" => Token::Bpf,
                        "notch" => Token::Notch,
                        "delay" => Token::Delay,
                        "reverb" => Token::Reverb,
                        "chorus" => Token::Chorus,
                        "phaser" => Token::Phaser,
                        "seq" => Token::Seq,
                        "speed" => Token::Speed,
                        "s" => Token::S,
                        "adsr" => Token::Adsr,
                        "env" => Token::Env,
                        "lfo" => Token::Lfo,
                        "mix" => Token::Mix,
                        "pan" => Token::Pan,
                        "gain" => Token::Gain,
                        "clip" => Token::Clip,
                        "distortion" => Token::Distortion,
                        "compressor" => Token::Compressor,
                        "o" => Token::Symbol("o".to_string()),
                        _ => Token::Symbol(ident),
                    };
                    self.tokens.push(token);
                }
                '/' if self.position + 1 < chars.len() && chars[self.position + 1] == '/' => {
                    // Skip comment
                    while self.position < chars.len() && chars[self.position] != '\n' {
                        self.position += 1;
                    }
                }
                _ => {
                    self.position += 1; // Skip unknown characters
                }
            }
        }

        self.tokens.push(Token::Eof);
        Ok(())
    }

    fn current_token(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current_token(), Token::Newline | Token::Eof)
            && self.current < self.tokens.len() - 1
        {
            if self.current_token() == &Token::Eof {
                break;
            }
            self.advance();
        }
    }

    /// Parse the environment (all chains and output)
    fn parse_environment(&mut self) -> Result<DspEnvironment, String> {
        let mut env = DspEnvironment {
            chains: HashMap::new(),
            output: None,
        };

        self.skip_newlines();

        while self.current_token() != &Token::Eof {
            self.skip_newlines();

            // Parse chain definition
            match self.current_token() {
                Token::Tilde => {
                    // Named chain: ~name: chain
                    self.advance();
                    let name = self.parse_identifier()?;
                    if self.current_token() != &Token::Colon {
                        return Err("Expected ':' after chain name".to_string());
                    }
                    self.advance();
                    let chain = self.parse_chain()?;
                    env.chains.insert(name, chain);
                }
                Token::Symbol(s) if s == "o" => {
                    // Output chain: o: chain
                    self.advance();
                    if self.current_token() != &Token::Colon {
                        return Err("Expected ':' after 'o'".to_string());
                    }
                    self.advance();
                    env.output = Some(self.parse_chain()?);
                }
                _ => {
                    return Err(format!("Unexpected token: {:?}", self.current_token()));
                }
            }

            self.skip_newlines();
        }

        Ok(env)
    }

    fn parse_identifier(&mut self) -> Result<String, String> {
        match self.current_token() {
            Token::Symbol(s) => {
                let name = s.clone();
                self.advance();
                Ok(name)
            }
            // Allow keywords to be used as identifiers after tilde
            Token::Sin => {
                self.advance();
                Ok("sin".to_string())
            }
            Token::Saw => {
                self.advance();
                Ok("saw".to_string())
            }
            Token::Square => {
                self.advance();
                Ok("square".to_string())
            }
            Token::Triangle => {
                self.advance();
                Ok("triangle".to_string())
            }
            Token::Noise => {
                self.advance();
                Ok("noise".to_string())
            }
            Token::Lpf => {
                self.advance();
                Ok("lpf".to_string())
            }
            Token::Hpf => {
                self.advance();
                Ok("hpf".to_string())
            }
            Token::Mul => {
                self.advance();
                Ok("mul".to_string())
            }
            Token::Add => {
                self.advance();
                Ok("add".to_string())
            }
            Token::Sub => {
                self.advance();
                Ok("sub".to_string())
            }
            Token::Div => {
                self.advance();
                Ok("div".to_string())
            }
            Token::Delay => {
                self.advance();
                Ok("delay".to_string())
            }
            Token::Reverb => {
                self.advance();
                Ok("reverb".to_string())
            }
            Token::Lfo => {
                self.advance();
                Ok("lfo".to_string())
            }
            Token::Mix => {
                self.advance();
                Ok("mix".to_string())
            }
            Token::Pan => {
                self.advance();
                Ok("pan".to_string())
            }
            Token::Gain => {
                self.advance();
                Ok("gain".to_string())
            }
            Token::S => {
                self.advance();
                Ok("s".to_string())
            }
            _ => Err("Expected identifier".to_string()),
        }
    }

    /// Parse a DSP chain (nodes connected with >>)
    fn parse_chain(&mut self) -> Result<DspChain, String> {
        let mut chain = DspChain::new();

        // Parse first element (could be arithmetic expression)
        let first_element = self.parse_expression()?;
        chain = chain >> first_element;

        // Parse chained nodes
        while self.current_token() == &Token::Chain {
            self.advance();
            let node = self.parse_node()?;
            chain = chain >> node;
        }

        Ok(chain)
    }

    /// Parse arithmetic expression (handles +, -, *, / on signals)
    fn parse_expression(&mut self) -> Result<DspNode, String> {
        let left = self.parse_term()?;

        match self.current_token() {
            Token::Plus => {
                self.advance();
                let right = self.parse_expression()?;
                Ok(DspNode::SignalAdd {
                    left: Box::new(DspChain::from_node(left)),
                    right: Box::new(DspChain::from_node(right)),
                })
            }
            Token::Minus => {
                self.advance();
                let right = self.parse_expression()?;
                Ok(DspNode::SignalSub {
                    left: Box::new(DspChain::from_node(left)),
                    right: Box::new(DspChain::from_node(right)),
                })
            }
            _ => Ok(left),
        }
    }

    /// Parse multiplication/division term
    fn parse_term(&mut self) -> Result<DspNode, String> {
        let left = self.parse_primary()?;

        match self.current_token() {
            Token::Star => {
                self.advance();
                let right = self.parse_term()?;
                Ok(DspNode::SignalMul {
                    left: Box::new(DspChain::from_node(left)),
                    right: Box::new(DspChain::from_node(right)),
                })
            }
            Token::Slash => {
                self.advance();
                let right = self.parse_term()?;
                Ok(DspNode::SignalDiv {
                    left: Box::new(DspChain::from_node(left)),
                    right: Box::new(DspChain::from_node(right)),
                })
            }
            _ => Ok(left),
        }
    }

    /// Parse primary element (node or parenthesized expression)
    fn parse_primary(&mut self) -> Result<DspNode, String> {
        match self.current_token() {
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                if self.current_token() != &Token::RightParen {
                    return Err("Expected ')'".to_string());
                }
                self.advance();
                Ok(expr)
            }
            _ => self.parse_node(),
        }
    }

    /// Parse a single DSP node
    fn parse_node(&mut self) -> Result<DspNode, String> {
        match self.current_token() {
            Token::Sin => {
                self.advance();
                let freq = self.parse_parameter()?;
                Ok(DspNode::Sin { freq })
            }
            Token::Saw => {
                self.advance();
                let freq = self.parse_parameter()?;
                Ok(DspNode::Saw { freq })
            }
            Token::Square => {
                self.advance();
                let freq = self.parse_parameter()?;
                let duty = self.parse_parameter_or_default(0.5)?;
                Ok(DspNode::Square { freq, duty })
            }
            Token::Triangle => {
                self.advance();
                let freq = self.parse_parameter()?;
                Ok(DspNode::Triangle { freq })
            }
            Token::Noise => {
                self.advance();
                Ok(DspNode::Noise { seed: 42 })
            }
            Token::Lpf => {
                self.advance();
                let cutoff = self.parse_parameter()?;
                let q = self.parse_parameter_or_default(0.7)?;
                Ok(DspNode::Lpf { cutoff, q })
            }
            Token::Hpf => {
                self.advance();
                let cutoff = self.parse_parameter()?;
                let q = self.parse_parameter_or_default(0.7)?;
                Ok(DspNode::Hpf { cutoff, q })
            }
            Token::Mul => {
                self.advance();
                let factor = self.parse_parameter()?;
                Ok(DspNode::Mul { factor })
            }
            Token::Add => {
                self.advance();
                let value = self.parse_parameter()?;
                Ok(DspNode::Add { value })
            }
            Token::Delay => {
                self.advance();
                let time = self.parse_parameter()?;
                let feedback = self.parse_parameter_or_default(0.5)?;
                let mix = self.parse_parameter_or_default(0.5)?;
                Ok(DspNode::Delay {
                    time,
                    feedback,
                    mix,
                })
            }
            Token::S => {
                self.advance();
                let pattern = self.parse_string()?;
                Ok(DspNode::S { pattern })
            }
            Token::Tilde => {
                self.advance();
                let name = self.parse_identifier()?;
                Ok(DspNode::Ref { name })
            }
            _ => Err(format!(
                "Unexpected token in node: {:?}",
                self.current_token()
            )),
        }
    }

    /// Parse a parameter (can be number, string pattern, or reference)
    fn parse_parameter(&mut self) -> Result<DspParameter, String> {
        match self.current_token() {
            Token::Number(n) => {
                let value = *n;
                self.advance();
                Ok(DspParameter::constant(value as f32))
            }
            Token::String(s) => {
                let pattern = s.clone();
                self.advance();
                Ok(DspParameter::pattern(&pattern))
            }
            Token::Tilde => {
                self.advance();
                let name = self.parse_identifier()?;
                Ok(DspParameter::reference(&name))
            }
            Token::LeftParen => {
                // Parse parenthesized expression
                self.advance();
                let param = self.parse_parameter_expression()?;
                if self.current_token() != &Token::RightParen {
                    return Err("Expected ')'".to_string());
                }
                self.advance();
                Ok(param)
            }
            _ => Err(format!(
                "Expected parameter, got {:?}",
                self.current_token()
            )),
        }
    }

    /// Parse a parameter with a default value
    fn parse_parameter_or_default(&mut self, default: f32) -> Result<DspParameter, String> {
        // Check if there's a parameter available
        match self.current_token() {
            Token::Number(_) | Token::String(_) | Token::Tilde | Token::LeftParen => {
                self.parse_parameter()
            }
            _ => Ok(DspParameter::constant(default)),
        }
    }

    /// Parse parameter expression (arithmetic on parameters)
    fn parse_parameter_expression(&mut self) -> Result<DspParameter, String> {
        let left = self.parse_parameter_term()?;

        // Check for addition/subtraction
        match self.current_token() {
            Token::Plus => {
                self.advance();
                let right = self.parse_parameter_expression()?;
                Ok(DspParameter::Expression(Box::new(
                    crate::dsp_parameter::ParameterExpression::Binary {
                        op: crate::dsp_parameter::BinaryOp::Add,
                        left,
                        right,
                    },
                )))
            }
            Token::Minus => {
                self.advance();
                let right = self.parse_parameter_expression()?;
                Ok(DspParameter::Expression(Box::new(
                    crate::dsp_parameter::ParameterExpression::Binary {
                        op: crate::dsp_parameter::BinaryOp::Subtract,
                        left,
                        right,
                    },
                )))
            }
            _ => Ok(left),
        }
    }

    /// Parse parameter term (multiplication/division)
    fn parse_parameter_term(&mut self) -> Result<DspParameter, String> {
        let left = self.parse_parameter_primary()?;

        // Check for multiplication/division
        match self.current_token() {
            Token::Star => {
                self.advance();
                let right = self.parse_parameter_term()?;
                Ok(DspParameter::Expression(Box::new(
                    crate::dsp_parameter::ParameterExpression::Binary {
                        op: crate::dsp_parameter::BinaryOp::Multiply,
                        left,
                        right,
                    },
                )))
            }
            Token::Slash => {
                self.advance();
                let right = self.parse_parameter_term()?;
                Ok(DspParameter::Expression(Box::new(
                    crate::dsp_parameter::ParameterExpression::Binary {
                        op: crate::dsp_parameter::BinaryOp::Divide,
                        left,
                        right,
                    },
                )))
            }
            _ => Ok(left),
        }
    }

    /// Parse primary parameter (number, string, reference, or parenthesized expression)
    fn parse_parameter_primary(&mut self) -> Result<DspParameter, String> {
        match self.current_token() {
            Token::Number(n) => {
                let value = *n;
                self.advance();
                Ok(DspParameter::constant(value as f32))
            }
            Token::String(s) => {
                let pattern = s.clone();
                self.advance();
                Ok(DspParameter::pattern(&pattern))
            }
            Token::Tilde => {
                self.advance();
                let name = self.parse_identifier()?;
                Ok(DspParameter::reference(&name))
            }
            Token::LeftParen => {
                self.advance();
                let param = self.parse_parameter_expression()?;
                if self.current_token() != &Token::RightParen {
                    return Err("Expected ')' in parameter expression".to_string());
                }
                self.advance();
                Ok(param)
            }
            Token::Minus => {
                // Handle unary minus
                self.advance();
                let param = self.parse_parameter_primary()?;
                Ok(DspParameter::Expression(Box::new(
                    crate::dsp_parameter::ParameterExpression::Unary {
                        op: crate::dsp_parameter::UnaryOp::Negate,
                        param,
                    },
                )))
            }
            _ => Err(format!(
                "Expected parameter, got {:?}",
                self.current_token()
            )),
        }
    }

    /// Parse a string literal
    fn parse_string(&mut self) -> Result<String, String> {
        match self.current_token() {
            Token::String(s) => {
                let string = s.clone();
                self.advance();
                Ok(string)
            }
            _ => Err("Expected string".to_string()),
        }
    }
}

/// Convenient function to parse Glicol code
pub fn parse_glicol_v2(input: &str) -> Result<DspEnvironment, String> {
    let mut parser = GlicolParser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_parameters() {
        let code = r#"
            ~source: saw "55 110 220"
            o: ~source >> lpf "1000 2000 500 3000" "0.5 0.8"
        "#;

        let result = parse_glicol_v2(code);
        assert!(result.is_ok());

        let env = result.unwrap();
        assert!(env.chains.contains_key("source"));
        assert!(env.output.is_some());
    }

    #[test]
    fn test_reference_parameters() {
        let code = r#"
            ~lfo: sin 2
            ~source: saw 110
            o: ~source >> lpf ~lfo 0.8
        "#;

        let result = parse_glicol_v2(code);
        if let Err(e) = &result {
            println!("Parse error: {}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_parameters() {
        let code = r#"
            ~bass: saw "55 110"
            ~cutoff: sin 0.5 >> mul 1000 >> add 1500
            o: ~bass >> lpf ~cutoff "0.5 0.8 0.3"
        "#;

        let result = parse_glicol_v2(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_signal_arithmetic() {
        let code = r#"
            ~carrier: sin 440
            ~modulator: sin "5 10 2"
            o: ~carrier * ~modulator
        "#;

        let result = parse_glicol_v2(code);
        assert!(result.is_ok());
    }
}
