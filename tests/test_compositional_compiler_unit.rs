//! Comprehensive unit tests for compositional_compiler.rs
//!
//! These tests verify compiler OUTPUT CORRECTNESS, not just compilation success.
//! They inspect the actual graph structure (node types, connectivity, signal routing)
//! produced by the compiler, going far beyond the existing is_ok() tests.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::{NodeId, Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform};

/// Helper: compile code and return the graph for inspection
fn compile_to_graph(code: &str) -> UnifiedSignalGraph {
    let (_, statements) = parse_program(code).unwrap();
    compile_program(statements, 44100.0, None)
        .unwrap_or_else(|e| panic!("Compilation failed: {}", e))
}

/// Helper: compile code, expect error, return the error message
fn compile_expect_error(code: &str) -> String {
    let (_, statements) = parse_program(code).unwrap();
    match compile_program(statements, 44100.0, None) {
        Err(e) => e,
        Ok(_) => panic!("Expected compilation error for: {}", code),
    }
}

// ========== 1. Basic DSL Compilation - Output Verification ==========

#[test]
fn test_constant_produces_constant_node() {
    let graph = compile_to_graph("out: 440");
    assert!(graph.has_output());
    let has_constant_440 = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Constant { value } if (*value - 440.0).abs() < 0.001)
        })
    });
    assert!(has_constant_440, "Graph should contain Constant(440.0)");
}

#[test]
fn test_sine_produces_oscillator_node() {
    let graph = compile_to_graph("out: sine 440");
    assert!(graph.has_output());
    let has_sine_osc = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Sine, .. })
        })
    });
    assert!(has_sine_osc, "Graph should contain a Sine oscillator");
}

#[test]
fn test_saw_produces_oscillator_node() {
    let graph = compile_to_graph("out: saw 110");
    let has_saw = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Saw, .. })
        })
    });
    assert!(has_saw, "Graph should contain a Saw oscillator");
}

#[test]
fn test_square_produces_oscillator_node() {
    let graph = compile_to_graph("out: square 220");
    let has_square = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Square, .. })
        })
    });
    assert!(has_square, "Graph should contain a Square oscillator");
}

#[test]
fn test_triangle_produces_oscillator_node() {
    let graph = compile_to_graph("out: tri 330");
    let has_tri = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Triangle, .. })
        })
    });
    assert!(has_tri, "Graph should contain a Triangle oscillator");
}

#[test]
fn test_oscillator_freq_is_node_signal() {
    let graph = compile_to_graph("out: sine 440");
    let osc = graph.nodes.iter().find_map(|opt| {
        opt.as_ref().and_then(|rc| match &**rc {
            SignalNode::Oscillator { freq, waveform: Waveform::Sine, .. } => Some(freq.clone()),
            _ => None,
        })
    });
    assert!(osc.is_some(), "Should have a sine oscillator");
    match osc.unwrap() {
        Signal::Node(_) => {}
        other => panic!("Expected freq to be Signal::Node, got {:?}", other),
    }
}

#[test]
fn test_s_produces_sample_node() {
    let graph = compile_to_graph(r#"out: s "bd sn hh""#);
    let has_sample = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Sample { pattern_str, .. } if pattern_str == "bd sn hh")
        })
    });
    assert!(has_sample, "Graph should contain a Sample node with pattern 'bd sn hh'");
}

#[test]
fn test_s_with_bank_selection() {
    let graph = compile_to_graph(r#"out: s "bd:0 bd:1 bd:2""#);
    let has_sample = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Sample { pattern_str, .. } if pattern_str.contains("bd:0"))
        })
    });
    assert!(has_sample, "Graph should contain a Sample node with bank selection");
}

#[test]
fn test_string_literal_produces_pattern_node() {
    let graph = compile_to_graph(r#"out: "440 880 220""#);
    let has_pattern = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| {
            matches!(&**rc, SignalNode::Pattern { pattern_str, .. } if pattern_str == "440 880 220")
        })
    });
    assert!(has_pattern, "Graph should contain a Pattern node for string literal");
}

