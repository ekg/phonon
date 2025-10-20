//! Compositional parser for Phonon DSL
//!
//! This parser provides full compositionality:
//! - Patterns are first-class expressions
//! - Audio chains are first-class expressions
//! - All operators work uniformly across expression types
//! - No special-casing based on context

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, space0},
    combinator::{map, opt, recognize, value},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};
use std::collections::HashMap;

// ============================================================================
// AST - Clean expression types with no special cases
// ============================================================================

/// Top-level statement in a Phonon program
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Bus assignment: ~name: expr
    BusAssignment { name: String, expr: Expr },
    /// Output: out: expr
    Output(Expr),
    /// Multi-channel output: out1: expr, out2: expr, etc.
    OutputChannel { channel: usize, expr: Expr },
    /// Tempo: cps: 2.0 or tempo: 120
    Tempo(f64),
}

/// Expression - the core of the language
/// All expressions are first-class and composable
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ========== Literals ==========
    /// Number literal: 440, 2.5, -1.0
    Number(f64),

    /// String literal (mini-notation pattern): "bd sn hh cp"
    String(String),

    // ========== References ==========
    /// Bus reference: ~drums, ~lfo
    BusRef(String),

    // ========== Function calls ==========
    /// Function call: lpf(input, cutoff, q), sine(440)
    Call { name: String, args: Vec<Expr> },

    // ========== Operators (all first-class!) ==========
    /// Chain operator: a # b (pipe a into b)
    Chain(Box<Expr>, Box<Expr>),

    /// Transform operator: pattern $ transform
    Transform {
        expr: Box<Expr>,
        transform: Transform,
    },

    /// Binary operators: +, -, *, /
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operators: -, !
    UnOp { op: UnOp, expr: Box<Expr> },

    /// Parenthesized expression (for grouping)
    Paren(Box<Expr>),

    /// List literal: [expr1, expr2, ...]
    List(Vec<Expr>),

    /// Chain input marker (only used internally by compiler for # operator)
    /// This is NOT parsed from source code - only created during compilation
    ChainInput(crate::unified_graph::NodeId),
}

/// Pattern transform operations
#[derive(Debug, Clone, PartialEq)]
pub enum Transform {
    /// fast n: speed up by factor n
    Fast(Box<Expr>),
    /// slow n: slow down by factor n
    Slow(Box<Expr>),
    /// rev: reverse pattern
    Rev,
    /// every n f: apply transform f every n cycles
    Every {
        n: Box<Expr>,
        transform: Box<Transform>,
    },
    /// degrade: randomly remove events
    Degrade,
    /// degradeBy p: remove events with probability p
    DegradeBy(Box<Expr>),
    /// stutter n: repeat each event n times
    Stutter(Box<Expr>),
    /// palindrome: pattern followed by its reverse
    Palindrome,
    /// shuffle amount: randomly shift events in time
    Shuffle(Box<Expr>),
    /// chop n: slice pattern into n equal parts
    Chop(Box<Expr>),
    /// striate n: alias for chop
    Striate(Box<Expr>),
    /// scramble n: Fisher-Yates shuffle of events
    Scramble(Box<Expr>),
    /// swing amount: add swing feel
    Swing(Box<Expr>),
    /// legato factor: adjust event duration (longer)
    Legato(Box<Expr>),
    /// staccato factor: make events shorter
    Staccato(Box<Expr>),
    /// echo times time feedback: echo/delay effect on pattern
    Echo {
        times: Box<Expr>,
        time: Box<Expr>,
        feedback: Box<Expr>,
    },
    /// segment n: divide pattern into n segments
    Segment(Box<Expr>),
    /// zoom begin end: focus on specific time range
    Zoom {
        begin: Box<Expr>,
        end: Box<Expr>,
    },
    /// compress begin end: compress pattern to time range
    Compress {
        begin: Box<Expr>,
        end: Box<Expr>,
    },
    /// spin n: rotate through n different versions
    Spin(Box<Expr>),
    /// mirror: palindrome within cycle (alias for palindrome)
    Mirror,
    /// gap n: insert silence every n cycles
    Gap(Box<Expr>),
    /// late amount: delay pattern in time
    Late(Box<Expr>),
    /// early amount: shift pattern earlier in time
    Early(Box<Expr>),
    /// dup n: duplicate pattern n times (like bd*n)
    Dup(Box<Expr>),
    /// fit n: fit pattern to n cycles
    Fit(Box<Expr>),
    /// stretch: sustain notes to fill gaps (legato 1.0)
    Stretch,
    /// rotL n: rotate pattern left by n steps
    RotL(Box<Expr>),
    /// rotR n: rotate pattern right by n steps
    RotR(Box<Expr>),
    /// iter n: iterate pattern shifting by 1/n each cycle
    Iter(Box<Expr>),
    /// iterBack n: iterate pattern backwards
    IterBack(Box<Expr>),
    /// ply n: repeat each event n times
    Ply(Box<Expr>),
    /// linger factor: linger on values for longer
    Linger(Box<Expr>),
    /// offset amount: shift pattern in time (alias for late)
    Offset(Box<Expr>),
    /// loop n: loop pattern n times within cycle
    Loop(Box<Expr>),
    /// chew n: chew through pattern
    Chew(Box<Expr>),
    /// fastGap factor: fast with gaps
    FastGap(Box<Expr>),
    /// discretise n: quantize time
    Discretise(Box<Expr>),
    /// compressGap begin end: compress to range with gaps
    CompressGap {
        begin: Box<Expr>,
        end: Box<Expr>,
    },
    /// reset cycles: restart pattern every n cycles
    Reset(Box<Expr>),
    /// restart n: restart pattern every n cycles (alias for reset)
    Restart(Box<Expr>),
    /// loopback: play backwards then forwards
    Loopback,
    /// binary n: bit mask pattern
    Binary(Box<Expr>),
    /// range min max: scale numeric values to range (numeric patterns only)
    Range {
        min: Box<Expr>,
        max: Box<Expr>,
    },
    /// quantize steps: quantize numeric values (numeric patterns only)
    Quantize(Box<Expr>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
}

// ============================================================================
// Parser - Proper precedence climbing
// ============================================================================

/// Skip optional whitespace and comments
fn skip_space_and_comments(input: &str) -> IResult<&str, ()> {
    let mut current = input;

    loop {
        let start_len = current.len();

        // Skip whitespace
        if let Ok((rest, _)) =
            take_while1::<_, _, nom::error::Error<&str>>(|c: char| c.is_whitespace())(current)
        {
            current = rest;
        }

        // Skip comments
        if let Ok((rest, _)) = parse_comment(current) {
            current = rest;
        }

        // If nothing was consumed, we're done
        if current.len() == start_len {
            break;
        }
    }

    Ok((current, ()))
}

/// Parse a complete Phonon program
pub fn parse_program(input: &str) -> IResult<&str, Vec<Statement>> {
    let (input, _) = skip_space_and_comments(input)?;
    let (input, statements) = separated_list0(multispace1, parse_statement)(input)?;
    let (input, _) = skip_space_and_comments(input)?;
    Ok((input, statements))
}

/// Parse a single statement
fn parse_statement(input: &str) -> IResult<&str, Statement> {
    // Try to parse each statement type
    alt((
        parse_bus_assignment,
        parse_output_channel, // Try multi-channel output first
        parse_output,         // Then single output
        parse_tempo,
    ))(input)
}

/// Parse bus assignment: ~name: expr
fn parse_bus_assignment(input: &str) -> IResult<&str, Statement> {
    let (input, _) = char('~')(input)?;
    let (input, name) = parse_identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, expr) = parse_expr(input)?;

