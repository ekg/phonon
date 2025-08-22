use phonon::pattern::Pattern;

fn main() {
    println!("Comparing euclidean patterns as mentioned by user:\n");
    
    let p1 = Pattern::<bool>::euclid(1, 8, 0).map(|b| if *b { "x".to_string() } else { "-".to_string() });
    let p3 = Pattern::<bool>::euclid(3, 8, 0).map(|b| if *b { "x".to_string() } else { "-".to_string() });
    
    // Query the patterns for their events
    use phonon::pattern::{State, TimeSpan, Fraction};
    use std::collections::HashMap;
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events1: Vec<_> = p1.query(&state).collect();
    let events3: Vec<_> = p3.query(&state).collect();
    
    println!("bd(1,8): {:?}", events1);
    println!("bd(3,8): {:?}", events3);
    
    println!("\nThese should be DIFFERENT (not the same):");
    if events1 == events3 {
        println!("❌ FAIL: bd(1,8) == bd(3,8) - They are the same!");
    } else {
        println!("✅ PASS: bd(1,8) != bd(3,8) - They are different as expected!");
    }
    
    println!("\nMore euclidean rhythm examples:");
    println!("(2,5) - Khafif-e-ramal: {:?}", Pattern::<bool>::euclid(2, 5, 0));
    println!("(3,4) - Cumbia/Calypso: {:?}", Pattern::<bool>::euclid(3, 4, 0));
    println!("(3,7) - Ruchenitza: {:?}", Pattern::<bool>::euclid(3, 7, 0));
    println!("(3,8) - Cuban tresillo: {:?}", Pattern::<bool>::euclid(3, 8, 0));
    println!("(4,9) - Aksak: {:?}", Pattern::<bool>::euclid(4, 9, 0));
    println!("(5,8) - Cuban cinquillo: {:?}", Pattern::<bool>::euclid(5, 8, 0));
    println!("(5,16) - Bossa Nova: {:?}", Pattern::<bool>::euclid(5, 16, 0));
    println!("(7,12) - West African bell: {:?}", Pattern::<bool>::euclid(7, 12, 0));
}