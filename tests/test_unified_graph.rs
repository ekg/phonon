use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{NodeId, Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform};

#[test]
fn test_basic_oscillator() {
    println!("\n=== Testing Basic Oscillator in Unified Graph ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a sine oscillator at 440Hz
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Create output node
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(osc),
    });

    graph.set_output(output);

    // Render a short buffer
    let buffer = graph.render(100);

    // Verify we get non-zero output
    let has_signal = buffer.iter().any(|&s| s != 0.0);
    assert!(has_signal, "Oscillator should produce output");

    // Verify it's oscillating (has both positive and negative values)
    let has_positive = buffer.iter().any(|&s| s > 0.0);
    let has_negative = buffer.iter().any(|&s| s < 0.0);
    assert!(has_positive && has_negative, "Sine wave should oscillate");

    println!("✓ Oscillator produces expected sine wave");
}

#[test]
fn test_pattern_as_signal() {
    println!("\n=== Testing Pattern as Signal Source ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0); // 1 cycle per second

    // Create a pattern node
    let pattern = parse_mini_notation("1 0 1 0");
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 1 0".to_string(),
        pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Use pattern to modulate oscillator amplitude
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    let modulated = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Node(pattern_node),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(modulated),
    });

    graph.set_output(output);

    // Render half a second (should have pattern changes)
    let buffer = graph.render(22050);

    // Check that we have both loud and quiet sections
    let max_amp = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let min_amp = buffer.iter().map(|s| s.abs()).fold(1.0f32, f32::min);

    assert!(max_amp > 0.5, "Should have loud sections");
    assert!(min_amp < 0.1, "Should have quiet sections");

    println!("✓ Pattern successfully modulates signal");
}

#[test]
fn test_bus_system() {
    println!("\n=== Testing Bus System with References ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create an LFO on a bus
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(2.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });
    graph.add_bus("lfo".to_string(), lfo);

    // Create oscillator modulated by the LFO via bus reference
    let base_freq = graph.add_node(SignalNode::Constant { value: 440.0 });
    let mod_amount = graph.add_node(SignalNode::Multiply {
        a: Signal::Bus("lfo".to_string()),
        b: Signal::Value(50.0),
    });
    let modulated_freq = graph.add_node(SignalNode::Add {
        a: Signal::Node(base_freq),
        b: Signal::Node(mod_amount),
    });

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(modulated_freq),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(osc),
    });

    graph.set_output(output);

    // Render and verify
    let buffer = graph.render(1000);
    assert!(buffer.iter().any(|&s| s != 0.0), "Should produce output");

    println!("✓ Bus system works with signal references");
}

#[test]
fn test_filter_chain() {
    println!("\n=== Testing Filter Chain ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create noise source
    let noise = graph.add_node(SignalNode::Noise { seed: 12345 });

    // Lowpass filter
    let lpf = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(noise),
        cutoff: Signal::Value(1000.0),
        q: Signal::Value(1.0),
        state: Default::default(),
    });

    // Highpass filter
    let hpf = graph.add_node(SignalNode::HighPass {
        input: Signal::Node(lpf),
        cutoff: Signal::Value(500.0),
        q: Signal::Value(1.0),
        state: Default::default(),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(hpf),
    });

    graph.set_output(output);

    // Render and check
    let buffer = graph.render(1000);

    // Filtered noise should have less variance than raw noise
    let variance: f32 = buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32;
    assert!(variance > 0.0, "Should have output");
    assert!(variance < 0.5, "Filtered noise should have lower variance");

    println!("✓ Filter chain processes correctly");
}

