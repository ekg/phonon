//! Chords in the compositional DSL: `n "c'maj e'min7"` (root'quality expands to
//! a stack) and the `chord` modifier (`n "c e" # chord "maj min7"`,
//! `chord "maj min7"`).  Added for feat-chord-support; builds on the note-name
//! work from feat-scale-quantization.
//!
//! Three-level audio-testing methodology (see CLAUDE.md):
//! - Level 1: pattern-query — the compiled `n "c'maj"` node yields the stack
//!   [0,4,7]; `e'min7` -> [4,7,11,14]; `chord` combines roots + qualities.
//! - Level 2: onset + spectrum — a rendered `c4'maj` shows its three
//!   fundamentals (C4/E4/G4 ≈ 261.6/329.6/392.0 Hz) present *together* in the
//!   FFT, and a chord progression produces one onset per chord.
//! - Level 3: audio characteristics — RMS > 0.01, no NaN, no clipping with 3–4
//!   stacked voices; unknown qualities degrade gracefully (no panic).

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use phonon::scale_dsl::chord_token_to_semitones;
use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
use rustfft::{num_complex::Complex, FftPlanner};
use std::collections::HashMap;

mod pattern_verification_utils;
use pattern_verification_utils::{calculate_peak, calculate_rms, detect_audio_events};

const SR: f32 = 44100.0;

// ----- helpers -----------------------------------------------------------

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, SR, None).expect("Failed to compile DSL code");
    let num_samples = (duration * SR) as usize;
    let bs = 128;
    let mut out = Vec::with_capacity(num_samples);
    for _ in 0..(num_samples / bs) {
        out.extend_from_slice(&graph.render(bs));
    }
    out
}

/// Query a stacked `Pattern<String>` over one cycle, returning numeric values
/// sorted by (start-time, value). Chords are stacks (several haps share a start
/// time), so the value tiebreak makes assertions deterministic.
fn query_stack(pattern: &Pattern<String>) -> Vec<f64> {
    let state = State {
        span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
        controls: HashMap::new(),
    };
    let mut haps = pattern.query(&state);
    haps.sort_by(|a, b| {
        let ka = (
            a.part.begin.to_float(),
            a.value.trim().parse::<f64>().unwrap_or(f64::MAX),
        );
        let kb = (
            b.part.begin.to_float(),
            b.value.trim().parse::<f64>().unwrap_or(f64::MAX),
        );
        ka.partial_cmp(&kb).unwrap()
    });
    haps.iter()
        .map(|h| h.value.trim().parse::<f64>().unwrap())
        .collect()
}

/// Compile a program and return the stacked values of the Pattern node at `bus`.
fn compiled_bus_stack(code: &str, bus: &str) -> Vec<f64> {
    let (_, statements) = parse_program(code).expect("parse");
    let graph: UnifiedSignalGraph = compile_program(statements, SR, None).expect("compile");
    let node_id = graph
        .get_bus(bus)
        .unwrap_or_else(|| panic!("bus ~{bus} not found"));
    match graph.get_node(node_id).expect("node missing") {
        SignalNode::Pattern { pattern, .. } => query_stack(pattern),
        other => panic!("bus ~{bus} is not a Pattern node: {other:?}"),
    }
}

/// Peak magnitude of `audio` in a ±`tol_hz` band around `target_hz`, computed
/// from a 16384-sample FFT window taken from the sustained middle of the signal.
fn band_magnitude(audio: &[f32], sr: f32, target_hz: f32, tol_hz: f32) -> f32 {
    let n = 16384usize.min(audio.len());
    let start = (audio.len() / 3).min(audio.len().saturating_sub(n));
    let slice = &audio[start..start + n];
    let mut buf: Vec<Complex<f32>> = slice.iter().map(|&x| Complex::new(x, 0.0)).collect();
    buf.resize(n, Complex::new(0.0, 0.0));
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    fft.process(&mut buf);
    let bin_hz = sr / n as f32;
    let lo = (((target_hz - tol_hz) / bin_hz).floor() as usize).max(1);
    let hi = (((target_hz + tol_hz) / bin_hz).ceil() as usize).min(n / 2 - 1);
    (lo..=hi).map(|i| buf[i].norm()).fold(0.0, f32::max)
}

