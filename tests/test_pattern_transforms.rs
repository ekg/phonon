use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::Pattern;

#[test]
fn test_fast_transform() {
    let pattern = parse_mini_notation("bd sn");
    let fast_pattern = pattern.fast(Pattern::pure(2.0));

    // Pattern should have twice as many events
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = fast_pattern.query(&state);
    assert!(events.len() > 0, "Fast pattern should have events");
}

#[test]
fn test_slow_transform() {
    let pattern = parse_mini_notation("bd sn hh cp");
    let slow_pattern = pattern.slow(Pattern::pure(2.0));

    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = slow_pattern.query(&state);
    assert!(events.len() > 0, "Slow pattern should have events");
}

#[test]
fn test_rev_transform() {
    let pattern = parse_mini_notation("bd sn");
    let rev_pattern = pattern.rev();

    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = rev_pattern.query(&state);
    assert!(events.len() > 0, "Reversed pattern should have events");
}

#[test]
fn test_every_transform() {
    let pattern = parse_mini_notation("bd");
    let every_pattern = pattern.clone().every(4, |p| p.fast(Pattern::pure(2.0)));

    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    // Test cycle 0 (should be transformed - fast)
    let state0 = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let events0 = every_pattern.query(&state0);

    // Test cycle 1 (should be normal)
    let state1 = State {
        span: TimeSpan::new(Fraction::new(1, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };
    let events1 = every_pattern.query(&state1);

    assert!(
        events0.len() > 0,
        "Every pattern should have events in cycle 0"
    );
    assert!(
        events1.len() > 0,
        "Every pattern should have events in cycle 1"
    );
}

// ============================================================================
// DSL Integration Tests
// Test that pattern transforms work through the DSL parser and compiler
// ============================================================================

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

/// Test that fast transform works in DSL with frequency patterns
#[test]
fn test_dsl_fast_transform() {
    let input = r#"
        tempo: 0.5
        out $ sine("110 220" |> fast 2) * 0.2
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 0.5 seconds
    let buffer = graph.render(22050);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Fast transform should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_fast_transform: RMS = {:.6}", rms);
}

/// Test that slow transform works in DSL
#[test]
fn test_dsl_slow_transform() {
    let input = r#"
        tempo: 0.5
        out $ sine("110 220 330 440" |> slow 2) * 0.2
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Slow transform should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_slow_transform: RMS = {:.6}", rms);
}

/// Test that rev transform works in DSL
#[test]
fn test_dsl_rev_transform() {
    let input = r#"
        tempo: 0.5
        out $ sine("110 220 330 440" |> rev) * 0.2
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Rev transform should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_rev_transform: RMS = {:.6}", rms);
}

/// Test chained transforms in DSL
#[test]
fn test_dsl_chained_transforms() {
    let input = r#"
        tempo: 0.5
        out $ sine("110 220" |> fast 2 |> rev) * 0.2
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Chained transforms should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_chained_transforms: RMS = {:.6}", rms);
}

/// Test every transform in DSL
#[test]
fn test_dsl_every_transform() {
    let input = r#"
        tempo: 1.0
        out $ sine("110 220" |> every 2 (fast 2)) * 0.2
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds to hear the alternation
    let buffer = graph.render(88200);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Every transform should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_every_transform: RMS = {:.6}", rms);
}

/// Test fast transform with sample playback
#[test]
fn test_dsl_fast_with_samples() {
    let input = r#"
        tempo: 0.5
        out $ s("bd sn" |> fast 2) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Fast transform with samples should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_fast_with_samples: RMS = {:.6}", rms);
}

/// Test rev transform with sample playback
#[test]
fn test_dsl_rev_with_samples() {
    let input = r#"
        tempo: 0.5
        out $ s("bd sn hh cp" |> rev) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Rev transform with samples should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_rev_with_samples: RMS = {:.6}", rms);
}

/// Test pattern transform with filter modulation
#[test]
fn test_dsl_transform_filter_modulation() {
    let input = r#"
        tempo: 0.5
        out $ saw 55 >> lpf("500 2000" |> fast 2, 0.8) * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Failed to parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Transform on filter modulation should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_transform_filter_modulation: RMS = {:.6}", rms);
}

/// Test precedence: |> should bind tighter than *
#[test]
fn test_dsl_transform_precedence() {
    // This should parse as: (sine("110 220" |> fast 2)) * 0.5
    let input = r#"
        tempo: 0.5
        out $ sine("110 220" |> fast 2) * 0.5
    "#;

    let result = parse_dsl(input);
    assert!(
        result.is_ok(),
        "Transform precedence should parse correctly"
    );

    let (_, statements) = result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(22050);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Precedence test should produce audio, got RMS: {}",
        rms
    );

    println!("✓ test_dsl_transform_precedence: RMS = {:.6}", rms);
}
