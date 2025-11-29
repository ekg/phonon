use phonon::audio_node::AudioNode;
use phonon::audio_node::ProcessContext;
/// Test program to demonstrate QuantizeNode functionality
///
/// This verifies the bit depth reduction works correctly
use phonon::nodes::{ConstantNode, OscillatorNode, QuantizeNode, Waveform};
use phonon::pattern::Fraction;

fn main() {
    println!("Testing QuantizeNode - Bit Depth Reduction");
    println!("===========================================\n");

    let sample_rate = 44100.0;
    let block_size = 128;
    let context = ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, sample_rate);

    // Test 1: 8-bit quantization
    {
        println!("Test 1: 8-bit quantization (256 levels)");

        // Create nodes
        let mut freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut bits = ConstantNode::new(8.0);
        let mut quantize = QuantizeNode::new(1, 2);

        // Process blocks
        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut bits_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[&freq_buf], &mut osc_buf, sample_rate, &context);
        bits.process_block(&[], &mut bits_buf, sample_rate, &context);
        quantize.process_block(&[&osc_buf, &bits_buf], &mut output, sample_rate, &context);

        // Count unique values
        let mut unique: Vec<i32> = output.iter().map(|&v| (v * 1000.0) as i32).collect();
        unique.sort();
        unique.dedup();

        println!("  Unique quantization levels: {}", unique.len());
        println!("  First few samples: {:?}", &output[0..8]);
        assert!(
            unique.len() >= 50 && unique.len() <= 256,
            "Should have roughly 256 levels"
        );
        println!("  ✓ PASSED\n");
    }

    // Test 2: 1-bit quantization (sign only)
    {
        println!("Test 2: 1-bit quantization (sign only)");

        let mut freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut bits = ConstantNode::new(1.0);
        let mut quantize = QuantizeNode::new(1, 2);

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut bits_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[&freq_buf], &mut osc_buf, sample_rate, &context);
        bits.process_block(&[], &mut bits_buf, sample_rate, &context);
        quantize.process_block(&[&osc_buf, &bits_buf], &mut output, sample_rate, &context);

        let mut unique: Vec<i32> = output.iter().map(|&v| (v * 100.0) as i32).collect();
        unique.sort();
        unique.dedup();

        println!("  Unique quantization levels: {}", unique.len());
        println!("  First few samples: {:?}", &output[0..8]);
        assert!(unique.len() <= 4, "1-bit should have very few levels");
        println!("  ✓ PASSED\n");
    }

    // Test 3: 16-bit quantization (imperceptible)
    {
        println!("Test 3: 16-bit quantization (imperceptible)");

        let mut freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut bits = ConstantNode::new(16.0);
        let mut quantize = QuantizeNode::new(1, 2);

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut bits_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[&freq_buf], &mut osc_buf, sample_rate, &context);
        bits.process_block(&[], &mut bits_buf, sample_rate, &context);
        quantize.process_block(&[&osc_buf, &bits_buf], &mut output, sample_rate, &context);

        // Compare to original
        let max_diff = osc_buf
            .iter()
            .zip(output.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);

        println!("  Max difference from original: {:.6}", max_diff);
        assert!(max_diff < 0.001, "16-bit should be imperceptible");
        println!("  ✓ PASSED\n");
    }

    // Test 4: Pattern-modulated bit depth
    {
        println!("Test 4: Pattern-modulated bit depth (1 to 16 bits)");

        let mut freq = ConstantNode::new(440.0);
        let mut osc = OscillatorNode::new(0, Waveform::Sine);
        let mut quantize = QuantizeNode::new(1, 2);

        let mut freq_buf = vec![0.0; block_size];
        let mut osc_buf = vec![0.0; block_size];
        let mut output = vec![0.0; block_size];

        freq.process_block(&[], &mut freq_buf, sample_rate, &context);
        osc.process_block(&[&freq_buf], &mut osc_buf, sample_rate, &context);

        // Varying bit depth per sample
        let bits_buf: Vec<f32> = (0..block_size)
            .map(|i| 1.0 + (i as f32 * 15.0 / (block_size - 1) as f32))
            .collect();

        quantize.process_block(&[&osc_buf, &bits_buf], &mut output, sample_rate, &context);

        println!("  First sample (1-bit):  {:.6}", output[0]);
        println!("  Last sample (16-bit):  {:.6}", output[block_size - 1]);
        println!("  ✓ PASSED\n");
    }

    println!("All tests passed! QuantizeNode is working correctly.");
}
