//! Mini-notation parser v3 - Everything is a pattern
//! 
//! This parser follows Strudel's architecture where all values are patterns
//! that can be composed and evaluated per cycle.

use crate::pattern::{Pattern, Fraction, TimeSpan, Hap};
use crate::pattern_ops::*;  // Import pattern operators
use std::sync::Arc;

/// Token types in mini-notation
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Symbol(String),      // bd, sn, etc.
    Number(f64),         // 1, 2.5, etc.
    Rest,                // ~
    OpenBracket,         // [
    CloseBracket,        // ]
    OpenAngle,           // <
    CloseAngle,          // >
    OpenParen,           // (
    CloseParen,          // )
    Comma,               // ,
    Star,                // *
    Slash,               // /
    Colon,               // :
    At,                  // @
    Percent,             // %
    Question,            // ?
    Exclamation,         // !
    Dot,                 // .
    Pipe,                // |
    Quote,               // ' for chords
}

/// Pattern value that can be either a string or number
#[derive(Debug, Clone, PartialEq)]
pub enum PatternValue {
    String(String),
    Number(f64),
}

impl PatternValue {
    pub fn as_string(&self) -> String {
        match self {
            PatternValue::String(s) => s.clone(),
            PatternValue::Number(n) => n.to_string(),
        }
    }
    
    pub fn as_number(&self) -> Option<f64> {
        match self {
            PatternValue::Number(n) => Some(*n),
            PatternValue::String(s) => s.parse().ok(),
        }
    }
}

/// AST node types
#[derive(Debug, Clone)]
enum AstNode {
    /// A literal value (becomes Pattern::pure)
    Atom(PatternValue),
    
    /// A pattern with alignment (stack, sequence, etc.)
    Pattern {
        children: Vec<AstNode>,
        alignment: Alignment,
    },
    
    /// An operator applied to a pattern
    Operator {
        pattern: Box<AstNode>,
        op: Operator,
    },
    
    /// Euclidean rhythm with pattern arguments
    Euclid {
        sample: String,
        pulses: Box<AstNode>,
        steps: Box<AstNode>,
        rotation: Option<Box<AstNode>>,
    },
    
    /// Rest/silence
    Rest,
}

#[derive(Debug, Clone)]
enum Alignment {
    Sequence,       // Default horizontal alignment
    Stack,          // Vertical alignment (polyrhythm with ,)
    Choose,         // Random choice (with |)
    Alternate,      // Alternation with < >
    FastSequence,   // Fast sequence in [ ]
}

#[derive(Debug, Clone)]
enum Operator {
    Fast(Box<AstNode>),
    Slow(Box<AstNode>),
    Replicate(usize),
    ReplicatePattern(Box<AstNode>),  // For dynamic replication with patterns
    Degrade(f64),
    Late(f64),
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
            if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '#' {
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
        
        // Handle negative numbers
        if self.peek() == Some('-') {
            num_str.push('-');
            self.advance();
        }
        
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
                        // Check if this is a channel reference like ~bass or just a rest ~
                        if let Some(next_ch) = self.peek() {
                            if next_ch.is_alphabetic() || next_ch == '_' {
                                // It's a channel reference
                                let name = self.read_symbol();
                                Token::Symbol(format!("~{}", name))
                            } else {
                                // It's a rest
                                Token::Rest
                            }
                        } else {
                            Token::Rest
                        }
                    },
                    '[' => {
                        self.advance();
                        Token::OpenBracket
                    },
                    ']' => {
                        self.advance();
                        Token::CloseBracket
                    },
                    '<' => {
                        self.advance();
                        Token::OpenAngle
                    },
                    '>' => {
                        self.advance();
                        Token::CloseAngle
                    },
                    '(' => {
                        self.advance();
                        Token::OpenParen
                    },
                    ')' => {
                        self.advance();
                        Token::CloseParen
                    },
                    ',' => {
                        self.advance();
                        Token::Comma
                    },
                    '*' => {
                        self.advance();
                        Token::Star
                    },
                    '/' => {
                        self.advance();
                        Token::Slash
                    },
                    ':' => {
                        self.advance();
                        Token::Colon
                    },
                    '@' => {
                        self.advance();
                        Token::At
                    },
                    '%' => {
                        self.advance();
                        Token::Percent
                    },
                    '?' => {
                        self.advance();
                        Token::Question
                    },
                    '!' => {
                        self.advance();
                        Token::Exclamation
                    },
                    '.' => {
                        self.advance();
                        Token::Dot
                    },
                    '|' => {
                        self.advance();
                        Token::Pipe
                    },
                    '\'' => {
                        self.advance();
                        Token::Quote
                    },
                    '-' | '0'..='9' => {
                        if let Some(num) = self.read_number() {
                            Token::Number(num)
                        } else {
                            self.advance();
                            continue;
                        }
                    },
                    _ if ch.is_alphabetic() => {
                        let symbol = self.read_symbol();
                        Token::Symbol(symbol)
                    },
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
    pub fn parse(&mut self) -> AstNode {
        // Parse first part
        let first = self.parse_sequence();

        // Check if there's a pipe for stacking
        if let Some(Token::Pipe) = self.current() {
            let mut patterns = vec![first];

            while let Some(Token::Pipe) = self.current() {
                self.advance(); // consume pipe
                patterns.push(self.parse_sequence());
            }

            // Create a stacked pattern
            AstNode::Pattern {
                children: patterns,
                alignment: Alignment::Stack,
            }
        } else {
            first
        }
    }
    
