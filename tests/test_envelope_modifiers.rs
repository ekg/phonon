/// Test per-event envelope modifiers with sample patterns
/// Tests the syntax: s "bd sn" # segments "0 1 0" "0.1 0.2"
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_segments_modifier() {
    let dsl = r#"
        tempo: 0.5
        out: s "bd sn" # segments "0 1 0" "0.1 0.2"
    "#;

    let (remaining, statements) = parse_dsl(dsl).expect("Failed to parse DSL");
    assert!(remaining.trim().is_empty(), "Unparsed input: {}", remaining);

    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);

    // If we got here without panicking, parsing and compilation succeeded
}

#[test]
fn test_adsr_modifier() {
    let dsl = r#"
        tempo: 0.5
        out: s "bd sn" # adsr 0.01 0.1 0.5 0.2
    "#;

    let (remaining, statements) = parse_dsl(dsl).expect("Failed to parse DSL");
    assert!(remaining.trim().is_empty(), "Unparsed input: {}", remaining);

    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_curve_modifier() {
    let dsl = r#"
        tempo: 0.5
        out: s "hh*4" # curve 0 1 0.1 2
    "#;

    let (remaining, statements) = parse_dsl(dsl).expect("Failed to parse DSL");
    assert!(remaining.trim().is_empty(), "Unparsed input: {}", remaining);

    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_mixed_envelopes() {
    let dsl = r#"
        tempo: 0.5
        ~drums: s "bd sn" # segments "0 1 0" "0.1 0.2"
        ~bass: s "bd" # adsr 0.01 0.1 0.5 0.3
        out: ~drums * 0.5 + ~bass * 0.5
    "#;

    let (remaining, statements) = parse_dsl(dsl).expect("Failed to parse DSL");
    assert!(remaining.trim().is_empty(), "Unparsed input: {}", remaining);

    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}