#[test]
fn test_constant_arithmetic_uses_pattern_combination() {
    // When both operands are numeric, compiler uses pattern-level combination
    let graph = compile_to_graph("out: 100 + 200");
    assert!(graph.has_output());
    let has_pattern = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Pattern { .. }))
    });
    assert!(has_pattern, "Constant arithmetic should produce Pattern node via pattern combination");
}

#[test]
fn test_signal_level_add_uses_expression_node() {
    let graph = compile_to_graph("~a $ sine 440\n~b $ saw 220\nout $ ~a + ~b");
    let has_add = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Add { .. }))
    });
    assert!(has_add, "Signal-level addition should produce Add node with Expression");
}

#[test]
fn test_signal_level_multiply_uses_expression_node() {
    let graph = compile_to_graph("~osc $ sine 440\nout $ ~osc * 0.5");
    let has_mul_expr = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| match &**rc {
            SignalNode::Add { a: Signal::Expression(expr), .. } => {
                matches!(&**expr, SignalExpr::Multiply(..))
            }
            _ => false,
        })
    });
    assert!(has_mul_expr, "Signal multiply wraps in Add node with Multiply expression");
}

// ========== 2. Effect Chain Compilation ==========

#[test]
fn test_lpf_chain_produces_lowpass_node() {
    let graph = compile_to_graph("out: sine 440 # lpf 1000 0.8");
    let has_lpf = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. }))
    });
    assert!(has_lpf, "Graph should contain a LowPass filter node");
}

#[test]
fn test_lpf_input_is_connected() {
    let graph = compile_to_graph("out: sine 440 # lpf 1000 0.8");
    let lpf = graph.nodes.iter().find_map(|opt| {
        opt.as_ref().and_then(|rc| match &**rc {
            SignalNode::LowPass { input, cutoff, q, .. } => Some((input.clone(), cutoff.clone(), q.clone())),
            _ => None,
        })
    });
    assert!(lpf.is_some(), "Should have a LowPass node");
    let (input, cutoff, q) = lpf.unwrap();
    assert!(matches!(input, Signal::Node(_)), "LowPass input should be Signal::Node");
    assert!(matches!(cutoff, Signal::Node(_)), "LowPass cutoff should be Signal::Node");
    assert!(matches!(q, Signal::Node(_)), "LowPass Q should be Signal::Node");
}

#[test]
fn test_hpf_chain_produces_highpass_node() {
    let graph = compile_to_graph("out: saw 220 # hpf 500 1.0");
    let has_hpf = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::HighPass { .. }))
    });
    assert!(has_hpf, "Graph should contain a HighPass filter node");
}

#[test]
fn test_bpf_chain_produces_bandpass_node() {
    let graph = compile_to_graph("out: square 110 # bpf 800 2.0");
    let has_bpf = graph.nodes.iter().any(|opt| {
        opt.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::BandPass { .. }))
    });
    assert!(has_bpf, "Graph should contain a BandPass filter node");
}

#[test]
fn test_chained_filters_both_present() {
    let graph = compile_to_graph("out: saw 110 # lpf 2000 0.8 # hpf 100 0.5");
    let lpf_count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. }))).count();
    let hpf_count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::HighPass { .. }))).count();
    assert_eq!(lpf_count, 1, "Should have exactly 1 LowPass node");
    assert_eq!(hpf_count, 1, "Should have exactly 1 HighPass node");
}

#[test]
fn test_hpf_input_chains_from_lpf() {
    let graph = compile_to_graph("out: saw 110 # lpf 2000 0.8 # hpf 100 0.5");
    let lpf_id = graph.nodes.iter().enumerate().find_map(|(idx, opt)| {
        opt.as_ref().and_then(|rc| if matches!(&**rc, SignalNode::LowPass { .. }) { Some(NodeId(idx)) } else { None })
    });
    assert!(lpf_id.is_some(), "Should have LowPass node");
    let hpf_input = graph.nodes.iter().find_map(|opt| {
        opt.as_ref().and_then(|rc| match &**rc {
            SignalNode::HighPass { input, .. } => Some(input.clone()),
            _ => None,
        })
    });
    assert!(hpf_input.is_some(), "Should have HighPass node");
    match hpf_input.unwrap() {
        Signal::Node(id) => assert_eq!(id, lpf_id.unwrap(), "HPF input should reference the LPF node"),
        other => panic!("HPF input should be Signal::Node, got {:?}", other),
    }
}

