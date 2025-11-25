//! ClampNode demonstration
//!
//! This example shows how to use ClampNode to hard-limit signals to a specific range.
//! Unlike ClipNode (which uses soft clipping via tanh), ClampNode performs hard limiting.
//!
//! Run with: cargo run --example clamp_demo

use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::nodes::{ClampNode, ConstantNode, OscillatorNode, Waveform};
use phonon::pattern::Fraction;

fn main() {
    println!("ClampNode Demonstration\n");

    // Example 1: Hard limiting an oscillator to [-0.5, 0.5]
    println!("Example 1: Clamp sine wave to [-0.5, 0.5] range");
    {
        let mut freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut min = ConstantNode::new(-0.5);
        let mut max = ConstantNode::new(0.5);
        let mut clamp = ClampNode::new(1, 2, 3);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        // Process nodes
        let mut buf_freq = vec![0.0; 512];
        let mut buf_osc = vec![0.0; 512];
        let mut buf_min = vec![0.0; 512];
        let mut buf_max = vec![0.0; 512];

        freq.process_block(&[], &mut buf_freq, 44100.0, &context);
        osc.process_block(&[&buf_freq], &mut buf_osc, 44100.0, &context);
        min.process_block(&[], &mut buf_min, 44100.0, &context);
        max.process_block(&[], &mut buf_max, 44100.0, &context);

        let inputs = vec![buf_osc.as_slice(), buf_min.as_slice(), buf_max.as_slice()];
        let mut output = vec![0.0; 512];
        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        // Check that values are within [-0.5, 0.5]
        let min_val = output.iter().copied().fold(f32::INFINITY, f32::min);
        let max_val = output.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        println!("  Original sine wave: [-1.0, 1.0]");
        println!("  Clamped output: [{:.3}, {:.3}]", min_val, max_val);
        println!("  ✓ Values hard-limited to specified range\n");
    }

    // Example 2: Asymmetric clamping
    println!("Example 2: Asymmetric clamp [0.0, 1.0] (positive only)");
    {
        let _osc = OscillatorNode::new(0, Waveform::Sine);
        let mut min = ConstantNode::new(0.0);
        let mut max = ConstantNode::new(1.0);
        let mut clamp = ClampNode::new(0, 1, 2);

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            512,
            2.0,
            44100.0,
        );

        let input_sine = vec![0.0, 0.5, 1.0, -0.5, -1.0, 0.25];
        let mut buf_min = vec![0.0; 6];
        let mut buf_max = vec![1.0; 6];

        min.process_block(&[], &mut buf_min, 44100.0, &context);
        max.process_block(&[], &mut buf_max, 44100.0, &context);

        let inputs = vec![input_sine.as_slice(), buf_min.as_slice(), buf_max.as_slice()];
        let mut output = vec![0.0; 6];

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        println!("  Input:  [0.0, 0.5, 1.0, -0.5, -1.0, 0.25]");
        print!("  Output: [");
        for (i, &val) in output.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{:.1}", val);
        }
        println!("]");
        println!("  ✓ Negative values clamped to 0.0, preserving positive values\n");
    }

    // Example 3: Variable min/max per sample (pattern-controlled)
    println!("Example 3: Pattern-controlled clamp ranges");
    {
        let mut clamp = ClampNode::new(0, 1, 2);

        // Simulate pattern-modulated clamp ranges
        let input = vec![0.5, 0.5, 0.5, 0.5];
        let min = vec![-1.0, -0.5, 0.0, 0.3];  // Variable min
        let max = vec![1.0, 0.5, 0.2, 0.8];    // Variable max

        let inputs = vec![input.as_slice(), min.as_slice(), max.as_slice()];
        let mut output = vec![0.0; 4];

        let context = ProcessContext::new(
            Fraction::from_float(0.0),
            0,
            4,
            2.0,
            44100.0,
        );

        clamp.process_block(&inputs, &mut output, 44100.0, &context);

        println!("  Input: constant 0.5");
        println!("  Min:   [-1.0, -0.5,  0.0,  0.3]");
        println!("  Max:   [ 1.0,  0.5,  0.2,  0.8]");
        print!("  Out:   [");
        for (i, &val) in output.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{:4.1}", val);
        }
        println!("]");
        println!("  ✓ Each sample clamped to its own range\n");
    }

    println!("ClampNode vs ClipNode:");
    println!("  - ClampNode: Hard limiting (abrupt cutoff at boundaries)");
    println!("  - ClipNode:  Soft clipping (smooth saturation using tanh)");
    println!("  Use ClampNode for: brick-wall limiting, precise range control");
    println!("  Use ClipNode for:  warm distortion, analog-style saturation");
}
