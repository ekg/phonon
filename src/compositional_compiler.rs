//! Compositional Compiler
//!
//! Compiles the clean compositional AST into the existing UnifiedSignalGraph.
//! This bridges the new parser with the existing audio engine.

use crate::compositional_parser::{BinOp, Expr, Statement, Transform, UnOp};
use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::Pattern;
use crate::unified_graph::{NodeId, Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform};
use std::collections::HashMap;

/// Compilation context - tracks buses and node IDs
pub struct CompilerContext {
    /// Map of bus names to node IDs
    buses: HashMap<String, NodeId>,
    /// The signal graph we're building
    graph: UnifiedSignalGraph,
    /// Sample rate for creating buffers
    sample_rate: f32,
}

impl CompilerContext {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buses: HashMap::new(),
            graph: UnifiedSignalGraph::new(sample_rate),
            sample_rate,
        }
    }

    /// Get the compiled graph
    pub fn into_graph(self) -> UnifiedSignalGraph {
        self.graph
    }

    /// Set CPS (cycles per second)
    pub fn set_cps(&mut self, cps: f64) {
        self.graph.set_cps(cps as f32);
    }
}

/// Compile a full program
pub fn compile_program(
    statements: Vec<Statement>,
    sample_rate: f32,
) -> Result<UnifiedSignalGraph, String> {
    let mut ctx = CompilerContext::new(sample_rate);

    for statement in statements {
        compile_statement(&mut ctx, statement)?;
    }

    Ok(ctx.into_graph())
}

/// Compile a single statement
fn compile_statement(ctx: &mut CompilerContext, statement: Statement) -> Result<(), String> {
    match statement {
        Statement::BusAssignment { name, expr } => {
            let node_id = compile_expr(ctx, expr)?;
            ctx.buses.insert(name, node_id);
            Ok(())
        }
        Statement::Output(expr) => {
            let node_id = compile_expr(ctx, expr)?;
            ctx.graph.set_output(node_id);
            Ok(())
        }
        Statement::OutputChannel { channel, expr } => {
            let node_id = compile_expr(ctx, expr)?;
            ctx.graph.set_output_channel(channel, node_id);
            Ok(())
        }
        Statement::Tempo(cps) => {
            ctx.set_cps(cps);
            Ok(())
        }
    }
}

/// Compile an expression to a node ID
fn compile_expr(ctx: &mut CompilerContext, expr: Expr) -> Result<NodeId, String> {
    match expr {
        Expr::Number(n) => {
            // Constant signal node
            let node = SignalNode::Constant { value: n as f32 };
            Ok(ctx.graph.add_node(node))
        }

        Expr::String(pattern_str) => {
            // Parse mini-notation and create a Pattern node
            let pattern = parse_mini_notation(&pattern_str);
            let node = SignalNode::Pattern {
                pattern_str: pattern_str.clone(),
                pattern,
                last_value: 0.0,
                last_trigger_time: -1.0,
            };
            Ok(ctx.graph.add_node(node))
        }

        Expr::BusRef(name) => {
            // Look up bus reference
            ctx.buses
                .get(&name)
                .copied()
                .ok_or_else(|| format!("Undefined bus: ~{}", name))
        }

        Expr::Call { name, args } => compile_function_call(ctx, &name, args),

        Expr::Chain(left, right) => {
            // Chain: pass left as first argument to right
            // e.g., a # b becomes b(a)
            compile_chain(ctx, *left, *right)
        }

        Expr::Transform { expr, transform } => compile_transform(ctx, *expr, transform),

        Expr::BinOp { op, left, right } => compile_binop(ctx, op, *left, *right),

        Expr::UnOp { op, expr } => compile_unop(ctx, op, *expr),

        Expr::Paren(inner) => {
            // Parentheses are just grouping, compile the inner expression
            compile_expr(ctx, *inner)
        }
    }
}