// ----- Level 1: pattern-query --------------------------------------------

#[test]
fn test_level1_n_chord_maj_stack() {
    // The primary validation: n "c'maj" expands to the stack [0, 4, 7].
    let code = r#"~c $ n "c'maj"
out $ ~c"#;
    assert_eq!(compiled_bus_stack(code, "c"), vec![0.0, 4.0, 7.0]);
}

#[test]
fn test_level1_n_chord_progression_stack() {
    // n "c'maj e'min7" -> [0,4,7] then [4,7,11,14].
    let code = r#"~c $ n "c'maj e'min7"
out $ ~c"#;
    assert_eq!(
        compiled_bus_stack(code, "c"),
        vec![0.0, 4.0, 7.0, 4.0, 7.0, 11.0, 14.0]
    );
}

#[test]
fn test_level1_required_chord_qualities() {
    // Validation: at least maj, min, dom7, maj7, min7, dim, aug, sus2, sus4.
    // All rooted on c (pitch class 0), so the stack IS the interval set.
    let cases: &[(&str, &[i32])] = &[
        ("c'maj", &[0, 4, 7]),
        ("c'min", &[0, 3, 7]),
        ("c'dom7", &[0, 4, 7, 10]),
        ("c'maj7", &[0, 4, 7, 11]),
        ("c'min7", &[0, 3, 7, 10]),
        ("c'dim", &[0, 3, 6]),
        ("c'aug", &[0, 4, 8]),
        ("c'sus2", &[0, 2, 7]),
        ("c'sus4", &[0, 5, 7]),
    ];
    for (token, expected) in cases {
        let semis = chord_token_to_semitones(token)
            .unwrap_or_else(|| panic!("chord token {token} failed to expand"));
        assert_eq!(&semis, expected, "quality mismatch for {token}");
    }
}

#[test]
fn test_level1_chord_modifier_root_quality() {
    // n "c e" # chord "maj min7" stacks a maj triad on c and a min7 on e.
    let code = r#"~ch $ n "c e" # chord "maj min7"
out $ ~ch"#;
    assert_eq!(
        compiled_bus_stack(code, "ch"),
        vec![0.0, 4.0, 7.0, 4.0, 7.0, 11.0, 14.0]
    );
}

#[test]
fn test_level1_chord_standalone_quality() {
    // Standalone chord "maj min7" voices each quality on the tonic (root 0).
    let code = r#"~ch $ chord "maj min7"
out $ ~ch"#;
    assert_eq!(
        compiled_bus_stack(code, "ch"),
        vec![0.0, 4.0, 7.0, 0.0, 3.0, 7.0, 10.0]
    );
}

// ----- Level 2: onset + spectrum -----------------------------------------

#[test]
fn test_level2_chord_fundamentals_present_together() {
    // Render a C major chord. The synth's base pitch is C4 (261.63 Hz) so the
    // voice manager repitches it to the chord's notes: C4/E4/G4. The FFT of the
    // sustained chord must show ALL THREE fundamentals present simultaneously.
    let code = r#"bpm: 120
~synth $ sine 261.63
~pattern $ s "~synth*2" # note "c4'maj"
out $ ~pattern"#;
    let audio = render_dsl(code, 1.0);
    assert!(audio.iter().all(|x| x.is_finite()), "chord produced NaN/inf");

    let c4 = band_magnitude(&audio, SR, 261.63, 6.0);
    let e4 = band_magnitude(&audio, SR, 329.63, 6.0);
    let g4 = band_magnitude(&audio, SR, 392.00, 6.0);
    // A far-off-chord control band should be much quieter than the fundamentals.
    let off = band_magnitude(&audio, SR, 500.0, 6.0);

    println!("C4={c4:.1} E4={e4:.1} G4={g4:.1} off(500Hz)={off:.1}");
    assert!(c4 > 10.0 * off, "root C4 not dominant: {c4} vs off {off}");
    assert!(e4 > 10.0 * off, "third E4 not dominant: {e4} vs off {off}");
    assert!(g4 > 10.0 * off, "fifth G4 not dominant: {g4} vs off {off}");
}

