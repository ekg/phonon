use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::pattern_ops::*;
use std::collections::HashMap;

fn main() {
    let p = Pattern::from_string("a b c").palindrome();

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(2, 1)),
        controls: HashMap::new(),
    };

    let haps = p.query(&state);
    println!("Total events: {}", haps.len());

    for hap in &haps {
        println!(
            "[{:.3}-{:.3}]: {:?}",
            hap.part.begin.to_float(),
            hap.part.end.to_float(),
            hap.value
        );
    }
}
