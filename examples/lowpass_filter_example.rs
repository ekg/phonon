/// Example: Using LowPassFilterNode in the DAW buffer passing architecture
///
/// This demonstrates how to create a signal graph with oscillator → lowpass filter
///
/// Run with: cargo run --example lowpass_filter_example

use phonon::nodes::{ConstantNode, OscillatorNode, LowPassFilterNode, Waveform};
use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::pattern::Fraction;

fn main() {
    println!("LowPassFilter Node Example");
    println!("==========================\n");

    // Create nodes
    // Node 0: Constant frequency (440 Hz)
    let mut freq_node = ConstantNode::new(440.0);

    // Node 1: Sawtooth oscillator (input: node 0)
    let mut osc_node = OscillatorNode::new(0, Waveform::Saw);

    // Node 2: Constant cutoff frequency (1000 Hz)
    let mut cutoff_node = ConstantNode::new(1000.0);

    // Node 3: Constant Q (Butterworth = 0.707)
    let mut q_node = ConstantNode::new(biquad::Q_BUTTERWORTH_F32);

    // Node 4: Lowpass filter (inputs: signal=1, cutoff=2, q=3)
    let mut lpf_node = LowPassFilterNode::new(1, 2, 3);

    println!("Signal Graph:");
    println!("  Node 0: ConstantNode(440.0 Hz)");
    println!("  Node 1: OscillatorNode(Saw, freq=Node0)");
    println!("  Node 2: ConstantNode(1000.0 Hz)");
    println!("  Node 3: ConstantNode(Q=0.707)");
    println!("  Node 4: LowPassFilterNode(signal=Node1, cutoff=Node2, q=Node3)");
    println!();

    // Create processing context
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        512,
        2.0,      // 2 cycles per second
        44100.0,  // 44.1kHz sample rate
    );

    // Allocate buffers
    let mut freq_buffer = vec![0.0; 512];
    let mut osc_buffer = vec![0.0; 512];
    let mut cutoff_buffer = vec![0.0; 512];
    let mut q_buffer = vec![0.0; 512];
    let mut output_buffer = vec![0.0; 512];

    // Process the graph (in dependency order)
    println!("Processing 512-sample block...\n");

    // Step 1: Generate frequency
    freq_node.process_block(&[], &mut freq_buffer, 44100.0, &context);

    // Step 2: Generate oscillator (depends on freq)
    let osc_inputs = vec![freq_buffer.as_slice()];
    osc_node.process_block(&osc_inputs, &mut osc_buffer, 44100.0, &context);

    // Step 3: Generate cutoff
    cutoff_node.process_block(&[], &mut cutoff_buffer, 44100.0, &context);

    // Step 4: Generate Q
    q_node.process_block(&[], &mut q_buffer, 44100.0, &context);

    // Step 5: Apply lowpass filter
    let lpf_inputs = vec![
        osc_buffer.as_slice(),
        cutoff_buffer.as_slice(),
        q_buffer.as_slice(),
    ];
    lpf_node.process_block(&lpf_inputs, &mut output_buffer, 44100.0, &context);

    // Calculate statistics
    let calculate_rms = |buffer: &[f32]| -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    };

    let input_rms = calculate_rms(&osc_buffer);
    let output_rms = calculate_rms(&output_buffer);

    println!("Results:");
    println!("  Input RMS:   {:.6}", input_rms);
    println!("  Output RMS:  {:.6}", output_rms);
    println!("  Attenuation: {:.2}%", (1.0 - output_rms / input_rms) * 100.0);
    println!();
    println!("  Filter State:");
    println!("    Cutoff: {:.1} Hz", lpf_node.cutoff());
    println!("    Q:      {:.3}", lpf_node.q());
    println!();

    // Show first few samples
    println!("First 10 samples:");
    println!("  Index | Input      | Output     | Difference");
    println!("  ------|------------|------------|------------");
    for i in 0..10 {
        println!(
            "  {:5} | {:10.6} | {:10.6} | {:10.6}",
            i,
            osc_buffer[i],
            output_buffer[i],
            osc_buffer[i] - output_buffer[i]
        );
    }
    println!();
    println!("✓ LowPassFilter successfully processed 512 samples!");
}
