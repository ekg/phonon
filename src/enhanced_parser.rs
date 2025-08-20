//! Enhanced parser for the complete modular synthesis DSL
//! 
//! Supports arithmetic operations, bus references, and pattern integration

use crate::signal_graph::{
    SignalGraph, Node, NodeId, BusId,
    SourceType, ProcessorType, AnalysisType
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Identifiers and literals
    BusName(String),        // ~name
    Identifier(String),     // name
    Number(f64),           // 123.45
    String(String),        // "string"
    
    // Operators
    Colon,                 // :
    Arrow,                 // >>
    Plus,                  // +
    Minus,                 // -
    Star,                  // *
    Slash,                 // /
    Greater,               // >
    Less,                  // <
    Equal,                 // =
    Dot,                   // .
    
    // Delimiters
    LeftParen,             // (
    RightParen,            // )
    LeftBracket,           // [
    RightBracket,          // ]
    LeftBrace,             // {
    RightBrace,            // }
    Comma,                 // ,
    
    // Keywords
    Route,                 // route
    When,                  // when
    If,                    // if
    Out,                   // out
    
    // Special
    Newline,
    Comment(String),
    EOF,
}

/// Expression tree for arithmetic and signal operations  
#[derive(Debug, Clone)]
pub enum Expression {
    Number(f64),
    BusRef(String),
    Identifier(String),
    String(String),
    
