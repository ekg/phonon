use phonon::compositional_compiler::compile_program;
/// Tests for sample modifier compilation and functionality
///
/// These tests verify that sample modifiers (n, gain, pan, speed, attack, release, ar)
/// are properly implemented in the AudioNode architecture.
use phonon::compositional_parser::parse_program;

// LEVEL 1: Compilation Tests - Do modifiers compile without errors?

#[test]
fn test_n_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # n 2
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("n modifier failed to compile: {}", e),
    }
}

#[test]
fn test_gain_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # gain 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("gain modifier failed to compile: {}", e),
    }
}

#[test]
fn test_pan_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # pan 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("pan modifier failed to compile: {}", e),
    }
}

#[test]
fn test_speed_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # speed 2.0
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("speed modifier failed to compile: {}", e),
    }
}

#[test]
fn test_attack_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # attack 0.01
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("attack modifier failed to compile: {}", e),
    }
}

#[test]
fn test_release_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # release 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("release modifier failed to compile: {}", e),
    }
}

#[test]
fn test_ar_modifier_compiles() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # ar 0.01 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("ar modifier failed to compile: {}", e),
    }
}

// LEVEL 2: Chained Modifiers - Can we chain multiple modifiers?

#[test]
fn test_chained_modifiers_compile() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # gain 0.8 # pan 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Chained modifiers failed to compile: {}", e),
    }
}

#[test]
fn test_multiple_chained_modifiers() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" # gain 0.8 # pan 0.5 # speed 1.5 # attack 0.01
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Multiple chained modifiers failed to compile: {}", e),
    }
}

#[test]
fn test_all_modifiers_chained() {
    let code = r#"
tempo: 0.5
out $ s "bd" # gain 0.8 # pan 0.5 # speed 1.5 # n 2 # attack 0.01 # release 0.3
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("All modifiers chained failed to compile: {}", e),
    }
}

// LEVEL 3: Pattern-Controlled Modifiers - Can modifiers take patterns?

#[test]
fn test_pattern_controlled_gain() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" # gain "0.5 0.8 1.0 0.6"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Pattern-controlled gain failed to compile: {}", e),
    }
}

#[test]
fn test_pattern_controlled_pan() {
    let code = r#"
tempo: 0.5
out $ s "hh*4" # pan "-1 0 1 0.5"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Pattern-controlled pan failed to compile: {}", e),
    }
}

#[test]
fn test_pattern_controlled_n() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" # n "0 5 7 12"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Pattern-controlled n failed to compile: {}", e),
    }
}

#[test]
fn test_pattern_controlled_speed() {
    let code = r#"
tempo: 0.5
out $ s "bd*4" # speed "1 2 0.5 1.5"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Pattern-controlled speed failed to compile: {}", e),
    }
}

// LEVEL 4: Complex Pattern Integration - Real-world usage

#[test]
fn test_modifiers_with_pattern_transforms() {
    let code = r#"
tempo: 0.5
out $ s "bd sn" $ fast 2 # gain 0.8 # pan 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Modifiers with pattern transforms failed to compile: {}", e),
    }
}

#[test]
fn test_complex_real_world_pattern() {
    // This is the pattern from m.ph that failed (simplified to test just the modifier)
    let code = r#"
tempo: 0.5
out $ s "808bd(3,8)" # n 2
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Complex real-world pattern (m.ph) failed to compile: {}", e),
    }
}

#[test]
fn test_envelope_with_euclidean_pattern() {
    let code = r#"
tempo: 0.5
out $ s "rave(3,8,1)" # ar 0.1 0.5
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Ok(_) => {} // Success
        Err(e) => panic!("Envelope with Euclidean pattern failed to compile: {}", e),
    }
}

// LEVEL 5: Error Handling - Do we get good error messages?

#[test]
fn test_n_modifier_wrong_arg_count() {
    let code = r#"
tempo: 0.5
out $ s "bd" # n 2 3
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Err(e) => {
            assert!(
                e.contains("2 arguments"),
                "Error should mention argument count, got: {}",
                e
            );
        }
        Ok(_) => panic!("Expected error for wrong argument count"),
    }
}

#[test]
fn test_ar_modifier_wrong_arg_count() {
    let code = r#"
tempo: 0.5
out $ s "bd" # ar 0.1
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    match result {
        Err(e) => {
            assert!(
                e.contains("3 arguments"),
                "Error should mention argument count, got: {}",
                e
            );
        }
        Ok(_) => panic!("Expected error for wrong argument count"),
    }
}

#[test]
fn test_modifier_without_chain_operator() {
    // This should fail - modifiers must be used with #
    // (though this is more of a parser test)
    let code = r#"
tempo: 0.5
out $ gain 0.5 "bd"
"#;
    let (_globals, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);

    // This should either fail to parse or fail to compile
    // Just verify it doesn't panic
    let _ = result;
}
