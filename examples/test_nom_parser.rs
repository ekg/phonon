use phonon::nom_parser::parse_dsl;
use std::time::Instant;

fn main() {
    println!("=== Nom Parser Test ===\n");
    
    // Test various DSL expressions
    let test_cases = vec![
        // Basic oscillators
        "o: sin 440",
        "o: saw 220 >> mul 0.5",
        
        // Bus references
        "~lfo: sin 0.5\no: ~lfo",
        
        // Arithmetic operations  
        "~lfo: sin 0.5 >> mul 0.5 >> add 0.5\no: saw 55 >> lpf ~lfo * 2000 + 500 0.8",
        
        // Pattern integration
        r#"o: s "bd sn hh cp""#,
        
        // Complex chains
        r#"
            ~lfo: sin 0.5 >> mul 0.5 >> add 0.5
            ~env: sin 2 >> mul 0.3 >> add 0.7
            ~bass: saw 55 >> lpf ~lfo * 2000 + 500 0.8
            ~lead: square 220 >> hpf ~env * 3000 + 1000 0.6
            ~drums: s "bd sn hh cp" >> mul 0.6
            o: ~bass * 0.4 + ~lead * 0.3 + ~drums
        "#,
    ];
    
    for (i, code) in test_cases.iter().enumerate() {
        println!("Test case {}:", i + 1);
        println!("Code: {}", code.lines().take(2).collect::<Vec<_>>().join(" ... "));
        
        match parse_dsl(code) {
            Ok(env) => {
                println!("✓ Parsed successfully");
                println!("  Buses: {:?}", env.ref_chains.keys().collect::<Vec<_>>());
                println!("  Has output: {}", env.output_chain.is_some());
            }
            Err(e) => {
                println!("✗ Parse error: {}", e);
            }
        }
        println!();
    }
    
    // Benchmark parsing speed
    println!("=== Performance Benchmark ===\n");
    
    let complex_code = r#"
        ~lfo1: sin 0.5 >> mul 0.5 >> add 0.5
        ~lfo2: sin 0.25 >> mul 0.3 >> add 0.7
        ~env: sin 2 >> mul 0.3 >> add 0.7
        ~bass: saw 55 >> lpf ~lfo1 * 2000 + 500 0.8
        ~lead: square 220 >> hpf ~env * 3000 + 1000 0.6
        ~pad: saw 110 >> lpf ~lfo2 * 1500 + 800 0.5
        ~drums: s "bd sn hh cp" >> mul 0.6
        ~sub: sin 27.5 >> mul ~lfo1
        o: ~bass * 0.3 + ~lead * 0.2 + ~pad * 0.2 + ~drums * 0.25 + ~sub * 0.05
    "#;
    
    // Warm up
    for _ in 0..100 {
        let _ = parse_dsl(complex_code);
    }
    
    let iterations = 10000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _ = parse_dsl(complex_code).unwrap();
    }
    
    let elapsed = start.elapsed();
    let per_parse = elapsed / iterations;
    
    println!("Parsed complex DSL {} times", iterations);
    println!("Total time: {:?}", elapsed);
    println!("Average per parse: {:?}", per_parse);
    println!("Throughput: {:.0} parses/second", 1.0 / per_parse.as_secs_f64());
    
    // For live coding, we want sub-millisecond parsing
    if per_parse.as_micros() < 1000 {
        println!("\n✓ Performance goal achieved: < 1ms per parse");
    } else {
        println!("\n✗ Performance needs improvement: {:?} per parse", per_parse);
    }
}