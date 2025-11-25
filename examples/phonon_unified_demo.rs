use phonon::phonon_lang::PhononEnv;

fn main() {
    println!("=== Phonon Unified Language Demo ===\n");
    println!("Tempo control + Pattern transformations in one clean syntax\n");

    let sample_rate = 44100.0;

    // Example 1: Basic tempo and patterns
    println!("Example 1: Basic Tempo Control");
    println!("{}", "-".repeat(40));

    let code1 = r#"
        bpm: 120
        ~drums: s "bd sn hh cp"
        o: ~drums
    "#;

    println!("Code:");
    for line in code1.lines() {
        if !line.trim().is_empty() {
            println!("  {}", line.trim());
        }
    }

    let mut env = PhononEnv::new(sample_rate);
    match env.eval(code1) {
        Ok(()) => {
            println!("\n✓ Parsed successfully!");
            println!("  Tempo: 120 BPM (CPS = {})", env.cps);
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }

    // Example 2: Pattern transformations with >>
    println!("\n\nExample 2: Pattern Transformations");
    println!("{}", "-".repeat(40));

    let code2 = r#"
        bpm: 128
        ~drums: s "bd sn" >> fast 2
        ~hats: s "hh*8" >> degrade
        ~bass: s "c2 e2 g2 c3" >> slow 2 >> rev
        o: ~drums
    "#;

    println!("Code:");
    for line in code2.lines() {
        if !line.trim().is_empty() {
            println!("  {}", line.trim());
        }
    }

    let mut env = PhononEnv::new(sample_rate);
    match env.eval(code2) {
        Ok(()) => {
            println!("\n✓ Parsed successfully!");
            println!("  Tempo: 128 BPM (House/Techno tempo)");
            println!("  Using >> operator for transformations");
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }

    // Example 3: Conditional transformations
    println!("\n\nExample 3: Conditional Transformations");
    println!("{}", "-".repeat(40));

    let code3 = r#"
        cps: 0.5
        ~drums: s "bd sn hh cp" >> every 4 (rev)
        ~melody: s "c4 d4 e4 f4" >> every 2 (fast 2)
        o: ~drums
    "#;

    println!("Code:");
    for line in code3.lines() {
        if !line.trim().is_empty() {
            println!("  {}", line.trim());
        }
    }

    let mut env = PhononEnv::new(sample_rate);
    match env.eval(code3) {
        Ok(()) => {
            println!("\n✓ Parsed successfully!");
            println!("  CPS: 0.5 (120 BPM equivalent)");
            println!("  Using 'every' for periodic transformations");
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }

    // Example 4: Complex chains
    println!("\n\nExample 4: Complex Transformation Chains");
    println!("{}", "-".repeat(40));

    let code4 = r#"
        bpm: 140
        ~beat: s "bd cp sn cp" >> fast 2 >> every 8 (rev) >> degrade
        ~melody: s "c4 e4 g4 b4" >> palindrome >> slow 2
        o: ~beat
    "#;

    println!("Code:");
    for line in code4.lines() {
        if !line.trim().is_empty() {
            println!("  {}", line.trim());
        }
    }

    let mut env = PhononEnv::new(sample_rate);
    match env.eval(code4) {
        Ok(()) => {
            println!("\n✓ Parsed successfully!");
            println!("  Tempo: 140 BPM (Dubstep/Trap)");
            println!("  Multiple transformations chained with >>");
        }
        Err(e) => println!("✗ Parse error: {}", e),
    }

    // Show the full power
    println!("\n\n=== Unified Syntax Summary ===");
    println!("{}", "-".repeat(40));

    println!("TEMPO CONTROL:");
    println!("  bpm: 120        // Set BPM");
    println!("  cps: 0.5        // Set cycles per second");
    println!();

    println!("PATTERN TRANSFORMATIONS:");
    println!("  s \"pattern\" >> fast 2");
    println!("  s \"pattern\" >> slow 2");
    println!("  s \"pattern\" >> rev");
    println!("  s \"pattern\" >> palindrome");
    println!("  s \"pattern\" >> degrade");
    println!("  s \"pattern\" >> every 4 (rev)");
    println!();

    println!("CHAINING:");
    println!("  s \"pattern\" >> fast 2 >> rev >> degrade");
    println!();

    println!("SYNTHESIS (future):");
    println!("  ~bass: saw 55 >> lpf 800 0.5 >> mul 0.4");
    println!("  ~kick: sin 60 >> env 0.01 0.2 >> mul 0.5");
    println!();

    println!("OUTPUT:");
    println!("  o: ~drums       // Short form");
    println!("  out: ~drums     // Long form");

    println!("\n=== Design Philosophy ===");
    println!("{}", "-".repeat(40));
    println!("• Unified >> operator for both patterns and synthesis");
    println!("• Clean tempo control with bpm: or cps:");
    println!("• All 169 Tidal/Strudel operators available");
    println!("• Seamless integration of patterns and synthesis");
    println!("• One consistent language for all musical expression");
}