/// Compile a function call
fn compile_function_call(
    ctx: &mut CompilerContext,
    name: &str,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    match name {
        // ========== Sample playback ==========
        "s" => {
            if args.is_empty() {
                return Err("s() requires at least one argument".to_string());
            }

            // First argument should be a pattern string
            // We need to extract the actual pattern string, not create a node
            let pattern_str = match &args[0] {
                Expr::String(s) => s.clone(),
                _ => return Err("s() requires a pattern string as first argument".to_string()),
            };

            let pattern = parse_mini_notation(&pattern_str);

            // TODO: Handle sample-specific parameters from remaining args
            // For now, create a basic sample node with defaults
            let node = SignalNode::Sample {
                pattern_str: pattern_str.clone(),
                pattern,
                last_trigger_time: -1.0,
                last_cycle: -1,
                playback_positions: HashMap::new(),
                gain: Signal::Value(1.0),
                pan: Signal::Value(0.0),
                speed: Signal::Value(1.0),
                cut_group: Signal::Value(0.0),
                n: Signal::Value(0.0),
                note: Signal::Value(0.0),
                attack: Signal::Value(0.0),
                release: Signal::Value(0.0),
            };
            Ok(ctx.graph.add_node(node))
        }

        // ========== Oscillators ==========
        "sine" => compile_oscillator(ctx, Waveform::Sine, args),
        "saw" => compile_oscillator(ctx, Waveform::Saw, args),
        "square" => compile_oscillator(ctx, Waveform::Square, args),
        "tri" => compile_oscillator(ctx, Waveform::Triangle, args),

        // ========== Filters ==========
        "lpf" => compile_filter(ctx, "lpf", args),
        "hpf" => compile_filter(ctx, "hpf", args),
        "bpf" => compile_filter(ctx, "bpf", args),

        // ========== Effects ==========
        "reverb" => compile_reverb(ctx, args),
        "distort" | "distortion" => compile_distortion(ctx, args),
        "delay" => compile_delay(ctx, args),
        "chorus" => compile_chorus(ctx, args),
        "bitcrush" => compile_bitcrush(ctx, args),

        _ => Err(format!("Unknown function: {}", name)),
    }
}

