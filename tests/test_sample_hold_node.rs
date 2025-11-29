/// Tests for SampleAndHoldNode - sample input when trigger crosses zero
///
/// This verifies that the sample-and-hold node:
/// 1. Holds value on trigger crossing (negative to positive)
/// 2. Updates on each trigger crossing
/// 3. Doesn't update without crossing
/// 4. Dependencies are correctly tracked
use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::nodes::constant::ConstantNode;
use phonon::nodes::sample_hold::SampleAndHoldNode;
use phonon::pattern::Fraction;

fn create_context(block_size: usize) -> ProcessContext {
    ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
}

#[test]
fn test_sample_hold_holds_value_on_trigger() {
    // When trigger crosses from negative to positive, should capture input value
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    // Input is ramping up: 0.0, 0.1, 0.2, 0.3, 0.4, 0.5
    // Trigger starts negative, crosses zero at index 2
    let input = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
    let trigger = vec![-1.0, -0.5, 0.5, 1.0, 0.8, 0.6]; // Crosses at index 2
    let inputs = vec![input.as_slice(), trigger.as_slice()];

    let mut output = vec![0.0; 6];
    let context = create_context(6);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // Should hold 0.0 initially, then capture 0.2 at the crossing
    assert_eq!(output[0], 0.0); // Initial state (held_value = 0.0)
    assert_eq!(output[1], 0.0); // Still holding 0.0 (no crossing yet)
    assert_eq!(output[2], 0.2); // Captured 0.2 at crossing
    assert_eq!(output[3], 0.2); // Holding 0.2
    assert_eq!(output[4], 0.2); // Still holding 0.2
    assert_eq!(output[5], 0.2); // Still holding 0.2
}

#[test]
fn test_sample_hold_updates_on_each_trigger_crossing() {
    // Multiple trigger crossings should update the held value each time
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    // Input changes over time
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

    // Trigger has two zero crossings: at index 1 and index 5
    let trigger = vec![
        -1.0, // neg
        0.5,  // CROSS - capture input[1] = 2.0
        0.3,  // pos
        -0.2, // neg (no cross from pos to neg)
        -0.5, // neg
        0.8,  // CROSS - capture input[5] = 6.0
        0.6,  // pos
        0.4,  // pos
    ];
    let inputs = vec![input.as_slice(), trigger.as_slice()];

    let mut output = vec![0.0; 8];
    let context = create_context(8);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // First crossing at index 1
    assert_eq!(output[0], 0.0); // Initial
    assert_eq!(output[1], 2.0); // Captured 2.0
    assert_eq!(output[2], 2.0); // Holding
    assert_eq!(output[3], 2.0); // Holding
    assert_eq!(output[4], 2.0); // Holding

    // Second crossing at index 5
    assert_eq!(output[5], 6.0); // Captured 6.0
    assert_eq!(output[6], 6.0); // Holding
    assert_eq!(output[7], 6.0); // Holding
}

#[test]
fn test_sample_hold_no_update_without_crossing() {
    // If trigger never crosses zero, should keep initial value
    // BUT: first sample triggers because last_trigger starts at 0.0
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    // Input changes
    let input = vec![1.0, 2.0, 3.0, 4.0];

    // Trigger stays positive (crossing on first sample from initial state)
    let trigger = vec![0.5, 0.8, 0.6, 0.3];
    let inputs = vec![input.as_slice(), trigger.as_slice()];

    let mut output = vec![0.0; 4];
    let context = create_context(4);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // First sample triggers (0.0 <= 0.0 && 0.5 > 0.0) captures 1.0
    // Then holds that value
    for sample in &output {
        assert_eq!(*sample, 1.0);
    }
}

#[test]
fn test_sample_hold_no_update_negative_to_negative() {
    // Trigger going from negative to more negative should not trigger
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    let input = vec![10.0, 20.0, 30.0, 40.0];

    // Trigger stays negative
    let trigger = vec![-0.1, -0.5, -0.8, -0.3];
    let inputs = vec![input.as_slice(), trigger.as_slice()];

    let mut output = vec![0.0; 4];
    let context = create_context(4);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // Should hold initial value (0.0)
    for sample in &output {
        assert_eq!(*sample, 0.0);
    }
}

