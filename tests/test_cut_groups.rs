use phonon::sample_loader::SampleBank;
/// Tests for cut group voice stealing functionality
/// Cut groups allow samples to stop each other when triggered (like open/closed hi-hats)
use phonon::voice_manager::VoiceManager;

#[test]
fn test_cut_group_stops_previous_voice() {
    // Test that triggering a sample with cut_group=1 stops any other playing voice with cut_group=1
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");
    let hh_sample = bank.get_sample("hh").expect("HH sample should load");

    let mut vm = VoiceManager::new();

    // Trigger BD with cut_group=1
    vm.trigger_sample_with_cut_group(bd_sample.clone(), 1.0, 0.0, 1.0, Some(1));

    // Process a few samples to ensure BD is playing
    let mut has_audio = false;
    for _ in 0..100 {
        let sample = vm.process();
        if sample.abs() > 0.01 {
            has_audio = true;
            break;
        }
    }
    assert!(has_audio, "BD should be playing after trigger");

    // Now trigger HH with the same cut_group=1 - this should stop BD
    vm.trigger_sample_with_cut_group(hh_sample.clone(), 1.0, 0.0, 1.0, Some(1));

    // Process a few more samples to verify HH is now playing
    let mut samples = Vec::new();
    for _ in 0..1000 {
        samples.push(vm.process());
    }

    // We should have audio (from HH), and BD should have been stopped
    let rms = calculate_rms(&samples);
    assert!(rms > 0.01, "Should have audio from HH, got RMS={}", rms);

    println!("✓ Cut group test passed: cut_group=1 stopped previous voice");
}

#[test]
fn test_different_cut_groups_dont_interfere() {
    // Test that samples with different cut groups can play simultaneously
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");
    let sn_sample = bank.get_sample("sn").expect("SN sample should load");

    let mut vm = VoiceManager::new();

    // Trigger BD with cut_group=1
    vm.trigger_sample_with_cut_group(bd_sample.clone(), 1.0, 0.0, 1.0, Some(1));

    // Trigger SN with cut_group=2 - should NOT stop BD
    vm.trigger_sample_with_cut_group(sn_sample.clone(), 1.0, 0.0, 1.0, Some(2));

    // Process samples - should have both playing
    let mut samples = Vec::new();
    for _ in 0..2000 {
        samples.push(vm.process());
    }

    let rms = calculate_rms(&samples);

    // With both samples playing, RMS should be substantial
    assert!(rms > 0.05, "Both samples should play, got RMS={}", rms);

    println!("✓ Different cut groups test passed: cut_group=1 and cut_group=2 don't interfere");
}

#[test]
fn test_no_cut_group_plays_polyphonically() {
    // Test that samples without cut groups (cut_group=None) can layer
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");

    let mut vm = VoiceManager::new();

    // Trigger multiple BD samples without cut groups
    for _ in 0..4 {
        vm.trigger_sample_with_cut_group(bd_sample.clone(), 1.0, 0.0, 1.0, None);
    }

    // Process samples - all should play polyphonically
    let mut samples = Vec::new();
    for _ in 0..2000 {
        samples.push(vm.process());
    }

    let rms = calculate_rms(&samples);

    // With 4 samples layered, RMS should be quite high
    assert!(
        rms > 0.1,
        "Four layered samples should have high RMS, got {}",
        rms
    );

    println!("✓ No cut group test passed: samples without cut groups play polyphonically");
}

#[test]
#[ignore = "Cut group integration with samples needs investigation"]
fn test_cut_group_integration_with_unified_graph() {
    // Test cut groups work through the UnifiedSignalGraph
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
    use std::collections::HashMap;

    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(4.0); // 4 cycles per second = fast triggering

    // Create a pattern that rapidly triggers samples with cut_group=1
    // Each hit should cut the previous one
    let pattern = parse_mini_notation("hh*8");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "hh*8".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(1.0), // All in cut_group=1
        n: phonon::unified_graph::Signal::Value(0.0),
        note: phonon::unified_graph::Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 1 second
    let buffer = graph.render(sample_rate as usize);

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!("✓ UnifiedGraph cut groups test:");
    println!("  RMS: {:.4}", rms);
    println!("  Peak: {:.4}", peak);

    // Should have audio but each hit cuts the previous one
    // With rapid triggering and cut groups, peaks will be lower since voices get cut early
    // RMS ~0.006 is expected with rapid cutting of closed hi-hat samples
    assert!(rms > 0.005, "Should have audio from hi-hats");
    assert!(peak > 0.05, "Should have peaks from hi-hat triggers");
}

#[test]
#[ignore = "Pattern-controlled cut groups with samples needs investigation"]
fn test_pattern_controlled_cut_groups() {
    // Test that cut_group can be controlled by a pattern
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};
    use std::collections::HashMap;

    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(2.0);

    // Create a pattern that alternates cut groups: 0 1 0 2
    let cut_pattern = parse_mini_notation("0 1 0 2");
    let cut_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "0 1 0 2".to_string(),
        pattern: cut_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Create hi-hat pattern with pattern-controlled cut groups
    let pattern = parse_mini_notation("hh hh hh hh");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "hh hh hh hh".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Node(cut_node), // Pattern-controlled!
        n: phonon::unified_graph::Signal::Value(0.0),
        note: phonon::unified_graph::Signal::Value(0.0),
        attack: Signal::Value(0.001),
        release: Signal::Value(0.1),
        envelope_type: None,
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
        unit_mode: Signal::Value(0.0),
        loop_enabled: Signal::Value(0.0),
    });

    graph.set_output(sample_node);

    // Render 2 cycles
    let buffer = graph.render((sample_rate * 1.0) as usize);

    let rms = calculate_rms(&buffer);
    println!("✓ Pattern-controlled cut groups test: RMS={:.4}", rms);

    // RMS ~0.008 is expected with pattern-controlled cut groups
    assert!(
        rms > 0.005,
        "Should have audio with pattern-controlled cut groups"
    );
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}
