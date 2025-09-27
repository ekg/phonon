use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

fn main() {
    println!("=== Phonon Pattern Transformations Demo ===\n");
    println!("We have implemented a comprehensive set of pattern transformations");
    println!("inspired by TidalCycles and Strudel.\n");

    // Helper to query one cycle
    let query_cycle = |pattern: Pattern<String>, cycle: i64| {
        let state = State {
            span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
            controls: HashMap::new(),
        };
        pattern.query(&state)
    };

    // Helper to print events
    let print_events = |name: &str, pattern: Pattern<String>| {
        println!("{}:", name);
        let events = query_cycle(pattern, 0);
        for event in events.iter().take(8) {
            let start = event.part.begin.to_float();
            let end = event.part.end.to_float();
            println!("  {} at {:.3}-{:.3}", event.value, start, end);
        }
        if events.len() > 8 {
            println!("  ... ({} events total)", events.len());
        }
        println!();
    };

    // Base pattern
    let base = parse_mini_notation("bd sn hh cp");
    print_events("Base pattern: \"bd sn hh cp\"", base.clone());

    // Time transformations
    println!("TIME TRANSFORMATIONS");
    println!("{}", "-".repeat(40));
    print_events("fast(2)", base.clone().fast(2.0));
    print_events("slow(2)", base.clone().slow(2.0));
    print_events("rev()", base.clone().rev());

    // Repetition
    println!("\nREPETITION & ECHO");
    println!("{}", "-".repeat(40));
    print_events("stutter(2)", base.clone().stutter(2));

    // Show echo effect (would need audio to hear properly)
    println!("echo(3, 0.125, 0.5) - adds echoes with decay");
    let echo_pattern = base.clone().echo(3, 0.125, 0.5);
    let echo_events = query_cycle(echo_pattern, 0);
    println!("  Original + {} echo events\n", echo_events.len() - 4);

    // Probability
    println!("PROBABILITY & RANDOMNESS");
    println!("{}", "-".repeat(40));
    println!("degrade() - randomly drops ~50% of events");
    println!("degrade_by(0.3) - drops 30% of events");
    println!("sometimes(f) - applies f 50% of the time");
    println!("often(f) - applies f 75% of the time");
    println!("rarely(f) - applies f 25% of the time\n");

    // Combination
    println!("PATTERN COMBINATION");
    println!("{}", "-".repeat(40));

    let pattern1 = parse_mini_notation("bd bd");
    let pattern2 = parse_mini_notation("~ sn");

    print_events("Pattern 1: \"bd bd\"", pattern1.clone());
    print_events("Pattern 2: \"~ sn\"", pattern2.clone());
    print_events(
        "overlay (plays both)",
        pattern1.clone().overlay(pattern2.clone()),
    );

    // Conditional application
    println!("CONDITIONAL APPLICATION");
    println!("{}", "-".repeat(40));

    println!("every(n, f) - Apply transformation every n cycles\n");

    let every_pattern = base.clone().every(2, |p| p.fast(2.0));
    println!("every(2, fast(2)) - doubles speed every 2nd cycle:");
    for cycle in 0..4 {
        let events = query_cycle(every_pattern.clone(), cycle);
        println!("  Cycle {}: {} events", cycle, events.len());
    }
    println!();

    // Stereo/Spatial
    println!("STEREO & SPATIAL");
    println!("{}", "-".repeat(40));

    println!("jux(f) - Applies function to right channel only");
    println!("jux(rev) creates stereo with reversed right channel");
    println!("(Returns Pattern<(T, T)> - a tuple pattern)\n");

    // Chaining
    println!("CHAINING TRANSFORMATIONS");
    println!("{}", "-".repeat(40));

    let chained = base.clone().fast(2.0).rev().every(3, |p| p.slow(1.5));

    println!("Pattern: base.fast(2).rev().every(3, slow(1.5))");
    println!("This chains multiple transformations together\n");

    // Summary
    println!("=== AVAILABLE TRANSFORMATIONS ===\n");

    let categories = vec![
        (
            "Time",
            vec!["fast", "slow", "rev", "palindrome", "iter", "early", "late"],
        ),
        ("Repetition", vec!["stutter", "echo", "ply"]),
        ("Combination", vec!["overlay", "stack", "append", "cat"]),
        (
            "Probability",
            vec!["degrade", "sometimes", "often", "rarely", "choose"],
        ),
        ("Conditional", vec!["every", "when", "chunk"]),
        ("Stereo", vec!["jux", "jux_rev", "pan"]),
        ("Structure", vec!["euclid", "struct", "mask", "shuffle"]),
        ("Values", vec!["add", "mul", "range", "segment"]),
    ];

    for (category, functions) in categories {
        println!("{}:", category);
        for func in functions {
            println!("  .{}(...)", func);
        }
        println!();
    }

    println!("=== HOW TO USE ===\n");
    println!("1. Parse a pattern:");
    println!("   let pattern = parse_mini_notation(\"bd sn hh cp\");");
    println!("\n2. Apply transformations (can chain):");
    println!("   let transformed = pattern.fast(2.0).rev().every(4, |p| p.slow(2.0));");
    println!("\n3. Use in DSP context:");
    println!("   Currently transformations work on Pattern<T> objects.");
    println!("   Integration with DSP synthesis chains is in progress.");
}