    Ok((
        input,
        Statement::BusAssignment {
            name: name.to_string(),
            expr,
        },
    ))
}

/// Parse multi-channel output: out1: expr, out2: expr, etc.
fn parse_output_channel(input: &str) -> IResult<&str, Statement> {
    let (input, _) = tag("out")(input)?;
    let (input, channel_str) = digit1(input)?;
    let channel: usize = channel_str.parse().unwrap();
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, expr) = parse_expr(input)?;

    Ok((input, Statement::OutputChannel { channel, expr }))
}

/// Parse output: out: expr
fn parse_output(input: &str) -> IResult<&str, Statement> {
    let (input, _) = tag("out")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, expr) = parse_expr(input)?;

    Ok((input, Statement::Output(expr)))
}

/// Parse tempo: cps: 2.0 or tempo: 120
fn parse_tempo(input: &str) -> IResult<&str, Statement> {
    let (input, _) = alt((tag("cps"), tag("tempo")))(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = parse_number(input)?;

    Ok((input, Statement::Tempo(value)))
}

// ============================================================================
// Expression parsing with proper precedence
// ============================================================================

/// Parse an expression (entry point)
/// Precedence (lowest to highest):
/// 1. # (chain)
/// 2. $ (transform)
/// 3. +, - (add, sub)
/// 4. *, / (mul, div)
/// 5. unary -, !
/// 6. function calls, parentheses, literals
pub fn parse_expr(input: &str) -> IResult<&str, Expr> {
    parse_chain_expr(input)
}

/// Parse chain expression: expr # expr
fn parse_chain_expr(input: &str) -> IResult<&str, Expr> {
    let (input, mut expr) = parse_transform_expr(input)?;

    // Parse any number of chains (left-associative)
    let mut current_input = input;
    loop {
        let (input, _) = space0(current_input)?;

        // Try to parse chain operator
        if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('#')(input) {
            let (input, _) = space0(input)?;
            let (input, right) = parse_transform_expr(input)?;

            expr = Expr::Chain(Box::new(expr), Box::new(right));
            current_input = input;
        } else {
            break;
        }
    }

    Ok((current_input, expr))
}

/// Parse transform expression: expr $ transform
fn parse_transform_expr(input: &str) -> IResult<&str, Expr> {
    let (input, mut expr) = parse_additive_expr(input)?;

    // Parse any number of transforms
    let mut current_input = input;
    loop {
        let (input, _) = space0(current_input)?;

        // Try to parse a transform operator
        if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('$')(input) {
            let (input, _) = space0(input)?;
            let (input, transform) = parse_transform(input)?;

            expr = Expr::Transform {
                expr: Box::new(expr),
                transform,
            };
            current_input = input;
        } else {
            break;
        }
    }

    Ok((current_input, expr))
}

/// Parse additive expression: expr + expr | expr - expr
fn parse_additive_expr(input: &str) -> IResult<&str, Expr> {
    let (input, mut expr) = parse_multiplicative_expr(input)?;

    let mut current_input = input;
    loop {
        let (input, _) = space0(current_input)?;

        // Try to parse + or -
        let op = if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('+')(input) {
            Some((input, BinOp::Add))
        } else if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('-')(input) {
            Some((input, BinOp::Sub))
        } else {
            None
        };

        if let Some((input, op)) = op {
            let (input, _) = space0(input)?;
            let (input, right) = parse_multiplicative_expr(input)?;

            expr = Expr::BinOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
            current_input = input;
        } else {
            break;
        }
    }

    Ok((current_input, expr))
}

/// Parse multiplicative expression: expr * expr | expr / expr
fn parse_multiplicative_expr(input: &str) -> IResult<&str, Expr> {
    let (input, mut expr) = parse_unary_expr(input)?;

    let mut current_input = input;
    loop {
        let (input, _) = space0(current_input)?;

        // Try to parse * or /
        let op = if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('*')(input) {
            Some((input, BinOp::Mul))
        } else if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('/')(input) {
            Some((input, BinOp::Div))
        } else {
            None
        };

        if let Some((input, op)) = op {
            let (input, _) = space0(input)?;
            let (input, right) = parse_unary_expr(input)?;

            expr = Expr::BinOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
            current_input = input;
        } else {
            break;
        }
    }

    Ok((current_input, expr))
}

/// Parse unary expression: -expr
fn parse_unary_expr(input: &str) -> IResult<&str, Expr> {
    // Try unary minus
    if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('-')(input) {
        let (input, _) = space0(input)?;
        let (input, expr) = parse_primary_expr(input)?;
        Ok((
            input,
            Expr::UnOp {
                op: UnOp::Neg,
                expr: Box::new(expr),
            },
        ))
    } else {
        parse_primary_expr(input)
    }
}

/// Parse primary expression: number, string, bus ref, function call, parentheses, list
fn parse_primary_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = space0(input)?;

    alt((
        map(parse_number, Expr::Number),
        parse_string_literal,
        parse_bus_ref_expr,
        parse_function_call,
        parse_list_expr,
        parse_paren_expr,
    ))(input)
}