#[test]
fn test_sample_hold_crossing_at_exact_zero() {
    // Trigger value exactly at 0.0 is NOT > 0.0, so doesn't trigger
    // Trigger crossing happens at index 2 (0.0 to 0.5)
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    let input = vec![5.0, 10.0, 15.0, 20.0];

    // Crossing from negative to exactly zero, then to positive
    let trigger = vec![-0.5, 0.0, 0.5, 1.0];
    let inputs = vec![input.as_slice(), trigger.as_slice()];

    let mut output = vec![0.0; 4];
    let context = create_context(4);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // No trigger at index 0 or 1 (0.0 is not > 0.0)
    // Trigger at index 2 (0.0 <= 0.0 && 0.5 > 0.0)
    assert_eq!(output[0], 0.0);
    assert_eq!(output[1], 0.0);
    assert_eq!(output[2], 15.0); // Captured at crossing
    assert_eq!(output[3], 15.0); // Holding
}

#[test]
fn test_sample_hold_dependencies() {
    // Verify input_nodes returns correct dependencies
    let sample_hold = SampleAndHoldNode::new(5, 10);
    let deps = sample_hold.input_nodes();

    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0], 5);
    assert_eq!(deps[1], 10);
}

#[test]
fn test_sample_hold_with_constant_nodes() {
    // Integration test with actual ConstantNodes
    let mut const_input = ConstantNode::new(3.14);
    let mut const_trigger = ConstantNode::new(0.5); // Always positive (crossing on first sample from initial state)
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    let context = create_context(8);

    // Process constants first
    let mut buf_input = vec![0.0; 8];
    let mut buf_trigger = vec![0.0; 8];

    const_input.process_block(&[], &mut buf_input, 44100.0, &context);
    const_trigger.process_block(&[], &mut buf_trigger, 44100.0, &context);

    // Now sample-and-hold
    let inputs = vec![buf_input.as_slice(), buf_trigger.as_slice()];
    let mut output = vec![0.0; 8];

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // First sample should trigger (last_trigger starts at 0.0, trigger[0] = 0.5)
    // So it should capture 3.14 and hold it
    for sample in &output {
        assert_eq!(*sample, 3.14);
    }
}

#[test]
fn test_sample_hold_negative_input_values() {
    // Sample-and-hold should work with negative input values
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    let input = vec![-5.0, -3.0, -1.0, 2.0, 4.0];
    let trigger = vec![-0.5, 0.5, 0.3, -0.2, 0.8]; // Crossings at index 1 and 4
    let inputs = vec![input.as_slice(), trigger.as_slice()];

    let mut output = vec![0.0; 5];
    let context = create_context(5);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    assert_eq!(output[0], 0.0); // Initial
    assert_eq!(output[1], -3.0); // Captured negative value
    assert_eq!(output[2], -3.0); // Holding
    assert_eq!(output[3], -3.0); // Holding
    assert_eq!(output[4], 4.0); // Captured new value
}

#[test]
fn test_sample_hold_full_block_512() {
    // Test with full 512-sample block
    let mut sample_hold = SampleAndHoldNode::new(0, 1);

    // Create ramping input
    let input: Vec<f32> = (0..512).map(|i| i as f32 * 0.01).collect();

    // Create trigger that crosses at sample 100
    let trigger: Vec<f32> = (0..512).map(|i| if i < 100 { -0.5 } else { 0.5 }).collect();

    let inputs = vec![input.as_slice(), trigger.as_slice()];
    let mut output = vec![0.0; 512];
    let context = create_context(512);

    sample_hold.process_block(&inputs, &mut output, 44100.0, &context);

    // Before crossing: should hold 0.0
    for i in 0..100 {
        assert_eq!(output[i], 0.0);
    }

    // After crossing: should hold value at index 100
    let held_value = 100.0 * 0.01;
    for i in 100..512 {
        assert_eq!(output[i], held_value);
    }
}
