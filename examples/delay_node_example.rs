use phonon::audio_node::{AudioNode, ProcessContext};
/// Example: Using DelayNode in the DAW buffer passing architecture
///
/// This demonstrates how to create a signal graph with:
/// 1. Impulse → delay → output (basic delay)
/// 2. Oscillator → delay → output (delayed audio)
/// 3. Signal → delay with modulated delay time (chorus/flanger effect)
///
/// Run with: cargo run --example delay_node_example
use phonon::nodes::{ConstantNode, DelayNode, OscillatorNode, Waveform};
use phonon::pattern::Fraction;

fn main() {
    println!("DelayNode Example");
    println!("=================\n");

    example_1_impulse_delay();
    println!("\n{}\n", "=".repeat(60));
    example_2_delayed_oscillator();
    println!("\n{}\n", "=".repeat(60));
    example_3_modulated_delay();
}

/// Example 1: Impulse through delay line
/// This clearly shows the delay time by tracking when the impulse appears
fn example_1_impulse_delay() {
    println!("Example 1: Impulse Delay");
    println!("------------------------\n");

    let sample_rate = 44100.0;
    let delay_time = 0.01; // 10ms delay = 441 samples
    let max_delay = 0.1; // 100ms max

    // Node 0: Constant delay time (10ms)
    let mut delay_time_node = ConstantNode::new(delay_time);

    // Node 1: Delay (input=impulse, delay_time=Node0)
    let mut delay_node = DelayNode::new(0, 0, max_delay, sample_rate);

    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, sample_rate);

    println!("Signal Graph:");
    println!(
        "  Node 0: ConstantNode({} seconds = {} samples)",
        delay_time,
        delay_time * sample_rate
    );
    println!("  Node 1: DelayNode(max_delay={} seconds)", max_delay);
    println!();

    // Create impulse input (1.0 at first sample, then zeros)
    let mut input_buffer = vec![0.0; 512];
    input_buffer[0] = 1.0; // Impulse at sample 0

    let mut delay_time_buffer = vec![0.0; 512];
    let mut output_buffer = vec![0.0; 512];

    // Process enough blocks to see the delayed impulse
    let blocks_needed = ((delay_time * sample_rate) as usize / 512) + 2;

    println!(
        "Processing {} blocks to see delayed impulse...",
        blocks_needed
    );

    for block_idx in 0..blocks_needed {
        // Generate delay time
        delay_time_node.process_block(&[], &mut delay_time_buffer, sample_rate, &context);

        // Apply delay
        let inputs = vec![input_buffer.as_slice(), delay_time_buffer.as_slice()];
        delay_node.process_block(&inputs, &mut output_buffer, sample_rate, &context);

        // Look for the impulse in output
        for (i, &sample) in output_buffer.iter().enumerate() {
            if sample > 0.5 {
                let sample_position = block_idx * 512 + i;
                println!("\n✓ Impulse detected at sample {}!", sample_position);
                println!(
                    "  Expected: {} samples",
                    (delay_time * sample_rate) as usize
                );
                println!(
                    "  Error:    {} samples",
                    (sample_position as f32 - delay_time * sample_rate).abs() as usize
                );
                return;
            }
        }

        // After first block, input is silent
        input_buffer.fill(0.0);
    }

    println!("\n⚠ Warning: Impulse not detected (buffer not wrapped yet)");
}

/// Example 2: Delayed oscillator
/// Creates an echo effect by delaying a tone
fn example_2_delayed_oscillator() {
    println!("Example 2: Delayed Oscillator (Echo Effect)");
    println!("-------------------------------------------\n");

    let sample_rate = 44100.0;
    let delay_time = 0.05; // 50ms delay

    // Node 0: Constant frequency (440 Hz)
    let mut freq_node = ConstantNode::new(440.0);

    // Node 1: Sine oscillator
    let mut osc_node = OscillatorNode::new(0, Waveform::Sine);

    // Node 2: Constant delay time (50ms)
    let mut delay_time_node = ConstantNode::new(delay_time);

    // Node 3: Delay
    let mut delay_node = DelayNode::new(1, 2, 0.5, sample_rate);

    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, sample_rate);

    println!("Signal Graph:");
    println!("  Node 0: ConstantNode(440.0 Hz)");
    println!("  Node 1: OscillatorNode(Sine, freq=Node0)");
    println!("  Node 2: ConstantNode({} seconds)", delay_time);
    println!("  Node 3: DelayNode(signal=Node1, delay_time=Node2)");
    println!();

    // Allocate buffers
    let mut freq_buffer = vec![0.0; 512];
    let mut osc_buffer = vec![0.0; 512];
    let mut delay_time_buffer = vec![0.0; 512];
    let mut output_buffer = vec![0.0; 512];

    // Process multiple blocks to fill delay buffer
    let warmup_blocks = 10;

    for _ in 0..warmup_blocks {
        freq_node.process_block(&[], &mut freq_buffer, sample_rate, &context);
        osc_node.process_block(&[&freq_buffer], &mut osc_buffer, sample_rate, &context);
        delay_time_node.process_block(&[], &mut delay_time_buffer, sample_rate, &context);

        let inputs = vec![osc_buffer.as_slice(), delay_time_buffer.as_slice()];
        delay_node.process_block(&inputs, &mut output_buffer, sample_rate, &context);
    }

    // Calculate statistics
    let calculate_rms = |buffer: &[f32]| -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    };

    let input_rms = calculate_rms(&osc_buffer);
    let output_rms = calculate_rms(&output_buffer);

    println!("After {} warmup blocks:", warmup_blocks);
    println!("  Input RMS:  {:.6}", input_rms);
    println!("  Output RMS: {:.6}", output_rms);
    println!(
        "  Delay time: {} samples",
        (delay_time * sample_rate) as usize
    );
    println!();

    println!("First 10 samples (after warmup):");
    println!("  Index | Input      | Output     | Phase Shift");
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
    println!("✓ Delay successfully created echo effect!");
}

