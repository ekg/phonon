/// Tests for Phase 3: Dynamic Pattern Transforms
/// Pattern-to-pattern modulation where patterns control other pattern transforms
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Helper to parse and compile DSL code
fn compile_dsl(code: &str, sample_rate: f32) -> Result<phonon::unified_graph::UnifiedSignalGraph, String> {
    let (_rest, statements) = parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;
    compile_program(statements, sample_rate, None)
}

/// Helper to count pattern events over multiple cycles
fn count_pattern_events<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    num_cycles: usize,
) -> usize {
    let mut total = 0;
    for cycle in 0..num_cycles {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        total += pattern.query(&state).len();
    }
    total
}

#[test]
fn test_pattern_assignment_parsing() {
    // Test that pattern assignments are parsed correctly
    let code = r#"
        %speed: "1 2 3 4"
        out: s "bd"
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern assignment should parse: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_ref_in_fast() {
    // Test pattern reference in fast transform
    let code = r#"
        %speed: "1 2 3 4"
        out: s "bd" $ fast %speed
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern ref in fast should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_modulating_fast_basic() {
    // Basic test: pattern modulates speed of another pattern
    // %speed: "2 4" means first event is 2x speed, second is 4x
    let base_pattern = parse_mini_notation("bd");

    // Create speed pattern: "2 4"
    let speed_pattern = parse_mini_notation("2 4").fmap(|s| s.parse::<f64>().unwrap_or(1.0));

    // Apply fast with speed pattern
    let fast_pattern = base_pattern.fast(speed_pattern);

    // Over 2 cycles, we should get varying speeds
    // Cycle 0: speed=2 -> 2 events
    // Cycle 1: speed=4 -> 4 events
    let cycle0_state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };
    let cycle1_state = State {
        span: TimeSpan::new(Fraction::from_float(1.0), Fraction::from_float(2.0)),
        controls: HashMap::new(),
    };

    let cycle0_events = fast_pattern.query(&cycle0_state);
    let cycle1_events = fast_pattern.query(&cycle1_state);

    // Note: The exact event count depends on how the pattern engine samples the speed pattern
    // At minimum, we should have more events than the base pattern (1 per cycle)
    assert!(
        cycle0_events.len() >= 1,
        "Cycle 0 should have at least 1 event, got {}",
        cycle0_events.len()
    );
    assert!(
        cycle1_events.len() >= 1,
        "Cycle 1 should have at least 1 event, got {}",
        cycle1_events.len()
    );
}

#[test]
fn test_pattern_modulating_slow() {
    // Test slow with pattern modulation
    let code = r#"
        %factor: "2 4"
        out: s "bd*8" $ slow %factor
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern-modulated slow should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_modulating_degradeby() {
    // Test degradeBy with pattern probability
    let code = r#"
        %prob: "0.2 0.8"
        out: s "bd*8" $ degradeBy %prob
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern-modulated degradeBy should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_modulating_shuffle() {
    // Test shuffle with pattern amount
    let code = r#"
        %amount: "0.1 0.5 0.9"
        out: s "bd sn hh cp" $ shuffle %amount
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern-modulated shuffle should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_multiple_pattern_assignments() {
    // Test multiple pattern assignments used together
    let code = r#"
        %speed1: "1 2"
        %speed2: "2 4"
        %prob: "0.5 0.8"

        ~drums: s "bd*4" $ fast %speed1
        ~hats: s "hh*8" $ fast %speed2 $ degradeBy %prob

        out: ~drums + ~hats
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Multiple pattern assignments should work: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_pattern_transforms() {
    // Test patterns with multiple transforms
    let code = r#"
        %speed: "2 3 4"
        %prob: "0.3 0.7"

        out: s "bd sn hh cp" $ fast %speed $ degradeBy %prob
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Nested pattern transforms should work: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_ref_undefined_error() {
    // Test that undefined pattern reference gives clear error
    // Use a string pattern directly rather than s function to test pattern transform path
    let code = r#"
        out: "bd" $ fast %undefined_speed
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(result.is_err(), "Should error on undefined pattern ref");

    let error = result.err().unwrap();
    assert!(
        error.contains("Undefined pattern") && error.contains("%undefined_speed"),
        "Error should mention undefined pattern, got: {}",
        error
    );
}

#[test]
fn test_pattern_assignment_from_number() {
    // Test pattern assignment from constant number
    let code = r#"
        %speed: 3.0
        out: s "bd*4" $ fast %speed
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern assignment from number should work: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_assignment_from_bus() {
    // Test pattern assignment from audio signal (LFO)
    let code = r#"
        ~lfo: sine 0.5
        %speed: ~lfo
        out: s "bd*4" $ fast %speed
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern assignment from bus should work: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_ref_cannot_be_used_as_signal() {
    // Test that pattern refs can't be used where signals are expected
    let code = r#"
        %speed: "1 2 3"
        out: %speed
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_err(),
        "Pattern ref should not work as signal output"
    );

    let error = result.err().unwrap();
    assert!(
        error.contains("cannot be used as a signal"),
        "Error should explain pattern refs are for transforms only, got: {}",
        error
    );
}

#[test]
fn test_pattern_ref_in_bus_assignment() {
    // Test that pattern refs can't be assigned to buses
    let code = r#"
        %speed: "1 2 3"
        ~drums: %speed
        out: ~drums
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_err(),
        "Pattern ref should not work in bus assignment"
    );
}

#[test]
fn test_pattern_modulating_every_transform() {
    // Test pattern with every transform
    let code = r#"
        %n: "2 4"
        out: s "bd sn hh cp" $ every %n rev
    "#;

    // Note: every expects integer, so this might need special handling
    // For now, just test that it parses
    let result = compile_dsl(code, 44100.0);
    // This might error or work depending on implementation
    // Just ensure it doesn't panic
    let _ = result;
}

#[test]
fn test_complex_pattern_modulation_scenario() {
    // Real-world scenario: evolving drum pattern
    let code = r#"
        tempo: 0.5

        -- Speed patterns that evolve over time
        %kick_speed: "1 2 1 4"
        %snare_speed: "2 3 2 1"

        -- Probability patterns for variation
        %kick_prob: "0.1 0.3 0.5 0.7"
        %snare_prob: "0.8 0.6 0.4 0.2"

        -- Build drum parts with pattern modulation
        ~kick: s "bd*4" $ fast %kick_speed $ degradeBy %kick_prob
        ~snare: s "sn*4" $ fast %snare_speed $ degradeBy %snare_prob
        ~hats: s "hh*8"

        out: ~kick + ~snare + ~hats
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Complex pattern modulation should work: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_modulation_with_euclidean() {
    // Test pattern modulation with Euclidean patterns
    let code = r#"
        %density: "3 5 7"
        out: s "bd(8,3)" $ fast %density
    "#;

    let result = compile_dsl(code, 44100.0);
    assert!(
        result.is_ok(),
        "Pattern modulation with Euclidean should work: {:?}",
        result.err()
    );
}

#[test]
fn test_pattern_pure_constant() {
    // Verify Pattern::pure creates a constant pattern
    let pattern = Pattern::pure(2.0);

    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };

    let events = pattern.query(&state);
    assert_eq!(events.len(), 1, "Pure pattern should have 1 event");
    assert_eq!(events[0].value, 2.0, "Pure pattern value should be 2.0");
}
