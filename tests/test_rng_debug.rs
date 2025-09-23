use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

#[test]
fn test_rng_generation() {
    println!("\n=== Test RNG Generation ===");

    for cycle in 0..3 {
        let mut rng = StdRng::seed_from_u64(cycle);
        println!("\nCycle {} with seed {}:", cycle, cycle);

        for i in 0..4 {
            let val = rng.gen::<f64>();
            let keep = val >= 0.5;
            println!("  Event {}: val={:.3}, keep={}", i, val, keep);
        }
    }
}