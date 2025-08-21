//! Improved mini-notation parser with proper recursive structure
//! 
//! This parser handles all nesting properly by having a single
//! parse_element function that all other parsers call recursively.

use crate::pattern::{Pattern, Fraction};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Symbol(String),
    Number(i32),
    Rest,
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenAngle,
    CloseAngle,
    Comma,
    Star,
    Slash,
    Plus,
    Minus,
    Percent,
    Colon,
    At,
    Exclamation,
    Question,
    Tilde,
    Pipe,
    Dot,
}

pub struct MiniNotationParser {
    tokens: Vec<Token>,
    position: usize,
}

impl MiniNotationParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }
    
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
    
    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += 1;
        token
    }
    
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1)
    }
    
    /// Main entry point
    pub fn parse(&mut self) -> Pattern<String> {
        self.parse_sequence()
    }
    
    /// Parse a sequence - the top level pattern
    fn parse_sequence(&mut self) -> Pattern<String> {
        let mut patterns = Vec::new();
        
        while self.current().is_some() {
            if let Some(pattern) = self.parse_element() {
                patterns.push(pattern);
            } else {
                self.advance(); // Skip unknown tokens
            }
        }
        
        if patterns.is_empty() {
            Pattern::silence()
        } else if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            self.fast_cat(patterns)
        }
    }
    
    /// Parse a single element - this is the key recursive function
    fn parse_element(&mut self) -> Option<Pattern<String>> {
        match self.current()? {
            Token::OpenAngle => {
                self.advance();
                Some(self.parse_alternation())
            },
            Token::OpenParen => {
                self.advance();
                Some(self.parse_polyrhythm())
            },
            Token::OpenBracket => {
                self.advance();
                Some(self.parse_group())
            },
            Token::OpenBrace => {
                self.advance();
                Some(self.parse_choice())
            },
            Token::Symbol(s) => {
                let s = s.clone();
                self.advance();
                
                // Check for function call (euclidean pattern)
                if let Some(Token::OpenParen) = self.current() {
                    Some(self.parse_euclidean_call(s))
                } else {
                    // Check for operators
                    let base = Pattern::pure(s);
                    if let Some(op_pattern) = self.parse_operators(base.clone()) {
                        Some(op_pattern)
                    } else {
                        Some(base)
                    }
                }
            },
            Token::Number(n) => {
                let n = *n;
                self.advance();
                
                let base = Pattern::pure(n.to_string());
                if let Some(op_pattern) = self.parse_operators(base.clone()) {
                    Some(op_pattern)
                } else {
                    Some(base)
                }
            },
            Token::Rest => {
                self.advance();
                Some(Pattern::silence())
            },
            _ => None
        }
    }
    
    /// Parse euclidean function call like bd(3,8,1)
    fn parse_euclidean_call(&mut self, sample: String) -> Pattern<String> {
        self.advance(); // consume OpenParen
        
        let mut pulses = 0;
        let mut steps = 0;
        let mut rotation = 0;
        
        // Parse pulses
        if let Some(Token::Number(n)) = self.current() {
            pulses = *n as usize;
            self.advance();
            
            // Check for comma and steps
            if let Some(Token::Comma) = self.current() {
                self.advance();
                
                if let Some(Token::Number(n)) = self.current() {
                    steps = *n as usize;
                    self.advance();
                    
                    // Check for rotation parameter
                    if let Some(Token::Comma) = self.current() {
                        self.advance();
                        
                        let mut is_negative = false;
                        if let Some(Token::Minus) = self.current() {
                            is_negative = true;
                            self.advance();
                        }
                        
                        if let Some(Token::Number(n)) = self.current() {
                            rotation = if is_negative {
                                -(*n as i32)
                            } else {
                                *n as i32
                            };
                            self.advance();
                        }
                    }
                }
            } else {
                // Default steps based on pulses
                steps = if pulses <= 8 { 8 } else if pulses <= 16 { 16 } else { 32 };
            }
        }
        
        // Consume closing paren
        if let Some(Token::CloseParen) = self.current() {
            self.advance();
        }
        
        // Create euclidean pattern
        self.euclidean_pattern_with_rotation(Pattern::pure(sample), pulses, steps, rotation)
    }
    
    /// Parse alternation <a b c> - cycles through elements
    fn parse_alternation(&mut self) -> Pattern<String> {
        let mut elements = Vec::new();
        
        while let Some(token) = self.current() {
            if let Token::CloseAngle = token {
                self.advance();
                break;
            }
            
            // Parse each element recursively
            if let Some(element) = self.parse_element() {
                elements.push(element);
            } else {
                self.advance(); // Skip unknown
            }
        }
        
        Pattern::slowcat(elements)
    }
    
    /// Parse choice {a,b,c} - randomly picks one
    fn parse_choice(&mut self) -> Pattern<String> {
        let mut elements = Vec::new();
        let mut current_group = Vec::new();
        
        while let Some(token) = self.current() {
            match token {
                Token::CloseBrace => {
                    self.advance();
                    // Add final group
                    if !current_group.is_empty() {
                        elements.push(self.combine_group(current_group));
                        current_group = Vec::new();
                    }
                    break;
                },
                Token::Comma => {
                    self.advance();
                    // Comma separates choices
                    if !current_group.is_empty() {
                        elements.push(self.combine_group(current_group));
                        current_group = Vec::new();
                    }
                },
                _ => {
                    // Parse element recursively
                    if let Some(element) = self.parse_element() {
                        current_group.push(element);
                    } else {
                        self.advance(); // Skip unknown
                    }
                }
            }
        }
        
        // Add any remaining group
        if !current_group.is_empty() {
            elements.push(self.combine_group(current_group));
        }
        
        // For now use slowcat - should be random choice
        Pattern::slowcat(elements)
    }
    
    /// Parse polyrhythm (a,b,c) - plays simultaneously
    fn parse_polyrhythm(&mut self) -> Pattern<String> {
        let mut elements = Vec::new();
        let mut current_group = Vec::new();
        
        while let Some(token) = self.current() {
            match token {
                Token::CloseParen => {
                    self.advance();
                    // Add final group
                    if !current_group.is_empty() {
                        elements.push(self.combine_group(current_group));
                        current_group = Vec::new();
                    }
                    break;
                },
                Token::Comma => {
                    self.advance();
                    // Comma separates polyrhythm elements
                    if !current_group.is_empty() {
                        elements.push(self.combine_group(current_group));
                        current_group = Vec::new();
                    }
                },
                _ => {
                    // Parse element recursively
                    if let Some(element) = self.parse_element() {
                        current_group.push(element);
                    } else {
                        self.advance(); // Skip unknown
                    }
                }
            }
        }
        
        // Add any remaining group
        if !current_group.is_empty() {
            elements.push(self.combine_group(current_group));
        }
        
        Pattern::stack(elements)
    }
    
    /// Parse group [a b c] - plays in sequence within one cycle unit
    fn parse_group(&mut self) -> Pattern<String> {
        let mut elements = Vec::new();
        
        while let Some(token) = self.current() {
            if let Token::CloseBracket = token {
                self.advance();
                break;
            }
            
            // Parse each element recursively
            if let Some(element) = self.parse_element() {
                elements.push(element);
            } else {
                self.advance(); // Skip unknown
            }
        }
        
        self.fast_cat(elements)
    }
    
    /// Combine a group of patterns into one
    fn combine_group(&mut self, patterns: Vec<Pattern<String>>) -> Pattern<String> {
        if patterns.is_empty() {
            Pattern::silence()
        } else if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            self.fast_cat(patterns)
        }
    }
    
    /// Parse operators like *, /, etc.
    fn parse_operators(&mut self, pattern: Pattern<String>) -> Option<Pattern<String>> {
        match self.current() {
            Some(Token::Star) => {
                self.advance();
                if let Some(Token::Number(n)) = self.current() {
                    let n = *n;
                    self.advance();
                    Some(pattern.fast(n as f64))
                } else {
                    Some(pattern)
                }
            },
            Some(Token::Slash) => {
                self.advance();
                if let Some(Token::Number(n)) = self.current() {
                    let n = *n;
                    self.advance();
                    Some(pattern.slow(n as f64))
                } else {
                    Some(pattern)
                }
            },
            Some(Token::Exclamation) => {
                self.advance();
                Some(pattern.replicate(1))  // Replicate once
            },
            Some(Token::At) => {
                self.advance();
                if let Some(Token::Number(n)) = self.current() {
                    let n = *n;
                    self.advance();
                    Some(pattern.degrade_by(n as f64 / 100.0))
                } else {
                    Some(pattern)
                }
            },
            Some(Token::Question) => {
                self.advance();
                Some(pattern.degrade_by(0.5))
            },
            _ => None
        }
    }
    
    /// Helper: fast concatenation
    fn fast_cat(&self, patterns: Vec<Pattern<String>>) -> Pattern<String> {
        if patterns.is_empty() {
            Pattern::silence()
        } else {
            Pattern::fastcat(patterns)
        }
    }
    
    /// Create euclidean pattern with rotation
    fn euclidean_pattern_with_rotation(
        &self,
        sample_pattern: Pattern<String>,
        pulses: usize,
        steps: usize,
        rotation: i32,
    ) -> Pattern<String> {
        use crate::pattern::bjorklund;
        
        if pulses == 0 || steps == 0 {
            return Pattern::silence();
        }
        
        let mut rhythm = bjorklund(pulses, steps);
        
        // Apply rotation
        if rotation != 0 {
            let rot = rotation.rem_euclid(steps as i32) as usize;
            rhythm.rotate_left(rot);
        }
        
        // Convert boolean rhythm to pattern
        let patterns: Vec<Pattern<String>> = rhythm
            .iter()
            .map(|&hit| {
                if hit {
                    sample_pattern.clone()
                } else {
                    Pattern::silence()
                }
            })
            .collect();
        
        Pattern::fastcat(patterns)
    }
}

