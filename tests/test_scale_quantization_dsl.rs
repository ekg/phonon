//! Scale quantization + note names in the compositional DSL
//! (`n "0 2 4" # scale "minor"`, `note "c e g"`, `n "c4 e4 g4"`).
//!
//! Distinct from the legacy `test_scale_quantization.rs` (which exercises the
//! older `unified_graph_parser` `scale("..","..","..")` form); this file covers
//! the modern `compositional_compiler` wiring added for feat-scale-quantization.
//!
//! Three-level audio-testing methodology (see CLAUDE.md):
//! - Level 1: pattern-query — the compiled `n "0 2 4" # scale "minor"` node
//!   yields semitone events [0, 3, 7]; `note "c e g"` yields [0, 4, 7].
//! - Level 2: onset + pitch — `sine (note "c e g")` sounds at the expected
//!   fundamentals (mtof of c4/e4/g4) across its three events.
//! - Level 3: audio characteristics — RMS > 0.01, no NaN; unknown scale/note
//!   names degrade gracefully (no panic).

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::scale_dsl::{note_names_to_semitone_pattern, quantize_degree_pattern};
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use std::collections::HashMap;

const SR: f32 = 44100.0;

// ----- helpers -----------------------------------------------------------

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, SR, None).expect("Failed to compile DSL code");
    let num_samples = (duration * SR) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Estimate the dominant frequency of a (roughly single-tone) segment using the
/// zero-crossing rate. Adequate for verifying a clean sine's pitch.
fn estimate_freq_zero_crossings(segment: &[f32], sample_rate: f32) -> f32 {
    let mut crossings = 0usize;
    for w in segment.windows(2) {
        if (w[0] <= 0.0 && w[1] > 0.0) || (w[0] >= 0.0 && w[1] < 0.0) {
            crossings += 1;
        }
    }
    (crossings as f32 * sample_rate) / (2.0 * segment.len() as f32)
}

/// Query a `Pattern<String>` over one cycle, returning numeric values in order.
fn query_pattern_values(pattern: &phonon::pattern::Pattern<String>) -> Vec<f64> {
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };
    let mut haps = pattern.query(&state);
    haps.sort_by(|a, b| {
        a.part
            .begin
            .to_float()
            .partial_cmp(&b.part.begin.to_float())
            .unwrap()
    });
    haps.iter()
        .map(|h| h.value.trim().parse::<f64>().unwrap())
        .collect()
}

/// Query the compiled Pattern node registered at bus `name`.
fn query_bus_pattern(graph: &UnifiedSignalGraph, name: &str) -> Vec<f64> {
    let node_id = graph
        .get_bus(name)
        .unwrap_or_else(|| panic!("bus ~{name} not found in compiled graph"));
    let node = graph.get_node(node_id).expect("bus node missing");
    match node {
        SignalNode::Pattern { pattern, .. } => query_pattern_values(pattern),
        other => panic!("bus ~{name} is not a Pattern node: {other:?}"),
    }
}

fn mtof(midi: f64) -> f32 {
    (440.0 * 2.0_f64.powf((midi - 69.0) / 12.0)) as f32
}

// ----- Level 1: pattern-query via the compiled DSL -----------------------

#[test]
fn test_level1_n_scale_minor_yields_semitones() {
    // The exact target syntax must compile and its Pattern node must yield the
    // relative semitones [0, 3, 7].  minor = [0,2,3,5,7,8,10]; deg 0/2/4.
    let code = r#"~mel $ n "0 2 4" # scale "minor"
out $ ~mel"#;
    let (_, statements) = parse_program(code).expect("parse");
    let graph = compile_program(statements, SR, None).expect("compile");
    assert_eq!(query_bus_pattern(&graph, "mel"), vec![0.0, 3.0, 7.0]);
}

#[test]
fn test_level1_scale_major_dorian_mixolydian_via_dsl() {
    // Validation: at least major, minor, dorian, mixolydian, pentatonic.
    let code = r#"~major $ n "0 1 2 3 4" # scale "major"
~dorian $ n "0 2 4 5" # scale "dorian"
~mixo $ n "0 3 6" # scale "mixolydian"
~pent $ n "0 1 2 3 4" # scale "pentatonic"
out $ ~major + ~dorian + ~mixo + ~pent"#;
    let (_, statements) = parse_program(code).expect("parse");
    let graph = compile_program(statements, SR, None).expect("compile");
    // major:  [0,2,4,5,7,9,11] deg 0..4 -> [0,2,4,5,7]
    assert_eq!(
        query_bus_pattern(&graph, "major"),
        vec![0.0, 2.0, 4.0, 5.0, 7.0]
    );
    // dorian: [0,2,3,5,7,9,10] deg 0,2,4,5 -> [0,3,7,9]
    assert_eq!(query_bus_pattern(&graph, "dorian"), vec![0.0, 3.0, 7.0, 9.0]);
    // mixolydian: [0,2,4,5,7,9,10] deg 0,3,6 -> [0,5,10]
    assert_eq!(query_bus_pattern(&graph, "mixo"), vec![0.0, 5.0, 10.0]);
    // pentatonic: [0,2,4,7,9] deg 0..4 -> [0,2,4,7,9]
    assert_eq!(
        query_bus_pattern(&graph, "pent"),
        vec![0.0, 2.0, 4.0, 7.0, 9.0]
    );
}

