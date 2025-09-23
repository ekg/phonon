//! High-performance nom-based parser for Phonon DSL
//! Optimized for live coding with minimal allocations and fast parsing

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1, take_until},
    character::complete::{char, multispace0, multispace1, alpha1, alphanumeric1, digit1},
    combinator::{map, map_res, opt, recognize, value, eof},
    multi::{many0, many1, separated_list0, separated_list1},
    number::complete::double,
    sequence::{delimited, preceded, terminated, tuple, pair},
};
use std::collections::HashMap;
use crate::glicol_dsp::{DspChain, DspNode, DspEnvironment, LfoShape};

/// AST representation for parsed expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Bus(String),
    Node(String, Vec<Expr>),
    Chain(Box<Expr>, Box<Expr>),
    Pattern(String),
    PatternOp(Box<Expr>, PatternTransform),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
}

/// Pattern transformation operations
#[derive(Debug, Clone, PartialEq)]
pub enum PatternTransform {
    Fast(f64),
    Slow(f64),
    Rev,
    Palindrome,
    Rotate(f64),
    Degrade(f64),
    DegradeBy(f64),
    Every(i32, Box<PatternTransform>),
    Sometimes(Box<PatternTransform>),
    Rarely(Box<PatternTransform>),
    Often(Box<PatternTransform>),
    Jux(Box<PatternTransform>),
    Chunk(i32, Box<PatternTransform>),
    Chop(i32),
    Striate(i32),
    Shuffle(i32),
    Scramble(i32),
    Spread(Box<PatternTransform>),
    Scale(String),
    Gain(f64),
    Pan(f64),
    Speed(f64),
    Crush(f64),
    Coarse(i32),
    Cut(i32),
    Legato(f64),
    Shape(f64),
    Squiz(f64),
    Accelerate(f64),
}

/// Parse a floating point number
fn parse_number(input: &str) -> IResult<&str, f64> {
    alt((
        double,
        map_res(digit1, |s: &str| s.parse::<f64>()),
    ))(input)
}

/// Parse an identifier (alphanumeric with underscores)
fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(
        pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_"))))
        )
    )(input)
}

/// Parse a bus reference (~name)
fn parse_bus_ref(input: &str) -> IResult<&str, Expr> {
    map(
        preceded(char('~'), parse_identifier),
        |name: &str| Expr::Bus(name.to_string())
    )(input)
}

/// Parse a string literal
fn parse_string(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        map(take_until("\""), |s: &str| s.to_string()),
        char('"')
    )(input)
}

/// Parse a pattern expression (s "pattern" or "pattern")
fn parse_pattern(input: &str) -> IResult<&str, Expr> {
    alt((
        map(
            preceded(
                tuple((tag("s"), multispace1)),
                parse_string
            ),
            Expr::Pattern
        ),
        map(parse_string, Expr::Pattern)
    ))(input)
}

/// Parse a primary expression (number, bus, pattern, or parenthesized)
fn parse_primary(input: &str) -> IResult<&str, Expr> {
    delimited(
        multispace0,
        alt((
            parse_bus_ref,
            parse_pattern,
            map(parse_number, Expr::Number),
            parse_node,
            delimited(char('('), parse_expr, char(')')),
        )),
        multispace0
    )(input)
}

/// Parse a DSP node with arguments
fn parse_node(input: &str) -> IResult<&str, Expr> {
    let (input, name) = parse_identifier(input)?;
    
    // Parse arguments if present (but not other nodes to avoid recursion)
    // Don't consume whitespace first - let preceded handle it
    let (input, args) = many0(preceded(multispace1, parse_node_arg))(input)?;
    
    Ok((input, Expr::Node(name.to_string(), args)))
}

/// Parse an argument for a node (number, bus ref, or pattern, but not another node)
fn parse_node_arg(input: &str) -> IResult<&str, Expr> {
    alt((
        parse_bus_ref,
        parse_pattern,
        map(parse_number, Expr::Number),
        delimited(char('('), parse_expr, char(')')),
    ))(input)
}

/// Parse multiplication and division (higher precedence)
fn parse_mul_div(input: &str) -> IResult<&str, Expr> {
    let (input, first) = parse_primary(input)?;
    
    let (input, operations) = many0(tuple((
        delimited(multispace0, alt((char('*'), char('/'))), multispace0),
        parse_primary
    )))(input)?;
    
    Ok((input, operations.into_iter().fold(first, |acc, (op, expr)| {
        match op {
            '*' => Expr::Mul(Box::new(acc), Box::new(expr)),
            '/' => Expr::Div(Box::new(acc), Box::new(expr)),
            _ => unreachable!()
        }
    })))
}

