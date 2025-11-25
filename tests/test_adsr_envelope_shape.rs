//! ADSR Envelope Shape Verification Tests
//!
//! These tests verify that ADSR parameters actually shape the audio signal correctly
//! by analyzing the amplitude envelope over time.

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{EnvState, Signal, SignalNode, UnifiedSignalGraph};

/// Find the peak amplitude in a buffer segment
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0, f32::max)
}

/// Calculate RMS (root mean square) of a buffer segment
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

#[test]
fn test_adsr_attack_phase() {
    // Test that attack parameter controls ramp-up time
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Create a long trigger (stays high)
    let trigger = graph.add_node(SignalNode::Constant { value: 1.0 });

    // Constant audio source
    let source = graph.add_node(SignalNode::Constant { value: 1.0 });

    // ADSR with 0.1s attack, instant decay, full sustain
    let envelope = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(source),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.1),  // 100ms attack
        decay: Signal::Value(0.0),   // Instant decay
        sustain: Signal::Value(1.0), // Full sustain
        release: Signal::Value(0.0), // Instant release
        state: EnvState::default(),
    });

    graph.set_output(envelope);

    // Render 0.2 seconds
    let buffer = graph.render((sample_rate * 0.2) as usize);

    // Check attack phase
    // At 25ms (quarter way through attack), amplitude should be ~0.25
    let t_25ms = (sample_rate * 0.025) as usize;
    let amp_25ms = calculate_rms(&buffer[t_25ms..t_25ms + 100]);
    println!("Amplitude at 25ms (should be ~0.25): {}", amp_25ms);
    assert!(
        amp_25ms > 0.15 && amp_25ms < 0.35,
        "At 25ms into 100ms attack, expected ~0.25, got {}",
        amp_25ms
    );

    // At 50ms (halfway through attack), amplitude should be ~0.5
    let t_50ms = (sample_rate * 0.05) as usize;
    let amp_50ms = calculate_rms(&buffer[t_50ms..t_50ms + 100]);
    println!("Amplitude at 50ms (should be ~0.5): {}", amp_50ms);
    assert!(
        amp_50ms > 0.4 && amp_50ms < 0.6,
        "At 50ms into 100ms attack, expected ~0.5, got {}",
        amp_50ms
    );

    // At 75ms (3/4 way through attack), amplitude should be ~0.75
    let t_75ms = (sample_rate * 0.075) as usize;
    let amp_75ms = calculate_rms(&buffer[t_75ms..t_75ms + 100]);
    println!("Amplitude at 75ms (should be ~0.75): {}", amp_75ms);
    assert!(
        amp_75ms > 0.65 && amp_75ms < 0.85,
        "At 75ms into 100ms attack, expected ~0.75, got {}",
        amp_75ms
    );

    // After attack completes (at 150ms), should be at full level
    let t_150ms = (sample_rate * 0.15) as usize;
    let amp_150ms = calculate_rms(&buffer[t_150ms..t_150ms + 100]);
    println!("Amplitude at 150ms (should be ~1.0): {}", amp_150ms);
    assert!(
        amp_150ms > 0.9,
        "After 100ms attack, expected ~1.0, got {}",
        amp_150ms
    );
}

#[test]
fn test_adsr_decay_and_sustain() {
    // Test that decay drops to sustain level
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Long trigger
    let trigger = graph.add_node(SignalNode::Constant { value: 1.0 });
    let source = graph.add_node(SignalNode::Constant { value: 1.0 });

    // ADSR: instant attack, 0.1s decay, 0.5 sustain level
    let envelope = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(source),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.001), // 1ms attack (essentially instant)
        decay: Signal::Value(0.1),    // 100ms decay
        sustain: Signal::Value(0.5),  // 50% sustain level
        release: Signal::Value(0.0),
        state: EnvState::default(),
    });

    graph.set_output(envelope);

    // Render 0.3 seconds
    let buffer = graph.render((sample_rate * 0.3) as usize);

    // Right after attack (at 5ms), should be at peak (~1.0)
    let t_5ms = (sample_rate * 0.005) as usize;
    let amp_5ms = calculate_rms(&buffer[t_5ms..t_5ms + 100]);
    println!("Amplitude at 5ms (should be ~1.0): {}", amp_5ms);
    assert!(
        amp_5ms > 0.9,
        "After attack, expected ~1.0, got {}",
        amp_5ms
    );

    // Halfway through decay (at 55ms), should be ~0.75 (halfway between 1.0 and 0.5)
    let t_55ms = (sample_rate * 0.055) as usize;
    let amp_55ms = calculate_rms(&buffer[t_55ms..t_55ms + 100]);
    println!("Amplitude at 55ms (should be ~0.75): {}", amp_55ms);
    assert!(
        amp_55ms > 0.65 && amp_55ms < 0.85,
        "Halfway through decay, expected ~0.75, got {}",
        amp_55ms
    );

    // After decay (at 200ms), should be at sustain level (0.5)
    let t_200ms = (sample_rate * 0.2) as usize;
    let amp_200ms = calculate_rms(&buffer[t_200ms..t_200ms + 100]);
    println!("Amplitude at 200ms (should be ~0.5): {}", amp_200ms);
    assert!(
        amp_200ms > 0.45 && amp_200ms < 0.55,
        "At sustain phase, expected ~0.5, got {}",
        amp_200ms
    );
}