#[test]
fn test_sample_through_filter_structure() {
    let graph = compile_to_graph(r#"out: s "bd sn" # lpf 2000 0.5"#);
    let has_sample = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. })));
    let has_lpf = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. })));
    assert!(has_sample, "Should have Sample node");
    assert!(has_lpf, "Should have LowPass node after sample");
}

#[test]
fn test_reverb_chain() {
    let graph = compile_to_graph("out: sine 440 # reverb 0.5 0.8");
    let has_reverb = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Reverb { .. })));
    assert!(has_reverb, "Graph should contain a Reverb node");
}

#[test]
fn test_distortion_chain() {
    let graph = compile_to_graph("out: saw 110 # distort 0.5");
    let has_dist = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Distortion { .. })));
    assert!(has_dist, "Graph should contain a Distortion node");
}

#[test]
fn test_delay_chain() {
    let graph = compile_to_graph("out: sine 440 # delay 0.5 0.3");
    let has_delay = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Delay { .. })));
    assert!(has_delay, "Graph should contain a Delay node");
}

// ========== 3. Bus Reference Compilation ==========

#[test]
fn test_bus_reference_resolves_to_correct_node() {
    let graph = compile_to_graph("~osc $ sine 440\nout $ ~osc");
    let bus_node = graph.get_bus("osc");
    assert!(bus_node.is_some(), "Bus 'osc' should be registered");
    let node = graph.get_node(bus_node.unwrap());
    assert!(matches!(node.unwrap(), SignalNode::Oscillator { waveform: Waveform::Sine, .. }));
}

#[test]
fn test_multiple_buses_registered() {
    let graph = compile_to_graph("~osc1 $ sine 440\n~osc2 $ saw 220\nout $ ~osc1 + ~osc2");
    assert!(graph.get_bus("osc1").is_some());
    assert!(graph.get_bus("osc2").is_some());
    let osc1 = graph.get_node(graph.get_bus("osc1").unwrap()).unwrap();
    assert!(matches!(osc1, SignalNode::Oscillator { waveform: Waveform::Sine, .. }));
    let osc2 = graph.get_node(graph.get_bus("osc2").unwrap()).unwrap();
    assert!(matches!(osc2, SignalNode::Oscillator { waveform: Waveform::Saw, .. }));
}

#[test]
fn test_bus_addition_produces_add_node() {
    let graph = compile_to_graph("~a $ sine 440\n~b $ sine 880\nout $ ~a + ~b");
    let has_add = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Add { .. })));
    assert!(has_add, "Mixing two buses should produce an Add node");
}

#[test]
fn test_bus_chain_through_effect() {
    let graph = compile_to_graph("~osc $ sine 440\nout $ ~osc # lpf 1000 0.8");
    let has_osc = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Sine, .. })));
    let has_lpf = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. })));
    assert!(has_osc && has_lpf, "Should have both oscillator and LowPass filter");
}

#[test]
fn test_forward_bus_reference() {
    let graph = compile_to_graph("~a $ sine 440\nout $ ~a + ~b\n~b $ saw 220");
    assert!(graph.get_bus("a").is_some());
    assert!(graph.get_bus("b").is_some());
    assert!(graph.has_output());
}

#[test]
fn test_modifier_bus_expands() {
    let graph = compile_to_graph("~filt # lpf 1000 0.8\nout $ sine 440 # ~filt");
    let has_lpf = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. })));
    assert!(has_lpf, "Modifier bus ~filt should expand to LowPass filter");
}

