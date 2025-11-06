/// Tests for unit and loop parameters (TidalCycles compatibility)
///
/// unit "r" (default) - rate mode: speed is a rate multiplier
/// unit "c" - cycle mode: speed syncs to cycle duration
///
/// loop 0 (default) - play once
/// loop 1 - loop continuously
///
/// Note: These parameters are now passed as kwargs to the s function.
/// Syntax: s "pattern" unit="r" loop=1
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper to test code compilation and rendering
fn test_code(code: &str, duration_seconds: f32) -> Vec<f32> {
    // Parse
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    // Compile
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0); // 2 cycles per second

    // Render
    let num_samples = (duration_seconds * 44100.0) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

fn find_audio_duration(buffer: &[f32], threshold: f32) -> usize {
    // Find the last point where absolute value exceeds threshold
    for i in (0..buffer.len()).rev() {
        if buffer[i].abs() > threshold {
            return i + 1;
        }
    }
    0
}

#[test]
fn test_unit_r_default_rate_mode() {
    // Default behavior: speed is a rate multiplier
    // This should work the same as current implementation
    let code = r#"
tempo: 2.0
out: s "bd" # speed 2.0
"#;

    let buffer = test_code(code, 1.0);
    let rms = calculate_rms(&buffer);
    println!("unit=r (implicit) RMS: {:.6}", rms);

    assert!(rms > 0.01, "Should produce sound with default rate mode");
}

#[test]
fn test_unit_r_explicit_rate_mode() {
    // Explicit unit "r" should behave same as default
    let code = r#"
tempo: 2.0
out: s "bd" unit="r"
"#;

    let buffer = test_code(code, 1.0);
    let rms = calculate_rms(&buffer);
    println!("unit=r (explicit) RMS: {:.6}", rms);

    assert!(rms > 0.01, "Should produce sound with explicit rate mode");
}

#[test]
fn test_unit_c_cycle_mode() {
    // Cycle mode: speed syncs to cycle timing
    // In this mode, speed represents how many cycles the sample should span
    let code = r#"
tempo: 2.0
out: s "bd" unit="c"
"#;

    let buffer = test_code(code, 1.0);
    let rms = calculate_rms(&buffer);
    println!("unit=c (cycle sync) RMS: {:.6}", rms);

    assert!(rms > 0.01, "Should produce sound with cycle mode");
}

#[test]
fn test_unit_pattern() {
    // unit can be a pattern that changes per event
    let code = r#"
tempo: 2.0
out: s "bd sn" unit="r c"
"#;

    let buffer = test_code(code, 1.0);
    let rms = calculate_rms(&buffer);
    println!("unit pattern RMS: {:.6}", rms);

    assert!(rms > 0.01, "Should produce sound with unit pattern");
}

#[test]
fn test_loop_0_plays_once() {
    // Default: sample plays once
    let code = r#"
tempo: 2.0
out: s "bd" loop=0
"#;

    let buffer = test_code(code, 2.0);
    let threshold = 0.001;
    let duration = find_audio_duration(&buffer, threshold);

    println!(
        "loop=0 duration: {} samples ({:.3}s)",
        duration,
        duration as f32 / 44100.0
    );

    // Should have audio but not fill entire buffer (plays once)
    assert!(duration > 1000, "Should have some audio");
    assert!(duration < 88200, "Should not fill entire 2 second buffer");
}

#[test]
fn test_loop_1_repeats() {
    // loop 1: sample should loop/repeat
    let code = r#"
tempo: 2.0
out: s "bd" loop=1
"#;

    let buffer = test_code(code, 2.0);

    // Analyze first and second halves
    let mid = buffer.len() / 2;
    let first_half_rms = calculate_rms(&buffer[0..mid]);
    let second_half_rms = calculate_rms(&buffer[mid..]);

    println!("loop=1 first half RMS: {:.6}", first_half_rms);
    println!("loop=1 second half RMS: {:.6}", second_half_rms);

    // Both halves should have audio (looping continues)
    assert!(first_half_rms > 0.01, "First half should have audio");
    assert!(
        second_half_rms > 0.01,
        "Second half should have audio (looping)"
    );
}

#[test]
fn test_loop_pattern() {
    // loop can be a pattern that changes per event
    let code = r#"
tempo: 2.0
out: s "bd sn" loop="0 1"
"#;

    let buffer = test_code(code, 1.0);
    let rms = calculate_rms(&buffer);
    println!("loop pattern RMS: {:.6}", rms);

    assert!(rms > 0.01, "Should produce sound with loop pattern");
}

#[test]
fn test_unit_loop_combined() {
    // Test both unit and loop parameters together
    let code = r#"
tempo: 2.0
out: s "bd" unit="c" loop=1
"#;

    let buffer = test_code(code, 1.0);
    let rms = calculate_rms(&buffer);
    println!("unit+loop combined RMS: {:.6}", rms);

    assert!(rms > 0.01, "Should produce sound with both parameters");
}
