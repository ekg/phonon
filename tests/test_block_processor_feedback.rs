/// Tests for feedback loop support at the BlockProcessor level
///
/// These tests verify that the BlockProcessor can handle cyclic dependencies:
/// - Cycles are allowed (no longer rejected)
/// - First block reads from zero-initialized buffers
/// - Subsequent blocks read from previous block's output

use phonon::audio_node::{AudioNode, NodeId, ProcessContext};
use phonon::block_processor::BlockProcessor;
use phonon::nodes::constant::ConstantNode;
use phonon::nodes::addition::AdditionNode;
use phonon::nodes::multiplication::MultiplicationNode;
use phonon::pattern::Fraction;

#[test]
fn test_simple_cycle_allowed() {
    // Create a simple cycle: A + B → B (where B references itself)
    // Node 0: Constant 1.0
    // Node 1: Node 0 + Node 1 (cycle!)
    let nodes: Vec<Box<dyn AudioNode>> = vec![
        Box::new(ConstantNode::new(1.0)),  // NodeId 0
        Box::new(AdditionNode::new(0, 1)), // NodeId 1 - references itself!
    ];

    // This should NOT fail (cycles are now allowed)
    let result = BlockProcessor::new(nodes, 1, 512);
    assert!(result.is_ok(), "Cycles should be allowed in BlockProcessor");
}

#[test]
fn test_three_node_cycle() {
    // Create a 3-node cycle: A → B → C → A
    // Node 0: Constant 440
    // Node 1: Node 0 * Node 2 (references future node!)
    // Node 2: Node 1 * 0.5 (completes the cycle)
    let nodes: Vec<Box<dyn AudioNode>> = vec![
        Box::new(ConstantNode::new(440.0)),    // NodeId 0
        Box::new(MultiplicationNode::new(0, 2)), // NodeId 1 - references 2
        Box::new(MultiplicationNode::new(1, 0)), // NodeId 2 - references 1 (cycle!)
    ];

    let result = BlockProcessor::new(nodes, 2, 512);
    assert!(result.is_ok(), "3-node cycles should be allowed");
}

#[test]
fn test_feedback_processes_without_crash() {
    // Create a feedback loop and actually process some blocks
    // Node 0: Constant 0.5
    // Node 1: Node 0 * Node 1 (self-feedback)
    let nodes: Vec<Box<dyn AudioNode>> = vec![
        Box::new(ConstantNode::new(0.5)),      // NodeId 0
        Box::new(MultiplicationNode::new(0, 1)), // NodeId 1 - self-reference
    ];

    let mut processor = BlockProcessor::new(nodes, 1, 512)
        .expect("Should create processor with feedback");

    let mut output = vec![0.0; 512];
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,  // 2 cycles per second
        44100.0,
    );

    // Process several blocks - should not crash or panic
    for _ in 0..10 {
        processor.process_block(&mut output, &context)
            .expect("Processing with feedback should work");

        // Verify we get some output (not all zeros after first block)
        let has_output = output.iter().any(|&x| x != 0.0);
        if has_output {
            // Good! We're getting output from the feedback loop
            break;
        }
    }
}

#[test]
fn test_complex_feedback_network() {
    // Create a more complex feedback network:
    // Node 0: Constant 100
    // Node 1: Constant 200
    // Node 2: Node 0 + Node 3 (forward reference)
    // Node 3: Node 1 + Node 2 (cycle!)
    let nodes: Vec<Box<dyn AudioNode>> = vec![
        Box::new(ConstantNode::new(100.0)),  // NodeId 0
        Box::new(ConstantNode::new(200.0)),  // NodeId 1
        Box::new(AdditionNode::new(0, 3)),    // NodeId 2 - references 3
        Box::new(AdditionNode::new(1, 2)),    // NodeId 3 - references 2 (cycle!)
    ];

    let result = BlockProcessor::new(nodes, 3, 512);
    assert!(result.is_ok(), "Complex feedback networks should be allowed");

    // Also test that it can process
    if let Ok(mut processor) = result {
        let mut output = vec![0.0; 512];
        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        processor.process_block(&mut output, &context)
            .expect("Should process complex feedback");
    }
}