    /// Parse a sequence (default alignment)
    fn parse_sequence(&mut self) -> AstNode {
        let mut children = Vec::new();
        
        while let Some(token) = self.current() {
            // Check for end of sequence markers
            match token {
                Token::CloseBracket | Token::CloseAngle | Token::CloseParen => break,
                Token::Comma | Token::Pipe => break,
                _ => {}
            }
            
            if let Some(child) = self.parse_element() {
                children.push(child);
            }
        }
        
        if children.is_empty() {
            AstNode::Rest
        } else if children.len() == 1 {
            children.into_iter().next().unwrap()
        } else {
            AstNode::Pattern {
                children,
                alignment: Alignment::Sequence,
            }
        }
    }
    
    /// Parse a single element (atom, group, alternation, etc.)
    fn parse_element(&mut self) -> Option<AstNode> {
        let node = match self.current()? {
            Token::Symbol(s) => {
                let s = s.clone();
                self.advance();
                
                // Check for function syntax (could be Euclidean rhythm or other function)
                if let Some(Token::OpenParen) = self.current() {
                    // Look ahead to determine if this is a Euclidean rhythm or just a function call
                    // For now, we'll try to parse as Euclidean and fall back to string representation
                    let start_pos = self.position;
                    self.advance(); // consume (
                    
                    // Try to parse as Euclidean rhythm
                    let first_arg = self.parse_argument();
                    
                    // Check if this looks like Euclidean syntax (has comma for second arg)
                    if let Some(Token::Comma) = self.current() {
                        self.advance();
                        
                        // Parse second argument
                        let steps = Box::new(self.parse_argument());
                        
                        // Optional rotation
                        let rotation = if let Some(Token::Comma) = self.current() {
                            self.advance();
                            Some(Box::new(self.parse_argument()))
                        } else {
                            None
                        };
                        
                        // Expect closing paren
                        if let Some(Token::CloseParen) = self.current() {
                            self.advance();
                            
                            // This is a valid Euclidean rhythm
                            return Some(AstNode::Euclid {
                                sample: s,
                                pulses: Box::new(first_arg),
                                steps,
                                rotation,
                            });
                        }
                    } else if let Some(Token::CloseParen) = self.current() {
                        // Single argument function like sine(440)
                        self.advance();
                        // Return as a complete string including the function call
                        let func_str = match first_arg {
                            AstNode::Atom(PatternValue::Number(n)) => format!("{}({})", s, n),
                            AstNode::Atom(PatternValue::String(arg)) => format!("{}({})", s, arg),
                            _ => format!("{}(...)", s),
                        };
                        return Some(AstNode::Atom(PatternValue::String(func_str)));
                    }
                    
                    // If we get here, reset and treat as simple atom
                    self.position = start_pos;
                    return Some(AstNode::Atom(PatternValue::String(s)));
                }
                
                // Check for chord notation with '
                if let Some(Token::Quote) = self.current() {
                    self.advance();
                    // Parse chord type (maj, min, etc.)
                    if let Some(Token::Symbol(chord_type)) = self.current() {
                        let chord = format!("{}'{}", s, chord_type);
                        self.advance();
                        return Some(AstNode::Atom(PatternValue::String(chord)));
                    }
                }
                
                AstNode::Atom(PatternValue::String(s))
            },
            Token::Number(n) => {
                let n = *n;
                self.advance();
                AstNode::Atom(PatternValue::Number(n))
            },
            Token::Rest => {
                self.advance();
                AstNode::Rest
            },
            Token::OpenBracket => {
                self.advance();
                let node = self.parse_group();
                if let Some(Token::CloseBracket) = self.current() {
                    self.advance();
                }
                node
            },
            Token::OpenAngle => {
                self.advance();
                let node = self.parse_alternation();
                if let Some(Token::CloseAngle) = self.current() {
                    self.advance();
                }
                node
            },
            Token::OpenParen => {
                self.advance();
                let node = self.parse_polyrhythm();
                if let Some(Token::CloseParen) = self.current() {
                    self.advance();
                }
                node
            },
            _ => {
                self.advance();
                return None;
            }
        };
        
        // Check for operators
        self.parse_operators(node)
    }
    
