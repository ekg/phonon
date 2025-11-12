//! Quick one-liner demos of Phonon pattern capabilities

use phonon::mini_notation::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_signal::*;
use phonon::pattern_tonal::*;
use std::collections::HashMap;

fn main() {
    println!("🎵 Phonon Quick Demos\n");

    // 1. Simple drum pattern
    println!("1. Drum pattern: \"bd sn bd sn\"");
    let drums = parse_mini_notation("bd sn bd sn");
    print_pattern(&drums);

    // 2. Pattern with rests
    println!("\n2. Pattern with rests: \"bd ~ sn ~\"");
    let rests = parse_mini_notation("bd ~ sn ~");
    print_pattern(&rests);

    // 3. Grouped patterns (play faster)
    println!("\n3. Grouped pattern: \"bd [sn sn] bd [sn sn sn]\"");
    let grouped = parse_mini_notation("bd [sn sn] bd [sn sn sn]");
    print_pattern(&grouped);

    // 4. Alternating patterns (different each cycle)
    println!("\n4. Alternating: \"<bd sn cp>\" (shows first 3 cycles)");
    let alt = parse_mini_notation("<bd sn cp>");
    for cycle in 0..3 {
        println!("  Cycle {}: {:?}", cycle, query_cycle(&alt, cycle));
    }

    // 5. Stacked patterns (play together)
    println!("\n5. Stacked patterns:");
    let kick = Pattern::from_string("bd ~ bd ~");
    let snare = Pattern::from_string("~ sn ~ sn");
    let hihat = Pattern::from_string("hh hh hh hh");
    let stacked = Pattern::stack(vec![kick, snare, hihat]);
    print_pattern(&stacked);

    // 6. Musical notes
    println!("\n6. Musical notes: \"c4 e4 g4 c5\"");
    let notes = Pattern::from_string("c4 e4 g4 c5");
    let midi = notes.note(); // Convert to MIDI numbers
    print_pattern_f64(&midi);

    // 7. Pattern transformations
    println!("\n7. Pattern speed transformations:");
    let base = parse_mini_notation("bd sn");
    println!("  Normal: {:?}", base.clone().first_cycle());
    println!("  Fast 2x: {:?}", base.clone().fast(Pattern::pure(2.0)).first_cycle());
    println!(
        "  Slow 2x: {:?}",
        base.clone().slow(Pattern::pure(2.0)).query_arc(0.0, 2.0)
    );

    // 8. Euclidean rhythms
    println!("\n8. Euclidean rhythm (5,8):");
    let euclid =
        Pattern::euclid(5, 8, 0).map(|b| if b { "x".to_string() } else { ".".to_string() });
    print_pattern(&euclid);

    // 9. Random patterns
    println!("\n9. Random choices:");
    let choices = choose(vec!["kick", "snare", "hat", "clap"]);
    println!("  Random drum each cycle: {:?}", choices.first_cycle());

    // 10. Mini-notation operators
    println!("\n10. Operators: \"bd*2 sn/2\"");
    let ops = parse_mini_notation("bd*2 sn/2");
    print_pattern(&ops);
}

fn print_pattern(pattern: &Pattern<String>) {
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let haps = pattern.query(&state);
    println!(
        "  Events: {:?}",
        haps.iter().map(|h| &h.value).collect::<Vec<_>>()
    );
}

fn print_pattern_f64(pattern: &Pattern<f64>) {
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let haps = pattern.query(&state);
    println!(
        "  Values: {:?}",
        haps.iter().map(|h| h.value).collect::<Vec<_>>()
    );
}

fn query_cycle(pattern: &Pattern<String>, cycle: i64) -> Vec<String> {
    let state = State {
        span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
        controls: HashMap::new(),
    };
    pattern.query(&state).into_iter().map(|h| h.value).collect()
}