/// Parse addition and subtraction (lower precedence)
fn parse_add_sub(input: &str) -> IResult<&str, Expr> {
    let (input, first) = parse_mul_div(input)?;
    
    let (input, operations) = many0(tuple((
        delimited(multispace0, alt((char('+'), char('-'))), multispace0),
        parse_mul_div
    )))(input)?;
    
    Ok((input, operations.into_iter().fold(first, |acc, (op, expr)| {
        match op {
            '+' => Expr::Add(Box::new(acc), Box::new(expr)),
            '-' => Expr::Sub(Box::new(acc), Box::new(expr)),
            _ => unreachable!()
        }
    })))
}

/// Parse pattern transformation function
fn parse_pattern_transform(input: &str) -> IResult<&str, PatternTransform> {
    let (input, name) = parse_identifier(input)?;
    let (input, _) = multispace0(input)?;
    
    match name {
        "fast" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Fast(n)))
        }
        "slow" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Slow(n)))
        }
        "rev" => Ok((input, PatternTransform::Rev)),
        "palindrome" => Ok((input, PatternTransform::Palindrome)),
        "rotate" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Rotate(n)))
        }
        "degrade" => {
            // Check if there's a parameter
            if let Ok((input, n)) = parse_number(input) {
                Ok((input, PatternTransform::DegradeBy(n)))
            } else {
                Ok((input, PatternTransform::Degrade(0.5)))
            }
        }
        "degradeBy" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::DegradeBy(n)))
        }
        "every" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            let (input, _) = multispace1(input)?;
            // Handle optional parentheses
            let (input, _) = multispace0(input)?;
            let (input, has_paren) = opt(char('('))(input)?;
            let (input, _) = multispace0(input)?;
            let (input, transform) = parse_pattern_transform(input)?;
            let (input, _) = multispace0(input)?;
            let input = if has_paren.is_some() {
                let (input, _) = char(')')(input)?;
                input
            } else {
                input
            };
            Ok((input, PatternTransform::Every(n, Box::new(transform))))
        }
        "sometimes" => {
            let (input, _) = multispace0(input)?;
            let (input, has_paren) = opt(char('('))(input)?;
            let (input, _) = multispace0(input)?;
            let (input, transform) = parse_pattern_transform(input)?;
            let (input, _) = multispace0(input)?;
            let input = if has_paren.is_some() {
                let (input, _) = char(')')(input)?;
                input
            } else {
                input
            };
            Ok((input, PatternTransform::Sometimes(Box::new(transform))))
        }
        "rarely" => {
            let (input, _) = multispace0(input)?;
            let (input, has_paren) = opt(char('('))(input)?;
            let (input, _) = multispace0(input)?;
            let (input, transform) = parse_pattern_transform(input)?;
            let (input, _) = multispace0(input)?;
            let input = if has_paren.is_some() {
                let (input, _) = char(')')(input)?;
                input
            } else {
                input
            };
            Ok((input, PatternTransform::Rarely(Box::new(transform))))
        }
        "often" => {
            let (input, _) = multispace0(input)?;
            let (input, has_paren) = opt(char('('))(input)?;
            let (input, _) = multispace0(input)?;
            let (input, transform) = parse_pattern_transform(input)?;
            let (input, _) = multispace0(input)?;
            let input = if has_paren.is_some() {
                let (input, _) = char(')')(input)?;
                input
            } else {
                input
            };
            Ok((input, PatternTransform::Often(Box::new(transform))))
        }
        "jux" => {
            let (input, _) = multispace0(input)?;
            let (input, transform) = parse_pattern_transform(input)?;
            Ok((input, PatternTransform::Jux(Box::new(transform))))
        }
        "chunk" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            let (input, _) = multispace1(input)?;
            let (input, _) = multispace0(input)?;
            let (input, has_paren) = opt(char('('))(input)?;
            let (input, _) = multispace0(input)?;
            let (input, transform) = parse_pattern_transform(input)?;
            let (input, _) = multispace0(input)?;
            let input = if has_paren.is_some() {
                let (input, _) = char(')')(input)?;
                input
            } else {
                input
            };
            Ok((input, PatternTransform::Chunk(n, Box::new(transform))))
        }
        "chop" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            Ok((input, PatternTransform::Chop(n)))
        }
        "striate" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            Ok((input, PatternTransform::Striate(n)))
        }
        "shuffle" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            Ok((input, PatternTransform::Shuffle(n)))
        }
        "scramble" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            Ok((input, PatternTransform::Scramble(n)))
        }
        "scale" => {
            let (input, scale_name) = alt((
                delimited(char('"'), take_until("\""), char('"')),
                parse_identifier
            ))(input)?;
            Ok((input, PatternTransform::Scale(scale_name.to_string())))
        }
        "gain" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Gain(n)))
        }
        "pan" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Pan(n)))
        }
        "speed" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Speed(n)))
        }
        "crush" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Crush(n)))
        }
        "coarse" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            Ok((input, PatternTransform::Coarse(n)))
        }
        "cut" => {
            let (input, n) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;
            Ok((input, PatternTransform::Cut(n)))
        }
        "legato" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Legato(n)))
        }
        "shape" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Shape(n)))
        }
        "squiz" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Squiz(n)))
        }
        "accelerate" => {
            let (input, n) = parse_number(input)?;
            Ok((input, PatternTransform::Accelerate(n)))
        }
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
}

