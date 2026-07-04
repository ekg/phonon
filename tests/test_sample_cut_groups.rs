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
    // Dense hi-hat pattern so consecutive voices overlap in time; all in cut
    // group 1. Canonical syntax: `# cut N`. The cut applies a 10ms fade-out to
    // the previous same-group voice (not an instant kill), so at most one fading
    // voice can briefly coexist with the freshly-triggered one => max_voices <= 2.
    // (Without a cut group, hh*8 overlaps up to ~9 voices.)
    let input = r#"
        tempo: 2
        out $ s "hh*8" # cut 1
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

    // With cut groups, overlap is limited to the fading previous voice plus the
    // new voice (10ms fade-out, so <= 2), far below the ~9 voices without a cut group.
    assert!(
        max_voices <= 2,
        "Cut group should limit overlap (<=2 with 10ms fade), but found {} voices",
        max_voices
    );

    // Verify we actually triggered samples
    let rms = calculate_rms(&buffer);
    println!("RMS with cut group: {:.4}", rms);
    assert!(rms > 0.01, "Should have audio");
}

#[test]
fn test_no_cut_group_allows_overlap() {
    // Dense hi-hat pattern with no cut group: all voices should overlap.
    let input = r#"
        tempo: 2
        out $ s "hh*8"
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
    // Alternating cut groups 1 and 2: voices in group 1 cut each other and
    // voices in group 2 cut each other, but a group-1 voice and a group-2 voice
    // can play simultaneously, so overlap still exceeds a single group.
    let input = r#"
        tempo: 2
        out $ s "hh*8" # cut "1 2 1 2 1 2 1 2"
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
    // Every event in the same cut group (1): each new hit fades out the previous
    // same-group voice, so overlap stays at <= 2 (fading + new).
    let input = r#"
        tempo: 2
        out $ s "hh*8" # cut 1
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

    // With all in same cut group, overlap stays at <= 2 (fading + new voice)
    assert!(
        max_voices <= 2,
        "Cut group pattern should limit overlap (<=2 with 10ms fade), but found {}",
        max_voices
    );
}

#[test]
fn test_cut_group_default_is_zero() {
    // Without cut group parameter, should default to 0 (no cutting)
    // This allows multiple voices to overlap
    let input_with_cut = r#"
        tempo: 2
        out $ s "hh*8" # cut 0
    "#;

    let input_without_cut = r#"
        tempo: 2
        out $ s "hh*8"
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
        tempo: 2
        out $ s "hh:0 hh:1 hh:0 hh:1" # cut 1
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

    // Each new hit fades out the previous same-group voice (10ms), so overlap
    // stays at <= 2 (one fading + the new hit).
    assert!(
        max_voices <= 2,
        "Hi-hat cut group should keep overlap <= 2 (10ms fade), found {}",
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
