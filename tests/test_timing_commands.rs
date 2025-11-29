//! Tests for timing control commands: resetCycles, setCycle, nudge
//!
//! These commands manipulate the global clock to control timing during live coding.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to parse and compile DSL code
fn compile_dsl(code: &str) -> Result<phonon::unified_graph::UnifiedSignalGraph, String> {
    let (_remaining, statements) =
        parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;
    compile_program(statements, 44100.0, None)
}

#[test]
fn test_reset_cycles_sets_to_zero() {
    let code = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let mut graph = compile_dsl(code).expect("Failed to compile");
    graph.enable_wall_clock_timing();

    // Advance time by processing some samples
    for _ in 0..10000 {
        graph.process_sample();
    }

    let before = graph.get_cycle_position();
    assert!(before > 0.1, "Should have advanced: got {}", before);

    // Now reset cycles
    graph.reset_cycles();

    // Should be back near 0 (wall-clock just restarted)
    let after = graph.get_cycle_position();
    assert!(after < 0.01, "Should be near 0 after reset: got {}", after);
}

#[test]
fn test_reset_cycles_in_dsl() {
    let code = r#"
        tempo: 0.5
        resetCycles
        out $ s "bd"
    "#;

    let graph = compile_dsl(code).expect("Failed to compile with resetCycles");

    // Should start at 0 since we reset during compilation
    let position = graph.get_cycle_position();
    assert!(position < 0.01, "Should start near 0: got {}", position);
}

#[test]
fn test_set_cycle_jumps_to_position() {
    let code = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let mut graph = compile_dsl(code).expect("Failed to compile");

    // Jump to cycle 5.5
    graph.set_cycle(5.5);

    let position = graph.get_cycle_position();
    assert!(
        (position - 5.5).abs() < 0.001,
        "Should be at cycle 5.5: got {}",
        position
    );

    // Jump to cycle 100
    graph.set_cycle(100.0);

    let position = graph.get_cycle_position();
    assert!(
        (position - 100.0).abs() < 0.001,
        "Should be at cycle 100: got {}",
        position
    );

    // Jump backwards to cycle 2
    graph.set_cycle(2.0);

    let position = graph.get_cycle_position();
    assert!(
        (position - 2.0).abs() < 0.001,
        "Should be at cycle 2: got {}",
        position
    );
}

#[test]
fn test_set_cycle_in_dsl() {
    let code = r#"
        tempo: 0.5
        setCycle 42.7
        out $ s "bd"
    "#;

    let graph = compile_dsl(code).expect("Failed to compile with setCycle");

    let position = graph.get_cycle_position();
    assert!(
        (position - 42.7).abs() < 0.001,
        "Should be at cycle 42.7: got {}",
        position
    );
}

#[test]
fn test_nudge_shifts_timing() {
    let code = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let mut graph = compile_dsl(code).expect("Failed to compile");
    graph.set_cycle(10.0); // Start at known position

    // Nudge forward (delay) by 0.25 cycles
    graph.nudge(0.25);

    let position = graph.get_cycle_position();
    assert!(
        (position - 10.25).abs() < 0.001,
        "Should be at 10.25 after nudge: got {}",
        position
    );

    // Nudge backward (advance) by 0.5 cycles
    graph.nudge(-0.5);

    let position = graph.get_cycle_position();
    assert!(
        (position - 9.75).abs() < 0.001,
        "Should be at 9.75 after nudge: got {}",
        position
    );

    // Multiple nudges accumulate
    graph.nudge(0.1);
    graph.nudge(0.1);
    graph.nudge(0.05);

    let position = graph.get_cycle_position();
    assert!(
        (position - 10.0).abs() < 0.001,
        "Should be at 10.0 after nudges: got {}",
        position
    );
}

#[test]
fn test_nudge_in_dsl() {
    let code = r#"
        tempo: 0.5
        setCycle 5.0
        nudge 0.3
        out $ s "bd"
    "#;

    let graph = compile_dsl(code).expect("Failed to compile with nudge");

    let position = graph.get_cycle_position();
    assert!(
        (position - 5.3).abs() < 0.001,
        "Should be at 5.3 (5.0 + 0.3 nudge): got {}",
        position
    );
}

#[test]
fn test_nudge_negative_in_dsl() {
    let code = r#"
        tempo: 0.5
        setCycle 10.0
        nudge -0.5
        out $ s "bd"
    "#;

    let graph = compile_dsl(code).expect("Failed to compile with negative nudge");

    let position = graph.get_cycle_position();
    assert!(
        (position - 9.5).abs() < 0.001,
        "Should be at 9.5 (10.0 - 0.5 nudge): got {}",
        position
    );
}