    /// Parse an argument (could be a number, alternation, etc.)
    fn parse_argument(&mut self) -> AstNode {
        match self.current() {
            Some(Token::Number(n)) => {
                let n = *n;
                self.advance();
                AstNode::Atom(PatternValue::Number(n))
            },
            Some(Token::OpenAngle) => {
                self.advance();
                let node = self.parse_alternation();
                if let Some(Token::CloseAngle) = self.current() {
                    self.advance();
                }
                node
            },
            Some(Token::Symbol(s)) => {
                let s = s.clone();
                self.advance();
                AstNode::Atom(PatternValue::String(s))
            },
            _ => AstNode::Atom(PatternValue::Number(1.0))
        }
    }
    
    /// Parse operators that follow an element
    fn parse_operators(&mut self, mut node: AstNode) -> Option<AstNode> {
        while let Some(token) = self.current() {
            match token {
                Token::Star => {
                    self.advance();
                    // Check if next token is an alternation
                    if matches!(self.current(), Some(Token::OpenAngle)) {
                        // Parse alternation pattern for dynamic replication
                        let amount = Box::new(self.parse_argument());
                        node = AstNode::Operator {
                            pattern: Box::new(node),
                            op: Operator::ReplicatePattern(amount),
                        };
                    } else {
                        // Parse static number
                        let amount = if let Some(Token::Number(n)) = self.current() {
                            let n = *n as usize;
                            self.advance();
                            n
                        } else {
                            2
                        };
                        node = AstNode::Operator {
                            pattern: Box::new(node),
                            op: Operator::Replicate(amount),
                        };
                    }
                },
                Token::Slash => {
                    self.advance();
                    let amount = Box::new(self.parse_argument());
                    node = AstNode::Operator {
                        pattern: Box::new(node),
                        op: Operator::Slow(amount),
                    };
                },
                Token::Question => {
                    self.advance();
                    let amount = if let Some(Token::Number(n)) = self.current() {
                        let n = *n;
                        self.advance();
                        n
                    } else {
                        0.5
                    };
                    node = AstNode::Operator {
                        pattern: Box::new(node),
                        op: Operator::Degrade(amount),
                    };
                },
                Token::At => {
                    self.advance();
                    if let Some(Token::Number(n)) = self.current() {
                        let n = *n;
                        self.advance();
                        node = AstNode::Operator {
                            pattern: Box::new(node),
                            op: Operator::Late(n),
                        };
                    }
                },
                _ => break
            }
        }
        Some(node)
    }
    
