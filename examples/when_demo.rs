/// WhenNode demonstration - Conditional signal routing examples
///
/// This example demonstrates the WhenNode's ability to route signals
/// conditionally based on various control sources. It showcases:
/// 1. Pattern-based switching between synth patches
/// 2. LFO-based gating/muting
/// 3. Audio-rate conditional processing
/// 4. Musical gate sequencing
use phonon::{
    audio_node::{AudioNode, ProcessContext},
    nodes::{constant::ConstantNode, oscillator::OscillatorNode, when::WhenNode, Waveform},
    pattern::Fraction,
};

fn main() {
    println!("=== WhenNode Demonstration ===\n");

    // Example 1: Pattern-based synth switching
    example_1_pattern_switching();

    // Example 2: LFO-based gating
    example_2_lfo_gating();

    // Example 3: Audio-rate conditional processing (half-wave rectifier)
    example_3_half_wave_rectifier();

    // Example 4: Threshold-based routing
    example_4_threshold_routing();

    println!("\n=== All Examples Completed ===");
}

fn example_1_pattern_switching() {
    println!("Example 1: Pattern-based Synth Switching");
    println!("=========================================");
    println!("Switching between two synth patches based on a control pattern.\n");

    // Setup nodes
    let mut const_pattern = ConstantNode::new(0.0); // Simulates pattern value
    let mut synth_a = ConstantNode::new(110.0); // Bass synth (low freq)
    let mut synth_b = ConstantNode::new(440.0); // Lead synth (high freq)
    let mut when = WhenNode::new(0, 1, 2); // When pattern>0.5: synth_b, else synth_a

    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

    // Process with pattern = 0.0 (should output synth_a)
    let mut buf_pattern = vec![0.0; 8];
    let mut buf_a = vec![110.0; 8];
    let mut buf_b = vec![440.0; 8];
    let mut output = vec![0.0; 8];

    const_pattern.process_block(&[], &mut buf_pattern, 44100.0, &context);
    synth_a.process_block(&[], &mut buf_a, 44100.0, &context);
    synth_b.process_block(&[], &mut buf_b, 44100.0, &context);

    let inputs = vec![buf_pattern.as_slice(), buf_b.as_slice(), buf_a.as_slice()];
    when.process_block(&inputs, &mut output, 44100.0, &context);

    println!("  Pattern = 0.0 (condition LOW):");
    println!("    Output: {} Hz (Synth A - Bass)", output[0]);
    assert_eq!(output[0], 110.0);

    // Process with pattern = 1.0 (should output synth_b)
    buf_pattern.fill(1.0);
    const_pattern = ConstantNode::new(1.0);
    const_pattern.process_block(&[], &mut buf_pattern, 44100.0, &context);

    let inputs = vec![buf_pattern.as_slice(), buf_b.as_slice(), buf_a.as_slice()];
    when.process_block(&inputs, &mut output, 44100.0, &context);

    println!("  Pattern = 1.0 (condition HIGH):");
    println!("    Output: {} Hz (Synth B - Lead)\n", output[0]);
    assert_eq!(output[0], 440.0);

    println!("✓ Pattern switching works correctly!\n");
}

fn example_2_lfo_gating() {
    println!("Example 2: LFO-based Gating");
    println!("===========================");
    println!("Using an LFO to gate/mute a signal rhythmically.\n");

    // Setup nodes
    let _lfo = OscillatorNode::new(0, Waveform::Square); // Square wave LFO
    let mut signal = ConstantNode::new(1.0); // Audio signal
    let mut silence = ConstantNode::new(0.0); // Silence
    let mut when = WhenNode::new(0, 1, 2); // When LFO>0.5: signal, else silence

    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

    // Simulate LFO output (alternating high/low)
    let buf_lfo = vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    let mut buf_signal = vec![1.0; 8];
    let mut buf_silence = vec![0.0; 8];
    let mut output = vec![0.0; 8];

    signal.process_block(&[], &mut buf_signal, 44100.0, &context);
    silence.process_block(&[], &mut buf_silence, 44100.0, &context);

    let inputs = vec![
        buf_lfo.as_slice(),
        buf_signal.as_slice(),
        buf_silence.as_slice(),
    ];
    when.process_block(&inputs, &mut output, 44100.0, &context);

    println!("  LFO pattern: [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]");
    println!(
        "  Output:      [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}]",
        output[0], output[1], output[2], output[3], output[4], output[5], output[6], output[7]
    );

    // First half: signal passes through
    assert_eq!(output[0], 1.0);
    assert_eq!(output[1], 1.0);
    assert_eq!(output[2], 1.0);
    assert_eq!(output[3], 1.0);

    // Second half: muted
    assert_eq!(output[4], 0.0);
    assert_eq!(output[5], 0.0);
    assert_eq!(output[6], 0.0);
    assert_eq!(output[7], 0.0);

    println!("\n✓ LFO gating creates rhythmic on/off effect!\n");
}