#[test]
fn test_envelope_generator() {
    println!("\n=== Testing Envelope Generator ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Create trigger pattern
    let trigger_pattern = parse_mini_notation("1 ~ ~ ~");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 ~ ~ ~".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Create oscillator
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Apply envelope
    let env = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(osc),
        trigger: Signal::Node(trigger),
        attack: 0.01,
        decay: 0.1,
        sustain: 0.5,
        release: 0.2,
        state: Default::default(),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(env),
    });

    graph.set_output(output);

    // Render one second (should have 2 enveloped triggers)
    let buffer = graph.render(44100);

    // Find peak amplitudes
    let mut peaks = Vec::new();
    let window_size = 2205; // 50ms windows
    for i in (0..buffer.len()).step_by(window_size) {
        let end = (i + window_size).min(buffer.len());
        let peak = buffer[i..end]
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        peaks.push(peak);
    }

    // Should have at least 2 distinct peaks (from the triggers)
    let high_peaks = peaks.iter().filter(|&&p| p > 0.3).count();
    assert!(high_peaks >= 2, "Should have multiple envelope triggers");

    println!("✓ Envelope generator works with pattern triggers");
}

#[test]
fn test_signal_expressions() {
    println!("\n=== Testing Signal Expressions ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create complex expression: (sine 440 + sine 550) * 0.5
    let osc1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    let osc2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(550.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Use expression for mixing
    let mixed = Signal::Expression(Box::new(SignalExpr::Multiply(
        Signal::Expression(Box::new(SignalExpr::Add(
            Signal::Node(osc1),
            Signal::Node(osc2),
        ))),
        Signal::Value(0.5),
    )));

    let output = graph.add_node(SignalNode::Output { input: mixed });
    graph.set_output(output);

    // Render and verify
    let buffer = graph.render(1000);

    // Should produce output
    assert!(
        buffer.iter().any(|&s| s != 0.0),
        "Expression should produce output"
    );

    // Check amplitude is reasonable (scaled by 0.5)
    let max_amp = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_amp < 1.5,
        "Mixed and scaled signal should be reasonable amplitude"
    );

    println!("✓ Signal expressions evaluate correctly");
}

#[test]
fn test_delay_effect() {
    println!("\n=== Testing Delay Effect ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a short impulse at the beginning
    let impulse_osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Gate it with a short pattern to create impulse
    let gate_pattern = parse_mini_notation("1 ~ ~ ~ ~ ~ ~ ~");
    let gate = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 ~ ~ ~ ~ ~ ~ ~".to_string(),
        pattern: gate_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    let impulse = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(impulse_osc),
        b: Signal::Node(gate),
    });

    // Apply delay
    let delayed = graph.add_node(SignalNode::Delay {
        input: Signal::Node(impulse),
        time: Signal::Value(0.25),    // 250ms delay
        feedback: Signal::Value(0.5), // 50% feedback
        mix: Signal::Value(0.5),      // 50% wet
        buffer: vec![0.0; 88200],     // 2 seconds at 44.1kHz
        write_idx: 0,
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(delayed),
    });

    graph.set_output(output);
    graph.set_cps(4.0); // Faster to get impulse sooner

    // Render 2 seconds
    let buffer = graph.render(88200);

    // Count peaks (should have original + echoes)
    let mut peaks = 0;
    let threshold = 0.01; // Lower threshold for impulse patterns
    let mut was_below = true;

    // Find max value for debugging
    let max_val = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max value in buffer: {}", max_val);

    for (i, &sample) in buffer.iter().enumerate() {
        if sample.abs() > threshold && was_below {
            peaks += 1;
            println!("Peak {} at sample {} with value {}", peaks, i, sample);
            was_below = false;
        } else if sample.abs() < threshold * 0.5 {
            was_below = true;
        }
    }

    println!("Found {} peaks", peaks);
    // Should have multiple peaks due to delay feedback
    assert!(
        peaks > 1,
        "Delay should create echoes: found {} peaks",
        peaks
    );

    println!("✓ Delay effect creates echoes with feedback");
}

