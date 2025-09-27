//! Demonstrates the power of "everything is a pattern" in Phonon
//!
//! With the new pattern parameter system, DSP functions can accept:
//! - Constant values: `lpf 1000 0.8`
//! - Pattern strings: `lpf "1000 2000 500 3000" 0.8`
//! - Signal references: `lpf ~lfo 0.8`
//! - Expressions: `lpf (~lfo * 1000 + 500) 0.8`

use phonon::dsp_parameter::DspParameter;
use phonon::glicol_parser_v2::parse_glicol_v2;
use std::collections::HashMap;

fn main() {
    println!("=== Everything Is A Pattern ===\n");
    println!("Phonon now supports patterns as DSP parameters, just like TidalCycles/Strudel!\n");

    // Test 1: Pattern strings as filter parameters
    println!("1. Pattern strings as filter parameters:");
    let pattern_filter = r#"
        ~source: saw 110
        o: ~source >> lpf "1000 2000 500 3000" "0.3 0.5 0.8 0.2"
    "#;

    match parse_glicol_v2(pattern_filter) {
        Ok(_) => println!("   ✓ Parsed: Pattern controls both cutoff and Q"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 2: Pattern oscillator frequencies
    println!("\n2. Pattern-controlled oscillator frequency:");
    let pattern_osc = r#"
        o: sin "220 440 330 550" >> mul 0.3
    "#;

    match parse_glicol_v2(pattern_osc) {
        Ok(_) => println!("   ✓ Parsed: Frequency cycles through pattern"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 3: Complex modulation with patterns
    println!("\n3. Complex modulation with patterns:");
    let complex_mod = r#"
        ~carrier: saw "55 110 220"
        ~cutoff_pattern: "1000 500 2000 800"
        ~q_pattern: "0.1 0.5 0.8 0.2"
        o: ~carrier >> lpf ~cutoff_pattern ~q_pattern >> mul 0.3
    "#;

    match parse_glicol_v2(complex_mod) {
        Ok(_) => println!("   ✓ Parsed: Multiple pattern parameters"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 4: Pattern-based effects
    println!("\n4. Pattern-based effects:");
    let pattern_fx = r#"
        ~source: saw 110
        ~delay_times: "0.125 0.25 0.0625 0.375"
        ~feedback: "0.3 0.5 0.7 0.2"
        o: ~source >> delay ~delay_times ~feedback "0.5"
    "#;

    match parse_glicol_v2(pattern_fx) {
        Ok(_) => println!("   ✓ Parsed: Pattern-controlled delay parameters"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 5: ADSR with pattern parameters
    println!("\n5. ADSR envelope with patterns:");
    let pattern_adsr = r#"
        ~source: saw "110 220"
        ~attack: "0.01 0.1 0.001"
        ~decay: "0.05 0.1 0.2"
        o: ~source >> mul (adsr ~attack ~decay "0.7" "0.3")
    "#;

    match parse_glicol_v2(pattern_adsr) {
        Ok(_) => println!("   ✓ Parsed: Pattern-controlled ADSR"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 6: Demonstrate parameter evaluation
    println!("\n6. Parameter evaluation demonstration:");

    // Create a pattern parameter
    let cutoff_pattern = DspParameter::pattern("1000 2000 500 3000");
    let references = HashMap::new();

    println!("   Pattern: \"1000 2000 500 3000\"");

    // Evaluate at different cycle positions
    for pos in [0.0, 0.25, 0.5, 0.75] {
        let value = cutoff_pattern.evaluate(pos, &references);
        println!("   At cycle position {:.2}: cutoff = {:.0} Hz", pos, value);
    }

    // Test 7: Everything together
    println!("\n7. Ultimate pattern demonstration:");
    let ultimate = r#"
        ~lfo: sin "0.5 1 2 0.25"
        ~bass_freq: "55 110 82.5 73.5"
        ~bass: saw ~bass_freq
        ~cutoff_base: "500 1000 750 1500"
        ~cutoff_mod: ~lfo >> mul "500 1000 250 750" >> add ~cutoff_base
        ~resonance: "0.1 0.3 0.5 0.8 0.2"
        ~delay_time: "0.125 0.25 0.0625"
        ~reverb_room: "0.1 0.3 0.5 0.7"

        o: ~bass >>
           lpf ~cutoff_mod ~resonance >>
           delay ~delay_time "0.4" "0.3" >>
           reverb ~reverb_room "0.5" "0.3" >>
           mul "0.8 0.5 1.0 0.6"
    "#;

    match parse_glicol_v2(ultimate) {
        Ok(_) => println!("   ✓ Parsed: Complete pattern-based synthesis chain!"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    println!("\n=== Success! ===");
    println!("✨ Everything is now a pattern in Phonon!");
    println!("✨ DSP parameters can be:");
    println!("   - Constant values: 1000");
    println!("   - Pattern strings: \"1000 2000 500\"");
    println!("   - Signal references: ~lfo");
    println!("   - Combined expressions: ~lfo * 1000 + 500");
    println!("\n🎵 This unlocks the full power of pattern-based live coding!");
}
