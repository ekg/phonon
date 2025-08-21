use phonon::pattern::bjorklund;

fn main() {
    println!("Comparing euclidean patterns as mentioned by user:\n");
    
    println!("bd(1,8): {:?}", bjorklund(1, 8));
    println!("bd(3,8): {:?}", bjorklund(3, 8));
    
    println!("\nThese should be DIFFERENT (not the same):");
    if bjorklund(1, 8) == bjorklund(3, 8) {
        println!("❌ FAIL: bd(1,8) == bd(3,8) - They are the same!");
    } else {
        println!("✅ PASS: bd(1,8) != bd(3,8) - They are different as expected!");
    }
    
    println!("\nMore euclidean rhythm examples:");
    println!("(2,5) - Khafif-e-ramal: {:?}", bjorklund(2, 5));
    println!("(3,4) - Cumbia/Calypso: {:?}", bjorklund(3, 4));
    println!("(3,7) - Ruchenitza: {:?}", bjorklund(3, 7));
    println!("(3,8) - Cuban tresillo: {:?}", bjorklund(3, 8));
    println!("(4,9) - Aksak: {:?}", bjorklund(4, 9));
    println!("(5,8) - Cuban cinquillo: {:?}", bjorklund(5, 8));
    println!("(5,16) - Bossa Nova: {:?}", bjorklund(5, 16));
    println!("(7,12) - West African bell: {:?}", bjorklund(7, 12));
}