    /// Parse a bracketed group [a b c] 
    fn parse_group(&mut self) -> AstNode {
        // Check if it contains commas (polyrhythm)
        let mut has_comma = false;
        let start_pos = self.position;
        
        // Scan ahead for commas
        while let Some(token) = self.current() {
            match token {
                Token::CloseBracket => break,
                Token::Comma => {
                    has_comma = true;
                    break;
                },
                _ => { self.advance(); }
            }
        }
        
        // Reset position
        self.position = start_pos;
        
        if has_comma {
            // Parse as polyrhythm/stack
            self.parse_polyrhythm_content()
        } else {
            // Parse as fast sequence
            let mut children = Vec::new();
            
            while let Some(token) = self.current() {
                if matches!(token, Token::CloseBracket) {
                    break;
                }
                
                if let Some(child) = self.parse_element() {
                    children.push(child);
                }
            }
            
            if children.is_empty() {
                AstNode::Rest
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                AstNode::Pattern {
                    children,
                    alignment: Alignment::FastSequence,
                }
            }
        }
    }
    
    /// Parse alternation <a b c>
    fn parse_alternation(&mut self) -> AstNode {
        let mut children = Vec::new();
        
        // Parse space-separated items
        while let Some(token) = self.current() {
            if matches!(token, Token::CloseAngle) {
                break;
            }
            
            if let Some(child) = self.parse_element() {
                children.push(child);
            }
        }
        
        if children.is_empty() {
            AstNode::Rest
        } else if children.len() == 1 {
            children.into_iter().next().unwrap()
        } else {
            AstNode::Pattern {
                children,
                alignment: Alignment::Alternate,
            }
        }
    }
    
    /// Parse polyrhythm (a, b, c) or content inside [a, b]
    fn parse_polyrhythm(&mut self) -> AstNode {
        self.parse_polyrhythm_content()
    }
    
    fn parse_polyrhythm_content(&mut self) -> AstNode {
        let mut patterns = Vec::new();
        let mut current_pattern = Vec::new();
        
        while let Some(token) = self.current() {
            match token {
                Token::CloseParen | Token::CloseBracket => {
                    if !current_pattern.is_empty() {
                        let pat = if current_pattern.len() == 1 {
                            current_pattern.into_iter().next().unwrap()
                        } else {
                            AstNode::Pattern {
                                children: current_pattern,
                                alignment: Alignment::Sequence,
                            }
                        };
                        patterns.push(pat);
                        current_pattern = Vec::new();
                    }
                    break;
                },
                Token::Comma => {
                    self.advance();
                    if !current_pattern.is_empty() {
                        let pat = if current_pattern.len() == 1 {
                            current_pattern.into_iter().next().unwrap()
                        } else {
                            AstNode::Pattern {
                                children: current_pattern,
                                alignment: Alignment::Sequence,
                            }
                        };
                        patterns.push(pat);
                        current_pattern = Vec::new();
                    }
                },
                _ => {
                    if let Some(elem) = self.parse_element() {
                        current_pattern.push(elem);
                    }
                }
            }
        }
        
        if patterns.is_empty() {
            AstNode::Rest
        } else if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            AstNode::Pattern {
                children: patterns,
                alignment: Alignment::Stack,
            }
        }
    }
}

