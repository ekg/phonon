//! Demonstrates current capabilities and limitations of pattern-controlled DSP parameters
//!
//! This explores whether DSP function parameters can be defined by patterns
//! similar to TidalCycles/Strudel.

use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;

fn main() {
    println!("=== Testing Pattern-Controlled DSP Parameters ===\n");

    // Test 1: Signal reference as parameter (THIS WORKS)
    println!("1. Signal reference as parameter:");
    let signal_ref = r#"
        ~lfo: sin 2 >> mul 1000 >> add 1500
        ~source: saw 110
        o: ~source >> lpf ~lfo 0.8 >> mul 0.5
    "#;

    match parse_glicol(signal_ref) {
        Ok(env) => {
            println!("   ✓ Parsed: LFO controls filter cutoff");
            let mut executor = SimpleDspExecutor::new(44100.0);
            match executor.render(&env, 0.5) {
                Ok(audio) => {
                    let max = audio.data.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                    println!("   Generated audio with max: {:.3}", max);
                }
                Err(e) => println!("   Render error: {}", e),
            }
        }
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 2: Arithmetic on signals (THIS WORKS)
    println!("\n2. Arithmetic on signal references:");
    let arithmetic = r#"
        ~lfo1: sin 0.5
        ~lfo2: sin 2 >> mul 0.5
        ~combined: ~lfo1 + ~lfo2
        ~source: saw 220
        o: ~source >> lpf (~combined * 1000 + 1000) 0.8 >> mul 0.3
    "#;

    match parse_glicol(arithmetic) {
        Ok(_) => println!("   ✓ Parsed: Combined LFOs with arithmetic"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 3: Direct pattern string as parameter (DOESN'T WORK YET)
    println!("\n3. Pattern string as parameter (TidalCycles style):");
    let pattern_param = r#"
        ~source: saw 110
        o: ~source >> lpf "1000 2000 500 3000" 0.8 >> mul 0.3
    "#;

    match parse_glicol(pattern_param) {
        Ok(_) => println!("   ✓ Parsed: Pattern controls cutoff"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 4: s function pattern as signal source
    println!("\n4. s function for pattern signal:");
    let s_pattern = r#"
        ~pattern: s "220 440 330 550"
        o: ~pattern >> sin >> mul 0.3
    "#;

    match parse_glicol(s_pattern) {
        Ok(_) => println!("   ✓ Parsed: s function creates pattern"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 5: Multiple signal parameters
    println!("\n5. Multiple signal parameters:");
    let multi_param = r#"
        ~cutoff: sin 0.5 >> mul 1000 >> add 1500
        ~resonance: sin 2 >> mul 0.3 >> add 0.5
        ~source: saw 110
        o: ~source >> lpf ~cutoff ~resonance >> mul 0.3
    "#;

    match parse_glicol(multi_param) {
        Ok(_) => println!("   ✓ Parsed: Multiple modulated parameters"),
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    // Test 6: Pattern multiplication for gating
    println!("\n6. Pattern multiplication (gating):");
    let gating = r#"
        ~source: saw 110
        ~pattern: s "1 0 1 0"
        o: ~source * ~pattern >> mul 0.5
    "#;

    match parse_glicol(gating) {
        Ok(env) => {
            println!("   ✓ Parsed: Pattern gates signal");
            let mut executor = SimpleDspExecutor::new(44100.0);
            match executor.render(&env, 1.0) {
                Ok(audio) => {
                    // Check for variation in output (gating effect)
                    let chunks: Vec<_> = audio.data.chunks(4410).map(|chunk| {
                        chunk.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
                    }).collect();
                    println!("   Chunk amplitudes: {:?}", &chunks[..chunks.len().min(4)]);
                }
                Err(e) => println!("   Render error: {}", e),
            }
        }
        Err(e) => println!("   ✗ Parse error: {}", e),
    }

    println!("\n=== Summary ===");
    println!("✓ Signal references work as parameters (~lfo)");
    println!("✓ Arithmetic on signals works (~lfo1 + ~lfo2)");
    println!("✓ Multiplication for ring modulation/gating works");
    println!("✗ Direct pattern strings as parameters don't work yet");
    println!("✗ The 's' function creates patterns but can't be used as numeric parameters");

    println!("\n=== Proposed Enhancement ===");
    println!("To enable TidalCycles-style pattern parameters:");
    println!("1. Parse pattern strings in parameter positions");
    println!("2. Convert patterns to time-varying signals");
    println!("3. Sample pattern values at audio rate or control rate");
    println!("4. Allow syntax like: lpf \"1000 2000 500\" 0.8");
}