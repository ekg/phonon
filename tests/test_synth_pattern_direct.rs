//! Test pattern-triggered synth voices using direct API

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

mod audio_test_utils;
use audio_test_utils::{calculate_rms, find_dominant_frequency};

#[test]
fn test_synth_pattern_direct_api() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Create a simple melody pattern: C4 E4 G4 C5
    let pattern = parse_mini_notation("c4 e4 g4 c5");

    let synth_node = graph.add_node(SignalNode::SynthPattern {
        pattern_str: "c4 e4 g4 c5".to_string(),
        pattern,
        last_trigger_time: -1.0,
        waveform: Waveform::Saw,
        attack: 0.01,
        decay: 0.1,
        sustain: 0.7,
        release: 0.2,
        filter_cutoff: 20000.0,
        filter_resonance: 0.0,
        filter_env_amount: 0.0,
        gain: Signal::Value(0.3),
        pan: Signal::Value(0.0),
    });

    graph.set_output(synth_node);

    // Render 2 cycles (1 second at 2 cps = 44100 samples)
    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    let dominant_freq = find_dominant_frequency(&buffer, 44100.0);

    println!("\n=== SynthPattern Direct API Test ===");
    println!("RMS: {:.4}", rms);
    println!("Dominant frequency: {:.1} Hz", dominant_freq);

    // Should produce audio
    assert!(
        rms > 0.01,
        "Synth pattern should produce audio, got RMS: {}",
        rms
    );

    // Frequency should be in the range of C4-C5 (261-523 Hz)
    assert!(
        dominant_freq > 200.0 && dominant_freq < 600.0,
        "Dominant frequency should be in C4-C5 range, got: {} Hz",
        dominant_freq
    );

    println!("✅ Pattern-triggered synth works!");
}

#[test]
fn test_synth_pattern_polyphony() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(4.0); // 4 cycles per second (fast)

    // Create a chord pattern that triggers multiple notes simultaneously
    let pattern = parse_mini_notation("[c4, e4, g4]");

    let synth_node = graph.add_node(SignalNode::SynthPattern {
        pattern_str: "[c4, e4, g4]".to_string(),
        pattern,
        last_trigger_time: -1.0,
        waveform: Waveform::Sine,
        attack: 0.01,
        decay: 0.1,
        sustain: 0.8,
        release: 0.3,
        filter_cutoff: 20000.0,
        filter_resonance: 0.0,
        filter_env_amount: 0.0,
        gain: Signal::Value(0.2),
        pan: Signal::Value(0.0),
    });

    graph.set_output(synth_node);

    // Render 1 cycle (0.25 seconds at 4 cps = 11025 samples)
    let buffer = graph.render(11025);

    let rms = calculate_rms(&buffer);

    println!("\n=== SynthPattern Polyphony Test ===");
    println!("RMS: {:.4}", rms);

    // Should produce audio from multiple voices
    assert!(
        rms > 0.01,
        "Polyphonic synth should produce audio, got RMS: {}",
        rms
    );

    println!("✅ Polyphonic synth works!");
}

#[test]
fn test_synth_pattern_adsr() {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0); // 1 cycle per second

    // Single note to test ADSR envelope
    let pattern = parse_mini_notation("a4");

    let synth_node = graph.add_node(SignalNode::SynthPattern {
        pattern_str: "a4".to_string(),
        pattern,
        last_trigger_time: -1.0,
        waveform: Waveform::Saw,
        attack: 0.1,  // 100ms attack
        decay: 0.2,   // 200ms decay
        sustain: 0.5, // 50% sustain level
        release: 0.3, // 300ms release
        filter_cutoff: 20000.0,
        filter_resonance: 0.0,
        filter_env_amount: 0.0,
        gain: Signal::Value(0.5),
        pan: Signal::Value(0.0),
    });

    graph.set_output(synth_node);

    // Render 1 second
    let buffer = graph.render(44100);

    // Analyze amplitude over time
    let chunk_size = 4410; // 100ms chunks
    let mut chunk_rms = Vec::new();
    for i in 0..(buffer.len() / chunk_size) {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(buffer.len());
        let chunk = &buffer[start..end];
        chunk_rms.push(calculate_rms(chunk));
    }

    println!("\n=== SynthPattern ADSR Test ===");
    println!("RMS per 100ms chunk:");
    for (i, rms) in chunk_rms.iter().enumerate() {
        println!("  {}00ms: {:.4}", i, rms);
    }

    // First chunk should have lower RMS (attack phase)
    // Middle chunks should have higher RMS (sustain phase)
    assert!(
        chunk_rms[0] < chunk_rms[2],
        "Attack should be quieter than sustain"
    );

    println!("✅ ADSR envelope shapes the sound!");
}
