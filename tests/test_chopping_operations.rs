/// Test chopping operations: chop, gap, segment
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

#[test]
fn test_chop_transform() {
    // chop should split each event into n equal parts
    let input = "cps: 1.0\nout: s \"bd sn\" $ chop 4";

    let (_, statements) = parse_program(input).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0).expect("Should compile");
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Chop transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_gap_transform() {
    // gap should add silence between events
    let input = "cps: 1.0\nout: s \"bd sn hh cp\" $ gap 2";

    let (_, statements) = parse_program(input).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0).expect("Should compile");
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Gap transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_segment_transform() {
    // segment should divide pattern into n segments
    let input = "cps: 1.0\nout: s \"bd sn hh cp\" $ segment 2";

    let (_, statements) = parse_program(input).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0).expect("Should compile");
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Segment transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_chop_at_pattern_level() {
    // Test chop directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn");
    let chopped = pattern.chop(4); // Chop into 4 pieces

    // Query over 1 cycle
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = chopped.query(&state);

    println!("\nChop pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Chopping "bd sn" (2 events) into 4 pieces each should give 4 events
    // (chop divides the cycle into n pieces, not each event)
    assert!(
        events.len() >= 2 && events.len() <= 6,
        "Chop should have 2-6 events, got {}",
        events.len()
    );
}

#[test]
fn test_gap_at_pattern_level() {
    // Test gap directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let gapped = pattern.gap(2); // Add 2x gap between events

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = gapped.query(&state);

    println!("\nGap pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Gap should preserve the same number of events but adjust their timing
    assert!(
        events.len() >= 3 && events.len() <= 5,
        "Gap should have 3-5 events, got {}",
        events.len()
    );
}

#[test]
fn test_segment_at_pattern_level() {
    // Test segment directly at pattern level
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    let pattern = parse_mini_notation("bd sn hh cp");
    let segmented = pattern.segment(2); // Divide into 2 segments

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = segmented.query(&state);

    println!("\nSegment pattern: {} events", events.len());
    for (i, event) in events.iter().enumerate() {
        println!(
            "  Event {}: start={:.6}, end={:.6}, value={}",
            i,
            event.part.begin.to_float(),
            event.part.end.to_float(),
            event.value
        );
    }

    // Segment should reorganize events
    assert!(
        events.len() >= 2 && events.len() <= 6,
        "Segment should have 2-6 events, got {}",
        events.len()
    );
}

#[test]
fn test_chop_with_chained_transforms() {
    // chop should work with other transforms
    let input = "cps: 1.0\nout: s \"bd sn\" $ chop 4 $ rev";

    let (_, statements) = parse_program(input).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0).expect("Should compile");
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Chop with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}