#[test]
fn test_bus_with_filter_params() {
    let graph = compile_to_graph("~cutoffs $ \"<300 200 1000>\" $ fast 2\nout $ s \"hh*4 cp\" # lpf ~cutoffs 0.8");
    let has_sample = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. })));
    let has_lpf = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. })));
    assert!(has_sample, "Should have sample node");
    assert!(has_lpf, "Should have LowPass using bus reference cutoff");
}

#[test]
fn test_auto_routing_when_no_out() {
    let graph = compile_to_graph("~a $ sine 440\n~b $ saw 220");
    assert!(graph.has_output(), "Auto-routing should set output when no explicit out:");
}

// ========== 4. Pattern Transform Compilation ==========

#[test]
fn test_fast_transform_marks_pattern() {
    let graph = compile_to_graph(r#"out: s "bd sn" $ fast 2"#);
    let has_sample = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| {
        matches!(&**rc, SignalNode::Sample { pattern_str, .. } if pattern_str.contains("transformed"))
    }));
    assert!(has_sample, "fast transform should mark pattern as transformed");
}

#[test]
fn test_slow_transform_on_string() {
    let graph = compile_to_graph(r#"out: "440 880" $ slow 2"#);
    let has_pattern = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| {
        matches!(&**rc, SignalNode::Pattern { pattern_str, .. } if pattern_str.contains("transformed"))
    }));
    assert!(has_pattern, "slow on string should produce transformed Pattern node");
}

#[test]
fn test_rev_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh" $ rev"#);
    let has = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| {
        matches!(&**rc, SignalNode::Sample { pattern_str, .. } if pattern_str.contains("transformed"))
    }));
    assert!(has, "rev should produce a transformed Sample node");
}

#[test]
fn test_stacked_transforms_single_sample() {
    let graph = compile_to_graph(r#"out: s "bd sn" $ fast 2 $ rev"#);
    let count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))).count();
    assert_eq!(count, 1, "Stacked transforms should produce exactly 1 Sample node");
}

#[test]
fn test_degrade_transform() {
    let graph = compile_to_graph(r#"out: s "bd*8" $ degrade"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_degrade_by_transform() {
    let graph = compile_to_graph(r#"out: s "hh*16" $ degradeBy 0.3"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_palindrome_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh" $ palindrome"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_stutter_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn" $ stutter 4"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_swing_transform() {
    let graph = compile_to_graph(r#"out: s "hh*8" $ swing 0.2"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_rotl_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh cp" $ rotL 1"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_rotr_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh cp" $ rotR 1"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_iter_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh" $ iter 3"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_chop_transform() {
    let graph = compile_to_graph(r#"out: s "bd" $ chop 4"#);
    assert!(graph.has_output());
}

#[test]
fn test_shuffle_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh cp" $ shuffle 4"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_bus_with_transform() {
    let graph = compile_to_graph("~drums $ s \"bd sn\" $ fast 2\nout $ ~drums");
    let has = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| {
        matches!(&**rc, SignalNode::Sample { pattern_str, .. } if pattern_str.contains("transformed"))
    }));
    assert!(has, "Bus with transform should produce transformed Sample");
}

#[test]
fn test_every_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn hh cp" $ every 3 rev"#);
    assert!(graph.has_output());
}

#[test]
fn test_sometimes_transform() {
    let graph = compile_to_graph(r#"out: s "bd sn" $ sometimes rev"#);
    assert!(graph.has_output());
}

// ========== 5. Error Cases ==========

#[test]
fn test_undefined_bus_error() {
    let err = compile_expect_error("out $ ~nonexistent");
    assert!(err.contains("not found") || err.contains("Undefined") || err.contains("nonexistent"),
        "Error should mention undefined bus, got: {}", err);
}

#[test]
fn test_reserved_bus_name_add() {
    let err = compile_expect_error("~add $ sine 440");
    assert!(err.contains("reserved"), "Should error about reserved name, got: {}", err);
}

#[test]
fn test_reserved_bus_name_sub() {
    let err = compile_expect_error("~sub $ sine 440");
    assert!(err.contains("reserved"), "got: {}", err);
}

#[test]
fn test_reserved_bus_name_mul() {
    let err = compile_expect_error("~mul $ sine 440");
    assert!(err.contains("reserved"), "got: {}", err);
}

#[test]
fn test_reserved_bus_name_div() {
    let err = compile_expect_error("~div $ sine 440");
    assert!(err.contains("reserved"), "got: {}", err);
}

#[test]
fn test_bare_s_produces_error() {
    // Parser interprets bare `s` as a Var -> "Undefined variable: s"
    let err = compile_expect_error("out: s");
    assert!(err.contains("Undefined") || err.contains("s"), "got: {}", err);
}

#[test]
fn test_unknown_function_error() {
    let err = compile_expect_error("out: nonexistent_fn 440");
    assert!(err.contains("Unknown") || err.contains("Undefined"), "got: {}", err);
}

// ========== 6. Statement Type Tests ==========

#[test]
fn test_tempo_statement() {
    let graph = compile_to_graph("cps: 2.0\nout: sine 440");
    assert_eq!(graph.cps, 2.0);
}

#[test]
fn test_bpm_statement() {
    let graph = compile_to_graph("bpm: 120\nout: sine 440");
    assert!((graph.cps - 0.5).abs() < 0.001, "BPM 120 in 4/4 should give cps=0.5, got {}", graph.cps);
}

#[test]
fn test_template_substitution() {
    let graph = compile_to_graph("@freq: 440\nout: sine @freq");
    assert!(graph.has_output());
    let has_osc = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc|
        matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Sine, .. })
    ));
    assert!(has_osc, "Template @freq should substitute into sine oscillator");
}