/// Parse pattern operations (|>)
/// Pattern ops have lower precedence than chains
fn parse_pattern_ops(input: &str) -> IResult<&str, Expr> {
    // First parse the base expression (which could include chains)
    let (input, first) = parse_chain(input)?;
    
    // Then check for pattern transforms (|> or $ operator)
    let (input, transforms) = many0(preceded(
        delimited(multispace0, alt((tag("|>"), tag("$"))), multispace0),
        parse_pattern_transform
    ))(input)?;
    
    // Apply transforms
    let with_transforms = transforms.into_iter().fold(first, |acc, transform| {
        Expr::PatternOp(Box::new(acc), transform)
    });
    
    // After pattern transforms, check if there's a chain continuation
    // This handles cases like: "pattern" |> fast 2 >> dsp
    let (input, chain_rest) = many0(preceded(
        delimited(multispace0, tag(">>"), multispace0),
        parse_add_sub
    ))(input)?;
    
    Ok((input, chain_rest.into_iter().fold(with_transforms, |acc, next| {
        Expr::Chain(Box::new(acc), Box::new(next))
    })))
}

/// Parse chain operations (>>)
fn parse_chain(input: &str) -> IResult<&str, Expr> {
    let (input, first) = parse_add_sub(input)?;
    
    let (input, rest) = many0(preceded(
        delimited(multispace0, tag(">>"), multispace0),
        parse_add_sub
    ))(input)?;
    
    Ok((input, rest.into_iter().fold(first, |acc, next| {
        Expr::Chain(Box::new(acc), Box::new(next))
    })))
}

/// Top-level expression parser
pub fn parse_expr(input: &str) -> IResult<&str, Expr> {
    parse_pattern_ops(input)
}

/// Parse a bus definition (~name: expression)
fn parse_bus_definition(input: &str) -> IResult<&str, (&str, Expr)> {
    tuple((
        preceded(char('~'), parse_identifier),
        preceded(
            delimited(multispace0, char(':'), multispace0),
            parse_expr
        )
    ))(input)
}

/// Parse an output definition (o: expression or out: expression)
fn parse_output_definition(input: &str) -> IResult<&str, Expr> {
    preceded(
        tuple((
            alt((tag("o"), tag("out"))),
            delimited(multispace0, char(':'), multispace0)
        )),
        parse_expr
    )(input)
}

/// Parse a complete line (bus definition or output)
fn parse_line(input: &str) -> IResult<&str, LineType> {
    // Skip leading whitespace
    let (input, _) = multispace0(input)?;
    
    alt((
        map(parse_bus_definition, |(name, expr)| LineType::Bus(name.to_string(), expr)),
        map(parse_output_definition, LineType::Output),
        value(LineType::Empty, multispace0)
    ))(input)
}

#[derive(Debug, Clone)]
enum LineType {
    Bus(String, Expr),
    Output(Expr),
    Empty,
}