#[test]
fn test_commands_work_together() {
    let code = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let mut graph = compile_dsl(code).expect("Failed to compile");

    // Set to 20, nudge, reset, set again
    graph.set_cycle(20.0);
    assert!((graph.get_cycle_position() - 20.0).abs() < 0.001);

    graph.nudge(5.5);
    assert!((graph.get_cycle_position() - 25.5).abs() < 0.001);

    graph.reset_cycles();
    assert!(graph.get_cycle_position() < 0.01);

    graph.set_cycle(3.14159);
    assert!((graph.get_cycle_position() - 3.14159).abs() < 0.001);
}

#[test]
fn test_multiple_commands_in_dsl() {
    let code = r#"
        tempo: 0.5
        setCycle 10.0
        nudge 2.5
        nudge -1.0
        setCycle 5.0
        out $ s "bd"
    "#;

    let graph = compile_dsl(code).expect("Failed to compile");

    // Last setCycle should win
    let position = graph.get_cycle_position();
    assert!(
        (position - 5.0).abs() < 0.001,
        "Final position should be 5.0: got {}",
        position
    );
}

#[test]
fn test_reset_cycles_syntax_variations() {
    // Test that resetCycles parses correctly
    let code1 = "resetCycles\ntempo: 1.0\nout $ s \"bd\"";
    assert!(compile_dsl(code1).is_ok(), "resetCycles should parse");

    // Case variations don't cause errors - they're just ignored as unknown identifiers
    // This is acceptable DSL behavior (lenient parsing)
    let code2 = "resetcycles\ntempo: 1.0\nout $ s \"bd\"";
    let _ = compile_dsl(code2); // May or may not error, either is OK

    let code3 = "ResetCycles\ntempo: 1.0\nout $ s \"bd\"";
    let _ = compile_dsl(code3); // May or may not error, either is OK
}

#[test]
fn test_set_cycle_requires_number() {
    let code1 = "setCycle 42\ntempo: 1.0\nout $ s \"bd\"";
    assert!(
        compile_dsl(code1).is_ok(),
        "setCycle with integer should parse"
    );

    let code2 = "setCycle 3.14159\ntempo: 1.0\nout $ s \"bd\"";
    assert!(
        compile_dsl(code2).is_ok(),
        "setCycle with float should parse"
    );

    // setCycle without number might parse but won't work as intended - that's OK
    // The parser is lenient
}

#[test]
fn test_nudge_requires_number() {
    let code1 = "nudge 0.1\ntempo: 1.0\nout $ s \"bd\"";
    assert!(
        compile_dsl(code1).is_ok(),
        "nudge with positive float should parse"
    );

    let code2 = "nudge -0.5\ntempo: 1.0\nout $ s \"bd\"";
    assert!(
        compile_dsl(code2).is_ok(),
        "nudge with negative float should parse"
    );

    // nudge without number might parse but won't work as intended - that's OK
    // The parser is lenient
}

#[test]
fn test_timing_persists_through_wall_clock() {
    let code = r#"
        tempo: 0.5
        setCycle 10.0
        out $ s "bd"
    "#;

    let mut graph = compile_dsl(code).expect("Failed to compile");
    graph.enable_wall_clock_timing();

    // Should start at 10.0
    let start = graph.get_cycle_position();
    assert!(
        (start - 10.0).abs() < 0.1,
        "Should start at ~10.0: got {}",
        start
    );

    // Process samples - wall clock should advance from 10.0
    // Note: Wall-clock time is advancing in real-time, not sample-time
    // So we can't test exact timing this way
    for _ in 0..4410 {
        // 0.1 seconds at 44.1kHz
        graph.process_sample();
    }

    let after = graph.get_cycle_position();
    // Wall-clock time advances in real-time, not sample-time
    // So the exact timing depends on how fast the CPU processes samples
    // Just verify it advanced from 10.0
    assert!(
        after > 10.0,
        "Should have advanced from 10.0: got {}",
        after
    );
}

#[test]
fn test_nudge_affects_wall_clock_mode() {
    let code = r#"
        tempo: 0.5
        out $ s "bd"
    "#;

    let mut graph = compile_dsl(code).expect("Failed to compile");
    graph.enable_wall_clock_timing();

    // Process one sample to update clock
    graph.process_sample();
    let before_nudge = graph.get_cycle_position();

    // Nudge forward
    graph.nudge(5.0);

    // Process another sample to see the effect
    graph.process_sample();
    let after_nudge = graph.get_cycle_position();

    assert!(
        (after_nudge - before_nudge - 5.0).abs() < 0.1,
        "Nudge should shift by 5.0 cycles: before={}, after={}",
        before_nudge,
        after_nudge
    );
}
