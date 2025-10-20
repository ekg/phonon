/// Test closure-based operations: chunk, jux
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_chunk_transform() {
    // chunk should apply transform to each chunk
    // chunk 4 (rev) divides into 4 chunks and reverses each
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ chunk 4 (rev)) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Chunk transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_chunk_at_pattern_level() {
    // Test chunk directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let chunked = pattern.chunk(4, |p| p.rev()); // Chunk into 4 pieces and reverse each

    // Query over 1 cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = chunked.query(&state);

    println!("\nChunk pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Should still have events
    assert!(
        events.len() >= 2 && events.len() <= 6,
        "Chunk should have 2-6 events, got {}",
        events.len()
    );
}

#[test]
#[ignore] // Jux requires stereo pattern support in DSL
fn test_jux_transform() {
    // jux should create stereo effect
    // jux (rev) plays original on left, reversed on right
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ jux (rev)) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Jux transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_jux_at_pattern_level() {
    // Test jux directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let juxed = pattern.jux(|p| p.rev()); // Original left, reversed right

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = juxed.query(&state);

    println!("\nJux pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value=({}, {})",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value.0,
            event.value.1
        );
    }

    // Jux creates two versions (original + transformed)
    assert!(
        events.len() >= 4 && events.len() <= 10,
        "Jux should have 4-10 events, got {}",
        events.len()
    );
}

#[test]
fn test_chunk_with_chained_transforms() {
    // chunk should work with other transforms
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ chunk 2 (fast 2) $ slow 0.5) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(88200); // 2 seconds for slow

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.0003,
        "Chunk with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
#[ignore] // Jux requires stereo pattern support in DSL
fn test_jux_with_chained_transforms() {
    // jux should work with other transforms
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ jux (fast 2) $ rev) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Jux with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}