    // Binary operations
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    
    // Comparisons
    GreaterThan(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
    
    // Function calls
    FunctionCall(String, Vec<Expression>),
    
    // Signal chain
    Chain(Box<Expression>, Box<Expression>),
    
    // Property access
    Property(Box<Expression>, String),
}

pub struct EnhancedParser {
    tokens: Vec<Token>,
    position: usize,
    graph: SignalGraph,
    node_counter: usize,
    buses: HashMap<String, Expression>,
    node_cache: HashMap<String, NodeId>,
}

impl EnhancedParser {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            tokens: Vec::new(),
            position: 0,
            graph: SignalGraph::new(sample_rate),
            node_counter: 0,
            buses: HashMap::new(),
            node_cache: HashMap::new(),
        }
    }
    
    fn generate_node_id(&mut self) -> NodeId {
        self.node_counter += 1;
        NodeId(format!("node_{}", self.node_counter))
    }
    
    fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::EOF)
    }
    
    fn peek_token(&self) -> &Token {
        self.tokens.get(self.position + 1).unwrap_or(&Token::EOF)
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
            Err(format!("Expected {:?}, found {:?}", expected, self.current_token()))
        }
    }
    
    /// Tokenize input string
    fn tokenize(input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();
        
        while let Some(&ch) = chars.peek() {
            match ch {
                ' ' | '\t' | '\r' => {
                    chars.next();
                }
                '\n' => {
                    chars.next();
                    tokens.push(Token::Newline);
                }
                '/' if chars.clone().nth(1) == Some('/') => {
                    // Comment
                    chars.next(); // first /
                    chars.next(); // second /
                    let mut comment = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '\n' {
                            break;
                        }
                        comment.push(ch);
                        chars.next();
                    }
                    tokens.push(Token::Comment(comment));
                }
                '~' => {
                    chars.next();
                    let mut name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            name.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::BusName(name));
                }
                '"' => {
                    chars.next();
                    let mut string = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '"' {
                            chars.next();
                            break;
                        }
                        string.push(ch);
                        chars.next();
                    }
                    tokens.push(Token::String(string));
                }
                ':' => {
                    chars.next();
                    tokens.push(Token::Colon);
                }
                '>' if chars.clone().nth(1) == Some('>') => {
                    chars.next();
                    chars.next();
                    tokens.push(Token::Arrow);
                }
                '>' => {
                    chars.next();
                    tokens.push(Token::Greater);
                }
                '<' => {
                    chars.next();
                    tokens.push(Token::Less);
                }
                '=' => {
                    chars.next();
                    tokens.push(Token::Equal);
                }
                '+' => {
                    chars.next();
                    tokens.push(Token::Plus);
                }
                '-' if chars.clone().nth(1).map_or(false, |c| c.is_numeric()) => {
                    // Negative number
                    chars.next();
                    let mut num_str = String::from("-");
                    while let Some(&ch) = chars.peek() {
                        if ch.is_numeric() || ch == '.' {
                            num_str.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Ok(num) = num_str.parse::<f64>() {
                        tokens.push(Token::Number(num));
                    }
                }
                '-' => {
                    chars.next();
                    tokens.push(Token::Minus);
                }
                '*' => {
                    chars.next();
                    tokens.push(Token::Star);
                }
                '/' => {
                    chars.next();
                    tokens.push(Token::Slash);
                }
                '(' => {
                    chars.next();
                    tokens.push(Token::LeftParen);
                }
                ')' => {
                    chars.next();
                    tokens.push(Token::RightParen);
                }
                '[' => {
                    chars.next();
                    tokens.push(Token::LeftBracket);
                }
                ']' => {
                    chars.next();
                    tokens.push(Token::RightBracket);
                }
                '{' => {
                    chars.next();
                    tokens.push(Token::LeftBrace);
                }
                '}' => {
                    chars.next();
                    tokens.push(Token::RightBrace);
                }
                ',' => {
                    chars.next();
                    tokens.push(Token::Comma);
                }
                '.' => {
                    chars.next();
                    tokens.push(Token::Dot);
                }
                ch if ch.is_numeric() => {
                    let mut num_str = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_numeric() || ch == '.' {
                            num_str.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Ok(num) = num_str.parse::<f64>() {
                        tokens.push(Token::Number(num));
                    }
                }
                ch if ch.is_alphabetic() => {
                    let mut ident = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            ident.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    let token = match ident.as_str() {
                        "route" => Token::Route,
                        "when" => Token::When,
                        "if" => Token::If,
                        "out" => Token::Out,
                        _ => Token::Identifier(ident),
                    };
                    tokens.push(token);
                }
                _ => {
                    chars.next(); // Skip unknown characters
                }
            }
        }
        
        tokens.push(Token::EOF);
        tokens
    }
    
    /// Parse a complete DSL string
    pub fn parse(&mut self, input: &str) -> Result<SignalGraph, String> {
        self.tokens = Self::tokenize(input);
        self.position = 0;
        
        while self.current_token() != &Token::EOF {
            // Skip newlines and comments
            match self.current_token() {
                Token::Newline | Token::Comment(_) => {
                    self.advance();
                    continue;
                }
                Token::BusName(_) => {
                    self.parse_bus_definition()?;
                }
                Token::Route => {
                    self.parse_route()?;
                }
                Token::Out => {
                    self.parse_output()?;
                }
                _ => {
                    // Skip unknown statements for now
                    self.advance();
                }
            }
        }
        
        // Build the signal graph from expressions
        self.build_graph()?;
        
        Ok(std::mem::replace(&mut self.graph, SignalGraph::new(44100.0)))
    }
    
    /// Parse bus definition: ~name: expression
    fn parse_bus_definition(&mut self) -> Result<(), String> {
        if let Token::BusName(name) = self.current_token().clone() {
            self.advance();
            self.expect(Token::Colon)?;
            
            let expr = self.parse_expression()?;
            self.buses.insert(name.clone(), expr);
            
            // Register bus in graph
            self.graph.add_bus(format!("~{}", name), 0.0);
            
            Ok(())
        } else {
            Err("Expected bus name".to_string())
        }
    }
    
    /// Parse expression with arithmetic operations
    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_additive()
    }
    
    /// Parse additive expression (+ and -)
    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;
        
        while let Token::Plus | Token::Minus = self.current_token() {
            let op = self.current_token().clone();
            self.advance();
            let right = self.parse_multiplicative()?;
            
            left = match op {
                Token::Plus => Expression::Add(Box::new(left), Box::new(right)),
                Token::Minus => Expression::Subtract(Box::new(left), Box::new(right)),
                _ => unreachable!(),
            };
        }
        
        Ok(left)
    }
    
    /// Parse multiplicative expression (* and /)
    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_chain()?;
        
        while let Token::Star | Token::Slash = self.current_token() {
            let op = self.current_token().clone();
            self.advance();
            let right = self.parse_chain()?;
            
            left = match op {
                Token::Star => Expression::Multiply(Box::new(left), Box::new(right)),
                Token::Slash => Expression::Divide(Box::new(left), Box::new(right)),
                _ => unreachable!(),
            };
        }
        
        Ok(left)
    }
    
    /// Parse signal chain (>>)
    fn parse_chain(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_primary()?;
        
        while let Token::Arrow = self.current_token() {
            self.advance();
            let right = self.parse_primary()?;
            left = Expression::Chain(Box::new(left), Box::new(right));
        }
        
        Ok(left)
    }
    
    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.current_token().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expression::Number(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expression::String(s))
            }
            Token::BusName(name) => {
                self.advance();
                Ok(Expression::BusRef(name))
            }
            Token::Identifier(name) => {
                self.advance();
                
                // Check for function call
                if let Token::LeftParen = self.current_token() {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.expect(Token::RightParen)?;
                    Ok(Expression::FunctionCall(name, args))
                } else {
                    Ok(Expression::Identifier(name))
                }
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            _ => Err(format!("Unexpected token in expression: {:?}", self.current_token()))
        }
    }
    
    /// Parse function arguments
    fn parse_arguments(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = Vec::new();
        
        if let Token::RightParen = self.current_token() {
            return Ok(args);
        }
        
        loop {
            args.push(self.parse_expression()?);
            
            if let Token::Comma = self.current_token() {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(args)
    }
    
    /// Parse route statement
    fn parse_route(&mut self) -> Result<(), String> {
        self.advance(); // Skip 'route'
        // TODO: Implement route parsing
        // For now, skip until newline
        while !matches!(self.current_token(), Token::Newline | Token::EOF) {
            self.advance();
        }
        Ok(())
    }
    
    /// Parse output statement
    fn parse_output(&mut self) -> Result<(), String> {
        self.advance(); // Skip 'out'
        if let Token::Colon = self.current_token() {
            self.advance();
            let expr = self.parse_expression()?;
            // Store the output expression
            self.buses.insert("out".to_string(), expr);
        }
        Ok(())
    }
    
    /// Build signal graph from parsed expressions
    fn build_graph(&mut self) -> Result<(), String> {
        // Store created nodes to avoid duplicates
        self.node_cache.clear();
        
        // Process all buses except "out" first
        for (bus_name, expr) in &self.buses.clone() {
            if bus_name != "out" {
                let node_id = self.process_bus_expression(bus_name, expr)?;
                self.node_cache.insert(bus_name.clone(), node_id);
            }
        }
        
        // Process output if present
        if let Some(out_expr) = self.buses.get("out").cloned() {
            // Create output node
            let output_node = Node::Output {
                id: NodeId("output".to_string()),
            };
            self.graph.add_node(output_node);
            
            // Process output expression and connect to output node
            let out_node_id = self.process_expression(&out_expr, "out")?;
            self.graph.connect(out_node_id, NodeId("output".to_string()), 1.0);
        }
        
        Ok(())
    }
    
    /// Process a bus expression and return its output node
    fn process_bus_expression(&mut self, bus_name: &str, expr: &Expression) -> Result<NodeId, String> {
        // Check cache first
        if let Some(node_id) = self.node_cache.get(bus_name) {
            return Ok(node_id.clone());
        }
        
        self.process_expression(expr, bus_name)
    }
    
    /// Process any expression and return its output node
    fn process_expression(&mut self, expr: &Expression, context: &str) -> Result<NodeId, String> {
        match expr {
            Expression::FunctionCall(name, args) => {
                // Create source or processor node
                let node_id = self.generate_node_id();
                let node = self.create_node_from_function(name, args, &node_id)?;
                self.graph.add_node(node);
                Ok(node_id)
            }
            Expression::BusRef(name) => {
                // Reference to another bus
                if let Some(node_id) = self.node_cache.get(name) {
                    Ok(node_id.clone())
                } else if let Some(bus_expr) = self.buses.get(name).cloned() {
                    let node_id = self.process_bus_expression(name, &bus_expr)?;
                    self.node_cache.insert(name.clone(), node_id.clone());
                    Ok(node_id)
                } else {
                    Err(format!("Unknown bus reference: {}", name))
                }
            }
            Expression::Add(left, right) => {
                // Create a mixer node for addition
                let left_node = self.process_expression(left, context)?;
                let right_node = self.process_expression(right, context)?;
                
                // Create a mixer node 
                let mixer_id = self.generate_node_id();
                let mixer_node = Node::Processor {
                    id: mixer_id.clone(),
                    processor_type: ProcessorType::Gain { amount: 1.0 }, // Acts as a summing node
                };
                self.graph.add_node(mixer_node);
                
                // Connect both inputs to the mixer
                self.graph.connect(left_node, mixer_id.clone(), 1.0);
                self.graph.connect(right_node, mixer_id.clone(), 1.0);
                
                Ok(mixer_id)
            }
            Expression::Multiply(left, right) => {
                // Process left side
                let left_node = self.process_expression(left, context)?;
                
                // If right is a number, create gain node
                if let Expression::Number(n) = right.as_ref() {
                    let gain_id = self.generate_node_id();
                    let gain_node = Node::Processor {
                        id: gain_id.clone(),
                        processor_type: ProcessorType::Gain { amount: *n as f32 },
                    };
                    self.graph.add_node(gain_node);
                    self.graph.connect(left_node, gain_id.clone(), 1.0);
                    Ok(gain_id)
                } else {
                    // For now, just return left side
                    Ok(left_node)
                }
            }
            Expression::Chain(left, right) => {
                // Process chain: left >> right
                let source_id = self.process_expression(left, context)?;
                let proc_id = self.process_expression(right, context)?;
                self.graph.connect(source_id, proc_id.clone(), 1.0);
                Ok(proc_id)
            }
            Expression::Number(n) => {
                // Create a constant value node
                let node_id = self.generate_node_id();
                let node = Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sine { freq: 0.0 }, // Will act as DC constant
                };
                self.graph.add_node(node);
                Ok(node_id)
            }
            _ => {
                // For other expressions, create a placeholder
                let node_id = self.generate_node_id();
                Ok(node_id)
            }
        }
    }
    
    /// Create a node from a function call
    fn create_node_from_function(&self, name: &str, args: &[Expression], node_id: &NodeId) -> Result<Node, String> {
        match name {
            "sine" => {
                let freq = self.eval_expression(&args[0])?;
                Ok(Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sine { freq },
                })
            }
            "saw" => {
                let freq = self.eval_expression(&args[0])?;
                Ok(Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Saw { freq },
                })
            }
            "lpf" => {
                if args.len() >= 2 {
                    let cutoff = self.eval_expression(&args[0])?;
                    let q = self.eval_expression(&args[1])?;
                    Ok(Node::Processor {
                        id: node_id.clone(),
                        processor_type: ProcessorType::LowPass { cutoff, q },
                    })
                } else {
                    Err("lpf requires cutoff and Q parameters".to_string())
                }
            }
            _ => {
                // Default to sine for unknown functions
                Ok(Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sine { freq: 440.0 },
                })
            }
        }
    }
    
    /// Convert expression to nodes and return the output node ID
    fn expression_to_nodes(&mut self, expr: &Expression, context: &str) -> Result<NodeId, String> {
        match expr {
            Expression::Number(n) => {
                // Create a constant source
                let node_id = self.generate_node_id();
                let node = Node::Source {
                    id: node_id.clone(),
                    source_type: SourceType::Sine { freq: 0.0 }, // Will use as constant
                };
                self.graph.add_node(node);
                Ok(node_id)
            }
            Expression::BusRef(name) => {
                // Reference to another bus - need to find or create its node
                // First check if we already have a node for this bus
                if let Some(existing_node) = self.graph.nodes.iter()
                    .find(|(_, n)| {
                        if let Node::Source { id, .. } | Node::Processor { id, .. } = n {
                            id.0.contains(name)
                        } else {
                            false
                        }
                    })
                    .map(|(id, _)| id.clone()) {
                    Ok(existing_node)
                } else if let Some(bus_expr) = self.buses.get(name).cloned() {
                    // Create the node for this bus
                    self.expression_to_nodes(&bus_expr, name)
                } else {
                    // Unknown bus - create placeholder
                    Ok(NodeId(name.clone()))
                }
            }
            Expression::FunctionCall(name, args) => {
                // Create appropriate node based on function
                let node_id = self.generate_node_id();
                let node = match name.as_str() {
                    "sine" => {
                        let freq = self.eval_expression(&args[0])?;
                        Node::Source {
                            id: node_id.clone(),
                            source_type: SourceType::Sine { freq },
                        }
                    }
                    "saw" => {
                        let freq = self.eval_expression(&args[0])?;
                        Node::Source {
                            id: node_id.clone(),
                            source_type: SourceType::Saw { freq },
                        }
                    }
                    "square" => {
                        let freq = self.eval_expression(&args[0])?;
                        Node::Source {
                            id: node_id.clone(),
                            source_type: SourceType::Square { freq },
                        }
                    }
                    "lpf" => {
                        if args.len() >= 2 {
                            let cutoff = self.eval_expression(&args[0])?;
                            let q = self.eval_expression(&args[1])?;
                            Node::Processor {
                                id: node_id.clone(),
                                processor_type: ProcessorType::LowPass { cutoff, q },
                            }
                        } else {
                            return Err("lpf requires cutoff and Q parameters".to_string());
                        }
                    }
                    "hpf" => {
                        if args.len() >= 2 {
                            let cutoff = self.eval_expression(&args[0])?;
                            let q = self.eval_expression(&args[1])?;
                            Node::Processor {
                                id: node_id.clone(),
                                processor_type: ProcessorType::HighPass { cutoff, q },
                            }
                        } else {
                            return Err("hpf requires cutoff and Q parameters".to_string());
                        }
                    }
                    "gain" => {
                        let amount = if args.is_empty() { 
                            1.0 
                        } else { 
                            self.eval_expression(&args[0])? as f32
                        };
                        Node::Processor {
                            id: node_id.clone(),
                            processor_type: ProcessorType::Gain { amount },
                        }
                    }
                    _ => {
                        // Unknown function - create a placeholder
                        Node::Source {
                            id: node_id.clone(),
                            source_type: SourceType::Sine { freq: 440.0 },
                        }
                    }
                };
                self.graph.add_node(node);
                Ok(node_id)
            }
            Expression::Add(left, right) => {
                // Create nodes for both sides and mix them
                let left_id = self.expression_to_nodes(left, context)?;
                let right_id = self.expression_to_nodes(right, context)?;
                let mix_id = self.generate_node_id();
                
                // Create a gain node to mix
                let mix_node = Node::Processor {
                    id: mix_id.clone(),
                    processor_type: ProcessorType::Gain { amount: 1.0 },
                };
                self.graph.add_node(mix_node);
                
                // Connect both inputs to mixer
                self.graph.connect(left_id, mix_id.clone(), 1.0);
                self.graph.connect(right_id, mix_id.clone(), 1.0);
                
                Ok(mix_id)
            }
            Expression::Multiply(left, right) => {
                // For now, just evaluate as constant if possible
                let left_id = self.expression_to_nodes(left, context)?;
                if let Expression::Number(n) = right.as_ref() {
                    // Apply gain
                    let gain_id = self.generate_node_id();
                    let gain_node = Node::Processor {
                        id: gain_id.clone(),
                        processor_type: ProcessorType::Gain { amount: *n as f32 },
                    };
                    self.graph.add_node(gain_node);
                    self.graph.connect(left_id, gain_id.clone(), 1.0);
                    Ok(gain_id)
                } else {
                    Ok(left_id)
                }
            }
            Expression::Chain(left, right) => {
                // Process chain: left >> right
                let source_id = self.expression_to_nodes(left, context)?;
                let proc_id = self.expression_to_nodes(right, context)?;
                self.graph.connect(source_id, proc_id.clone(), 1.0);
                Ok(proc_id)
            }
            _ => Ok(NodeId(format!("placeholder_{}", context)))
        }
    }
    
    /// Evaluate expression to a numeric value
    fn eval_expression(&self, expr: &Expression) -> Result<f64, String> {
        match expr {
            Expression::Number(n) => Ok(*n),
            Expression::Add(l, r) => Ok(self.eval_expression(l)? + self.eval_expression(r)?),
            Expression::Subtract(l, r) => Ok(self.eval_expression(l)? - self.eval_expression(r)?),
            Expression::Multiply(l, r) => Ok(self.eval_expression(l)? * self.eval_expression(r)?),
            Expression::Divide(l, r) => Ok(self.eval_expression(l)? / self.eval_expression(r)?),
            _ => Ok(0.0) // Default for non-numeric expressions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tokenize_arithmetic() {
        let tokens = EnhancedParser::tokenize("~lfo: sine(2) * 0.5 + 0.5");
        assert!(tokens.contains(&Token::BusName("lfo".to_string())));
        assert!(tokens.contains(&Token::Star));
        assert!(tokens.contains(&Token::Plus));
        assert!(tokens.contains(&Token::Number(0.5)));
    }
    
    #[test]
    fn test_parse_arithmetic_expression() {
        let mut parser = EnhancedParser::new(44100.0);
        parser.tokens = vec![
            Token::Number(2.0),
            Token::Plus,
            Token::Number(3.0),
            Token::Star,
            Token::Number(4.0),
            Token::EOF,
        ];
        parser.position = 0;
        
        let expr = parser.parse_expression().unwrap();
        // Should parse as 2 + (3 * 4) due to precedence
        match expr {
            Expression::Add(left, right) => {
                assert!(matches!(*left.as_ref(), Expression::Number(2.0)));
                assert!(matches!(*right.as_ref(), Expression::Multiply(_, _)));
            }
            _ => panic!("Expected Add expression"),
        }
    }
    
    #[test]
    fn test_parse_bus_reference() {
        let mut parser = EnhancedParser::new(44100.0);
        let input = "~lfo: sine(2)\n~modulated: ~lfo * 100";
        let result = parser.parse(input);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_pattern_string() {
        let mut parser = EnhancedParser::new(44100.0);
        let input = r#"~rhythm: "bd sn bd sn""#;
        let result = parser.parse(input);
        assert!(result.is_ok());
    }
}