fn example_3_half_wave_rectifier() {
    println!("Example 3: Half-Wave Rectifier (Audio-Rate Processing)");
    println!("=======================================================");
    println!("Using WhenNode to pass only positive values of a waveform.\n");

    // Input waveform oscillating between -1 and 1
    let waveform = vec![-1.0, -0.5, 0.0, 0.5, 1.0, 0.5, 0.0, -0.5];

    // Create condition: is waveform positive?
    // In real usage, you'd use GreaterThanNode, but we'll simulate it here
    let is_positive: Vec<f32> = waveform
        .iter()
        .map(|&x| if x > 0.0 { 1.0 } else { 0.0 })
        .collect();

    let zero = vec![0.0; 8];

    // Setup when node
    let mut when = WhenNode::new(0, 1, 2);
    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

    let inputs = vec![is_positive.as_slice(), waveform.as_slice(), zero.as_slice()];
    let mut output = vec![0.0; 8];

    when.process_block(&inputs, &mut output, 44100.0, &context);

    println!(
        "  Input:  [{:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}]",
        waveform[0],
        waveform[1],
        waveform[2],
        waveform[3],
        waveform[4],
        waveform[5],
        waveform[6],
        waveform[7]
    );
    println!(
        "  Output: [{:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}, {:5.1}]",
        output[0], output[1], output[2], output[3], output[4], output[5], output[6], output[7]
    );

    // Verify positive values pass through
    assert_eq!(output[3], 0.5);
    assert_eq!(output[4], 1.0);
    assert_eq!(output[5], 0.5);

    // Verify negative values become zero
    assert_eq!(output[0], 0.0);
    assert_eq!(output[1], 0.0);
    assert_eq!(output[7], 0.0);

    println!("\n✓ Half-wave rectification working! (negative values zeroed)\n");
}

fn example_4_threshold_routing() {
    println!("Example 4: Custom Threshold Routing");
    println!("====================================");
    println!("Using custom threshold to route signals based on amplitude.\n");

    // Setup nodes with custom threshold (0.8)
    let mut when = WhenNode::with_threshold(0, 1, 2, 0.8);

    // Amplitude envelope (simulating varying signal level)
    let amplitude = vec![0.5, 0.7, 0.79, 0.8, 0.81, 0.9, 1.0, 0.6];

    let loud = vec![100.0; 8]; // "Loud" signal
    let quiet = vec![10.0; 8]; // "Quiet" signal

    let context = ProcessContext::new(Fraction::from_float(0.0), 0, 8, 2.0, 44100.0);

    let inputs = vec![amplitude.as_slice(), loud.as_slice(), quiet.as_slice()];
    let mut output = vec![0.0; 8];

    when.process_block(&inputs, &mut output, 44100.0, &context);

    println!("  Threshold: 0.8");
    println!(
        "  Amplitude: [{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
        amplitude[0],
        amplitude[1],
        amplitude[2],
        amplitude[3],
        amplitude[4],
        amplitude[5],
        amplitude[6],
        amplitude[7]
    );
    println!(
        "  Output:    [{:5.0}, {:5.0}, {:5.0}, {:5.0}, {:5.0}, {:5.0}, {:5.0}, {:5.0}]",
        output[0], output[1], output[2], output[3], output[4], output[5], output[6], output[7]
    );

    // Values <= 0.8 should route to quiet
    assert_eq!(output[0], 10.0); // 0.5 <= 0.8
    assert_eq!(output[1], 10.0); // 0.7 <= 0.8
    assert_eq!(output[2], 10.0); // 0.79 <= 0.8
    assert_eq!(output[3], 10.0); // 0.8 == 0.8 (not >)

    // Values > 0.8 should route to loud
    assert_eq!(output[4], 100.0); // 0.81 > 0.8
    assert_eq!(output[5], 100.0); // 0.9 > 0.8
    assert_eq!(output[6], 100.0); // 1.0 > 0.8

    println!("\n  Above threshold (>0.8): loud signal (100.0)");
    println!("  Below threshold (≤0.8): quiet signal (10.0)");
    println!("\n✓ Custom threshold routing works correctly!\n");
}