/// Parse the entire DSL input into an environment
pub fn parse_dsl(input: &str) -> Result<DspEnvironment, String> {
    let mut env = DspEnvironment::new();
    let mut buses = HashMap::new();
    
    // Process line by line for better error reporting
    for (line_num, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        
        match parse_line(line) {
            Ok((_, LineType::Bus(name, expr))) => {
                let chain = expr_to_dsp_chain(&expr, &buses)?;
                buses.insert(name.clone(), chain.clone());
                env.add_ref(name, chain);
            }
            Ok((_, LineType::Output(expr))) => {
                let chain = expr_to_dsp_chain(&expr, &buses)?;
                env.set_output(chain);
            }
            Ok((_, LineType::Empty)) => {}
            Err(_e) => {
                // Try to provide better error messages
                if line.trim().starts_with("o:") || line.trim().starts_with("out:") {
                    // This is an output line that failed to parse
                    return Err(format!("Failed to parse output line {}: {}", line_num + 1, line.trim()));
                }
                // Skip lines that don't parse (might be comments or other content)
                continue;
            }
        }
    }
    
    if env.output_chain.is_none() {
        return Err("No output (o: or out:) defined".to_string());
    }
    
    Ok(env)
}

/// Convert an expression AST to a DSP chain
fn expr_to_dsp_chain(expr: &Expr, buses: &HashMap<String, DspChain>) -> Result<DspChain, String> {
    match expr {
        Expr::Number(n) => Ok(DspChain {
            nodes: vec![DspNode::Value(*n as f32)]
        }),
        
        Expr::Bus(name) => {
            buses.get(name)
                .cloned()
                .ok_or_else(|| format!("Unknown bus: ~{}", name))
        }
        
        Expr::Node(name, args) => {
            let node = match name.as_str() {
                // Oscillators
                "sin" | "sine" => {
                    let freq = get_number_arg(args, 0, 440.0)?;
                    DspNode::Sin { freq: freq as f32 }
                }
                "saw" => {
                    let freq = get_number_arg(args, 0, 440.0)?;
                    DspNode::Saw { freq: freq as f32 }
                }
                "square" => {
                    let freq = get_number_arg(args, 0, 440.0)?;
                    DspNode::Square { freq: freq as f32, duty: 0.5 }
                }
                "tri" | "triangle" => {
                    let freq = get_number_arg(args, 0, 440.0)?;
                    DspNode::Triangle { freq: freq as f32 }
                }
                
                // Noise generators
                "noise" => DspNode::Noise { seed: 42 },
                "pink" => DspNode::Pink { seed: 42 },
                "brown" => DspNode::Brown { seed: 42 },
                
                // Math operations
                "mul" => {
                    let factor = get_number_arg(args, 0, 1.0)?;
                    DspNode::Mul { factor: factor as f32 }
                }
                "add" => {
                    let value = get_number_arg(args, 0, 0.0)?;
                    DspNode::Add { value: value as f32 }
                }
                
                // Filters
                "lpf" => {
                    let cutoff = get_number_arg(args, 0, 1000.0)?;
                    let q = get_number_arg(args, 1, 0.7)?;
                    DspNode::Lpf { cutoff: cutoff as f32, q: q as f32 }
                }
                "hpf" => {
                    let cutoff = get_number_arg(args, 0, 1000.0)?;
                    let q = get_number_arg(args, 1, 0.7)?;
                    DspNode::Hpf { cutoff: cutoff as f32, q: q as f32 }
                }
                
                // Effects
                "delay" => {
                    let time = get_number_arg(args, 0, 0.25)?;
                    let feedback = get_number_arg(args, 1, 0.5)?;
                    let mix = get_number_arg(args, 2, 0.5)?;
                    DspNode::Delay { 
                        time: time as f32, 
                        feedback: feedback as f32, 
                        mix: mix as f32 
                    }
                }
                "reverb" => {
                    let room_size = get_number_arg(args, 0, 0.5)?;
                    let damping = get_number_arg(args, 1, 0.5)?;
                    let mix = get_number_arg(args, 2, 0.3)?;
                    DspNode::Reverb { 
                        room_size: room_size as f32,
                        damping: damping as f32,
                        mix: mix as f32 
                    }
                }
                
                // Pattern support
                "s" => {
                    if let Some(Expr::Pattern(pattern)) = args.get(0) {
                        DspNode::Pattern { 
                            pattern: pattern.clone(),
                            speed: 1.0 
                        }
                    } else {
                        return Err(format!("s requires a pattern string"));
                    }
                }
                
                _ => return Err(format!("Unknown node type: {}", name))
            };
            
            Ok(DspChain { nodes: vec![node] })
        }
        
        Expr::Chain(left, right) => {
            let mut chain = expr_to_dsp_chain(left, buses)?;
            let right_chain = expr_to_dsp_chain(right, buses)?;
            chain.nodes.extend(right_chain.nodes);
            Ok(chain)
        }
        
        Expr::Pattern(pattern) => {
            Ok(DspChain {
                nodes: vec![DspNode::Pattern { 
                    pattern: pattern.clone(),
                    speed: 1.0 
                }]
            })
        }
        
        Expr::PatternOp(base, transform) => {
            // Convert pattern operations into a special node or apply transformation
            // For now, we'll convert the base and note the transform in metadata
            let mut chain = expr_to_dsp_chain(base, buses)?;
            
            // Apply transform as a metadata/processing instruction
            // This would need to be handled by the pattern engine
            match transform {
                PatternTransform::Fast(n) => {
                    // Speed up the pattern
                    if let Some(DspNode::Pattern { speed, .. }) = chain.nodes.first_mut() {
                        *speed *= *n as f32;
                    }
                }
                PatternTransform::Slow(n) => {
                    // Slow down the pattern
                    if let Some(DspNode::Pattern { speed, .. }) = chain.nodes.first_mut() {
                        *speed /= *n as f32;
                    }
                }
                // Other transforms would be handled by the pattern engine
                _ => {
                    // For now, just pass through
                    // In a full implementation, these would modify the pattern
                }
            }
            
            Ok(chain)
        }
        
        // Math operations create signal math nodes
        Expr::Add(left, right) => {
            let left_chain = expr_to_dsp_chain(left, buses)?;
            let right_chain = expr_to_dsp_chain(right, buses)?;
            Ok(DspChain {
                nodes: vec![DspNode::SignalAdd { 
                    left: Box::new(left_chain),
                    right: Box::new(right_chain)
                }]
            })
        }
        
        Expr::Mul(left, right) => {
            let left_chain = expr_to_dsp_chain(left, buses)?;
            let right_chain = expr_to_dsp_chain(right, buses)?;
            Ok(DspChain {
                nodes: vec![DspNode::SignalMul { 
                    left: Box::new(left_chain),
                    right: Box::new(right_chain)
                }]
            })
        }
        
        Expr::Sub(left, right) => {
            let left_chain = expr_to_dsp_chain(left, buses)?;
            let right_chain = expr_to_dsp_chain(right, buses)?;
            Ok(DspChain {
                nodes: vec![DspNode::SignalSub { 
                    left: Box::new(left_chain),
                    right: Box::new(right_chain)
                }]
            })
        }
        
        Expr::Div(left, right) => {
            let left_chain = expr_to_dsp_chain(left, buses)?;
            let right_chain = expr_to_dsp_chain(right, buses)?;
            Ok(DspChain {
                nodes: vec![DspNode::SignalDiv { 
                    left: Box::new(left_chain),
                    right: Box::new(right_chain)
                }]
            })
        }
    }
}

