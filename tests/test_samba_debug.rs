use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

#[test]
fn debug_samba() {
    // Brazilian samba pattern
    let pattern = parse_mini_notation("bd(5,16)");

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);
    println!("bd(5,16) produced {} events:", haps.len());

    for (i, hap) in haps.iter().enumerate() {
        println!(
            "  Event {}: begin={}/{}",
            i, hap.part.begin.numerator, hap.part.begin.denominator
        );
    }
}