/// Convert AST to Pattern of PatternValue (for argument evaluation)
pub fn ast_to_pattern_value(ast: AstNode) -> Pattern<PatternValue> {
    match ast {
        AstNode::Atom(val) => Pattern::pure(val),
        
        AstNode::Rest => Pattern::silence(),
        
        AstNode::Pattern { children, alignment } => {
            let patterns: Vec<Pattern<PatternValue>> = children.into_iter()
                .map(ast_to_pattern_value)
                .collect();
            
            match alignment {
                Alignment::Sequence => Pattern::cat(patterns),
                Alignment::Stack => Pattern::stack(patterns),
                Alignment::Choose => {
                    // Random choice - for now just use first
                    patterns.into_iter().next().unwrap_or(Pattern::silence())
                },
                Alignment::Alternate => Pattern::slowcat(patterns),
                Alignment::FastSequence => Pattern::fastcat(patterns),
            }
        },
        
        AstNode::Operator { pattern, op } => {
            let pat = ast_to_pattern_value(*pattern);
            match op {
                Operator::Fast(amount) => {
                    match *amount {
                        AstNode::Atom(PatternValue::Number(n)) => pat.fast(n),
                        _ => pat
                    }
                },
                Operator::Slow(amount) => {
                    match *amount {
                        AstNode::Atom(PatternValue::Number(n)) => pat.slow(n),
                        _ => pat
                    }
                },
                Operator::Replicate(n) => {
                    let patterns = vec![pat; n];
                    Pattern::fastcat(patterns)
                },
                Operator::ReplicatePattern(amount) => {
                    // Convert amount AST to pattern
                    let amount_pat = ast_to_pattern_value(*amount);

                    // Create a pattern that evaluates replication with pattern argument
                    Pattern::new(move |state| {
                        // Get the cycle number to determine which value to use
                        let cycle = state.span.begin.to_float().floor() as i64;

                        // Create a query for just the current cycle point
                        let point_state = crate::pattern::State {
                            span: TimeSpan::new(
                                Fraction::new(cycle, 1),
                                Fraction::new(cycle, 1) + Fraction::new(1, 1000000),
                            ),
                            controls: state.controls.clone(),
                        };

                        // Query the amount pattern to get the value for this cycle
                        let n = amount_pat.query(&point_state)
                            .first()
                            .and_then(|h| h.value.as_number())
                            .unwrap_or(2.0) as usize;

                        // Create n copies and concatenate them fast
                        let patterns = vec![pat.clone(); n];
                        let replicated = Pattern::fastcat(patterns);
                        replicated.query(state)
                    })
                },
                Operator::Degrade(amount) => {
                    use crate::pattern_ops::*;
                    pat.degrade_by(amount)
                },
                Operator::Late(amount) => {
                    use crate::pattern_ops::*;
                    pat.late(amount)
                },
            }
        },
        
        // For euclidean in value context, just return a pattern of the sample name
        AstNode::Euclid { sample, .. } => {
            Pattern::pure(PatternValue::String(sample))
        },
    }
}

