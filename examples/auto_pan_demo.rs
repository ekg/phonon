/// Auto-Pan Demo - Demonstrates automatic stereo panning
///
/// This example shows how to use AutoPanNode to create sweeping
/// stereo movement with different LFO waveforms.
///
/// Run with: cargo run --example auto_pan_demo
use phonon::audio_node::AudioNode;
use phonon::audio_node::ProcessContext;
use phonon::nodes::{AutoPanNode, AutoPanWaveform, ConstantNode, OscillatorNode, Waveform};
use phonon::pattern::Fraction;

fn main() {
    println!("=== Auto-Pan Demo ===\n");

    let sample_rate = 44100.0;
    let duration = 2.0; // 2 seconds
    let buffer_size = (sample_rate * duration) as usize;

    // Create context
    let context = ProcessContext::new(
        Fraction::from_float(0.0),
        0,
        buffer_size,
        2.0, // 120 BPM
        sample_rate,
    );

    println!("Demo 1: Sine wave auto-pan (smooth sweeping)");
    demo_autopan(
        sample_rate,
        buffer_size,
        &context,
        AutoPanWaveform::Sine,
        0.5,   // 0.5 Hz panning rate
        1.0,   // Full depth (L-R)
        440.0, // 440 Hz audio
    );

    println!("\nDemo 2: Triangle wave auto-pan (linear movement)");
    demo_autopan(
        sample_rate,
        buffer_size,
        &context,
        AutoPanWaveform::Triangle,
        1.0,   // 1 Hz panning rate
        1.0,   // Full depth
        220.0, // 220 Hz audio
    );

    println!("\nDemo 3: Square wave auto-pan (hard switching)");
    demo_autopan(
        sample_rate,
        buffer_size,
        &context,
        AutoPanWaveform::Square,
        2.0,   // 2 Hz panning rate
        1.0,   // Full depth
        880.0, // 880 Hz audio
    );

    println!("\nDemo 4: Slow sine auto-pan with reduced depth");
    demo_autopan(
        sample_rate,
        buffer_size,
        &context,
        AutoPanWaveform::Sine,
        0.25,  // Very slow (0.25 Hz)
        0.5,   // Half depth (subtle panning)
        330.0, // 330 Hz audio
    );

    println!("\n=== Demo Complete ===");
    println!("AutoPanNode successfully demonstrated!");
    println!("- Sine: Smooth, natural sweeping motion");
    println!("- Triangle: Linear movement with direction changes");
    println!("- Square: Hard left/right switching");
}

fn demo_autopan(
    sample_rate: f32,
    buffer_size: usize,
    context: &ProcessContext,
    waveform: AutoPanWaveform,
    rate: f32,
    depth: f32,
    audio_freq: f32,
) {
    // Create nodes
    let mut freq_node = ConstantNode::new(audio_freq);
    let mut osc_node = OscillatorNode::new(0, Waveform::Saw);
    let mut rate_node = ConstantNode::new(rate);
    let mut depth_node = ConstantNode::new(depth);
    let mut auto_pan_node = AutoPanNode::new(1, 2, 3, waveform);

    // Generate frequency buffer
    let mut freq_buf = vec![0.0; buffer_size];
    freq_node.process_block(&[], &mut freq_buf, sample_rate, context);

    // Generate audio signal
    let freq_inputs = vec![freq_buf.as_slice()];
    let mut audio_buf = vec![0.0; buffer_size];
    osc_node.process_block(&freq_inputs, &mut audio_buf, sample_rate, context);

    // Generate rate and depth buffers
    let mut rate_buf = vec![0.0; buffer_size];
    let mut depth_buf = vec![0.0; buffer_size];
    rate_node.process_block(&[], &mut rate_buf, sample_rate, context);
    depth_node.process_block(&[], &mut depth_buf, sample_rate, context);

    // Apply auto-pan
    let inputs = vec![
        audio_buf.as_slice(),
        rate_buf.as_slice(),
        depth_buf.as_slice(),
    ];
    let mut output = vec![0.0; buffer_size];
    auto_pan_node.process_block(&inputs, &mut output, sample_rate, context);

    // Analyze output
    let rms = calculate_rms(&output);
    let range = calculate_range(&output);
    let min = output.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = output.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    println!("  Waveform: {:?}", waveform);
    println!("  Rate: {} Hz", rate);
    println!("  Depth: {}", depth);
    println!("  Audio Frequency: {} Hz", audio_freq);
    println!("  RMS Level: {:.4}", rms);
    println!(
        "  Output Range: {:.4} (min: {:.4}, max: {:.4})",
        range, min, max
    );
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

fn calculate_range(buffer: &[f32]) -> f32 {
    let min = buffer.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = buffer.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    max - min
}
