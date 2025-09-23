//! Complete Guide to Pattern Modulation in Phonon
//!
//! This demonstrates how to apply pattern transformations in Phonon,
//! equivalent to TidalCycles' `$` operator and method chaining.

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::Pattern;

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║       PATTERN MODULATION IN PHONON                   ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    // ==================================================================
    // PART 1: Basic Pattern Operations with Method Chaining
    // ==================================================================
    println!("═══ 1. METHOD CHAINING (Rust Code) ═══\n");

    println!("In Rust code, you can chain pattern operations directly:");
    println!("```rust");
    println!("let pattern = parse_mini_notation(\"100 200 300 400\")");
    println!("    .fast(2)      // Speed up 2x");
    println!("    .rev()        // Reverse");
    println!("    .degrade();   // Random dropout");
    println!("```\n");

    // Demonstrate the effect
    let basic = parse_mini_notation("100 200 300 400");
    let fast = basic.clone().fast(2);
    let slow = basic.clone().slow(2);
    let reversed = basic.clone().rev();

    println!("Original: \"100 200 300 400\"");
    println!("  → Events in first cycle: [100, 200, 300, 400]");

    println!("\nAfter .fast(2):");
    println!("  → Events in first cycle: [100, 200, 300, 400, 100, 200, 300, 400]");
    println!("  → Pattern plays twice as fast");

    println!("\nAfter .slow(2):");
    println!("  → Events in first cycle: [100, 200]");
    println!("  → Pattern plays at half speed");

    println!("\nAfter .rev():");
    println!("  → Events in first cycle: [400, 300, 200, 100]");
    println!("  → Pattern plays backwards");

    // ==================================================================
    // PART 2: The |> Operator in DSL
    // ==================================================================
    println!("\n═══ 2. THE |> OPERATOR (DSL Syntax) ═══\n");

    println!("In Phonon DSL, use the |> operator (pipe-forward) for pattern operations:");
    println!("This is equivalent to TidalCycles' $ operator but with left-to-right flow.\n");

    println!("TidalCycles:  \"100 200 300 400\" $ fast 2");
    println!("Phonon:       \"100 200 300 400\" |> fast 2");
    println!("              └─ pattern ──────┘ │  └─ operation");
    println!("                                 └──── pipe operator\n");

    // Show various operations
    let examples = vec![
        ("\"bd sn\" |> fast 2", "Speed up pattern 2x"),
        ("\"bd sn\" |> slow 3", "Slow down pattern 3x"),
        ("\"bd sn\" |> rev", "Reverse the pattern"),
        ("\"bd*4\" |> degrade", "Random 50% dropout"),
        ("\"bd*4\" |> degradeBy 0.3", "Random 30% dropout"),
        ("\"0 3 7\" |> rotate 0.25", "Rotate pattern by 1/4 cycle"),
        ("\"bd sn\" |> every 4 rev", "Reverse every 4th cycle"),
        ("\"bd sn\" |> sometimes (fast 2)", "Sometimes speed up"),
        ("\"bd(5,8)\" |> euclid", "Euclidean rhythm"),
    ];

    println!("Common pattern operations:");
    for (code, desc) in &examples {
        println!("  {:35} // {}", code, desc);
    }

    // ==================================================================
    // PART 3: Chaining Multiple Operations
    // ==================================================================
    println!("\n═══ 3. CHAINING MULTIPLE OPERATIONS ═══\n");

    println!("You can chain multiple operations with |>:");
    println!("Each operation is applied left-to-right.\n");

    let chains = vec![
        (
            "\"100 200\" |> fast 2 |> rev",
            vec!["Original: [100, 200]", "After fast 2: [100, 200, 100, 200]", "After rev: [200, 100, 200, 100]"]
        ),
        (
            "\"bd sn\" |> fast 2 |> every 4 rev",
            vec!["Speed up 2x", "Then reverse every 4th cycle"]
        ),
        (
            "\"0 7 3 10\" |> slow 2 |> rotate 0.25",
            vec!["Slow down to half speed", "Then rotate by quarter cycle"]
        ),
    ];

    for (code, steps) in &chains {
        println!("Example: {}", code);
        for step in steps {
            println!("  → {}", step);
        }
        println!();
    }

    // ==================================================================
    // PART 4: Operations in Complex DSL Code
    // ==================================================================
    println!("═══ 4. PATTERN OPS IN COMPLETE DSL CODE ═══\n");

    println!("Pattern operations can be used anywhere in the DSL:");
    println!("```phonon");
    println!("// Drums with pattern operations");
    println!("~drums: \"bd sn [bd bd] sn\" |> fast 2 |> every 4 rev");
    println!("");
    println!("// Apply to pattern parameters");
    println!("o: sin (\"220 440 330\" |> slow 2) >> lpf 1000 0.8");
    println!("");
    println!("// Complex chains");
    println!("~beat: \"bd*4 sn . hh*8\" |> fast 2 >> amp 0.7 >> pan (-0.5)");
    println!("```\n");

    // ==================================================================
    // PART 5: Order of Operations
    // ==================================================================
    println!("═══ 5. ORDER OF OPERATIONS ═══\n");

    println!("The |> operator binds tightly to patterns:");
    println!("Pattern ops (|>) → DSP chains (>>) → Arithmetic (+, *, etc.)\n");

    println!("Examples showing precedence:");
    println!("  \"bd sn\" |> fast 2 >> lpf 1000");
    println!("  └─ Parses as: (\"bd sn\" |> fast 2) >> lpf 1000");
    println!("");
    println!("  \"100 200\" |> slow 2 * 0.5");
    println!("  └─ Parses as: ((\"100 200\" |> slow 2) * 0.5)");
    println!("");
    println!("  ~pattern: \"0 7\" |> fast 2 |> rev >> delay 0.125 0.3");
    println!("  └─ Pattern ops first, then DSP chain\n");

    // ==================================================================
    // PART 6: Comparison with TidalCycles
    // ==================================================================
    println!("═══ 6. TIDALCYCLES → PHONON CONVERSION ═══\n");

    let conversions = vec![
        ("\"bd sn\" $ fast 2", "\"bd sn\" |> fast 2"),
        ("\"bd sn\" $ fast 2 $ rev", "\"bd sn\" |> fast 2 |> rev"),
        ("fast 2 $ \"bd sn\"", "\"bd sn\" |> fast 2"),
        ("every 4 rev $ \"bd sn\"", "\"bd sn\" |> every 4 rev"),
        ("\"0 3 7\" # speed (sine * 2 + 1)", "\"0 3 7\" >> speed (~sine * 2 + 1)"),
        ("stack [\"bd*4\", \"hh*8\"]", "\"bd*4\" | \"hh*8\"  // or use ~buses"),
    ];

    println!("TidalCycles → Phonon syntax conversions:");
    println!("");
    for (tidal, phonon) in &conversions {
        println!("TidalCycles: {}", tidal);
        println!("Phonon:      {}", phonon);
        println!("");
    }

    // ==================================================================
    // PART 7: Available Pattern Operations
    // ==================================================================
    println!("═══ 7. ALL AVAILABLE PATTERN OPERATIONS ═══\n");

    println!("Time operations:");
    println!("  fast N        - Speed up by factor N");
    println!("  slow N        - Slow down by factor N");
    println!("  early N       - Shift earlier by N cycles");
    println!("  late N        - Shift later by N cycles");
    println!("  offset N      - Offset by N cycles");
    println!("  rotate N      - Rotate by N cycles");
    println!("");

    println!("Structural operations:");
    println!("  rev           - Reverse pattern");
    println!("  palindrome    - Forward then backward");
    println!("  iter N        - Iterate shifted copies");
    println!("  chunk N F     - Apply F to chunks");
    println!("  chop N        - Chop into N pieces");
    println!("");

    println!("Probability operations:");
    println!("  degrade       - Random 50% dropout");
    println!("  degradeBy N   - Random N% dropout");
    println!("  sometimes F   - Apply F 50% of cycles");
    println!("  often F       - Apply F 75% of cycles");
    println!("  rarely F      - Apply F 25% of cycles");
    println!("  every N F     - Apply F every N cycles");
    println!("");

    println!("Combination operations:");
    println!("  overlay P     - Overlay with pattern P");
    println!("  append P      - Append pattern P");
    println!("  fastcat [P]   - Concatenate patterns");
    println!("  slowcat [P]   - Alternate patterns");
    println!("  stack [P]     - Stack patterns");

    // ==================================================================
    // PART 8: Real-World Examples
    // ==================================================================
    println!("\n═══ 8. REAL-WORLD EXAMPLES ═══\n");

    println!("Complete drum pattern with operations:");
    println!("```phonon");
    println!("~kick: \"bd . . bd . . bd .\" |> every 8 (slow 2)");
    println!("~snare: \". . sn . . . sn .\" |> rotate 0.125");
    println!("~hats: \"hh*16\" |> degradeBy 0.3 |> pan 0.7");
    println!("~perc: \"cp? rim?\" |> sometimes rev");
    println!("out: ~kick + ~snare + ~hats + ~perc");
    println!("```\n");

    println!("Melodic pattern with transformations:");
    println!("```phonon");
    println!("~melody: \"0 3 7 12\" |> slow 2 |> every 4 rev");
    println!("~bass: \"0 0 12 7\" |> slow 4 |> rotate 0.25");
    println!("~lead: sin (~melody * 100 + 440) >> lpf 2000 0.8");
    println!("~sub: saw (~bass * 55) >> lpf 500 0.9");
    println!("```\n");

    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  Pattern modulation is the heart of live coding!     ║");
    println!("║  Use |> to transform patterns on the fly.            ║");
    println!("╚══════════════════════════════════════════════════════╝");
}