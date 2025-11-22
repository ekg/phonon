// Compiler functions for complex feedback network analysis nodes
// These should be integrated into compositional_compiler.rs

use crate::compositional_compiler::{CompilerContext, extract_chain_input, compile_expr};
use crate::compositional_parser::Expr;
use crate::unified_graph::{SignalNode, Signal, NodeId, AdaptiveCompressorState};

pub fn compile_zero_crossing(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input (handles both standalone and chained forms)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // zero_crossing has one optional parameter: window_size in seconds (default 0.1)
    let window_seconds = if params.len() == 1 {
        // Compile the provided window size
        match &params[0] {
            Expr::Number(n) => *n as f32,
            _ => {
                let _node_id = compile_expr(ctx, params[0].clone())?;
                // For non-constant, use default
                0.1
            }
        }
    } else if params.is_empty() {
        0.1 // Default 100ms window
    } else {
        return Err(format!(
            "zero_crossing requires 0 or 1 parameter (window_size), got {}",
            params.len()
        ));
    };

    // Calculate window size in samples
    let window_samples = (window_seconds * ctx.sample_rate) as u32;
    let window_samples = window_samples.max(1); // At least 1 sample

    let node = SignalNode::ZeroCrossing {
        input: input_signal,
        last_sample: 0.0,
        crossing_count: 0,
        sample_count: 0,
        window_samples,
        last_frequency: 0.0,
    };

    Ok(ctx.graph.add_node(node))
}

pub fn compile_adaptive_compressor(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // adaptive_compressor main_input sidechain_input threshold ratio attack release adaptive_factor
    // When chained: input # adaptive_compressor sidechain threshold ratio attack release adaptive_factor

    let (main_input, params) = extract_chain_input(ctx, &args)?;

    // Requires 6 parameters: sidechain_input, threshold, ratio, attack, release, adaptive_factor
    if params.len() != 6 {
        return Err(format!(
            "adaptive_compressor requires 6 parameters (sidechain, threshold, ratio, attack, release, adaptive_factor), got {}",
            params.len()
        ));
    }

    // Compile all parameters
    let sidechain_node = compile_expr(ctx, params[0].clone())?;
    let threshold_node = compile_expr(ctx, params[1].clone())?;
    let ratio_node = compile_expr(ctx, params[2].clone())?;
    let attack_node = compile_expr(ctx, params[3].clone())?;
    let release_node = compile_expr(ctx, params[4].clone())?;
    let adaptive_factor_node = compile_expr(ctx, params[5].clone())?;

    let node = SignalNode::AdaptiveCompressor {
        main_input,
        sidechain_input: Signal::Node(sidechain_node),
        threshold: Signal::Node(threshold_node),
        ratio: Signal::Node(ratio_node),
        attack: Signal::Node(attack_node),
        release: Signal::Node(release_node),
        adaptive_factor: Signal::Node(adaptive_factor_node),
        state: AdaptiveCompressorState::new(),
    };

    Ok(ctx.graph.add_node(node))
}
