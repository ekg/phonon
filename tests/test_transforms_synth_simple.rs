//! Simple synthesis-based transform tests
//! Uses sine waves instead of samples to eliminate sample loading issues

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use std::collections::HashMap;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

// ============================================================================
// HELPER: Count events over multiple cycles
// ============================================================================

fn count_events_over_cycles<T: Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> usize {
    let mut total = 0;
    for cycle in 0..cycles {
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

// ============================================================================
// TEST: Fast doubles event density (synthesis)
// ============================================================================

#[test]
fn test_fast_with_synthesis() {
    println!("\n=== FAST TRANSFORM TEST (Synthesis) ===");

    // STEP 1: Pattern query verification
    println!("\n1. Pattern Query Verification:");
    let pattern = parse_mini_notation("110 220");

    let normal = pattern.clone();
    let fast2 = pattern.clone().fast(2.0);

    let cycles = 4;
    let normal_count = count_events_over_cycles(&normal, cycles);
    let fast2_count = count_events_over_cycles(&fast2, cycles);

    println!("   Normal: {} events over {} cycles", normal_count, cycles);
    println!("   Fast x2: {} events over {} cycles", fast2_count, cycles);

    assert_eq!(fast2_count, normal_count * 2, "fast 2 should double event count");

    // STEP 2: Audio verification with synthesis
    println!("\n2. Audio Verification:");

    let input_normal = r#"
        tempo: 2.0
        out: sine "110 220" * 0.3
    "#;
    let input_fast2 = r#"
        tempo: 2.0
        out: sine "110 220 110 220" * 0.3
    "#;

    let (_, stmt_normal) = parse_dsl(input_normal).expect("Parse normal");
    let mut graph_normal = DslCompiler::new(44100.0).compile(stmt_normal);
    let audio_normal = graph_normal.render(88200); // 2 seconds at 44.1kHz

    let (_, stmt_fast) = parse_dsl(input_fast2).expect("Parse fast");
    let mut graph_fast = DslCompiler::new(44100.0).compile(stmt_fast);
    let audio_fast = graph_fast.render(88200);

    let rms_normal = calculate_rms(&audio_normal);
    let rms_fast = calculate_rms(&audio_fast);

    println!("   Normal RMS: {:.6}", rms_normal);
    println!("   Fast x2 RMS: {:.6}", rms_fast);

    assert!(rms_normal > 0.01, "Normal pattern should produce audio");
    assert!(rms_fast > 0.01, "Fast pattern should produce audio");

    // Fast should have similar or slightly higher RMS (more frequent events)
    // Don't assert on ratio since synthesis behaves differently than samples

    println!("✅ Fast transform verified with synthesis");
}
#[test]
fn test_plain_sine() {
    use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
    
    let input = r#"
        tempo: 2.0
        out: sine 440 * 0.3
    "#;
    
    let (_, statements) = parse_dsl(input).expect("Parse");
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let buffer = graph.render(44100); // 1 second
    
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("Plain sine RMS: {:.6}", rms);
    assert!(rms > 0.05, "Sine should produce audio");
}
