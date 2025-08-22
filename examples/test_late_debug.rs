use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::pattern_ops::*;
use std::collections::HashMap;

fn main() {
    let p = Pattern::from_string("a b").late(0.25);
    
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let haps = p.query(&state);
    println!("Total events: {}", haps.len());
    
    for hap in &haps {
        println!("[{:.3}-{:.3}]: {:?}", 
            hap.part.begin.to_float(), 
            hap.part.end.to_float(),
            hap.value);
    }
}