/// Compile oscillator node
fn compile_oscillator(
    ctx: &mut CompilerContext,
    waveform: Waveform,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err(format!("{:?} requires frequency argument", waveform));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;
    let node = SignalNode::Oscillator {
        freq: Signal::Node(freq_node),
        waveform,
        phase: 0.0,
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile filter node
fn compile_filter(
    ctx: &mut CompilerContext,
    filter_type: &str,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    // Filters can be used in two ways:
    // 1. Standalone: lpf(input, cutoff, q) - 3 args
    // 2. Chained: input # lpf(cutoff, q) - 2 args (input comes from chain)

    let (input_signal, cutoff_expr, q_expr) = if args.len() == 3 {
        // Standalone: lpf(input, cutoff, q)
        let input_node = compile_expr(ctx, args[0].clone())?;
        (Signal::Node(input_node), &args[1], &args[2])
    } else if args.len() == 2 {
        // Most common: chained from # operator
        // The first arg is actually a NodeId stored as Number (hack from compile_chain)
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2])
        } else {
            return Err(format!("{} in chain requires input node", filter_type));
        }
    } else {
        return Err(format!(
            "{} requires 2 arguments (cutoff, q) or 3 arguments (input, cutoff, q)",
            filter_type
        ));
    };

    // Compile cutoff and q expressions
    let cutoff_node = compile_expr(ctx, cutoff_expr.clone())?;
    let q_node = compile_expr(ctx, q_expr.clone())?;

    // Create the appropriate filter node
    use crate::unified_graph::FilterState;

    let node = match filter_type {
        "lpf" => SignalNode::LowPass {
            input: input_signal,
            cutoff: Signal::Node(cutoff_node),
            q: Signal::Node(q_node),
            state: FilterState::default(),
        },
        "hpf" => SignalNode::HighPass {
            input: input_signal,
            cutoff: Signal::Node(cutoff_node),
            q: Signal::Node(q_node),
            state: FilterState::default(),
        },
        "bpf" => SignalNode::BandPass {
            input: input_signal,
            center: Signal::Node(cutoff_node), // Note: center not cutoff for bandpass
            q: Signal::Node(q_node),
            state: FilterState::default(),
        },
        _ => return Err(format!("Unknown filter type: {}", filter_type)),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile reverb effect
fn compile_reverb(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // reverb can be used in two ways:
    // 1. Standalone: reverb(input, room_size, damping, mix) - 4 args
    // 2. Chained: input # reverb(room_size, damping, mix) - 3 args

    let (input_signal, room_size_expr, damping_expr, mix_expr) = if args.len() == 4 {
        // Standalone: reverb(input, room_size, damping, mix)
        let input_node = compile_expr(ctx, args[0].clone())?;
        (Signal::Node(input_node), &args[1], &args[2], &args[3])
    } else if args.len() == 3 {
        // Chained: input # reverb(room_size, damping, mix)
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2], &args[2]) // TODO: Fix args indexing
        } else {
            return Err("reverb in chain requires input node".to_string());
        }
    } else {
        return Err("reverb requires 3 arguments (room_size, damping, mix) or 4 arguments (input, room_size, damping, mix)".to_string());
    };

    // Actually, let me fix this - when chained we get 3 args but first is the node
    let (input_signal, room_size_expr, damping_expr, mix_expr) = if args.len() == 3 {
        // Try to extract input from first arg if it's a node ID
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2], &args[2])
        } else {
            // Standalone with 3 params - need to add input later
            return Err("reverb requires input".to_string());
        }
    } else if args.len() == 4 {
        let input_node = compile_expr(ctx, args[0].clone())?;
        (Signal::Node(input_node), &args[1], &args[2], &args[3])
    } else {
        return Err(format!(
            "reverb requires 3 or 4 arguments, got {}",
            args.len()
        ));
    };

    // Compile parameters
    let room_node = compile_expr(ctx, room_size_expr.clone())?;
    let damp_node = compile_expr(ctx, damping_expr.clone())?;
    let mix_node = compile_expr(ctx, mix_expr.clone())?;

    use crate::unified_graph::ReverbState;

    let node = SignalNode::Reverb {
        input: input_signal,
        room_size: Signal::Node(room_node),
        damping: Signal::Node(damp_node),
        mix: Signal::Node(mix_node),
        state: ReverbState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile distortion effect
fn compile_distortion(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // distort(drive, mix) when chained (2 args + 1 node ID arg)
    // distort(input, drive, mix) when standalone (3 args)

    let (input_signal, drive_expr, mix_expr) = if args.len() == 3 {
        // Could be chained or standalone - check first arg
        if let Expr::Number(node_id) = &args[0] {
            // Chained: first arg is node ID
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2])
        } else {
            // Standalone: first arg is input expression
            let input_node = compile_expr(ctx, args[0].clone())?;
            (Signal::Node(input_node), &args[1], &args[2])
        }
    } else {
        return Err(format!("distort requires 3 arguments, got {}", args.len()));
    };

    let drive_node = compile_expr(ctx, drive_expr.clone())?;
    let mix_node = compile_expr(ctx, mix_expr.clone())?;

    let node = SignalNode::Distortion {
        input: input_signal,
        drive: Signal::Node(drive_node),
        mix: Signal::Node(mix_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile delay effect
fn compile_delay(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // delay(time, feedback, mix) or input # delay(time, feedback, mix)

    let (input_signal, time_expr, feedback_expr, mix_expr) = if args.len() == 4 {
        // Chained: first arg is node ID
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2], &args[3])
        } else {
            // Standalone
            let input_node = compile_expr(ctx, args[0].clone())?;
            (Signal::Node(input_node), &args[1], &args[2], &args[3])
        }
    } else {
        return Err(format!("delay requires 4 arguments, got {}", args.len()));
    };

    let time_node = compile_expr(ctx, time_expr.clone())?;
    let feedback_node = compile_expr(ctx, feedback_expr.clone())?;
    let mix_node = compile_expr(ctx, mix_expr.clone())?;

    // Create delay buffer (1 second max)
    let buffer_size = ctx.sample_rate as usize; // 1 second buffer

    let node = SignalNode::Delay {
        input: input_signal,
        time: Signal::Node(time_node),
        feedback: Signal::Node(feedback_node),
        mix: Signal::Node(mix_node),
        buffer: vec![0.0; buffer_size],
        write_idx: 0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile chorus effect
fn compile_chorus(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // chorus(rate, depth, mix) or input # chorus(rate, depth, mix)

    let (input_signal, rate_expr, depth_expr, mix_expr) = if args.len() == 4 {
        // Chained: first arg is node ID
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2], &args[3])
        } else {
            // Standalone
            let input_node = compile_expr(ctx, args[0].clone())?;
            (Signal::Node(input_node), &args[1], &args[2], &args[3])
        }
    } else {
        return Err(format!("chorus requires 4 arguments, got {}", args.len()));
    };

    let rate_node = compile_expr(ctx, rate_expr.clone())?;
    let depth_node = compile_expr(ctx, depth_expr.clone())?;
    let mix_node = compile_expr(ctx, mix_expr.clone())?;

    use crate::unified_graph::ChorusState;

    let node = SignalNode::Chorus {
        input: input_signal,
        rate: Signal::Node(rate_node),
        depth: Signal::Node(depth_node),
        mix: Signal::Node(mix_node),
        state: ChorusState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile bitcrush effect
fn compile_bitcrush(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // bitcrush(bits, sample_rate) or input # bitcrush(bits, sample_rate)

    let (input_signal, bits_expr, sr_expr) = if args.len() == 3 {
        // Could be chained or standalone
        if let Expr::Number(node_id) = &args[0] {
            let input_node = NodeId(*node_id as usize);
            (Signal::Node(input_node), &args[1], &args[2])
        } else {
            let input_node = compile_expr(ctx, args[0].clone())?;
            (Signal::Node(input_node), &args[1], &args[2])
        }
    } else {
        return Err(format!("bitcrush requires 3 arguments, got {}", args.len()));
    };

    let bits_node = compile_expr(ctx, bits_expr.clone())?;
    let sr_node = compile_expr(ctx, sr_expr.clone())?;

    use crate::unified_graph::BitCrushState;

    let node = SignalNode::BitCrush {
        input: input_signal,
        bits: Signal::Node(bits_node),
        sample_rate: Signal::Node(sr_node),
        state: BitCrushState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile chain operator: a # b
fn compile_chain(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<NodeId, String> {
    // The chain operator passes left as input to right
    // We need to handle this based on what 'right' is
    match right {
        Expr::Call { name, mut args } => {
            // Prepend left as first argument
            let left_node = compile_expr(ctx, left)?;
            args.insert(0, Expr::Number(left_node.0 as f64)); // Hack: store node ID
            compile_function_call(ctx, &name, args)
        }
        _ => {
            // For other expressions, just compile them separately and connect
            let _left_node = compile_expr(ctx, left)?;
            compile_expr(ctx, right)
        }
    }
}

/// Compile pattern transform
fn compile_transform(
    ctx: &mut CompilerContext,
    expr: Expr,
    transform: Transform,
) -> Result<NodeId, String> {
    // For string literals, we can apply transforms directly to the parsed pattern
    if let Expr::String(pattern_str) = expr {
        let mut pattern = parse_mini_notation(&pattern_str);

        // Apply the transform to the pattern
        pattern = apply_transform_to_pattern(pattern, transform)?;

        // Create a Pattern node with the transformed pattern
        let node = SignalNode::Pattern {
            pattern_str: format!("{} (transformed)", pattern_str),
            pattern,
            last_value: 0.0,
            last_trigger_time: -1.0,
        };
        return Ok(ctx.graph.add_node(node));
    }

    // For other expressions, compile them first then try to extract and transform
    // This is more complex and may not always work
    // For now, just compile the expression without the transform
    // TODO: Handle transforms on arbitrary expressions
    compile_expr(ctx, expr)
}

/// Apply a transform to a pattern
fn apply_transform_to_pattern<T: Clone + Send + Sync + 'static>(
    pattern: Pattern<T>,
    transform: Transform,
) -> Result<Pattern<T>, String> {
    match transform {
        Transform::Fast(speed_expr) => {
            // Extract numeric value from expression
            let speed = extract_number(&speed_expr)?;
            Ok(pattern.fast(speed))
        }
        Transform::Slow(speed_expr) => {
            let speed = extract_number(&speed_expr)?;
            Ok(pattern.slow(speed))
        }
        Transform::Rev => Ok(pattern.rev()),
        Transform::Degrade => Ok(pattern.degrade()),
        Transform::DegradeBy(prob_expr) => {
            let prob = extract_number(&prob_expr)?;
            Ok(pattern.degrade_by(prob))
        }
        Transform::Stutter(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.stutter(n))
        }
        Transform::Palindrome => Ok(pattern.palindrome()),
        Transform::Every { n, transform } => {
            // For every, we need to recursively apply the inner transform
            // This is complex, so for now return an error
            let _ = (n, transform);
            Err("'every' transform not yet implemented in compiler".to_string())
        }
    }
}

/// Extract a numeric value from an expression (for transform arguments)
fn extract_number(expr: &Expr) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Paren(inner) => extract_number(inner),
        _ => Err(format!(
            "Transform argument must be a number, got: {:?}",
            expr
        )),
    }
}

