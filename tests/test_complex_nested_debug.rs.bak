use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_complex_nested() {
    println!("\n=== Debug Complex Nested Pattern ===");

    // Pattern: [bd sn, hh*2] <cp arpy>
    let pattern = parse_mini_notation("[bd sn, hh*2] <cp arpy>");

    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("\nCycle {}:", cycle);
        for event in &events {
            println!(
                "  {:.3} -> {:.3} : {}",
                event.part.begin.to_float(),
                event.part.end.to_float(),
                event.value
            );
        }
    }
}