/// Parse parenthesized expression: (expr)
fn parse_paren_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('(')(input)?;
    let (input, _) = space0(input)?;
    let (input, expr) = parse_expr(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, Expr::Paren(Box::new(expr))))
}

/// Parse list expression: [expr1, expr2, ...]
fn parse_list_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('[')(input)?;
    let (input, _) = space0(input)?;

    // Parse comma-separated expressions
    let (input, exprs) = separated_list0(
        delimited(space0, char(','), space0),
        parse_expr
    )(input)?;

    let (input, _) = space0(input)?;
    let (input, _) = char(']')(input)?;

    Ok((input, Expr::List(exprs)))
}

/// Parse bus reference: ~name
fn parse_bus_ref_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('~')(input)?;
    let (input, name) = parse_identifier(input)?;
    Ok((input, Expr::BusRef(name.to_string())))
}

/// Parse function call: name arg1 arg2 ...
/// ONLY space-separated syntax is supported (no parentheses/commas)
fn parse_function_call(input: &str) -> IResult<&str, Expr> {
    let (input, name) = parse_identifier(input)?;

    // Use hspace1 to prevent consuming newlines (which would eat next statement)
    match hspace1(input) {
        Ok((input, _)) => {
            // Parse first argument
            let (input, first_arg) = parse_primary_expr(input)?;

            // Parse remaining space-separated arguments (using hspace1!)
            let (input, mut rest_args) = many0(preceded(hspace1, parse_primary_expr))(input)?;

            // Combine all args
            let mut args = vec![first_arg];
            args.append(&mut rest_args);

            Ok((
                input,
                Expr::Call {
                    name: name.to_string(),
                    args,
                },
            ))
        }
        Err(_) => {
            // Just an identifier with no arguments - invalid function call
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    }
}

/// Parse a transform
fn parse_transform(input: &str) -> IResult<&str, Transform> {
    // Split into three alt() groups due to nom's tuple limit
    alt((
        parse_transform_group_1,
        parse_transform_group_2,
        parse_transform_group_3,
    ))(input)
}

/// Parse transform group 1 (first half of transforms)
fn parse_transform_group_1(input: &str) -> IResult<&str, Transform> {
    alt((
        // every n transform (MUST come first - recursive)
        map(
            tuple((
                terminated(tag("every"), space1),
                terminated(parse_primary_expr, space1),
                parse_transform,
            )),
            |(_, n, transform)| Transform::Every {
                n: Box::new(n),
                transform: Box::new(transform),
            },
        ),
        // fast n
        map(
            preceded(terminated(tag("fast"), space1), parse_primary_expr),
            |expr| Transform::Fast(Box::new(expr)),
        ),
        // slow n
        map(
            preceded(terminated(tag("slow"), space1), parse_primary_expr),
            |expr| Transform::Slow(Box::new(expr)),
        ),
        // rev
        value(Transform::Rev, tag("rev")),
        // degradeBy p (MUST come before degrade!)
        map(
            preceded(terminated(tag("degradeBy"), space1), parse_primary_expr),
            |expr| Transform::DegradeBy(Box::new(expr)),
        ),
        // degrade
        value(Transform::Degrade, tag("degrade")),
        // stutter n
        map(
            preceded(terminated(tag("stutter"), space1), parse_primary_expr),
            |expr| Transform::Stutter(Box::new(expr)),
        ),
        // palindrome
        value(Transform::Palindrome, tag("palindrome")),
        // shuffle amount
        map(
            preceded(terminated(tag("shuffle"), space1), parse_primary_expr),
            |expr| Transform::Shuffle(Box::new(expr)),
        ),
        // chop n
        map(
            preceded(terminated(tag("chop"), space1), parse_primary_expr),
            |expr| Transform::Chop(Box::new(expr)),
        ),
        // striate n
        map(
            preceded(terminated(tag("striate"), space1), parse_primary_expr),
            |expr| Transform::Striate(Box::new(expr)),
        ),
        // scramble n
        map(
            preceded(terminated(tag("scramble"), space1), parse_primary_expr),
            |expr| Transform::Scramble(Box::new(expr)),
        ),
        // swing amount
        map(
            preceded(terminated(tag("swing"), space1), parse_primary_expr),
            |expr| Transform::Swing(Box::new(expr)),
        ),
        // legato factor
        map(
            preceded(terminated(tag("legato"), space1), parse_primary_expr),
            |expr| Transform::Legato(Box::new(expr)),
        ),
        // staccato factor (MUST come before striate!)
        map(
            preceded(terminated(tag("staccato"), space1), parse_primary_expr),
            |expr| Transform::Staccato(Box::new(expr)),
        ),
    ))(input)
}

/// Parse transform group 2 (second half of transforms)
fn parse_transform_group_2(input: &str) -> IResult<&str, Transform> {
    alt((
        // echo times time feedback
        map(
            tuple((
                terminated(tag("echo"), space1),
                terminated(parse_primary_expr, space1),
                terminated(parse_primary_expr, space1),
                parse_primary_expr,
            )),
            |(_, times, time, feedback)| Transform::Echo {
                times: Box::new(times),
                time: Box::new(time),
                feedback: Box::new(feedback),
            },
        ),
        // segment n
        map(
            preceded(terminated(tag("segment"), space1), parse_primary_expr),
            |expr| Transform::Segment(Box::new(expr)),
        ),
        // zoom begin end
        map(
            tuple((
                terminated(tag("zoom"), space1),
                terminated(parse_primary_expr, space1),
                parse_primary_expr,
            )),
            |(_, begin, end)| Transform::Zoom {
                begin: Box::new(begin),
                end: Box::new(end),
            },
        ),
        // compress begin end
        map(
            tuple((
                terminated(tag("compress"), space1),
                terminated(parse_primary_expr, space1),
                parse_primary_expr,
            )),
            |(_, begin, end)| Transform::Compress {
                begin: Box::new(begin),
                end: Box::new(end),
            },
        ),
        // spin n
        map(
            preceded(terminated(tag("spin"), space1), parse_primary_expr),
            |expr| Transform::Spin(Box::new(expr)),
        ),
        // mirror
        value(Transform::Mirror, tag("mirror")),
        // gap n
        map(
            preceded(terminated(tag("gap"), space1), parse_primary_expr),
            |expr| Transform::Gap(Box::new(expr)),
        ),
        // late amount
        map(
            preceded(terminated(tag("late"), space1), parse_primary_expr),
            |expr| Transform::Late(Box::new(expr)),
        ),
        // early amount
        map(
            preceded(terminated(tag("early"), space1), parse_primary_expr),
            |expr| Transform::Early(Box::new(expr)),
        ),
        // dup n
        map(
            preceded(terminated(tag("dup"), space1), parse_primary_expr),
            |expr| Transform::Dup(Box::new(expr)),
        ),
        // fit n
        map(
            preceded(terminated(tag("fit"), space1), parse_primary_expr),
            |expr| Transform::Fit(Box::new(expr)),
        ),
        // stretch
        value(Transform::Stretch, tag("stretch")),
        // rotL n
        map(
            preceded(terminated(tag("rotL"), space1), parse_primary_expr),
            |expr| Transform::RotL(Box::new(expr)),
        ),
        // rotR n
        map(
            preceded(terminated(tag("rotR"), space1), parse_primary_expr),
            |expr| Transform::RotR(Box::new(expr)),
        ),
        // iter n
        map(
            preceded(terminated(tag("iter"), space1), parse_primary_expr),
            |expr| Transform::Iter(Box::new(expr)),
        ),
        // iterBack n
        map(
            preceded(terminated(tag("iterBack"), space1), parse_primary_expr),
            |expr| Transform::IterBack(Box::new(expr)),
        ),
        // ply n
        map(
            preceded(terminated(tag("ply"), space1), parse_primary_expr),
            |expr| Transform::Ply(Box::new(expr)),
        ),
        // linger factor
        map(
            preceded(terminated(tag("linger"), space1), parse_primary_expr),
            |expr| Transform::Linger(Box::new(expr)),
        ),
    ))(input)
}

/// Parse transform group 3 (third group of transforms)
fn parse_transform_group_3(input: &str) -> IResult<&str, Transform> {
    alt((
        // offset amount
        map(
            preceded(terminated(tag("offset"), space1), parse_primary_expr),
            |expr| Transform::Offset(Box::new(expr)),
        ),
        // loop n
        map(
            preceded(terminated(tag("loop"), space1), parse_primary_expr),
            |expr| Transform::Loop(Box::new(expr)),
        ),
        // chew n
        map(
            preceded(terminated(tag("chew"), space1), parse_primary_expr),
            |expr| Transform::Chew(Box::new(expr)),
        ),
        // fastGap factor
        map(
            preceded(terminated(tag("fastGap"), space1), parse_primary_expr),
            |expr| Transform::FastGap(Box::new(expr)),
        ),
        // discretise n
        map(
            preceded(terminated(tag("discretise"), space1), parse_primary_expr),
            |expr| Transform::Discretise(Box::new(expr)),
        ),
        // compressGap begin end
        map(
            tuple((
                terminated(tag("compressGap"), space1),
                terminated(parse_primary_expr, space1),
                parse_primary_expr,
            )),
            |(_, begin, end)| Transform::CompressGap {
                begin: Box::new(begin),
                end: Box::new(end),
            },
        ),
        // restart n (MUST come before reset!)
        map(
            preceded(terminated(tag("restart"), space1), parse_primary_expr),
            |expr| Transform::Restart(Box::new(expr)),
        ),
        // reset cycles
        map(
            preceded(terminated(tag("reset"), space1), parse_primary_expr),
            |expr| Transform::Reset(Box::new(expr)),
        ),
        // loopback
        value(Transform::Loopback, tag("loopback")),
        // binary n
        map(
            preceded(terminated(tag("binary"), space1), parse_primary_expr),
            |expr| Transform::Binary(Box::new(expr)),
        ),
        // quantize steps (MUST come before range!)
        map(
            preceded(terminated(tag("quantize"), space1), parse_primary_expr),
            |expr| Transform::Quantize(Box::new(expr)),
        ),
        // range min max
        map(
            tuple((
                terminated(tag("range"), space1),
                terminated(parse_primary_expr, space1),
                parse_primary_expr,
            )),
            |(_, min, max)| Transform::Range {
                min: Box::new(min),
                max: Box::new(max),
            },
        ),
    ))(input)
}

// ============================================================================
// Lexical parsers
// ============================================================================

/// Parse identifier: alphanumeric starting with letter
fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)
}

