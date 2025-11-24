#![allow(unused_variables)]
//! Compositional Compiler
//!
//! Compiles the clean compositional AST into the AudioNode architecture.
//! Uses block-based buffer passing for efficient DAW-style processing.

use crate::compositional_parser::{BinOp, Expr, Statement, Transform, UnOp};
use crate::mini_notation_v3::parse_mini_notation;
use crate::pattern::Pattern;
use crate::superdirt_synths::SynthLibrary;
use crate::unified_graph::{DattorroState, NodeId, Signal, SignalExpr, SignalNode, TapeDelayState, UnifiedSignalGraph, Waveform};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Use AudioNode architecture (DAW-style block processing)
/// Set to false to use legacy SignalNode architecture (sample-by-sample)
const USE_AUDIO_NODES: bool = false;

/// Metadata for SamplePatternNode (used for parameter modification)
#[derive(Clone)]
struct SampleNodeMetadata {
    pattern: Arc<Pattern<String>>,
}

/// Compilation context - tracks buses, functions, templates, and node IDs
pub struct CompilerContext {
    /// Map of bus names to node IDs
    buses: HashMap<String, NodeId>,
    /// Map of template names to their expressions
    templates: HashMap<String, Expr>,
    /// Map of function names to their definitions
    functions: HashMap<String, FunctionDef>,
    /// Effect bus definitions (name -> (effect_function, params))
    effect_buses: HashMap<String, (String, Vec<Expr>)>,
    /// Effect bus sends (bus_name -> vec of input node IDs)
    effect_bus_sends: HashMap<String, Vec<NodeId>>,
    /// The signal graph we're building (OLD architecture)
    graph: UnifiedSignalGraph,
    /// Sample rate for creating buffers
    sample_rate: f32,
    /// Synth library for pre-built synthesizers
    synth_lib: SynthLibrary,
    /// NEW: AudioNode-based graph (DAW architecture)
    audio_node_graph: crate::audio_node_graph::AudioNodeGraph,
    /// NEW: Flag to use AudioNode architecture instead of SignalNode
    use_audio_nodes: bool,
    /// NEW: Track SamplePatternNode metadata for parameter modification (AudioNode mode)
    sample_node_metadata: HashMap<usize, SampleNodeMetadata>,
    /// NEW: Pattern registry for pattern-to-pattern modulation (%pattern_name references)
    pattern_registry: HashMap<String, Pattern<f64>>,
}

/// Function definition storage
#[derive(Clone, Debug)]
struct FunctionDef {
    params: Vec<String>,
    body: Vec<Statement>,
    return_expr: Expr,
}

/// Parameter extractor - handles mixed positional and keyword arguments
/// Supports both `:name value` and `name=value` syntax
struct ParamExtractor {
    positional: Vec<Expr>,
    kwargs: HashMap<String, Expr>,
}

impl ParamExtractor {
    /// Create a new extractor from a list of arguments
    fn new(args: Vec<Expr>) -> Self {
        let mut positional = Vec::new();
        let mut kwargs = HashMap::new();

        for arg in args {
            match arg {
                Expr::Kwarg { name, value } => {
                    kwargs.insert(name, *value);
                }
                _ => {
                    positional.push(arg);
                }
            }
        }

        Self { positional, kwargs }
    }

    /// Get a required parameter (no default)
    /// Looks first at positional[index], then at kwargs[name]
    fn get_required(&self, index: usize, name: &str) -> Result<Expr, String> {
        // Try positional first
        if let Some(expr) = self.positional.get(index) {
            return Ok(expr.clone());
        }

        // Try keyword
        if let Some(expr) = self.kwargs.get(name) {
            return Ok(expr.clone());
        }

        Err(format!(
            "Missing required parameter '{}' (positional index {})",
            name, index
        ))
    }

    /// Get an optional parameter with a default value
    /// Returns positional[index] if present, else kwargs[name], else default
    fn get_optional(
        &self,
        index: usize,
        name: &str,
        default: f32,
    ) -> Expr {
        // Try positional first
        if let Some(expr) = self.positional.get(index) {
            return expr.clone();
        }

        // Try keyword
        if let Some(expr) = self.kwargs.get(name) {
            return expr.clone();
        }

        // Use default
        Expr::Number(default as f64)
    }

    /// Get count of positional arguments provided
    fn positional_count(&self) -> usize {
        self.positional.len()
    }

    /// Check if a keyword argument was provided
    fn has_kwarg(&self, name: &str) -> bool {
        self.kwargs.contains_key(name)
    }
}

impl CompilerContext {
    pub fn new(sample_rate: f32) -> Self {
        // Use const flag to determine architecture
        let use_audio_nodes = USE_AUDIO_NODES;

        Self {
            buses: HashMap::new(),
            templates: HashMap::new(),
            functions: HashMap::new(),
            effect_buses: HashMap::new(),
            effect_bus_sends: HashMap::new(),
            graph: UnifiedSignalGraph::new(sample_rate),
            sample_rate,
            synth_lib: SynthLibrary::with_sample_rate(sample_rate),
            audio_node_graph: crate::audio_node_graph::AudioNodeGraph::new(sample_rate),
            use_audio_nodes,
            sample_node_metadata: HashMap::new(),
            pattern_registry: HashMap::new(),
        }
    }

    /// Get the compiled graph (OLD architecture)
    pub fn into_graph(self) -> UnifiedSignalGraph {
        self.graph
    }

    /// Get the compiled AudioNode graph (NEW architecture)
    pub fn into_audio_node_graph(mut self) -> crate::audio_node_graph::AudioNodeGraph {
        // If using AudioNodes, perform multi-output mixing before returning
        if self.use_audio_nodes {
            self.finalize_audio_node_outputs();
        }
        self.audio_node_graph
    }

    /// Finalize AudioNode outputs by mixing numbered outputs if needed
    ///
    /// This is called before returning the AudioNodeGraph to ensure that
    /// if o1:, o2:, etc. are used without an explicit out:, they get mixed together.
    fn finalize_audio_node_outputs(&mut self) {
        // Check if there's already a main output
        if self.audio_node_graph.has_output() {
            // Main output exists, nothing to do
            return;
        }

        // Get all numbered outputs from the graph
        let numbered_outputs = self.audio_node_graph.get_numbered_outputs();

        if numbered_outputs.is_empty() {
            // No outputs at all, nothing to do
            return;
        }

        // Mix all numbered outputs together
        let mixed_output = if numbered_outputs.len() == 1 {
            // Only one output, use it directly
            numbered_outputs[0].1
        } else {
            // Multiple outputs - create AdditionNodes to mix them
            use crate::nodes::addition::AdditionNode;

            let mut result = numbered_outputs[0].1;
            for &(_channel, node_id) in &numbered_outputs[1..] {
                // Create addition node to mix this output with the accumulated result
                let add_node = Box::new(AdditionNode::new(result, node_id));
                result = self.audio_node_graph.add_audio_node(add_node);
            }
            result
        };

        // Set the mixed result as the main output
        self.audio_node_graph.set_output(mixed_output);
    }

    /// Check if using AudioNode architecture
    pub fn is_using_audio_nodes(&self) -> bool {
        self.use_audio_nodes
    }

    /// Set CPS (cycles per second)
    pub fn set_cps(&mut self, cps: f64) {
        self.graph.set_cps(cps as f32);
        self.audio_node_graph.set_tempo(cps);
    }

    /// Check if a function name is an effect
    fn is_effect_function(name: &str) -> bool {
        matches!(
            name,
            // Effects that support effect bus routing
            "reverb" | "convolve" | "convolution" | "freeze" |
            "distort" | "distortion" | "dist" |
            "delay" | "tapedelay" | "tape" | "multitap" | "pingpong" | "plate" |
            "chorus" | "flanger" |
            "compressor" | "comp" |
            "sidechain_compressor" | "sidechain_comp" | "sc_comp" |
            "expander" | "expand" |
            "bitcrush" | "coarse" |
            "djf" | "ring" |
            "tremolo" | "trem" |
            "vibrato" | "vib" |
            "phaser" | "ph" |
            "xfade" | "mix"
        )
    }

    /// Compile an effect bus by mixing all its sends and applying the effect
    fn compile_effect_bus(&mut self, bus_name: &str) -> Result<NodeId, String> {
        // Check if already compiled
        if let Some(&node_id) = self.buses.get(bus_name) {
            return Ok(node_id);
        }

        // Get effect definition
        let (effect_name, effect_args) = self
            .effect_buses
            .get(bus_name)
            .cloned()
            .ok_or_else(|| format!("Effect bus '{}' not found", bus_name))?;

        // Get all sends to this bus
        let sends = self
            .effect_bus_sends
            .get(bus_name)
            .cloned()
            .unwrap_or_default();

        if sends.is_empty() {
            return Err(format!(
                "Effect bus '{}' has no inputs. Use 'signal # {}' to send signals to it.",
                bus_name, bus_name
            ));
        }

        // Mix all sends together
        let mixed_input = if sends.len() == 1 {
            sends[0]
        } else {
            // Chain Add nodes to mix all sends
            let mut result = sends[0];
            for &send_node in &sends[1..] {
                result = self.graph.add_node(SignalNode::Add {
                    a: Signal::Node(result),
                    b: Signal::Node(send_node),
                });
            }
            result
        };

        // Apply the effect to the mixed input
        // Create args with ChainInput as first argument
        let mut full_args = vec![Expr::ChainInput(mixed_input)];
        full_args.extend(effect_args);

        let effect_node = compile_function_call(self, &effect_name, full_args)?;

        // Store the compiled effect bus
        self.buses.insert(bus_name.to_string(), effect_node);
        self.graph.add_bus(bus_name.to_string(), effect_node);

        Ok(effect_node)
    }
}

