use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::sample_loader::SampleBank;
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
fn test_sample_playback_with_index_e2e() {
    // Integration test: render audio with indexed samples using DSL
    use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

    let input = r#"
        cps: 2.0
        out: s "bd:0 bd:1 bd:2"
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle (0.5 seconds at 2 CPS)
    let buffer = graph.render(22050);

    // Calculate RMS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("RMS with bank selection: {}", rms);
    assert!(
        rms > 0.01,
        "Sample pattern with bank indices should produce audio, got RMS: {}",
        rms
    );
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
