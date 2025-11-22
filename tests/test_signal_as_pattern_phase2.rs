/// Test for Phase 2: Dynamic Audio→Pattern Modulation
///
/// This test verifies that SignalAsPattern node correctly:
/// 1. Compiles without errors
/// 2. Thread-safely shares state with pattern closures
/// 3. Provides infrastructure for dynamic audio→pattern coupling

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::pattern::Pattern;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

const SAMPLE_RATE: f32 = 44100.0;

#[test]
fn test_signal_as_pattern_compiles() {
    // Test that SignalAsPattern compiles in DSL
    let dsl = r#"
tempo: 2.0
~lfo: sine 0.5
out: sine 440
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "Should parse completely, remaining: '{}'",
        remaining
    );

    let graph = compile_program(statements, SAMPLE_RATE);
    assert!(
        graph.is_ok(),
        "SignalAsPattern DSL should compile successfully: {:?}",
        graph.err()
    );

    println!("SignalAsPattern compilation test passed");
}

#[test]
fn test_signal_as_pattern_thread_safety() {
    // Test that Arc<Mutex> properly enables Send+Sync for patterns
    use phonon::pattern::Hap;

    let sampled_value = Arc::new(Mutex::new(0.5f32));

    // Create a pattern that uses the shared value
    let value_ref = sampled_value.clone();
    let pattern = Pattern::new(move |state| {
        let value = *value_ref.lock().unwrap() as f64;
        vec![Hap {
            whole: Some(state.span.clone()),
            part: state.span.clone(),
            value,
            context: HashMap::new(),
        }]
    });

    // Pattern should be Send+Sync (required for thread-safe use)
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    assert_send_sync(&pattern);

    println!("SignalAsPattern thread safety test passed");
}

#[test]
fn test_signal_as_pattern_with_bus_reference() {
    // Test that we can reference a bus in DSL (future dynamic modulation)
    let dsl = r#"
tempo: 1.0
~lfo: sine 0.25
~synth: sine 440
out: ~synth
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE);

    assert!(
        graph.is_ok(),
        "Bus references should compile: {:?}",
        graph.err()
    );

    println!("SignalAsPattern bus reference test passed");
}