/// Parse mini notation string into a pattern
pub fn parse_mini_notation(s: &str) -> Pattern<String> {
    let tokens = tokenize(s);
    let mut parser = MiniNotationParser::new(tokens);
    parser.parse()
}

/// Tokenize input string
fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\n' => continue,  // Skip whitespace
            '[' => tokens.push(Token::OpenBracket),
            ']' => tokens.push(Token::CloseBracket),
            '(' => tokens.push(Token::OpenParen),
            ')' => tokens.push(Token::CloseParen),
            '{' => tokens.push(Token::OpenBrace),
            '}' => tokens.push(Token::CloseBrace),
            '<' => tokens.push(Token::OpenAngle),
            '>' => tokens.push(Token::CloseAngle),
            ',' => tokens.push(Token::Comma),
            '*' => tokens.push(Token::Star),
            '/' => tokens.push(Token::Slash),
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '%' => tokens.push(Token::Percent),
            ':' => tokens.push(Token::Colon),
            '@' => tokens.push(Token::At),
            '!' => tokens.push(Token::Exclamation),
            '?' => tokens.push(Token::Question),
            '~' => tokens.push(Token::Tilde),
            '|' => tokens.push(Token::Pipe),
            '.' => tokens.push(Token::Dot),
            '0'..='9' => {
                let mut num = String::new();
                num.push(ch);
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_digit() {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num.parse::<i32>() {
                    tokens.push(Token::Number(n));
                }
            },
            _ if ch.is_alphabetic() || ch == '_' => {
                let mut sym = String::new();
                sym.push(ch);
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        sym.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if sym == "_" {
                    tokens.push(Token::Rest);
                } else {
                    tokens.push(Token::Symbol(sym));
                }
            },
            _ => {} // Skip unknown characters
        }
    }
    
    tokens
}