#[test]
fn test_level1_scale_accepts_pattern_argument() {
    // Architectural rule: `scale` takes a *pattern* of scale names.
    // degrees "2 2 2 2" over one cycle, scales "minor major":
    //   first half -> minor (deg 2 -> 3), second half -> major (deg 2 -> 4).
    let degrees = parse_mini_notation("2 2 2 2");
    let scales = parse_mini_notation("minor major");
    let quantized = quantize_degree_pattern(degrees, scales);
    assert_eq!(query_pattern_values(&quantized), vec![3.0, 3.0, 4.0, 4.0]);
}

#[test]
fn test_level1_note_names_yield_semitones() {
    // note "c e g" -> [0, 4, 7] (relative pitch classes).
    let names = parse_mini_notation("c e g");
    let semis = note_names_to_semitone_pattern(names);
    assert_eq!(query_pattern_values(&semis), vec![0.0, 4.0, 7.0]);
}

// ----- Level 2: onset + pitch --------------------------------------------

#[test]
fn test_level2_sine_note_names_pitch() {
    // Render `sine (note "c e g")`. Standalone `note` yields a frequency pattern
    // (c4/e4/g4 -> 261.6/329.6/392.0 Hz). Verify each event's fundamental.
    // cps = 1.0 => one cycle == 1 second; the three notes occupy thirds.
    let code = r#"cps: 1.0
out $ sine (note "c e g")"#;
    let audio = render_dsl(code, 1.0);
    assert!(!audio.is_empty());
    assert!(
        audio.iter().all(|x| x.is_finite()),
        "audio contains NaN/inf"
    );

    let expected = [mtof(60.0), mtof(64.0), mtof(67.0)]; // c4, e4, g4
    let n = audio.len();
    for (i, &exp) in expected.iter().enumerate() {
        let seg = &audio[n * i / 3..n * (i + 1) / 3];
        // Analyze the middle 60% of each third to avoid boundary transients.
        let inner = &seg[seg.len() * 20 / 100..seg.len() * 80 / 100];
        let freq = estimate_freq_zero_crossings(inner, SR);
        let tol = exp * 0.05; // 5%
        assert!(
            (freq - exp).abs() < tol,
            "note {i}: expected ~{exp:.1} Hz, detected {freq:.1} Hz"
        );
    }
}

#[test]
fn test_level2_sine_note_names_three_ascending_events() {
    // Three notes => three distinct, ascending detected pitches (c < e < g).
    let code = r#"cps: 1.0
out $ sine (note "c e g")"#;
    let audio = render_dsl(code, 1.0);
    let n = audio.len();
    let mut pitches = Vec::new();
    for i in 0..3 {
        let seg = &audio[n * i / 3..n * (i + 1) / 3];
        let inner = &seg[seg.len() * 20 / 100..seg.len() * 80 / 100];
        pitches.push(estimate_freq_zero_crossings(inner, SR));
    }
    assert!(
        pitches[0] < pitches[1] && pitches[1] < pitches[2],
        "expected ascending pitches c<e<g, got {pitches:?}"
    );
}

// ----- Level 2/3: audible scale-quantized melody -------------------------

#[test]
fn test_level3_scale_quantized_melody_audible() {
    // Full musical chain: scale degrees -> semitones -> +root -> mtof -> sine.
    let code = r#"cps: 1.0
out $ sine (mtof ((n "0 2 4 7" # scale "minor") + 60))"#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "scale melody RMS too low: {rms}");
    assert!(
        audio.iter().all(|x| x.is_finite()),
        "scale melody produced NaN/inf"
    );
}

// ----- Level 3: graceful degradation -------------------------------------

#[test]
fn test_level3_unknown_scale_no_panic() {
    // Unknown scale name must not panic; chromatic identity fallback.
    let code = r#"~mel $ n "0 3 5" # scale "not_a_real_scale"
out $ ~mel"#;
    let (_, statements) = parse_program(code).expect("parse");
    let graph = compile_program(statements, SR, None).expect("compile");
    assert_eq!(query_bus_pattern(&graph, "mel"), vec![0.0, 3.0, 5.0]);
}

#[test]
fn test_level3_unknown_note_name_no_panic() {
    // Unknown note name inside a sine frequency pattern renders without panic.
    let code = r#"cps: 1.0
out $ sine (note "c zonk g")"#;
    let audio = render_dsl(code, 1.0);
    assert!(!audio.is_empty());
    assert!(
        audio.iter().all(|x| x.is_finite()),
        "unknown note name produced NaN/inf"
    );
}

#[test]
fn test_level3_n_note_names_render() {
    // `n "c4 e4 g4"` as a standalone source must compile and render audibly.
    let code = r#"cps: 1.0
out $ sine (n "c4 e4 g4")"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "n note-name melody RMS too low: {rms}");
    assert!(audio.iter().all(|x| x.is_finite()));
}
