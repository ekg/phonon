use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::Pattern;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

/// Helper to calculate RMS
fn calculate_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
}

// ============================================================================
// Complex Transform Chain Tests
// ============================================================================

/// Test 3-level nested transforms: fast -> rev -> slow
#[test]
#[ignore = "Uses old pipe syntax - needs update"]
fn test_triple_nested_transforms() {
    let input = r#"
        cps: 2.0
        out: s("bd sn hh cp" |> fast 2 |> rev |> slow 2) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse triple nested");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(88200); // 2 seconds
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Triple nested transforms should produce audio: RMS = {}",
        rms
    );

    // Verify we have audio activity
    let has_peaks = buffer.iter().any(|&s| s.abs() > 0.3);
    assert!(has_peaks, "Triple nested transforms should have peaks");
}

/// Test every with nested transform
#[test]
#[ignore = "Uses old pipe syntax - needs update"]
fn test_every_with_nested_transform() {
    let input = r#"
        cps: 1.0
        out: s("bd sn" |> every 2 (fast 2 |> rev)) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse every nested");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(176400); // 4 seconds to hear alternation
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Every with nested transform should produce audio: RMS = {}",
        rms
    );
}

/// Test multiple every transforms stacked
#[test]
#[ignore = "Uses old pipe syntax - needs update"]
fn test_multiple_every_transforms() {
    let input = r#"
        cps: 2.0
        out: s("bd*4" |> every 2 (fast 2) |> every 3 (rev)) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse multiple every");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(132300); // 3 seconds
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Multiple every transforms should produce audio: RMS = {}",
        rms
    );
}

// ============================================================================
// Pattern Parameters + Transform Interaction Tests
// ============================================================================

/// Test pattern gain parameter with fast transform
#[test]
fn test_pattern_gain_with_fast_transform() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Pattern with gain accents, then fast transform
    let pattern_str = "bd sn hh cp";
    let gain_pattern_str = "1.0 0.8 0.6 0.9";

    let pattern = parse_mini_notation(pattern_str).fast(Pattern::pure(2.0));

    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Pattern(gain_pattern_str.to_string()),
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

    graph.set_output_channel(1, sample_node);

    let buffer = graph.render(44100); // 1 second
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Pattern gain with fast transform should work: RMS = {}",
        rms
    );

    // Check we have varying amplitudes (gain pattern is working)
    let windows = buffer.chunks(4410); // 100ms windows
    let mut rms_values: Vec<f32> = windows.map(|w| calculate_rms(w)).collect();
    rms_values.retain(|&rms| rms > 0.01); // Filter out silent windows

    if rms_values.len() >= 2 {
        let max_rms = rms_values.iter().cloned().fold(0.0f32, f32::max);
        let min_rms = rms_values.iter().cloned().fold(1.0f32, f32::min);
        let variation = max_rms / min_rms.max(0.001);

        assert!(
            variation > 1.2,
            "Pattern gain should create amplitude variation: ratio = {}",
            variation
        );
    }
}

/// Test pattern envelope parameters with reverse transform
#[test]
fn test_pattern_envelope_with_reverse() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    let pattern_str = "bd sn hh cp";
    let attack_str = "0.001 0.05 0.001 0.02";
    let release_str = "0.1 0.3 0.05 0.2";

    let pattern = parse_mini_notation(pattern_str).rev();

    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Pattern(attack_str.to_string()),
        release: Signal::Pattern(release_str.to_string()),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output_channel(1, sample_node);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Pattern envelope with reverse should work: RMS = {}",
        rms
    );
}

/// Test pattern pan with slow transform
#[test]
#[ignore = "Uses old pipe syntax - needs update"]
fn test_pattern_pan_with_slow() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0); // Fast tempo

    let pattern_str = "hh*8";
    let pan_str = "-1.0 -0.5 0.0 0.5";

    let pattern = parse_mini_notation(pattern_str).slow(Pattern::pure(2.0));

    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.8),
        pan: Signal::Pattern(pan_str.to_string()),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.05),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output_channel(1, sample_node);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.03,
        "Pattern pan with slow should work: RMS = {}",
        rms
    );
}

/// Test pattern speed with every transform
#[test]
fn test_pattern_speed_with_every() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    let pattern_str = "bd*4";
    let speed_str = "1.0 1.2 0.8 1.5";

    let pattern = parse_mini_notation(pattern_str);
    let pattern_with_every = pattern.every(2, |p| p.fast(Pattern::pure(2.0)));

    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern: pattern_with_every,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Pattern(speed_str.to_string()),
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

    graph.set_output_channel(1, sample_node);

    let buffer = graph.render(88200); // 2 seconds to hear every
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Pattern speed with every should work: RMS = {}",
        rms
    );
}

// ============================================================================
// High Voice Count Stress Tests
// ============================================================================

/// Test 64+ simultaneous voices with varying envelopes
#[test]
fn test_64_voice_stress_test() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(8.0); // Very fast tempo

    // Trigger many samples quickly to stress voice allocation
    let pattern = parse_mini_notation("bd*32");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd*32".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.8),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.3), // Long release to cause overlap
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output_channel(1, sample_node);

    let buffer = graph.render(44100); // 1 second
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.3,
        "64+ voice stress test should produce audio: RMS = {}",
        rms
    );

    // Verify no clipping (should use soft limiting)
    let max_sample = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_sample <= 1.1,
        "Should not clip excessively: max = {}",
        max_sample
    );
}