/// Parse number: integer or float
fn parse_number(input: &str) -> IResult<&str, f64> {
    let (input, sign) = opt(char('-'))(input)?;
    let (input, int_part) = digit1(input)?;
    let (input, frac_part) = opt(preceded(char('.'), digit1))(input)?;

    let num_str = if let Some(frac) = frac_part {
        format!("{}.{}", int_part, frac)
    } else {
        int_part.to_string()
    };

    let mut value: f64 = num_str.parse().unwrap();
    if sign.is_some() {
        value = -value;
    }

    Ok((input, value))
}

/// Parse string literal: "..."
fn parse_string_literal(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('"')(input)?;
    let (input, content) = take_until("\"")(input)?;
    let (input, _) = char('"')(input)?;

    Ok((input, Expr::String(content.to_string())))
}

/// Parse whitespace (at least one space/newline)
fn space1(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_whitespace())(input)
}

/// Parse horizontal whitespace only (no newlines)
/// Used for function call arguments to prevent consuming next statement
fn hspace1(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c == ' ' || c == '\t')(input)
}

/// Parse a comment line: # at start of line until newline
fn parse_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = char('#')(input)?;
    let (input, _) = take_until("\n")(input)?;
    let (input, _) = char('\n')(input)?;
    Ok((input, ()))
}