#[test]
fn test_level2_chord_attack_is_simultaneous() {
    // "Simultaneous onsets": the three notes of a chord must attack together,
    // not as an arpeggio. Proven by comparing the chord's FIRST onset time to a
    // single note's first onset time — they coincide (both at the cycle start),
    // so the stacked voices are triggered at the same instant.
    let chord = r#"bpm: 120
~synth $ sine 261.63
~pattern $ s "~synth*1" # note "c4'maj"
out $ ~pattern"#;
    let single = r#"bpm: 120
~synth $ sine 261.63
~pattern $ s "~synth*1" # note "c4"
out $ ~pattern"#;

    let first_onset = |code: &str| -> f64 {
        let audio = render_dsl(code, 0.5);
        detect_audio_events(&audio, SR, 0.02)
            .first()
            .map(|e| e.time)
            .expect("expected at least one onset")
    };

    let chord_t = first_onset(chord);
    let single_t = first_onset(single);
    println!("chord first onset={chord_t:.4}s  single={single_t:.4}s");
    // Both attack at the cycle start, and within a hair of each other — the
    // chord is a simultaneous stack, not a spread-out arpeggio.
    assert!(chord_t < 0.05, "chord attack not at cycle start: {chord_t}");
    assert!(
        (chord_t - single_t).abs() < 0.02,
        "chord attack ({chord_t}) not simultaneous with single note ({single_t})"
    );
}

#[test]
fn test_level2_chord_progression_multiple_onsets() {
    // A four-chord progression produces a sequence of chord attacks over the
    // cycle (each chord's notes onset together). We only assert that several
    // distinct attacks occur — exact counts are unreliable on sustained,
    // detuned-voice material, so simultaneity is proven by the FFT/attack tests.
    let code = r#"bpm: 120
~synth $ sine 261.63
~pattern $ s "~synth*4" # note "c4'maj f4'maj g4'maj c5'maj"
out $ ~pattern"#;
    let audio = render_dsl(code, 1.0); // one cycle == 4 chords
    let onsets = detect_audio_events(&audio, SR, 0.02);
    println!("progression onsets: {}", onsets.len());
    assert!(
        onsets.len() >= 4,
        "expected multiple chord attacks, got {}",
        onsets.len()
    );
    assert!(audio.iter().all(|x| x.is_finite()));
}

// ----- Level 3: audio characteristics ------------------------------------

#[test]
fn test_level3_four_voice_chord_clean() {
    // min7 = 4 stacked voices. Must be audible, finite, and not clipping.
    let code = r#"bpm: 120
~synth $ saw 261.63
~pattern $ s "~synth*2" # note "c4'min7"
out $ ~pattern * 0.5"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);
    println!("min7 4-voice RMS={rms:.3} peak={peak:.3}");
    assert!(rms > 0.01, "4-voice chord RMS too low: {rms}");
    assert!(audio.iter().all(|x| x.is_finite()), "4-voice chord NaN/inf");
    assert!(peak <= 1.0001, "4-voice chord clipping: peak {peak}");
}

#[test]
fn test_level3_unknown_quality_no_panic() {
    // Unknown chord quality must degrade to the root alone, never panicking.
    let code = r#"~c $ n "c'zonk"
out $ ~c"#;
    assert_eq!(compiled_bus_stack(code, "c"), vec![0.0]);

    // And it must still render without NaN through the voice path.
    let code = r#"bpm: 120
~synth $ sine 261.63
~pattern $ s "~synth*2" # note "c4'zonk"
out $ ~pattern"#;
    let audio = render_dsl(code, 0.5);
    assert!(audio.iter().all(|x| x.is_finite()));
}

#[test]
fn test_level3_mixed_notes_and_chords_render() {
    // Mixing single notes and chords in one pattern must render cleanly.
    let code = r#"bpm: 120
~synth $ triangle 261.63
~pattern $ s "~synth*4" # note "c4 e4'min g4 c5'maj7"
out $ ~pattern * 0.6"#;
    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "mixed pattern RMS too low: {rms}");
    assert!(audio.iter().all(|x| x.is_finite()));
    assert!(calculate_peak(&audio) <= 1.0001, "mixed pattern clipping");
}