/// Helper to extract numeric arguments
fn get_number_arg(args: &[Expr], index: usize, default: f64) -> Result<f64, String> {
    match args.get(index) {
        Some(Expr::Number(n)) => Ok(*n),
        None => Ok(default),
        Some(expr) => Err(format!("Expected number, got {:?}", expr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number("42"), Ok(("", 42.0)));
        assert_eq!(parse_number("3.14"), Ok(("", 3.14)));
        assert_eq!(parse_number("440.0 "), Ok((" ", 440.0)));
    }
    
    #[test]
    fn test_parse_bus_ref() {
        assert_eq!(
            parse_bus_ref("~lfo"),
            Ok(("", Expr::Bus("lfo".to_string())))
        );
        assert_eq!(
            parse_bus_ref("~bass_env"),
            Ok(("", Expr::Bus("bass_env".to_string())))
        );
    }
    
    #[test]
    fn test_parse_pattern() {
        assert_eq!(
            parse_pattern(r#"s "bd sn""#),
            Ok(("", Expr::Pattern("bd sn".to_string())))
        );
        assert_eq!(
            parse_pattern(r#""hh*16""#),
            Ok(("", Expr::Pattern("hh*16".to_string())))
        );
    }
    
    #[test]
    fn test_parse_node() {
        // When parsing just the node, remaining input should have the argument
        let (rest, expr) = parse_node("sin 440").unwrap();
        assert_eq!(rest, "");  // Node parsing should consume arguments
        match expr {
            Expr::Node(name, args) => {
                assert_eq!(name, "sin");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], Expr::Number(440.0));
            }
            _ => panic!("Expected Node")
        }
    }
    
    #[test]
    fn test_parse_chain() {
        let (rest, expr) = parse_chain("sin 440 >> mul 0.5").unwrap();
        assert_eq!(rest, "");
        match expr {
            Expr::Chain(left, right) => {
                match left.as_ref() {
                    Expr::Node(name, _) => assert_eq!(name, "sin"),
                    _ => panic!("Expected sin node")
                }
                match right.as_ref() {
                    Expr::Node(name, _) => assert_eq!(name, "mul"),
                    _ => panic!("Expected mul node")
                }
            }
            _ => panic!("Expected Chain")
        }
    }
    
    #[test]
    fn test_parse_arithmetic() {
        let (rest, expr) = parse_expr("~lfo * 2000 + 500").unwrap();
        assert_eq!(rest, "");
        match expr {
            Expr::Add(left, right) => {
                match left.as_ref() {
                    Expr::Mul(_, _) => {}
                    _ => panic!("Expected multiplication")
                }
                match right.as_ref() {
                    Expr::Number(500.0) => {}
                    _ => panic!("Expected 500")
                }
            }
            _ => panic!("Expected Add")
        }
    }
    
    #[test]
    fn test_parse_complete_dsl() {
        let code = r#"
            ~lfo: sin 0.5 >> mul 0.5 >> add 0.5
            ~bass: saw 55 >> lpf 1000 0.8
            o: ~bass >> mul 0.4
        "#;
        
        let env = parse_dsl(code).unwrap();
        assert!(env.ref_chains.contains_key("lfo"));
        assert!(env.ref_chains.contains_key("bass"));
        assert!(env.output_chain.is_some());
    }
    
    #[test]
    fn test_parse_pattern_transforms() {
        // Test basic transforms
        let (rest, transform) = parse_pattern_transform("fast 2").unwrap();
        assert_eq!(rest, "");
        assert_eq!(transform, PatternTransform::Fast(2.0));
        
        let (rest, transform) = parse_pattern_transform("slow 0.5").unwrap();
        assert_eq!(rest, "");
        assert_eq!(transform, PatternTransform::Slow(0.5));
        
        let (rest, transform) = parse_pattern_transform("rev").unwrap();
        assert_eq!(rest, "");
        assert_eq!(transform, PatternTransform::Rev);
        
        let (rest, transform) = parse_pattern_transform("rotate 0.25").unwrap();
        assert_eq!(rest, "");
        assert_eq!(transform, PatternTransform::Rotate(0.25));
        
        // Test transforms with nested transforms
        let (rest, transform) = parse_pattern_transform("every 3 rev").unwrap();
        assert_eq!(rest, "");
        match transform {
            PatternTransform::Every(n, boxed) => {
                assert_eq!(n, 3);
                assert_eq!(*boxed, PatternTransform::Rev);
            }
            _ => panic!("Expected Every transform")
        }
    }
    
    #[test]
    fn test_parse_pattern_ops() {
        // Single pattern transform
        let (rest, expr) = parse_pattern_ops(r#""bd sn" |> fast 2"#).unwrap();
        assert_eq!(rest, "");
        match expr {
            Expr::PatternOp(base, transform) => {
                match base.as_ref() {
                    Expr::Pattern(p) => assert_eq!(p, "bd sn"),
                    _ => panic!("Expected Pattern")
                }
                assert_eq!(transform, PatternTransform::Fast(2.0));
            }
            _ => panic!("Expected PatternOp")
        }
        
        // Multiple pattern transforms
        let (rest, expr) = parse_pattern_ops(r#""bd sn" |> fast 2 |> rev"#).unwrap();
        assert_eq!(rest, "");
        match expr {
            Expr::PatternOp(base, transform) => {
                assert_eq!(transform, PatternTransform::Rev);
                match base.as_ref() {
                    Expr::PatternOp(inner_base, inner_transform) => {
                        assert_eq!(inner_transform, &PatternTransform::Fast(2.0));
                        match inner_base.as_ref() {
                            Expr::Pattern(p) => assert_eq!(p, "bd sn"),
                            _ => panic!("Expected Pattern at base")
                        }
                    }
                    _ => panic!("Expected nested PatternOp")
                }
            }
            _ => panic!("Expected PatternOp")
        }
    }
    
    #[test]
    fn test_pattern_then_dsp() {
        // Pattern with transforms followed by DSP chain
        let (rest, expr) = parse_expr(r#""bd*4" |> fast 2 >> lpf 1000 0.8"#).unwrap();
        assert_eq!(rest, "");
        
        // Should parse as: ("bd*4" |> fast 2) >> lpf 1000 0.8
        match expr {
            Expr::Chain(left, right) => {
                // Left should be pattern with transform
                match left.as_ref() {
                    Expr::PatternOp(base, transform) => {
                        match base.as_ref() {
                            Expr::Pattern(p) => assert_eq!(p, "bd*4"),
                            _ => panic!("Expected Pattern")
                        }
                        assert_eq!(transform, &PatternTransform::Fast(2.0));
                    }
                    _ => panic!("Expected PatternOp on left")
                }
                // Right should be DSP node
                match right.as_ref() {
                    Expr::Node(name, args) => {
                        assert_eq!(name, "lpf");
                        assert_eq!(args.len(), 2);
                    }
                    _ => panic!("Expected Node on right")
                }
            }
            _ => panic!("Expected Chain at top level")
        }
    }
    
    #[test]
    fn test_complex_pattern_operations() {
        // Test every with nested transform
        let code = r#""bd sn hh cp" |> every 3 rev |> fast 2"#;
        let (rest, expr) = parse_expr(code).unwrap();
        assert_eq!(rest, "");
        
        // Test scale transform
        let (rest, transform) = parse_pattern_transform(r#"scale "minor""#).unwrap();
        assert_eq!(rest, "");
        assert_eq!(transform, PatternTransform::Scale("minor".to_string()));
        
        // Test degradeBy
        let (rest, transform) = parse_pattern_transform("degradeBy 0.3").unwrap();
        assert_eq!(rest, "");
        assert_eq!(transform, PatternTransform::DegradeBy(0.3));
        
        // Test chunk
        let (rest, transform) = parse_pattern_transform("chunk 4 rev").unwrap();
        assert_eq!(rest, "");
        match transform {
            PatternTransform::Chunk(n, boxed) => {
                assert_eq!(n, 4);
                assert_eq!(*boxed, PatternTransform::Rev);
            }
            _ => panic!("Expected Chunk transform")
        }
    }
    
    #[test]
    fn test_parse_complete_with_patterns() {
        let code = r#"
            ~drums: "bd sn hh cp" |> fast 2 |> every 4 rev
            ~melody: "0 3 7 10" |> slow 2 |> scale "minor"
            ~bass: "0 0 12 7" |> slow 4
            o: ~drums >> mul 0.8
        "#;
        
        let env = parse_dsl(code).unwrap();
        assert!(env.ref_chains.contains_key("drums"));
        assert!(env.ref_chains.contains_key("melody"));
        assert!(env.ref_chains.contains_key("bass"));
        assert!(env.output_chain.is_some());
    }
    
    #[test]
    fn test_parsing_speed() {
        use std::time::Instant;
        
        let code = r#"
            ~lfo: sin 0.5 >> mul 0.5 >> add 0.5
            ~env: sin 2 >> mul 0.3 >> add 0.7
            ~bass: saw 55 >> lpf ~lfo * 2000 + 500 0.8
            ~lead: square 220 >> hpf ~env * 3000 + 1000 0.6
            ~drums: s "bd sn hh cp" >> mul 0.6
            o: ~bass * 0.4 + ~lead * 0.3 + ~drums
        "#;
        
        let start = Instant::now();
        let iterations = 10000;
        
        for _ in 0..iterations {
            let _ = parse_dsl(code);
        }
        
        let elapsed = start.elapsed();
        let per_parse = elapsed / iterations;
        
        println!("Nom parser: {} iterations in {:?}", iterations, elapsed);
        println!("Average: {:?} per parse", per_parse);
        println!("Throughput: {:.0} parses/second", 1.0 / per_parse.as_secs_f64());
        
        // Should be well under 1ms per parse
        assert!(per_parse.as_micros() < 1000);
    }
}