/// Compile a full program
pub fn compile_program(
    statements: Vec<Statement>,
    sample_rate: f32,
) -> Result<UnifiedSignalGraph, String> {
    let mut ctx = CompilerContext::new(sample_rate);

    // PASS 1: Pre-register all bus names with placeholder nodes
    // This allows circular dependencies (a -> b -> a)
    for statement in &statements {
        if let Statement::BusAssignment { name, .. } = statement {
            // Create a placeholder node (Constant 0.0) for this bus
            // This will be overwritten in Pass 2, but allows forward references
            let placeholder_node = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });
            ctx.buses.insert(name.clone(), placeholder_node);
            ctx.graph.add_bus(name.clone(), placeholder_node);
        }
    }

    // PASS 2: Compile all statements (can now reference any bus, including forward refs)
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
pub fn compile_statement(ctx: &mut CompilerContext, statement: Statement) -> Result<(), String> {
    match statement {
        Statement::BusAssignment { name, expr } => {
            // All bus assignments are compiled immediately as normal signal chains
            // This allows effects to be chained and used inline: ~feel: delay ... # reverb ...
            // Previous "effect bus" system was for send/return routing, which is not the current design
            if ctx.use_audio_nodes {
                // NEW: AudioNode path
                let node_id = compile_expr_audio_node(ctx, expr)?;
                ctx.buses.insert(name.clone(), NodeId(node_id));
            } else {
                // OLD: SignalNode path
                let node_id = compile_expr(ctx, expr)?;
                ctx.buses.insert(name.clone(), node_id);
                ctx.graph.add_bus(name, node_id); // Register bus in graph for auto-routing
            }
            Ok(())
        }
        Statement::TemplateAssignment { name, expr } => {
            // Store the template expression for later substitution
            ctx.templates.insert(name, expr);
            Ok(())
        }
        Statement::PatternAssignment { name, expr } => {
            // Pattern assignments create Pattern<f64> for use in transforms
            // %speed: "1 2 3 4" creates a pattern that can be used in fast %speed
            let pattern = match expr {
                Expr::String(s) => {
                    // Mini-notation pattern: %speed: "1 2 3 4"
                    let string_pattern = parse_mini_notation(&s);
                    string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0))
                }
                Expr::Number(n) => {
                    // Constant pattern: %speed: 2.0
                    Pattern::pure(n)
                }
                Expr::BusRef(bus_name) => {
                    // Audio signal as pattern: %speed: ~lfo
                    create_signal_pattern_for_transform(
                        ctx,
                        &bus_name,
                        0.0,
                        1.0,
                        &name,
                    )?
                }
                _ => {
                    return Err(format!(
                        "Pattern assignment %{}: unsupported expression type. Use string pattern, number, or bus reference.",
                        name
                    ));
                }
            };

            ctx.pattern_registry.insert(name, pattern);
            Ok(())
        }
        Statement::Output(expr) => {
            if ctx.use_audio_nodes {
                // NEW: AudioNode path
                let node_id = compile_expr_audio_node(ctx, expr)?;
                ctx.audio_node_graph.set_output(node_id);
            } else {
                // OLD: SignalNode path
                let node_id = compile_expr(ctx, expr)?;
                ctx.graph.set_output(node_id);
            }
            Ok(())
        }
        Statement::OutputChannel { channel, expr } => {
            if ctx.use_audio_nodes {
                // NEW: AudioNode path
                let node_id = compile_expr_audio_node(ctx, expr)?;
                ctx.audio_node_graph.set_numbered_output(channel, node_id);
            } else {
                // OLD: SignalNode path
                let node_id = compile_expr(ctx, expr)?;
                ctx.graph.set_output_channel(channel, node_id);
            }
            Ok(())
        }
        Statement::Tempo(cps) => {
            // tempo: value directly sets cycles per second
            // Example: tempo: 1.0 → 1 cycle per second
            ctx.set_cps(cps);
            Ok(())
        }
        Statement::Bpm {
            bpm,
            time_signature,
        } => {
            // bpm: value sets beats per minute
            // Convert to cycles per second based on time signature
            // Default time signature is 4/4 (4 beats per bar/cycle)

            let (numerator, _denominator) = time_signature.unwrap_or((4, 4));
            let beats_per_bar = numerator as f64;

            // Formula: cps = bpm / (beats_per_bar × 60)
            // Example: 120 BPM in 4/4 → 120 / (4 × 60) = 0.5 cps
            // Example: 120 BPM in 3/4 → 120 / (3 × 60) = 0.67 cps
            let cps = bpm / (beats_per_bar * 60.0);

            ctx.set_cps(cps);
            Ok(())
        }
        Statement::OutputMixMode(mode_str) => {
            // outmix: sqrt|gain|tanh|hard|none
            // Sets how multiple output channels are mixed together
            use crate::unified_graph::OutputMixMode;
            match OutputMixMode::from_str(&mode_str) {
                Some(mode) => {
                    ctx.graph.set_output_mix_mode(mode);
                    Ok(())
                }
                None => Err(format!(
                    "Invalid output mix mode '{}'. Valid modes: gain, sqrt, tanh, hard, none",
                    mode_str
                )),
            }
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
        Statement::Hush => {
            // Silence all outputs (keeps them defined but hushed)
            ctx.graph.hush_all();
            Ok(())
        }
        Statement::Panic => {
            // Stop all audio immediately (kills voices and silences outputs)
            ctx.graph.panic();
            Ok(())
        }
        Statement::ResetCycles => {
            // Reset cycle position to 0 (like Tidal's resetCycles)
            ctx.graph.reset_cycles();
            Ok(())
        }
        Statement::SetCycle(cycle) => {
            // Jump to specific cycle position
            ctx.graph.set_cycle(cycle);
            Ok(())
        }
        Statement::Nudge(amount) => {
            // Shift timing by amount (positive = delay, negative = advance)
            ctx.graph.nudge(amount);
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
            // Check if this is an effect bus
            if ctx.effect_buses.contains_key(&name) {
                // Compile the effect bus (mixing all sends)
                return ctx.compile_effect_bus(&name);
            }

            // Otherwise, look up normal bus reference
            ctx.buses
                .get(&name)
                .copied()
                .ok_or_else(|| format!("Undefined bus: ~{}", name))
        }

        Expr::TemplateRef(name) => {
            // Look up template and substitute (macro expansion)
            let template_expr = ctx
                .templates
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("Undefined template: @{}", name))?;

            // Recursively compile the template expression
            compile_expr(ctx, template_expr)
        }

        Expr::PatternRef(name) => {
            // Pattern references are only valid as transform parameters, not as signals
            Err(format!(
                "Pattern reference %{} cannot be used as a signal. Pattern references are only valid as parameters to transforms (e.g., fast %speed).",
                name
            ))
        }

        Expr::Var(name) => {
            // Check if it's a zero-argument function first
            if name == "noise" {
                return compile_noise(ctx, vec![]);
            }
            if name == "pink" {
                return compile_pink(ctx, vec![]);
            }
            if name == "white_noise" {
                return compile_white_noise(ctx, vec![]);
            }
            if name == "pink_noise" {
                return compile_pink_noise(ctx, vec![]);
            }
            if name == "brown_noise" {
                return compile_brown_noise(ctx, vec![]);
            }

            // Zero-arg oscillators = LFOs at 1 Hz (for modulation)
            if name == "sine" {
                return compile_oscillator(ctx, Waveform::Sine, vec![Expr::Number(1.0)]);
            }
            if name == "saw" {
                return compile_oscillator(ctx, Waveform::Saw, vec![Expr::Number(1.0)]);
            }
            if name == "square" {
                return compile_oscillator(ctx, Waveform::Square, vec![Expr::Number(1.0)]);
            }
            if name == "tri" || name == "triangle" {
                return compile_oscillator(ctx, Waveform::Triangle, vec![Expr::Number(1.0)]);
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

        Expr::Kwarg { name, .. } => {
            // Kwargs should only appear as function arguments
            Err(format!(
                "Keyword argument '{}' can only be used as a function argument",
                name
            ))
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

/// Compile constant value to AudioNode (NEW architecture)
///
/// Creates a ConstantNode that outputs a fixed value. Returns the node ID
/// wrapped in the unified_graph::NodeId type.
fn compile_constant_audio_node(ctx: &mut CompilerContext, value: f32) -> usize {
    use crate::nodes::constant::ConstantNode;

    let node = Box::new(ConstantNode::new(value));
    ctx.audio_node_graph.add_audio_node(node)
}

/// Compile sine oscillator to AudioNode (NEW architecture)
///
/// Creates an OscillatorNode configured for sine wave generation.
/// The frequency is provided by another node (audio_node::NodeId).
fn compile_sine_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::oscillator::{OscillatorNode, Waveform};

    if args.is_empty() {
        return Err("sine requires frequency argument".to_string());
    }

    // Compile frequency argument to get an audio_node::NodeId (usize)
    let freq_node_id = compile_expr_audio_node(ctx, args[0].clone())?;

    // Create oscillator node
    let node = Box::new(OscillatorNode::new(freq_node_id, Waveform::Sine));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile saw oscillator to AudioNode (NEW architecture)
///
/// Creates an OscillatorNode configured for sawtooth wave generation.
/// The frequency is provided by another node (audio_node::NodeId).
fn compile_saw_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::oscillator::{OscillatorNode, Waveform};

    if args.is_empty() {
        return Err("saw requires frequency argument".to_string());
    }

    let freq_node_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let node = Box::new(OscillatorNode::new(freq_node_id, Waveform::Saw));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile square oscillator to AudioNode (NEW architecture)
///
/// Creates an OscillatorNode configured for square wave generation.
/// The frequency is provided by another node (audio_node::NodeId).
fn compile_square_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::oscillator::{OscillatorNode, Waveform};

    if args.is_empty() {
        return Err("square requires frequency argument".to_string());
    }

    let freq_node_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let node = Box::new(OscillatorNode::new(freq_node_id, Waveform::Square));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile triangle oscillator to AudioNode (NEW architecture)
///
/// Creates an OscillatorNode configured for triangle wave generation.
/// The frequency is provided by another node (audio_node::NodeId).
fn compile_triangle_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::oscillator::{OscillatorNode, Waveform};

    if args.is_empty() {
        return Err("triangle requires frequency argument".to_string());
    }

    let freq_node_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let node = Box::new(OscillatorNode::new(freq_node_id, Waveform::Triangle));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile lowpass filter to AudioNode (NEW architecture)
///
/// Creates a LowPassFilterNode that attenuates frequencies above the cutoff.
/// The resonance parameter controls the Q peak at the cutoff frequency.
fn compile_lpf_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::LowPassFilterNode;

    if args.len() < 3 {
        return Err("lpf requires 3 arguments: input, cutoff, resonance".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let cutoff_id = compile_expr_audio_node(ctx, args[1].clone())?;
    let resonance_id = compile_expr_audio_node(ctx, args[2].clone())?;

    let node = Box::new(LowPassFilterNode::new(input_id, cutoff_id, resonance_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile highpass filter to AudioNode (NEW architecture)
///
/// Creates a HighPassFilterNode that attenuates frequencies below the cutoff.
/// The resonance parameter controls the Q peak at the cutoff frequency.
fn compile_hpf_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::HighPassFilterNode;

    if args.len() < 3 {
        return Err("hpf requires 3 arguments: input, cutoff, resonance".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let cutoff_id = compile_expr_audio_node(ctx, args[1].clone())?;
    let resonance_id = compile_expr_audio_node(ctx, args[2].clone())?;

    let node = Box::new(HighPassFilterNode::new(input_id, cutoff_id, resonance_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile bandpass filter to AudioNode (NEW architecture)
///
/// Creates a BandPassFilterNode that passes frequencies near the center frequency
/// while attenuating frequencies above and below. The resonance parameter controls
/// the Q (selectivity) of the filter.
fn compile_bpf_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::BandPassFilterNode;

    if args.len() < 3 {
        return Err("bpf requires 3 arguments: input, center, resonance".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let center_id = compile_expr_audio_node(ctx, args[1].clone())?;
    let resonance_id = compile_expr_audio_node(ctx, args[2].clone())?;

    let node = Box::new(BandPassFilterNode::new(input_id, center_id, resonance_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile addition to AudioNode (NEW architecture)
///
/// Creates an AdditionNode that sums two input signals sample-by-sample.
fn compile_add_audio_node(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<usize, String> {
    use crate::nodes::addition::AdditionNode;

    // Compile both operands to get audio_node::NodeIds (usize)
    let left_id = compile_expr_audio_node(ctx, left)?;
    let right_id = compile_expr_audio_node(ctx, right)?;

    // Create addition node
    let node = Box::new(AdditionNode::new(left_id, right_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile multiplication to AudioNode (NEW architecture)
///
/// Creates a MultiplicationNode that multiplies two input signals sample-by-sample.
fn compile_multiply_audio_node(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<usize, String> {
    use crate::nodes::multiplication::MultiplicationNode;

    // Compile both operands to get audio_node::NodeIds (usize)
    let left_id = compile_expr_audio_node(ctx, left)?;
    let right_id = compile_expr_audio_node(ctx, right)?;

    // Create multiplication node
    let node = Box::new(MultiplicationNode::new(left_id, right_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile subtraction to AudioNode (NEW architecture)
///
/// Creates a SubtractionNode that subtracts the right signal from the left signal sample-by-sample.
fn compile_subtract_audio_node(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<usize, String> {
    use crate::nodes::subtraction::SubtractionNode;

    // Compile both operands to get audio_node::NodeIds (usize)
    let left_id = compile_expr_audio_node(ctx, left)?;
    let right_id = compile_expr_audio_node(ctx, right)?;

    // Create subtraction node
    let node = Box::new(SubtractionNode::new(left_id, right_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile division to AudioNode (NEW architecture)
///
/// Creates a DivisionNode that divides the left signal by the right signal sample-by-sample.
/// Includes protection against division by zero.
fn compile_divide_audio_node(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<usize, String> {
    use crate::nodes::division::DivisionNode;

    // Compile both operands to get audio_node::NodeIds (usize)
    let left_id = compile_expr_audio_node(ctx, left)?;
    let right_id = compile_expr_audio_node(ctx, right)?;

    // Create division node
    let node = Box::new(DivisionNode::new(left_id, right_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile delay effect to AudioNode (NEW architecture)
///
/// Creates a DelayNode that delays the input signal.
/// The delay time is provided by another node (pattern-controllable).
fn compile_delay_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::DelayNode;

    if args.len() < 2 {
        return Err("delay requires 2 arguments: input, delay_time".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let delay_time_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Use reasonable defaults: max_delay = 2.0 seconds, sample_rate from context
    let max_delay = 2.0;
    let sample_rate = ctx.sample_rate;

    let node = Box::new(DelayNode::new(input_id, delay_time_id, max_delay, sample_rate));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile reverb effect to AudioNode (NEW architecture)
///
/// Creates a ReverbNode using Schroeder reverb algorithm.
/// Parameters: room_size, damping, wet (all pattern-controllable).
fn compile_reverb_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::ReverbNode;

    if args.len() < 4 {
        return Err("reverb requires 4 arguments: input, room_size, damping, wet".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let room_size_id = compile_expr_audio_node(ctx, args[1].clone())?;
    let damping_id = compile_expr_audio_node(ctx, args[2].clone())?;
    let wet_id = compile_expr_audio_node(ctx, args[3].clone())?;

    let node = Box::new(ReverbNode::new(input_id, room_size_id, damping_id, wet_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile distortion effect to AudioNode (NEW architecture)
///
/// Creates a DistortionNode that applies tanh waveshaping saturation.
/// Parameters: drive (1.0 to 100.0), mix (0.0 = dry, 1.0 = wet).
fn compile_distortion_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::DistortionNode;

    if args.len() < 3 {
        return Err("distortion requires 3 arguments: input, drive, mix".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let drive_id = compile_expr_audio_node(ctx, args[1].clone())?;
    let mix_id = compile_expr_audio_node(ctx, args[2].clone())?;

    let node = Box::new(DistortionNode::new(input_id, drive_id, mix_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile unipolar converter to AudioNode
///
/// Maps bipolar (-1 to 1) signals to unipolar (0 to 1) range.
fn compile_unipolar_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::UnipolarNode;

    if args.len() != 1 {
        return Err("unipolar expects 1 argument".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let node = Box::new(UnipolarNode::new(input_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile bipolar clamper to AudioNode
///
/// Clamps signals to bipolar (-1 to 1) range.
fn compile_bipolar_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::BipolarNode;

    if args.len() != 1 {
        return Err("bipolar expects 1 argument".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let node = Box::new(BipolarNode::new(input_id));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile range mapper to AudioNode
///
/// Maps input range to output range linearly.
fn compile_range_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    use crate::nodes::RangeNode;

    if args.len() != 5 {
        return Err("range expects 5 arguments: input, in_min, in_max, out_min, out_max".to_string());
    }

    let input_id = compile_expr_audio_node(ctx, args[0].clone())?;
    let in_min = extract_number(&args[1])? as f32;
    let in_max = extract_number(&args[2])? as f32;
    let out_min = extract_number(&args[3])? as f32;
    let out_max = extract_number(&args[4])? as f32;

    let node = Box::new(RangeNode::new(input_id, in_min, in_max, out_min, out_max));
    Ok(ctx.audio_node_graph.add_audio_node(node))
}

/// Compile begin modifier for AudioNode architecture: s "bd" # begin "0 0.25 0.5"
///
/// Sets the sample start point for slicing (0.0 = start, 0.5 = middle, 1.0 = end).
/// Currently not supported in AudioNode mode as sample playback uses SignalNode architecture.
fn compile_begin_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    let _ = (ctx, args); // Suppress unused warnings
    Err("Sample modifier 'begin' is not yet supported in AudioNode mode. Sample playback currently uses the SignalNode architecture. Use without AudioNode mode for now.".to_string())
}

/// Compile end modifier for AudioNode architecture: s "bd" # end "0.5 0.75 1"
///
/// Sets the sample end point for slicing (0.0 = start, 0.5 = middle, 1.0 = end).
/// Currently not supported in AudioNode mode as sample playback uses SignalNode architecture.
fn compile_end_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    let _ = (ctx, args); // Suppress unused warnings
    Err("Sample modifier 'end' is not yet supported in AudioNode mode. Sample playback currently uses the SignalNode architecture. Use without AudioNode mode for now.".to_string())
}

/// Compile loop modifier for AudioNode architecture: s "bd" # loop "1"
///
/// Sets whether the sample should loop (0 = play once, 1 = loop continuously).
/// Currently not supported in AudioNode mode as sample playback uses SignalNode architecture.
fn compile_loop_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    let _ = (ctx, args); // Suppress unused warnings
    Err("Sample modifier 'loop' is not yet supported in AudioNode mode. Sample playback currently uses the SignalNode architecture. Use without AudioNode mode for now.".to_string())
}

/// Compile cut modifier for AudioNode architecture: s "bd" # cut "1 2 1"
///
/// Sets the cut group for voice stealing (samples in same group stop each other).
/// Currently not supported in AudioNode mode as sample playback uses SignalNode architecture.
fn compile_cut_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    let _ = (ctx, args); // Suppress unused warnings
    Err("Sample modifier 'cut' is not yet supported in AudioNode mode. Sample playback currently uses the SignalNode architecture. Use without AudioNode mode for now.".to_string())
}

/// Compile unit modifier for AudioNode architecture: s "bd" # unit "c"
///
/// Sets the playback unit mode ("r" = rate mode, "c" = cycle mode).
/// Currently not supported in AudioNode mode as sample playback uses SignalNode architecture.
fn compile_unit_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    let _ = (ctx, args); // Suppress unused warnings
    Err("Sample modifier 'unit' is not yet supported in AudioNode mode. Sample playback currently uses the SignalNode architecture. Use without AudioNode mode for now.".to_string())
}

/// Compile n/note modifier for AudioNode architecture: s "bd" # n "0 1 2"
///
/// Sets the pitch offset in semitones for sample playback.
/// Creates a new SamplePatternNode with the n parameter set.
fn compile_n_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 2 {
        return Err(format!(
            "n requires 2 arguments (sample_input, n_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "n must be used with the chain operator: s \"bd\" # n \"0 1 2\"".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "n can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile the n parameter expression to get its node ID
    let n_node_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with the n parameter using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_n(n_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile gain modifier for AudioNode architecture: s "bd" # gain "0.8 0.5 1.0"
///
/// Sets the volume for each sample trigger.
/// Creates a new SamplePatternNode with the gain parameter set.
fn compile_gain_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 2 {
        return Err(format!(
            "gain requires 2 arguments (sample_input, gain_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "gain must be used with the chain operator: s \"bd\" # gain \"0.8\"".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "gain can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile the gain parameter expression to get its node ID
    let gain_node_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with the gain parameter using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_gain(gain_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile pan modifier for AudioNode architecture: s "bd" # pan "-1 1 0"
///
/// Sets the stereo pan position (-1 = left, 0 = center, 1 = right).
/// Creates a new SamplePatternNode with the pan parameter set.
fn compile_pan_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 2 {
        return Err(format!(
            "pan requires 2 arguments (sample_input, pan_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "pan must be used with the chain operator: s \"bd\" # pan \"-1 1\"".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "pan can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile the pan parameter expression to get its node ID
    let pan_node_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with the pan parameter using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_pan(pan_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile speed modifier for AudioNode architecture: s "bd" # speed "1 0.5 2"
///
/// Sets the playback speed (1.0 = normal, 0.5 = half speed, 2.0 = double speed).
/// Creates a new SamplePatternNode with the speed parameter set.
fn compile_speed_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 2 {
        return Err(format!(
            "speed requires 2 arguments (sample_input, speed_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "speed must be used with the chain operator: s \"bd\" # speed \"1 2\"".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "speed can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile the speed parameter expression to get its node ID
    let speed_node_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with the speed parameter using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_speed(speed_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile attack modifier for AudioNode architecture: s "bd" # attack "0.01 0.1"
///
/// Sets the envelope attack time in seconds for sample playback.
/// Creates a new SamplePatternNode with the attack parameter set.
fn compile_attack_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 2 {
        return Err(format!(
            "attack requires 2 arguments (sample_input, attack_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "attack must be used with the chain operator: s \"bd\" # attack \"0.01\"".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "attack can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile the attack parameter expression to get its node ID
    let attack_node_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with the attack parameter using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_attack(attack_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile release modifier for AudioNode architecture: s "bd" # release "0.1 0.5"
///
/// Sets the envelope release time in seconds for sample playback.
/// Creates a new SamplePatternNode with the release parameter set.
fn compile_release_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 2 {
        return Err(format!(
            "release requires 2 arguments (sample_input, release_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "release must be used with the chain operator: s \"bd\" # release \"0.1\"".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "release can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile the release parameter expression to get its node ID
    let release_node_id = compile_expr_audio_node(ctx, args[1].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with the release parameter using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_release(release_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile ar modifier for AudioNode architecture: s "bd" # ar 0.01 0.5
///
/// Shorthand for setting both attack and release times.
/// Creates a new SamplePatternNode with both attack and release parameters set.
fn compile_ar_modifier_audio_node(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<usize, String> {
    if args.len() != 3 {
        return Err(format!(
            "ar requires 3 arguments (sample_input, attack_time, release_time), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a SamplePatternNode
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => node_id.0,
        _ => {
            return Err(
                "ar must be used with the chain operator: s \"bd\" # ar 0.01 0.5".to_string(),
            )
        }
    };

    // Get the sample node metadata and clone the pattern
    let pattern = ctx.sample_node_metadata.get(&sample_node_id)
        .ok_or_else(|| {
            "ar can only be used with sample (s) patterns, not other signals".to_string()
        })?
        .pattern.clone();

    // Compile both parameter expressions to get their node IDs
    let attack_node_id = compile_expr_audio_node(ctx, args[1].clone())?;
    let release_node_id = compile_expr_audio_node(ctx, args[2].clone())?;

    // Get voice_manager and sample_bank from audio_node_graph
    let voice_manager = ctx.audio_node_graph.voice_manager();
    let sample_bank = ctx.audio_node_graph.sample_bank();

    // Create a new SamplePatternNode with both attack and release parameters using builder pattern
    let node = Box::new(crate::nodes::SamplePatternNode::new(
        pattern.clone(),
        voice_manager,
        sample_bank,
    ).with_attack(attack_node_id).with_release(release_node_id));

    // Add to graph and get node ID
    let new_node_id = ctx.audio_node_graph.add_audio_node(node);

    // Store metadata for the new node (for potential chaining of modifiers)
    ctx.sample_node_metadata.insert(
        new_node_id,
        SampleNodeMetadata {
            pattern: pattern.clone(),
        },
    );

    Ok(new_node_id)
}

/// Compile signal chain operator (#) for AudioNode architecture
///
/// The chain operator passes the left expression as the first argument to the right expression.
/// Example: `saw 110 # lpf 1000 0.8` becomes `lpf (saw 110) 1000 0.8`
fn compile_chain_audio_node(ctx: &mut CompilerContext, left: Expr, right: Expr) -> Result<usize, String> {
    match right {
        Expr::Call { name, mut args } => {
            // Compile left expression to get input signal
            let left_node = compile_expr_audio_node(ctx, left)?;

            // Prepend left as first argument using ChainInput marker
            args.insert(0, Expr::ChainInput(NodeId(left_node)));

            // Compile the function call with modified args
            // For now, dispatch to known functions
            // TODO: This should eventually call a generic compile_function_call_audio_node
            match name.as_str() {
                "lpf" => compile_lpf_audio_node(ctx, args),
                "hpf" => compile_hpf_audio_node(ctx, args),
                "bpf" => compile_bpf_audio_node(ctx, args),
                "delay" => compile_delay_audio_node(ctx, args),
                "reverb" => compile_reverb_audio_node(ctx, args),
                "distortion" | "dist" => compile_distortion_audio_node(ctx, args),
                "n" | "note" => compile_n_modifier_audio_node(ctx, args),
                "gain" => compile_gain_modifier_audio_node(ctx, args),
                "pan" => compile_pan_modifier_audio_node(ctx, args),
                "speed" => compile_speed_modifier_audio_node(ctx, args),
                "begin" => compile_begin_modifier_audio_node(ctx, args),
                "end" => compile_end_modifier_audio_node(ctx, args),
                "loop" => compile_loop_modifier_audio_node(ctx, args),
                "cut" => compile_cut_modifier_audio_node(ctx, args),
                "unit" => compile_unit_modifier_audio_node(ctx, args),
                "attack" => compile_attack_modifier_audio_node(ctx, args),
                "release" => compile_release_modifier_audio_node(ctx, args),
                "ar" => compile_ar_modifier_audio_node(ctx, args),
                _ => Err(format!("Chain operator: function '{}' not yet supported in AudioNode mode", name)),
            }
        }

        Expr::BusRef(bus_name) => {
            // Bus references in chains are problematic (same as old architecture)
            // For now, just return the left signal (pass-through)
            eprintln!("⚠️  Warning: Bus '~{}' used in chain - effect will be ignored", bus_name);
            eprintln!("   Workaround: Use the effect directly instead of through a bus");
            compile_expr_audio_node(ctx, left)
        }

        _ => {
            Err(format!("Chain operator: right side must be a function call or bus reference, got: {:?}", right))
        }
    }
}

/// Compile expression to AudioNode (NEW architecture)
///
/// Main dispatcher for compiling expressions using the AudioNode architecture.
/// Handles constants, oscillators (sine, saw, square, triangle), and binary operations
/// (addition, multiplication, subtraction, division).
/// Returns an audio_node::NodeId (usize) that can be used as input to other nodes.
fn compile_expr_audio_node(ctx: &mut CompilerContext, expr: Expr) -> Result<usize, String> {
    match expr {
        Expr::Number(n) => {
            Ok(compile_constant_audio_node(ctx, n as f32))
        }

        Expr::BusRef(name) => {
            // Look up bus in the buses HashMap
            ctx.buses
                .get(&name)
                .map(|node_id| node_id.0) // Extract usize from NodeId wrapper
                .ok_or_else(|| format!("Bus '{}' not found", name))
        }

        Expr::PatternRef(name) => {
            // Pattern references are only valid as transform parameters, not as signals
            Err(format!(
                "Pattern reference %{} cannot be used as a signal. Pattern references are only valid as parameters to transforms (e.g., fast %speed).",
                name
            ))
        }

        Expr::Call { name, args } if name == "sine" => {
            compile_sine_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "saw" => {
            compile_saw_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "square" => {
            compile_square_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "triangle" || name == "tri" => {
            compile_triangle_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "lpf" => {
            compile_lpf_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "hpf" => {
            compile_hpf_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "bpf" => {
            compile_bpf_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "delay" => {
            compile_delay_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "reverb" => {
            compile_reverb_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "distortion" || name == "dist" => {
            compile_distortion_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "unipolar" => {
            compile_unipolar_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "bipolar" => {
            compile_bipolar_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "range" => {
            compile_range_audio_node(ctx, args)
        }

        Expr::Call { name, args } if name == "s" => {
            // Sample playback function: s "bd sn hh cp"
            if args.len() != 1 {
                return Err(format!("s function expects 1 argument (pattern string), got {}", args.len()));
            }

            // Extract the string argument and compile it as Expr::String
            compile_expr_audio_node(ctx, args[0].clone())
        }

        Expr::BinOp { op: BinOp::Add, left, right } => {
            compile_add_audio_node(ctx, *left, *right)
        }

        Expr::BinOp { op: BinOp::Mul, left, right } => {
            compile_multiply_audio_node(ctx, *left, *right)
        }

        Expr::BinOp { op: BinOp::Sub, left, right } => {
            compile_subtract_audio_node(ctx, *left, *right)
        }

        Expr::BinOp { op: BinOp::Div, left, right } => {
            compile_divide_audio_node(ctx, *left, *right)
        }

        Expr::Chain(left, right) => {
            compile_chain_audio_node(ctx, *left, *right)
        }

        Expr::ChainInput(node_id) => {
            // ChainInput is used internally by chain operator
            // Extract usize from NodeId wrapper
            Ok(node_id.0)
        }

        Expr::Paren(inner) => {
            // Parenthesized expression - just compile the inner expression
            compile_expr_audio_node(ctx, *inner)
        }

        Expr::String(pattern_str) => {
            // Pattern string - create a SamplePatternNode
            // Example: "bd sn hh cp" or s "bd sn hh cp"

            // Parse mini-notation to create Pattern<String>
            let pattern = parse_mini_notation(&pattern_str);
            let pattern = Arc::new(pattern);

            // Get voice_manager and sample_bank from audio_node_graph
            let voice_manager = ctx.audio_node_graph.voice_manager();
            let sample_bank = ctx.audio_node_graph.sample_bank();

            // Create SamplePatternNode
            let node = Box::new(crate::nodes::SamplePatternNode::new(
                pattern.clone(),
                voice_manager,
                sample_bank,
            ));

            // Add to graph and get node ID
            let node_id = ctx.audio_node_graph.add_audio_node(node);

            // Store metadata for potential parameter modification
            ctx.sample_node_metadata.insert(
                node_id,
                SampleNodeMetadata {
                    pattern: pattern.clone(),
                },
            );

            Ok(node_id)
        }

        Expr::Transform { expr, transform } => {
            // Pattern transform - apply transform to pattern, then compile
            // Example: "bd sn" $ fast 2

            // Check if inner expr is a String (pattern)
            match expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Parse mini-notation to create Pattern<String>
                    let mut pattern = parse_mini_notation(&pattern_str);

                    // Apply transform using the helper function
                    pattern = apply_transform_to_pattern(ctx, pattern, transform.clone())?;

                    // Wrap in Arc
                    let pattern = Arc::new(pattern);

                    // Get voice_manager and sample_bank from audio_node_graph
                    let voice_manager = ctx.audio_node_graph.voice_manager();
                    let sample_bank = ctx.audio_node_graph.sample_bank();

                    // Create SamplePatternNode
                    let node = Box::new(crate::nodes::SamplePatternNode::new(
                        pattern.clone(),
                        voice_manager,
                        sample_bank,
                    ));

                    // Add to graph and get node ID
                    let node_id = ctx.audio_node_graph.add_audio_node(node);

                    // Store metadata for potential parameter modification
                    ctx.sample_node_metadata.insert(
                        node_id,
                        SampleNodeMetadata {
                            pattern: pattern.clone(),
                        },
                    );

                    Ok(node_id)
                }
                _ => {
                    // For non-pattern expressions, compile without transform
                    // (transforms only apply to patterns)
                    eprintln!("⚠️  Transform on non-pattern expression not yet supported: {:?}", transform);
                    compile_expr_audio_node(ctx, *expr)
                }
            }
        }

        Expr::Call { name, args } => {
            // For function calls not explicitly handled above, try compile_function_call
            // which returns a NodeId (SignalNode). Extract the usize from NodeId.
            let node_id = compile_function_call(ctx, &name, args)?;
            Ok(node_id.0) // Convert NodeId to usize
        }

        _ => Err(format!("AudioNode compilation not yet implemented for: {:?}", expr)),
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
        Expr::Transform {
            expr: inner,
            transform,
        } => Expr::Transform {
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
        "wedge" => compile_wedge(ctx, args),
        "sew" => compile_sew(ctx, args),

        // ========== Sample playback ==========
        "s" => {
            if args.is_empty() {
                return Err("s() requires at least one argument".to_string());
            }

            // Separate positional args from kwargs
            let (positional_args, kwargs): (Vec<_>, Vec<_>) = args
                .into_iter()
                .partition(|arg| !matches!(arg, Expr::Kwarg { .. }));

            // Handle different argument types:
            // 1. Simple string: s "bd"
            // 2. Parenthesized transform: s ("bd" $ fast 2)
            // 3. Direct transform via $: s "bd" $ rev $ fast 2
            //    This creates: Call { name: "s", args: [String("bd"), Transform{...}] }
            // 4. With kwargs: s "bd" gain="0.5 1.0" pan=~lfo
            let (pattern_str, pattern) = if positional_args.len() >= 2 {
                // Case 3: s "pattern" $ transform(s)
                // args[0] is the pattern, args[1..] are transforms applied via $
                if let Expr::String(base_str) = &positional_args[0] {
                    // Start with base pattern
                    let mut pattern = parse_mini_notation(base_str);

                    // Apply transforms from args[1..]
                    for transform_expr in &positional_args[1..] {
                        match transform_expr {
                            Expr::Transform { expr, transform } => {
                                // Extract all nested transforms
                                fn extract_transforms_from_chain(
                                    expr: &Expr,
                                    transforms: &mut Vec<Transform>,
                                ) -> Result<(), String> {
                                    match expr {
                                        Expr::Transform {
                                            expr: inner_expr,
                                            transform,
                                        } => {
                                            transforms.push(transform.clone());
                                            extract_transforms_from_chain(inner_expr, transforms)
                                        }
                                        Expr::Call { name, args } => {
                                            // Convert Call to Transform
                                            let t = match name.as_str() {
                                                "fast" if args.len() == 1 => {
                                                    Transform::Fast(Box::new(args[0].clone()))
                                                }
                                                "slow" if args.len() == 1 => {
                                                    Transform::Slow(Box::new(args[0].clone()))
                                                }
                                                "squeeze" if args.len() == 1 => {
                                                    Transform::Squeeze(Box::new(args[0].clone()))
                                                }
                                                "rev" if args.is_empty() => Transform::Rev,
                                                "palindrome" if args.is_empty() => {
                                                    Transform::Palindrome
                                                }
                                                "degrade" if args.is_empty() => Transform::Degrade,
                                                "degradeBy" if args.len() == 1 => {
                                                    Transform::DegradeBy(Box::new(args[0].clone()))
                                                }
                                                "stutter" if args.len() == 1 => {
                                                    Transform::Stutter(Box::new(args[0].clone()))
                                                }
                                                "shuffle" if args.len() == 1 => {
                                                    Transform::Shuffle(Box::new(args[0].clone()))
                                                }
                                                "fastGap" if args.len() == 1 => {
                                                    Transform::FastGap(Box::new(args[0].clone()))
                                                }
                                                "iter" if args.len() == 1 => {
                                                    Transform::Iter(Box::new(args[0].clone()))
                                                }
                                                "loopAt" if args.len() == 1 => {
                                                    Transform::LoopAt(Box::new(args[0].clone()))
                                                }
                                                "early" if args.len() == 1 => {
                                                    Transform::Early(Box::new(args[0].clone()))
                                                }
                                                "slice" if args.len() == 2 => {
                                                    Transform::Slice {
                                                        n: Box::new(args[0].clone()),
                                                        indices: Box::new(args[1].clone()),
                                                    }
                                                }
                                                "late" if args.len() == 1 => {
                                                    Transform::Late(Box::new(args[0].clone()))
                                                }
                                                _ => {
                                                    return Err(format!(
                                                        "Unknown transform: {}",
                                                        name
                                                    ))
                                                }
                                            };
                                            transforms.push(t);
                                            Ok(())
                                        }
                                        Expr::String(_) => Ok(()), // Base case - no more transforms
                                        _ => Err(format!(
                                            "Unexpected expression in transform chain: {:?}",
                                            expr
                                        )),
                                    }
                                }

                                let mut transforms = vec![transform.clone()];
                                extract_transforms_from_chain(expr, &mut transforms)?;

                                // Apply transforms in reverse order (innermost first)
                                for t in transforms.iter().rev() {
                                    pattern = apply_transform_to_pattern(ctx, pattern, t.clone())?;
                                }
                            }
                            Expr::Call { name, args } => {
                                // Handle Call expressions as transforms (e.g., s "bd" $ squeeze 3)
                                let transform = match name.as_str() {
                                    "fast" if args.len() == 1 => {
                                        Transform::Fast(Box::new(args[0].clone()))
                                    }
                                    "slow" if args.len() == 1 => {
                                        Transform::Slow(Box::new(args[0].clone()))
                                    }
                                    "hurry" if args.len() == 1 => {
                                        Transform::Hurry(Box::new(args[0].clone()))
                                    }
                                    "squeeze" if args.len() == 1 => {
                                        Transform::Squeeze(Box::new(args[0].clone()))
                                    }
                                    "rev" if args.is_empty() => Transform::Rev,
                                    "palindrome" if args.is_empty() => Transform::Palindrome,
                                    "degrade" if args.is_empty() => Transform::Degrade,
                                    "degradeBy" if args.len() == 1 => {
                                        Transform::DegradeBy(Box::new(args[0].clone()))
                                    }
                                    "slice" if args.len() == 2 => {
                                        Transform::Slice {
                                            n: Box::new(args[0].clone()),
                                            indices: Box::new(args[1].clone()),
                                        }
                                    }
                                    "stutter" if args.len() == 1 => {
                                        Transform::Stutter(Box::new(args[0].clone()))
                                    }
                                    "loopAt" if args.len() == 1 => {
                                        Transform::LoopAt(Box::new(args[0].clone()))
                                    }
                                    _ => return Err(format!("Unknown transform: {}", name)),
                                };
                                pattern = apply_transform_to_pattern(ctx, pattern, transform)?;
                            }
                            Expr::TemplateRef(name) => {
                                // Handle template references (e.g., s "bd" $ @swing)
                                pattern = apply_transform_to_pattern(ctx, pattern, Transform::TemplateRef(name.clone()))?;
                            }
                            _ => {
                                return Err(format!(
                                    "Expected transform as second argument to s(), got: {:?}",
                                    transform_expr
                                ))
                            }
                        }
                    }

                    (format!("{} (transformed)", base_str), pattern)
                } else {
                    return Err("First argument to s() must be a string pattern".to_string());
                }
            } else {
                // Single argument cases
                match &positional_args[0] {
                    Expr::String(s) => {
                        // Simple case: just a pattern string
                        (s.clone(), parse_mini_notation(s))
                    }
                    Expr::Call { name, args } if name == "choose" => {
                        // Handle choose function: s (choose ["bd", "sn", "hh"])
                        if args.len() != 1 {
                            return Err("choose requires exactly one list argument".to_string());
                        }

                        match &args[0] {
                            Expr::List(options) => {
                                // Extract string options from list
                                let string_options: Result<Vec<String>, String> = options
                                    .iter()
                                    .map(|expr| match expr {
                                        Expr::String(s) => Ok(s.clone()),
                                        _ => Err("choose requires a list of strings".to_string()),
                                    })
                                    .collect();

                                let options_vec = string_options?;
                                if options_vec.is_empty() {
                                    return Err("choose requires at least one option".to_string());
                                }

                                // Create pattern using Pattern::choose()
                                let pattern = Pattern::choose(options_vec.clone());
                                (format!("choose {:?}", options_vec), pattern)
                            }
                            _ => return Err("choose requires a list argument: choose [\"bd\", \"sn\", \"hh\"]".to_string()),
                        }
                    }
                    Expr::Call { name, args } if name == "wchoose" => {
                        // Handle wchoose: s (wchoose [["bd", 3], ["sn", 1]])
                        if args.len() != 1 {
                            return Err("wchoose requires exactly one list argument".to_string());
                        }

                        match &args[0] {
                            Expr::List(pairs) => {
                                let weighted_options: Result<Vec<(String, f64)>, String> = pairs
                                    .iter()
                                    .map(|pair_expr| match pair_expr {
                                        Expr::List(pair) if pair.len() == 2 => {
                                            match (&pair[0], &pair[1]) {
                                                (Expr::String(s), Expr::Number(w)) => {
                                                    Ok((s.clone(), *w))
                                                }
                                                _ => Err("wchoose pairs must be [string, number]".to_string()),
                                            }
                                        }
                                        _ => Err("wchoose requires list of [value, weight] pairs".to_string()),
                                    })
                                    .collect();

                                let options_vec = weighted_options?;
                                if options_vec.is_empty() {
                                    return Err("wchoose requires at least one option".to_string());
                                }

                                let pattern = Pattern::wchoose(options_vec.clone());
                                (format!("wchoose {:?}", options_vec), pattern)
                            }
                            _ => return Err("wchoose requires a list argument".to_string()),
                        }
                    }
                    Expr::Paren(inner) => {
                        // Unwrap parentheses and check for transform
                        match &**inner {
                            Expr::Transform { expr, transform } => {
                                // Recursively extract base pattern and apply all transforms
                                fn extract_pattern_and_transforms(
                                    expr: &Expr,
                                    transforms: &mut Vec<Transform>,
                                ) -> Result<String, String> {
                                    match expr {
                                        Expr::String(s) => Ok(s.clone()),
                                        Expr::Transform {
                                            expr: inner_expr,
                                            transform,
                                        } => {
                                            // Collect transforms in reverse order (innermost first)
                                            transforms.push(transform.clone());
                                            extract_pattern_and_transforms(inner_expr, transforms)
                                        }
                                        // Handle Call expressions that are actually transforms
                                        // This handles cases like "rev $ fast 2" where "fast 2" is parsed as a Call
                                        Expr::Call { name, args } => {
                                            let transform = match name.as_str() {
                                            "fast" if args.len() == 1 => Transform::Fast(Box::new(args[0].clone())),
                                            "slow" if args.len() == 1 => Transform::Slow(Box::new(args[0].clone())),
                                            "squeeze" if args.len() == 1 => Transform::Squeeze(Box::new(args[0].clone())),
                                            "rev" if args.is_empty() => Transform::Rev,
                                            "palindrome" if args.is_empty() => Transform::Palindrome,
                                            "degrade" if args.is_empty() => Transform::Degrade,
                                            "degradeBy" if args.len() == 1 => Transform::DegradeBy(Box::new(args[0].clone())),
                                            "stutter" if args.len() == 1 => Transform::Stutter(Box::new(args[0].clone())),
                                            "shuffle" if args.len() == 1 => Transform::Shuffle(Box::new(args[0].clone())),
                                            "fastGap" if args.len() == 1 => Transform::FastGap(Box::new(args[0].clone())),
                                            "slice" if args.len() == 2 => Transform::Slice {
                                                n: Box::new(args[0].clone()),
                                                indices: Box::new(args[1].clone()),
                                            },
                                            "iter" if args.len() == 1 => Transform::Iter(Box::new(args[0].clone())),
                                            "loopAt" if args.len() == 1 => Transform::LoopAt(Box::new(args[0].clone())),
                                            "early" if args.len() == 1 => Transform::Early(Box::new(args[0].clone())),
                                            "late" if args.len() == 1 => Transform::Late(Box::new(args[0].clone())),
                                            _ => return Err(format!("Unknown transform or invalid call in transform chain: {}", name)),
                                        };
                                            transforms.push(transform);
                                            // A Call that's a transform has no inner pattern - it's a leaf
                                            // Return empty string as placeholder
                                            Ok("".to_string())
                                        }
                                        _ => Err("s() pattern must be a string or transform chain"
                                            .to_string()),
                                    }
                                }

                                let mut transforms = vec![transform.clone()];
                                let base_str =
                                    extract_pattern_and_transforms(&**expr, &mut transforms)?;

                                // Parse base pattern
                                let mut pattern = parse_mini_notation(&base_str);

                                // Apply transforms in reverse order (innermost first)
                                for t in transforms.iter().rev() {
                                    pattern = apply_transform_to_pattern(ctx, pattern, t.clone())?;
                                }

                                (format!("{} (transformed)", base_str), pattern)
                            }
                            Expr::String(s) => {
                                // Just a parenthesized string
                                (s.clone(), parse_mini_notation(s))
                            }
                            Expr::Call { name, args } if name == "choose" => {
                                // Handle choose: s (choose ["bd", "sn", "hh"])
                                if args.len() != 1 {
                                    return Err("choose requires exactly one list argument".to_string());
                                }

                                match &args[0] {
                                    Expr::List(options) => {
                                        let string_options: Result<Vec<String>, String> = options
                                            .iter()
                                            .map(|expr| match expr {
                                                Expr::String(s) => Ok(s.clone()),
                                                _ => Err("choose requires a list of strings".to_string()),
                                            })
                                            .collect();

                                        let options_vec = string_options?;
                                        if options_vec.is_empty() {
                                            return Err("choose requires at least one option".to_string());
                                        }

                                        let pattern = Pattern::choose(options_vec.clone());
                                        (format!("choose {:?}", options_vec), pattern)
                                    }
                                    _ => return Err("choose requires a list argument".to_string()),
                                }
                            }
                            Expr::Call { name, args } if name == "wchoose" => {
                                // Handle wchoose: s (wchoose [["bd", 3], ["sn", 1]])
                                if args.len() != 1 {
                                    return Err("wchoose requires exactly one list argument".to_string());
                                }

                                match &args[0] {
                                    Expr::List(pairs) => {
                                        let weighted_options: Result<Vec<(String, f64)>, String> = pairs
                                            .iter()
                                            .map(|pair_expr| match pair_expr {
                                                Expr::List(pair) if pair.len() == 2 => {
                                                    // Extract value (must be string) and weight (must be number)
                                                    match (&pair[0], &pair[1]) {
                                                        (Expr::String(s), Expr::Number(w)) => {
                                                            Ok((s.clone(), *w))
                                                        }
                                                        _ => Err("wchoose pairs must be [string, number]".to_string()),
                                                    }
                                                }
                                                _ => Err("wchoose requires list of [value, weight] pairs".to_string()),
                                            })
                                            .collect();

                                        let options_vec = weighted_options?;
                                        if options_vec.is_empty() {
                                            return Err("wchoose requires at least one option".to_string());
                                        }

                                        let pattern = Pattern::wchoose(options_vec.clone());
                                        (format!("wchoose {:?}", options_vec), pattern)
                                    }
                                    _ => return Err("wchoose requires a list argument".to_string()),
                                }
                            }
                            _ => {
                                return Err(
                                    "s() requires a pattern string or transform as first argument"
                                        .to_string(),
                                )
                            }
                        }
                    }
                    Expr::Transform { expr, transform } => {
                        // Direct transform - recursively handle chained transforms
                        fn extract_pattern_and_transforms(
                            expr: &Expr,
                            transforms: &mut Vec<Transform>,
                        ) -> Result<String, String> {
                            match expr {
                                Expr::String(s) => Ok(s.clone()),
                                Expr::Transform {
                                    expr: inner_expr,
                                    transform,
                                } => {
                                    // Collect transforms in reverse order (innermost first)
                                    transforms.push(transform.clone());
                                    extract_pattern_and_transforms(inner_expr, transforms)
                                }
                                _ => {
                                    Err("s() pattern must be a string or transform chain"
                                        .to_string())
                                }
                            }
                        }

                        let mut transforms = vec![transform.clone()];
                        let base_str = extract_pattern_and_transforms(&**expr, &mut transforms)?;

                        // Parse base pattern
                        let mut pattern = parse_mini_notation(&base_str);

                        // Apply transforms in reverse order (innermost first)
                        for t in transforms.iter().rev() {
                            pattern = apply_transform_to_pattern(ctx, pattern, t.clone())?;
                        }

                        (format!("{} (transformed)", base_str), pattern)
                    }
                    _ => return Err("s() requires a pattern string as first argument".to_string()),
                }
            };

            // Process kwargs to set sample parameters
            let mut gain = Signal::Value(1.0);
            let mut pan = Signal::Value(0.0);
            let mut speed = Signal::Value(1.0);
            let mut cut_group = Signal::Value(0.0);
            let mut n = Signal::Value(0.0);
            let mut note = Signal::Value(0.0);
            let mut attack = Signal::Value(0.0);
            let mut release = Signal::Value(0.0);
            let mut unit_mode = Signal::Value(0.0); // 0 = rate mode (default)
            let mut loop_enabled = Signal::Value(0.0); // 0 = no loop (default)
            let mut begin = Signal::Value(0.0); // 0.0 = start of sample
            let mut end = Signal::Value(1.0);   // 1.0 = end of sample

            for kwarg in kwargs {
                if let Expr::Kwarg { name, value } = kwarg {
                    // Assign to appropriate parameter
                    match name.as_str() {
                        "unit" => {
                            // Convert string "r"/"c" to numeric: 0=rate, 1=cycle
                            if let Expr::String(s) = *value {
                                let mode_val = if s == "c" || s == "C" { 1.0 } else { 0.0 };
                                unit_mode = Signal::Value(mode_val);
                            } else {
                                // Pattern or expression
                                let value_node_id = compile_expr(ctx, *value)?;
                                unit_mode = Signal::Node(value_node_id);
                            }
                        }
                        "loop" => {
                            // Compile the loop value expression
                            let value_node_id = compile_expr(ctx, *value)?;
                            loop_enabled = Signal::Node(value_node_id);
                        }
                        _ => {
                            // Compile the value expression to a node
                            let value_node_id = compile_expr(ctx, *value)?;
                            let signal = Signal::Node(value_node_id);

                            match name.as_str() {
                                "gain" => gain = signal,
                                "pan" => pan = signal,
                                "speed" => speed = signal,
                                "cut" | "cut_group" => cut_group = signal,
                                "n" => n = signal,
                                "note" => note = signal,
                                "attack" => attack = signal,
                                "release" => release = signal,
                                "begin" => begin = signal,
                                "end" => end = signal,
                                _ => return Err(format!("Unknown sample parameter: {}", name)),
                            }
                        }
                    }
                }
            }

            let node = SignalNode::Sample {
                pattern_str: pattern_str.clone(),
                pattern,
                last_trigger_time: -1.0,
                last_cycle: -1,
                playback_positions: HashMap::new(),
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                attack,
                release,
                envelope_type: None,
                unit_mode,
                loop_enabled,
                begin,
                end,
            };
            Ok(ctx.graph.add_node(node))
        }

        // ========== Oscillators (continuous) ==========
        "sine" => compile_oscillator(ctx, Waveform::Sine, args),
        "saw" => compile_oscillator(ctx, Waveform::Saw, args),
        "square" => compile_oscillator(ctx, Waveform::Square, args),
        "tri" | "triangle" => compile_oscillator(ctx, Waveform::Triangle, args),
        "fm" => compile_fm(ctx, args),
        "pm" => compile_pm(ctx, args),
        "blip" => compile_blip(ctx, args),
        "vco" => compile_vco(ctx, args),
        "wavetable" => compile_wavetable(ctx, args),
        "granular" => compile_granular(ctx, args),
        "pluck" => compile_karplus_strong(ctx, args),
        "waveguide" => compile_waveguide(ctx, args),
        "formant" => compile_formant(ctx, args),
        "vowel" => compile_vowel(ctx, args),
        "additive" => compile_additive(ctx, args),
        "vocoder" => compile_vocoder(ctx, args),
        "pitch_shift" => compile_pitch_shift(ctx, args),
        "white_noise" => compile_white_noise(ctx, args),
        "pink_noise" => compile_pink_noise(ctx, args),
        "brown_noise" => compile_brown_noise(ctx, args),
        "impulse" => compile_impulse(ctx, args),
        "lag" => compile_lag(ctx, args),
        "xline" => compile_xline(ctx, args),
        "asr" => compile_asr(ctx, args),
        "pulse" => compile_pulse(ctx, args),
        "ring_mod" => compile_ring_mod(ctx, args),
        "fmcrossmod" | "fm_crossmod" => compile_fm_crossmod(ctx, args),
        "limiter" => compile_limiter(ctx, args),
        "pan2_l" => compile_pan2_l(ctx, args),
        "pan2_r" => compile_pan2_r(ctx, args),

        // ========== fundsp UGens ==========
        "organ_hz" | "organ" => compile_organ_hz(ctx, args),
        "moog_hz" => compile_moog_hz(ctx, args),
        "reverb_stereo" => compile_reverb_stereo(ctx, args),
        "fchorus" => compile_fundsp_chorus(ctx, args),
        "saw_hz" => compile_saw_hz(ctx, args),
        "soft_saw_hz" | "soft_saw" => compile_soft_saw_hz(ctx, args),
        "square_hz" => compile_square_hz(ctx, args),
        "triangle_hz" => compile_triangle_hz(ctx, args),
        "noise" => compile_noise(ctx, args),
        "pink" => compile_pink(ctx, args),

        // ========== Pattern-triggered synths ==========
        "sine_trig" => compile_synth_pattern(ctx, Waveform::Sine, args),
        "saw_trig" => compile_synth_pattern(ctx, Waveform::Saw, args),
        "square_trig" => compile_synth_pattern(ctx, Waveform::Square, args),
        "tri_trig" => compile_synth_pattern(ctx, Waveform::Triangle, args),

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
        "notch" => compile_filter(ctx, "notch", args),
        "comb" => compile_comb(ctx, args),
        "moog_ladder" | "moog" => compile_moog_ladder(ctx, args),
        "parametric_eq" | "eq" => compile_parametric_eq(ctx, args),

        // ========== Effects ==========
        "reverb" => compile_reverb(ctx, args),
        "convolve" | "convolution" => compile_convolve(ctx, args),
        "freeze" => compile_freeze(ctx, args),
        "distort" | "distortion" | "dist" => compile_distortion(ctx, args),
        "delay" => compile_delay(ctx, args),
        "tapedelay" | "tape" => compile_tapedelay(ctx, args),
        "multitap" => compile_multitap(ctx, args),
        "pingpong" => compile_pingpong(ctx, args),
        "plate" => compile_plate(ctx, args),
        "chorus" => compile_chorus(ctx, args),
        "flanger" => compile_flanger(ctx, args),
        "compressor" | "comp" => compile_compressor(ctx, args),
        "sidechain_compressor" | "sidechain_comp" | "sc_comp" => compile_sidechain_compressor(ctx, args),
        "expander" | "expand" => compile_expander(ctx, args),
        "bitcrush" => compile_bitcrush(ctx, args),
        "coarse" => compile_coarse(ctx, args),
        "djf" => compile_djf(ctx, args),
        "ring" => compile_ring(ctx, args),
        "tremolo" | "trem" => compile_tremolo(ctx, args),
        "vibrato" | "vib" => compile_vibrato(ctx, args),
        "phaser" | "ph" => compile_phaser(ctx, args),
        "xfade" => compile_xfade(ctx, args),
        "mix" => compile_mix(ctx, args),
        "if" => compile_if(ctx, args),
        "select" => compile_select(ctx, args),
        "allpass" => compile_allpass(ctx, args),
        "svf_lp" => compile_svf_lp(ctx, args),
        "svf_hp" => compile_svf_hp(ctx, args),
        "svf_bp" => compile_svf_bp(ctx, args),
        "svf_notch" => compile_svf_notch(ctx, args),
        "bq_lp" => compile_bq_lp(ctx, args),
        "bq_hp" => compile_bq_hp(ctx, args),
        "bq_bp" => compile_bq_bp(ctx, args),
        "bq_notch" => compile_bq_notch(ctx, args),
        "resonz" => compile_resonz(ctx, args),
        "rlpf" => compile_rlpf(ctx, args),
        "rhpf" => compile_rhpf(ctx, args),
        "tap" | "probe" => compile_tap(ctx, args),

        // ========== Envelope ==========
        "env" | "envelope" => compile_envelope(ctx, args),
        "env_trig" => compile_envelope_pattern(ctx, args),
        "adsr" => compile_adsr(ctx, args),
        "ad" => compile_ad(ctx, args),
        "line" => compile_line(ctx, args),
        "curve" => compile_curve(ctx, args),
        "segments" => compile_segments(ctx, args),

        // ========== Analysis ==========
        "rms" => compile_rms(ctx, args),
        "schmidt" => compile_schmidt(ctx, args),
        "latch" => compile_latch(ctx, args),
        "timer" => compile_timer(ctx, args),
        "peak_follower" => compile_peak_follower(ctx, args),
        "amp_follower" => compile_amp_follower(ctx, args),

        // ========== Sample Parameter Modifiers ==========
        "n" => compile_n_modifier(ctx, args),
        "note" => compile_note_modifier(ctx, args),
        "gain" => compile_gain_modifier(ctx, args),
        "pan" => compile_pan_modifier(ctx, args),
        "speed" => compile_speed_modifier(ctx, args),
        "cut" => compile_cut_modifier(ctx, args),
        "attack" => compile_attack_modifier(ctx, args),
        "release" => compile_release_modifier(ctx, args),
        "ar" => compile_ar_modifier(ctx, args),
        "begin" => compile_begin_modifier(ctx, args),
        "end" => compile_end_modifier(ctx, args),
        "unit" => compile_unit_modifier(ctx, args),
        "loop" => compile_loop_modifier(ctx, args),

        // General amplitude modifier for any signal (oscillators, filters, etc.)
        "amp" => compile_amp(ctx, args),

        // ========== Pattern Structure ==========
        "struct" => compile_struct(ctx, args),

        // ========== Pattern Generators (Numeric) ==========
        "run" => compile_run(ctx, args),
        "scan" => compile_scan(ctx, args),
        "irand" => compile_irand(ctx, args),
        "rand" => compile_rand(ctx, args),
        "sine" => compile_sine_wave(ctx, args),
        "cosine" => compile_cosine_wave(ctx, args),
        "saw" => compile_saw_wave(ctx, args),
        "tri" => compile_tri_wave(ctx, args),
        "square" => compile_square_wave(ctx, args),

        // ========== Conditional Value Generators ==========
        "every_val" => compile_every_val(ctx, args),
        "sometimes_val" => compile_sometimes_val(ctx, args),
        "sometimes_by_val" => compile_sometimes_by_val(ctx, args),
        "whenmod_val" => compile_whenmod_val(ctx, args),

        // ========== Conditional Effects ==========
        "every_effect" => compile_every_effect(ctx, args),
        "sometimes_effect" => compile_sometimes_effect(ctx, args),
        "whenmod_effect" => compile_whenmod_effect(ctx, args),

        // ========== Signal Utilities ==========
        "range" => compile_range(ctx, args),
        "gain" => compile_gain(ctx, args),
        "pan" => compile_pan(ctx, args),
        "min" => compile_min(ctx, args),
        "wrap" => compile_wrap(ctx, args),
        "sample_hold" => compile_sample_hold(ctx, args),
        "decimator" => compile_decimator(ctx, args),

        _ => {
            // Check if this is a common parameter modifier being used with $ instead of #
            let parameter_modifiers = [
                "speed", "gain", "pan", "note", "ar", "attack", "release",
                "begin", "end", "loop", "crush", "coarse", "cutoff", "resonance",
                "room", "size", "dry"
            ];

            if parameter_modifiers.contains(&name) {
                Err(format!(
                    "Unknown function: '{}'. Did you mean to use '#' instead of '$'?\n\
                     '{}' is a parameter modifier and must be used with the chain operator:\n\
                     Example: s \"bd\" # {} <value>\n\
                     Use '$' for pattern transforms (fast, slow, rev, etc.)",
                    name, name, name
                ))
            } else {
                Err(format!("Unknown function: {}", name))
            }
        }
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

    // Mix all nodes together using Mix node (normalizes automatically)
    // This prevents volume multiplication when stacking multiple patterns
    let signals: Vec<Signal> = nodes.iter().map(|&n| Signal::Node(n)).collect();

    let mix_node = SignalNode::Mix { signals };

    Ok(ctx.graph.add_node(mix_node))
}

/// Compile cat combinator - concatenates patterns within each cycle
/// Each pattern gets an equal division of the cycle time
/// Usage: cat [s "bd", s "sn", s "hh"]  -> plays bd (0-0.33), sn (0.33-0.66), hh (0.66-1.0)
/// Also supports: cat ["bd", "sn", "hh"] for convenience
fn compile_cat(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("cat requires a list argument".to_string());
    }

    // First argument should be a list
    let pattern_strs = match &args[0] {
        Expr::List(exprs) => {
            // Extract pattern strings from each expression
            // Supports both direct strings and s "pattern" calls
            exprs
                .iter()
                .map(|expr| match expr {
                    // Direct string: "bd"
                    Expr::String(s) => Ok(s.clone()),
                    // s "bd" call - extract the pattern string
                    Expr::Call { name, args } if name == "s" && !args.is_empty() => {
                        match &args[0] {
                            Expr::String(s) => Ok(s.clone()),
                            _ => Err("s() call in cat must have a string argument".to_string()),
                        }
                    }
                    _ => Err(
                        "cat requires strings or s calls: cat [\"bd\", \"sn\"] or cat [s \"bd\", s \"sn\"]"
                            .to_string(),
                    ),
                })
                .collect::<Result<Vec<String>, String>>()?
        }
        _ => return Err("cat requires a list as argument".to_string()),
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
        envelope_type: None,
        unit_mode: Signal::Value(0.0),    // 0 = rate mode (default)
        loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
        begin: Signal::Value(0.0),        // 0.0 = start of sample
        end: Signal::Value(1.0),          // 1.0 = end of sample
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile slowcat combinator - alternates between patterns on each cycle
/// Cycle 0 plays pattern 0, cycle 1 plays pattern 1, etc.
/// Usage: slowcat [s "bd*4", s "sn*4", s "hh*4"] -> cycle 0: bd*4, cycle 1: sn*4, cycle 2: hh*4, repeat
/// Also supports: slowcat ["bd*4", "sn*4", "hh*4"] for convenience
fn compile_slowcat(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("slowcat requires a list argument".to_string());
    }

    // First argument should be a list
    let pattern_strs = match &args[0] {
        Expr::List(exprs) => {
            // Extract pattern strings from each expression
            // Supports both direct strings and s "pattern" calls
            exprs
                .iter()
                .map(|expr| match expr {
                    // Direct string: "bd*4"
                    Expr::String(s) => Ok(s.clone()),
                    // s "bd*4" call - extract the pattern string
                    Expr::Call { name, args } if name == "s" && !args.is_empty() => {
                        match &args[0] {
                            Expr::String(s) => Ok(s.clone()),
                            _ => Err("s() call in slowcat must have a string argument".to_string()),
                        }
                    }
                    _ => Err(
                        "slowcat requires strings or s calls: slowcat [\"bd\", \"sn\"] or slowcat [s \"bd\", s \"sn\"]"
                            .to_string(),
                    ),
                })
                .collect::<Result<Vec<String>, String>>()?
        }
        _ => {
            return Err(
                "slowcat requires a list as argument".to_string(),
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
        envelope_type: None,
        unit_mode: Signal::Value(0.0),    // 0 = rate mode (default)
        loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
        begin: Signal::Value(0.0),        // 0.0 = start of sample
        end: Signal::Value(1.0),          // 1.0 = end of sample
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile wedge combinator - combines two patterns with a ratio
/// Pattern 1 gets ratio portion of each cycle, pattern 2 gets (1-ratio) portion
/// Usage: wedge 0.25 (s "bd*4") (s "hh*8") -> first pattern gets 25%, second gets 75%
/// Also supports: wedge 0.5 "bd*4" "hh*8" for convenience
fn compile_wedge(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() < 3 {
        return Err("wedge requires 3 arguments: ratio pat1 pat2".to_string());
    }

    // First argument is the ratio
    let ratio = match &args[0] {
        Expr::Number(n) => *n,
        _ => return Err("wedge first argument must be a number (ratio between 0 and 1)".to_string()),
    };

    if ratio < 0.0 || ratio > 1.0 {
        return Err("wedge ratio must be between 0 and 1".to_string());
    }

    // Extract pattern strings for both patterns
    let extract_pattern_str = |expr: &Expr| -> Result<String, String> {
        match expr {
            Expr::String(s) => Ok(s.clone()),
            Expr::Call { name, args } if name == "s" && !args.is_empty() => {
                match &args[0] {
                    Expr::String(s) => Ok(s.clone()),
                    _ => Err("s() call in wedge must have a string argument".to_string()),
                }
            }
            _ => Err("wedge patterns must be strings or s calls".to_string()),
        }
    };

    let pat1_str = extract_pattern_str(&args[1])?;
    let pat2_str = extract_pattern_str(&args[2])?;

    // Parse patterns
    let pat1 = parse_mini_notation(&pat1_str);
    let pat2 = parse_mini_notation(&pat2_str);

    // Combine using Pattern::wedge
    let combined_pattern = Pattern::wedge(ratio, pat1, pat2);
    let combined_str = format!("wedge {} \"{}\" \"{}\"", ratio, pat1_str, pat2_str);

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
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile sew combinator - switch between two patterns based on boolean pattern
/// Usage: sew "t f" (s "bd*4") (s "sn*4") - plays bd when true, sn when false
/// Also supports: sew "t f" "bd*4" "sn*4" for convenience
fn compile_sew(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() < 3 {
        return Err("sew requires 3 arguments: bool_pattern pat_true pat_false".to_string());
    }

    // Extract pattern strings for all three patterns
    let extract_pattern_str = |expr: &Expr| -> Result<String, String> {
        match expr {
            Expr::String(s) => Ok(s.clone()),
            Expr::Call { name, args } if name == "s" && !args.is_empty() => {
                match &args[0] {
                    Expr::String(s) => Ok(s.clone()),
                    _ => Err("s() call in sew must have a string argument".to_string()),
                }
            }
            _ => Err("sew patterns must be strings or s calls".to_string()),
        }
    };

    let bool_str = extract_pattern_str(&args[0])?;
    let pat_true_str = extract_pattern_str(&args[1])?;
    let pat_false_str = extract_pattern_str(&args[2])?;

    // Parse patterns
    let bool_pattern = parse_mini_notation(&bool_str);
    let pat_true = parse_mini_notation(&pat_true_str);
    let pat_false = parse_mini_notation(&pat_false_str);

    // Combine using Pattern::sew
    let combined_pattern = Pattern::sew(bool_pattern, pat_true, pat_false);
    let combined_str = format!("sew \"{}\" \"{}\" \"{}\"", bool_str, pat_true_str, pat_false_str);

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
        envelope_type: None,
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile oscillator node
/// Supports both positional and keyword arguments:
///   sine 440           - positional
///   sine :freq 440     - keyword
///   sine 440 +0.5      - with semitone offset
fn compile_oscillator(
    ctx: &mut CompilerContext,
    waveform: Waveform,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    // Use ParamExtractor for keyword argument support
    let extractor = ParamExtractor::new(args);

    // Required parameter: frequency
    let freq_expr = extractor.get_required(0, "freq")?;
    let freq_node = compile_expr(ctx, freq_expr)?;

    // Optional parameter: semitone offset (default 0.0)
    // Parse from second argument if provided
    let offset_expr = extractor.get_optional(1, "offset", 0.0);
    let semitone_offset = match offset_expr {
        Expr::Number(n) => n as f32,
        Expr::String(s) => {
            // Parse strings like "+0.5" or "-2.3"
            s.parse::<f32>().map_err(|_| {
                format!("Invalid semitone offset '{}', expected number like +0.5 or -2.3", s)
            })?
        }
        _ => {
            return Err(format!(
                "Semitone offset must be a number or string, got {:?}",
                offset_expr
            ));
        }
    };

    let node = SignalNode::Oscillator {
        freq: Signal::Node(freq_node),
        waveform,
        semitone_offset,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        carrier_phase: RefCell::new(0.0),
        modulator_phase: RefCell::new(0.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Phase Modulation (PM) oscillator
/// PM uses external modulation signal directly (not internal oscillator like FM)
/// Syntax: pm carrier_freq modulation_signal mod_index
fn compile_pm(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!(
            "pm requires 3 parameters (carrier_freq, modulation, mod_index), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let carrier_node = compile_expr(ctx, args[0].clone())?;
    let modulation_node = compile_expr(ctx, args[1].clone())?;
    let index_node = compile_expr(ctx, args[2].clone())?;

    let node = SignalNode::PMOscillator {
        carrier_freq: Signal::Node(carrier_node),
        modulation: Signal::Node(modulation_node),
        mod_index: Signal::Node(index_node),
        carrier_phase: RefCell::new(0.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Blip oscillator (band-limited impulse train)
/// Syntax: blip frequency
fn compile_blip(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "blip requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    // Compile frequency parameter as signal (supports pattern modulation!)
    let frequency_node = compile_expr(ctx, args[0].clone())?;

    let node = SignalNode::Blip {
        frequency: Signal::Node(frequency_node),
        phase: RefCell::new(0.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile VCO (Voltage-Controlled Oscillator)
/// Syntax: vco frequency waveform [pulse_width]
/// Waveforms: 0=saw, 1=square, 2=triangle, 3=sine
fn compile_vco(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "vco requires 2-3 parameters (frequency, waveform, [pulse_width]), got {}",
            args.len()
        ));
    }

    // Compile parameters as signals (supports pattern modulation!)
    let frequency_node = compile_expr(ctx, args[0].clone())?;
    let waveform_node = compile_expr(ctx, args[1].clone())?;

    // Pulse width is optional, defaults to 0.5 (50% duty cycle)
    let pulse_width_node = if args.len() == 3 {
        compile_expr(ctx, args[2].clone())?
    } else {
        // Default to 0.5
        ctx.graph.add_node(SignalNode::Constant { value: 0.5 })
    };

    let node = SignalNode::VCO {
        frequency: Signal::Node(frequency_node),
        waveform: Signal::Node(waveform_node),
        pulse_width: Signal::Node(pulse_width_node),
        phase: RefCell::new(0.0),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile wavetable oscillator
/// Reads through stored waveform at variable speeds for pitch control
fn compile_wavetable(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "wavetable requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    // Compile frequency parameter as a signal (supports pattern modulation!)
    let freq_node = compile_expr(ctx, args[0].clone())?;

    use crate::unified_graph::WavetableState;

    let node = SignalNode::Wavetable {
        freq: Signal::Node(freq_node),
        state: WavetableState::new(), // Default: sine wave
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile granular synthesizer
/// Breaks audio into small grains and overlaps them with varying parameters
fn compile_granular(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 4 {
        return Err(format!(
            "granular requires 4 parameters (source, grain_size_ms, density, pitch), got {}",
            args.len()
        ));
    }

    // Compile all parameters as signals (supports pattern modulation!)
    let source_node = compile_expr(ctx, args[0].clone())?;
    let grain_size_node = compile_expr(ctx, args[1].clone())?;
    let density_node = compile_expr(ctx, args[2].clone())?;
    let pitch_node = compile_expr(ctx, args[3].clone())?;

    use crate::unified_graph::GranularState;

    let node = SignalNode::Granular {
        source: Signal::Node(source_node),
        grain_size_ms: Signal::Node(grain_size_node),
        density: Signal::Node(density_node),
        pitch: Signal::Node(pitch_node),
        state: GranularState::default(), // 2-second buffer
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Karplus-Strong string synthesis
/// Physical modeling of plucked strings using delay line
fn compile_karplus_strong(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Support both 1 arg (freq only) and 2 args (freq + damping)
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "pluck requires 1-2 parameters (frequency, [damping=0.5]), got {}",
            args.len()
        ));
    }

    // Compile frequency parameter (pattern-modulatable)
    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Compile damping parameter (default 0.5 if not provided)
    let damping_signal = if args.len() == 2 {
        Signal::Node(compile_expr(ctx, args[1].clone())?)
    } else {
        // Default damping: 0.5 (moderate decay)
        Signal::Value(0.5)
    };

    use crate::unified_graph::KarplusStrongState;

    // Calculate initial delay line size based on default frequency
    // We'll resize dynamically if frequency changes
    let initial_size = (ctx.graph.sample_rate() / 440.0) as usize;

    let node = SignalNode::KarplusStrong {
        freq: Signal::Node(freq_node),
        damping: damping_signal,
        trigger: Signal::Value(1.0), // Default: always triggered
        state: KarplusStrongState::new(initial_size),
        last_freq: 440.0, // Will be updated on first sample
        last_trigger: 0.0, // For edge detection
    };

    Ok(ctx.graph.add_node(node))
}

/// Digital waveguide physical modeling (more sophisticated than Karplus-Strong)
/// Uses bidirectional delay lines to simulate wave propagation in strings/tubes
fn compile_waveguide(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Requires 3 parameters: frequency, damping, pickup_position
    if args.len() != 3 {
        return Err(format!(
            "waveguide requires 3 parameters (frequency, damping, pickup_position), got {}",
            args.len()
        ));
    }

    // Compile all parameters (all pattern-modulatable)
    let freq_node = compile_expr(ctx, args[0].clone())?;
    let damping_node = compile_expr(ctx, args[1].clone())?;
    let pickup_node = compile_expr(ctx, args[2].clone())?;

    use crate::unified_graph::WaveguideState;

    // Calculate initial delay line size based on default frequency
    // We'll resize dynamically if frequency changes
    let initial_size = (ctx.graph.sample_rate() / 440.0) as usize;

    let node = SignalNode::Waveguide {
        freq: Signal::Node(freq_node),
        damping: Signal::Node(damping_node),
        pickup_position: Signal::Node(pickup_node),
        state: WaveguideState::new(initial_size),
        last_freq: 440.0, // Will be updated on first sample
    };

    Ok(ctx.graph.add_node(node))
}

/// Formant synthesis - filters source through three resonant bandpass filters
/// Creates vowel sounds by emphasizing specific frequency ranges (formants)
///
/// Parameters: source, f1, f2, f3, bw1, bw2, bw3
/// - source: input signal to filter
/// - f1, f2, f3: formant frequencies (Hz)
/// - bw1, bw2, bw3: formant bandwidths (Hz)
///
/// Common vowel formants (male voice, Hz):
/// - /a/ (father): formant 730 1090 2440 80 90 120
/// - /e/ (bet):    formant 530 1840 2480 80 90 120
/// - /i/ (beet):   formant 270 2290 3010 60 90 150
/// - /o/ (boat):   formant 570 840 2410 80 90 120
/// - /u/ (boot):   formant 300 870 2240 60 70 100
fn compile_formant(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Requires 7 parameters: source, f1, f2, f3, bw1, bw2, bw3
    if args.len() != 7 {
        return Err(format!(
            "formant requires 7 parameters (source, f1, f2, f3, bw1, bw2, bw3), got {}",
            args.len()
        ));
    }

    // Compile all parameters
    let source_node = compile_expr(ctx, args[0].clone())?;
    let f1_node = compile_expr(ctx, args[1].clone())?;
    let f2_node = compile_expr(ctx, args[2].clone())?;
    let f3_node = compile_expr(ctx, args[3].clone())?;
    let bw1_node = compile_expr(ctx, args[4].clone())?;
    let bw2_node = compile_expr(ctx, args[5].clone())?;
    let bw3_node = compile_expr(ctx, args[6].clone())?;

    use crate::unified_graph::FormantState;

    // Create formant state
    let state = FormantState::new(ctx.graph.sample_rate());

    let node = SignalNode::Formant {
        source: Signal::Node(source_node),
        f1: Signal::Node(f1_node),
        f2: Signal::Node(f2_node),
        f3: Signal::Node(f3_node),
        bw1: Signal::Node(bw1_node),
        bw2: Signal::Node(bw2_node),
        bw3: Signal::Node(bw3_node),
        state,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile vowel filter (TidalCycles-style formant filter)
/// vowel "pattern" - accepts patterns of vowel letters: a, e, i, o, u
/// Maps vowel letters to formant filter frequencies
fn compile_vowel(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "vowel requires 1 parameter (vowel pattern), got {}",
            params.len()
        ));
    }

    // Parse vowel pattern - convert vowel letters to numeric selectors
    // For now, compile the expression and expect it to be a pattern of vowel letters
    // The pattern should output values like "a", "e", etc.
    // We'll create a mapping node that converts letters to numbers

    // For simplicity in first implementation: accept a single vowel letter
    // or a pattern that will be interpreted as vowel selector numbers
    let vowel_expr = &params[0];

    // Check if it's a string literal with vowel letters
    let vowel_signal = if let Expr::String(s) = vowel_expr {
        // Map first vowel letter to index
        let vowel_idx = match s.chars().next().unwrap_or('a') {
            'a' | 'A' => 0.0,
            'e' | 'E' => 1.0,
            'i' | 'I' => 2.0,
            'o' | 'O' => 3.0,
            'u' | 'U' => 4.0,
            _ => 0.0, // Default to 'a'
        };

        // Use Signal::Value for constant
        Signal::Value(vowel_idx)
    } else {
        // For numeric patterns, compile as-is
        let node = compile_expr(ctx, vowel_expr.clone())?;
        Signal::Node(node)
    };

    use crate::unified_graph::FormantState;

    let node = SignalNode::Vowel {
        source: input_signal,
        vowel: vowel_signal,
        state: FormantState::new(ctx.graph.sample_rate()),
    };

    Ok(ctx.graph.add_node(node))
}

/// Additive synthesis - creates complex timbres by summing sine wave partials
/// Each partial is a harmonic (integer multiple of fundamental) with independent amplitude
///
/// Parameters: freq, amplitudes
/// - freq: fundamental frequency (Hz) - pattern-modulatable
/// - amplitudes: space-separated amplitude values for each partial (e.g., "1.0 0.5 0.25")
///   Partial 1 = fundamental, Partial 2 = 2×fundamental, etc.
///
/// Example: additive 440 "1.0 0.5 0.25" creates 440Hz + 880Hz(×0.5) + 1320Hz(×0.25)
fn compile_additive(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Requires 2 parameters: freq, amplitudes
    if args.len() != 2 {
        return Err(format!(
            "additive requires 2 parameters (freq, amplitudes), got {}",
            args.len()
        ));
    }

    // Compile frequency parameter (pattern-modulatable)
    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Parse amplitudes - extract numeric values from pattern string
    let amplitudes: Vec<f32> = match &args[1] {
        Expr::String(s) => {
            // Parse mini-notation string to extract numbers
            s.split_whitespace()
                .filter_map(|token| token.parse::<f32>().ok())
                .collect()
        }
        Expr::Number(n) => {
            // Single amplitude value
            vec![*n as f32]
        }
        _ => {
            return Err(
                "additive amplitudes must be a string (e.g., \"1.0 0.5 0.25\") or number"
                    .to_string(),
            );
        }
    };

    if amplitudes.is_empty() {
        return Err("additive requires at least one amplitude value".to_string());
    }

    use crate::unified_graph::AdditiveState;

    // Create additive state
    let state = AdditiveState::new(ctx.graph.sample_rate());

    let node = SignalNode::Additive {
        freq: Signal::Node(freq_node),
        amplitudes,
        state,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile vocoder
/// Syntax: vocoder modulator carrier num_bands
/// Example: vocoder ~voice ~synth 8
fn compile_vocoder(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Requires 3 parameters: modulator, carrier, num_bands
    if args.len() != 3 {
        return Err(format!(
            "vocoder requires 3 parameters (modulator, carrier, num_bands), got {}",
            args.len()
        ));
    }

    // Compile modulator signal (usually voice or rhythmic source)
    let modulator_node = compile_expr(ctx, args[0].clone())?;

    // Compile carrier signal (usually synth with rich harmonics)
    let carrier_node = compile_expr(ctx, args[1].clone())?;

    // Parse num_bands parameter
    let num_bands = match &args[2] {
        Expr::Number(n) => {
            let bands = *n as usize;
            if bands < 2 || bands > 32 {
                return Err("vocoder num_bands must be between 2 and 32".to_string());
            }
            bands
        }
        _ => {
            return Err("vocoder num_bands must be a number (e.g., 8, 16)".to_string());
        }
    };

    use crate::unified_graph::VocoderState;

    // Create vocoder state with specified number of bands
    let state = VocoderState::new(num_bands, ctx.graph.sample_rate());

    let node = SignalNode::Vocoder {
        modulator: Signal::Node(modulator_node),
        carrier: Signal::Node(carrier_node),
        num_bands,
        state,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile pitch shifter
fn compile_pitch_shift(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Requires 2 parameters: input signal, semitones
    if args.len() != 2 {
        return Err(format!(
            "pitch_shift requires 2 parameters (input, semitones), got {}",
            args.len()
        ));
    }

    // Compile input signal
    let input_node = compile_expr(ctx, args[0].clone())?;

    // Compile semitones parameter (can be pattern-modulated)
    let semitones_node = compile_expr(ctx, args[1].clone())?;

    use crate::unified_graph::PitchShifterState;

    // Create pitch shifter state with default grain size (50ms)
    let state = PitchShifterState::new(50.0, ctx.graph.sample_rate());

    let node = SignalNode::PitchShift {
        input: Signal::Node(input_node),
        semitones: Signal::Node(semitones_node),
        state,
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

/// Compile impulse generator (periodic single-sample spikes)
fn compile_impulse(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::ImpulseState;

    if args.len() != 1 {
        return Err(format!(
            "impulse requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;
    let node = SignalNode::Impulse {
        frequency: Signal::Node(freq_node),
        state: ImpulseState::default(),
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile lag (exponential slew limiter / portamento)
fn compile_lag(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::LagState;

    if args.len() != 2 {
        return Err(format!(
            "lag requires 2 parameters (input, lag_time), got {}",
            args.len()
        ));
    }

    let input_node = compile_expr(ctx, args[0].clone())?;
    let lag_time_node = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::Lag {
        input: Signal::Node(input_node),
        lag_time: Signal::Node(lag_time_node),
        state: LagState::default(),
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile xline (exponential envelope)
fn compile_xline(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::XLineState;

    if args.len() != 3 {
        return Err(format!(
            "xline requires 3 parameters (start, end, duration), got {}",
            args.len()
        ));
    }

    let start_node = compile_expr(ctx, args[0].clone())?;
    let end_node = compile_expr(ctx, args[1].clone())?;
    let duration_node = compile_expr(ctx, args[2].clone())?;

    let node = SignalNode::XLine {
        start: Signal::Node(start_node),
        end: Signal::Node(end_node),
        duration: Signal::Node(duration_node),
        state: XLineState::default(),
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile ASR (Attack-Sustain-Release) envelope
fn compile_asr(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::ASRState;

    // Use ParamExtractor for keyword argument support
    let extractor = ParamExtractor::new(args);

    // All three parameters are required
    let gate_expr = extractor.get_required(0, "gate")?;
    let gate_node = compile_expr(ctx, gate_expr)?;

    let attack_expr = extractor.get_required(1, "attack")?;
    let attack_node = compile_expr(ctx, attack_expr)?;

    let release_expr = extractor.get_required(2, "release")?;
    let release_node = compile_expr(ctx, release_expr)?;

    let node = SignalNode::ASR {
        gate: Signal::Node(gate_node),
        attack: Signal::Node(attack_node),
        release: Signal::Node(release_node),
        state: ASRState::default(),
    };
    Ok(ctx.graph.add_node(node))
}

/// Compile pulse oscillator (variable pulse width)
fn compile_pulse(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "pulse requires 2 parameters (frequency, pulse_width), got {}",
            args.len()
        ));
    }

    // Compile frequency and pulse_width as signals (supports pattern modulation!)
    let freq_node = compile_expr(ctx, args[0].clone())?;
    let width_node = compile_expr(ctx, args[1].clone())?;

    // Create fundsp pulse unit (bandlimited PWM oscillator)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_pulse(ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Pulse,
        inputs: vec![Signal::Node(freq_node), Signal::Node(width_node)],
        state: Arc::new(Mutex::new(state)),
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

/// Compile FM cross-modulation effect
/// Formula: carrier * cos(2π * mod_depth * modulator)
fn compile_fm_crossmod(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!(
            "fmcrossmod requires 3 parameters (carrier, modulator, mod_depth), got {}",
            args.len()
        ));
    }

    // Compile carrier, modulator, and mod_depth
    let carrier_node = compile_expr(ctx, args[0].clone())?;
    let modulator_node = compile_expr(ctx, args[1].clone())?;
    let mod_depth_node = compile_expr(ctx, args[2].clone())?;

    // Create FMCrossMod node
    let node = SignalNode::FMCrossMod {
        carrier: Signal::Node(carrier_node),
        modulator: Signal::Node(modulator_node),
        mod_depth: Signal::Node(mod_depth_node),
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

    let release = if args.len() >= 3 {
        Signal::Node(compile_expr(ctx, args[2].clone())?)
    } else {
        Signal::Value(0.01)
    };

    let node = SignalNode::Limiter {
        input: Signal::Node(input_node),
        threshold: Signal::Node(threshold_node),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_pan2_l(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "pan2_l requires 2 parameters (input, position), got {}",
            args.len()
        ));
    }

    // Compile input signal and pan position
    let input_node = compile_expr(ctx, args[0].clone())?;
    let position_node = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::Pan2Left {
        input: Signal::Node(input_node),
        position: Signal::Node(position_node),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_pan2_r(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "pan2_r requires 2 parameters (input, position), got {}",
            args.len()
        ));
    }

    // Compile input signal and pan position
    let input_node = compile_expr(ctx, args[0].clone())?;
    let position_node = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::Pan2Right {
        input: Signal::Node(input_node),
        position: Signal::Node(position_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile fundsp organ_hz oscillator
/// Organ synthesis with additive harmonics (from fundsp library)
/// Usage: organ_hz frequency
fn compile_organ_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.is_empty() {
        return Err("organ_hz requires frequency argument".to_string());
    }

    // Compile frequency parameter as a signal (supports pattern modulation!)
    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Create fundsp organ_hz unit
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_organ_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::OrganHz,
        inputs: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_moog_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "moog_hz requires 2 parameters (cutoff, resonance), got {}",
            params.len()
        ));
    }

    // Compile cutoff and resonance parameters (supports pattern modulation!)
    let cutoff_node = compile_expr(ctx, params[0].clone())?;
    let resonance_node = compile_expr(ctx, params[1].clone())?;

    // Create fundsp moog_hz unit (initialized with default params)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_moog_hz(1000.0, 0.5, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::MoogHz,
        inputs: vec![
            input_signal,
            Signal::Node(cutoff_node),
            Signal::Node(resonance_node),
        ],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_reverb_stereo(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "reverb_stereo requires 2 parameters (wet, time), got {}",
            params.len()
        ));
    }

    // Compile wet and time parameters (supports pattern modulation!)
    let wet_node = compile_expr(ctx, params[0].clone())?;
    let time_node = compile_expr(ctx, params[1].clone())?;

    // Create fundsp reverb_stereo unit (initialized with default params)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_reverb_stereo(0.5, 1.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::ReverbStereo,
        inputs: vec![
            input_signal,
            Signal::Node(wet_node),
            Signal::Node(time_node),
        ],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_fundsp_chorus(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 3 {
        return Err(format!(
            "fchorus requires 3 parameters (separation, variation, mod_frequency), got {}",
            params.len()
        ));
    }

    // Compile parameters (supports pattern modulation!)
    let separation_node = compile_expr(ctx, params[0].clone())?;
    let variation_node = compile_expr(ctx, params[1].clone())?;
    let mod_freq_node = compile_expr(ctx, params[2].clone())?;

    // Create fundsp chorus unit (initialized with default params)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_chorus(0, 0.015, 0.005, 0.3, ctx.graph.sample_rate() as f64);

    // Create constant node for fixed seed
    let seed_node = ctx.graph.add_node(SignalNode::Constant { value: 0.0 });

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Chorus,
        inputs: vec![
            input_signal,
            Signal::Node(seed_node), // Fixed seed=0
            Signal::Node(separation_node),
            Signal::Node(variation_node),
            Signal::Node(mod_freq_node),
        ],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_saw_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "saw_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Create fundsp saw_hz unit (initialized with default frequency)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_saw_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SawHz,
        inputs: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_soft_saw_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "soft_saw_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Create fundsp soft_saw_hz unit (initialized with default frequency)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_soft_saw_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SoftSawHz,
        inputs: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_square_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "square_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Create fundsp square_hz unit (initialized with default frequency)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_square_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::SquareHz,
        inputs: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_triangle_hz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!(
            "triangle_hz requires 1 parameter (frequency), got {}",
            args.len()
        ));
    }

    let freq_node = compile_expr(ctx, args[0].clone())?;

    // Create fundsp triangle_hz unit (initialized with default frequency)
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_triangle_hz(440.0, ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::TriangleHz,
        inputs: vec![Signal::Node(freq_node)],
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_noise(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("noise takes no parameters, got {}", args.len()));
    }

    // Create fundsp noise unit
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_noise(ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Noise,
        inputs: vec![], // No inputs!
        state: Arc::new(Mutex::new(state)),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_pink(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("pink takes no parameters, got {}", args.len()));
    }

    // Create fundsp pink noise unit
    use crate::unified_graph::{FundspState, FundspUnitType};
    use std::sync::{Arc, Mutex};

    let state = FundspState::new_pink(ctx.graph.sample_rate() as f64);

    let node = SignalNode::FundspUnit {
        unit_type: FundspUnitType::Pink,
        inputs: vec![], // No inputs!
        state: Arc::new(Mutex::new(state)),
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
        return Err(format!(
            "{:?}_trig requires pattern string argument",
            waveform
        ));
    }

    // First argument should be a pattern string
    let pattern_str = match &args[0] {
        Expr::String(s) => s.clone(),
        _ => {
            return Err(format!(
                "{:?}_trig requires a pattern string as first argument",
                waveform
            ))
        }
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

    // Use ParamExtractor for optional q parameter
    let extractor = ParamExtractor::new(params);

    // cutoff is required (positional index 0, or :cutoff)
    let cutoff_expr = extractor.get_required(0, "cutoff")?;
    let cutoff_node = compile_expr(ctx, cutoff_expr)?;

    // q is optional (positional index 1, or :q, defaults to 1.0)
    let q_expr = extractor.get_optional(1, "q", 1.0);
    let q_node = compile_expr(ctx, q_expr)?;

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
        "notch" => SignalNode::Notch {
            input: input_signal,
            center: Signal::Node(cutoff_node),
            q: Signal::Node(q_node),
            state: FilterState::default(),
        },
        _ => return Err(format!("Unknown filter type: {}", filter_type)),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Comb filter (feedback delay line)
/// Syntax: comb input frequency feedback
/// Example: ~impulse # comb 440 0.95
fn compile_comb(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Comb expects 2 params after input: frequency, feedback
    if params.len() != 2 {
        return Err(format!(
            "comb requires 2 parameters (frequency, feedback), got {}",
            params.len()
        ));
    }

    let frequency_node = compile_expr(ctx, params[0].clone())?;
    let feedback_node = compile_expr(ctx, params[1].clone())?;

    // Create delay buffer (1 second max delay at sample rate)
    let buffer_size = ctx.sample_rate as usize;
    let buffer = vec![0.0; buffer_size];

    let node = SignalNode::Comb {
        input: input_signal,
        frequency: Signal::Node(frequency_node),
        feedback: Signal::Node(feedback_node),
        buffer,
        write_pos: 0,
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

    // Use ParamExtractor for optional mix parameter
    let extractor = ParamExtractor::new(params);

    // room_size and damping are required
    let room_expr = extractor.get_required(0, "room_size")?;
    let room_node = compile_expr(ctx, room_expr)?;

    let damp_expr = extractor.get_required(1, "damping")?;
    let damp_node = compile_expr(ctx, damp_expr)?;

    // mix is optional (defaults to 0.3 = 30% wet)
    let mix_expr = extractor.get_optional(2, "mix", 0.3);
    let mix_node = compile_expr(ctx, mix_expr)?;

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

/// Compile convolution reverb
fn compile_convolve(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if !params.is_empty() {
        return Err(format!(
            "convolve requires no additional parameters (uses built-in IR), got {}",
            params.len()
        ));
    }

    use crate::unified_graph::ConvolutionState;

    let node = SignalNode::Convolution {
        input: input_signal,
        state: ConvolutionState::new(ctx.graph.sample_rate()),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile spectral freeze
fn compile_freeze(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "freeze requires 1 parameter (trigger), got {}",
            params.len()
        ));
    }

    // Compile trigger parameter
    let trigger_node = compile_expr(ctx, params[0].clone())?;

    use crate::unified_graph::SpectralFreezeState;

    let node = SignalNode::SpectralFreeze {
        input: input_signal,
        trigger: Signal::Node(trigger_node),
        state: SpectralFreezeState::new(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile distortion effect
fn compile_distortion(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Use ParamExtractor for optional mix parameter
    let extractor = ParamExtractor::new(params);

    // drive is required
    let drive_expr = extractor.get_required(0, "drive")?;
    let drive_node = compile_expr(ctx, drive_expr)?;

    // mix is optional (defaults to 0.5 = 50% wet/dry)
    let mix_expr = extractor.get_optional(1, "mix", 0.5);
    let mix_node = compile_expr(ctx, mix_expr)?;

    let node = SignalNode::Distortion {
        input: input_signal,
        drive: Signal::Node(drive_node),
        mix: Signal::Node(mix_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile range function - maps signal from -1..1 to min..max
/// Usage: range min max signal
/// Formula: output = min + (signal + 1) * 0.5 * (max - min)
fn compile_range(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!("range requires 3 arguments (min, max, signal), got {}", args.len()));
    }

    // Compile min, max, and signal
    let min_node = compile_expr(ctx, args[0].clone())?;
    let max_node = compile_expr(ctx, args[1].clone())?;
    let signal_node = compile_expr(ctx, args[2].clone())?;

    // Create the range scaling expression:
    // output = min + (signal + 1) * 0.5 * (max - min)

    // signal + 1
    let signal_plus_1 = ctx.graph.add_node(SignalNode::Add {
        a: Signal::Node(signal_node),
        b: Signal::Value(1.0),
    });

    // (signal + 1) * 0.5
    let normalized = ctx.graph.add_node(SignalNode::Multiply {
        a: Signal::Node(signal_plus_1),
        b: Signal::Value(0.5),
    });

    // max - min  (implemented as max + (-1 * min))
    let neg_min = ctx.graph.add_node(SignalNode::Multiply {
        a: Signal::Node(min_node),
        b: Signal::Value(-1.0),
    });
    let range_width = ctx.graph.add_node(SignalNode::Add {
        a: Signal::Node(max_node),
        b: Signal::Node(neg_min),
    });

    // normalized * (max - min)
    let scaled = ctx.graph.add_node(SignalNode::Multiply {
        a: Signal::Node(normalized),
        b: Signal::Node(range_width),
    });

    // min + scaled
    let output = ctx.graph.add_node(SignalNode::Add {
        a: Signal::Node(min_node),
        b: Signal::Node(scaled),
    });

    Ok(output)
}

/// Compile gain function - multiplies signal by gain amount
/// Usage: signal # gain amount
/// Example: s "bd" # gain 0.5  (reduce volume by half)
/// Example: s "bd" # gain "0.5 1.0 0.8"  (pattern-controlled gain)
fn compile_gain(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input and gain parameter
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.is_empty() {
        return Err("gain requires a gain amount".to_string());
    }

    // Compile the gain amount (can be a number, pattern, or LFO)
    let gain_node = compile_expr(ctx, params[0].clone())?;

    // Create multiply node: output = input * gain
    let output = ctx.graph.add_node(SignalNode::Multiply {
        a: input_signal,
        b: Signal::Node(gain_node),
    });

    Ok(output)
}

/// Compile pan function - stereo panning
/// Usage: signal # pan position
/// Position: -1 = hard left, 0 = center, 1 = hard right
/// Example: s "bd" # pan "-1"  (hard left)
/// Example: s "bd" # pan "0.5 -0.5"  (pattern-controlled panning)
/// Note: Returns left channel only. Use pan2_l and pan2_r for stereo output.
fn compile_pan(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input and pan parameter
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.is_empty() {
        return Err("pan requires a pan position (-1 to 1)".to_string());
    }

    // Compile the pan position
    let pan_node = compile_expr(ctx, params[0].clone())?;

    // Create pan2 left channel (for mono output, just return left)
    // For full stereo, users should use pan2_l and pan2_r separately
    let output = ctx.graph.add_node(SignalNode::Pan2Left {
        input: input_signal,
        position: Signal::Node(pan_node),
    });

    Ok(output)
}

/// Compile min function - minimum of two signals
/// Usage: min signal_a signal_b
/// Example: min (sine 0.5) 0.0  (rectify sine wave)
/// Example: min ~lfo ~env  (modulate with minimum of two signals)
fn compile_min(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!("min requires exactly 2 arguments, got {}", args.len()));
    }

    // Compile both input signals
    let a_node = compile_expr(ctx, args[0].clone())?;
    let b_node = compile_expr(ctx, args[1].clone())?;

    // Create Min node
    let output = ctx.graph.add_node(SignalNode::Min {
        a: Signal::Node(a_node),
        b: Signal::Node(b_node),
    });

    Ok(output)
}

/// Compile wrap function - wrap signal into range using modulo
/// Usage: wrap input min max
/// Example: wrap (sine 5.0) 0.0 1.0  (wrap sine between 0 and 1)
/// Example: wrap ~lfo -1.0 1.0  (wrap LFO into bipolar range)
fn compile_wrap(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!("wrap requires exactly 3 arguments (input, min, max), got {}", args.len()));
    }

    // Compile all three input signals
    let input_node = compile_expr(ctx, args[0].clone())?;
    let min_node = compile_expr(ctx, args[1].clone())?;
    let max_node = compile_expr(ctx, args[2].clone())?;

    // Create Wrap node
    let output = ctx.graph.add_node(SignalNode::Wrap {
        input: Signal::Node(input_node),
        min: Signal::Node(min_node),
        max: Signal::Node(max_node),
    });

    Ok(output)
}

/// Compile sample-and-hold node
/// Usage: sample_hold(input, trigger)
/// Captures input when trigger crosses from negative/zero to positive
fn compile_sample_hold(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!("sample_hold requires exactly 2 arguments (input, trigger), got {}", args.len()));
    }

    // Compile input and trigger signals
    let input_node = compile_expr(ctx, args[0].clone())?;
    let trigger_node = compile_expr(ctx, args[1].clone())?;

    // Create SampleAndHold node
    let output = ctx.graph.add_node(SignalNode::SampleAndHold {
        input: Signal::Node(input_node),
        trigger: Signal::Node(trigger_node),
        held_value: std::cell::RefCell::new(0.0),
        last_trigger: std::cell::RefCell::new(0.0),
    });

    Ok(output)
}

/// Compile decimator effect (sample rate reduction for lo-fi/retro effects)
/// Usage: signal # decimator(factor, smooth)
/// - factor: Decimation factor (1.0 = no effect, 2.0 = half rate, etc.)
/// - smooth: Smoothing amount (0.0 = harsh/stepped, 1.0 = smooth)
fn compile_decimator(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!("decimator requires exactly 3 arguments (input, factor, smooth), got {}", args.len()));
    }

    // Compile input, factor, and smooth signals
    let input_node = compile_expr(ctx, args[0].clone())?;
    let factor_node = compile_expr(ctx, args[1].clone())?;
    let smooth_node = compile_expr(ctx, args[2].clone())?;

    // Create Decimator node
    let output = ctx.graph.add_node(SignalNode::Decimator {
        input: Signal::Node(input_node),
        factor: Signal::Node(factor_node),
        smooth: Signal::Node(smooth_node),
        sample_counter: std::cell::RefCell::new(0.0),
        held_value: std::cell::RefCell::new(0.0),
        smooth_state: std::cell::RefCell::new(0.0),
    });

    Ok(output)
}

/// Compile pattern-triggered envelope (rhythmic gate)
/// Usage: signal # env_trig("pattern", attack, decay, sustain, release)
fn compile_envelope_pattern(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
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

    // Use ParamExtractor for optional feedback and mix parameters
    let extractor = ParamExtractor::new(params);

    // time is required (delay time in seconds)
    let time_expr = extractor.get_required(0, "time")?;
    let time_node = compile_expr(ctx, time_expr)?;

    // feedback is optional (defaults to 0.5 = moderate repeats)
    let feedback_expr = extractor.get_optional(1, "feedback", 0.5);
    let feedback_node = compile_expr(ctx, feedback_expr)?;

    // mix is optional (defaults to 0.5 = 50% wet/dry)
    let mix_expr = extractor.get_optional(2, "mix", 0.5);
    let mix_node = compile_expr(ctx, mix_expr)?;

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

/// Compile tape delay effect (vintage tape simulation)
fn compile_tapedelay(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() < 2 {
        return Err(format!(
            "tapedelay requires at least 2 parameters (time, feedback), got {}",
            params.len()
        ));
    }

    // Required parameters
    let time_node = compile_expr(ctx, params[0].clone())?;
    let feedback_node = compile_expr(ctx, params[1].clone())?;

    // Optional parameters with defaults
    let wow_rate_node = if params.len() > 2 {
        compile_expr(ctx, params[2].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.5 })  // Default: subtle wobble
    };

    let wow_depth_node = if params.len() > 3 {
        compile_expr(ctx, params[3].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.02 })
    };

    let flutter_rate_node = if params.len() > 4 {
        compile_expr(ctx, params[4].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 6.0 })  // Default: 6 Hz shimmer
    };

    let flutter_depth_node = if params.len() > 5 {
        compile_expr(ctx, params[5].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.05 })
    };

    let saturation_node = if params.len() > 6 {
        compile_expr(ctx, params[6].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.3 })  // Default: mild warmth
    };

    let mix_node = if params.len() > 7 {
        compile_expr(ctx, params[7].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.5 })  // Default: 50/50 mix
    };

    let node = SignalNode::TapeDelay {
        input: input_signal,
        time: Signal::Node(time_node),
        feedback: Signal::Node(feedback_node),
        wow_rate: Signal::Node(wow_rate_node),
        wow_depth: Signal::Node(wow_depth_node),
        flutter_rate: Signal::Node(flutter_rate_node),
        flutter_depth: Signal::Node(flutter_depth_node),
        saturation: Signal::Node(saturation_node),
        mix: Signal::Node(mix_node),
        state: TapeDelayState::new(ctx.sample_rate),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile multi-tap delay (rhythmic multiple echoes)
fn compile_multitap(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() < 2 {
        return Err(format!(
            "multitap requires at least 2 parameters (time, taps), got {}",
            params.len()
        ));
    }

    // Required parameters
    let time_node = compile_expr(ctx, params[0].clone())?;

    // Extract taps count (must be a constant)
    let taps = if let Expr::Number(n) = params[1].clone() {
        n as usize
    } else {
        return Err("multitap 'taps' parameter must be a constant number".to_string());
    };

    // Optional parameters with defaults
    let feedback_node = if params.len() > 2 {
        compile_expr(ctx, params[2].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.5 })  // Default: moderate feedback
    };

    let mix_node = if params.len() > 3 {
        compile_expr(ctx, params[3].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.6 })  // Default: 60% wet
    };

    // Create delay buffer (1 second max)
    let buffer_size = ctx.sample_rate as usize;

    let node = SignalNode::MultiTapDelay {
        input: input_signal,
        time: Signal::Node(time_node),
        taps,
        feedback: Signal::Node(feedback_node),
        mix: Signal::Node(mix_node),
        buffer: vec![0.0; buffer_size],
        write_idx: 0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile ping-pong delay (stereo bouncing)
fn compile_pingpong(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() < 2 {
        return Err(format!(
            "pingpong requires at least 2 parameters (time, feedback), got {}",
            params.len()
        ));
    }

    // Required parameters
    let time_node = compile_expr(ctx, params[0].clone())?;
    let feedback_node = compile_expr(ctx, params[1].clone())?;

    // Optional parameters with defaults
    let stereo_width_node = if params.len() > 2 {
        compile_expr(ctx, params[2].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.8 })  // Default: strong ping-pong
    };

    let channel = if params.len() > 3 {
        if let Expr::Number(n) = params[3].clone() {
            n != 0.0
        } else {
            return Err("pingpong 'channel' parameter must be a constant (0=left, 1=right)".to_string());
        }
    } else {
        false  // Default: start on left
    };

    let mix_node = if params.len() > 4 {
        compile_expr(ctx, params[4].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.7 })  // Default: 70% wet
    };

    // Create delay buffers (1 second max each)
    let buffer_size = ctx.sample_rate as usize;

    let node = SignalNode::PingPongDelay {
        input: input_signal,
        time: Signal::Node(time_node),
        feedback: Signal::Node(feedback_node),
        stereo_width: Signal::Node(stereo_width_node),
        channel,
        mix: Signal::Node(mix_node),
        buffer_l: vec![0.0; buffer_size],
        buffer_r: vec![0.0; buffer_size],
        write_idx: 0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Dattorro plate reverb
fn compile_plate(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() < 2 {
        return Err(format!(
            "plate requires at least 2 parameters (pre_delay, decay), got {}",
            params.len()
        ));
    }

    // Required parameters
    let pre_delay_node = compile_expr(ctx, params[0].clone())?;
    let decay_node = compile_expr(ctx, params[1].clone())?;

    // Optional parameters with defaults
    let diffusion_node = if params.len() > 2 {
        compile_expr(ctx, params[2].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.7 })  // Default: dense diffusion
    };

    let damping_node = if params.len() > 3 {
        compile_expr(ctx, params[3].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.3 })  // Default: some HF rolloff
    };

    let mod_depth_node = if params.len() > 4 {
        compile_expr(ctx, params[4].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.3 })  // Default: subtle shimmer
    };

    let mix_node = if params.len() > 5 {
        compile_expr(ctx, params[5].clone())?
    } else {
        ctx.graph.add_node(SignalNode::Constant { value: 0.5 })  // Default: 50/50 mix
    };

    let node = SignalNode::DattorroReverb {
        input: input_signal,
        pre_delay: Signal::Node(pre_delay_node),
        decay: Signal::Node(decay_node),
        diffusion: Signal::Node(diffusion_node),
        damping: Signal::Node(damping_node),
        mod_depth: Signal::Node(mod_depth_node),
        mix: Signal::Node(mix_node),
        state: DattorroState::new(ctx.sample_rate),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile chorus effect
fn compile_chorus(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Use ParamExtractor for optional mix parameter
    let extractor = ParamExtractor::new(params);

    // rate and depth are required
    let rate_expr = extractor.get_required(0, "rate")?;
    let rate_node = compile_expr(ctx, rate_expr)?;

    let depth_expr = extractor.get_required(1, "depth")?;
    let depth_node = compile_expr(ctx, depth_expr)?;

    // mix is optional (defaults to 0.3 = 30% wet)
    let mix_expr = extractor.get_optional(2, "mix", 0.3);
    let mix_node = compile_expr(ctx, mix_expr)?;

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
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Flanger expects 3 params after input: depth, rate, feedback
    if params.len() != 3 {
        return Err(format!(
            "flanger requires 3 parameters (depth, rate, feedback), got {}",
            params.len()
        ));
    }

    let depth_node = compile_expr(ctx, params[0].clone())?;
    let rate_node = compile_expr(ctx, params[1].clone())?;
    let feedback_node = compile_expr(ctx, params[2].clone())?;

    use crate::unified_graph::FlangerState;

    let node = SignalNode::Flanger {
        input: input_signal,
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

/// Compile sidechain compressor effect
fn compile_sidechain_compressor(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract main input (handles both standalone and chained forms)
    let (main_input, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 5 {
        return Err(format!(
            "sidechain_compressor requires 5 parameters (sidechain_input, threshold, ratio, attack, release), got {}",
            params.len()
        ));
    }

    let sidechain_node = compile_expr(ctx, params[0].clone())?;
    let threshold_node = compile_expr(ctx, params[1].clone())?;
    let ratio_node = compile_expr(ctx, params[2].clone())?;
    let attack_node = compile_expr(ctx, params[3].clone())?;
    let release_node = compile_expr(ctx, params[4].clone())?;

    use crate::unified_graph::CompressorState;

    let node = SignalNode::SidechainCompressor {
        main_input: main_input,
        sidechain_input: Signal::Node(sidechain_node),
        threshold: Signal::Node(threshold_node),
        ratio: Signal::Node(ratio_node),
        attack: Signal::Node(attack_node),
        release: Signal::Node(release_node),
        state: CompressorState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile expander effect (upward expansion - boosts signals above threshold)
fn compile_expander(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 4 {
        return Err(format!(
            "expander requires 4 parameters (threshold, ratio, attack, release), got {}",
            params.len()
        ));
    }

    let threshold_node = compile_expr(ctx, params[0].clone())?;
    let ratio_node = compile_expr(ctx, params[1].clone())?;
    let attack_node = compile_expr(ctx, params[2].clone())?;
    let release_node = compile_expr(ctx, params[3].clone())?;

    use crate::unified_graph::ExpanderState;

    let node = SignalNode::Expander {
        input: input_signal,
        threshold: Signal::Node(threshold_node),
        ratio: Signal::Node(ratio_node),
        attack: Signal::Node(attack_node),
        release: Signal::Node(release_node),
        state: ExpanderState::default(),
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

/// Compile coarse effect (sample rate reduction)
/// coarse n - reduces sample rate to 1/n (TidalCycles equivalent)
/// Implemented as bitcrush with bits=16 (no bit reduction, just sample rate reduction)
fn compile_coarse(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "coarse requires 1 parameter (sample_rate factor), got {}",
            params.len()
        ));
    }

    let sr_node = compile_expr(ctx, params[0].clone())?;

    // Use bitcrush with full bit depth (16 bits = no bit reduction)
    // Only apply sample rate reduction
    use crate::unified_graph::BitCrushState;

    let node = SignalNode::BitCrush {
        input: input_signal,
        bits: Signal::Value(16.0), // Full bit depth - no bit reduction
        sample_rate: Signal::Node(sr_node),
        state: BitCrushState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile djf (DJ filter) effect
/// djf value - DJ filter sweep: 0-0.5 = lowpass, 0.5-1 = highpass
/// Maps 0-1 parameter to filter type and cutoff frequency
fn compile_djf(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "djf requires 1 parameter (filter value 0-1), got {}",
            params.len()
        ));
    }

    let value_node = compile_expr(ctx, params[0].clone())?;

    // Create a DJFilter node that internally handles the low/high pass transition
    // The value parameter (0-1) controls the filter sweep:
    // 0.0-0.5: lowpass (cutoff increases with value)
    // 0.5-1.0: highpass (cutoff increases with value)
    use crate::unified_graph::FilterState;

    let node = SignalNode::DJFilter {
        input: input_signal,
        value: Signal::Node(value_node),
        state: FilterState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile ring modulation effect
/// ring freq - multiplies input by carrier frequency
/// Example: saw 220 # ring 440
fn compile_ring(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "ring requires 1 parameter (carrier frequency), got {}",
            params.len()
        ));
    }

    let freq_node = compile_expr(ctx, params[0].clone())?;

    let node = SignalNode::RingMod {
        input: input_signal,
        freq: Signal::Node(freq_node),
        phase: 0.0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile tremolo effect (amplitude modulation)
/// Syntax: tremolo rate depth
/// Example: ~signal # tremolo 5.0 0.7
fn compile_tremolo(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "tremolo requires 2 parameters (rate, depth), got {}",
            params.len()
        ));
    }

    let rate_node = compile_expr(ctx, params[0].clone())?;
    let depth_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::Tremolo {
        input: input_signal,
        rate: Signal::Node(rate_node),
        depth: Signal::Node(depth_node),
        phase: 0.0, // Start at phase 0
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile vibrato effect (pitch modulation)
/// Syntax: vibrato rate depth
/// Example: ~signal # vibrato 5.5 0.4
fn compile_vibrato(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "vibrato requires 2 parameters (rate, depth), got {}",
            params.len()
        ));
    }

    let rate_node = compile_expr(ctx, params[0].clone())?;
    let depth_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::Vibrato {
        input: input_signal,
        rate: Signal::Node(rate_node),
        depth: Signal::Node(depth_node),
        phase: 0.0,              // Start at phase 0
        delay_buffer: Vec::new(), // Initialized on first use
        buffer_pos: 0,           // Start at buffer position 0
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile phaser effect (spectral sweeping via allpass filter cascade)
/// Syntax: phaser rate depth feedback stages
/// Example: ~signal # phaser 0.5 0.7 0.4 6
fn compile_phaser(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 4 {
        return Err(format!(
            "phaser requires 4 parameters (rate, depth, feedback, stages), got {}",
            params.len()
        ));
    }

    let rate_node = compile_expr(ctx, params[0].clone())?;
    let depth_node = compile_expr(ctx, params[1].clone())?;
    let feedback_node = compile_expr(ctx, params[2].clone())?;

    // stages parameter must be a constant integer
    let stages = match &params[3] {
        Expr::Number(n) => {
            let val = *n as usize;
            if val < 2 || val > 12 {
                return Err(format!(
                    "phaser stages must be between 2 and 12, got {}",
                    val
                ));
            }
            val
        }
        _ => {
            return Err(
                "phaser stages parameter must be a constant number (2 to 12)".to_string()
            )
        }
    };

    let node = SignalNode::Phaser {
        input: input_signal,
        rate: Signal::Node(rate_node),
        depth: Signal::Node(depth_node),
        feedback: Signal::Node(feedback_node),
        stages,
        phase: 0.0,
        allpass_z1: Vec::new(), // Initialized on first use
        allpass_y1: Vec::new(), // Initialized on first use
        feedback_sample: 0.0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile XFade (crossfader between two signals)
/// Syntax: xfade signal_a signal_b position
/// position: 0.0 = 100% signal_a, 1.0 = 100% signal_b
fn compile_xfade(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!(
            "xfade requires 3 parameters (signal_a, signal_b, position), got {}",
            args.len()
        ));
    }

    let signal_a_node = compile_expr(ctx, args[0].clone())?;
    let signal_b_node = compile_expr(ctx, args[1].clone())?;
    let position_node = compile_expr(ctx, args[2].clone())?;

    let node = SignalNode::XFade {
        signal_a: Signal::Node(signal_a_node),
        signal_b: Signal::Node(signal_b_node),
        position: Signal::Node(position_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Mix (sum multiple signals)
/// Syntax: mix signal1 signal2 signal3 ...
/// Sums all input signals together
fn compile_mix(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() < 2 {
        return Err(format!(
            "mix requires at least 2 signals, got {}",
            args.len()
        ));
    }

    // Compile all signal arguments
    let mut signal_nodes = Vec::new();
    for arg in args {
        let node = compile_expr(ctx, arg)?;
        signal_nodes.push(Signal::Node(node));
    }

    let node = SignalNode::Mix {
        signals: signal_nodes,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Conditional (if-then-else) routing
/// Syntax: if condition then_signal else_signal
/// Routes to then_signal if condition > 0.5, else routes to else_signal
fn compile_if(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!(
            "if requires 3 parameters (condition, then_signal, else_signal), got {}",
            args.len()
        ));
    }

    let condition_node = compile_expr(ctx, args[0].clone())?;
    let then_node = compile_expr(ctx, args[1].clone())?;
    let else_node = compile_expr(ctx, args[2].clone())?;

    let node = SignalNode::Conditional {
        condition: Signal::Node(condition_node),
        then_signal: Signal::Node(then_node),
        else_signal: Signal::Node(else_node),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Select/Multiplex node
/// Syntax: select index signal1 signal2 signal3 ...
/// Selects one of N signals based on index (pattern-modulatable)
fn compile_select(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() < 2 {
        return Err(format!(
            "select requires at least 2 parameters (index, signals...), got {}",
            args.len()
        ));
    }

    // First arg is the index signal
    let index_node = compile_expr(ctx, args[0].clone())?;

    // Remaining args are the signals to select from
    let mut signal_nodes = Vec::new();
    for arg in args.iter().skip(1) {
        let node = compile_expr(ctx, arg.clone())?;
        signal_nodes.push(Signal::Node(node));
    }

    if signal_nodes.is_empty() {
        return Err("select requires at least one signal to select from".to_string());
    }

    let node = SignalNode::Select {
        index: Signal::Node(index_node),
        inputs: signal_nodes,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Allpass filter
/// Syntax: allpass input coefficient
/// Allpass filter for phase manipulation and reverb building
fn compile_allpass(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::AllpassState;

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "allpass requires 1 parameter (coefficient), got {}",
            params.len()
        ));
    }

    let coefficient_node = compile_expr(ctx, params[0].clone())?;

    let node = SignalNode::Allpass {
        input: input_signal,
        coefficient: Signal::Node(coefficient_node),
        state: AllpassState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile SVF lowpass filter
/// Usage: signal # svf_lp frequency resonance
fn compile_svf_lp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_svf_mode(ctx, args, 0)
}

/// Compile SVF highpass filter
/// Usage: signal # svf_hp frequency resonance
fn compile_svf_hp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_svf_mode(ctx, args, 1)
}

/// Compile SVF bandpass filter
/// Usage: signal # svf_bp frequency resonance
fn compile_svf_bp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_svf_mode(ctx, args, 2)
}

/// Compile SVF notch filter
/// Usage: signal # svf_notch frequency resonance
fn compile_svf_notch(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_svf_mode(ctx, args, 3)
}

/// Helper function to compile SVF with specified mode
fn compile_svf_mode(ctx: &mut CompilerContext, args: Vec<Expr>, mode: usize) -> Result<NodeId, String> {
    use crate::unified_graph::SVFState;

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "svf requires 2 parameters (frequency, resonance), got {}",
            params.len()
        ));
    }

    let frequency_node = compile_expr(ctx, params[0].clone())?;
    let resonance_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::SVF {
        input: input_signal,
        frequency: Signal::Node(frequency_node),
        resonance: Signal::Node(resonance_node),
        mode,
        state: SVFState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Biquad lowpass filter
/// Usage: signal # bq_lp frequency q
fn compile_bq_lp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_biquad_mode(ctx, args, 0)
}

/// Compile Biquad highpass filter
/// Usage: signal # bq_hp frequency q
fn compile_bq_hp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_biquad_mode(ctx, args, 1)
}

/// Compile Biquad bandpass filter
/// Usage: signal # bq_bp frequency q
fn compile_bq_bp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_biquad_mode(ctx, args, 2)
}

/// Compile Biquad notch filter
/// Usage: signal # bq_notch frequency q
fn compile_bq_notch(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    compile_biquad_mode(ctx, args, 3)
}

/// Helper function to compile Biquad with specified mode
fn compile_biquad_mode(ctx: &mut CompilerContext, args: Vec<Expr>, mode: usize) -> Result<NodeId, String> {
    use crate::unified_graph::BiquadState;

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "biquad requires 2 parameters (frequency, q), got {}",
            params.len()
        ));
    }

    let frequency_node = compile_expr(ctx, params[0].clone())?;
    let q_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::Biquad {
        input: input_signal,
        frequency: Signal::Node(frequency_node),
        q: Signal::Node(q_node),
        mode,
        state: BiquadState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Resonz (resonant bandpass) filter
/// Usage: signal # resonz frequency q
fn compile_resonz(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::BiquadState;

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "resonz requires 2 parameters (frequency, q), got {}",
            params.len()
        ));
    }

    let frequency_node = compile_expr(ctx, params[0].clone())?;
    let q_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::Resonz {
        input: input_signal,
        frequency: Signal::Node(frequency_node),
        q: Signal::Node(q_node),
        state: BiquadState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile RLPF (resonant lowpass) filter
/// Usage: signal # rlpf cutoff resonance
fn compile_rlpf(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::BiquadState;

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "rlpf requires 2 parameters (cutoff, resonance), got {}",
            params.len()
        ));
    }

    let cutoff_node = compile_expr(ctx, params[0].clone())?;
    let resonance_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::RLPF {
        input: input_signal,
        cutoff: Signal::Node(cutoff_node),
        resonance: Signal::Node(resonance_node),
        state: BiquadState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile RHPF (resonant highpass) filter
/// Usage: signal # rhpf cutoff resonance
fn compile_rhpf(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::BiquadState;

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 2 {
        return Err(format!(
            "rhpf requires 2 parameters (cutoff, resonance), got {}",
            params.len()
        ));
    }

    let cutoff_node = compile_expr(ctx, params[0].clone())?;
    let resonance_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::RHPF {
        input: input_signal,
        cutoff: Signal::Node(cutoff_node),
        resonance: Signal::Node(resonance_node),
        state: BiquadState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile tap/probe effect for debugging signal flow
/// Usage: signal # tap "filename.wav" duration
/// Records signal to WAV file while passing it through unchanged
fn compile_tap(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    use crate::unified_graph::TapState;
    use std::sync::{Arc, Mutex};

    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() < 2 {
        return Err(format!(
            "tap requires 2 parameters (filename, duration), got {}",
            params.len()
        ));
    }

    // Extract filename (must be a string literal)
    let filename = match &params[0] {
        Expr::String(s) => s.clone(),
        _ => return Err("tap filename must be a string literal".to_string()),
    };

    // Extract duration in seconds
    let duration = extract_number(&params[1])?;
    if duration <= 0.0 {
        return Err("tap duration must be positive".to_string());
    }

    // Create tap state
    let tap_state = TapState::new(filename, duration as f32, ctx.sample_rate);

    let node = SignalNode::Tap {
        input: input_signal,
        state: Arc::new(Mutex::new(tap_state)),
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

    // Compile all parameters as signals (supports pattern modulation!)
    let attack_node = compile_expr(ctx, params[0].clone())?;
    let decay_node = compile_expr(ctx, params[1].clone())?;
    let sustain_node = compile_expr(ctx, params[2].clone())?;
    let release_node = compile_expr(ctx, params[3].clone())?;

    use crate::unified_graph::EnvState;

    // env is for continuous signals - no auto-triggering
    // For rhythmic triggering, use:
    // - struct "pattern" (signal) - imposes rhythm with auto-envelope
    // - env_trig "pattern" attack decay sustain release - pattern-triggered envelope
    let node = SignalNode::Envelope {
        input: input_signal,
        trigger: Signal::Value(1.0), // Always on (continuous envelope, goes to sustain and stays there)
        attack: Signal::Node(attack_node),
        decay: Signal::Node(decay_node),
        sustain: Signal::Node(sustain_node),
        release: Signal::Node(release_node),
        state: EnvState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_adsr(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // ADSR can be used in two ways:
    // 1. Standalone ADSR envelope generator: adsr 0.01 0.1 [:sustain 0.7] [:release 0.2]
    // 2. Sample envelope modifier: s "bd" # adsr 0.01 0.1 [:sustain 0.7] [:release 0.2]

    if args.is_empty() {
        return Err("adsr requires at least 2 parameters (attack, decay)".to_string());
    }

    // Check if first argument is ChainInput (chained form)
    let is_chained = matches!(&args[0], Expr::ChainInput(_));

    if is_chained {
        // Chained form: s "bd" # adsr 0.01 0.1 [:sustain 0.7] [:release 0.2]
        let (input_signal, params) = extract_chain_input(ctx, &args)?;

        // Use ParamExtractor for optional parameters
        let extractor = ParamExtractor::new(params);

        // attack and decay are required
        let attack_expr = extractor.get_required(0, "attack")?;
        let attack_node = compile_expr(ctx, attack_expr)?;

        let decay_expr = extractor.get_required(1, "decay")?;
        let decay_node = compile_expr(ctx, decay_expr)?;

        // sustain and release are optional
        let sustain_expr = extractor.get_optional(2, "sustain", 0.7);  // 70% sustain level
        let sustain_node = compile_expr(ctx, sustain_expr)?;

        let release_expr = extractor.get_optional(3, "release", 0.2);  // 200ms release
        let release_node = compile_expr(ctx, release_expr)?;

        // Modify the Sample node to use ADSR envelope
        use crate::unified_graph::RuntimeEnvelopeType;

        if let Signal::Node(sample_node_id) = input_signal {
            let sample_node = ctx
                .graph
                .get_node(sample_node_id)
                .ok_or_else(|| "Invalid node reference".to_string())?;

            if let SignalNode::Sample {
                pattern_str,
                pattern,
                gain,
                pan,
                speed,
                cut_group,
                n,
                note,
                unit_mode,
                loop_enabled,
                ..
            } = sample_node
            {
                // Create new Sample with ADSR envelope
                let new_sample = SignalNode::Sample {
                    pattern_str: pattern_str.clone(),
                    pattern: pattern.clone(),
                    last_trigger_time: -1.0,
                    last_cycle: -1,
                    playback_positions: HashMap::new(),
                    gain: gain.clone(),
                    pan: pan.clone(),
                    speed: speed.clone(),
                    cut_group: cut_group.clone(),
                    n: n.clone(),
                    note: note.clone(),
                    attack: Signal::Node(attack_node),
                    release: Signal::Node(release_node),
                    envelope_type: Some(RuntimeEnvelopeType::ADSR {
                        decay: Signal::Node(decay_node),
                        sustain: Signal::Node(sustain_node),
                    }),
                    unit_mode: unit_mode.clone(),
                    loop_enabled: loop_enabled.clone(),
                    begin: Signal::Value(0.0),
                    end: Signal::Value(1.0),
                };

                Ok(ctx.graph.add_node(new_sample))
            } else {
                Err("adsr modifier can only be used with sample (s) patterns".to_string())
            }
        } else {
            Err("adsr modifier requires input from chain operator (#)".to_string())
        }
    } else {
        // Standalone form: adsr 0.01 0.1 [:sustain 0.7] [:release 0.2]
        use crate::unified_graph::ADSRState;

        // Use ParamExtractor for optional parameters
        let extractor = ParamExtractor::new(args);

        // attack and decay are required
        let attack_expr = extractor.get_required(0, "attack")?;
        let attack_node = compile_expr(ctx, attack_expr)?;

        let decay_expr = extractor.get_required(1, "decay")?;
        let decay_node = compile_expr(ctx, decay_expr)?;

        // sustain and release are optional
        let sustain_expr = extractor.get_optional(2, "sustain", 0.7);  // 70% sustain level
        let sustain_node = compile_expr(ctx, sustain_expr)?;

        let release_expr = extractor.get_optional(3, "release", 0.2);  // 200ms release
        let release_node = compile_expr(ctx, release_expr)?;

        let node = SignalNode::ADSR {
            attack: Signal::Node(attack_node),
            decay: Signal::Node(decay_node),
            sustain: Signal::Node(sustain_node),
            release: Signal::Node(release_node),
            state: ADSRState::default(),
        };

        Ok(ctx.graph.add_node(node))
    }
}

fn compile_ad(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Use ParamExtractor for keyword argument support
    let extractor = ParamExtractor::new(args);

    // Both parameters are required
    let attack_expr = extractor.get_required(0, "attack")?;
    let attack_node = compile_expr(ctx, attack_expr)?;

    let decay_expr = extractor.get_required(1, "decay")?;
    let decay_node = compile_expr(ctx, decay_expr)?;

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

fn compile_curve(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 4 {
        return Err(format!(
            "curve requires 4 parameters (start, end, duration, curve), got {}",
            args.len()
        ));
    }

    // Compile each parameter as a signal (supports pattern modulation!)
    let start_node = compile_expr(ctx, args[0].clone())?;
    let end_node = compile_expr(ctx, args[1].clone())?;
    let duration_node = compile_expr(ctx, args[2].clone())?;
    let curve_node = compile_expr(ctx, args[3].clone())?;

    let node = SignalNode::Curve {
        start: Signal::Node(start_node),
        end: Signal::Node(end_node),
        duration: Signal::Node(duration_node),
        curve: Signal::Node(curve_node),
        elapsed_time: 0.0, // Start at beginning
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Segments envelope (arbitrary breakpoint)
/// Syntax: segments "level0 level1 level2 ..." "time1 time2 ..."
/// Example: segments "0 1 0.5 0" "0.1 0.2 0.1"
fn compile_segments(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "segments requires 2 parameters (levels_string, times_string), got {}",
            args.len()
        ));
    }

    // Extract levels string
    let levels_str = match &args[0] {
        Expr::String(s) => s.clone(),
        _ => return Err("segments requires first argument to be a string of levels".to_string()),
    };

    // Extract times string
    let times_str = match &args[1] {
        Expr::String(s) => s.clone(),
        _ => return Err("segments requires second argument to be a string of times".to_string()),
    };

    // Parse levels
    let levels: Result<Vec<f32>, _> = levels_str
        .split_whitespace()
        .map(|s| s.parse::<f32>())
        .collect();

    let levels = levels.map_err(|e| format!("Failed to parse levels: {}", e))?;

    // Parse times
    let times: Result<Vec<f32>, _> = times_str
        .split_whitespace()
        .map(|s| s.parse::<f32>())
        .collect();

    let times = times.map_err(|e| format!("Failed to parse times: {}", e))?;

    // Validate: N levels needs N-1 times
    if levels.len() < 2 {
        return Err("segments requires at least 2 levels".to_string());
    }

    if times.len() != levels.len() - 1 {
        return Err(format!(
            "segments requires {} times for {} levels (got {})",
            levels.len() - 1,
            levels.len(),
            times.len()
        ));
    }

    let node = SignalNode::Segments {
        levels,
        times,
        current_segment: 0,
        segment_elapsed: 0.0,
        current_value: 0.0, // Will be set to first level on first sample
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile RMS (Root Mean Square) analyzer
/// Syntax: rms input window_size
/// Example: ~level = ~signal # rms 0.01
fn compile_rms(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // RMS expects 1 param after input: window_size (in seconds)
    if params.len() != 1 {
        return Err(format!(
            "rms requires 1 parameter (window_size), got {}",
            params.len()
        ));
    }

    let window_size_node = compile_expr(ctx, params[0].clone())?;

    // Create buffer based on maximum expected window size
    // We'll allocate 1 second worth of samples as max
    let max_buffer_size = ctx.sample_rate as usize;
    let buffer = vec![0.0; max_buffer_size];

    let node = SignalNode::RMS {
        input: input_signal,
        window_size: Signal::Node(window_size_node),
        buffer,
        write_idx: 0,
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Schmidt trigger (gate with hysteresis)
/// Syntax: schmidt input high_threshold low_threshold
/// Example: ~gate = ~input # schmidt 0.5 -0.5
fn compile_schmidt(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Schmidt expects 2 params after input: high_threshold, low_threshold
    if params.len() != 2 {
        return Err(format!(
            "schmidt requires 2 parameters (high_threshold, low_threshold), got {}",
            params.len()
        ));
    }

    let high_threshold_node = compile_expr(ctx, params[0].clone())?;
    let low_threshold_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::Schmidt {
        input: input_signal,
        high_threshold: Signal::Node(high_threshold_node),
        low_threshold: Signal::Node(low_threshold_node),
        state: false, // Start in low state
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile Latch (Sample & Hold)
/// Syntax: latch input gate
/// Example: ~held = ~noise # latch ~clock
fn compile_latch(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Latch expects 1 param after input: gate signal
    if params.len() != 1 {
        return Err(format!(
            "latch requires 1 parameter (gate), got {}",
            params.len()
        ));
    }

    let gate_node = compile_expr(ctx, params[0].clone())?;

    let node = SignalNode::Latch {
        input: input_signal,
        gate: Signal::Node(gate_node),
        held_value: 0.0, // Start with 0
        last_gate: 0.0,  // Start with gate low
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_timer(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Timer takes no additional parameters - only the trigger input
    if !params.is_empty() {
        return Err(format!(
            "timer requires no parameters (only trigger input), got {}",
            params.len()
        ));
    }

    let node = SignalNode::Timer {
        trigger: input_signal,
        elapsed_time: 0.0, // Start at 0
        last_trigger: 0.0, // Start with trigger low
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_peak_follower(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Peak follower requires 2 parameters: attack_time, release_time
    if params.len() != 2 {
        return Err(format!(
            "peak_follower requires 2 parameters (attack_time, release_time), got {}",
            params.len()
        ));
    }

    let attack_node = compile_expr(ctx, params[0].clone())?;
    let release_node = compile_expr(ctx, params[1].clone())?;

    let node = SignalNode::PeakFollower {
        input: input_signal,
        attack_time: Signal::Node(attack_node),
        release_time: Signal::Node(release_node),
        current_peak: 0.0, // Start at 0
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_amp_follower(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Amp follower requires 3 parameters: attack_time, release_time, window_size
    if params.len() != 3 {
        return Err(format!(
            "amp_follower requires 3 parameters (attack_time, release_time, window_size), got {}",
            params.len()
        ));
    }

    let attack_node = compile_expr(ctx, params[0].clone())?;
    let release_node = compile_expr(ctx, params[1].clone())?;
    let window_node = compile_expr(ctx, params[2].clone())?;

    // Initialize with a reasonable default buffer size (10ms at 44.1kHz)
    let initial_buffer_size = 441;

    let node = SignalNode::AmpFollower {
        input: input_signal,
        attack_time: Signal::Node(attack_node),
        release_time: Signal::Node(release_node),
        window_size: Signal::Node(window_node),
        buffer: vec![0.0; initial_buffer_size],
        write_idx: 0,
        current_envelope: 0.0,
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
        Expr::BusRef(bus_name) => {
            // Bus references in chains are BROKEN
            // The correct behavior would be to re-instantiate the bus's effect chain
            // with the left signal as input, but buses are compiled to NodeIds which
            // can't be cloned with new inputs.
            //
            // For now: just return the left signal (pass-through)
            // This at least preserves the signal instead of dropping it

            eprintln!("⚠️  Warning: Bus '~{}' used in chain - effect will be ignored", bus_name);
            eprintln!("   Workaround: Use the effect directly instead of through a bus");
            eprintln!("   e.g., 's \"bd\" # delay 0.25 0.8' instead of 's \"bd\" # ~mydelay'");

            // Return left signal (pass-through)
            compile_expr(ctx, left)
        }
        Expr::Var(name) => {
            // Treat as zero-argument function call with chain input
            // This handles cases like: ~trigger # timer
            let left_node = compile_expr(ctx, left)?;
            let args = vec![Expr::ChainInput(left_node)];
            compile_function_call(ctx, &name, args)
        }
        Expr::TemplateRef(name) => {
            // Expand template and re-chain with the expanded expression
            // This handles: s "bd" # @filt where @filt: lpf 1000 0.8
            let template_expr = ctx
                .templates
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("Undefined template: @{}", name))?;

            // Recursively compile chain with expanded template
            compile_chain(ctx, left, template_expr)
        }
        _ => {
            // For other expressions, just compile them separately and connect
            let _left_node = compile_expr(ctx, left)?;
            compile_expr(ctx, right)
        }
    }
}

/// Helper to modify a Sample node's parameter
/// Returns a new Sample node with the updated parameter
fn modify_sample_param(
    ctx: &mut CompilerContext,
    sample_node_id: NodeId,
    param_name: &str,
    new_value: Signal,
) -> Result<NodeId, String> {
    // Get the Sample node
    let sample_node = ctx
        .graph
        .get_node(sample_node_id)
        .ok_or_else(|| "Invalid node reference".to_string())?;

    if let SignalNode::Sample {
        pattern_str,
        pattern,
        gain,
        pan,
        speed,
        cut_group,
        n,
        note,
        attack,
        release,
        envelope_type,
        unit_mode,
        loop_enabled,
        ..
    } = sample_node
    {
        // Create new Sample with updated parameter
        let new_sample = SignalNode::Sample {
            pattern_str: pattern_str.clone(),
            pattern: pattern.clone(),
            last_trigger_time: -1.0,
            last_cycle: -1,
            playback_positions: HashMap::new(),
            gain: if param_name == "gain" {
                new_value.clone()
            } else {
                gain.clone()
            },
            pan: if param_name == "pan" {
                new_value.clone()
            } else {
                pan.clone()
            },
            speed: if param_name == "speed" {
                new_value.clone()
            } else {
                speed.clone()
            },
            cut_group: cut_group.clone(),
            n: if param_name == "n" {
                new_value.clone()
            } else {
                n.clone()
            },
            note: if param_name == "note" {
                new_value.clone()
            } else {
                note.clone()
            },
            attack: if param_name == "attack" {
                new_value.clone()
            } else {
                attack.clone()
            },
            release: if param_name == "release" {
                new_value.clone()
            } else {
                release.clone()
            },
            envelope_type: envelope_type.clone(),
            unit_mode: if param_name == "unit" {
                new_value.clone()
            } else {
                unit_mode.clone()
            },
            loop_enabled: if param_name == "loop" {
                new_value
            } else {
                loop_enabled.clone()
            },
            begin: Signal::Value(0.0),
            end: Signal::Value(1.0),
        };

        Ok(ctx.graph.add_node(new_sample))
    } else {
        Err(format!(
            "{} can only be used with sample (s) patterns, not other signals",
            param_name
        ))
    }
}

/// Check if a transform contains Transform::Effect (recursively)
fn transform_contains_effect(transform: &Transform) -> bool {
    match transform {
        Transform::Effect(_) => true,
        Transform::Every { transform, .. } => transform_contains_effect(transform),
        Transform::EveryPrime { transform, .. } => transform_contains_effect(transform),
        Transform::Sometimes(transform) => transform_contains_effect(transform),
        Transform::SometimesBy { transform, .. } => transform_contains_effect(transform),
        Transform::Whenmod { transform, .. } => transform_contains_effect(transform),
        Transform::Compose(transforms) => transforms.iter().any(|t| transform_contains_effect(t)),
        _ => false,
    }
}

/// Compile a transform that contains effect chains as conditional signal nodes
fn compile_conditional_effect_transform(
    ctx: &mut CompilerContext,
    expr: Expr,
    transform: Transform,
) -> Result<NodeId, String> {
    // Check if this is a Compose with mixed pattern and effect transforms
    if let Transform::Compose(ref transforms) = transform {
        let mut has_pattern = false;
        let mut has_effect = false;

        for t in transforms {
            if transform_contains_effect(t) {
                has_effect = true;
            } else {
                has_pattern = true;
            }
        }

        // If we have BOTH pattern and effect transforms, we need special handling
        if has_pattern && has_effect {
            return compile_mixed_conditional_transform(ctx, expr, transform);
        }
    }

    // Otherwise, compile the base expression (input signal) and apply effect transform
    let input_node = compile_expr(ctx, expr)?;
    compile_effect_transform(ctx, Signal::Node(input_node), transform)
}

/// Compile a conditional transform that mixes pattern and effect transforms
/// Example: every 3 (fast 2 $ # lpf 300)
/// This applies BOTH the pattern transform AND the effect transform conditionally
fn compile_mixed_conditional_transform(
    ctx: &mut CompilerContext,
    expr: Expr,
    transform: Transform,
) -> Result<NodeId, String> {
    // For now, we'll apply the whole composed transform at the pattern level,
    // then handle effects at the signal level
    // This is a simplified implementation - a full implementation would need
    // to create conditional nodes at both pattern and signal levels

    // Compile the base expression (which should create a signal)
    let input_node = compile_expr(ctx, expr)?;

    // Apply the transform (which will route through normal pattern transform path)
    // Actually, this won't work because we're already in the effect transform path

    // For now, return a helpful error
    Err(
        "Mixed pattern+effect transforms in same conditional not yet fully supported.\n\
         Workaround: Chain separate conditionals:\n\
         Instead of: every 3 (fast 2 $ # lpf 300)\n\
         Use: every 3 (fast 2) $ every 3 (# lpf 300)".to_string()
    )
}

/// Compile a transform with effects into conditional signal nodes
fn compile_effect_transform(
    ctx: &mut CompilerContext,
    input: Signal,
    transform: Transform,
) -> Result<NodeId, String> {
    match transform {
        Transform::Every { n, transform } => {
            let n_val = match *n {
                Expr::Number(num) => num as i32,
                _ => return Err("every requires a numeric argument".to_string()),
            };

            // Check if the inner transform is an effect
            if let Transform::Effect(effect_expr) = *transform {
                // Compile the effect expression with the input as ChainInput
                let effect_node = compile_effect_chain(ctx, input.clone(), *effect_expr)?;

                // Create EveryEffect node
                let node = SignalNode::EveryEffect {
                    input,
                    effect: Signal::Node(effect_node),
                    n: n_val,
                };
                Ok(ctx.graph.add_node(node))
            } else if transform_contains_effect(&transform) {
                // Recursive case: inner transform contains effects
                compile_effect_transform(ctx, input, *transform)
            } else {
                Err("Expected effect transform inside every".to_string())
            }
        }

        Transform::Sometimes(transform) => {
            if let Transform::Effect(effect_expr) = *transform {
                let effect_node = compile_effect_chain(ctx, input.clone(), *effect_expr)?;

                let node = SignalNode::SometimesEffect {
                    input,
                    effect: Signal::Node(effect_node),
                    prob: 0.5,
                };
                Ok(ctx.graph.add_node(node))
            } else if transform_contains_effect(&transform) {
                compile_effect_transform(ctx, input, *transform)
            } else {
                Err("Expected effect transform inside sometimes".to_string())
            }
        }

        Transform::SometimesBy { prob, transform } => {
            let prob_val = match *prob {
                Expr::Number(num) => num,
                _ => return Err("sometimesBy requires a numeric probability".to_string()),
            };

            if let Transform::Effect(effect_expr) = *transform {
                let effect_node = compile_effect_chain(ctx, input.clone(), *effect_expr)?;

                let node = SignalNode::SometimesEffect {
                    input,
                    effect: Signal::Node(effect_node),
                    prob: prob_val,
                };
                Ok(ctx.graph.add_node(node))
            } else if transform_contains_effect(&transform) {
                compile_effect_transform(ctx, input, *transform)
            } else {
                Err("Expected effect transform inside sometimesBy".to_string())
            }
        }

        Transform::Whenmod { modulo, offset, transform } => {
            let modulo_val = match *modulo {
                Expr::Number(num) => num as i32,
                _ => return Err("whenmod requires numeric modulo".to_string()),
            };
            let offset_val = match *offset {
                Expr::Number(num) => num as i32,
                _ => return Err("whenmod requires numeric offset".to_string()),
            };

            if let Transform::Effect(effect_expr) = *transform {
                let effect_node = compile_effect_chain(ctx, input.clone(), *effect_expr)?;

                let node = SignalNode::WhenmodEffect {
                    input,
                    effect: Signal::Node(effect_node),
                    modulo: modulo_val,
                    offset: offset_val,
                };
                Ok(ctx.graph.add_node(node))
            } else if transform_contains_effect(&transform) {
                compile_effect_transform(ctx, input, *transform)
            } else {
                Err("Expected effect transform inside whenmod".to_string())
            }
        }

        Transform::Effect(effect_expr) => {
            // Directly compile the effect chain
            compile_effect_chain(ctx, input, *effect_expr)
        }

        Transform::Compose(transforms) => {
            // Handle composition of pattern transforms and effect transforms
            // Separate pattern transforms from effect transforms
            let mut pattern_transforms = Vec::new();
            let mut effect_transforms = Vec::new();

            for t in transforms {
                if transform_contains_effect(&t) {
                    effect_transforms.push(t);
                } else {
                    pattern_transforms.push(t);
                }
            }

            // If we have ONLY effects, apply them sequentially
            if pattern_transforms.is_empty() {
                let mut current_signal = input;
                for t in effect_transforms {
                    let node_id = compile_effect_transform(ctx, current_signal, t)?;
                    current_signal = Signal::Node(node_id);
                }
                match current_signal {
                    Signal::Node(id) => Ok(id),
                    _ => Err("Expected node after effect chain".to_string()),
                }
            } else {
                // We have mixed pattern and effect transforms
                // This is a more complex case - we need to handle this at the
                // conditional level, not here. For now, return an error.
                Err("Mixed pattern and effect transforms in Compose not yet supported in conditional context. Use separate conditionals or chain them: every 3 (fast 2) $ every 3 (# lpf 300)".to_string())
            }
        }

        _ => Err(format!("Unsupported transform for effects: {:?}", transform)),
    }
}

/// Compile an effect chain expression with a given input
fn compile_effect_chain(
    ctx: &mut CompilerContext,
    input: Signal,
    effect_expr: Expr,
) -> Result<NodeId, String> {
    // Replace ChainInput markers in the effect expression with the actual input
    // and compile the chain
    match effect_expr {
        Expr::Chain(left, right) => {
            // Chain: left # right
            // Compile left with input, then pass result to right
            let left_node = if matches!(*left, Expr::ChainInput(_)) {
                // Left is the input
                match input {
                    Signal::Node(id) => id,
                    _ => return Err("Expected node for chain input".to_string()),
                }
            } else {
                compile_effect_chain(ctx, input.clone(), *left)?
            };

            // Compile right with left as input
            compile_effect_chain(ctx, Signal::Node(left_node), *right)
        }

        Expr::Call { name, mut args } => {
            // This is an effect function - inject the input as the first argument
            args.insert(0, Expr::ChainInput(match input {
                Signal::Node(id) => id,
                _ => return Err("Expected node for effect input".to_string()),
            }));
            compile_function_call(ctx, &name, args)
        }

        _ => Err(format!("Unsupported effect expression: {:?}", effect_expr)),
    }
}

/// Compile pattern transform
fn compile_transform(
    ctx: &mut CompilerContext,
    expr: Expr,
    transform: Transform,
) -> Result<NodeId, String> {
    // First, check if this transform contains effect chains (Transform::Effect)
    // If so, compile as conditional signal nodes instead of pattern transforms
    if transform_contains_effect(&transform) {
        return compile_conditional_effect_transform(ctx, expr, transform);
    }

    // Handle function calls like `s "bd sn" $ fast 2`
    if let Expr::Call { name, args } = &expr {
        // Check if this is the `s` function (sample pattern)
        if name == "s" && !args.is_empty() {
            if let Expr::String(pattern_str) = &args[0] {
                // Parse and transform the pattern
                let mut pattern = parse_mini_notation(&pattern_str);
                pattern = apply_transform_to_pattern(ctx, pattern, transform)?;

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
                    envelope_type: None,
                    unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                    loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                    begin: Signal::Value(0.0),
                    end: Signal::Value(1.0),
                };
                return Ok(ctx.graph.add_node(node));
            }
        }
    }

    // For string literals, we can apply transforms directly to the parsed pattern
    if let Expr::String(pattern_str) = expr {
        let mut pattern = parse_mini_notation(&pattern_str);

        // Apply the transform to the pattern
        pattern = apply_transform_to_pattern(ctx, pattern, transform)?;

        // Create a Pattern node with the transformed pattern
        let node = SignalNode::Pattern {
            pattern_str: format!("{} (transformed)", pattern_str),
            pattern,
            last_value: 0.0,
            last_trigger_time: -1.0,
        };
        return Ok(ctx.graph.add_node(node));
    }

    // Handle nested transforms: Transform { expr: Transform { ... }, transform }
    // This happens when you chain multiple transforms: expr $ transform1 $ transform2
    if let Expr::Transform {
        expr: inner_expr,
        transform: inner_transform,
    } = expr.clone()
    {
        // Collect all transforms in the chain
        fn collect_transforms(expr: Expr, transforms: &mut Vec<Transform>) -> Expr {
            match expr {
                Expr::Transform {
                    expr: inner,
                    transform,
                } => {
                    transforms.push(transform);
                    collect_transforms(*inner, transforms)
                }
                other => other,
            }
        }

        let mut all_transforms = vec![transform];
        let base_expr = collect_transforms(
            Expr::Transform {
                expr: inner_expr,
                transform: inner_transform,
            },
            &mut all_transforms,
        );

        // Now base_expr should be either a Call or String, and all_transforms has all transforms
        // Apply all transforms in reverse order (they were collected outer-to-inner)
        match base_expr {
            Expr::Call { name, args } if name == "s" && !args.is_empty() => {
                if let Expr::String(pattern_str) = &args[0] {
                    let mut pattern = parse_mini_notation(pattern_str);

                    // Apply all transforms in reverse order (innermost first)
                    for t in all_transforms.iter().rev() {
                        pattern = apply_transform_to_pattern(ctx, pattern, t.clone())?;
                    }

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
                        envelope_type: None,
                        unit_mode: Signal::Value(0.0), // 0 = rate mode (default)
                        loop_enabled: Signal::Value(0.0), // 0 = no loop (default)
                        begin: Signal::Value(0.0),
                        end: Signal::Value(1.0),
                    };
                    return Ok(ctx.graph.add_node(node));
                }
            }
            Expr::String(pattern_str) => {
                let mut pattern = parse_mini_notation(&pattern_str);

                // Apply all transforms in reverse order (innermost first)
                for t in all_transforms.iter().rev() {
                    pattern = apply_transform_to_pattern(ctx, pattern, t.clone())?;
                }

                let node = SignalNode::Pattern {
                    pattern_str: format!("{} (transformed)", pattern_str),
                    pattern,
                    last_value: 0.0,
                    last_trigger_time: -1.0,
                };
                return Ok(ctx.graph.add_node(node));
            }
            _ => {}
        }
    }

    // For other expressions, compile them first then try to extract and transform
    // This is more complex and may not always work
    // For now, just compile the expression without the transform
    // TODO: Handle transforms on arbitrary expressions
    compile_expr(ctx, expr)
}

/// Create a pattern from an audio signal with range mapping
/// This bridges the gap between continuous audio signals and discrete pattern parameters
///
/// Implementation:
///   1. Creates SignalAsPattern node from bus signal
///   2. Node samples signal once per cycle (thread-safe with Arc<Mutex>)
///   3. Pattern closure reads cached value from shared state
///   4. Provides dynamic audio→pattern coupling
fn create_signal_pattern_for_transform(
    ctx: &mut CompilerContext,
    bus_name: &str,
    out_min: f32,
    out_max: f32,
    _transform_name: &str,
) -> Result<Pattern<f64>, String> {
    use std::sync::Arc;
    use std::sync::Mutex;
    use crate::unified_graph::{SignalNode, Signal};
    use crate::pattern::Hap;
    use std::collections::HashMap;

    // Create shared state cells for thread-safe communication
    let midpoint = (out_min + out_max) / 2.0;
    let sampled_value = Arc::new(Mutex::new(midpoint));
    let sample_cycle = Arc::new(Mutex::new(-1.0f32));

    // Create SignalAsPattern node that will sample the bus signal
    let sap_node = SignalNode::SignalAsPattern {
        signal: Signal::Bus(bus_name.to_string()),
        last_sampled_value: sampled_value.clone(),
        last_sample_cycle: sample_cycle.clone(),
    };

    // Add node to graph (this will be evaluated during audio processing)
    ctx.graph.add_node(sap_node);

    // Create pattern that reads from shared state
    let value_ref = sampled_value.clone();
    let pattern = Pattern::new(move |state| {
        // Read the current sampled value (set by SignalAsPattern during audio eval)
        let value = *value_ref.lock().unwrap() as f64;

        // Return a single event spanning the query span with the sampled value
        vec![Hap {
            whole: Some(state.span.clone()),
            part: state.span.clone(),
            value,
            context: HashMap::new(),
        }]
    });

    Ok(pattern)
}

/// Helper for applying transforms in closures where we only have templates
/// This version doesn't support Expr::Bus in transform parameters
/// For bus support, use apply_transform_to_pattern with full CompilerContext
fn apply_transform_to_pattern_simple<T: Clone + Send + Sync + Debug + 'static>(
    templates: &HashMap<String, Expr>,
    pattern: Pattern<T>,
    transform: Transform,
) -> Result<Pattern<T>, String> {
    // Create a minimal context with just templates
    // This is a hack - we can't actually compile new nodes in this context
    // But for nested transforms in closures, we only need template resolution

    let mut ctx = CompilerContext::new(44100.0);
    ctx.templates = templates.clone();

    // Call the main function
    apply_transform_to_pattern(&mut ctx, pattern, transform)
}

/// Apply a transform to a pattern
fn apply_transform_to_pattern<T: Clone + Send + Sync + Debug + 'static>(
    ctx: &mut CompilerContext,
    pattern: Pattern<T>,
    transform: Transform,
) -> Result<Pattern<T>, String> {
    match transform {
        // Template reference: look up template and apply it
        Transform::TemplateRef(name) => {
            // Look up the template expression
            let template_expr = ctx.templates
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("Undefined template: @{}", name))?;

            // The template should be a transform function call
            // Extract the transform from the expression
            if let Expr::Call { name: fn_name, args } = template_expr {
                // Match against known transform functions
                let transform = match fn_name.as_str() {
                    "fast" if args.len() == 1 => Transform::Fast(Box::new(args[0].clone())),
                    "slow" if args.len() == 1 => Transform::Slow(Box::new(args[0].clone())),
                    "squeeze" if args.len() == 1 => Transform::Squeeze(Box::new(args[0].clone())),
                    "rev" if args.is_empty() => Transform::Rev,
                    "palindrome" if args.is_empty() => Transform::Palindrome,
                    "degrade" if args.is_empty() => Transform::Degrade,
                    "degradeBy" if args.len() == 1 => Transform::DegradeBy(Box::new(args[0].clone())),
                    "stutter" if args.len() == 1 => Transform::Stutter(Box::new(args[0].clone())),
                    "shuffle" if args.len() == 1 => Transform::Shuffle(Box::new(args[0].clone())),
                    "swing" if args.len() == 1 => Transform::Swing(Box::new(args[0].clone())),
                    "chop" if args.len() == 1 => Transform::Chop(Box::new(args[0].clone())),
                    "slice" if args.len() == 2 => Transform::Slice {
                        n: Box::new(args[0].clone()),
                        indices: Box::new(args[1].clone()),
                    },
                    "scramble" if args.len() == 1 => Transform::Scramble(Box::new(args[0].clone())),
                    "iter" if args.len() == 1 => Transform::Iter(Box::new(args[0].clone())),
                    "loopAt" if args.len() == 1 => Transform::LoopAt(Box::new(args[0].clone())),
                    "early" if args.len() == 1 => Transform::Early(Box::new(args[0].clone())),
                    "late" if args.len() == 1 => Transform::Late(Box::new(args[0].clone())),
                    _ => return Err(format!("Template @{} is not a valid transform", name)),
                };

                // Recursively apply the extracted transform
                apply_transform_to_pattern(ctx, pattern, transform)
            } else {
                Err(format!("Template @{} is not a transform function", name))
            }
        }
        Transform::Fast(speed_expr) => {
            // All speeds are patterns - constants wrapped with Pattern::pure()
            let speed_pattern = match speed_expr.as_ref() {
                Expr::String(s) => {
                    // Pattern-based speed: fast "2 3 4"
                    let string_pattern = parse_mini_notation(s);
                    // Convert Pattern<String> to Pattern<f64>
                    string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0))
                }
                Expr::BusRef(bus_name) => {
                    // AUTO-MAGIC: Audio signal with sensible default range
                    // For 'fast', map to 0.25x - 4x speed range
                    create_signal_pattern_for_transform(
                        ctx,
                        bus_name,
                        0.25,
                        4.0,
                        "fast",
                    )?
                }
                Expr::PatternRef(pattern_name) => {
                    // Pattern-to-pattern modulation: fast %speed
                    ctx.pattern_registry
                        .get(pattern_name)
                        .cloned()
                        .ok_or_else(|| format!("Undefined pattern: %{}", pattern_name))?
                }
                _ => {
                    // Constant speed: fast 2 -> Pattern::pure(2.0)
                    let speed = extract_number(&speed_expr)?;
                    Pattern::pure(speed)
                }
            };
            Ok(pattern.fast(speed_pattern))
        }
        Transform::Slow(speed_expr) => {
            // All speeds are patterns - constants wrapped with Pattern::pure()
            let speed_pattern = match speed_expr.as_ref() {
                Expr::String(s) => {
                    // Pattern-based speed: slow "2 3 4"
                    let string_pattern = parse_mini_notation(s);
                    // Convert Pattern<String> to Pattern<f64>
                    string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0))
                }
                Expr::BusRef(bus_name) => {
                    // AUTO-MAGIC: Audio signal with sensible default range
                    // For 'slow', map to 0.25x - 4x speed range (same as fast)
                    create_signal_pattern_for_transform(
                        ctx,
                        bus_name,
                        0.25,
                        4.0,
                        "slow",
                    )?
                }
                Expr::PatternRef(pattern_name) => {
                    // Pattern-to-pattern modulation: slow %speed
                    ctx.pattern_registry
                        .get(pattern_name)
                        .cloned()
                        .ok_or_else(|| format!("Undefined pattern: %{}", pattern_name))?
                }
                _ => {
                    // Constant speed: slow 2 -> Pattern::pure(2.0)
                    let speed = extract_number(&speed_expr)?;
                    Pattern::pure(speed)
                }
            };
            Ok(pattern.slow(speed_pattern))
        }
        Transform::Hurry(factor_expr) => {
            // Hurry = fast + speed combined (Tidal's hurry)
            let factor_pattern = match factor_expr.as_ref() {
                Expr::String(s) => {
                    // Pattern-based hurry: hurry "2 3 4"
                    let string_pattern = parse_mini_notation(s);
                    string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0))
                }
                _ => {
                    // Constant hurry: hurry 2 -> Pattern::pure(2.0)
                    let factor = extract_number(&factor_expr)?;
                    Pattern::pure(factor)
                }
            };
            Ok(pattern.hurry(factor_pattern))
        }
        Transform::Squeeze(factor_expr) => {
            // Support both pattern strings and constant numbers
            match factor_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based squeeze - parse string pattern and convert to f64
                    let string_pattern = parse_mini_notation(pattern_str);
                    let factor_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0));
                    Ok(pattern.squeeze_pattern(factor_pattern))
                }
                _ => {
                    // Constant squeeze
                    let factor = extract_number(&factor_expr)?;
                    Ok(pattern.squeeze(factor))
                }
            }
        }
        Transform::Rev => Ok(pattern.rev()),
        Transform::Degrade => Ok(pattern.degrade()),
        Transform::DegradeBy(prob_expr) => {
            // Support both pattern strings and constant numbers
            match prob_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based probability - parse string pattern and convert to f64
                    let string_pattern = parse_mini_notation(pattern_str);
                    let prob_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.5));
                    Ok(pattern.degrade_by(prob_pattern))
                }
                Expr::BusRef(bus_name) => {
                    // AUTO-MAGIC: Audio signal controls probability (0-1 range)
                    let prob_pattern = create_signal_pattern_for_transform(
                        ctx,
                        bus_name,
                        0.0,
                        1.0,
                        "degradeBy",
                    )?;
                    Ok(pattern.degrade_by(prob_pattern))
                }
                Expr::PatternRef(pattern_name) => {
                    // Pattern-to-pattern modulation: degradeBy %prob
                    let prob_pattern = ctx.pattern_registry
                        .get(pattern_name)
                        .cloned()
                        .ok_or_else(|| format!("Undefined pattern: %{}", pattern_name))?;
                    Ok(pattern.degrade_by(prob_pattern))
                }
                _ => {
                    // Constant probability
                    let prob = extract_number(&prob_expr)?;
                    Ok(pattern.degrade_by(Pattern::pure(prob)))
                }
            }
        }
        Transform::Stutter(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.stutter(n))
        }
        Transform::Palindrome => Ok(pattern.palindrome()),
        Transform::Shuffle(amount_expr) => {
            // Support both pattern strings and constant numbers
            match amount_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based shuffle amount - parse string pattern and convert to f64
                    let string_pattern = parse_mini_notation(pattern_str);
                    let amount_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.5));
                    Ok(pattern.shuffle(amount_pattern))
                }
                Expr::PatternRef(pattern_name) => {
                    // Pattern-to-pattern modulation: shuffle %amount
                    let amount_pattern = ctx.pattern_registry
                        .get(pattern_name)
                        .cloned()
                        .ok_or_else(|| format!("Undefined pattern: %{}", pattern_name))?;
                    Ok(pattern.shuffle(amount_pattern))
                }
                _ => {
                    // Constant shuffle amount
                    let amount = extract_number(&amount_expr)?;
                    Ok(pattern.shuffle(Pattern::pure(amount)))
                }
            }
        }
        Transform::Chop(n_expr) | Transform::Striate(n_expr) => {
            // chop and striate are aliases - both slice pattern into n parts
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.chop(n))
        }
        Transform::Stripe(n_expr) => {
            // stripe n - repeat pattern n times over n cycles at random speeds
            let n = extract_number(&n_expr)? as usize;
            Ok(Pattern::stripe(n, pattern))
        }
        Transform::Bite { n, selector } => {
            // bite n selector - slice into n bits, select which to play with selector pattern
            let n_val = extract_number(&n)? as usize;

            // Extract selector pattern from the expression
            let selector_pattern = match selector.as_ref() {
                Expr::String(s) => {
                    // Parse mini-notation pattern
                    use crate::mini_notation_v3::parse_mini_notation;
                    parse_mini_notation(s)
                }
                Expr::Number(num) => {
                    // Single number - create a pattern with just that index
                    Pattern::from_string(&num.to_string())
                }
                _ => return Err("bite selector must be a string pattern or number".to_string()),
            };

            Ok(pattern.bite(n_val, selector_pattern))
        }
        Transform::Slice { n, indices } => {
            // slice n indices_pattern - reorder n slices by indices
            let n_val = extract_number(&n)? as usize;

            // Extract indices pattern from the expression
            let indices_pattern = match indices.as_ref() {
                Expr::String(s) => {
                    // Parse mini-notation pattern
                    use crate::mini_notation_v3::parse_mini_notation;
                    parse_mini_notation(s)
                }
                Expr::Number(num) => {
                    // Single number - create a pattern with just that index
                    Pattern::from_string(&num.to_string())
                }
                _ => return Err("slice indices must be a string pattern (e.g., \"0 2 1 3\")".to_string()),
            };

            Ok(pattern.slice_pattern(n_val, indices_pattern))
        }
        Transform::Struct(struct_expr) => {
            // struct pattern - apply structure/rhythm from boolean pattern to values
            // Example: struct "t ~ t ~" or struct "t(3,8)"
            let struct_pattern = match struct_expr.as_ref() {
                Expr::String(s) => {
                    // Parse mini-notation pattern for structure
                    // Convert "t" to true, "~" to false
                    use crate::mini_notation_v3::parse_mini_notation;

                    // Parse as string pattern first
                    let string_pattern = parse_mini_notation(s);

                    // Convert to boolean pattern: "t" -> true, anything else (including "~") -> false
                    Pattern::new(move |state| {
                        string_pattern
                            .query(state)
                            .into_iter()
                            .map(|hap| crate::pattern::Hap {
                                whole: hap.whole,
                                part: hap.part,
                                value: hap.value == "t",
                                context: hap.context,
                            })
                            .collect()
                    })
                }
                _ => return Err("struct pattern must be a string (e.g., \"t ~ t ~\" or \"t(3,8)\")".to_string()),
            };

            Ok(pattern.struct_pattern(struct_pattern))
        }
        Transform::Scramble(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.scramble(n))
        }
        Transform::Swing(amount_expr) => {
            // Support both pattern strings and constant numbers
            match amount_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based swing - parse string pattern and convert to f64
                    let string_pattern = parse_mini_notation(pattern_str);
                    let amount_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.5));
                    Ok(pattern.swing(amount_pattern))
                }
                _ => {
                    // Constant swing
                    let amount = extract_number(&amount_expr)?;
                    Ok(pattern.swing(Pattern::pure(amount)))
                }
            }
        }
        Transform::Legato(factor_expr) => {
            // Support both pattern strings and constant numbers
            match factor_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based legato - parse string pattern and convert to f64
                    let string_pattern = parse_mini_notation(pattern_str);
                    let factor_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0));
                    Ok(pattern.legato(factor_pattern))
                }
                _ => {
                    // Constant legato
                    let factor = extract_number(&factor_expr)?;
                    Ok(pattern.legato(Pattern::pure(factor)))
                }
            }
        }
        Transform::Staccato(factor_expr) => {
            // Support both pattern strings and constant numbers
            match factor_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based staccato - parse string pattern and convert to f64
                    let string_pattern = parse_mini_notation(pattern_str);
                    let factor_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0));
                    Ok(pattern.staccato(factor_pattern))
                }
                _ => {
                    // Constant staccato
                    let factor = extract_number(&factor_expr)?;
                    Ok(pattern.staccato(Pattern::pure(factor)))
                }
            }
        }
        Transform::Echo {
            times,
            time,
            feedback,
        } => {
            let times_val = extract_number(&times)? as usize;
            let time_val = extract_number(&time)?;
            let feedback_val = extract_number(&feedback)?;
            Ok(pattern.echo(times_val, Pattern::pure(time_val), Pattern::pure(feedback_val)))
        }
        Transform::Stut { n, time, decay } => {
            let n_val = extract_number(&n)?;
            let time_val = extract_number(&time)?;
            let decay_val = extract_number(&decay)?;
            Ok(pattern.stut(
                Pattern::pure(n_val),
                Pattern::pure(time_val),
                Pattern::pure(decay_val),
            ))
        }
        Transform::Segment(n_expr) => {
            let n = extract_number(&n_expr)? as usize;
            Ok(pattern.segment(n))
        }
        Transform::Zoom { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.zoom(Pattern::pure(begin_val), Pattern::pure(end_val)))
        }
        Transform::Compress { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.compress(Pattern::pure(begin_val), Pattern::pure(end_val)))
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
            // All amounts are patterns - constants wrapped with Pattern::pure()
            let amount_pattern = match amount_expr.as_ref() {
                Expr::String(s) => {
                    // Pattern-based amount: late "0.25 0.5"
                    let string_pattern = parse_mini_notation(s);
                    string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.0))
                }
                Expr::BusRef(bus_name) => {
                    // AUTO-MAGIC: Audio signal controls timing offset (-0.5 to 0.5)
                    create_signal_pattern_for_transform(
                        ctx,
                        bus_name,
                        -0.5,
                        0.5,
                        "late",
                    )?
                }
                _ => {
                    // Constant amount: late 0.5 -> Pattern::pure(0.5)
                    let amount = extract_number(&amount_expr)?;
                    Pattern::pure(amount)
                }
            };
            Ok(pattern.late(amount_pattern))
        }
        Transform::Early(amount_expr) => {
            // All amounts are patterns - constants wrapped with Pattern::pure()
            let amount_pattern = match amount_expr.as_ref() {
                Expr::String(s) => {
                    // Pattern-based amount: early "0.25 0.5"
                    let string_pattern = parse_mini_notation(s);
                    string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(0.0))
                }
                Expr::BusRef(bus_name) => {
                    // AUTO-MAGIC: Audio signal controls timing offset (-0.5 to 0.5)
                    create_signal_pattern_for_transform(
                        ctx,
                        bus_name,
                        -0.5,
                        0.5,
                        "early",
                    )?
                }
                _ => {
                    // Constant amount: early 0.5 -> Pattern::pure(0.5)
                    let amount = extract_number(&amount_expr)?;
                    Pattern::pure(amount)
                }
            };
            Ok(pattern.early(amount_pattern))
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

            // Clone the pattern, transform, and templates for use in the closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            // Manually inline Pattern::every logic to avoid closure issues
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as i32;
                if cycle % n_val == 0 {
                    // Apply the transform on cycles divisible by n
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
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
        Transform::EveryPrime { n, offset, transform } => {
            // every' n offset transform: apply transform when (cycle - offset) % n == 0
            let n_val = extract_number(&n)? as i32;
            let offset_val = extract_number(&offset)? as i32;

            // Clone the pattern, transform, and templates for use in the closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            // Manually inline Pattern::every' logic
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as i32;
                if (cycle - offset_val) % n_val == 0 {
                    // Apply the transform on matching cycles
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
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
        Transform::FoldEvery { transforms, n } => {
            // foldEvery [t1, t2, t3] n: cycle through transforms every n cycles
            let n_val = extract_number(&n)? as i32;

            if transforms.is_empty() {
                return Ok(pattern);
            }

            // Clone everything for use in the closure
            let transforms_clone = transforms.clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            // Manually inline foldEvery logic
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as i32;

                // Determine which transform to apply based on cycle count
                if cycle % n_val == 0 {
                    let cycle_count = (cycle / n_val) as usize;
                    let transform_index = cycle_count % transforms_clone.len();
                    let selected_transform = transforms_clone[transform_index].clone();

                    // Apply the selected transform
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), selected_transform)
                    {
                        Ok(transformed) => transformed.query(state),
                        Err(_) => pattern_clone.query(state), // Fallback to original on error
                    }
                } else {
                    // Use original pattern on non-matching cycles
                    pattern_clone.query(state)
                }
            }))
        }
        Transform::Sometimes(transform) => {
            // Apply transform 50% of the time (per cycle)
            use rand::{rngs::StdRng, Rng, SeedableRng};

            // Clone the pattern, transform, and templates for use in the closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            // Manually inline Pattern::sometimes logic to avoid closure issues
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as u64;
                let mut rng = StdRng::seed_from_u64(cycle);

                if rng.gen::<f64>() < 0.5 {
                    // Apply the transform with 50% probability
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
                    {
                        Ok(transformed) => transformed.query(state),
                        Err(_) => pattern_clone.query(state), // Fallback to original on error
                    }
                } else {
                    // Use original pattern otherwise
                    pattern_clone.query(state)
                }
            }))
        }
        Transform::SometimesBy { prob, transform } => {
            // Apply transform with specified probability (per cycle)
            use rand::{rngs::StdRng, Rng, SeedableRng};

            // Extract the probability
            let prob_val = extract_number(&prob)?;

            // Clone the pattern, transform, and templates for use in the closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            // Manually inline Pattern::sometimes_by logic to avoid closure issues
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as u64;
                let mut rng = StdRng::seed_from_u64(cycle);

                if rng.gen::<f64>() < prob_val {
                    // Apply the transform with specified probability
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
                    {
                        Ok(transformed) => transformed.query(state),
                        Err(_) => pattern_clone.query(state), // Fallback to original on error
                    }
                } else {
                    // Use original pattern otherwise
                    pattern_clone.query(state)
                }
            }))
        }
        Transform::Rot(n_expr) => {
            // rot n - rotate values by n positions
            let rot_pattern = match n_expr.as_ref() {
                Expr::String(s) => Pattern::from_string(s),
                _ => {
                    // Try to extract as number (handles negative, parentheses, etc.)
                    let n = extract_number(n_expr.as_ref())?;
                    Pattern::pure(n.to_string())
                }
            };
            Ok(pattern.rot(rot_pattern))
        }
        Transform::Trunc(fraction_expr) => {
            // trunc fraction - truncate to play only first fraction of cycle
            let frac = extract_number(fraction_expr.as_ref())?;
            let fraction_pattern = Pattern::pure(frac);
            Ok(pattern.trunc(fraction_pattern))
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
        Transform::LoopAt(n_expr) => {
            // Check if this is a pattern (string) or constant (number)
            match n_expr.as_ref() {
                Expr::String(pattern_str) => {
                    // Pattern-based loopAt
                    let duration_pattern = parse_mini_notation(pattern_str);
                    Ok(pattern.loop_at_pattern(duration_pattern))
                }
                _ => {
                    // Constant loopAt
                    let n = extract_number(&n_expr)?;
                    Ok(pattern.loop_at(Pattern::pure(n)))
                }
            }
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
            Ok(pattern.focus(Pattern::pure(begin_val), Pattern::pure(end_val)))
        }
        Transform::Smooth(_amount_expr) => {
            // Note: smooth() only works on Pattern<f64>, not Pattern<T>
            Err("smooth transform only works with numeric patterns (from oscillators), not sample patterns".to_string())
        }
        Transform::Trim { begin, end } => {
            let begin_val = extract_number(&begin)?;
            let end_val = extract_number(&end)?;
            Ok(pattern.trim(Pattern::pure(begin_val), Pattern::pure(end_val)))
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
            // Clone pattern, transform, and templates for use in closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(Pattern::new(move |state| {
                let cycle_phase = state.span.begin.to_float() % 1.0;
                if cycle_phase >= begin_val && cycle_phase < end_val {
                    // Inside the range: apply transform
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
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
            // Clone pattern, transform, and templates for use in closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(Pattern::new(move |state| {
                let cycle_phase = state.span.begin.to_float() % 1.0;
                if cycle_phase < begin_val || cycle_phase >= end_val {
                    // Outside the range: apply transform
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
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
            let templates_clone = ctx.templates.clone();

            Ok(pattern.superimpose(move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Chunk { n, transform } => {
            let n_val = extract_number(&n)? as usize;
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.chunk(n_val, move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Sometimes(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.sometimes(move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Often(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.often(move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Rarely(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.rarely(move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::SometimesBy { prob, transform } => {
            let prob_val = extract_number(&prob)?;
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.sometimes_by(prob_val, move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::AlmostAlways(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.sometimes_by(0.9, move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::AlmostNever(transform) => {
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.sometimes_by(0.1, move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                }
            }))
        }

        Transform::Always(transform) => {
            let inner_transform = (*transform).clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.always(move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
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
            let templates_clone = ctx.templates.clone();

            Ok(pattern.when_mod(
                modulo_val,
                offset_val,
                move |p| match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                },
            ))
        }

        Transform::Wait(cycles_expr) => {
            let cycles = extract_number(&cycles_expr)?;
            // wait is an alias for late
            Ok(pattern.late(Pattern::pure(cycles)))
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

        Transform::Jux(transform) => {
            let inner_transform = (*transform).clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.jux_ctx(move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(e) => panic!("Transform error in jux: {}", e),
                }
            }))
        }

        Transform::JuxBy { amount, transform } => {
            let amount_val = extract_number(&amount)?;
            let inner_transform = (*transform).clone();
            let templates_clone = ctx.templates.clone();

            Ok(pattern.jux_by_ctx(Pattern::pure(amount_val), move |p| {
                match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(e) => panic!("Transform error in juxBy: {}", e),
                }
            }))
        }

        Transform::Compose(transforms) => {
            // Apply transforms in sequence (left to right)
            let mut result = pattern;
            for transform in transforms {
                result = apply_transform_to_pattern(ctx, result, transform)?;
            }
            Ok(result)
        }

        Transform::Undegrade => Ok(pattern.undegrade()),

        Transform::Accelerate(rate_expr) => {
            let rate = extract_number(&rate_expr)?;
            Ok(pattern.accelerate(Pattern::pure(rate)))
        }

        Transform::Humanize {
            time_var,
            velocity_var,
        } => {
            let time_var_val = extract_number(&time_var)?;
            let velocity_var_val = extract_number(&velocity_var)?;
            Ok(pattern.humanize(Pattern::pure(time_var_val), Pattern::pure(velocity_var_val)))
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
            let templates_clone = ctx.templates.clone();

            Ok(pattern.within(
                begin_val,
                end_val,
                move |p| match apply_transform_to_pattern_simple(&templates_clone, p, inner_transform.clone()) {
                    Ok(transformed) => transformed,
                    Err(_) => pattern_clone.clone(),
                },
            ))
        }

        Transform::Whenmod {
            modulo,
            offset,
            transform,
        } => {
            // Apply transform when (cycle - offset) % modulo == 0
            let modulo_val = extract_number(&modulo)? as i32;
            let offset_val = extract_number(&offset)? as i32;

            // Clone the pattern, transform, and templates for use in the closure
            let inner_transform = (*transform).clone();
            let pattern_clone = pattern.clone();
            let templates_clone = ctx.templates.clone();

            // Manually inline Pattern::whenmod logic
            Ok(Pattern::new(move |state| {
                let cycle = state.span.begin.to_float().floor() as i32;
                if (cycle - offset_val) % modulo_val == 0 {
                    // Apply the transform on matching cycles
                    match apply_transform_to_pattern_simple(&templates_clone, pattern_clone.clone(), inner_transform.clone())
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

        Transform::Euclid { pulses, steps } => {
            let pulses_val = extract_number(&pulses)? as usize;
            let steps_val = extract_number(&steps)? as usize;
            Ok(pattern.euclidean_legato(pulses_val, steps_val))
        }

        Transform::Effect(_effect_expr) => {
            // Transform::Effect is not a pattern transform - it's a marker for effect chains
            // that should be handled at the signal level by creating conditional signal nodes.
            // This case should not be reached in normal operation as the compiler should
            // detect effect transforms and handle them specially.
            Err("Transform::Effect cannot be applied to patterns - it must be handled at the signal level".to_string())
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

// ========== Sample Parameter Modifier Functions ==========

/// Compile n modifier: s "bd" # n "0 1 2"
/// Sets the sample index for sample selection
fn compile_n_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "n requires 2 arguments (sample_input, n_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a Sample node
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "n must be used with the chain operator: s \"bd\" # n \"0 1 2\"".to_string(),
            )
        }
    };

    eprintln!(
        "[DEBUG] n modifier: input node = {}, creating modified node...",
        sample_node_id.0
    );

    // Second arg is the n pattern
    let n_value = compile_expr(ctx, args[1].clone())?;

    // Modify the Sample node
    let result = modify_sample_param(ctx, sample_node_id, "n", Signal::Node(n_value))?;
    eprintln!("[DEBUG] n modifier: output node = {}", result.0);
    Ok(result)
}

/// Compile note modifier: s "bd" # note "0 5 7"
/// Sets the pitch shift in semitones for sample playback
fn compile_note_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "note requires 2 arguments (sample_input, note_pattern), got {}",
            args.len()
        ));
    }

    // First arg should be ChainInput pointing to a Sample node
    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "note must be used with the chain operator: s \"bd\" # note \"0 5 7\"".to_string(),
            )
        }
    };

    // Second arg is the note pattern (semitone offsets)
    let note_value = compile_expr(ctx, args[1].clone())?;

    // Modify the Sample node
    modify_sample_param(ctx, sample_node_id, "note", Signal::Node(note_value))
}

/// Compile gain modifier: s "bd" # gain "0.8 0.5 1.0"
/// Sets the volume for each sample trigger
fn compile_gain_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Try to modify sample parameter if it's a Sample node
    // Otherwise, fall back to general signal multiplication
    if args.len() != 2 {
        return Err(format!(
            "gain requires 2 arguments (input, gain_amount), got {}",
            args.len()
        ));
    }

    let input_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "gain must be used with the chain operator: s \"bd\" # gain 0.8".to_string(),
            )
        }
    };

    // Check if input is a Sample node - if so, modify its gain parameter
    // Otherwise, create a Multiply node (works for any signal)
    if let Some(node) = ctx.graph.get_node(input_node_id) {
        if matches!(node, SignalNode::Sample { .. }) {
            // It's a sample - modify its gain parameter
            let gain_value = compile_expr(ctx, args[1].clone())?;
            return modify_sample_param(ctx, input_node_id, "gain", Signal::Node(gain_value));
        }
    }

    // Not a sample (e.g., after # lpf) - use general signal multiplication
    let gain_node = compile_expr(ctx, args[1].clone())?;
    let output = ctx.graph.add_node(SignalNode::Multiply {
        a: Signal::Node(input_node_id),
        b: Signal::Node(gain_node),
    });
    Ok(output)
}

/// Compile pan modifier: s "bd" # pan "-1 1 0"
/// Sets the stereo pan position (-1 = left, 0 = center, 1 = right)
fn compile_pan_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "pan requires 2 arguments (sample_input, pan_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "pan must be used with the chain operator: s \"bd\" # pan \"-1 1\"".to_string(),
            )
        }
    };

    let pan_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "pan", Signal::Node(pan_value))
}

/// Compile speed modifier: s "bd" # speed "1 0.5 2"
/// Sets the playback speed (1.0 = normal, 0.5 = half speed, 2.0 = double speed)
fn compile_speed_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "speed requires 2 arguments (sample_input, speed_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "speed must be used with the chain operator: s \"bd\" # speed \"1 2\"".to_string(),
            )
        }
    };

    let speed_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "speed", Signal::Node(speed_value))
}

/// Compile begin modifier: s "bd" # begin "0 0.25 0.5"
/// Sets the start point for sample slicing (0.0 = start, 0.5 = middle, 1.0 = end)
fn compile_begin_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "begin requires 2 arguments (sample_input, begin_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "begin must be used with the chain operator: s \"bd\" # begin \"0.5\"".to_string(),
            )
        }
    };

    let begin_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "begin", Signal::Node(begin_value))
}

/// Compile end modifier: s "bd" # end "0.5 0.75 1"
/// Sets the end point for sample slicing (0.0 = start, 0.5 = middle, 1.0 = end)
fn compile_end_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "end requires 2 arguments (sample_input, end_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "end must be used with the chain operator: s \"bd\" # end \"1\"".to_string(),
            )
        }
    };

    let end_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "end", Signal::Node(end_value))
}

/// Compile cut modifier: s "bd" # cut "1 2 1"
/// Sets the cut group for voice stealing (samples in same group stop each other)
fn compile_cut_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "cut requires 2 arguments (sample_input, cut_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "cut must be used with the chain operator: s \"bd\" # cut \"1\"".to_string(),
            )
        }
    };

    let cut_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "cut", Signal::Node(cut_value))
}

/// Compile unit modifier: s "bd" # unit "c"
/// Sets the playback unit mode ("r" = rate mode, "c" = cycle mode)
fn compile_unit_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "unit requires 2 arguments (sample_input, unit_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "unit must be used with the chain operator: s \"bd\" # unit \"c\"".to_string(),
            )
        }
    };

    let unit_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "unit", Signal::Node(unit_value))
}

/// Compile loop modifier: s "bd" # loop "1"
/// Sets whether the sample should loop (0 = play once, 1 = loop continuously)
fn compile_loop_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "loop requires 2 arguments (sample_input, loop_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "loop must be used with the chain operator: s \"bd\" # loop \"1\"".to_string(),
            )
        }
    };

    let loop_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "loop", Signal::Node(loop_value))
}

/// Compile attack modifier: s "bd" # attack "0.01 0.1"
/// Sets the envelope attack time in seconds
fn compile_attack_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "attack requires 2 arguments (sample_input, attack_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "attack must be used with the chain operator: s \"bd\" # attack \"0.01\""
                    .to_string(),
            )
        }
    };

    let attack_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "attack", Signal::Node(attack_value))
}

/// Compile release modifier: s "bd" # release "0.1 0.2"
/// Sets the envelope release time in seconds
fn compile_release_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "release requires 2 arguments (sample_input, release_pattern), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "release must be used with the chain operator: s \"bd\" # release \"0.1\""
                    .to_string(),
            )
        }
    };

    let release_value = compile_expr(ctx, args[1].clone())?;
    modify_sample_param(ctx, sample_node_id, "release", Signal::Node(release_value))
}

/// Compile ar modifier: s "bd" # ar 0.01 0.5
/// Shorthand for setting both attack and release times
/// Common in Tidal/SuperCollider for quick envelope shaping
fn compile_ar_modifier(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!(
            "ar requires 3 arguments (sample_input, attack_time, release_time), got {}",
            args.len()
        ));
    }

    let sample_node_id = match &args[0] {
        Expr::ChainInput(node_id) => *node_id,
        _ => {
            return Err(
                "ar must be used with the chain operator: s \"bd\" # ar 0.01 0.5"
                    .to_string(),
            )
        }
    };

    // Set attack time
    let attack_value = compile_expr(ctx, args[1].clone())?;
    let node_after_attack = modify_sample_param(ctx, sample_node_id, "attack", Signal::Node(attack_value))?;

    // Set release time
    let release_value = compile_expr(ctx, args[2].clone())?;
    modify_sample_param(ctx, node_after_attack, "release", Signal::Node(release_value))
}

/// Compile amp modifier: applies amplitude/gain to ANY signal
/// Works with oscillators, samples, filters, etc.
/// Usage: sine 440 # amp 0.3  OR  s "bd" # amp "0.5 0.8 1.0"
fn compile_amp(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input signal and parameters
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 1 {
        return Err(format!(
            "amp requires 1 parameter (amplitude), got {}",
            params.len()
        ));
    }

    // Compile the amplitude value (can be a number or pattern)
    let amp_value = compile_expr(ctx, params[0].clone())?;

    // Create a Multiply node to apply amplitude
    let node = SignalNode::Multiply {
        a: input_signal,
        b: Signal::Node(amp_value),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile struct function: imposes boolean pattern structure on a signal
/// Usage: struct "t(3,8)" (sine "444")
/// The boolean pattern determines when the signal triggers
/// Each "true" event triggers an envelope on the input signal
fn compile_struct(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!(
            "struct requires 2 arguments (pattern_string, signal), got {}",
            args.len()
        ));
    }

    // First argument: boolean pattern string
    let pattern_str = match &args[0] {
        Expr::String(s) => s.clone(),
        _ => return Err("struct requires a pattern string as first argument".to_string()),
    };

    // Parse the boolean pattern
    // In mini-notation, "t" = true, "f" or "~" = false, "x" = true
    let bool_pattern =
        parse_mini_notation(&pattern_str).fmap(|s: String| s == "t" || s == "x" || s == "1");

    // Second argument: signal to apply structure to
    let signal_node = compile_expr(ctx, args[1].clone())?;
    let input_signal = Signal::Node(signal_node);

    // Create StructuredSignal node with default percussive envelope
    // Default: fast attack (1ms), short decay (100ms), no sustain, short release (50ms)
    // This gives a "ping" sound typical of live coding
    use crate::unified_graph::EnvState;

    let node = SignalNode::StructuredSignal {
        input: input_signal,
        bool_pattern_str: pattern_str.clone(),
        bool_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        attack: 0.001, // 1ms attack
        decay: 0.1,    // 100ms decay
        sustain: 0.0,  // No sustain (percussive)
        release: 0.05, // 50ms release
        state: EnvState::default(),
    };

    Ok(ctx.graph.add_node(node))
}

/// Compile run pattern generator: run 4 -> generates 0,1,2,3 per cycle
fn compile_run(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!("run requires 1 argument (n), got {}", args.len()));
    }

    // Extract n value
    let n = extract_number(&args[0])? as usize;
    if n == 0 {
        return Err("run requires n > 0".to_string());
    }

    // Create the run pattern
    let pattern = Pattern::<f64>::run(n);

    // Wrap in PatternEvaluator node
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

/// Compile scan pattern generator: scan 4 -> cumulative pattern (0), (0 1), (0 1 2), (0 1 2 3)
fn compile_scan(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!("scan requires 1 argument (n), got {}", args.len()));
    }

    // Extract n value
    let n = extract_number(&args[0])? as usize;
    if n == 0 {
        return Err("scan requires n > 0".to_string());
    }

    // Create the scan pattern
    let pattern = Pattern::<f64>::scan(n);

    // Wrap in PatternEvaluator node
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

/// Compile irand pattern generator: irand 4 -> random integers 0-3 per cycle
fn compile_irand(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 1 {
        return Err(format!("irand requires 1 argument (n), got {}", args.len()));
    }

    // Extract n value
    let n = extract_number(&args[0])? as usize;
    if n == 0 {
        return Err("irand requires n > 0".to_string());
    }

    // Create the irand pattern
    let pattern = Pattern::<f64>::irand(n);

    // Wrap in PatternEvaluator node
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

/// Compile rand pattern generator: rand -> random floats 0.0-1.0 per cycle
fn compile_rand(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("rand takes no arguments, got {}", args.len()));
    }

    // Create the rand pattern
    let pattern = Pattern::<f64>::rand();

    // Wrap in PatternEvaluator node
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_sine_wave(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("sine takes no arguments, got {}", args.len()));
    }

    let pattern = Pattern::<f64>::sine_wave();
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_cosine_wave(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("cosine takes no arguments, got {}", args.len()));
    }

    let pattern = Pattern::<f64>::cosine_wave();
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_saw_wave(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("saw takes no arguments, got {}", args.len()));
    }

    let pattern = Pattern::<f64>::saw_wave();
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_tri_wave(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("tri takes no arguments, got {}", args.len()));
    }

    let pattern = Pattern::<f64>::tri_wave();
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_square_wave(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if !args.is_empty() {
        return Err(format!("square takes no arguments, got {}", args.len()));
    }

    let pattern = Pattern::<f64>::square_wave();
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

/// Conditional value generators for audio effects
fn compile_every_val(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!("every_val requires 3 arguments (n, on_val, off_val), got {}", args.len()));
    }

    let n = extract_number(&args[0])? as i32;
    let on_val = extract_number(&args[1])?;
    let off_val = extract_number(&args[2])?;

    let pattern = Pattern::<f64>::every_val(n, on_val, off_val);
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_sometimes_val(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 2 {
        return Err(format!("sometimes_val requires 2 arguments (on_val, off_val), got {}", args.len()));
    }

    let on_val = extract_number(&args[0])?;
    let off_val = extract_number(&args[1])?;

    let pattern = Pattern::<f64>::sometimes_val(on_val, off_val);
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_sometimes_by_val(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 3 {
        return Err(format!("sometimes_by_val requires 3 arguments (prob, on_val, off_val), got {}", args.len()));
    }

    let prob = extract_number(&args[0])?;
    let on_val = extract_number(&args[1])?;
    let off_val = extract_number(&args[2])?;

    let pattern = Pattern::<f64>::sometimes_by_val(prob, on_val, off_val);
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

fn compile_whenmod_val(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    if args.len() != 4 {
        return Err(format!("whenmod_val requires 4 arguments (modulo, offset, on_val, off_val), got {}", args.len()));
    }

    let modulo = extract_number(&args[0])? as i32;
    let offset = extract_number(&args[1])? as i32;
    let on_val = extract_number(&args[2])?;
    let off_val = extract_number(&args[3])?;

    let pattern = Pattern::<f64>::whenmod_val(modulo, offset, on_val, off_val);
    let node = SignalNode::PatternEvaluator { pattern };
    Ok(ctx.graph.add_node(node))
}

/// Conditional effect compilers
/// These create signal-level conditional routing for effects

fn compile_every_effect(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // every_effect n (effect_chain)
    // When used in chain: input # every_effect 2 (lpf 500 0.8)
    // args[0] = ChainInput (the input signal)
    // args[1] = n (cycle interval)
    // args[2] = effect expr (the effect to apply conditionally)

    if args.len() != 3 {
        return Err(format!("every_effect requires 3 arguments (input, n, effect), got {}", args.len()));
    }

    // Extract the chained input
    let input = compile_expr(ctx, args[0].clone())?;

    // Extract n
    let n = extract_number(&args[1])? as i32;

    // Compile the effect expression (which should be an effect chain)
    let effect = compile_expr(ctx, args[2].clone())?;

    // Create the conditional effect node
    let node = SignalNode::EveryEffect {
        input: Signal::Node(input),
        effect: Signal::Node(effect),
        n,
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_sometimes_effect(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // sometimes_effect (effect_chain)
    // When used in chain: input # sometimes_effect (lpf 500 0.8)

    if args.len() != 2 {
        return Err(format!("sometimes_effect requires 2 arguments (input, effect), got {}", args.len()));
    }

    let input = compile_expr(ctx, args[0].clone())?;
    let effect = compile_expr(ctx, args[1].clone())?;

    let node = SignalNode::SometimesEffect {
        input: Signal::Node(input),
        effect: Signal::Node(effect),
        prob: 0.5,
    };

    Ok(ctx.graph.add_node(node))
}

fn compile_whenmod_effect(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // whenmod_effect modulo offset (effect_chain)
    // When used in chain: input # whenmod_effect 3 1 (lpf 500 0.8)

    if args.len() != 4 {
        return Err(format!("whenmod_effect requires 4 arguments (input, modulo, offset, effect), got {}", args.len()));
    }

    let input = compile_expr(ctx, args[0].clone())?;
    let modulo = extract_number(&args[1])? as i32;
    let offset = extract_number(&args[2])? as i32;
    let effect = compile_expr(ctx, args[3].clone())?;

    let node = SignalNode::WhenmodEffect {
        input: Signal::Node(input),
        effect: Signal::Node(effect),
        modulo,
        offset,
    };

    Ok(ctx.graph.add_node(node))
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
            eprintln!("Error message: {}", e);
            assert!(e.contains("not found") || e.contains("Undefined bus"));
        }
    }

    // ========== Pattern Transform Tests ==========

    #[test]
    fn test_compile_pattern_fast() {
        let code = r#"out: "bd sn" $ fast 2"#;
        let (_, statements) = parse_program(code).unwrap();
        let result = compile_program(statements, 44100.0);
        match result {
            Ok(_) => {},
            Err(e) => panic!("Failed to compile fast transform: {}", e),
        }
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
        match result {
            Ok(_) => {},
            Err(e) => panic!("Failed to compile sample bank selection: {}", e),
        }
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
