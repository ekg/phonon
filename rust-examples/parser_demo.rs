//! Parser demo - shows the DSL parsing in action
//! 
//! Run with: cargo run --example parser_demo

use fermion::enhanced_parser::EnhancedParser;
use fermion::signal_graph::SignalGraph;

fn main() {
    println!("🎵 Phonon Modular Synthesis DSL Parser Demo 🎵\n");
    println!("{}", "=".repeat(60));
    
    demo_basic_parsing();
    demo_arithmetic();
    demo_signal_chains();
    demo_complex_patch();
    
    println!("\n{}", "=".repeat(60));
    println!("✨ Parser Demo Complete!");
}

fn demo_basic_parsing() {
    println!("\n📝 Demo 1: Basic Bus Definitions");
    println!("{}", "-".repeat(40));
    
    let dsl = r#"
~lfo: sine(2)
~osc: saw(440)
~mixed: ~lfo * ~osc
out: ~mixed
"#;
    
    println!("Input DSL:");
    println!("{}", dsl);
    
    let mut parser = EnhancedParser::new(44100.0);
    match parser.parse(dsl) {
        Ok(graph) => {
            println!("✅ Successfully parsed!");
            println!("  Created {} buses", graph.buses.len());
            println!("  Created {} nodes", graph.nodes.len());
            for (bus_id, value) in &graph.buses {
                println!("    Bus {}: initial value = {}", bus_id.0, value);
            }
        }
        Err(e) => println!("❌ Parse error: {}", e),
    }
}

fn demo_arithmetic() {
    println!("\n🔢 Demo 2: Arithmetic Operations");
    println!("{}", "-".repeat(40));
    
    let dsl = r#"
~lfo: sine(0.5) * 0.5 + 0.5
~modulated: 440 + ~lfo * 100
~scaled: ~modulated / 2
out: ~scaled
"#;
    
    println!("Input DSL:");
    println!("{}", dsl);
    
    let mut parser = EnhancedParser::new(44100.0);
    match parser.parse(dsl) {
        Ok(graph) => {
            println!("✅ Successfully parsed arithmetic operations!");
            println!("  Operators supported: + - * /");
            println!("  Precedence: * / before + -");
            println!("  Created {} buses", graph.buses.len());
        }
        Err(e) => println!("❌ Parse error: {}", e),
    }
}

fn demo_signal_chains() {
    println!("\n🔗 Demo 3: Signal Chains");
    println!("{}", "-".repeat(40));
    
    let dsl = r#"
~source: saw(110)
~filtered: ~source >> lpf(1000, 0.7)
~delayed: ~filtered >> delay(0.25)
~reverbed: ~delayed >> reverb(0.8)
out: ~reverbed
"#;
    
    println!("Input DSL:");
    println!("{}", dsl);
    
    let mut parser = EnhancedParser::new(44100.0);
    match parser.parse(dsl) {
        Ok(graph) => {
            println!("✅ Successfully parsed signal chain!");
            println!("  Chain operator: >>");
            println!("  Effects: lpf, delay, reverb");
            println!("  Created {} buses", graph.buses.len());
        }
        Err(e) => println!("❌ Parse error: {}", e),
    }
}

fn demo_complex_patch() {
    println!("\n🎼 Demo 4: Complex Modular Patch");
    println!("{}", "-".repeat(40));
    
    let dsl = r#"
// === LFOs and Control ===
~lfo_slow: sine(0.25) * 0.5 + 0.5
~lfo_fast: sine(6) * 0.3

// === Oscillators ===
~bass: saw(55) >> lpf(~lfo_slow * 2000 + 500, 0.8)
~lead: square(440 + ~lfo_fast * 20)

// === Pattern Integration ===
~kick: "bd ~ ~ bd"
~hats: "hh*8" >> hpf(2000, 0.8)

// === Audio Analysis ===
~bass_rms: ~bass >> rms(0.05)
~bass_transient: ~bass >> transient

// === Cross-Modulation ===
~hats_modulated: ~hats >> gain(1 - ~bass_rms * 0.5)

// === Mix ===
~mix: ~bass * 0.3 + ~lead * 0.2 + ~kick * 0.4 + ~hats_modulated * 0.1
~master: ~mix >> compress(0.3, 4) >> limit(0.95)

out: ~master
"#;
    
    println!("Input DSL (Complex Patch):");
    println!("  • Multiple LFOs");
    println!("  • Oscillators with filters");
    println!("  • Pattern strings");
    println!("  • Audio analysis");
    println!("  • Cross-modulation");
    println!("  • Master processing");
    
    let mut parser = EnhancedParser::new(44100.0);
    match parser.parse(dsl) {
        Ok(graph) => {
            println!("\n✅ Successfully parsed complex patch!");
            println!("  Created {} buses", graph.buses.len());
            println!("  Created {} nodes", graph.nodes.len());
            
            // Show some of the buses
            println!("\n  Sample of created buses:");
            for (i, (bus_id, _)) in graph.buses.iter().enumerate() {
                if i < 5 {
                    println!("    • {}", bus_id.0);
                }
            }
            if graph.buses.len() > 5 {
                println!("    ... and {} more", graph.buses.len() - 5);
            }
            
            println!("\n  Features demonstrated:");
            println!("    ✓ Bus definitions with ~prefix");
            println!("    ✓ Arithmetic operations");
            println!("    ✓ Signal chains with >>");
            println!("    ✓ Pattern strings in quotes");
            println!("    ✓ Function calls with parameters");
            println!("    ✓ Comments with //");
            println!("    ✓ Cross-modulation expressions");
        }
        Err(e) => println!("❌ Parse error: {}", e),
    }
}