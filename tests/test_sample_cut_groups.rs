/// Test cut groups for sample playback
///
/// Cut groups allow samples to stop each other when triggered.
/// Common use case: hi-hat open and closed - closed stops the open sound.
///
/// Syntax: s("pattern", gain, pan, speed, cut_group)
/// - s("hh:0 hh:1", "1 1", "0 0", "1 1", "1 1") - both in cut group 1
/// - Cut group 0 = no cutting behavior (default)
/// - Cut group N > 0 = voices in group N stop each other
///
/// This test verifies that voices in the same cut group stop each other.
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_cut_group_stops_previous_voice() {
    // Two hi-hats in cut group 1
    // The second should stop the first
    // Positional args: s("pattern", gain, pan, speed, cut_group)
    let input = r#"
        tempo: 0.5
        out $ s("hh hh", "1.0 1.0", "0 0", "1 1", "1 1")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle = 0.5 seconds at 2 CPS = 22050 samples
    let buffer = graph.render(22050);

    // Track voice count throughout
    let mut max_voices = 0;
    let mut voice_counts = Vec::new();

    // Re-create graph to track voices
    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    for _ in 0..22050 {
        let _ = graph.process_sample();
        let voices = graph.active_voice_count();
        voice_counts.push(voices);
        if voices > max_voices {
            max_voices = voices;
        }
    }

    println!("Max voices with cut group: {}", max_voices);

    // Find where the max occurs
    let max_positions: Vec<usize> = voice_counts
        .iter()
        .enumerate()
        .filter(|(_, &count)| count == max_voices)
        .map(|(idx, _)| idx)
        .collect();
    println!(
        "Max voice count ({}) occurs at samples: {:?}",
        max_voices,
        &max_positions[..max_positions.len().min(10)]
    );

    // With cut groups, we should never have more than 1 voice active
    // (second voice stops first voice)
    assert!(
        max_voices <= 1,
        "Cut group should limit to 1 voice, but found {} voices",
        max_voices
    );

    // Verify we actually triggered samples
    let rms = calculate_rms(&buffer);
    println!("RMS with cut group: {:.4}", rms);
    assert!(rms > 0.01, "Should have audio");
}

#[test]
fn test_no_cut_group_allows_overlap() {
    // Two hi-hats with cut group 0 (no cutting)
    // Both should play simultaneously
    let input = r#"
        tempo: 0.5
        out $ s("hh hh", "1.0 1.0", "0 0", "1 1", "0 0")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let mut max_voices = 0;

    for _ in 0..22050 {
        let _ = graph.process_sample();
        let voices = graph.active_voice_count();
        if voices > max_voices {
            max_voices = voices;
        }
    }

    println!("Max voices without cut group: {}", max_voices);

    // Without cut groups, we should see multiple voices active
    assert!(
        max_voices >= 2,
        "Without cut group, should have at least 2 voices, but found {}",
        max_voices
    );
}

#[test]
fn test_different_cut_groups_dont_interact() {
    // Two samples in different cut groups
    // They should not stop each other
    let input = r#"
        tempo: 0.5
        out $ s("hh hh", "1.0 1.0", "0 0", "1 1", "1 2")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let mut max_voices = 0;

    for _ in 0..22050 {
        let _ = graph.process_sample();
        let voices = graph.active_voice_count();
        if voices > max_voices {
            max_voices = voices;
        }
    }

    println!("Max voices with different cut groups: {}", max_voices);

    // Different cut groups should allow overlap
    assert!(
        max_voices >= 2,
        "Different cut groups should allow overlap, but found only {} voices",
        max_voices
    );
}

#[test]
fn test_cut_group_pattern() {
    // Pattern with alternating cut groups
    // Cut group 1 events should stop each other
    let input = r#"
        tempo: 0.5
        out $ s("hh*8", "1 1 1 1 1 1 1 1", "0 0 0 0 0 0 0 0", "1 1 1 1 1 1 1 1", "1 1 1 1 1 1 1 1")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let mut max_voices = 0;

    for _ in 0..22050 {
        let _ = graph.process_sample();
        let voices = graph.active_voice_count();
        if voices > max_voices {
            max_voices = voices;
        }
    }

    println!("Max voices with cut group pattern: {}", max_voices);

    // With all in same cut group, should never exceed 1 voice
    assert!(
        max_voices <= 1,
        "Cut group pattern should limit to 1 voice, but found {}",
        max_voices
    );
}

#[test]
fn test_cut_group_default_is_zero() {
    // Without cut group parameter, should default to 0 (no cutting)
    // This allows multiple voices to overlap
    let input_with_cut = r#"
        tempo: 0.5
        out $ s("hh hh", "1.0 1.0", "0 0", "1 1", "0 0")
    "#;

    let input_without_cut = r#"
        tempo: 0.5
        out $ s "hh hh"
    "#;

    // With explicit cut group 0
    let (_, statements) = parse_dsl(input_with_cut).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_with = compiler.compile(statements);

    let mut max_voices_with = 0;
    for _ in 0..22050 {
        let _ = graph_with.process_sample();
        let voices = graph_with.active_voice_count();
        if voices > max_voices_with {
            max_voices_with = voices;
        }
    }

    // Without cut group parameter
    let (_, statements) = parse_dsl(input_without_cut).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph_without = compiler.compile(statements);

    let mut max_voices_without = 0;
    for _ in 0..22050 {
        let _ = graph_without.process_sample();
        let voices = graph_without.active_voice_count();
        if voices > max_voices_without {
            max_voices_without = voices;
        }
    }

    println!("Max voices with cut=0: {}", max_voices_with);
    println!("Max voices without cut param: {}", max_voices_without);

    // Both should allow overlapping voices (>= 2)
    assert!(
        max_voices_with >= 2,
        "Explicit cut=0 should allow overlap, found {}",
        max_voices_with
    );
    assert!(
        max_voices_without >= 2,
        "Default (no cut param) should allow overlap, found {}",
        max_voices_without
    );
}

#[test]
fn test_hi_hat_open_close_simulation() {
    // Realistic hi-hat scenario: closed hits stop open hits
    // hh:0 = open, hh:1 = closed (both in cut group 1)
    let input = r#"
        tempo: 0.5
        out $ s("hh:0 hh:1 hh:0 hh:1", "1 1 1 1", "0 0 0 0", "1 1 1 1", "1 1 1 1")
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);

    // Verify audio is produced
    let rms = calculate_rms(&buffer);
    println!("Hi-hat open/close RMS: {:.4}", rms);
    assert!(rms > 0.01, "Should have audio");

    // Track voice count
    let (_, statements) = parse_dsl(input).expect("Failed to parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let mut max_voices = 0;
    for _ in 0..22050 {
        let _ = graph.process_sample();
        let voices = graph.active_voice_count();
        if voices > max_voices {
            max_voices = voices;
        }
    }

    println!("Max voices in hi-hat simulation: {}", max_voices);

    // Should never exceed 1 voice (each new hit stops previous)
    assert!(
        max_voices <= 1,
        "Hi-hat cut group should keep max 1 voice, found {}",
        max_voices
    );
}

/// Helper function to calculate RMS
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}
