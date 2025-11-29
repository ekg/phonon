use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_parse_and_compile_dsl() {
    println!("\n=== Testing Unified DSL Parser and Compilation ===");

    let dsl_code = r#"
        ~lfo: sine 0.5 * 0.5 + 0.5
        ~cutoff: ~lfo * 2000 + 500
        ~bass: saw 110 >> lpf(~cutoff, 0.8)
        out $ ~bass * 0.4
    "#;

    // Parse the DSL
    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok(), "DSL should parse successfully");

    let (_, statements) = parse_result.unwrap();
    assert_eq!(statements.len(), 4, "Should have 4 statements");

    // Compile to graph
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render some audio
    let buffer = graph.render(100);
    assert!(buffer.iter().any(|&s| s != 0.0), "Should produce output");

    println!("✓ DSL parses and compiles to working graph");
}

#[test]
fn test_pattern_in_dsl() {
    println!("\n=== Testing Pattern Integration in DSL ===");

    let dsl_code = r#"
        ~rhythm: "1 0 1 0"
        ~osc: sine 220
        ~gated: ~osc * ~rhythm
        cps: 2
        out $ ~gated
    "#;

    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok());

    let (_, statements) = parse_result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render half a second
    let buffer = graph.render(22050);

    // Should have alternating loud/quiet sections
    let quarter = buffer.len() / 4;
    let first_max = buffer[0..quarter]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    let second_max = buffer[quarter..2 * quarter]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    // Pattern should create variation
    assert!(
        (first_max - second_max).abs() > 0.1,
        "Pattern should create variation"
    );

    println!("✓ Patterns work in DSL");
}

#[test]
fn test_complex_modulation_dsl() {
    println!("\n=== Testing Complex Modulation in DSL ===");

    let dsl_code = r#"
        ~mod_freq: sine 0.1 * 2 + 3
        ~mod: sine(~mod_freq) * 100
        ~carrier: sine(440 + ~mod)
        ~env_speed: 4
        ~env: sine(~env_speed) * 0.5 + 0.5
        out $ ~carrier * ~env * 0.3
    "#;

    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok());

    let (_, statements) = parse_result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(4410); // 100ms

    // Check we have modulated output
    assert!(buffer.iter().any(|&s| s != 0.0), "Should produce output");

    // Check for amplitude variation from envelope
    let max_amp = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let min_amp = buffer
        .iter()
        .filter(|&&s| s.abs() > 0.001)
        .map(|s| s.abs())
        .fold(1.0f32, f32::min);

    assert!(max_amp > min_amp * 1.5, "Should have amplitude modulation");

    println!("✓ Complex modulation works in DSL");
}

#[test]
fn test_filter_chain_dsl() {
    println!("\n=== Testing Filter Chain in DSL ===");

    let dsl_code = r#"
        ~noise: saw 100
        ~filtered: ~noise >> lpf 1000 2 >> hpf 200 1
        out $ ~filtered * 0.5
    "#;

    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok());

    println!("✓ Filter chain DSL parses correctly");
}

#[test]
fn test_arithmetic_in_dsl() {
    println!("\n=== Testing Arithmetic Expressions in DSL ===");

    let dsl_code = r#"
        ~a: 440
        ~b: 110
        ~sum: ~a + ~b
        ~product: ~a * 2
        ~complex: (~a + ~b) * 0.5 - 100
        out $ sine(~complex)
    "#;

    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok());

    let (_, statements) = parse_result.unwrap();
    assert_eq!(statements.len(), 6);

    println!("✓ Arithmetic expressions work in DSL");
}

/// Example of the full vision - patterns embedded in synthesis
#[test]
fn test_pattern_driven_fm_synthesis() {
    println!("\n=== Testing Pattern-Driven FM Synthesis ===");

    // This demonstrates the power of the unified system
    let dsl_code = r#"
        cps: 0.5
        ~pitch_pattern: "220 330 440 550"
        ~mod_pattern: "100 200 50 150"
        ~modulator: sine(~mod_pattern)
        ~carrier: sine(~pitch_pattern + ~modulator)
        ~rhythm: "1 1 0 1"
        out $ ~carrier * ~rhythm * 0.3
    "#;

    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok(), "Pattern-driven FM should parse");

    let (_, statements) = parse_result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds (1 cycle at 0.5 cps)
    let buffer = graph.render(88200);

    // Verify output exists
    assert!(buffer.iter().any(|&s| s != 0.0), "Should produce FM output");

    println!("✓ Pattern-driven FM synthesis works!");
}

/// The ultimate test - sidechain compression using patterns
#[test]
fn test_sidechain_with_patterns() {
    println!("\n=== Testing Sidechain Compression with Patterns ===");

    // This is the example from UNIFIED_ARCHITECTURE.md
    let dsl_code = r#"
        cps: 2
        ~kick: "1 0 0 0"
        ~bass_notes: "55 55 82.5 55"
        ~bass: saw(~bass_notes)
        ~sidechain: 1 - (~kick * 0.8)
        ~compressed_bass: ~bass * ~sidechain
        out $ ~compressed_bass * 0.4
    "#;

    let parse_result = parse_dsl(dsl_code);
    assert!(parse_result.is_ok(), "Sidechain DSL should parse");

    println!("✓ Sidechain compression pattern parses!");
    println!("\nThis demonstrates the vision:");
    println!("- Patterns control both rhythm and synthesis");
    println!("- Any signal can modulate any parameter");
    println!("- Everything flows through one unified graph");
}
