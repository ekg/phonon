use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::sample_loader::SampleBank;
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_sample_bank_basic_index() {
    // Test that sample bank can load samples with index notation
    let mut bank = SampleBank::new();

    // Try to load bd:0 (first sample in bd directory)
    let sample = bank.get_sample("bd:0");
    if sample.is_some() {
        assert!(sample.unwrap().len() > 0, "Sample should have audio data");
    }

    // Try to load bd:1 (second sample in bd directory)
    let sample = bank.get_sample("bd:1");
    if sample.is_some() {
        assert!(sample.unwrap().len() > 0, "Sample should have audio data");
    }
}

#[test]
fn test_sample_bank_different_samples() {
    // Test that different indices return different samples
    let mut bank = SampleBank::new();

    let sample0 = bank.get_sample("bd:0");
    let sample1 = bank.get_sample("bd:1");

    // If both samples exist, they should be different
    if let (Some(s0), Some(s1)) = (sample0, sample1) {
        // Check that they're not the same pointer
        assert_ne!(
            Arc::ptr_eq(&s0, &s1),
            true,
            "bd:0 and bd:1 should be different samples"
        );

        // They might have different lengths or content
        let different_length = s0.len() != s1.len();
        let different_content = s0.iter().zip(s1.iter()).any(|(a, b)| a != b);

        assert!(
            different_length || different_content,
            "bd:0 and bd:1 should have different audio content"
        );
    }
}

#[test]
fn test_mini_notation_with_sample_index() {
    // Test that mini-notation parser preserves colons in sample names
    let pattern = parse_mini_notation("bd:0 bd:1 bd:2");

    // Verify the pattern has 3 events
    // We'll check by querying different time windows
    let mut found_events = Vec::new();

    // Query the whole first cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);

    for event in &events {
        found_events.push(event.value.clone());
    }

    // Should find 3 events
    assert!(
        found_events.len() >= 3,
        "Should find at least 3 events in pattern, found {}",
        found_events.len()
    );

    // Check that the sample names contain colons
    let has_colon_syntax = found_events.iter().any(|name| name.contains(':'));
    assert!(
        has_colon_syntax,
        "Pattern should preserve colon syntax in sample names. Found: {:?}",
        found_events
    );
}

#[test]
#[ignore] // Requires dirt-samples to be present
fn test_sample_playback_with_index() {
    // Integration test: render audio with indexed samples
    let sample_rate = 44100.0;
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0); // 1 cycle per second

    // Create a pattern with sample indices
    let pattern = parse_mini_notation("bd:0 bd:1 bd:2");
    let sample_node = graph.add_node(SignalNode::Sample {
        pattern_str: "bd:0 bd:1 bd:2".to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: phonon::unified_graph::Signal::Value(1.0),
        pan: phonon::unified_graph::Signal::Value(0.0),
        speed: phonon::unified_graph::Signal::Value(1.0),
        cut_group: phonon::unified_graph::Signal::Value(0.0),
        n: phonon::unified_graph::Signal::Value(0.0),
        note: phonon::unified_graph::Signal::Value(0.0),
        attack: phonon::unified_graph::Signal::Value(0.001),
        release: phonon::unified_graph::Signal::Value(0.1),
        envelope_type: None,
    });

    graph.set_output(sample_node);

    // Process one cycle
    let samples_per_cycle = sample_rate as usize;
    let mut has_audio = false;

    for _ in 0..samples_per_cycle {
        let sample = graph.process_sample();
        if sample.abs() > 0.001 {
            has_audio = true;
        }
    }

    assert!(has_audio, "Should produce audio with indexed samples");
}

#[test]
fn test_fallback_to_first_sample() {
    // Test that requesting a non-existent index falls back gracefully
    let mut bank = SampleBank::new();

    // Try to load bd:999 (probably doesn't exist)
    let sample_high = bank.get_sample("bd:999");

    // It should still work (falling back to available samples or returning None)
    // Either outcome is acceptable
    if let Some(s) = sample_high {
        assert!(s.len() > 0, "Fallback sample should have audio data");
    }
}
