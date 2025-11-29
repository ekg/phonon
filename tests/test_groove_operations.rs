/// Test groove operations: swing, shuffle
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_swing_transform() {
    // swing should add swing/shuffle feel to events
    let input = r#"
        cps: 1.0
        out $ s "bd sn hh cp" $ swing 0.5 * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Swing transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_shuffle_transform() {
    // shuffle should shuffle pattern by n
    let input = r#"
        cps: 1.0
        out $ s "bd sn hh cp" $ shuffle 2 * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(88200); // Render 2 seconds to catch delayed events

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.0003,
        "Shuffle transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_swing_at_pattern_level() {
    // Test swing directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let swung = pattern.swing(Pattern::pure(0.5)); // Add 50% swing

    // Query over 1 cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = swung.query(&state);

    println!("\nSwing pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Swing should preserve event count but adjust timing
    assert!(
        events.len() >= 3 && events.len() <= 5,
        "Swing should have 3-5 events, got {}",
        events.len()
    );
}

#[test]
fn test_shuffle_at_pattern_level() {
    // Test shuffle directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let shuffled = pattern.shuffle(Pattern::pure(3.0)); // Shuffle by 3

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = shuffled.query(&state);

    println!("\nShuffle pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Shuffle should preserve event count
    assert!(
        events.len() >= 3 && events.len() <= 5,
        "Shuffle should have 3-5 events, got {}",
        events.len()
    );
}

#[test]
fn test_swing_with_chained_transforms() {
    // swing should work with other transforms
    let input = r#"
        cps: 1.0
        out $ s "bd sn hh cp" $ swing 0.5 $ fast 2 * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Swing with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_shuffle_with_chained_transforms() {
    // shuffle should work with other transforms
    let input = r#"
        cps: 1.0
        out $ s "bd sn hh cp" $ shuffle 2 $ rev * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.0003,
        "Shuffle with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}