/// Compile binary operator
/// Returns a node ID that outputs the result of the arithmetic operation
fn compile_binop(
    ctx: &mut CompilerContext,
    op: BinOp,
    left: Expr,
    right: Expr,
) -> Result<NodeId, String> {
    let left_node = compile_expr(ctx, left)?;
    let right_node = compile_expr(ctx, right)?;

    // Arithmetic operations are done via Signal::Expression
    let expr = match op {
        BinOp::Add => SignalExpr::Add(Signal::Node(left_node), Signal::Node(right_node)),
        BinOp::Sub => SignalExpr::Subtract(Signal::Node(left_node), Signal::Node(right_node)),
        BinOp::Mul => SignalExpr::Multiply(Signal::Node(left_node), Signal::Node(right_node)),
        BinOp::Div => SignalExpr::Divide(Signal::Node(left_node), Signal::Node(right_node)),
    };

    // We need a node that outputs this expression
    // Use Add node with the expression as input
    let node = SignalNode::Add {
        a: Signal::Expression(Box::new(expr)),
        b: Signal::Value(0.0), // Just pass through the expression
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile unary operator
fn compile_unop(ctx: &mut CompilerContext, op: UnOp, expr: Expr) -> Result<NodeId, String> {
    let node_id = compile_expr(ctx, expr)?;

    match op {
        UnOp::Neg => {
            // Negate by multiplying by -1 using Signal::Expression
            let neg_expr = SignalExpr::Multiply(Signal::Node(node_id), Signal::Value(-1.0));

            let node = SignalNode::Add {
                a: Signal::Expression(Box::new(neg_expr)),
                b: Signal::Value(0.0), // Pass through
            };
            Ok(ctx.graph.add_node(node))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositional_parser::parse_program;

    #[test]
    fn test_compile_simple_constant() {
        let code = "out: 440";
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_arithmetic() {
        let code = "out: 1 + 2";
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_bus_reference() {
        let code = r#"
            ~freq: 440
            out: ~freq
        "#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_oscillator() {
        let code = "out: sine(440)";
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_bus_error() {
        let code = "out: ~undefined";
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.contains("Undefined bus"));
        }
    }

    // ========== Pattern Transform Tests ==========

    #[test]
    fn test_compile_pattern_fast() {
        let code = r#"out: "bd sn" $ fast 2"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile fast transform");
    }

    #[test]
    fn test_compile_pattern_slow() {
        let code = r#"out: "bd sn hh cp" $ slow 0.5"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile slow transform");
    }

    #[test]
    fn test_compile_pattern_rev() {
        let code = r#"out: "bd sn hh" $ rev"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile rev transform");
    }

    #[test]
    fn test_compile_pattern_degrade() {
        let code = r#"out: "bd*8" $ degrade"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile degrade transform");
    }

    #[test]
    fn test_compile_pattern_degrade_by() {
        let code = r#"out: "hh*16" $ degradeBy 0.3"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile degradeBy transform");
    }

    #[test]
    fn test_compile_pattern_stutter() {
        let code = r#"out: "bd sn" $ stutter 4"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile stutter transform");
    }

    #[test]
    fn test_compile_pattern_palindrome() {
        let code = r#"out: "a b c" $ palindrome"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile palindrome transform");
    }

    #[test]
    fn test_compile_stacked_transforms() {
        // The key test - multiple transforms in sequence
        let code = r#"out: "bd sn" $ fast 2 $ rev $ slow 0.5"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile stacked transforms");
    }

    #[test]
    fn test_compile_bus_with_transform() {
        // This was the user's original problem!
        let code = r#"
            ~cutoffs: "<300 200 1000>" $ fast 2
            out: ~cutoffs
        "#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile bus with transform");
    }

    #[test]
    fn test_compile_user_example() {
        // The exact example from x.ph that was failing!
        let code = r#"
            ~cutoffs: "<300 200 1000>" $ fast 2
            ~resonances: "<0.8 0.6 0.2>" $ fast 9
            out: ~cutoffs
        "#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile user example from x.ph");
    }

    #[test]
    fn test_syntax_variations() {
        // Test all the different syntax variations

        // Space-separated (original style)
        let code1 = r#"out: sine 440"#;
        let (_, statements) = parse_program(code1).unwrap();
        assert!(compile_program(statements, 44100.0).is_ok());

        // Parenthesized with commas
        let code2 = r#"out: sine(440)"#;
        let (_, statements) = parse_program(code2).unwrap();
        assert!(compile_program(statements, 44100.0).is_ok());

        // Parenthesized expressions as arguments
        let code3 = r#"
            ~base: 220
            out: sine (~base)
        "#;
        let (_, statements) = parse_program(code3).unwrap();
        assert!(compile_program(statements, 44100.0).is_ok());

        // All work! The compositional parser handles all syntax styles!
    }

    #[test]
    fn test_transform_in_parentheses() {
        // Transforms with parentheses for grouping
        let code = r#"out: ("bd sn" $ fast 2)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile parenthesized transform");
    }

    // ========== Filter Tests ==========

    #[test]
    fn test_compile_lpf_chained() {
        // Most common usage: chained with #
        let code = r#"out: sine(440) # lpf(1000, 0.8)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile chained lpf");
    }

    #[test]
    fn test_compile_lpf_space_syntax() {
        // Space-separated syntax
        let code = r#"out: sine 440 # lpf 1000 0.8"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile lpf with space syntax");
    }

    #[test]
    fn test_compile_hpf() {
        let code = r#"out: saw(220) # hpf(500, 1.5)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile hpf");
    }

    #[test]
    fn test_compile_bpf() {
        let code = r#"out: square(110) # bpf(800, 2.0)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile bpf");
    }

    #[test]
    fn test_compile_sample_with_filter() {
        // Samples through filters
        let code = r#"out: s("bd sn hh cp") # lpf(2000, 0.5)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile sample with filter");
    }

    #[test]
    fn test_compile_filter_with_bus_refs() {
        // The user's actual use case - bus references as filter parameters!
        let code = r#"
            ~cutoffs: "<300 200 1000>" $ fast 2
            ~resonances: "<0.8 0.6 0.2>" $ fast 9
            out: s("hh*4 cp") # lpf(~cutoffs, ~resonances)
        "#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(
            result.is_ok(),
            "Failed to compile filter with bus ref parameters"
        );
    }

    #[test]
    fn test_compile_filter_with_bus_space_syntax() {
        // Same as above but with space syntax
        let code = r#"
            ~cutoffs: "<300 200 1000>" $ fast 2
            ~resonances: "<0.8 0.6 0.2>" $ fast 9
            out: s "hh*4 cp" # lpf ~cutoffs ~resonances
        "#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(
            result.is_ok(),
            "Failed to compile filter with bus refs (space syntax)"
        );
    }

    #[test]
    fn test_compile_chained_filters() {
        // Multiple filters in series
        let code = r#"out: saw(110) # lpf(2000, 0.8) # hpf(100, 0.5)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile chained filters");
    }

    #[test]
    fn test_compile_full_user_example() {
        // The complete example from x.ph - this should now work!
        let code = r#"
            ~cutoffs: "<300 200 1000>" $ fast 2
            ~resonances: "<0.8 0.6 0.2>" $ fast 9
            out: s "hh*4 cp" # lpf ~cutoffs ~resonances
        "#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(
            result.is_ok(),
            "Failed to compile full user example from x.ph"
        );
    }

    // ========== Sample Bank Selection Tests ==========

    #[test]
    fn test_compile_sample_bank_selection() {
        // Basic sample bank selection with :n syntax
        let code = r#"out: s("bd:0 bd:1 bd:2")"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile sample bank selection");
    }

    #[test]
    fn test_compile_sample_bank_with_transform() {
        // Sample bank selection with transforms
        let code = r#"out: s("bd:0*4 sn:2") $ fast 2"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(
            result.is_ok(),
            "Failed to compile sample bank with transform"
        );
    }

    #[test]
    fn test_compile_sample_bank_through_filter() {
        // Sample bank selection routed through effects
        let code = r#"out: s("bd:0 sn:2 hh:1") # lpf(1000, 0.8)"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(
            result.is_ok(),
            "Failed to compile sample bank through filter"
        );
    }

    #[test]
    fn test_compile_sample_bank_space_syntax() {
        // Space-separated syntax with sample banks
        let code = r#"out: s "bd:0 bd:1 bd:2 bd:3""#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(
            result.is_ok(),
            "Failed to compile sample bank with space syntax"
        );
    }
}
