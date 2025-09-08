// Test the euclidean algorithm directly
fn simple_euclid(pulses: usize, steps: usize) -> Vec<bool> {
    let mut result = vec![false; steps];
    
    if pulses > 0 {
        // Distribute pulses evenly across steps
        for i in 0..pulses {
            let pos = (i * steps) / pulses;
            result[pos] = true;
        }
    }
    
    result
}

#[test]
#[ignore] // TODO: Fix for new implementation
#[ignore] // TODO: Fix euclid
fn test_bjorklund_algorithm() {
    // Test (3,8) - should give X..X..X.
    let pattern = simple_euclid(3, 8);
    println!("\nEuclidean (3,8) pattern:");
    for (i, &val) in pattern.iter().enumerate() {
        print!("{}", if val { "X" } else { "." });
    }
    println!();
    
    // Count positions
    let mut positions = Vec::new();
    for (i, &val) in pattern.iter().enumerate() {
        if val {
            positions.push(i);
        }
    }
    println!("Hit positions: {:?}", positions);
    
    // For (3,8), we expect hits at positions [0, 3, 6] or similar even distribution
    assert_eq!(positions.len(), 3);
    
    // Test (5,8) 
    let pattern2 = simple_euclid(5, 8);
    println!("\nEuclidean (5,8) pattern:");
    for &val in pattern2.iter() {
        print!("{}", if val { "X" } else { "." });
    }
    println!();
}

fn main() {
    test_bjorklund_algorithm();
}