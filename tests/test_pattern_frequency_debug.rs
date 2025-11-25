//! Debug test for pattern frequency parameters
//!
//! This test creates ADSR-gated sine waves with pattern-controlled frequency
//! and verifies:
//! 1. Frequency is correct (110 Hz vs 220 Hz)
//! 2. Sine wave is pure (single FFT peak, no harmonics)
//! 3. Pattern actually alternates between frequencies

use std::cell::RefCell;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};

mod audio_test_utils;
use audio_test_utils::{find_dominant_frequency, find_frequency_peaks};

/// Helper to check if a sine wave is pure (only fundamental, no harmonics)
fn measure_sine_purity(buffer: &[f32], sample_rate: f32, _expected_freq: f32) -> (f32, bool) {
    // Find top 10 peaks
    let peaks = find_frequency_peaks(buffer, sample_rate, 10);

    if peaks.is_empty() {
        return (0.0, false);
    }

    // The strongest peak should be the fundamental
    let fundamental_freq = peaks[0].0;
    let fundamental_mag = peaks[0].1;

    // Check for actual harmonics (integer multiples of fundamental)
    // For a pure sine, harmonics at 2x, 3x, 4x etc should be at least 100x weaker
    let mut has_harmonics = false;
    for (freq, mag) in &peaks[1..] {
        // Check if this peak is near an integer multiple of the fundamental
        // (within 20% to account for FFT bin resolution)
        for harmonic_num in 2..=10 {
            let expected_harmonic = fundamental_freq * harmonic_num as f32;
            let tolerance = fundamental_freq * 0.2; // 20% tolerance

            if (*freq - expected_harmonic).abs() < tolerance {
                if *mag > fundamental_mag * 0.01 {
                    println!("  WARNING: Harmonic {} at {:.1} Hz with magnitude {:.2} ({:.1}% of fundamental)",
                             harmonic_num, freq, mag, (mag / fundamental_mag * 100.0));
                    has_harmonics = true;
                }
            }
        }
    }

    let is_pure = !has_harmonics;
    (fundamental_freq, is_pure)
}

#[test]
fn test_manual_sine_synthesis_reference() {
    // First, verify we can manually synthesize a pure 110 Hz sine wave
    // This is our reference - it should be perfectly pure
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second = 0.5s per cycle

    // Manual sine wave at 110 Hz (no pattern, just constant)
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // ADSR envelope to gate each note (attack + decay + release = 0.5s = one cycle)
    // Pattern: <1 0> alternates trigger on/off each cycle
    let trigger_pattern = parse_mini_notation("<1 0>");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "<1 0>".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    let gated = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(osc),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.01), // 10ms attack
        decay: Signal::Value(0.0),   // No decay phase
        sustain: Signal::Value(1.0), // Full sustain
        release: Signal::Value(0.2), // 200ms release (within 0.5s cycle)
        state: Default::default(),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(gated),
        b: Signal::Value(0.5),
    });

    graph.set_output(scaled);

    // Render 2 seconds (4 cycles: on, off, on, off)
    let buffer = graph.render((sample_rate * 2.0) as usize);

    // Analyze the first "on" cycle (samples 0-22050)
    let first_cycle = &buffer[1000..15000]; // Skip attack, analyze sustain
    let (freq, is_pure) = measure_sine_purity(first_cycle, sample_rate, 110.0);

    println!("\n=== Manual Sine Synthesis Reference ===");
    println!("Expected: 110 Hz pure sine wave");
    println!("Detected: {:.1} Hz", freq);
    println!("Is pure: {}", is_pure);

    assert!(
        (freq - 110.0).abs() < 5.0,
        "Manual sine should be 110 Hz, got {:.1} Hz",
        freq
    );
    assert!(is_pure, "Manual sine should be pure (no harmonics)");

    println!("✅ Manual sine synthesis works perfectly");
}