#[test]
fn test_adsr_release_phase() {
    // Test that release phase decays after trigger ends
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Trigger: high for 0.25s, then low for rest of test
    // Using <1 0> at 2cps means: cycle 0 = 1 (0.5s), cycle 1 = 0 (0.5s), etc.
    let trigger_pattern = parse_mini_notation("<1 0>");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "<1 0>".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    let source = graph.add_node(SignalNode::Constant { value: 1.0 });

    // ADSR: instant attack, no decay, full sustain, 0.1s release
    let envelope = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(source),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.001),
        decay: Signal::Value(0.0),
        sustain: Signal::Value(1.0),
        release: Signal::Value(0.1), // 100ms release
        state: EnvState::default(),
    });

    graph.set_output(envelope);
    graph.set_cps(2.0); // 2 cycles/sec, so each cycle is 0.5s

    // Render 1 second
    let buffer = graph.render(sample_rate as usize);

    // During trigger (at 0.1s), should be at full level
    let t_100ms = (sample_rate * 0.1) as usize;
    let amp_100ms = calculate_rms(&buffer[t_100ms..t_100ms + 100]);
    println!(
        "Amplitude at 0.1s (during trigger, should be ~1.0): {}",
        amp_100ms
    );
    assert!(
        amp_100ms > 0.9,
        "During trigger, expected ~1.0, got {}",
        amp_100ms
    );

    // Trigger ends at 0.5s, release begins
    // At 0.55s (50ms into release), should be ~0.5
    let t_550ms = (sample_rate * 0.55) as usize;
    let amp_550ms = calculate_rms(&buffer[t_550ms..t_550ms + 100]);
    println!(
        "Amplitude at 0.55s (50ms into release, should be ~0.5): {}",
        amp_550ms
    );
    assert!(
        amp_550ms > 0.4 && amp_550ms < 0.6,
        "50ms into release, expected ~0.5, got {}",
        amp_550ms
    );

    // After release completes (at 0.65s), should be near zero
    let t_650ms = (sample_rate * 0.65) as usize;
    let amp_650ms = calculate_rms(&buffer[t_650ms..t_650ms + 100]);
    println!(
        "Amplitude at 0.65s (after release, should be ~0.0): {}",
        amp_650ms
    );
    assert!(
        amp_650ms < 0.1,
        "After release, expected ~0.0, got {}",
        amp_650ms
    );
}

#[test]
fn test_adsr_percussive_envelope() {
    // Test a percussive envelope (no sustain)
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // Trigger pattern: 4 hits
    let trigger_pattern = parse_mini_notation("1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    let source = graph.add_node(SignalNode::Constant { value: 1.0 });

    // Percussive ADSR: fast attack, medium decay, no sustain, medium release
    let envelope = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(source),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.005), // 5ms attack
        decay: Signal::Value(0.05),   // 50ms decay
        sustain: Signal::Value(0.0),  // No sustain (percussive)
        release: Signal::Value(0.05), // 50ms release
        state: EnvState::default(),
    });

    graph.set_output(envelope);
    graph.set_cps(2.0); // 2 cycles/sec

    // Render 2 seconds
    let buffer = graph.render((sample_rate * 2.0) as usize);

    // Find the 4 transients - each should have distinct envelope shape
    let chunk_size = (sample_rate * 0.5) as usize; // 0.5s per trigger

    for i in 0..4 {
        let start = i * chunk_size;
        let chunk = &buffer[start..start + chunk_size];

        // Find peak in this chunk
        let peak = find_peak(chunk);
        println!("Chunk {} peak: {}", i, peak);

        // Each hit should reach reasonable amplitude
        assert!(peak > 0.5, "Chunk {} peak should be >0.5, got {}", i, peak);

        // Verify envelope decays within the chunk (not sustaining)
        let early_rms = calculate_rms(&chunk[0..1000]);
        let late_rms = calculate_rms(&chunk[chunk_size - 1000..chunk_size]);
        println!(
            "Chunk {} early RMS: {}, late RMS: {}",
            i, early_rms, late_rms
        );

        assert!(
            late_rms < early_rms * 0.5,
            "Chunk {} should decay, early={} late={}",
            i,
            early_rms,
            late_rms
        );
    }
}

#[test]
fn test_adsr_with_varying_sustain_levels() {
    // Test different sustain levels produce different sustain amplitudes
    let sample_rate = 44100.0;

    for (sustain_level, label) in [(0.25, "25%"), (0.5, "50%"), (0.75, "75%")] {
        let mut graph = UnifiedSignalGraph::new(sample_rate);

        let trigger = graph.add_node(SignalNode::Constant { value: 1.0 });
        let source = graph.add_node(SignalNode::Constant { value: 1.0 });

        let envelope = graph.add_node(SignalNode::Envelope {
            input: Signal::Node(source),
            trigger: Signal::Node(trigger),
            attack: Signal::Value(0.01),
            decay: Signal::Value(0.05),
            sustain: Signal::Value(sustain_level),
            release: Signal::Value(0.05),
            state: EnvState::default(),
        });

        graph.set_output(envelope);

        // Render 0.2 seconds to reach sustain phase
        let buffer = graph.render((sample_rate * 0.2) as usize);

        // Check sustain level at 150ms (should be in sustain phase)
        let t_150ms = (sample_rate * 0.15) as usize;
        let amp_150ms = calculate_rms(&buffer[t_150ms..t_150ms + 1000]);

        println!(
            "Sustain level {} - measured amplitude: {}",
            label, amp_150ms
        );

        let tolerance = 0.15;
        assert!(
            (amp_150ms - sustain_level).abs() < tolerance,
            "Sustain level {}: expected {}, got {} (tolerance {})",
            label,
            sustain_level,
            amp_150ms,
            tolerance
        );
    }
}