#[test]
fn test_audio_analysis_nodes() {
    println!("\n=== Testing Audio Analysis Nodes ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create test signal
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(100.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Add RMS analyzer
    let rms = graph.add_node(SignalNode::RMS {
        input: Signal::Node(osc),
        window_size: Signal::Value(0.01), // 10ms window
        buffer: vec![0.0; 441],           // 10ms at 44.1kHz
        write_idx: 0,
    });

    // Add transient detector
    let transient = graph.add_node(SignalNode::Transient {
        input: Signal::Node(osc),
        threshold: 0.5,
        last_value: 0.0,
    });

    // Output the RMS value
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(rms),
    });

    graph.set_output(output);

    // Render and check RMS values
    let buffer = graph.render(4410); // 100ms

    // RMS of sine wave should stabilize around 0.707
    let last_samples = &buffer[buffer.len() - 100..];
    let avg_rms = last_samples.iter().sum::<f32>() / last_samples.len() as f32;

    assert!(
        (avg_rms - 0.707).abs() < 0.1,
        "RMS should be close to 0.707 for sine wave"
    );

    println!("✓ Audio analysis nodes work correctly");
}

#[test]
fn test_conditional_processing() {
    println!("\n=== Testing Conditional Processing with When ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    // Gate pattern
    let gate = parse_mini_notation("1 0 1 0");
    let gate_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 1 0".to_string(),
        pattern: gate,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Signal to gate
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Conditional processing
    let gated = graph.add_node(SignalNode::When {
        input: Signal::Node(osc),
        condition: Signal::Node(gate_node),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(gated),
    });

    graph.set_output(output);

    // Render one second
    let buffer = graph.render(44100);

    // Should have alternating loud/quiet sections
    let quarter = buffer.len() / 4;

    // First quarter (gate = 1) should have signal
    let first_max = buffer[0..quarter]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(first_max > 0.5, "First quarter should have signal");

    // Second quarter (gate = 0) should be silent
    let second_max = buffer[quarter..2 * quarter]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(second_max < 0.1, "Second quarter should be gated");

    println!("✓ Conditional processing with When works");
}

#[test]
fn test_pattern_driven_synthesis() {
    println!("\n=== Testing Pattern-Driven Synthesis ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Frequency pattern
    let freq_pattern = parse_mini_notation("220 330 440 550");
    let freq_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "220 330 440 550".to_string(),
        pattern: freq_pattern,
        last_value: 220.0,
        last_trigger_time: -1.0,
    });

    // Oscillator controlled by pattern
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(freq_node),
        waveform: Waveform::Saw,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
    });

    // Filter cutoff pattern
    let cutoff_pattern = parse_mini_notation("1000 2000 500 3000");
    let cutoff_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1000 2000 500 3000".to_string(),
        pattern: cutoff_pattern,
        last_value: 1000.0,
        last_trigger_time: -1.0,
    });

    // Filter with pattern-controlled cutoff
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc),
        cutoff: Signal::Node(cutoff_node),
        q: Signal::Value(2.0),
        state: Default::default(),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(filtered),
    });

    graph.set_output(output);

    // Render 2 seconds (4 complete cycles)
    let buffer = graph.render(88200);

    // Verify we have output
    assert!(buffer.iter().any(|&s| s != 0.0), "Should produce output");

    // Check for variation (different frequencies and filters)
    let quarters = buffer.len() / 8;
    let mut section_powers = Vec::new();

    for i in 0..8 {
        let start = i * quarters;
        let end = start + quarters;
        let power: f32 = buffer[start..end].iter().map(|s| s * s).sum::<f32>() / quarters as f32;
        section_powers.push(power);
    }

    // Should have variation between sections
    let max_power = section_powers.iter().cloned().fold(0.0f32, f32::max);
    let min_power = section_powers.iter().cloned().fold(1.0f32, f32::min);
    assert!(
        (max_power - min_power) > 0.001,
        "Should have variation from patterns"
    );

    println!("✓ Pattern-driven synthesis works correctly");
}