#[test]
fn test_pattern_controlled_frequency_with_alternation() {
    // Now test pattern-controlled frequency: sine "<110 220>"
    // This should alternate between 110 Hz and 220 Hz each cycle
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second

    // Pattern for frequency: alternates 110 <-> 220 each cycle
    let freq_pattern = parse_mini_notation("<110 220>");
    let freq_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "<110 220>".to_string(),
        pattern: freq_pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    // Sine oscillator with pattern-controlled frequency
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(freq_node),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // ADSR to gate each note
    let trigger_pattern = parse_mini_notation("<1 0 1 0>");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "<1 0 1 0>".to_string(),
        pattern: trigger_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    let gated = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(osc),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.0),
        sustain: Signal::Value(1.0),
        release: Signal::Value(0.2),
        state: Default::default(),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(gated),
        b: Signal::Value(0.5),
    });

    graph.set_output(scaled);

    // Render 2 seconds (4 cycles)
    // Cycle 0 (0.0-0.5s): freq=110, trigger=1 → 110 Hz note
    // Cycle 1 (0.5-1.0s): freq=220, trigger=0 → silence
    // Cycle 2 (1.0-1.5s): freq=110, trigger=1 → 110 Hz note
    // Cycle 3 (1.5-2.0s): freq=220, trigger=0 → silence
    let buffer = graph.render((sample_rate * 2.0) as usize);

    println!("\n=== Pattern-Controlled Frequency Test ===");
    println!("Pattern: <110 220>");
    println!("Trigger: <1 0 1 0>");
    println!("Expected: Cycle 0: 110Hz, Cycle 1: silent, Cycle 2: 110Hz, Cycle 3: silent");

    // Analyze cycle 0 (110 Hz expected)
    let cycle0 = &buffer[2000..18000]; // 0.0-0.4s (skip attack, before release)
    let (freq0, pure0) = measure_sine_purity(cycle0, sample_rate, 110.0);

    println!("\nCycle 0 analysis (expected 110 Hz):");
    println!("  Frequency: {:.1} Hz", freq0);
    println!("  Is pure: {}", pure0);

    // Analyze cycle 2 (110 Hz expected - alternation repeats)
    let cycle2 = &buffer[44100 + 2000..44100 + 18000]; // 1.0-1.4s
    let (freq2, pure2) = measure_sine_purity(cycle2, sample_rate, 110.0);

    println!("\nCycle 2 analysis (expected 110 Hz):");
    println!("  Frequency: {:.1} Hz", freq2);
    println!("  Is pure: {}", pure2);

    // Both should be 110 Hz (alternation pattern repeats: 110, 220, 110, 220...)
    // But with trigger <1 0 1 0>, we only hear cycles 0 and 2, both at 110 Hz
    assert!(
        (freq0 - 110.0).abs() < 10.0,
        "Cycle 0 should be 110 Hz, got {:.1} Hz (BROKEN - pattern freq not working!)",
        freq0
    );

    assert!(
        (freq2 - 110.0).abs() < 10.0,
        "Cycle 2 should be 110 Hz, got {:.1} Hz",
        freq2
    );

    assert!(pure0, "Cycle 0 should be pure sine wave");
    assert!(pure2, "Cycle 2 should be pure sine wave");

    println!("\n✅ Pattern frequency control works and produces pure sine waves!");
}