/// Parse whitespace and comments
/// This is used between statements
fn space_and_comments(input: &str) -> IResult<&str, ()> {
    let mut current = input;

    loop {
        // Try to skip whitespace
        if let Ok((rest, _)) =
            take_while1::<_, _, nom::error::Error<&str>>(|c: char| c.is_whitespace())(current)
        {
            current = rest;
            continue;
        }

        // Try to skip comment (only if # is at start of line context)
        // We need to peek ahead to check if this is a comment or chain operator
        // A comment starts with optional whitespace, then #
        // But we only want to consume it if there's whitespace or start of input before it
        if let Ok((rest, _)) = parse_comment(current) {
            current = rest;
            continue;
        }

        break;
    }

    if current != input {
        Ok((current, ()))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Space,
        )))
    }
}

/// Parse multispace (at least one space/newline), skipping comments
fn multispace1(input: &str) -> IResult<&str, &str> {
    let start = input;

    // Skip whitespace and comments
    let (input, _) = space_and_comments(input)?;

    // Return the consumed part
    let consumed = &start[..start.len() - input.len()];
    Ok((input, consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number("42"), Ok(("", 42.0)));
        assert_eq!(parse_number("3.14"), Ok(("", 3.14)));
        assert_eq!(parse_number("-1.5"), Ok(("", -1.5)));
    }

    #[test]
    fn test_parse_string() {
        let result = parse_string_literal("\"bd sn hh cp\"");
        assert!(result.is_ok());
        if let Ok((_, Expr::String(s))) = result {
            assert_eq!(s, "bd sn hh cp");
        }
    }

    #[test]
    fn test_parse_bus_ref() {
        let result = parse_bus_ref_expr("~drums");
        assert!(result.is_ok());
        if let Ok((_, Expr::BusRef(name))) = result {
            assert_eq!(name, "drums");
        }
    }

    #[test]
    fn test_parse_function_call_space_separated() {
        let result = parse_function_call("lpf 500 0.8");
        println!("Function call result: {:?}", result);
        assert!(result.is_ok());
        if let Ok((_, Expr::Call { name, args })) = result {
            assert_eq!(name, "lpf");
            assert_eq!(args.len(), 2);
        }
    }

    #[test]
    fn test_parse_function_call_with_parens_should_fail() {
        // Parenthesized syntax is NOT supported - space-separated only!
        let result = parse_function_call("lpf(500, 0.8)");
        assert!(
            result.is_err(),
            "Parenthesized syntax should not be supported"
        );
    }

    #[test]
    fn test_parse_chain() {
        let result = parse_expr("s \"bd\" # lpf 500 0.8");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_transform() {
        let result = parse_expr("\"bd sn\" $ fast 2");
        assert!(result.is_ok());
        if let Ok((_, Expr::Transform { expr, transform })) = result {
            assert!(matches!(*expr, Expr::String(_)));
            assert!(matches!(transform, Transform::Fast(_)));
        }
    }

    #[test]
    fn test_parse_nested_transforms() {
        // This should work: pattern $ fast 2 $ rev
        let result = parse_expr("\"bd sn\" $ fast 2 $ rev");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_transform_in_chain() {
        // This should work: (pattern $ fast 2) # lpf 500 0.8
        let result = parse_expr("(\"bd sn\" $ fast 2) # lpf 500 0.8");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_bus_assignment() {
        let result = parse_statement("~drums: s \"bd sn hh cp\"");
        assert!(result.is_ok());
        if let Ok((_, Statement::BusAssignment { name, expr })) = result {
            assert_eq!(name, "drums");
        }
    }

    #[test]
    fn test_parse_bus_assignment_with_transform() {
        // This is the key test - transform on bus assignment!
        let result = parse_statement("~fast_drums: \"bd sn\" $ fast 2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pattern_bus_in_lpf() {
        // This should work: lpf with pattern bus
        let result = parse_expr("s \"hh\" # lpf ~cutoffs 0.8");
        assert!(result.is_ok());
    }

    // ========================================================================
    // COMPREHENSIVE AST TESTS - Operator Precedence and Nesting
    // ========================================================================

    #[test]
    fn test_operator_precedence_transform_vs_chain() {
        // $ has higher precedence than # (binds tighter)
        // "bd" $ fast 2 # lpf 500 0.8
        // should parse as: (("bd" $ fast 2) # (lpf 500 0.8))
        let result = parse_expr("\"bd\" $ fast 2 # lpf 500 0.8");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            match expr {
                Expr::Chain(left, _right) => {
                    // The left side should be a transform
                    match *left {
                        Expr::Transform { .. } => (), // Expected
                        _ => panic!("Expected transform on left side of chain"),
                    }
                }
                _ => panic!("Expected chain at top level"),
            }
        }
    }

    #[test]
    fn test_operator_precedence_chain_vs_add() {
        // # has lower precedence than +
        // a # b + c should parse as: a # (b + c)
        let result = parse_expr("~a # ~b + ~c");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            match expr {
                Expr::Chain(left, right) => {
                    // Right side should be addition
                    match *right {
                        Expr::BinOp { op: BinOp::Add, .. } => (), // Expected
                        _ => panic!("Expected addition on right side of chain"),
                    }
                }
                _ => panic!("Expected chain at top level"),
            }
        }
    }

    #[test]
    fn test_operator_precedence_add_vs_mul() {
        // * has higher precedence than +
        // a + b * c should parse as: a + (b * c)
        let result = parse_expr("1 + 2 * 3");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            match expr {
                Expr::BinOp {
                    op: BinOp::Add,
                    right,
                    ..
                } => {
                    match *right {
                        Expr::BinOp { op: BinOp::Mul, .. } => (), // Expected
                        _ => panic!("Expected multiplication on right side"),
                    }
                }
                _ => panic!("Expected addition at top level"),
            }
        }
    }

    #[test]
    fn test_nested_parentheses() {
        // ((a + b) * c) should preserve grouping
        let result = parse_expr("((1 + 2) * 3)");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            match expr {
                Expr::Paren(inner) => {
                    match *inner {
                        Expr::BinOp {
                            op: BinOp::Mul,
                            left,
                            ..
                        } => {
                            match *left {
                                Expr::Paren(_) => (), // Expected
                                _ => panic!("Expected nested paren"),
                            }
                        }
                        _ => panic!("Expected mul inside paren"),
                    }
                }
                _ => panic!("Expected paren at top level"),
            }
        }
    }

    #[test]
    fn test_stacked_transforms() {
        // Multiple transforms in sequence
        let result = parse_expr("\"bd sn\" $ fast 2 $ slow 0.5 $ rev");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            // Should be: Transform(Transform(Transform("bd sn", fast 2), slow 0.5), rev)
            match expr {
                Expr::Transform {
                    expr: inner,
                    transform,
                } => {
                    assert!(matches!(transform, Transform::Rev));
                    match *inner {
                        Expr::Transform { .. } => (), // Another transform inside
                        _ => panic!("Expected nested transform"),
                    }
                }
                _ => panic!("Expected transform at top level"),
            }
        }
    }

    #[test]
    fn test_transform_with_expression_arg() {
        // fast (2 + 1) should parse the expression as the argument
        let result = parse_expr("\"bd\" $ fast (2 + 1)");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            match expr {
                Expr::Transform { transform, .. } => {
                    match transform {
                        Transform::Fast(arg) => {
                            match *arg {
                                Expr::Paren(_) => (), // Expected
                                _ => panic!("Expected paren expression in fast arg"),
                            }
                        }
                        _ => panic!("Expected Fast transform"),
                    }
                }
                _ => panic!("Expected transform"),
            }
        }
    }

    #[test]
    fn test_chain_with_transforms() {
        // (pattern $ fast 2) # (lpf 500 0.8 $ slow 0.5)
        // With precedence # < $, the right side parses as a transform first
        let result = parse_expr("(\"bd\" $ fast 2) # lpf 500 0.8 $ slow 0.5");
        assert!(result.is_ok());
        if let Ok((_, expr)) = result {
            match expr {
                Expr::Chain(left, right) => {
                    // Left should be a transform (parenthesized)
                    match *left {
                        Expr::Paren(_) => (),
                        _ => panic!("Expected paren on left"),
                    }
                    // Right should be a transform
                    match *right {
                        Expr::Transform { .. } => (),
                        _ => panic!("Expected transform on right"),
                    }
                }
                _ => panic!("Expected chain at top level"),
            }
        }
    }

    #[test]
    fn test_complex_nesting() {
        // Really nest it!
        // ((a # b) $ fast 2) # ((c $ slow 3) # d) $ rev
        let result = parse_expr("((~a # ~b) $ fast 2) # ((~c $ slow 3) # ~d) $ rev");
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_function_args() {
        // lpf with 3 arguments
        let result = parse_expr("lpf 1000 0.8 ~lfo");
        assert!(result.is_ok());
        if let Ok((_, Expr::Call { args, .. })) = result {
            assert_eq!(args.len(), 3);
        }
    }

    #[test]
    fn test_nested_function_calls_should_fail() {
        // Parenthesized syntax is NOT supported
        // lpf(saw(110), 1000, 0.8) should fail
        let result = parse_expr("lpf(saw(110), 1000, 0.8)");
        assert!(
            result.is_err(),
            "Parenthesized nested calls should not be supported"
        );
    }

    #[test]
    fn test_bus_in_arithmetic() {
        // ~lfo * 2000 + 500
        let result = parse_expr("~lfo * 2000 + 500");
        assert!(result.is_ok());
        if let Ok((
            _,
            Expr::BinOp {
                op: BinOp::Add,
                left,
                ..
            },
        )) = result
        {
            // Left side should be multiplication
            match *left {
                Expr::BinOp {
                    op: BinOp::Mul,
                    left,
                    ..
                } => {
                    match *left {
                        Expr::BusRef(_) => (), // Expected
                        _ => panic!("Expected bus ref"),
                    }
                }
                _ => panic!("Expected mul on left"),
            }
        }
    }

    // ========================================================================
    // ALL TRANSFORM TYPES
    // ========================================================================

    #[test]
    fn test_all_transforms() {
        // Test each transform type
        let tests = vec![
            (
                "\"bd\" $ fast 2",
                Transform::Fast(Box::new(Expr::Number(2.0))),
            ),
            (
                "\"bd\" $ slow 2",
                Transform::Slow(Box::new(Expr::Number(2.0))),
            ),
            ("\"bd\" $ rev", Transform::Rev),
            ("\"bd\" $ degrade", Transform::Degrade),
            (
                "\"bd\" $ degradeBy 0.5",
                Transform::DegradeBy(Box::new(Expr::Number(0.5))),
            ),
            (
                "\"bd\" $ stutter 3",
                Transform::Stutter(Box::new(Expr::Number(3.0))),
            ),
            ("\"bd\" $ palindrome", Transform::Palindrome),
        ];

        for (code, expected_transform) in tests {
            let result = parse_expr(code);
            assert!(result.is_ok(), "Failed to parse: {}", code);
            if let Ok((_, Expr::Transform { transform, .. })) = result {
                assert_eq!(
                    transform, expected_transform,
                    "Transform mismatch for: {}",
                    code
                );
            } else {
                panic!("Expected Transform for: {}", code);
            }
        }
    }

    #[test]
    fn test_transform_with_bus_arg() {
        // fast ~speed where ~speed is a bus
        let result = parse_expr("\"bd\" $ fast ~speed");
        assert!(result.is_ok());
        if let Ok((_, Expr::Transform { transform, .. })) = result {
            match transform {
                Transform::Fast(arg) => match *arg {
                    Expr::BusRef(name) => assert_eq!(name, "speed"),
                    _ => panic!("Expected bus ref in fast arg"),
                },
                _ => panic!("Expected Fast transform"),
            }
        }
    }

    // ========================================================================
    // STATEMENT PARSING
    // ========================================================================

    #[test]
    fn test_parse_simple_program() {
        // Test with semicolons first
        let code = "~drums: s \"bd\"; ~filtered: ~drums";
        let result = parse_program(code);
        println!("Simple test result: {:?}", result);
        // We don't have semicolon support, so let's try newlines
        let code2 = "~drums: s \"bd\"\n~filtered: ~drums";
        let result2 = parse_program(code2);
        println!("Newline test result: {:?}", result2);
        if let Ok((rest, statements)) = result2 {
            println!("Statements: {}, Remaining: '{}'", statements.len(), rest);
        }
    }

    #[test]
    fn test_parse_program_multiple_statements() {
        let code = r#"
            ~drums: s "bd sn hh cp"
            ~filtered: ~drums # lpf 2000 0.8
            out: ~filtered $ fast 2
        "#;
        let result = parse_program(code);
        if result.is_err() {
            println!("Parse error: {:?}", result);
        }
        assert!(result.is_ok());
        if let Ok((rest, statements)) = result {
            println!("Remaining: '{}'", rest);
            println!("Statements: {}", statements.len());
            assert_eq!(statements.len(), 3);
        }
    }

    #[test]
    fn test_parse_tempo() {
        let result = parse_statement("cps: 2.5");
        assert!(result.is_ok());
        if let Ok((_, Statement::Tempo(val))) = result {
            assert_eq!(val, 2.5);
        }
    }

    #[test]
    fn test_parse_output() {
        let result = parse_statement("out: ~drums # reverb 0.5 0.7 0.3");
        assert!(result.is_ok());
        if let Ok((_, Statement::Output(_))) = result {
            // Success
        } else {
            panic!("Expected Output statement");
        }
    }

    // ========================================================================
    // EDGE CASES
    // ========================================================================

    #[test]
    fn test_empty_function_call_should_fail() {
        // Space-separated syntax requires at least one argument
        // noise() is invalid - should be just "noise" (identifier) or "noise arg"
        let result = parse_function_call("noise()");
        assert!(result.is_err(), "Empty parentheses should not be supported");
    }

    #[test]
    fn test_space_separated_chain() {
        // ONLY space-separated syntax is supported
        // s "bd" # lpf 1000 0.8 # reverb 0.5 0.7 0.3
        let result = parse_expr("s \"bd\" # lpf 1000 0.8 # reverb 0.5 0.7 0.3");
        assert!(result.is_ok(), "Space-separated chain should work");
    }

    #[test]
    fn test_unary_minus() {
        // -1.5
        let result = parse_expr("-1.5");
        assert!(result.is_ok());
        if let Ok((
            _,
            Expr::UnOp {
                op: UnOp::Neg,
                expr,
            },
        )) = result
        {
            match *expr {
                Expr::Number(n) => assert_eq!(n, 1.5),
                _ => panic!("Expected number"),
            }
        }
    }

    #[test]
    fn test_negative_number_in_pattern() {
        // "-1 0 1" should be a string literal, not expressions
        let result = parse_expr("\"-1 0 1\"");
        assert!(result.is_ok());
        if let Ok((_, Expr::String(s))) = result {
            assert_eq!(s, "-1 0 1");
        }
    }

    #[test]
    fn test_division() {
        // 10 / 2
        let result = parse_expr("10 / 2");
        assert!(result.is_ok());
        if let Ok((_, Expr::BinOp { op: BinOp::Div, .. })) = result {
            // Success
        }
    }

    #[test]
    fn test_subtraction() {
        // 10 - 5
        let result = parse_expr("10 - 5");
        assert!(result.is_ok());
        if let Ok((_, Expr::BinOp { op: BinOp::Sub, .. })) = result {
            // Success
        }
    }

    #[test]
    fn test_whitespace_handling() {
        // Test with various horizontal whitespace
        // Note: newlines NOT allowed between function name and args (they end statements)
        let tests = vec!["s \"bd\"", "s  \"bd\"", "s\t\"bd\""];

        for code in tests {
            let result = parse_expr(code);
            assert!(result.is_ok(), "Failed with whitespace: {:?}", code);
        }
    }

    #[test]
    fn test_real_world_example_1() {
        // Real-world pattern from user
        let code = "s \"hh*4 cp\" # lpf \"<300 200 1000>\" \"<0.8 0.6 0.2>\"";
        let result = parse_expr(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_real_world_example_2() {
        // Pattern with transform on bus
        let code = "~cutoffs: \"<300 200 1000>\" $ fast 2";
        let result = parse_statement(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_real_world_example_3() {
        // Complex chain with multiple effects
        let code = "s \"bd sn\" # lpf 500 0.8 # reverb 0.5 0.7 0.3 # distort 2.0";
        let result = parse_expr(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_real_world_example_4() {
        // LFO modulating filter with space-separated syntax
        let code = "saw 55 # lpf (~lfo * 2000 + 500) 0.8";
        let result = parse_expr(code);
        assert!(result.is_ok(), "Space-separated LFO modulation should work");
    }

    #[test]
    fn test_stack_should_work() {
        // The user mentioned "stack! should work"
        // Let's test deeply nested stacks of operations
        let deeply_nested = "((((~a # ~b) $ fast 2) # ~c) $ slow 0.5) # ((~d $ rev) # ~e)";
        let result = parse_expr(deeply_nested);
        assert!(result.is_ok(), "Deep nesting failed");

        // Stack of transforms
        let stacked_transforms = "\"bd\" $ fast 2 $ slow 3 $ rev $ palindrome $ degrade";
        let result = parse_expr(stacked_transforms);
        assert!(result.is_ok(), "Stacked transforms failed");

        // Stack of chains
        let stacked_chains = "~a # ~b # ~c # ~d # ~e # ~f";
        let result = parse_expr(stacked_chains);
        assert!(result.is_ok(), "Stacked chains failed");

        // Stack of arithmetic
        let stacked_arithmetic = "1 + 2 - 3 * 4 / 5 + 6";
        let result = parse_expr(stacked_arithmetic);
        assert!(result.is_ok(), "Stacked arithmetic failed");
    }

    // ========================================================================
    // COMMENT SUPPORT
    // ========================================================================

    #[test]
    fn test_comment_at_start_of_program() {
        let code = r#"# This is a comment
~drums: s "bd sn hh cp"
out: ~drums"#;
        let result = parse_program(code);
        assert!(
            result.is_ok(),
            "Failed to parse program with comment at start"
        );
        if let Ok((_, statements)) = result {
            assert_eq!(statements.len(), 2);
        }
    }

    #[test]
    fn test_comment_between_statements() {
        let code = r#"~drums: s "bd sn hh cp"
# This is a comment in the middle
out: ~drums"#;
        let result = parse_program(code);
        assert!(
            result.is_ok(),
            "Failed to parse program with comment between statements"
        );
        if let Ok((_, statements)) = result {
            assert_eq!(statements.len(), 2);
        }
    }

    #[test]
    fn test_multiple_comments() {
        let code = r#"# Comment 1
# Comment 2
~drums: s "bd sn hh cp"
# Comment 3
# Comment 4
out: ~drums
# Comment at end"#;
        let result = parse_program(code);
        assert!(
            result.is_ok(),
            "Failed to parse program with multiple comments"
        );
        if let Ok((_, statements)) = result {
            assert_eq!(statements.len(), 2);
        }
    }

    #[test]
    fn test_chain_operator_not_confused_with_comment() {
        // Make sure # as chain operator still works
        let code = "~drums: s \"bd sn\" # lpf 500 0.8";
        let result = parse_statement(code);
        assert!(result.is_ok(), "Chain operator # should still work");
    }

    #[test]
    fn test_complex_example_with_comments() {
        let code = r#"# Complex live coding session
tempo: 2.0

# Drums section
~kick: s "bd ~ bd ~"
~snare: s "~ sn ~ sn"
~hats: s "hh*8" $ fast 2

# Mix drums
~drums: ~kick + ~snare + ~hats
~filtered_drums: ~drums # lpf 2000 0.6

# Bass section
~bass_freq: "55 82.5 110" $ slow 2
~bass: saw ~bass_freq # lpf 500 0.8

# Output mix
out: ~filtered_drums * 0.6 + ~bass * 0.4
"#;
        let result = parse_program(code);
        assert!(
            result.is_ok(),
            "Failed to parse complex example with comments"
        );
        if let Ok((rest, statements)) = result {
            assert_eq!(rest.trim(), "", "Should consume entire program");
            // Should have: tempo, 6 bus assignments, 1 output = 8 statements
            assert!(
                statements.len() >= 8,
                "Should have at least 8 statements, got {}",
                statements.len()
            );
        }
    }
}
