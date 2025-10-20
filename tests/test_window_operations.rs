/// Test time window operations: zoom, focus, within
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_zoom_transform() {
    // zoom should focus on a portion of the pattern cycle
    // zoom 0.0 0.5 focuses on first half of pattern
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ zoom 0.0 0.5) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Zoom transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_focus_transform() {
    // focus should zoom to a specific section
    // focus 0.25 0.75 focuses on middle half
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ focus 0.25 0.75) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Focus transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_within_transform() {
    // within should apply a transform to a time window
    // within 0.25 0.75 (fast 2) applies fast(2) to middle half
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ within 0.25 0.75 (fast 2)) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Within transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_zoom_at_pattern_level() {
    // Test zoom directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let zoomed = pattern.zoom(0.0, 0.5); // Focus on first half

    // Query over 1 cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = zoomed.query(&state);

    println!("\\nZoom pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Zooming to first half should give us only first 2 events (bd, sn)
    // stretched across the full cycle
    assert!(
        events.len() >= 1 && events.len() <= 3,
        "Zoom should have 1-3 events, got {}",
        events.len()
    );
}

#[test]
fn test_focus_at_pattern_level() {
    // Test focus directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let focused = pattern.focus(0.25, 0.75); // Focus on middle half

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = focused.query(&state);

    println!("\\nFocus pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Focusing on middle half (0.25-0.75) should give us events from that region
    assert!(
        events.len() >= 1 && events.len() <= 3,
        "Focus should have 1-3 events, got {}",
        events.len()
    );
}

#[test]
fn test_within_at_pattern_level() {
    // Test within directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    // Apply fast(2) to middle half (0.25-0.75)
    let with_transform = pattern.within(0.25, 0.75, |p| p.fast(2.0));

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = with_transform.query(&state);

    println!("\\nWithin pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Within should apply fast(2) only to the middle section
    // Original has 4 events, fast(2) on middle section should add 2 more = ~6 total
    assert!(
        events.len() >= 4 && events.len() <= 8,
        "Within should have 4-8 events, got {}",
        events.len()
    );
}

#[test]
fn test_zoom_with_chained_transforms() {
    // zoom should work with other transforms
    let input = r#"
        cps: 1.0
        out: s("bd sn hh cp" $ zoom 0.0 0.5 $ fast 2) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Zoom with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}
