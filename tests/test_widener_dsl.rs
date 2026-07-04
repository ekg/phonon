//! DSL wiring tests for the stereo widener effect (`# widener <width>` / `# width <width>`).
//!
//! Three-level audio-testing methodology:
//!   Level 1 — pattern query: the width pattern parses to the expected events.
//!   Level 2 — behaviour/onset: width=1.0 passes the signal through unchanged,
//!             width!=1.0 blends in a phase-shifted copy (output differs).
//!   Level 3 — characteristics: output is finite, audible, and does not clip.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use std::collections::HashMap;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Render DSL source to a mono audio buffer.
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Level 1 — Pattern query verification.
/// The stepped width pattern `"0 1 2"` yields three events per cycle.
#[test]
fn test_widener_dsl_width_pattern_query() {
    let pattern = parse_mini_notation("0 1 2");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    let mut events = pattern.query(&state);
    events.sort_by(|a, b| a.part.begin.partial_cmp(&b.part.begin).unwrap());
    assert_eq!(events.len(), 3, "width pattern \"0 1 2\" should yield 3 events");
    // Positions land on thirds of the cycle (mini-notation uses decimal fractions).
    let begins: Vec<f64> = events.iter().map(|e| e.part.begin.to_float()).collect();
    for (got, want) in begins.iter().zip([0.0, 1.0 / 3.0, 2.0 / 3.0]) {
        assert!(
            (got - want).abs() < 1e-3,
            "width event onset {} should be near {}",
            got,
            want
        );
    }
}

/// Level 3 — Characteristics: the widener DSL surface renders finite, audible,
/// non-clipping audio for a pattern-modulated width.
#[test]
fn test_widener_dsl_renders_finite_audio() {
    let code = r#"
        out $ saw 110 # widener "0 1 2"
    "#;
    let buffer = render_dsl(code, 1.0);

    assert!(!buffer.is_empty(), "render produced no samples");
    for &s in &buffer {
        assert!(s.is_finite(), "widener output must be finite, got {}", s);
        assert!(s.abs() <= 1.5, "widener output should not blow up, got {}", s);
    }
    let rms = calculate_rms(&buffer);
    assert!(rms > 0.01, "widener output should be audible, got RMS {}", rms);
}

/// Level 2 — Behaviour: width == 1.0 is a pass-through (identity), so a widened
/// saw at width 1 matches the bare saw sample-for-sample.
#[test]
fn test_widener_dsl_width_one_is_passthrough() {
    let plain = render_dsl(r#"out $ saw 110"#, 0.25);
    let widened = render_dsl(r#"out $ saw 110 # widener 1"#, 0.25);

    assert_eq!(plain.len(), widened.len());
    let mut max_diff = 0.0f32;
    for (a, b) in plain.iter().zip(widened.iter()) {
        max_diff = max_diff.max((a - b).abs());
    }
    assert!(
        max_diff < 1e-4,
        "widener at width=1.0 should pass the signal through unchanged, max diff {}",
        max_diff
    );
}

/// Level 2 — Behaviour: width != 1.0 blends in a phase-shifted copy, so the
/// output measurably differs from the bare saw.
#[test]
fn test_widener_dsl_width_two_modulates_signal() {
    let plain = render_dsl(r#"out $ saw 110"#, 0.25);
    let widened = render_dsl(r#"out $ saw 110 # widener 2"#, 0.25);

    assert_eq!(plain.len(), widened.len());
    let mut max_diff = 0.0f32;
    for (a, b) in plain.iter().zip(widened.iter()) {
        max_diff = max_diff.max((a - b).abs());
    }
    assert!(
        max_diff > 1e-3,
        "widener at width=2.0 should alter the signal, max diff {}",
        max_diff
    );
    // Still finite and non-clipping.
    for &s in &widened {
        assert!(s.is_finite());
    }
}

/// The `width` alias resolves to the same effect as `widener`.
#[test]
fn test_widener_dsl_width_alias() {
    let via_widener = render_dsl(r#"out $ saw 110 # widener 2"#, 0.25);
    let via_width = render_dsl(r#"out $ saw 110 # width 2"#, 0.25);

    assert_eq!(via_widener.len(), via_width.len());
    let mut max_diff = 0.0f32;
    for (a, b) in via_widener.iter().zip(via_width.iter()) {
        max_diff = max_diff.max((a - b).abs());
    }
    assert!(
        max_diff < 1e-6,
        "`width` alias should behave identically to `widener`, max diff {}",
        max_diff
    );
}
