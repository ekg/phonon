#![allow(unused_variables)]
//! Compositional Compiler
//!
//! Compiles the clean compositional AST into the existing UnifiedSignalGraph.
//! This bridges the new parser with the existing audio engine.

use crate::compositional_parser::{BinOp, Expr, Statement, Transform, UnOp};
use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::Pattern;
use crate::superdirt_synths::SynthLibrary;
use crate::unified_graph::{NodeId, Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform};
use std::collections::HashMap;

/// Compilation context - tracks buses, functions, and node IDs
pub struct CompilerContext {
    /// Map of bus names to node IDs
    buses: HashMap<String, NodeId>,
    /// Map of function names to their definitions
    functions: HashMap<String, FunctionDef>,
    /// The signal graph we're building
    graph: UnifiedSignalGraph,
    /// Sample rate for creating buffers
    sample_rate: f32,
    /// Synth library for pre-built synthesizers
    synth_lib: SynthLibrary,
}

/// Function definition storage
#[derive(Clone, Debug)]
struct FunctionDef {
    params: Vec<String>,
    body: Vec<Statement>,
    return_expr: Expr,
}

impl CompilerContext {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buses: HashMap::new(),
            functions: HashMap::new(),
            graph: UnifiedSignalGraph::new(sample_rate),
            sample_rate,
            synth_lib: SynthLibrary::with_sample_rate(sample_rate),
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

    let mut graph = ctx.into_graph();

    // Auto-routing: If no explicit 'out:' was set, mix all buses to output
    if !graph.has_output() {
        let bus_names = graph.get_all_bus_names();
        if !bus_names.is_empty() {
            // Get all bus node IDs
            let bus_nodes: Vec<_> = bus_names
                .iter()
                .filter_map(|name| graph.get_bus(name))
                .collect();

            if !bus_nodes.is_empty() {
                // Mix all buses together
                let mixed = if bus_nodes.len() == 1 {
                    bus_nodes[0]
                } else {
                    // Chain Add nodes to mix all buses
                    let mut result = bus_nodes[0];
                    for &node in &bus_nodes[1..] {
                        result = graph.add_node(SignalNode::Add {
                            a: Signal::Node(result),
                            b: Signal::Node(node),
                        });
                    }
                    result
                };
                graph.set_output(mixed);
            }
        }
    }

    Ok(graph)
}