#[test]
fn test_output_channels() {
    let graph = compile_to_graph("out1: sine 440\nout2: saw 220");
    let channels = graph.get_output_channels();
    assert!(channels.len() >= 2, "Should have at least 2 output channels, got {}", channels.len());
}

// ========== 7. Oscillator Variant Tests ==========

#[test]
fn test_zero_arg_sine_is_lfo() {
    let graph = compile_to_graph("out: sine");
    let has_sine = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc|
        matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Sine, .. })
    ));
    assert!(has_sine, "Zero-arg sine should produce Sine oscillator (1Hz LFO)");
    let has_one = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc|
        matches!(&**rc, SignalNode::Constant { value } if (*value - 1.0).abs() < 0.001)
    ));
    assert!(has_one, "Zero-arg sine should use freq=1.0");
}

#[test]
fn test_zero_arg_saw_is_lfo() {
    let graph = compile_to_graph("out: saw");
    let has_saw = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc|
        matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Saw, .. })
    ));
    assert!(has_saw, "Zero-arg saw should produce Saw oscillator (1Hz LFO)");
}

#[test]
fn test_noise_produces_fundsp_unit() {
    let graph = compile_to_graph("out: noise");
    let has_noise = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc|
        matches!(&**rc, SignalNode::FundspUnit { .. })
    ));
    assert!(has_noise, "noise should produce a FundspUnit node");
}

#[test]
fn test_white_noise_produces_white_noise_node() {
    let graph = compile_to_graph("out: white_noise");
    let has_white = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc|
        matches!(&**rc, SignalNode::WhiteNoise { .. })
    ));
    assert!(has_white, "white_noise should produce a WhiteNoise node");
}

// ========== 8. Complex Composition Tests ==========

#[test]
fn test_complete_music_program() {
    let graph = compile_to_graph("~drums $ s \"bd sn hh cp\"\n~bass $ saw 55\nout $ ~drums + ~bass");
    let sample_count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))).count();
    let osc_count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Oscillator { .. }))).count();
    let add_count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Add { .. }))).count();
    assert!(sample_count >= 1, "Should have Sample node");
    assert!(osc_count >= 1, "Should have Oscillator node");
    assert!(add_count >= 1, "Should have Add node");
}

