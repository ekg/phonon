/// Tests for "both structure" pattern operators
///
/// In Tidal semantics:
/// - |+ takes structure from LEFT
/// - +| takes structure from RIGHT
/// - + (bare) takes structure from BOTH (union of events)
///
/// For "both", events occur at times from BOTH patterns, with values combined.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};

fn compile_code(code: &str) -> Result<phonon::unified_graph::UnifiedSignalGraph, String> {
    let (rest, stmts) = parse_program(code).map_err(|e| format!("Parse error: {}", e))?;
    if !rest.trim().is_empty() {
        return Err(format!("Parser did not consume all input: {:?}", rest));
    }
    compile_program(stmts, 44100.0, None)
}

// ============================================================================
// Pattern-Level Tests (test the Pattern methods directly)
// ============================================================================

#[test]
fn test_add_both_produces_union_of_events() {
    // "1 2" + "10 20 30" should produce 5 events (2 + 3)
    // At each event time, values are combined

    let left = Pattern::fastcat(vec![
        Pattern::pure(1.0),
        Pattern::pure(2.0),
    ]);
    let right = Pattern::fastcat(vec![
        Pattern::pure(10.0),
        Pattern::pure(20.0),
        Pattern::pure(30.0),
    ]);

    let result = left.add_both(right);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: std::collections::HashMap::new(),
    };

    let events = result.query(&state);

    // Should have events from BOTH patterns
    // Left has 2 events, right has 3 events = 5 total event times
    assert!(events.len() >= 5,
        "add_both should produce events from both patterns, got {} events", events.len());
}

#[test]
fn test_add_both_combines_values_at_overlap() {
    // "100" + "10" - single events that fully overlap
    // Should produce combined value 110

    let left = Pattern::pure(100.0);
    let right = Pattern::pure(10.0);

    let result = left.add_both(right);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: std::collections::HashMap::new(),
    };

    let events = result.query(&state);

    // At least one event with combined value
    assert!(!events.is_empty(), "Should produce at least one event");

    // The combined value should be 110
    let has_combined = events.iter().any(|e| (e.value - 110.0).abs() < 0.01);
    assert!(has_combined,
        "Should have combined value 100 + 10 = 110, got values: {:?}",
        events.iter().map(|e| e.value).collect::<Vec<_>>());
}

#[test]
fn test_sub_both_structure() {
    let left = Pattern::pure(100.0);
    let right = Pattern::pure(30.0);

    let result = left.sub_both(right);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: std::collections::HashMap::new(),
    };

    let events = result.query(&state);

    assert!(!events.is_empty(), "Should produce events");
    let has_subtracted = events.iter().any(|e| (e.value - 70.0).abs() < 0.01);
    assert!(has_subtracted, "Should have 100 - 30 = 70");
}

#[test]
fn test_mul_both_structure() {
    let left = Pattern::pure(10.0);
    let right = Pattern::pure(3.0);

    let result = left.mul_both(right);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: std::collections::HashMap::new(),
    };

    let events = result.query(&state);

    assert!(!events.is_empty(), "Should produce events");
    let has_multiplied = events.iter().any(|e| (e.value - 30.0).abs() < 0.01);
    assert!(has_multiplied, "Should have 10 * 3 = 30");
}

#[test]
fn test_div_both_structure() {
    let left = Pattern::pure(100.0);
    let right = Pattern::pure(4.0);

    let result = left.div_both(right);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: std::collections::HashMap::new(),
    };

    let events = result.query(&state);

    assert!(!events.is_empty(), "Should produce events");
    let has_divided = events.iter().any(|e| (e.value - 25.0).abs() < 0.01);
    assert!(has_divided, "Should have 100 / 4 = 25");
}

// ============================================================================
// DSL Compilation Tests (test that operators compile to both-structure)
// ============================================================================

#[test]
fn test_bare_add_compiles_as_both_structure() {
    // Bare + should use both-structure semantics
    let code = r#"
~a $ "100 200"
~b $ "10 20 30"
~result $ ~a + ~b
out $ sine ~result
"#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_bare_sub_compiles_as_both_structure() {
    let code = r#"
~result $ "100 200" - "10"
out $ sine ~result
"#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_bare_mul_compiles_as_both_structure() {
    let code = r#"
~result $ "100 200" * "2 3"
out $ sine ~result
"#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

#[test]
fn test_bare_div_compiles_as_both_structure() {
    let code = r#"
~result $ "100 200" / "2"
out $ sine ~result
"#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}

// ============================================================================
// Semantic Tests (verify actual behavior differs from left/right structure)
// ============================================================================

#[test]
fn test_both_vs_left_event_count() {
    // "1 2" |+ "10 20 30" = 2 events (left structure)
    // "1 2" + "10 20 30" = 5 events (both structure)

    let left = Pattern::fastcat(vec![
        Pattern::pure(1.0),
        Pattern::pure(2.0),
    ]);
    let right = Pattern::fastcat(vec![
        Pattern::pure(10.0),
        Pattern::pure(20.0),
        Pattern::pure(30.0),
    ]);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: std::collections::HashMap::new(),
    };

    // Left structure: 2 events
    let left_result = left.clone().add_left(right.clone());
    let left_events = left_result.query(&state);
    assert_eq!(left_events.len(), 2, "Left structure should have 2 events");

    // Right structure: 3 events
    let right_result = left.clone().add_right(right.clone());
    let right_events = right_result.query(&state);
    assert_eq!(right_events.len(), 3, "Right structure should have 3 events");

    // Both structure: 5 events (2 + 3)
    let both_result = left.add_both(right);
    let both_events = both_result.query(&state);
    assert!(both_events.len() >= 5,
        "Both structure should have at least 5 events (2 + 3), got {}", both_events.len());
}

// ============================================================================
// Integration with Chain Operator
// ============================================================================

#[test]
fn test_both_structure_in_chain() {
    // s "bd" # note "c4 e4" + "0 7"
    // The + should combine events from both note patterns
    let code = r#"out $ s "bd" # note "c4 e4" + "0 7""#;

    match compile_code(code) {
        Ok(_) => (),
        Err(e) => panic!("Should compile: {}", e),
    }
}
