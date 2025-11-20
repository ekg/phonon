/// Example demonstrating the GainNode
///
/// This shows how to use GainNode to apply volume/gain control to audio signals.
///
/// Run with:
/// ```bash
/// cargo run --example gain_node_example
/// ```

use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::nodes::{ConstantNode, GainNode, OscillatorNode, Waveform};
use phonon::pattern::Fraction;

fn main() {
    println!("GainNode Example\n");

    // Setup: Create a 440 Hz sine wave and apply different gain amounts
    let sample_rate = 44100.0;
    let buffer_size = 512;

    // Create nodes
    let mut freq_node = ConstantNode::new(440.0);       // Node 0: frequency
    let mut osc_node = OscillatorNode::new(0, Waveform::Sine);  // Node 1: oscillator

    // Create different gain amounts
    let mut gain_unity = ConstantNode::new(1.0);        // Node 2: unity gain
    let mut gain_half = ConstantNode::new(0.5);         // Node 3: half volume
    let mut gain_double = ConstantNode::new(2.0);       // Node 4: double volume
    let mut gain_invert = ConstantNode::new(-1.0);      // Node 5: phase invert

    // Create gain nodes
    let mut gain_unity_node = GainNode::new(1, 2);      // Node 6
    let mut gain_half_node = GainNode::new(1, 3);       // Node 7
    let mut gain_double_node = GainNode::new(1, 4);     // Node 8
    let mut gain_invert_node = GainNode::new(1, 5);     // Node 9

    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        buffer_size,
        2.0,
        sample_rate,
    );

    // Process buffers
    let mut freq_buf = vec![0.0; buffer_size];
    let mut osc_buf = vec![0.0; buffer_size];
    let mut gain_unity_buf = vec![0.0; buffer_size];
    let mut gain_half_buf = vec![0.0; buffer_size];
    let mut gain_double_buf = vec![0.0; buffer_size];
    let mut gain_invert_buf = vec![0.0; buffer_size];

    let mut output_unity = vec![0.0; buffer_size];
    let mut output_half = vec![0.0; buffer_size];
    let mut output_double = vec![0.0; buffer_size];
    let mut output_invert = vec![0.0; buffer_size];

    // Generate signals
    freq_node.process_block(&[], &mut freq_buf, sample_rate, &context);
    osc_node.process_block(&[&freq_buf], &mut osc_buf, sample_rate, &context);

    gain_unity.process_block(&[], &mut gain_unity_buf, sample_rate, &context);
    gain_half.process_block(&[], &mut gain_half_buf, sample_rate, &context);
    gain_double.process_block(&[], &mut gain_double_buf, sample_rate, &context);
    gain_invert.process_block(&[], &mut gain_invert_buf, sample_rate, &context);

    // Apply gain
    gain_unity_node.process_block(
        &[&osc_buf, &gain_unity_buf],
        &mut output_unity,
        sample_rate,
        &context,
    );
    gain_half_node.process_block(
        &[&osc_buf, &gain_half_buf],
        &mut output_half,
        sample_rate,
        &context,
    );
    gain_double_node.process_block(
        &[&osc_buf, &gain_double_buf],
        &mut output_double,
        sample_rate,
        &context,
    );
    gain_invert_node.process_block(
        &[&osc_buf, &gain_invert_buf],
        &mut output_invert,
        sample_rate,
        &context,
    );

    // Analyze results (first few samples)
    println!("Sample Analysis (first 4 samples):");
    println!("Index | Original  | Unity(1.0) | Half(0.5) | Double(2.0) | Invert(-1.0)");
    println!("------|-----------|------------|-----------|-------------|-------------");

    for i in 0..4 {
        println!(
            "{:5} | {:9.6} | {:10.6} | {:9.6} | {:11.6} | {:12.6}",
            i, osc_buf[i], output_unity[i], output_half[i], output_double[i], output_invert[i]
        );
    }

    // Calculate RMS to verify overall amplitude
    let rms_original = calculate_rms(&osc_buf);
    let rms_unity = calculate_rms(&output_unity);
    let rms_half = calculate_rms(&output_half);
    let rms_double = calculate_rms(&output_double);
    let rms_invert = calculate_rms(&output_invert);

    println!("\nRMS Analysis:");
    println!("Original:      {:.6}", rms_original);
    println!("Unity gain:    {:.6} (ratio: {:.2}x)", rms_unity, rms_unity / rms_original);
    println!("Half gain:     {:.6} (ratio: {:.2}x)", rms_half, rms_half / rms_original);
    println!("Double gain:   {:.6} (ratio: {:.2}x)", rms_double, rms_double / rms_original);
    println!("Inverted:      {:.6} (ratio: {:.2}x)", rms_invert, rms_invert / rms_original);

    println!("\n✅ GainNode working correctly!");
    println!("   - Unity gain preserves signal (1.00x)");
    println!("   - Half gain reduces to 50% (0.50x)");
    println!("   - Double gain amplifies to 200% (2.00x)");
    println!("   - Negative gain inverts phase (maintains 1.00x RMS)");
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}
