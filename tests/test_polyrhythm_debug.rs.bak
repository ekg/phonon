use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_polyrhythm_alternation() {
    println!("\n=== Debug Polyrhythm with Alternation ===");

    // Test the alternation part first
    let alt_pattern = parse_mini_notation("<sn cp>*2");

    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = alt_pattern.query(&state);
        println!("\nCycle {} - <sn cp>*2:", cycle);
        for event in &events {
            println!(
                "  {:.3} -> {:.3} : {}",
                event.part.begin.to_float(),
                event.part.end.to_float(),
                event.value
            );
        }

        let sn_count = events.iter().filter(|e| e.value == "sn").count();
        let cp_count = events.iter().filter(|e| e.value == "cp").count();
        println!("  sn: {}, cp: {}", sn_count, cp_count);
    }

    // Now test the full polyrhythm
    let pattern = parse_mini_notation("[bd*3, <sn cp>*2]");

    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        println!("\nCycle {} - [bd*3, <sn cp>*2]:", cycle);

        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        let sn_count = events.iter().filter(|e| e.value == "sn").count();
        let cp_count = events.iter().filter(|e| e.value == "cp").count();

        println!("  bd: {}, sn: {}, cp: {}", bd_count, sn_count, cp_count);
    }
}