#[test]
fn test_pattern_frequency_both_notes_gated() {
    // Test with both notes gated to hear both 110 Hz and 220 Hz
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second

    // Frequency pattern: <110 220>
    let freq_pattern = parse_mini_notation("<110 220>");
    let freq_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "<110 220>".to_string(),
        pattern: freq_pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(freq_node),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Trigger both notes: <1 1>
    let trigger_pattern = parse_mini_notation("<1 1>");
    let trigger = graph.add_node(SignalNode::Pattern {
        pattern_str: "<1 1>".to_string(),
        pattern: trigger_pattern,
        last_value: 1.0,
        last_trigger_time: -1.0,
    });

    let gated = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(osc),
        trigger: Signal::Node(trigger),
        attack: Signal::Value(0.01),
        decay: Signal::Value(0.0),
        sustain: Signal::Value(1.0),
        release: Signal::Value(0.2),
        state: Default::default(),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(gated),
        b: Signal::Value(0.5),
    });

    graph.set_output(scaled);

    // Render 2 seconds
    // Cycle 0: freq=110, trigger=1 → 110 Hz
    // Cycle 1: freq=220, trigger=1 → 220 Hz
    // Cycle 2: freq=110, trigger=1 → 110 Hz
    // Cycle 3: freq=220, trigger=1 → 220 Hz
    let buffer = graph.render((sample_rate * 2.0) as usize);

    println!("\n=== Both Notes Gated (110 Hz and 220 Hz) ===");

    // Analyze cycle 0 (110 Hz expected)
    let cycle0 = &buffer[2000..18000];
    let (freq0, pure0) = measure_sine_purity(cycle0, sample_rate, 110.0);

    // Analyze cycle 1 (220 Hz expected)
    let cycle1 = &buffer[22050 + 2000..22050 + 18000];
    let (freq1, pure1) = measure_sine_purity(cycle1, sample_rate, 220.0);

    println!(
        "\nCycle 0 (expected 110 Hz): {:.1} Hz, pure: {}",
        freq0, pure0
    );
    println!(
        "Cycle 1 (expected 220 Hz): {:.1} Hz, pure: {}",
        freq1, pure1
    );

    // Verify 110 Hz in cycle 0
    assert!(
        (freq0 - 110.0).abs() < 10.0,
        "Cycle 0 should be 110 Hz, got {:.1} Hz",
        freq0
    );

    // Verify 220 Hz in cycle 1
    assert!(
        (freq1 - 220.0).abs() < 10.0,
        "Cycle 1 should be 220 Hz, got {:.1} Hz",
        freq1
    );

    // Verify both are pure
    assert!(pure0, "110 Hz should be pure");
    assert!(pure1, "220 Hz should be pure");

    // Verify they're different (pattern is actually alternating)
    let ratio = freq1 / freq0;
    assert!(
        (ratio - 2.0).abs() < 0.2,
        "Frequency ratio should be ~2.0 (octave), got {:.2}",
        ratio
    );

    println!("\n✅ Pattern alternates correctly between 110 Hz and 220 Hz!");
    println!("   Ratio: {:.2}x (expected 2.0 for octave)", ratio);
}

#[test]
fn test_diagnose_4700hz_problem() {
    // Reproduce the exact problem: what IS producing 4704 Hz?
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(3.0); // Same as the failing test

    // Exact same setup as failing test
    let freq_pattern = parse_mini_notation("110 220 330");
    let freq_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "110 220 330".to_string(),
        pattern: freq_pattern,
        last_value: 110.0,
        last_trigger_time: -1.0,
    });

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(freq_node),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Value(0.2),
    });

    graph.set_output(scaled);

    // Render and analyze
    let buffer = graph.render(44100); // 1 second

    let dominant = find_dominant_frequency(&buffer, sample_rate);
    let peaks = find_frequency_peaks(&buffer, sample_rate, 10);

    println!("\n=== Diagnosing 4704 Hz Problem ===");
    println!("Pattern: \"110 220 330\"");
    println!("Expected: Frequencies should cycle through 110, 220, 330 Hz");
    println!("\nDominant frequency: {:.1} Hz", dominant);
    println!("\nTop 10 frequency peaks:");
    for (i, (freq, mag)) in peaks.iter().enumerate() {
        println!("  {}. {:.1} Hz (magnitude: {:.2})", i + 1, freq, mag);
    }

    // This test documents the problem - don't assert, just report
    println!("\n⚠️  PROBLEM DOCUMENTED:");
    if dominant > 1000.0 {
        println!(
            "   Frequency is WAY too high: {:.1} Hz instead of ~110-330 Hz",
            dominant
        );
        println!("   This suggests pattern values aren't being applied correctly!");
    }
}
