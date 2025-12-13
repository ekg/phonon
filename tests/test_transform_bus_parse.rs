use phonon::compositional_parser::parse_program;

#[test]
fn test_off_transform_parse() {
    // Test off transform parsing
    let tests = vec![
        ("~drums $ s \"bd sn\" $ off 0.125 (fast 2)", "off with fast"),
        ("~drums $ s \"bd sn\" $ off 0.5 rev", "off with rev"),
    ];

    for (input, desc) in tests {
        println!("\nTest: {} -> '{}'", desc, input);
        match parse_program(input) {
            Ok((remaining, stmts)) => {
                println!("  Remaining: {:?}", remaining);
                println!("  Statements: {:?}", stmts);
                let remaining_trimmed = remaining.trim();
                assert!(remaining_trimmed.is_empty(),
                    "Unparsed input remaining for '{}': '{}'\nStatements: {:?}", desc, remaining, stmts);
            }
            Err(e) => panic!("  Parse error for '{}': {:?}", desc, e),
        }
    }
}

#[test]
fn test_transform_bus_basic() {
    // Test simple case
    let test1 = "~quick $ fast 2";
    println!("Test 1: '{}'", test1);
    match parse_program(test1) {
        Ok((remaining, stmts)) => {
            println!("  Remaining: {:?}", remaining);
            println!("  Statements: {:?}", stmts);
            assert!(!stmts.is_empty(), "Should have parsed a statement");
        }
        Err(e) => panic!("  Parse error: {:?}", e),
    }
}

#[test]
fn test_output_with_samples() {
    let test2 = "out $ s \"bd sn\"";
    println!("Test 2: '{}'", test2);
    match parse_program(test2) {
        Ok((remaining, stmts)) => {
            println!("  Remaining: {:?}", remaining);
            println!("  Statements: {:?}", stmts);
            assert!(!stmts.is_empty(), "Should have parsed a statement");
        }
        Err(e) => panic!("  Parse error: {:?}", e),
    }
}

#[test]
fn test_existing_bus_syntax() {
    let test3 = "~bass $ s \"bd sn\"";
    println!("Test 3: '{}'", test3);
    match parse_program(test3) {
        Ok((remaining, stmts)) => {
            println!("  Remaining: {:?}", remaining);
            println!("  Statements: {:?}", stmts);
            assert!(!stmts.is_empty(), "Should have parsed a statement");
        }
        Err(e) => panic!("  Parse error: {:?}", e),
    }
}

#[test]
fn test_bus_arithmetic() {
    // Test bus references with arithmetic operations
    let tests = vec![
        ("out $ ~drums", "simple bus ref"),
        ("out $ ~drums + ~kicks", "bus addition"),
        ("out $ ~drums + ~kicks * 0.5", "bus add + multiply"),
    ];

    for (input, desc) in tests {
        println!("\nTest: {} -> '{}'", desc, input);
        match parse_program(input) {
            Ok((remaining, stmts)) => {
                println!("  Remaining: {:?}", remaining);
                println!("  Statements: {:?}", stmts);
                // Check no unparsed content remains
                let remaining_trimmed = remaining.trim();
                assert!(remaining_trimmed.is_empty(),
                    "Unparsed input remaining for '{}': '{}'", desc, remaining);
            }
            Err(e) => panic!("  Parse error for '{}': {:?}", desc, e),
        }
    }
}
