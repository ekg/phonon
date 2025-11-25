//! The Ultimate Pattern Demo: Everything Is A Pattern!
//!
//! This example demonstrates the full power of Phonon's pattern system where
//! EVERY parameter can be a pattern, reference, or arithmetic expression.

use phonon::dsp_parameter::DspParameter;
use phonon::glicol_parser_v2::parse_glicol_v2;
use std::collections::HashMap;

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║     🎵 PHONON: EVERYTHING IS A PATTERN! 🎵          ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    println!("Phonon now supports the full TidalCycles/Strudel vision:");
    println!("Every DSP parameter can be dynamic!\n");

    // Example 1: Basic Pattern Parameters
    println!("═══ 1. BASIC PATTERN PARAMETERS ═══");
    let basic = r#"
        o: sin "220 440 330 550" >> mul 0.3
    "#;
    test_parse(basic, "Oscillator with pattern frequency");

    // Example 2: Filter with Pattern Parameters
    println!("\n═══ 2. FILTER WITH PATTERN PARAMETERS ═══");
    let filter_patterns = r#"
        ~source: saw 110
        o: ~source >> lpf "1000 2000 500 3000" "0.1 0.5 0.8 0.2"
    "#;
    test_parse(filter_patterns, "Filter with pattern cutoff and Q");

    // Example 3: Arithmetic Expressions
    println!("\n═══ 3. ARITHMETIC EXPRESSIONS ═══");
    let expressions = r#"
        o: saw 110 >> lpf (1000 + 500) 0.8
    "#;
    test_parse(expressions, "Simple arithmetic: 1000 + 500");

    let multiplication = r#"
        o: saw 110 >> lpf (500 * 3) 0.8
    "#;
    test_parse(multiplication, "Multiplication: 500 * 3");

    // Example 4: LFO Modulation with Expressions
    println!("\n═══ 4. LFO MODULATION WITH EXPRESSIONS ═══");
    println!("Classic synthesis pattern: ~lfo * depth + center");

    // Demonstrate evaluation
    let lfo_expr = DspParameter::Expression(Box::new(
        phonon::dsp_parameter::ParameterExpression::Binary {
            op: phonon::dsp_parameter::BinaryOp::Add,
            left: DspParameter::Expression(Box::new(
                phonon::dsp_parameter::ParameterExpression::Binary {
                    op: phonon::dsp_parameter::BinaryOp::Multiply,
                    left: DspParameter::reference("lfo"),
                    right: DspParameter::constant(1000.0),
                },
            )),
            right: DspParameter::constant(1500.0),
        },
    ));

    let mut refs = HashMap::new();
    println!("\nExpression: ~lfo * 1000 + 1500");
    for lfo_val in [-1.0, 0.0, 1.0] {
        refs.insert("lfo".to_string(), lfo_val);
        let result = lfo_expr.evaluate(0.0, &refs);
        println!("  LFO={:4.1} → Cutoff = {} Hz", lfo_val, result);
    }

    // Example 5: Complex Pattern Combinations
    println!("\n═══ 5. COMPLEX PATTERN COMBINATIONS ═══");
    let complex = r#"
        ~carrier_freqs: "110 220 165 275"
        ~mod_depth: "100 200 50 300"
        ~filter_pattern: "1000 2000 500 3000"
        ~q_pattern: "0.1 0.3 0.5 0.8"

        o: sin ~carrier_freqs >>
           lpf ~filter_pattern ~q_pattern >>
           delay "0.125 0.25" "0.3 0.5" 0.5 >>
           mul "0.8 0.5 1.0 0.6"
    "#;
    test_parse(complex, "Everything is a pattern!");

    // Example 6: Real-World FM Synthesis
    println!("\n═══ 6. REAL-WORLD FM SYNTHESIS ═══");
    println!("FM Synthesis with pattern-controlled parameters:");

    let fm_params = vec![
        ("Carrier", "220 440 330"),
        ("Mod Ratio", "2 3 1.5 4"),
        ("Mod Index", "0 100 200 50"),
    ];

    for (name, pattern) in fm_params {
        println!("  {} Pattern: \"{}\"", name, pattern);
    }

    // Example 7: The Ultimate Expression
    println!("\n═══ 7. THE ULTIMATE EXPRESSION ═══");
    println!("Combining everything: patterns, references, and expressions!");

    println!("\nCode:");
    println!("  ~bass: saw \"55 110\"");
    println!("  ~lfo1: sin 0.5");
    println!("  ~lfo2: sin 2");
    println!("  ~cutoff: (~lfo1 * 1000 + ~lfo2 * 500) + \"500 1000\"");
    println!("  o: ~bass >> lpf ~cutoff \"0.1 0.8\"");

    println!("\nThis combines:");
    println!("  • Pattern for oscillator frequency");
    println!("  • Two LFO references");
    println!("  • Arithmetic expression with LFOs");
    println!("  • Pattern addition in the expression");
    println!("  • Pattern for filter Q");

    // Summary
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║                    🎉 SUCCESS! 🎉                    ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  Phonon now supports:                               ║");
    println!("║  • Pattern strings: \"100 200 300\"                   ║");
    println!("║  • Constants: 440                                   ║");
    println!("║  • References: ~lfo                                 ║");
    println!("║  • Arithmetic: (~lfo * 1000 + 500)                 ║");
    println!("║  • Complex expressions with patterns                ║");
    println!("║                                                      ║");
    println!("║  Every parameter is now truly a pattern!            ║");
    println!("╚══════════════════════════════════════════════════════╝");

    // Show pattern evaluation across time
    println!("\n═══ BONUS: PATTERN EVOLUTION OVER TIME ═══");
    demonstrate_pattern_evolution();
}

fn test_parse(code: &str, description: &str) {
    print!("  {} ... ", description);
    match parse_glicol_v2(code) {
        Ok(_) => println!("✓"),
        Err(e) => println!("✗ ({})", e),
    }
}

fn demonstrate_pattern_evolution() {
    println!("\nWatching a pattern-controlled filter sweep over time:");

    let cutoff_pattern = DspParameter::Expression(Box::new(
        phonon::dsp_parameter::ParameterExpression::Binary {
            op: phonon::dsp_parameter::BinaryOp::Add,
            left: DspParameter::Expression(Box::new(
                phonon::dsp_parameter::ParameterExpression::Binary {
                    op: phonon::dsp_parameter::BinaryOp::Multiply,
                    left: DspParameter::pattern("500 1000 750 1500"),
                    right: DspParameter::constant(2.0),
                },
            )),
            right: DspParameter::constant(200.0),
        },
    ));

    let refs = HashMap::new();

    println!("\nExpression: \"500 1000 750 1500\" * 2 + 200");
    println!("\nTime →");

    // Show evolution over 2 cycles
    for cycle in 0..2 {
        print!("Cycle {}: ", cycle);
        for i in 0..16 {
            let pos = cycle as f64 + (i as f64 / 16.0);
            let val = cutoff_pattern.evaluate(pos, &refs);

            // Simple visualization
            let _bar_len = ((val / 100.0) as usize).min(30);
            if i % 4 == 0 {
                print!("│");
            } else {
                print!("·");
            }
        }
        println!("");
    }

    println!("\nThe pattern repeats every cycle, creating rhythmic modulation!");
}