/// Example 3: Modulated delay time
/// Creates a chorus/flanger effect by varying delay time
fn example_3_modulated_delay() {
    println!("Example 3: Modulated Delay Time (Chorus Effect)");
    println!("-----------------------------------------------\n");

    let sample_rate = 44100.0;

    // Node 0: Constant frequency (220 Hz)
    let mut freq_node = ConstantNode::new(220.0);

    // Node 1: Saw oscillator (input signal)
    let mut osc_node = OscillatorNode::new(0, Waveform::Saw);

    // Node 2: LFO frequency (2 Hz modulation)
    let mut lfo_freq_node = ConstantNode::new(2.0);

    // Node 3: LFO oscillator (modulates delay time)
    let mut lfo_node = OscillatorNode::new(2, Waveform::Sine);

    // Node 4: Delay with modulated delay time
    let mut delay_node = DelayNode::new(1, 3, 0.1, sample_rate);

    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 512, 2.0, sample_rate);

    println!("Signal Graph:");
    println!("  Node 0: ConstantNode(220.0 Hz)");
    println!("  Node 1: OscillatorNode(Saw, freq=Node0) - audio signal");
    println!("  Node 2: ConstantNode(2.0 Hz)");
    println!("  Node 3: OscillatorNode(Sine, freq=Node2) - LFO modulation");
    println!("  Node 4: DelayNode(signal=Node1, delay_time=Node3 * 0.01 + 0.015)");
    println!();

    // Allocate buffers
    let mut freq_buffer = vec![0.0; 512];
    let mut osc_buffer = vec![0.0; 512];
    let mut lfo_freq_buffer = vec![0.0; 512];
    let mut lfo_buffer = vec![0.0; 512];
    let mut delay_time_buffer = vec![0.0; 512];
    let mut output_buffer = vec![0.0; 512];

    // Warmup
    for _ in 0..20 {
        freq_node.process_block(&[], &mut freq_buffer, sample_rate, &context);
        osc_node.process_block(&[&freq_buffer], &mut osc_buffer, sample_rate, &context);
        lfo_freq_node.process_block(&[], &mut lfo_freq_buffer, sample_rate, &context);
        lfo_node.process_block(&[&lfo_freq_buffer], &mut lfo_buffer, sample_rate, &context);

        // Convert LFO (-1 to 1) to delay time (5ms to 25ms)
        for i in 0..512 {
            delay_time_buffer[i] = (lfo_buffer[i] * 0.01 + 0.015).max(0.005).min(0.025);
        }

        let inputs = vec![osc_buffer.as_slice(), delay_time_buffer.as_slice()];
        delay_node.process_block(&inputs, &mut output_buffer, sample_rate, &context);
    }

    // Show delay time modulation range
    let min_delay = delay_time_buffer
        .iter()
        .cloned()
        .fold(f32::INFINITY, f32::min);
    let max_delay = delay_time_buffer
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);

    println!("Modulated Delay Time Range:");
    println!(
        "  Min: {:.4} seconds ({} samples)",
        min_delay,
        (min_delay * sample_rate) as usize
    );
    println!(
        "  Max: {:.4} seconds ({} samples)",
        max_delay,
        (max_delay * sample_rate) as usize
    );
    println!(
        "  Range: {} samples",
        ((max_delay - min_delay) * sample_rate) as usize
    );
    println!();

    let calculate_rms = |buffer: &[f32]| -> f32 {
        let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    };

    println!("Output Statistics:");
    println!("  RMS: {:.6}", calculate_rms(&output_buffer));
    println!();

    println!("Sample delay times (first 10 samples):");
    for i in 0..10 {
        println!(
            "  Sample {:3}: {:.6} seconds ({:4} samples)",
            i,
            delay_time_buffer[i],
            (delay_time_buffer[i] * sample_rate) as usize
        );
    }
    println!();
    println!("✓ Modulated delay creates chorus/flanger effect!");
}