#[test]
fn test_full_effect_chain() {
    let graph = compile_to_graph("~drums $ s \"bd sn hh*2 cp\" $ fast 2\nout $ ~drums # lpf 2000 0.8 # reverb 0.3 0.5");
    let has_sample = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. })));
    let has_lpf = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. })));
    let has_reverb = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Reverb { .. })));
    assert!(has_sample && has_lpf && has_reverb, "Should have Sample, LowPass, and Reverb nodes");
}

#[test]
fn test_modulated_filter_cutoff() {
    let graph = compile_to_graph("~lfo $ sine 2\nout $ saw 110 # lpf (~lfo * 1000 + 500) 0.8");
    let has_sine = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Sine, .. })));
    let has_saw = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Oscillator { waveform: Waveform::Saw, .. })));
    let has_lpf = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::LowPass { .. })));
    assert!(has_sine, "Should have sine LFO");
    assert!(has_saw, "Should have saw oscillator");
    assert!(has_lpf, "Should have LowPass filter");
    let has_expr = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Add { a: Signal::Expression(_), .. })));
    assert!(has_expr, "Should have expression node for arithmetic modulation");
}

// ========== 9. Graph Connectivity Tests ==========

#[test]
fn test_oscillator_freq_references_constant_440() {
    let graph = compile_to_graph("out: sine 440");
    let osc_freq = graph.nodes.iter().find_map(|o| o.as_ref().and_then(|rc| match &**rc {
        SignalNode::Oscillator { freq, .. } => Some(freq.clone()),
        _ => None,
    })).expect("Should have oscillator");
    if let Signal::Node(freq_id) = osc_freq {
        let freq_node = graph.get_node(freq_id).expect("Freq node should exist");
        assert!(matches!(freq_node, SignalNode::Constant { value } if (*value - 440.0).abs() < 0.001),
            "Freq should be Constant(440.0), got: {:?}", freq_node);
    } else { panic!("Oscillator freq should be Signal::Node"); }
}

#[test]
fn test_filter_cutoff_references_constant_1000() {
    let graph = compile_to_graph("out: sine 440 # lpf 1000 0.8");
    let cutoff = graph.nodes.iter().find_map(|o| o.as_ref().and_then(|rc| match &**rc {
        SignalNode::LowPass { cutoff, .. } => Some(cutoff.clone()),
        _ => None,
    })).expect("Should have LowPass");
    if let Signal::Node(id) = cutoff {
        let node = graph.get_node(id).expect("Cutoff node should exist");
        assert!(matches!(node, SignalNode::Constant { value } if (*value - 1000.0).abs() < 0.001));
    } else { panic!("LowPass cutoff should be Signal::Node"); }
}

#[test]
fn test_filter_q_references_constant_08() {
    let graph = compile_to_graph("out: sine 440 # lpf 1000 0.8");
    let q = graph.nodes.iter().find_map(|o| o.as_ref().and_then(|rc| match &**rc {
        SignalNode::LowPass { q, .. } => Some(q.clone()),
        _ => None,
    })).expect("Should have LowPass");
    if let Signal::Node(id) = q {
        let node = graph.get_node(id).expect("Q node should exist");
        assert!(matches!(node, SignalNode::Constant { value } if (*value - 0.8).abs() < 0.001));
    } else { panic!("LowPass Q should be Signal::Node"); }
}

// ========== 10. FM/PM Oscillator ==========

#[test]
fn test_fm_oscillator() {
    let graph = compile_to_graph("out: fm 440 2 3");
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::FMOscillator { .. }))));
}

#[test]
fn test_pm_oscillator() {
    let graph = compile_to_graph("out: pm 440 2 3");
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::PMOscillator { .. }))));
}

// ========== 11. Sample Modifier ==========

#[test]
fn test_gain_modifier() {
    let graph = compile_to_graph(r#"out: s "bd sn" # gain 0.5"#);
    let modified = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| match &**rc {
        SignalNode::Sample { gain, .. } => !matches!(gain, Signal::Value(v) if (*v - 1.0).abs() < 0.001),
        _ => false,
    }));
    assert!(modified, "gain modifier should change Sample gain field from default");
}