/// Convert AST to Pattern
/// This is where the magic happens - everything becomes a pattern that can be evaluated
pub fn ast_to_pattern(ast: AstNode) -> Pattern<String> {
    match ast {
        AstNode::Atom(val) => Pattern::pure(val.as_string()),
        
        AstNode::Rest => Pattern::silence(),
        
        AstNode::Pattern { children, alignment } => {
            let patterns: Vec<Pattern<String>> = children.into_iter()
                .map(ast_to_pattern)
                .collect();
            
            match alignment {
                Alignment::Sequence => Pattern::cat(patterns),
                Alignment::Stack => Pattern::stack(patterns),
                Alignment::Choose => {
                    // Random choice - for now just use first
                    // TODO: Implement proper random choice
                    patterns.into_iter().next().unwrap_or(Pattern::silence())
                },
                Alignment::Alternate => Pattern::slowcat(patterns),
                Alignment::FastSequence => Pattern::fastcat(patterns),
            }
        },
        
        AstNode::Operator { pattern, op } => {
            let pat = ast_to_pattern(*pattern);
            match op {
                Operator::Fast(amount) => {
                    // Evaluate the amount pattern to get a number
                    // For now, just handle simple cases
                    match *amount {
                        AstNode::Atom(PatternValue::Number(n)) => pat.fast(n),
                        _ => pat // TODO: Handle pattern-based speed
                    }
                },
                Operator::Slow(amount) => {
                    match *amount {
                        AstNode::Atom(PatternValue::Number(n)) => pat.slow(n),
                        _ => pat
                    }
                },
                Operator::Replicate(n) => {
                    // Create n copies and concatenate them fast
                    let patterns = vec![pat; n];
                    Pattern::fastcat(patterns)
                },
                Operator::ReplicatePattern(amount) => {
                    // Convert amount AST to pattern
                    let amount_pat = ast_to_pattern_value(*amount);

                    // Create a pattern that evaluates replication with pattern argument
                    Pattern::new(move |state| {
                        // Get the cycle number to determine which value to use
                        let cycle = state.span.begin.to_float().floor() as i64;

                        // Create a query for just the current cycle point
                        let point_state = crate::pattern::State {
                            span: TimeSpan::new(
                                Fraction::new(cycle, 1),
                                Fraction::new(cycle, 1) + Fraction::new(1, 1000000),
                            ),
                            controls: state.controls.clone(),
                        };

                        // Query the amount pattern to get the value for this cycle
                        let n = amount_pat.query(&point_state)
                            .first()
                            .and_then(|h| h.value.as_number())
                            .unwrap_or(2.0) as usize;

                        // Create n copies and concatenate them fast
                        let patterns = vec![pat.clone(); n];
                        let replicated = Pattern::fastcat(patterns);
                        replicated.query(state)
                    })
                },
                Operator::Degrade(amount) => {
                    use crate::pattern_ops::*;
                    pat.degrade_by(amount)
                },
                Operator::Late(amount) => {
                    use crate::pattern_ops::*;
                    pat.late(amount)
                },
            }
        },
        
        AstNode::Euclid { sample, pulses, steps, rotation } => {
            // Convert argument ASTs to patterns
            let pulses_pat = ast_to_pattern_value(*pulses);
            let steps_pat = ast_to_pattern_value(*steps);
            let rotation_pat = rotation.map(|r| ast_to_pattern_value(*r))
                .unwrap_or_else(|| Pattern::pure(PatternValue::Number(0.0)));
            
            // Create a pattern that evaluates euclidean with pattern arguments
            Pattern::new(move |state| {
                // Get the cycle number to determine which value to use from alternations
                let cycle = state.span.begin.to_float().floor() as i64;
                
                // Create a query for just the current cycle point
                let point_state = crate::pattern::State {
                    span: TimeSpan::new(
                        Fraction::new(cycle, 1),
                        Fraction::new(cycle, 1) + Fraction::new(1, 1000000),
                    ),
                    controls: state.controls.clone(),
                };
                
                // Query each argument pattern to get the value for this cycle
                let p = pulses_pat.query(&point_state)
                    .first()
                    .and_then(|h| h.value.as_number())
                    .unwrap_or(1.0) as usize;
                    
                let s = steps_pat.query(&point_state)
                    .first()
                    .and_then(|h| h.value.as_number())
                    .unwrap_or(8.0) as usize;
                    
                let r = rotation_pat.query(&point_state)
                    .first()
                    .and_then(|h| h.value.as_number())
                    .unwrap_or(0.0) as i32;
                
                // Create the euclidean pattern for these parameters
                let euclid_bool = Pattern::<bool>::euclid(p, s, r);
                
                // Convert to string pattern
                let sample_pattern = euclid_bool.fmap({
                    let sample = sample.clone();
                    move |hit| {
                        if hit { sample.clone() } else { "~".to_string() }
                    }
                });
                
                // Query the resulting pattern
                sample_pattern.query(state)
            })
        },
    }
}

/// Parse mini-notation string into a Pattern
pub fn parse_mini_notation(input: &str) -> Pattern<String> {
    let mut parser = MiniNotationParser::new(input);
    let ast = parser.parse();
    ast_to_pattern(ast)
}

// Make PatternValue work with Pattern
unsafe impl Send for PatternValue {}
unsafe impl Sync for PatternValue {}

