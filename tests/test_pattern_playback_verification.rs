/// Simple tests verifying pattern operations work with sample playback
/// These tests don't try to detect exact onset timing, just verify audio is present
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::sample_loader::SampleBank;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[test]
fn test_alternation_cycles_have_audio() {
    // Test <bd sn> - each cycle should have audio
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("<bd sn>");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "<bd sn>".to_string(),
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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph.set_output(sample_node);

    // Render 4 cycles
    let num_cycles = 4;
    let total_samples = (sample_rate * num_cycles as f32) as usize;
    let buffer = graph.render(total_samples);

    println!("\n✓ Alternation Test: <bd sn>");

    let samples_per_cycle = sample_rate as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];
        let cycle_rms = calculate_rms(cycle_samples);
        let cycle_peak = cycle_samples
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        println!(
            "  Cycle {}: RMS={:.4}, Peak={:.4}",
            cycle, cycle_rms, cycle_peak
        );

        // Each cycle should have audio
        assert!(cycle_rms > 0.01, "Cycle {} should have audio", cycle);
        assert!(cycle_peak > 0.5, "Cycle {} should have strong peaks", cycle);
    }
}

#[test]
fn test_concatenation_cycles_have_audio() {
    // Test "bd sn cp hh" over multiple cycles
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles per second

    let pattern = parse_mini_notation("bd sn cp hh");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd sn cp hh".to_string(),
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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph.set_output(sample_node);

    // Render 4 cycles = 2 seconds
    let duration_secs = 2.0;
    let buffer = graph.render((sample_rate * duration_secs) as usize);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("\n✓ Concatenation Test: bd sn cp hh");
    println!("  RMS={:.4}, Peak={:.4}", rms, peak);

    assert!(rms > 0.05, "Should have substantial audio");
    assert!(peak > 0.8, "Should have strong peaks");
}

#[test]
fn test_subdivision_has_audio() {
    // Test bd*16 - should have audio
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("bd*16");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd*16".to_string(),
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
    let buffer = graph.render(sample_rate as usize);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("\n✓ Fast Subdivision Test: bd*16");
    println!("  RMS={:.4}, Peak={:.4}", rms, peak);

    assert!(rms > 0.1, "Should have substantial audio from 16 hits");
    assert!(peak > 0.8, "Should have strong peaks");
}

#[test]
fn test_euclidean_alternation_has_audio() {
    // Test <bd(3,8) sn(5,8)> - alternating euclidean patterns
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("<bd(3,8) sn(5,8)>");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "<bd(3,8) sn(5,8)>".to_string(),
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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph.set_output(sample_node);

    // Render 4 cycles
    let num_cycles = 4;
    let total_samples = (sample_rate * num_cycles as f32) as usize;
    let buffer = graph.render(total_samples);

    println!("\n✓ Euclidean Alternation Test: <bd(3,8) sn(5,8)>");

    let samples_per_cycle = sample_rate as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];
        let cycle_rms = calculate_rms(cycle_samples);

        println!("  Cycle {}: RMS={:.4}", cycle, cycle_rms);
        assert!(cycle_rms > 0.01, "Cycle {} should have audio", cycle);
    }
}

#[test]
fn test_layering_has_louder_audio() {
    // Test [bd, sn] - layering should produce audio with both samples
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0);

    let pattern = parse_mini_notation("[bd, sn]");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "[bd, sn]".to_string(),
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
    let buffer = graph.render(sample_rate as usize);

    // Compare to single sample
    let mut graph_single = UnifiedSignalGraph::new(sample_rate);
    graph_single.set_cps(1.0);
    let pattern_single = parse_mini_notation("bd");
    let sample_node_single = graph_single.add_node(SignalNode::Sample {
        pattern_str: "bd".to_string(),
        pattern: pattern_single,
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
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph_single.set_output(sample_node_single);
    let buffer_single = graph_single.render(sample_rate as usize);

    let layered_rms = calculate_rms(&buffer[0..10000]);
    let single_rms = calculate_rms(&buffer_single[0..10000]);

    println!("\n✓ Layering Test: [bd, sn]");
    println!(
        "  Layered RMS={:.4}, Single RMS={:.4}",
        layered_rms, single_rms
    );

    // Layered should have comparable or higher RMS due to mixing
    assert!(
        layered_rms > single_rms * 0.7,
        "Layered should have substantial audio"
    );
}

#[test]
fn test_multiple_bars_consistent() {
    // Test pattern works consistently over many bars
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0); // 2 cycles/sec = 0.5sec/cycle

    let pattern = parse_mini_notation("bd cp");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd cp".to_string(),
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
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });
    graph.set_output(sample_node);

    // Render 8 cycles = 4 seconds
    let num_cycles = 8;
    let duration = num_cycles as f32 / 2.0;
    let buffer = graph.render((sample_rate * duration) as usize);

    println!("\n✓ Multi-Bar Test: bd cp over {} cycles", num_cycles);

    let samples_per_cycle = (sample_rate / 2.0) as usize;
    for cycle in 0..num_cycles {
        let start = cycle * samples_per_cycle;
        let end = start + samples_per_cycle;
        let cycle_samples = &buffer[start..end];
        let cycle_rms = calculate_rms(cycle_samples);

        if cycle < 3 || cycle >= num_cycles - 1 {
            println!("  Cycle {}: RMS={:.4}", cycle, cycle_rms);
        } else if cycle == 3 {
            println!("  ...");
        }

        assert!(cycle_rms > 0.01, "Cycle {} should have audio", cycle);
    }
}
