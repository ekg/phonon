//! Test parsing and compiling piano.ph file and keyword boundary handling

use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;

#[test]
fn test_parse_piano_ph() {
    let code = std::fs::read_to_string("piano.ph").expect("Failed to read piano.ph");
    println!("File content ({} bytes):", code.len());
    println!("{}", &code);
    println!("\n--- Parsing... ---\n");

    let (rest, statements) = parse_program(&code).expect("Parse failed");

    println!("✅ Parsed {} statements", statements.len());
    for (i, stmt) in statements.iter().enumerate() {
        println!("  {}: {:?}", i, stmt);
    }

    if !rest.trim().is_empty() {
        println!("\n❌ Remaining unparsed ({} chars): {}", rest.len(), rest);
        panic!("Failed to parse entire file, remaining: {}", rest);
    }
}

/// Test parsing the core statements without the workflow comments
#[test]
fn test_parse_piano_core() {
    let code = r#"
tempo: 0.5
~piano $ saw ~midi # adsr 0.01 0.1 0.7 0.3
~warm $ ~piano # lpf 3000 0.6
~verb $ ~warm # reverb 0.4 0.7
out $ ~verb * 0.6
"#;

    println!("Code:\n{}", code);
    println!("\n--- Parsing... ---\n");

    let (rest, statements) = parse_program(&code).expect("Parse failed");

    println!("✅ Parsed {} statements", statements.len());
    for (i, stmt) in statements.iter().enumerate() {
        println!("  {}: {:?}", i, stmt);
    }

    assert_eq!(statements.len(), 5, "Expected 5 statements");
    assert!(rest.trim().is_empty(), "Remaining unparsed: {}", rest);
}

/// Test that keyword transforms like "rev" don't incorrectly match "reverb"
/// This was a bug where `tag("rev")` would match the prefix of "reverb"
#[test]
fn test_keyword_boundary_rev_vs_reverb() {
    // This should work: reverb as a function call
    let code_reverb = "~test $ saw 55 # reverb 0.4 0.7";
    let (rest, stmts) = parse_program(code_reverb).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Reverb should parse completely, got rest: {}", rest);
    assert_eq!(stmts.len(), 1);

    // This should also work: rev as a transform
    let code_rev = "~test $ s \"bd sn\" $ rev";
    let (rest, stmts) = parse_program(code_rev).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Rev should parse completely, got rest: {}", rest);
    assert_eq!(stmts.len(), 1);
}

/// Test other keywords don't match longer identifiers
#[test]
fn test_keyword_boundaries() {
    // "degrade" should not match "degradeBy" or "degradeSeed" prefix
    let code = r#"
~a $ s "bd" $ degradeBy 0.5
~b $ s "bd" $ degradeSeed 42
~c $ s "bd" $ degrade
"#;
    let (rest, stmts) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty(), "All keywords should parse correctly, rest: {}", rest);
    assert_eq!(stmts.len(), 3);
}

/// Test that piano-like code compiles successfully (not just parses)
/// Note: Uses fixed frequency oscillator instead of synth to avoid requiring MIDI hardware
#[test]
fn test_compile_piano_core() {
    // Use fixed frequency instead of synth (which needs MIDI) to test oscillator+filter+reverb chain
    let code = r#"
tempo: 0.5
~piano $ saw 440
~warm $ ~piano # lpf 3000 0.6
~verb $ ~warm # reverb 0.4 0.7
out $ ~verb * 0.6
"#;

    println!("Code:\n{}", code);
    println!("\n--- Parsing... ---\n");

    let (rest, statements) = parse_program(&code).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Remaining unparsed: {}", rest);
    println!("✅ Parsed {} statements", statements.len());

    println!("\n--- Compiling... ---\n");
    match compile_program(statements, 44100.0, None) {
        Ok(_) => println!("✅ Compilation successful!"),
        Err(e) => panic!("❌ Compilation failed: {}", e),
    }
}

/// Test ADSR works with oscillators (saw, sine, etc) not just samples
#[test]
fn test_adsr_with_oscillator() {
    // This should compile - saw oscillator with ADSR envelope
    let code = "~test $ saw 440 # adsr 0.01 0.1 0.7 0.3";

    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty());

    compile_program(statements, 44100.0, None).expect("ADSR should work with oscillators");

    // Sine oscillator should also work
    let code_sine = "~test $ sine 440 # adsr 0.01 0.1 0.7 0.3";
    let (rest, statements) = parse_program(code_sine).expect("Parse failed");
    assert!(rest.trim().is_empty());

    compile_program(statements, 44100.0, None).expect("ADSR should work with sine oscillator");
}
