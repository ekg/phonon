/// Test Pattern DSP Parameters (gain, pan, speed, cut) with audio verification
///
/// This test suite verifies that DSP parameters for the s() function work correctly:
/// - gain: amplitude scaling (0.0-10.0)
/// - pan: stereo positioning (-1.0 = left, 1.0 = right)
/// - speed: playback rate (0.01-10.0, where 1.0 = normal, 2.0 = double speed)
/// - cut: cut groups for voice stealing (same number = voices stop each other)
///
/// Tests are performed at the DSL level (phonon language) with audio verification.
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

/// Helper to render a sample pattern with DSP parameters
fn render_sample_pattern(
    pattern_str: &str,
    gain: Signal,
    pan: Signal,
    speed: Signal,
    cut_group: Signal,
    cycles: usize,
) -> Vec<f32> {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second for testing

    // Parse the pattern
    let pattern = parse_mini_notation(pattern_str);

    // Create Sample node with DSP parameters
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain,
        pan,
        speed,
        cut_group,
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render N cycles (each cycle is 0.5 seconds at 2 CPS)
    let samples_per_cycle = (44100.0 / 2.0) as usize;
    graph.render(samples_per_cycle * cycles)
}

#[test]
fn test_gain_parameter_constant() {
    // Test that constant gain parameter scales amplitude
    // gain=0.5 should produce half the amplitude of gain=1.0

    let buffer_gain_1 = render_sample_pattern(
        "bd",
        Signal::Value(1.0), // Full gain
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    let buffer_gain_half = render_sample_pattern(
        "bd",
        Signal::Value(0.5), // Half gain
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    // Calculate RMS for both buffers
    let rms_1 =
        (buffer_gain_1.iter().map(|x| x * x).sum::<f32>() / buffer_gain_1.len() as f32).sqrt();
    let rms_half = (buffer_gain_half.iter().map(|x| x * x).sum::<f32>()
        / buffer_gain_half.len() as f32)
        .sqrt();

    println!("Gain 1.0 RMS: {:.6}, Gain 0.5 RMS: {:.6}", rms_1, rms_half);

    // RMS should be approximately half (within 20% tolerance for sample variations)
    let ratio = rms_half / rms_1;
    assert!(
        (ratio - 0.5).abs() < 0.1,
        "Gain 0.5 should produce ~50% RMS vs gain 1.0, got ratio {:.3}",
        ratio
    );
}

#[test]
fn test_gain_parameter_zero() {
    // Test that gain=0.0 produces silence
    let buffer = render_sample_pattern(
        "bd*4",
        Signal::Value(0.0), // Zero gain = silence
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    let max_amplitude = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    println!("Max amplitude with gain=0.0: {}", max_amplitude);
    assert!(
        max_amplitude < 0.001,
        "Gain 0.0 should produce silence, got max amplitude {}",
        max_amplitude
    );
}

#[test]
fn test_gain_parameter_high() {
    // Test that gain > 1.0 increases amplitude
    let buffer_gain_1 = render_sample_pattern(
        "bd",
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    let buffer_gain_2 = render_sample_pattern(
        "bd",
        Signal::Value(2.0), // Double gain
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    let peak_1 = buffer_gain_1.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    let peak_2 = buffer_gain_2.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    println!("Gain 1.0 peak: {:.3}, Gain 2.0 peak: {:.3}", peak_1, peak_2);

    assert!(
        peak_2 > peak_1 * 1.2,
        "Gain 2.0 should produce higher amplitude than gain 1.0, got ratio {:.3}",
        peak_2 / peak_1
    );
}

#[test]
fn test_pan_parameter_left() {
    // Note: This test verifies that pan parameter is being passed to voices
    // Actual stereo positioning would require stereo rendering, which isn't
    // implemented yet in this test. This test just verifies no errors occur.

    let buffer = render_sample_pattern(
        "hh*4",
        Signal::Value(1.0),
        Signal::Value(-1.0), // Full left
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    // Should render without errors
    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pan left RMS: {:.6}", rms);
    assert!(rms > 0.005, "Should have audible content");
}

#[test]
fn test_pan_parameter_right() {
    let buffer = render_sample_pattern(
        "hh*4",
        Signal::Value(1.0),
        Signal::Value(1.0), // Full right
        Signal::Value(1.0),
        Signal::Value(0.0),
        1,
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pan right RMS: {:.6}", rms);
    assert!(rms > 0.005, "Should have audible content");
}

#[test]
fn test_speed_parameter_normal() {
    // Test that speed=1.0 plays at normal rate
    let buffer = render_sample_pattern(
        "bd",
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(1.0), // Normal speed
        Signal::Value(0.0),
        1,
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Speed 1.0 RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");
}

#[test]
fn test_speed_parameter_double() {
    // Test that speed=2.0 plays at double speed (higher pitch, shorter duration)
    let buffer = render_sample_pattern(
        "bd",
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(2.0), // Double speed
        Signal::Value(0.0),
        1,
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Speed 2.0 RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // TODO: Verify that the sample plays faster (shorter duration)
    // This would require analyzing the transient envelope
}

#[test]
fn test_speed_parameter_half() {
    // Test that speed=0.5 plays at half speed (lower pitch, longer duration)
    let buffer = render_sample_pattern(
        "bd",
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(0.5), // Half speed
        Signal::Value(0.0),
        1,
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Speed 0.5 RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");
}

#[test]
fn test_pattern_based_speed() {
    // Test that speed can be controlled by a pattern
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Speed pattern: "1.0 2.0" - normal speed then double speed
    let speed_pattern = parse_mini_notation("1.0 2.0");
    let speed_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "1.0 2.0".to_string(),
        pattern: speed_pattern,
        last_value: 1.0,
        last_trigger_time: -1.0,
    });

    // Sample pattern: two kicks
    let sample_pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Node(speed_node), // Pattern-controlled speed!
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pattern-based speed RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // Note: Verifying different playback speeds through audio analysis would require
    // spectral analysis or duration analysis, which is complex. For now, we verify
    // that pattern-based speed doesn't crash and produces audio.
    // The fix to eval_signal_at_time() ensures each event gets its own speed value.
}

#[test]
fn test_pattern_based_gain() {
    // Test that gain can be controlled by a pattern
    // Create a gain pattern that varies
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Gain pattern: "0.2 1.0" - alternates between quiet and loud
    let gain_pattern = parse_mini_notation("0.2 1.0");
    let gain_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "0.2 1.0".to_string(),
        pattern: gain_pattern,
        last_value: 1.0,
        last_trigger_time: -1.0,
    });

    // Sample pattern: two kicks
    let sample_pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Node(gain_node), // Pattern-controlled gain!
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 cycle (should have 2 kicks with different gains)
    let buffer = graph.render(22050);

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pattern-based gain RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // Verify that the two kicks have different amplitudes
    // Split buffer into halves - first kick gets gain=0.2, second gets gain=1.0
    let first_half = &buffer[0..11025];
    let second_half = &buffer[11025..22050];

    let first_peak = first_half.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    let second_peak = second_half.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    println!("First BD (gain=0.2):  peak = {:.6}", first_peak);
    println!("Second BD (gain=1.0): peak = {:.6}", second_peak);
    let ratio = second_peak / first_peak;
    println!("Ratio: {:.3} (expected ~5.0)", ratio);

    // Second kick should be ~5x louder (gain 1.0 / 0.2 = 5.0)
    assert!(
        (ratio - 5.0).abs() < 1.0,
        "Pattern gain not working: ratio = {:.3}, expected 5.0",
        ratio
    );
}

#[test]
fn test_pattern_based_n() {
    // Test that n (sample number) can be controlled by a pattern
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // N pattern: "0 1" - selects bd:0 then bd:1
    let n_pattern = parse_mini_notation("0 1");
    let n_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "0 1".to_string(),
        pattern: n_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Sample pattern: two kicks (will be modified to bd:0 and bd:1 by n pattern)
    let sample_pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Node(n_node), // Pattern-controlled n!
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pattern-based n RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // Note: Verifying that different samples are triggered would require comparing
    // waveform characteristics of bd:0 vs bd:1. For now, we verify that pattern-based
    // n doesn't crash and produces audio.
}

#[test]
fn test_pattern_based_note() {
    // Test that note (pitch shift) can be controlled by a pattern
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Note pattern: "0 12" - original pitch then octave up
    let note_pattern = parse_mini_notation("0 12");
    let note_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "0 12".to_string(),
        pattern: note_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Sample pattern: two kicks
    let sample_pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Node(note_node), // Pattern-controlled note!
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pattern-based note RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // Note: Verifying pitch changes would require spectral analysis (FFT).
    // For now, we verify that pattern-based note doesn't crash and produces audio.
    // The fix to eval_signal_at_time() ensures each event gets its own note value.
}

#[test]
#[ignore] // Enable once cut groups are fully verified
fn test_cut_group_voice_stealing() {
    // Test that samples with the same cut group stop each other
    // This is tricky to test without access to voice internals

    // Pattern: rapid hi-hats with cut group 1
    // Each new hat should cut the previous one
    let buffer = render_sample_pattern(
        "hh*16",
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(1.0), // Cut group 1
        1,
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Cut group RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // TODO: Verify that voices are being cut
    // Would need to inspect VoiceManager state or analyze transients
}

#[test]
fn test_pattern_based_attack() {
    // Test that attack (envelope attack time) can be controlled by a pattern
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Attack pattern: "0.001 0.1" - fast attack then slow attack
    let attack_pattern = parse_mini_notation("0.001 0.1");
    let attack_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "0.001 0.1".to_string(),
        pattern: attack_pattern,
        last_value: 0.001,
        last_trigger_time: -1.0,
    });

    // Sample pattern: two kicks
    let sample_pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Node(attack_node), // Pattern-controlled attack!
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pattern-based attack RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // Note: Verifying different attack times would require onset/transient analysis.
    // For now, we verify that pattern-based attack doesn't crash and produces audio.
}

#[test]
fn test_pattern_based_release() {
    // Test that release (envelope release time) can be controlled by a pattern
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Release pattern: "0.01 0.5" - short tail then long tail
    let release_pattern = parse_mini_notation("0.01 0.5");
    let release_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "0.01 0.5".to_string(),
        pattern: release_pattern,
        last_value: 0.1,
        last_trigger_time: -1.0,
    });

    // Sample pattern: two kicks
    let sample_pattern = parse_mini_notation("bd bd");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd bd".to_string(),
        pattern: sample_pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Node(release_node), // Pattern-controlled release!
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 cycle
    let buffer = graph.render(22050);

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Pattern-based release RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");

    // Note: Verifying different release times would require tail/decay analysis.
    // For now, we verify that pattern-based release doesn't crash and produces audio.
}

#[test]
fn test_multiple_dsp_parameters_together() {
    // Test that multiple DSP parameters work together
    let buffer = render_sample_pattern(
        "bd sn hh cp",
        Signal::Value(0.8), // Gain
        Signal::Value(0.5), // Pan slightly right
        Signal::Value(1.2), // Speed up by 20%
        Signal::Value(0.0), // No cut group
        1,
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Multiple parameters RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");
}

#[test]
fn test_dsp_parameters_with_euclidean_rhythm() {
    // Test DSP parameters with complex Euclidean patterns
    let buffer = render_sample_pattern(
        "bd(3,8)",
        Signal::Value(1.0),
        Signal::Value(0.0),
        Signal::Value(1.0),
        Signal::Value(0.0),
        2, // 2 cycles
    );

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Euclidean with DSP params RMS: {:.6}", rms);
    assert!(rms > 0.01, "Should have audible content");
}
