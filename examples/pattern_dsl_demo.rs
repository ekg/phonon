use phonon::pattern_lang_parser::{PatternParser, PatternExpr, TransformOp};

fn main() {
    println!("=== Phonon Pattern DSL Demo ===\n");
    println!("We now support Tidal/Strudel-style pattern transformations!");
    println!("Using >> to chain transformations (like our DSP chains)\n");
    
    // Test cases
    let test_patterns = vec![
        // Basic patterns
        r#"s "bd sn hh cp""#,
        
        // Simple transformations
        r#"s "bd sn" >> fast 2"#,
        r#"s "bd sn" >> rev"#,
        r#"s "bd sn" >> slow 2"#,
        
        // Chained transformations
        r#"s "bd sn" >> fast 2 >> rev"#,
        r#"s "bd sn" >> fast 2 >> rev >> degrade"#,
        
        // Conditional transformations
        r#"s "bd sn" >> every 4 (slow 2)"#,
        r#"s "bd sn" >> sometimes rev"#,
        r#"s "bd sn" >> often (fast 2)"#,
        
        // Complex transformations
        r#"s "bd sn" >> euclid 3 8"#,
        r#"s "bd sn" >> echo 3 0.125 0.5"#,
        r#"s "bd sn" >> stutter 2 >> fast 2"#,
        
        // Value transformations
        r#"s "bd sn" >> pan 0.75"#,
        r#"s "bd sn" >> range 0.2 0.8"#,
        
        // Stack and cat
        r#"stack [s "bd*4", s "hh*8"]"#,
        r#"cat [s "bd sn", s "hh cp"]"#,
    ];
    
    for pattern_str in test_patterns {
        println!("Pattern: {}", pattern_str);
        
        let mut parser = PatternParser::new(pattern_str);
        match parser.parse() {
            Ok(expr) => {
                print_expr(&expr, 1);
            },
            Err(e) => {
                println!("  ERROR: {}", e);
            }
        }
        println!();
    }
    
    println!("=== How Pattern Transformations Work ===\n");
    
    println!("1. PARSING:");
    println!("   The parser recognizes pattern sources and transformation chains");
    println!("   s \"bd sn\" >> fast 2 >> rev");
    println!("   └─source─┘ └──transform──┘\n");
    
    println!("2. AST STRUCTURE:");
    println!("   Transforms are nested, applied right-to-left:");
    println!("   Transform(Rev,");
    println!("     Transform(Fast(2),");
    println!("       MiniNotation(\"bd sn\")))\n");
    
    println!("3. EVALUATION:");
    println!("   Each transform modifies the pattern:");
    println!("   - fast(2): doubles speed");
    println!("   - rev: reverses within cycle");
    println!("   - every(n, f): applies f every n cycles\n");
    
    println!("=== Integration with Phonon DSL ===\n");
    
    println!("CURRENT SYNTAX:");
    println!("  ~kick: sin 60 >> mul 0.5");
    println!("  o: s \"bd sn hh cp\"\n");
    
    println!("WITH TRANSFORMATIONS:");
    println!("  ~kick: sin 60 >> mul 0.5");
    println!("  ~drums: s \"bd sn\" >> fast 2 >> rev");
    println!("  o: ~drums >> lpf 800 0.5\n");
    
    println!("COMPLEX EXAMPLE:");
    println!("  ~bd: s \"bd*4\" >> every 4 (fast 2)");
    println!("  ~sn: s \"~ sn ~ sn\" >> sometimes rev");
    println!("  ~hh: s \"hh*8\" >> degrade >> pan 0.7");
    println!("  o: stack [~bd, ~sn, ~hh] >> reverb 0.2\n");
    
    println!("=== Available Transformations ===\n");
    
    println!("TIME: fast, slow, rev, early, late, hurry");
    println!("STRUCTURE: every, chunk, euclid, segment");
    println!("PROBABILITY: degrade, sometimes, often, rarely");
    println!("REPETITION: stutter, echo, ply");
    println!("STEREO: jux, pan");
    println!("VALUES: add, mul, range");
    println!("ADVANCED: compress, zoom, inside, outside");
}

fn print_expr(expr: &PatternExpr, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    match expr {
        PatternExpr::MiniNotation(s) => {
            println!("{}MiniNotation: \"{}\"", prefix, s);
        },
        PatternExpr::Reference(name) => {
            println!("{}Reference: ~{}", prefix, name);
        },
        PatternExpr::Transform { pattern, op } => {
            println!("{}Transform:", prefix);
            print_op(op, indent + 1);
            println!("{}  Applied to:", prefix);
            print_expr(pattern, indent + 2);
        },
        PatternExpr::Stack(patterns) => {
            println!("{}Stack:", prefix);
            for p in patterns {
                print_expr(p, indent + 1);
            }
        },
        PatternExpr::Cat(patterns) => {
            println!("{}Cat:", prefix);
            for p in patterns {
                print_expr(p, indent + 1);
            }
        },
    }
}

fn print_op(op: &TransformOp, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    match op {
        TransformOp::Fast(n) => println!("{}fast({})", prefix, n),
        TransformOp::Slow(n) => println!("{}slow({})", prefix, n),
        TransformOp::Rev => println!("{}rev", prefix),
        TransformOp::Every(n, f) => {
            println!("{}every {} with:", prefix, n);
            print_op(f, indent + 1);
        },
        TransformOp::Sometimes(f) => {
            println!("{}sometimes:", prefix);
            print_op(f, indent + 1);
        },
        TransformOp::Often(f) => {
            println!("{}often:", prefix);
            print_op(f, indent + 1);
        },
        TransformOp::Rarely(f) => {
            println!("{}rarely:", prefix);
            print_op(f, indent + 1);
        },
        TransformOp::Degrade => println!("{}degrade", prefix),
        TransformOp::DegradeBy(n) => println!("{}degradeBy({})", prefix, n),
        TransformOp::Stutter(n) => println!("{}stutter({})", prefix, n),
        TransformOp::Echo { times, time, feedback } => {
            println!("{}echo({}, {}, {})", prefix, times, time, feedback);
        },
        TransformOp::Euclid { pulses, steps, rotation } => {
            if let Some(rot) = rotation {
                println!("{}euclid({}, {}, {})", prefix, pulses, steps, rot);
            } else {
                println!("{}euclid({}, {})", prefix, pulses, steps);
            }
        },
        TransformOp::Pan(n) => println!("{}pan({})", prefix, n),
        TransformOp::Range(min, max) => println!("{}range({}, {})", prefix, min, max),
        _ => println!("{}{:?}", prefix, op),
    }
}