/// Compile a single statement
fn compile_statement(ctx: &mut CompilerContext, statement: Statement) -> Result<(), String> {
    match statement {
        Statement::BusAssignment { name, expr } => {
            let node_id = compile_expr(ctx, expr)?;
            ctx.buses.insert(name.clone(), node_id);
            ctx.graph.add_bus(name, node_id); // Register bus in graph for auto-routing
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
        Statement::FunctionDef {
            name,
            params,
            body,
            return_expr,
        } => {
            // Store function definition for later use
            ctx.functions.insert(
                name.clone(),
                FunctionDef {
                    params,
                    body,
                    return_expr,
                },
            );
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

        Expr::Var(name) => {
            // Check if it's a zero-argument function first
            if name == "white_noise" {
                return compile_white_noise(ctx, vec![]);
            }
            if name == "pink_noise" {
                return compile_pink_noise(ctx, vec![]);
            }
            if name == "brown_noise" {
                return compile_brown_noise(ctx, vec![]);
            }

            // Otherwise, look up variable (function parameter)
            ctx.buses
                .get(&name)
                .copied()
                .ok_or_else(|| format!("Undefined variable: {}", name))
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

        Expr::List(_exprs) => {
            // Lists should only be used as arguments to functions like stack
            // They shouldn't appear as standalone expressions
            Err("Lists can only be used as function arguments (e.g., stack [...])".to_string())
        }

        Expr::ChainInput(_) => {
            // ChainInput is only used internally by the compiler
            // It should never appear in parsed code
            Err(
                "ChainInput is an internal compiler marker and should not appear in source code"
                    .to_string(),
            )
        }
    }
}

/// Compile a user-defined function by substituting parameters
fn compile_user_function(
    ctx: &mut CompilerContext,
    func_def: &FunctionDef,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    // Check parameter count
    if args.len() != func_def.params.len() {
        return Err(format!(
            "Function expects {} arguments, got {}",
            func_def.params.len(),
            args.len()
        ));
    }

    // Create a parameter substitution map
    let mut param_values: HashMap<String, NodeId> = HashMap::new();

    // Compile all argument expressions
    for (param_name, arg_expr) in func_def.params.iter().zip(args.iter()) {
        let node_id = compile_expr(ctx, arg_expr.clone())?;
        param_values.insert(param_name.clone(), node_id);
    }

    // Save current bus context
    let saved_buses = ctx.buses.clone();

    // Replace bus references in function body with parameter values
    for (param, value) in &param_values {
        ctx.buses.insert(param.clone(), *value);
    }

    // Compile function body (bus assignments)
    for stmt in &func_def.body {
        compile_statement(ctx, stmt.clone())?;
    }

    // Compile return expression
    let return_expr = substitute_params(func_def.return_expr.clone(), &param_values);
    let result = compile_expr(ctx, return_expr)?;

    // Restore bus context (remove local buses)
    ctx.buses = saved_buses;

    Ok(result)
}

/// Substitute parameter references in an expression
fn substitute_params(expr: Expr, params: &HashMap<String, NodeId>) -> Expr {
    match expr {
        // If it's a bus ref that's also a parameter, return it unchanged
        // (the context already has the substituted value)
        Expr::BusRef(name) => Expr::BusRef(name),
        // Recursively substitute in composite expressions
        Expr::Call { name, args } => Expr::Call {
            name,
            args: args
                .into_iter()
                .map(|arg| substitute_params(arg, params))
                .collect(),
        },
        Expr::Chain(left, right) => Expr::Chain(
            Box::new(substitute_params(*left, params)),
            Box::new(substitute_params(*right, params)),
        ),
        Expr::BinOp { op, left, right } => Expr::BinOp {
            op,
            left: Box::new(substitute_params(*left, params)),
            right: Box::new(substitute_params(*right, params)),
        },
        Expr::UnOp { op, expr: inner } => Expr::UnOp {
            op,
            expr: Box::new(substitute_params(*inner, params)),
        },
        Expr::Paren(inner) => Expr::Paren(Box::new(substitute_params(*inner, params))),
        Expr::Transform { expr: inner, transform } => Expr::Transform {
            expr: Box::new(substitute_params(*inner, params)),
            transform,
        },
        // Literals don't need substitution
        _ => expr,
    }
}

/// Compile a function call
fn compile_function_call(
    ctx: &mut CompilerContext,
    name: &str,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    // Check for user-defined functions first
    if let Some(func_def) = ctx.functions.get(name).cloned() {
        return compile_user_function(ctx, &func_def, args);
    }

    // Fall back to built-in functions
    match name {
        // ========== Pattern Combinators ==========
        "stack" => compile_stack(ctx, args),
        "cat" => compile_cat(ctx, args),
        "slowcat" => compile_slowcat(ctx, args),

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

        // ========== Oscillators (continuous) ==========
        "sine" => compile_oscillator(ctx, Waveform::Sine, args),
        "saw" => compile_oscillator(ctx, Waveform::Saw, args),
        "square" => compile_oscillator(ctx, Waveform::Square, args),
        "tri" => compile_oscillator(ctx, Waveform::Triangle, args),
        "fm" => compile_fm(ctx, args),
        "white_noise" => compile_white_noise(ctx, args),
        "pink_noise" => compile_pink_noise(ctx, args),
        "brown_noise" => compile_brown_noise(ctx, args),
        "pulse" => compile_pulse(ctx, args),
        "ring_mod" => compile_ring_mod(ctx, args),
        "limiter" => compile_limiter(ctx, args),

        // ========== Pattern-triggered synths ==========
        "sine_trig" => compile_synth_pattern(ctx, Waveform::Sine, args),
        "saw_trig" => compile_synth_pattern(ctx, Waveform::Saw, args),
        "square_trig" => compile_synth_pattern(ctx, Waveform::Square, args),
        "tri_trig" => compile_synth_pattern(ctx, Waveform::Triangle, args),

        // ========== Noise ==========
        "noise" => {
            // Noise generator - arguments are ignored (for parser compatibility)
            // Parser requires at least one argument, so users call: noise 0
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u32)
                .unwrap_or(42); // Fallback to constant seed if system time fails
            let node = SignalNode::Noise { seed };
            Ok(ctx.graph.add_node(node))
        }

        // ========== SuperDirt Synths ==========
        "superkick" => compile_superkick(ctx, args),
        "supersaw" => compile_supersaw(ctx, args),
        "superpwm" => compile_superpwm(ctx, args),
        "superchip" => compile_superchip(ctx, args),
        "superfm" => compile_superfm(ctx, args),
        "supersnare" => compile_supersnare(ctx, args),
        "superhat" => compile_superhat(ctx, args),

        // ========== Filters ==========
        "lpf" => compile_filter(ctx, "lpf", args),
        "hpf" => compile_filter(ctx, "hpf", args),
        "bpf" => compile_filter(ctx, "bpf", args),
        "moog_ladder" | "moog" => compile_moog_ladder(ctx, args),
        "parametric_eq" | "eq" => compile_parametric_eq(ctx, args),

        // ========== Effects ==========
        "reverb" => compile_reverb(ctx, args),
        "distort" | "distortion" => compile_distortion(ctx, args),
        "delay" => compile_delay(ctx, args),
        "chorus" => compile_chorus(ctx, args),
        "flanger" => compile_flanger(ctx, args),
        "compressor" | "comp" => compile_compressor(ctx, args),
        "bitcrush" => compile_bitcrush(ctx, args),

        // ========== Envelope ==========
        "env" | "envelope" => compile_envelope(ctx, args),
        "env_trig" => compile_envelope_pattern(ctx, args),
        "adsr" => compile_adsr(ctx, args),
        "ad" => compile_ad(ctx, args),
        "line" => compile_line(ctx, args),

        _ => Err(format!("Unknown function: {}", name)),
    }
}

// ========== Helper Functions ==========

/// Extract chained input and remaining parameter expressions
///
/// Effects and filters can be used in two ways:
/// 1. Standalone: effect(input, param1, param2, ...)
/// 2. Chained: input # effect(param1, param2, ...)
///
/// This helper extracts the input signal and returns remaining parameters.
fn extract_chain_input(
    ctx: &mut CompilerContext,
    args: &[Expr],
) -> Result<(Signal, Vec<Expr>), String> {
    if args.is_empty() {
        return Err("Function requires at least one argument".to_string());
    }

    match &args[0] {
        Expr::ChainInput(node_id) => {
            // Chained: input comes from chain operator
            Ok((Signal::Node(*node_id), args[1..].to_vec()))
        }
        _ => {
            // Standalone: first arg is the input, compile it
            let input_node = compile_expr(ctx, args[0].clone())?;
            Ok((Signal::Node(input_node), args[1..].to_vec()))
        }
    }
}

/// Compile stack combinator - plays multiple patterns/signals simultaneously
fn compile_stack(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("stack requires a list argument".to_string());
    }

    // First argument should be a list
    let exprs = match &args[0] {
        Expr::List(exprs) => exprs,
        _ => {
            return Err("stack requires a list as argument: stack [expr1, expr2, ...]".to_string())
        }
    };

    if exprs.is_empty() {
        return Err("stack requires at least one expression in the list".to_string());
    }

    // Compile each expression to a node
    let nodes: Result<Vec<NodeId>, String> = exprs
        .iter()
        .map(|expr| compile_expr(ctx, expr.clone()))
        .collect();

    let nodes = nodes?;

    // Mix all nodes together by chaining Add nodes
    // For [a, b, c], create: Add(Add(a, b), c)
    let mut result = nodes[0];
    for &node in &nodes[1..] {
        let add_node = SignalNode::Add {
            a: Signal::Node(result),
            b: Signal::Node(node),
        };
        result = ctx.graph.add_node(add_node);
    }

    Ok(result)
}

/// Compile cat combinator - concatenates patterns within each cycle
/// Each pattern gets an equal division of the cycle time
/// Usage: cat [s "bd", s "sn", s "hh"]  -> plays bd (0-0.33), sn (0.33-0.66), hh (0.66-1.0)
fn compile_cat(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("cat requires a list argument".to_string());
    }

    // First argument should be a list
    let pattern_strs = match &args[0] {
        Expr::List(exprs) => {
            // Extract pattern strings from each expression
            exprs
                .iter()
                .map(|expr| match expr {
                    Expr::String(s) => Ok(s.clone()),
                    _ => Err(
                        "cat requires a list of pattern strings: cat [\"bd\", \"sn\", ...]"
                            .to_string(),
                    ),
                })
                .collect::<Result<Vec<String>, String>>()?
        }
        _ => return Err("cat requires a list as argument: cat [\"bd\", \"sn\", ...]".to_string()),
    };

    if pattern_strs.is_empty() {
        return Err("cat requires at least one pattern in the list".to_string());
    }

    // Parse each pattern string
    let patterns: Vec<Pattern<String>> = pattern_strs
        .iter()
        .map(|s| parse_mini_notation(s))
        .collect();

    // Combine using Pattern::cat
    let combined_pattern = Pattern::cat(patterns);
    let combined_str = format!("cat [{}]", pattern_strs.join(", "));

    // Create a Sample node with the combined pattern
    let node = SignalNode::Sample {
        pattern_str: combined_str,
        pattern: combined_pattern,
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

/// Compile slowcat combinator - alternates between patterns on each cycle
/// Cycle 0 plays pattern 0, cycle 1 plays pattern 1, etc.
/// Usage: slowcat [s "bd*4", s "sn*4", s "hh*4"] -> cycle 0: bd*4, cycle 1: sn*4, cycle 2: hh*4, repeat
fn compile_slowcat(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("slowcat requires a list argument".to_string());
    }

    // First argument should be a list
    let pattern_strs = match &args[0] {
        Expr::List(exprs) => {
            // Extract pattern strings from each expression
            exprs
                .iter()
                .map(|expr| match expr {
                    Expr::String(s) => Ok(s.clone()),
                    _ => Err(
                        "slowcat requires a list of pattern strings: slowcat [\"bd\", \"sn\", ...]"
                            .to_string(),
                    ),
                })
                .collect::<Result<Vec<String>, String>>()?
        }
        _ => {
            return Err(
                "slowcat requires a list as argument: slowcat [\"bd\", \"sn\", ...]".to_string(),
            )
        }
    };

    if pattern_strs.is_empty() {
        return Err("slowcat requires at least one pattern in the list".to_string());
    }

    // Parse each pattern string
    let patterns: Vec<Pattern<String>> = pattern_strs
        .iter()
        .map(|s| parse_mini_notation(s))
        .collect();

    // Combine using Pattern::slowcat
    let combined_pattern = Pattern::slowcat(patterns);
    let combined_str = format!("slowcat [{}]", pattern_strs.join(", "));

    // Create a Sample node with the combined pattern
    let node = SignalNode::Sample {
        pattern_str: combined_str,
        pattern: combined_pattern,
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

/// Compile FM oscillator
/// Usage: fm carrier_freq modulator_freq mod_index
fn compile_fm(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!(
            "fm requires 3 parameters (carrier_freq, modulator_freq, mod_index), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let carrier_node = compile_expr(ctx, args[0].clone())?;
    let modulator_node = compile_expr(ctx, args[1].clone())?;
    let index_node = compile_expr(ctx, args[2].clone())?;

    let node = SignalNode::FMOscillator {
        carrier_freq: Signal::Node(carrier_node),
        modulator_freq: Signal::Node(modulator_node),
        mod_index: Signal::Node(index_node),
        carrier_phase: 0.0,
        modulator_phase: 0.0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile white noise generator
fn compile_white_noise(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!(
            "white_noise takes no parameters, got {}",
            args.len()
        ));
    }

    let node = SignalNode::WhiteNoise;
    Ok(ctx.graph.add_node(node))
}

/// Compile pink noise generator (1/f spectrum, equal energy per octave)
fn compile_pink_noise(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::PinkNoiseState;

    if !args.is_empty() {
        return Err(format!(
            "pink_noise takes no parameters, got {}",
            args.len()
        ));
    }

    let node = SignalNode::PinkNoise {
        state: PinkNoiseState::default(),
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile brown noise generator (6dB/octave rolloff, random walk)
fn compile_brown_noise(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::BrownNoiseState;

    if !args.is_empty() {
        return Err(format!(
            "brown_noise takes no parameters, got {}",
            args.len()
        ));
    }

    let node = SignalNode::BrownNoise {
        state: BrownNoiseState::default(),
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile pulse oscillator (variable pulse width)
fn compile_pulse(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "pulse requires 2 parameters (freq, width), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let freq_node = compile_expr(ctx, args[0].clone())?;
    let width_node = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::Pulse {
        freq: Signal::Node(freq_node),
        width: Signal::Node(width_node),
        phase: 0.0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile ring modulation (signal multiplication)
/// Ring modulation creates sidebands at sum and difference frequencies
fn compile_ring_mod(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "ring_mod requires 2 parameters (signal1, signal2), got {}",
            args.len()
        ));
    }

    // Compile both signals
    let signal1 = compile_expr(ctx, args[0].clone())?;
    let signal2 = compile_expr(ctx, args[1].clone())?;

    // Ring modulation is just multiplication of two signals
    let node = SignalNode::Multiply {
        a: Signal::Node(signal1),
        b: Signal::Node(signal2),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile brick-wall limiter
fn compile_limiter(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "limiter requires 2 parameters (input, threshold), got {}",
            args.len()
        ));
    }

    // Compile input signal and threshold
    let input_node = compile_expr(ctx, args[0].clone())?;
    let threshold_node = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::Limiter {
        input: Signal::Node(input_node),
        threshold: Signal::Node(threshold_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile pattern-triggered synth node (with envelope)
fn compile_synth_pattern(
    ctx: &mut CompilerContext,
    waveform: Waveform,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err(format!("{:?}_trig requires pattern string argument", waveform));
    }

    // First argument should be a pattern string
    let pattern_str = match &args[0] {
        Expr::String(s) => s.clone(),
        _ => return Err(format!("{:?}_trig requires a pattern string as first argument", waveform)),
    };

    let pattern = parse_mini_notation(&pattern_str);

    // Parse optional ADSR parameters (attack, decay, sustain, release)
    // Default ADSR: percussive envelope (0.001, 0.1, 0.0, 0.1)
    let attack = if args.len() > 1 {
        match &args[1] {
            Expr::Number(n) => *n as f32,
            _ => 0.001,
        }
    } else {
        0.001
    };

    let decay = if args.len() > 2 {
        match &args[2] {
            Expr::Number(n) => *n as f32,
            _ => 0.1,
        }
    } else {
        0.1
    };

    let sustain = if args.len() > 3 {
        match &args[3] {
            Expr::Number(n) => *n as f32,
            _ => 0.0,
        }
    } else {
        0.0
    };

    let release = if args.len() > 4 {
        match &args[4] {
            Expr::Number(n) => *n as f32,
            _ => 0.1,
        }
    } else {
        0.1
    };

    // TODO: Handle gain and pan parameters
    let node = SignalNode::SynthPattern {
        pattern_str: pattern_str.clone(),
        pattern,
        last_trigger_time: -1.0,
        waveform,
        attack,
        decay,
        sustain,
        release,
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile filter node
fn compile_filter(
    ctx: &mut CompilerContext,
    filter_type: &str,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "{} requires 2 parameters (cutoff, q), got {}",
            filter_type,
            params.len()
        ));
    }

    // Compile cutoff and q expressions
    let cutoff_node = compile_expr(ctx, params[0].clone())?;
    let q_node = compile_expr(ctx, params[1].clone())?;

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

/// Compile Moog Ladder filter
fn compile_moog_ladder(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Moog ladder expects 3 params: input, cutoff, resonance
    if args.len() != 3 {
        return Err(format!(
            "moog_ladder requires 3 parameters (input, cutoff, resonance), got {}",
            args.len()
        ));
    }

    let input_node = compile_expr(ctx, args[0].clone())?;
    let cutoff_node = compile_expr(ctx, args[1].clone())?;
    let resonance_node = compile_expr(ctx, args[2].clone())?;

    use crate::unified_graph::MoogLadderState;

    let node = SignalNode::MoogLadder {
        input: Signal::Node(input_node),
        cutoff: Signal::Node(cutoff_node),
        resonance: Signal::Node(resonance_node),
        state: MoogLadderState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Parametric EQ (3-band peaking equalizer)
fn compile_parametric_eq(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Parametric EQ expects 10 params: input + 3 bands × 3 params each
    if args.len() != 10 {
        return Err(format!(
            "parametric_eq requires 10 parameters (input, low_freq, low_gain, low_q, mid_freq, mid_gain, mid_q, high_freq, high_gain, high_q), got {}",
            args.len()
        ));
    }

    let input_node = compile_expr(ctx, args[0].clone())?;
    let low_freq_node = compile_expr(ctx, args[1].clone())?;
    let low_gain_node = compile_expr(ctx, args[2].clone())?;
    let low_q_node = compile_expr(ctx, args[3].clone())?;
    let mid_freq_node = compile_expr(ctx, args[4].clone())?;
    let mid_gain_node = compile_expr(ctx, args[5].clone())?;
    let mid_q_node = compile_expr(ctx, args[6].clone())?;
    let high_freq_node = compile_expr(ctx, args[7].clone())?;
    let high_gain_node = compile_expr(ctx, args[8].clone())?;
    let high_q_node = compile_expr(ctx, args[9].clone())?;

    use crate::unified_graph::ParametricEQState;

    let node = SignalNode::ParametricEQ {
        input: Signal::Node(input_node),
        low_freq: Signal::Node(low_freq_node),
        low_gain: Signal::Node(low_gain_node),
        low_q: Signal::Node(low_q_node),
        mid_freq: Signal::Node(mid_freq_node),
        mid_gain: Signal::Node(mid_gain_node),
        mid_q: Signal::Node(mid_q_node),
        high_freq: Signal::Node(high_freq_node),
        high_gain: Signal::Node(high_gain_node),
        high_q: Signal::Node(high_q_node),
        state: ParametricEQState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile reverb effect
fn compile_reverb(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 && params.len() != 3 {
        return Err(format!(
            "reverb requires 2-3 parameters (room_size, damping, [mix=0.3]), got {}",
            params.len()
        ));
    }

    // Compile parameters
    let room_node = compile_expr(ctx, params[0].clone())?;
    let damp_node = compile_expr(ctx, params[1].clone())?;
    let mix_node = if params.len() == 3 {
        compile_expr(ctx, params[2].clone())?
    } else {
        // Default mix = 0.3 (30% wet)
        ctx.graph.add_node(SignalNode::Constant { value: 0.3 })
    };

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
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 && params.len() != 2 {
        return Err(format!(
            "distort requires 1-2 parameters (drive, [mix=0.5]), got {}",
            params.len()
        ));
    }

    let drive_node = compile_expr(ctx, params[0].clone())?;
    let mix_node = if params.len() == 2 {
        compile_expr(ctx, params[1].clone())?
    } else {
        // Default mix = 0.5 (50% wet)
        ctx.graph.add_node(SignalNode::Constant { value: 0.5 })
    };

    let node = SignalNode::Distortion {
        input: input_signal,
        drive: Signal::Node(drive_node),
        mix: Signal::Node(mix_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile pattern-triggered envelope (rhythmic gate)
/// Usage: signal # env_trig("pattern", attack, decay, sustain, release)
fn compile_envelope_pattern(
    ctx: &mut CompilerContext,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.is_empty() {
        return Err("env_trig requires at least a pattern string argument".to_string());
    }

    // First parameter should be a pattern string
    let pattern_str = match &params[0] {
        Expr::String(s) => s.clone(),
        _ => return Err("env_trig requires a pattern string as first argument".to_string()),
    };

    let pattern = parse_mini_notation(&pattern_str);

    // Parse optional ADSR parameters (attack, decay, sustain, release)
    // Default ADSR: percussive envelope (0.001, 0.1, 0.0, 0.1)
    let attack = if params.len() > 1 {
        extract_number(&params[1])? as f32
    } else {
        0.001
    };

    let decay = if params.len() > 2 {
        extract_number(&params[2])? as f32
    } else {
        0.1
    };

    let sustain = if params.len() > 3 {
        extract_number(&params[3])? as f32
    } else {
        0.0
    };

    let release = if params.len() > 4 {
        extract_number(&params[4])? as f32
    } else {
        0.1
    };

    use crate::unified_graph::EnvState;

    let node = SignalNode::EnvelopePattern {
        input: input_signal,
        pattern_str: pattern_str.clone(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        attack,
        decay,
        sustain,
        release,
        state: EnvState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile delay effect
fn compile_delay(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 3 {
        return Err(format!(
            "delay requires 3 parameters (time, feedback, mix), got {}",
            params.len()
        ));
    }

    let time_node = compile_expr(ctx, params[0].clone())?;
    let feedback_node = compile_expr(ctx, params[1].clone())?;
    let mix_node = compile_expr(ctx, params[2].clone())?;

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
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 && params.len() != 3 {
        return Err(format!(
            "chorus requires 2-3 parameters (rate, depth, [mix=0.3]), got {}",
            params.len()
        ));
    }

    let rate_node = compile_expr(ctx, params[0].clone())?;
    let depth_node = compile_expr(ctx, params[1].clone())?;
    let mix_node = if params.len() == 3 {
        compile_expr(ctx, params[2].clone())?
    } else {
        // Default mix = 0.3 (30% wet)
        ctx.graph.add_node(SignalNode::Constant { value: 0.3 })
    };

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

/// Compile flanger effect
fn compile_flanger(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Flanger expects 4 params: input, depth, rate, feedback
    if args.len() != 4 {
        return Err(format!(
            "flanger requires 4 parameters (input, depth, rate, feedback), got {}",
            args.len()
        ));
    }

    let input_node = compile_expr(ctx, args[0].clone())?;
    let depth_node = compile_expr(ctx, args[1].clone())?;
    let rate_node = compile_expr(ctx, args[2].clone())?;
    let feedback_node = compile_expr(ctx, args[3].clone())?;

    use crate::unified_graph::FlangerState;

    let node = SignalNode::Flanger {
        input: Signal::Node(input_node),
        depth: Signal::Node(depth_node),
        rate: Signal::Node(rate_node),
        feedback: Signal::Node(feedback_node),
        state: FlangerState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile compressor effect
fn compile_compressor(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 5 {
        return Err(format!(
            "compressor requires 5 parameters (threshold, ratio, attack, release, makeup_gain), got {}",
            params.len()
        ));
    }

    let threshold_node = compile_expr(ctx, params[0].clone())?;
    let ratio_node = compile_expr(ctx, params[1].clone())?;
    let attack_node = compile_expr(ctx, params[2].clone())?;
    let release_node = compile_expr(ctx, params[3].clone())?;
    let makeup_node = compile_expr(ctx, params[4].clone())?;

    use crate::unified_graph::CompressorState;

    let node = SignalNode::Compressor {
        input: input_signal,
        threshold: Signal::Node(threshold_node),
        ratio: Signal::Node(ratio_node),
        attack: Signal::Node(attack_node),
        release: Signal::Node(release_node),
        makeup_gain: Signal::Node(makeup_node),
        state: CompressorState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile bitcrush effect
fn compile_bitcrush(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "bitcrush requires 2 parameters (bits, sample_rate), got {}",
            params.len()
        ));
    }

    let bits_node = compile_expr(ctx, params[0].clone())?;
    let sr_node = compile_expr(ctx, params[1].clone())?;

    use crate::unified_graph::BitCrushState;

    let node = SignalNode::BitCrush {
        input: input_signal,
        bits: Signal::Node(bits_node),
        sample_rate: Signal::Node(sr_node),
        state: BitCrushState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

// ========== SuperDirt Synth Compilers ==========

/// Compile SuperKick synth
/// Usage: superkick(freq, pitch_env, sustain, noise)
fn compile_superkick(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("superkick requires at least freq argument".to_string());
    }

    let freq = Signal::Node(compile_expr(ctx, args[0].clone())?);
    let pitch_env = if args.len() > 1 {
        Some(Signal::Node(compile_expr(ctx, args[1].clone())?))
    } else {
        None
    };
    let sustain = if args.len() > 2 {
        Some(extract_number(&args[2])? as f32)
    } else {
        None
    };
    let noise_amt = if args.len() > 3 {
        Some(Signal::Node(compile_expr(ctx, args[3].clone())?))
    } else {
        None
    };

    let node_id = ctx
        .synth_lib
        .build_kick(&mut ctx.graph, freq, pitch_env, sustain, noise_amt);
    Ok(node_id)
}

/// Compile SuperSaw synth
/// Usage: supersaw(freq, detune, voices)
fn compile_supersaw(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("supersaw requires freq argument".to_string());
    }

    let freq = Signal::Node(compile_expr(ctx, args[0].clone())?);
    let detune = if args.len() > 1 {
        Some(extract_number(&args[1])? as f32)
    } else {
        None
    };
    let voices = if args.len() > 2 {
        Some(extract_number(&args[2])? as usize)
    } else {
        None
    };

    let node_id = ctx
        .synth_lib
        .build_supersaw(&mut ctx.graph, freq, detune, voices);
    Ok(node_id)
}

/// Compile SuperPWM synth
/// Usage: superpwm(freq, pwm_rate, pwm_depth)
fn compile_superpwm(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("superpwm requires freq argument".to_string());
    }

    let freq = Signal::Node(compile_expr(ctx, args[0].clone())?);
    let pwm_rate = if args.len() > 1 {
        Some(extract_number(&args[1])? as f32)
    } else {
        None
    };
    let pwm_depth = if args.len() > 2 {
        Some(extract_number(&args[2])? as f32)
    } else {
        None
    };

    let node_id = ctx
        .synth_lib
        .build_superpwm(&mut ctx.graph, freq, pwm_rate, pwm_depth);
    Ok(node_id)
}

/// Compile SuperChip synth
/// Usage: superchip(freq, vibrato_rate, vibrato_depth)
fn compile_superchip(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("superchip requires freq argument".to_string());
    }

    let freq = Signal::Node(compile_expr(ctx, args[0].clone())?);
    let vibrato_rate = if args.len() > 1 {
        Some(extract_number(&args[1])? as f32)
    } else {
        None
    };
    let vibrato_depth = if args.len() > 2 {
        Some(extract_number(&args[2])? as f32)
    } else {
        None
    };

    let node_id = ctx
        .synth_lib
        .build_superchip(&mut ctx.graph, freq, vibrato_rate, vibrato_depth);
    Ok(node_id)
}

/// Compile SuperFM synth
/// Usage: superfm(freq, mod_ratio, mod_index)
fn compile_superfm(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("superfm requires freq argument".to_string());
    }

    let freq = Signal::Node(compile_expr(ctx, args[0].clone())?);
    let mod_ratio = if args.len() > 1 {
        Some(extract_number(&args[1])? as f32)
    } else {
        None
    };
    let mod_index = if args.len() > 2 {
        Some(extract_number(&args[2])? as f32)
    } else {
        None
    };

    let node_id = ctx
        .synth_lib
        .build_superfm(&mut ctx.graph, freq, mod_ratio, mod_index);
    Ok(node_id)
}

/// Compile SuperSnare synth
/// Usage: supersnare(freq, snappy, sustain)
fn compile_supersnare(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("supersnare requires freq argument".to_string());
    }

    let freq = Signal::Node(compile_expr(ctx, args[0].clone())?);
    let snappy = if args.len() > 1 {
        Some(extract_number(&args[1])? as f32)
    } else {
        None
    };
    let sustain = if args.len() > 2 {
        Some(extract_number(&args[2])? as f32)
    } else {
        None
    };

    let node_id = ctx
        .synth_lib
        .build_snare(&mut ctx.graph, freq, snappy, sustain);
    Ok(node_id)
}

/// Compile SuperHat synth
/// Usage: superhat(bright, sustain)
fn compile_superhat(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let bright = if !args.is_empty() {
        Some(extract_number(&args[0])? as f32)
    } else {
        None
    };
    let sustain = if args.len() > 1 {
        Some(extract_number(&args[1])? as f32)
    } else {
        None
    };

    let node_id = ctx.synth_lib.build_hat(&mut ctx.graph, bright, sustain);
    Ok(node_id)
}

/// Compile envelope wrapper
/// Usage: signal # env(attack, decay, sustain, release)
/// Or: env(input, attack, decay, sustain, release)
fn compile_envelope(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 4 {
        return Err(format!(
            "env requires 4 parameters (attack, decay, sustain, release), got {}",
            params.len()
        ));
    }

    let attack = extract_number(&params[0])? as f32;
    let decay = extract_number(&params[1])? as f32;
    let sustain_level = extract_number(&params[2])? as f32;
    let release = extract_number(&params[3])? as f32;

    use crate::unified_graph::EnvState;

    // For continuous signals, use a constant trigger (always on)
    // This keeps the envelope in sustain phase
    let node = SignalNode::Envelope {
        input: input_signal,
        trigger: Signal::Value(1.0), // Always triggered for continuous signals
        attack,
        decay,
        sustain: sustain_level,
        release,
        state: EnvState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_adsr(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 4 {
        return Err(format!(
            "adsr requires 4 parameters (attack, decay, sustain, release), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let attack_node = compile_expr(ctx, args[0].clone())?;
    let decay_node = compile_expr(ctx, args[1].clone())?;
    let sustain_node = compile_expr(ctx, args[2].clone())?;
    let release_node = compile_expr(ctx, args[3].clone())?;

    use crate::unified_graph::ADSRState;

    let node = SignalNode::ADSR {
        attack: Signal::Node(attack_node),
        decay: Signal::Node(decay_node),
        sustain: Signal::Node(sustain_node),
        release: Signal::Node(release_node),
        state: ADSRState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_ad(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "ad requires 2 parameters (attack, decay), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let attack_node = compile_expr(ctx, args[0].clone())?;
    let decay_node = compile_expr(ctx, args[1].clone())?;

    use crate::unified_graph::ADState;

    let node = SignalNode::AD {
        attack: Signal::Node(attack_node),
        decay: Signal::Node(decay_node),
        state: ADState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_line(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "line requires 2 parameters (start, end), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let start_node = compile_expr(ctx, args[0].clone())?;
    let end_node = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::Line {
        start: Signal::Node(start_node),
        end: Signal::Node(end_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile chain operator: a # b
fn compile_chain(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<NodeId, String> {
    // The chain operator passes left as input to right
    // We need to handle this based on what 'right' is
    match right {
        Expr::Call { name, mut args } => {
            // Prepend left as first argument using proper ChainInput marker
            let left_node = compile_expr(ctx, left)?;
            args.insert(0, Expr::ChainInput(left_node)); // Type-safe!
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
    // Handle function calls like `s "bd sn" $ fast 2`
    if let Expr::Call { name, args } = &expr {
        // Check if this is the `s` function (sample pattern)
        if name == "s" && !args.is_empty() {
            if let Expr::String(pattern_str) = &args[0] {
                // Parse and transform the pattern
                let mut pattern = parse_mini_notation(&pattern_str);
                pattern = apply_transform_to_pattern(pattern, transform)?;

                // Create Sample node with transformed pattern
                let node = SignalNode::Sample {
                    pattern_str: format!("{} (transformed)", pattern_str),
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
                return Ok(ctx.graph.add_node(node));
            }
        }
    }

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
        Transform::Shuffle(amount_expr) => {
            let amount = extract_number(&amount_expr)?;
            Ok(pattern.shuffle(amount))
        }
        Transform::Chop(n_expr) | Transform::Striate(n_expr) => {
            // chop and striate are aliases - both slice pattern into n parts
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.chop(n))
        }
        Transform::Scramble(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.scramble(n))
        }
        Transform::Swing(amount_expr) => {
            let amount = extract_number(&amount_expr)?;
            Ok(pattern.swing(amount))
        }
        Transform::Legato(factor_expr) => {
            let factor = extract_number(&factor_expr)?;
            Ok(pattern.legato(factor))
        }
        Transform::Staccato(factor_expr) => {
            let factor = extract_number(&factor_expr)?;
            Ok(pattern.staccato(factor))
        }
        Transform::Echo {
            times,
            time,
            feedback,
        } => {
            let times_val = extract_number(&times)? as usize;
            let time_val = extract_number(&time)?;
            let feedback_val = extract_number(&feedback)?;
            Ok(pattern.echo(times_val, time_val, feedback_val))
        }
        Transform::Segment(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.segment(n))
        }
        Transform::Zoom { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.zoom(begin_val, end_val))
        }
        Transform::Compress { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.compress(begin_val, end_val))
        }
        Transform::Spin(n_expr) => {
            let n = extract_number(&n_expr)? as i32;
            Ok(pattern.spin(n))
        }
        Transform::Mirror => Ok(pattern.mirror()),
        Transform::Gap(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.gap(n))
        }
        Transform::Late(amount_expr) => {
            let amount = extract_number(&amount_expr)?;
            Ok(pattern.late(amount))
        }
        Transform::Early(amount_expr) => {
            let amount = extract_number(&amount_expr)?;
            Ok(pattern.early(amount))
        }
        Transform::Dup(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.dup(n))
        }
        Transform::Fit(n_expr) => {
            let n = extract_number(&n_expr)? as i32;
            Ok(pattern.fit(n))
        }
        Transform::Stretch => Ok(pattern.stretch()),
        Transform::Every { n, transform } => {
            // Extract the cycle interval
            let n_val = extract_number(&n)? as i32;

            // Clone the pattern and transform for use in the closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            // Manually inline Pattern::every logic to avoid closure issues
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as i32;
                if cycle % n_val == 0 {
                    // Apply the transform on cycles divisible by n
                    match apply_transform_to_pattern(pattern_clone.clone(), inner_transform.clone())
                    {
                        Ok(transformed) => transformed.query(state),
                        Err(_) => pattern_clone.query(state), // Fallback to original on error
                    }
                } else {
                    // Use original pattern on other cycles
                    pattern_clone.query(state)
                }
            }))
        }
        Transform::RotL(n_expr) => {
            let n = extract_number(&n_expr)?;
            Ok(pattern.rotate_left(n))
        }
        Transform::RotR(n_expr) => {
            let n = extract_number(&n_expr)?;
            Ok(pattern.rotate_right(n))
        }
        Transform::Iter(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.iter(n))
        }
        Transform::IterBack(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.iter_back(n))
        }
        Transform::Ply(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.ply(n))
        }
        Transform::Linger(factor_expr) => {
            let factor = extract_number(&factor_expr)?;
            Ok(pattern.linger(factor))
        }
        Transform::Offset(amount_expr) => {
            let amount = extract_number(&amount_expr)?;
            Ok(pattern.offset(amount))
        }
        Transform::Loop(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.loop_pattern(n))
        }
        Transform::Chew(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.chew(n))
        }
        Transform::FastGap(factor_expr) => {
            let factor = extract_number(&factor_expr)?;
            Ok(pattern.fast_gap(factor))
        }
        Transform::Discretise(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.discretise(n))
        }
        Transform::CompressGap { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.compress_gap(begin_val, end_val))
        }
        Transform::Reset(cycles_expr) => {
            let cycles = extract_number(&cycles_expr)? as i32;
            Ok(pattern.reset(cycles))
        }
        Transform::Restart(n_expr) => {
            let n = extract_number(&n_expr)? as i32;
            Ok(pattern.restart(n))
        }
        Transform::Loopback => Ok(pattern.loopback()),
        Transform::Binary(n_expr) => {
            let n = extract_number(&n_expr)? as u32;
            Ok(pattern.binary(n))
        }
        Transform::Range { min, max } => {
            // Note: range() only works on Pattern<f64>, not Pattern<T>
            // This will fail to compile if applied to non-numeric patterns
            // We need to handle this specially
            Err("range transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Quantize(_steps_expr) => {
            // Note: quantize() only works on Pattern<f64>, not Pattern<T>
            Err("quantize transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Focus {
            cycle_begin,
            cycle_end,
        } => {
            let begin_val = extract_number(&cycle_begin)?;
            let end_val = extract_number(&cycle_end)?;
            Ok(pattern.focus(begin_val, end_val))
        }
        Transform::Smooth(_amount_expr) => {
            // Note: smooth() only works on Pattern<f64>, not Pattern<T>
            Err("smooth transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Trim { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.trim(begin_val, end_val))
        }
        Transform::Exp(_base_expr) => {
            // Note: exp() only works on Pattern<f64>, not Pattern<T>
            Err("exp transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Log(_base_expr) => {
            // Note: log() only works on Pattern<f64>, not Pattern<T>
            Err("log transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Walk(_step_expr) => {
            // Note: walk() only works on Pattern<f64>, not Pattern<T>
            Err("walk transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Inside {
            begin,
            end,
            transform,
        } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            // Clone pattern and transform for use in closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(Pattern::new(move |state| {
                let cycle_phase = state.span.begin.to_float() % 1.0;
                if cycle_phase >= begin_val && cycle_phase < end_val {
                    // Inside the range: apply transform
                    match apply_transform_to_pattern(pattern_clone.clone(), inner_transform.clone())
                    {
                        Ok(transformed) => transformed.query(state),
                        Err(_) => pattern_clone.query(state),
                    }
                } else {
                    // Outside the range: use original
                    pattern_clone.query(state)
                }
            }))
        }
        Transform::Outside {
            begin,
            end,
            transform,
        } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            // Clone pattern and transform for use in closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(Pattern::new(move |state| {
                let cycle_phase = state.span.begin.to_float() % 1.0;
                if cycle_phase < begin_val || cycle_phase >= end_val {
                    // Outside the range: apply transform
                    match apply_transform_to_pattern(pattern_clone.clone(), inner_transform.clone())
                    {
                        Ok(transformed) => transformed.query(state),
                        Err(_) => pattern_clone.query(state),
                    }
                } else {
                    // Inside the range: use original
                    pattern_clone.query(state)
                }
            }))
        }
        Transform::Superimpose(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.superimpose(move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Chunk { n, transform } => {
            let n_val = extract_number(&n)? as usize;
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.chunk(n_val, move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Sometimes(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.sometimes(move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Often(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.often(move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Rarely(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.rarely(move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::SometimesBy { prob, transform } => {
            let prob_val = extract_number(&prob)?;
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.sometimes_by(prob_val, move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::AlmostAlways(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.sometimes_by(0.9, move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::AlmostNever(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.sometimes_by(0.1, move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Always(transform) => {
            let inner_transform = (*transform).clone();

            Ok(pattern.always(move |p| {
                match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(e) => panic!("Transform error in always: {}", e),
                }
            }))
        }

        Transform::Whenmod {
            modulo,
            offset,
            transform,
        } => {
            let modulo_val = extract_number(&modulo)? as i32;
            let offset_val = extract_number(&offset)? as i32;
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.when_mod(
                modulo_val,
                offset_val,
                move |p| match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                },
            ))
        }

        Transform::Wait(cycles_expr) => {
            let cycles = extract_number(&cycles_expr)?;
            // wait is an alias for late
            Ok(pattern.late(cycles))
        }
        Transform::Mask(mask_expr) => {
            // Note: mask() works with boolean patterns or patterns that can be converted to bool
            // For now, we'll just pass the error that this is not yet implemented
            Err(
                "mask transform is not yet fully implemented - requires boolean pattern argument"
                    .to_string(),
            )
        }
        Transform::Weave(count_expr) => {
            // Note: weave() expects a Pattern<T> argument, not a count
            // This needs different DSL syntax or a different operation
            Err(
                "weave transform requires a pattern argument - not yet exposed to DSL in this form"
                    .to_string(),
            )
        }

        Transform::DegradeSeed(seed_expr) => {
            let seed = extract_number(&seed_expr)? as u64;
            Ok(pattern.degrade_seed(seed))
        }

        Transform::Undegrade => Ok(pattern.undegrade()),

        Transform::Accelerate(rate_expr) => {
            let rate = extract_number(&rate_expr)?;
            Ok(pattern.accelerate(rate))
        }

        Transform::Humanize {
            time_var,
            velocity_var,
        } => {
            let time_var_val = extract_number(&time_var)?;
            let velocity_var_val = extract_number(&velocity_var)?;
            Ok(pattern.humanize(time_var_val, velocity_var_val))
        }

        Transform::Within {
            begin,
            end,
            transform,
        } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();

            Ok(pattern.within(
                begin_val,
                end_val,
                move |p| match apply_transform_to_pattern(p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                },
            ))
        }

        Transform::Euclid { pulses, steps } => {
            let pulses_val = extract_number(&pulses)? as usize;
            let steps_val = extract_number(&steps)? as usize;
            Ok(pattern.euclidean_legato(pulses_val, steps_val))
        }
    }
}

/// Extract a numeric value from an expression (for transform arguments)
fn extract_number(expr: &Expr) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Paren(inner) => extract_number(inner),
        Expr::UnOp {
            op: UnOp::Neg,
            expr,
        } => {
            let value = extract_number(expr)?;
            Ok(-value)
        }
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
        // Use space-separated syntax: sine 440 (not sine(440))
        let code = "out: sine 440";
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
        // Test the supported space-separated syntax

        // Space-separated (Phonon standard)
        let code1 = r#"out: sine 440"#;
        let (_, statements) = parse_program(code1).unwrap();
        assert!(compile_program(statements, 44100.0).is_ok());

        // Parenthesized expressions as arguments
        let code2 = r#"
            ~base: 220
            out: sine (~base)
        "#;
        let (_, statements) = parse_program(code2).unwrap();
        assert!(compile_program(statements, 44100.0).is_ok());

        // Multiple arguments
        let code3 = r#"out: lpf (sine 440) 1000 0.8"#;
        let (_, statements) = parse_program(code3).unwrap();
        assert!(compile_program(statements, 44100.0).is_ok());
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
        let code = r#"out: sine 440 # lpf 1000 0.8"#;
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
        let code = r#"out: saw 220 # hpf 500 1.5"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile hpf");
    }

    #[test]
    fn test_compile_bpf() {
        let code = r#"out: square 110 # bpf 800 2.0"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile bpf");
    }

    #[test]
    fn test_compile_sample_with_filter() {
        // Samples through filters
        let code = r#"out: s "bd sn hh cp" # lpf 2000 0.5"#;
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
            out: s "hh*4 cp" # lpf ~cutoffs ~resonances
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
        let code = r#"out: saw 110 # lpf 2000 0.8 # hpf 100 0.5"#;
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
        let code = r#"out: s "bd:0 bd:1 bd:2""#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        assert!(result.is_ok(), "Failed to compile sample bank selection");
    }

    #[test]
    fn test_compile_sample_bank_with_transform() {
        // Sample bank selection with transforms
        let code = r#"out: s "bd:0*4 sn:2" $ fast 2"#;
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
        let code = r#"out: s "bd:0 sn:2 hh:1" # lpf 1000 0.8"#;
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
