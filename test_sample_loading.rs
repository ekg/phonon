use phonon::sample_loader::SampleBank;

fn main() {
    let mut bank = SampleBank::new();
    
    println!("Testing sample loading:");
    
    let samples = ["bd", "sn", "hh", "cp"];
    
    for name in &samples {
        if let Some(data) = bank.get_sample(name) {
            println!("  {} ✓ - {} samples", name, data.len());
        } else {
            println!("  {} ✗ - not loaded", name);
        }
    }
    
    // Also test dirt-specific names
    println!("\nTesting dirt names:");
    let dirt_names = ["bd/BT0A0A7", "sn/ST0T0S0"];
    
    for name in &dirt_names {
        if let Some(data) = bank.get_sample(name) {
            println!("  {} ✓ - {} samples", name, data.len());
        } else {
            println!("  {} ✗ - not loaded", name);
        }
    }
}