/// Pattern extensions for mini-notation
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Create a fast concatenation of patterns (plays all in one cycle)
    pub fn fastcat(patterns: Vec<Pattern<T>>) -> Self {
        if patterns.is_empty() {
            return Pattern::silence();
        }

        let n = patterns.len() as f64;
        Pattern::new(move |state| {
            let mut all_haps = Vec::new();

            // Process each cycle that overlaps with the query
            let start_cycle = state.span.begin.to_float().floor() as i64;
            let end_cycle = state.span.end.to_float().ceil() as i64;

            for cycle in start_cycle..end_cycle {
                let cycle_f = cycle as f64;

                // Check if this cycle overlaps with our query
                let cycle_begin = cycle_f.max(state.span.begin.to_float());
                let cycle_end = (cycle_f + 1.0).min(state.span.end.to_float());

                if cycle_begin >= cycle_end {
                    continue;
                }

                for (i, pattern) in patterns.iter().enumerate() {
                    // Each pattern gets 1/n of each cycle
                    let pattern_begin = cycle_f + (i as f64 / n);
                    let pattern_end = cycle_f + ((i + 1) as f64 / n);

                    // Check if this pattern slice overlaps with our query
                    let query_begin = cycle_begin.max(pattern_begin);
                    let query_end = cycle_end.min(pattern_end);

                    if query_begin >= query_end {
                        continue;
                    }

                    // Scale the query to the pattern's local time (0-1)
                    let scaled_begin = (query_begin - pattern_begin) * n;
                    let scaled_end = (query_end - pattern_begin) * n;

                    let query_span = TimeSpan::new(
                        Fraction::from_float(scaled_begin),
                        Fraction::from_float(scaled_end),
                    );

                    let query_state = crate::pattern::State {
                        span: query_span,
                        controls: state.controls.clone(),
                    };

                    let haps = pattern.query(&query_state);

                    // Transform haps back to absolute time
                    for mut hap in haps {
                        let part_begin = hap.part.begin.to_float() / n + pattern_begin;
                        let part_end = hap.part.end.to_float() / n + pattern_begin;

                        hap.part = TimeSpan::new(
                            Fraction::from_float(part_begin),
                            Fraction::from_float(part_end),
                        );

                        if let Some(whole) = hap.whole {
                            let whole_begin = whole.begin.to_float() / n + pattern_begin;
                            let whole_end = whole.end.to_float() / n + pattern_begin;
                            hap.whole = Some(TimeSpan::new(
                                Fraction::from_float(whole_begin),
                                Fraction::from_float(whole_end),
                            ));
                        }

                        all_haps.push(hap);
                    }
                }
            }

            all_haps
        })
    }
    
}

// Add missing rand import
use rand;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_alternation() {
        // Test space-separated alternation syntax
        let pattern = parse_mini_notation("<bd sn cp>");
        
        // Query multiple cycles to see alternation
        for cycle in 0..3 {
            let state = crate::pattern::State {
                span: crate::pattern::TimeSpan::new(
                    crate::pattern::Fraction::new(cycle as i64, 1),
                    crate::pattern::Fraction::new((cycle + 1) as i64, 1),
                ),
                controls: std::collections::HashMap::new(),
            };
            
            let events = pattern.query(&state);
            println!("Cycle {}: {:?}", cycle, events.iter().map(|e| &e.value).collect::<Vec<_>>());
            
            // Each cycle should have exactly one event
            assert_eq!(events.len(), 1, "Each cycle should have one event");
            
            // Events should alternate: bd, sn, cp, bd, sn, cp...
            let expected = match cycle % 3 {
                0 => "bd",
                1 => "sn",
                2 => "cp",
                _ => unreachable!(),
            };
            assert_eq!(events[0].value, expected, "Cycle {} should have {}", cycle, expected);
        }
    }
    
    #[test]
    fn test_alternation_in_euclid() {
        // This should parse correctly now
        let pattern = parse_mini_notation("bd(<3 4>,8)");
        
        // Query multiple cycles to see alternation between 3 and 4 pulses
        for cycle in 0..4 {
            let state = crate::pattern::State {
                span: crate::pattern::TimeSpan::new(
                    crate::pattern::Fraction::new(cycle as i64, 1),
                    crate::pattern::Fraction::new((cycle + 1) as i64, 1),
                ),
                controls: std::collections::HashMap::new(),
            };
            
            let events = pattern.query(&state);
            let bd_count = events.iter().filter(|e| e.value == "bd").count();
            
            println!("Cycle {}: {} bd events", cycle, bd_count);
            
            // Even cycles should have 3 events, odd cycles should have 4
            let expected = if cycle % 2 == 0 { 3 } else { 4 };
            assert_eq!(bd_count, expected, "Cycle {} should have {} bd events", cycle, expected);
        }
    }
    
    #[test]
    fn test_chord_notation() {
        let pattern = parse_mini_notation("c'maj d'min");
        // Should parse chord notation correctly
    }
    
    #[test]
    fn test_nested_patterns() {
        let pattern = parse_mini_notation("[bd(3,8), <cp sn>*2]");
        // Should handle nested patterns correctly
    }
}