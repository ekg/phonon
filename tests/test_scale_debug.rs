use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_scale_parsing_debug() {
    let input = r#"
        out: scale("0", "major", "60")
    "#;

    let result = parse_dsl(input);
    println!("Parse result: {:?}", result);

    match result {
        Ok((remaining, statements)) => {
            println!("Remaining: '{}'", remaining);
            println!("Statements: {:#?}", statements);

            let compiler = DslCompiler::new(44100.0);
            let mut graph = compiler.compile(statements);

            // Process a few samples and see what we get
            for i in 0..5 {
                let sample = graph.process_sample();
                println!("Sample {}: {}", i, sample);
            }
        }
        Err(e) => {
            panic!("Parse failed: {:?}", e);
        }
    }
}

#[test]
fn test_multiline_parsing_debug() {
    // Test 1: Single line (should work)
    let input1 = r#"out: scale("0", "major", "60")"#;
    println!("Test 1 - Single line: {:?}", input1);
    match parse_dsl(input1) {
        Ok((remaining, statements)) => {
            println!("  Remaining: '{}'", remaining);
            println!("  Statements count: {}", statements.len());
            println!("  Statements: {:#?}", statements);
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Test 2: Two lines with cps (currently fails)
    let input2 = "cps: 1.0\nout: scale(\"0\", \"major\", \"60\")";
    println!("\nTest 2 - Two lines: {:?}", input2);
    match parse_dsl(input2) {
        Ok((remaining, statements)) => {
            println!("  Remaining: '{}'", remaining);
            println!("  Statements count: {}", statements.len());
            println!("  Statements: {:#?}", statements);
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Test 3: Two lines, both output statements
    let input3 = "out: sine 440\nout: scale(\"0\", \"major\", \"60\")";
    println!("\nTest 3 - Two output lines: {:?}", input3);
    match parse_dsl(input3) {
        Ok((remaining, statements)) => {
            println!("  Remaining: '{}'", remaining);
            println!("  Statements count: {}", statements.len());
            println!("  Statements: {:#?}", statements);
        }
        Err(e) => println!("  Error: {:?}", e),
    }
}

#[test]
fn test_scale_as_freq_source() {
    let input = r#"
        cps: 1.0
        ~freq: scale("0", "major", "60")
        out: sine(~freq) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    println!("Parsed statements: {:#?}", statements);

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Process samples
    for i in 0..10 {
        let sample = graph.process_sample();
        println!("Sample {}: {}", i, sample);
    }
}