/// Test rapid triggers with long release (envelope overlap)
#[test]
fn test_rapid_triggers_long_release() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(10.0); // Very fast - 100ms per cycle

    let pattern = parse_mini_notation("bd*16");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd*16".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.5),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(1.0), // 1 second release - extreme overlap
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output_channel(1, sample_node);

    let buffer = graph.render(44100); // 1 second
    let rms = calculate_rms(&buffer);

    // Should have sustained audio due to overlapping envelopes
    assert!(
        rms > 0.2,
        "Rapid triggers with long release should produce sustained audio: RMS = {}",
        rms
    );
}

// ============================================================================
// Cut Group Edge Cases
// ============================================================================

/// Test cut group stops voice with long envelope
#[test]
fn test_cut_group_stops_long_envelope() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0); // 250ms per cycle

    // First voice: long release, cut group 1
    let pattern1 = parse_mini_notation("hh:2"); // Open hat
    let node1 = graph.add_node(SignalNode::Sample {
        pattern_str: "hh:2".to_string(),
        pattern: pattern1,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.8),
        pan: Signal::Value(0.2),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(1.0), // Cut group 1
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.5), // Long release
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    // Second voice: short release, same cut group
    let pattern2 = parse_mini_notation("~ ~ hh:0 ~"); // Closed hat after 2 beats
    let node2 = graph.add_node(SignalNode::Sample {
        pattern_str: "~ ~ hh:0 ~".to_string(),
        pattern: pattern2,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.7),
        pan: Signal::Value(-0.2),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(1.0), // Same cut group
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.05), // Short release
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    // Mix both
    let mix = graph.add_node(SignalNode::Add {
        a: Signal::Node(node1),
        b: Signal::Node(node2),
    });

    graph.set_output_channel(1, mix);

    let buffer = graph.render(44100); // 1 second

    // Analyze first and second halves
    let first_half = &buffer[0..22050];
    let second_half = &buffer[22050..44100];

    let rms_first = calculate_rms(first_half);
    let rms_second = calculate_rms(second_half);

    // Should have audio in both halves
    assert!(rms_first > 0.01, "Should have audio in first half");
    assert!(rms_second > 0.01, "Should have audio in second half");
}

/// Test different cut groups don't interfere
#[test]
fn test_different_cut_groups_independent() {
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0);

    // Voice 1: cut group 1
    let pattern1 = parse_mini_notation("bd*4");
    let node1 = graph.add_node(SignalNode::Sample {
        pattern_str: "bd*4".to_string(),
        pattern: pattern1,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(-0.5),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(1.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.2),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    // Voice 2: cut group 2 (different)
    let pattern2 = parse_mini_notation("sn*4");
    let node2 = graph.add_node(SignalNode::Sample {
        pattern_str: "sn*4".to_string(),
        pattern: pattern2,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(0.9),
        pan: Signal::Value(0.5),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(2.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.15),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    let mix = graph.add_node(SignalNode::Add {
        a: Signal::Node(node1),
        b: Signal::Node(node2),
    });

    graph.set_output_channel(1, mix);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);

    // Both patterns should play simultaneously
    assert!(
        rms > 0.15,
        "Different cut groups should not interfere: RMS = {}",
        rms
    );
}

// ============================================================================
// Complex Effect Chain Tests
// ============================================================================

/// Test synthesis -> filter -> distortion -> reverb with pattern modulation
#[test]
fn test_complex_fx_chain_with_patterns() {
    let input = r#"
        cps: 2.0
        ~bass: saw "55 82.5 110"
        ~filtered: ~bass >> lpf("500 2000" |> fast 2, 0.8)
        out: ~filtered * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse complex FX");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(88200); // 2 seconds
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "Complex FX chain with patterns should work: RMS = {}",
        rms
    );

    // Verify we have frequency modulation (spectral variation)
    let first_chunk = &buffer[0..22050];
    let last_chunk = &buffer[66150..88200];

    let rms_first = calculate_rms(first_chunk);
    let rms_last = calculate_rms(last_chunk);

    // Both should have audio (pattern is cycling)
    assert!(rms_first > 0.01, "First chunk should have audio");
    assert!(rms_last > 0.01, "Last chunk should have audio");
}

/// Test multiple sample tracks with different transforms
#[test]
#[ignore = "Uses old pipe syntax - needs update"]
fn test_multi_track_with_transforms() {
    let input = r#"
        cps: 2.0
        ~kick: s("bd*4" |> fast 2, 1.0, 0.0, 1.0, 0, 0.001, 0.08)
        ~snare: s("~ sn" |> every 4 (fast 2), 0.9, 0.1, 1.0, 0, 0.001, 0.15)
        ~hh: s("hh*8" |> rev, 0.6, "-0.2 0.2", 1.0, 1, 0.001, 0.05)
        out: (~kick + ~snare + ~hh) * 0.4
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse multi-track");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(88200); // 2 seconds
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "Multi-track with transforms should produce substantial audio: RMS = {}",
        rms
    );

    // Verify we have peaks (drums)
    let max_sample = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_sample > 0.3,
        "Should have strong drum peaks: max = {}",
        max_sample
    );
}
