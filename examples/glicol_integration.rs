//! Example demonstrating Glicol-style DSP syntax with mini-notation patterns

use phonon::glicol_parser::parse_glicol;
use phonon::mini_notation::parse_mini_notation;

fn main() {
    println!("=== Glicol-Style DSP with Mini-Notation ===\n");
    
    // Example 1: Simple sine wave with amplitude modulation
    println!("Example 1: Amplitude Modulation");
    println!("--------------------------------");
    let code1 = r#"
        ~amp: sin 1.0 >> mul 0.3 >> add 0.5
        o: sin 440 >> mul ~amp
    "#;
    println!("Code:\n{}", code1);
    
    match parse_glicol(code1) {
        Ok(env) => {
            println!("✓ Parsed successfully!");
            println!("  Reference chains: {:?}", env.ref_chains.keys().collect::<Vec<_>>());
            if let Some(output) = &env.output_chain {
                println!("  Output chain has {} nodes", output.nodes.len());
            }
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }
    
    println!();
    
    // Example 2: Pattern with effects
    println!("Example 2: Pattern with Effects");
    println!("--------------------------------");
    let code2 = r#"
        o: seq "bd sn bd sn" >> sp >> reverb 0.8 0.5
    "#;
    println!("Code:\n{}", code2);
    
    match parse_glicol(code2) {
        Ok(env) => {
            println!("✓ Parsed successfully!");
            if let Some(output) = &env.output_chain {
                println!("  Output chain nodes:");
                for (i, node) in output.nodes.iter().enumerate() {
                    println!("    {}: {:?}", i, node);
                }
            }
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }
    
    println!();
    
    // Example 3: Complex modular synthesis
    println!("Example 3: Complex Modular Synthesis");
    println!("-------------------------------------");
    let code3 = r#"
        ~lfo: sin 0.5 >> mul 0.5 >> add 0.5
        ~env: sin 0.25 >> mul 0.4 >> add 0.6
        ~bass: saw 55 >> lpf 2000 0.8 >> mul ~env
        ~hats: noise 1 >> hpf 8000 2.0 >> mul 0.2
        o: ~bass >> mul 0.5
    "#;
    println!("Code:\n{}", code3);
    
    match parse_glicol(code3) {
        Ok(env) => {
            println!("✓ Parsed successfully!");
            println!("  Reference chains:");
            for name in env.ref_chains.keys() {
                println!("    ~{}", name);
            }
            if let Some(output) = &env.output_chain {
                println!("  Output chain has {} nodes", output.nodes.len());
            }
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }
    
    println!();
    
    // Example 4: Mini-notation within Glicol
    println!("Example 4: Mini-Notation Integration");
    println!("------------------------------------");
    
    // First parse a mini-notation pattern
    let pattern = parse_mini_notation("<[bd(3,8) sn] hh*4>");
    println!("Mini-notation: <[bd(3,8) sn] hh*4>");
    
    // Get first cycle events
    let events = pattern.first_cycle();
    println!("First cycle events:");
    for event in &events {
        println!("  {} at time {}-{}", 
            event.value, 
            event.part.begin.to_float(), 
            event.part.end.to_float());
    }
    
    println!();
    
    // Now use pattern in Glicol-style DSP
    let code4 = r#"
        ~rhythm: seq "bd(3,8) sn hh*4" >> sp
        ~bass: saw 55 >> lpf 1000 0.8
        o: ~rhythm >> mul 0.8
    "#;
    println!("Glicol with pattern:\n{}", code4);
    
    match parse_glicol(code4) {
        Ok(env) => {
            println!("✓ Parsed successfully!");
            println!("  Created rhythm and bass chains");
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }
    
    println!();
    
    // Example 5: Using ~ in mini-notation for rests
    println!("Example 5: Tilde (~) in Mini-Notation");
    println!("--------------------------------------");
    
    let pattern_with_rest = parse_mini_notation("bd ~ sn ~");
    println!("Pattern: bd ~ sn ~");
    
    let events = pattern_with_rest.first_cycle();
    println!("Events (~ is silence/rest):");
    for event in &events {
        if event.value.is_empty() {
            println!("  [rest] at time {}-{}", 
                event.part.begin.to_float(), 
                event.part.end.to_float());
        } else {
            println!("  {} at time {}-{}", 
                event.value, 
                event.part.begin.to_float(), 
                event.part.end.to_float());
        }
    }
    
    println!("\n=== Integration Complete ===");
    println!("\nThe ~ notation works in both contexts:");
    println!("- In Glicol: ~name refers to a lazy-evaluated reference chain");
    println!("- In mini-notation: ~ represents a rest/silence");
    println!("\nThis allows for powerful combinations like:");
    println!("  ~rhythm: seq \"bd ~ sn ~\" >> sp");
    println!("  ~mod: sin 0.5 >> mul ~rhythm");
    println!("  o: saw 440 >> lpf ~mod*2000+500 0.8");
}