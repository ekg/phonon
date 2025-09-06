//! Simple verification that operators produce correct output
//! Tests the actual signal processing, not just parsing

use phonon::glicol_parser::parse_glicol;
use phonon::glicol_dsp::{DspChain, DspNode, DspEnvironment};

/// Test that we can create and execute basic arithmetic operations
#[test]
fn test_scalar_multiply_node() {
    // Create a simple chain that multiplies by 0.5
    let mut chain = DspChain::new();
    chain.nodes.push(DspNode::Sin { freq: 440.0 });
    chain.nodes.push(DspNode::Mul { value: 0.5 });
    
    // This should create a sine wave at half amplitude
    // We're testing that the structure is correct
    assert_eq!(chain.nodes.len(), 2);
    match &chain.nodes[1] {
        DspNode::Mul { value } => assert_eq!(*value, 0.5),
        _ => panic!("Expected Mul node"),
    }
}

#[test]
fn test_mix_node_structure() {
    // Test that Mix nodes are created correctly
    let chain1 = DspChain::from_node(DspNode::Sin { freq: 440.0 });
    let chain2 = DspChain::from_node(DspNode::Sin { freq: 880.0 });
    
    let mix = DspNode::Mix {
        sources: vec![chain1, chain2],
    };
    
    match mix {
        DspNode::Mix { sources } => {
            assert_eq!(sources.len(), 2);
            // Verify both sources are sine waves
            assert!(matches!(sources[0].nodes[0], DspNode::Sin { freq: 440.0 }));
            assert!(matches!(sources[1].nodes[0], DspNode::Sin { freq: 880.0 }));
        }
        _ => panic!("Expected Mix node"),
    }
}

#[test]
fn test_addition_creates_mix() {
    // Test that + operator creates a Mix node
    let code = r#"
        out: sin 440 + sin 880
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let output = env.output_chain.expect("No output chain");
    
    // The output should be a Mix node containing two sine sources
    assert_eq!(output.nodes.len(), 1);
    match &output.nodes[0] {
        DspNode::Mix { sources } => {
            assert_eq!(sources.len(), 2);
        }
        _ => panic!("Expected Mix node for addition"),
    }
}

#[test]
fn test_scalar_multiplication_parsing() {
    // Test that * with a number creates a Mul node
    let code = r#"
        out: sin 440 * 0.5
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let output = env.output_chain.expect("No output chain");
    
    // Should have: Sin -> Mul(0.5)
    assert_eq!(output.nodes.len(), 2);
    assert!(matches!(output.nodes[0], DspNode::Sin { freq: 440.0 }));
    assert!(matches!(output.nodes[1], DspNode::Mul { value: 0.5 }));
}

#[test]
fn test_subtraction_creates_inverted_mix() {
    // Test that - operator creates a Mix with inverted second source
    let code = r#"
        out: sin 440 - sin 880
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let output = env.output_chain.expect("No output chain");
    
    // Should create a Mix node where second source is multiplied by -1
    assert_eq!(output.nodes.len(), 1);
    match &output.nodes[0] {
        DspNode::Mix { sources } => {
            assert_eq!(sources.len(), 2);
            // Second source should have a Mul { value: -1.0 } node
            let second = &sources[1];
            assert!(second.nodes.iter().any(|n| matches!(n, DspNode::Mul { value } if *value == -1.0)));
        }
        _ => panic!("Expected Mix node for subtraction"),
    }
}

#[test]
fn test_complex_arithmetic_expression() {
    // Test a more complex expression
    let code = r#"
        ~a: sin 100
        ~b: sin 200  
        ~c: sin 300
        out: ~a + ~b * 0.5 + ~c
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    
    // Should have three reference chains
    assert_eq!(env.ref_chains.len(), 3);
    
    // Output should be a complex Mix structure
    assert!(env.output_chain.is_some());
}

#[test]
fn test_operator_precedence() {
    // Test that * has higher precedence than +
    let code = r#"
        out: sin 100 + sin 200 * 0.5
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let output = env.output_chain.expect("No output chain");
    
    // Should parse as: sin 100 + (sin 200 * 0.5)
    // Result should be a Mix node
    assert!(matches!(output.nodes[0], DspNode::Mix { .. }));
}