#[test]
fn test_speed_modifier() {
    let graph = compile_to_graph(r#"out: s "bd sn" # speed 2.0"#);
    let modified = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| match &**rc {
        SignalNode::Sample { speed, .. } => !matches!(speed, Signal::Value(v) if (*v - 1.0).abs() < 0.001),
        _ => false,
    }));
    assert!(modified, "speed modifier should change Sample speed field from default");
}

#[test]
fn test_pan_modifier() {
    let graph = compile_to_graph(r#"out: s "bd sn" # pan 0.7"#);
    let modified = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| match &**rc {
        SignalNode::Sample { pan, .. } => !matches!(pan, Signal::Value(v) if (*v - 0.0).abs() < 0.001),
        _ => false,
    }));
    assert!(modified, "pan modifier should change Sample pan field from default");
}

// ========== 12. Combinators ==========

#[test]
fn test_stack_produces_mix_node() {
    let graph = compile_to_graph(r#"out: stack [s "bd", s "sn"]"#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Mix { .. }))));
}

#[test]
fn test_cat_produces_sample_node() {
    let graph = compile_to_graph(r#"out: cat ["bd", "sn", "hh"]"#);
    let has = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| {
        matches!(&**rc, SignalNode::Sample { pattern_str, .. } if pattern_str.contains("cat"))
    }));
    assert!(has, "cat should produce Sample node with 'cat' in pattern_str");
}

// ========== 13. Feedback / Self-Reference ==========

#[test]
fn test_self_reference_creates_unit_delay() {
    let graph = compile_to_graph("~input $ sine 1\n~accum $ ~input + ~accum * 0.9\nout $ ~accum");
    let has_delay = graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| {
        matches!(&**rc, SignalNode::UnitDelay { bus_name } if bus_name == "accum")
    }));
    assert!(has_delay, "Self-referencing bus should create UnitDelay node");
}

// ========== 14. Function Bus ==========

#[test]
fn test_function_bus() {
    let graph = compile_to_graph("~mix a b $ a + b\nout $ ~mix (sine 440) (sine 880)");
    assert!(graph.has_output());
    let osc_count = graph.nodes.iter().filter(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Oscillator { .. }))).count();
    assert!(osc_count >= 2, "Function bus should instantiate both oscillator arguments");
}

// ========== 15. Syntax Edge Cases ==========

#[test]
fn test_nested_parentheses() {
    assert!(compile_to_graph("out: (sine (440))").has_output());
}

#[test]
fn test_negative_number() {
    assert!(compile_to_graph("out: sine 440 * -1").has_output());
}

#[test]
fn test_euclidean_mini_notation() {
    let graph = compile_to_graph(r#"out: s "bd(3,8)""#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_alternation_mini_notation() {
    let graph = compile_to_graph(r#"out: s "<bd sn> hh""#);
    assert!(graph.nodes.iter().any(|o| o.as_ref().map_or(false, |rc| matches!(&**rc, SignalNode::Sample { .. }))));
}

#[test]
fn test_comment_handling() {
    assert!(compile_to_graph("-- comment\nout: sine 440").has_output());
}

// ========== 16. Multi-Output ==========

#[test]
fn test_multi_output_channels() {
    let graph = compile_to_graph("out1: sine 440\nout2: saw 220\nout3: square 110");
    assert!(graph.get_output_channels().len() >= 3);
}

#[test]
fn test_multi_output_node_types() {
    let graph = compile_to_graph("out1: sine 440\nout2: saw 220");
    for (ch, node_id) in &graph.get_output_channels() {
        let node = graph.get_node(*node_id).expect("Channel node should exist");
        match ch {
            1 => assert!(matches!(node, SignalNode::Oscillator { waveform: Waveform::Sine, .. })),
            2 => assert!(matches!(node, SignalNode::Oscillator { waveform: Waveform::Saw, .. })),
            _ => {}
        }
    }
}

// ========== 17. Transform Bus ==========

#[test]
fn test_transform_bus() {
    let graph = compile_to_graph("~fx $ fast 2\nout $ s \"bd sn\" $ ~fx");
    assert!(graph.has_output());
}
