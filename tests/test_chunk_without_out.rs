//! Regression tests for audit finding **U1** (docs/audits/live-transition-2026-07.md §5).
//!
//! Live-coding C-x replaces the whole graph with only the evaluated chunk
//! (`ModalEditor::eval_chunk` -> `load_code` -> `compile_program`). The audit
//! predicted that C-x'ing a block with no `out` (e.g. `~bass $ sine 55`) would
//! *silence* output. The glitch stress harness instead measured a sudden
//! ~0.7 RMS blast: the compiler's "mix all buses" fallback routed the lone
//! plain bus to the speakers at UNITY gain.
//!
//! Decided behavior (investigate-u1-swapping): the auto-sum convenience is kept —
//! a multi-bus file with no `out`/`~master` still sounds — but the auto-summed
//! output is bounded by a documented headroom gain (`AUTO_ROUTE_HEADROOM_GAIN`,
//! −12 dB) so a lone raw generator can never blast at unity. Add an explicit
//! `out $ ...` to control the mix precisely.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn compile_and_render(code: &str, seconds: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("compile failed");
    let n = (sample_rate * seconds) as usize;
    graph.render(n)
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// U1 core: a lone plain `~name` bus with no `out` must NOT blast at unity gain
/// (~0.7 RMS). It stays audible (auto-sum preserved) but bounded by the −12 dB
/// headroom. A raw `sine 55` is ~0.707 RMS at unity → ~0.177 after headroom.
#[test]
fn test_lone_bus_without_out_is_attenuated_not_blast() {
    let audio = compile_and_render("tempo: 1.0\n~bass $ sine 55", 1.0);
    let r = rms(&audio);
    let p = peak(&audio);
    assert!(
        r < 0.35,
        "lone bus without `out` must be attenuated, not the ~0.7 RMS unity blast, got RMS {r:.5}"
    );
    assert!(
        r > 0.05,
        "auto-sum should still be audible (bounded, not silent), got RMS {r:.5}"
    );
    assert!(p < 0.5, "peak must stay well below full scale, got peak {p:.5}");
}

/// The multi-plain-bus auto-sum is preserved (audible) but bounded — the old
/// fallback summed at unity and could clip.
#[test]
fn test_multiple_plain_buses_auto_sum_is_bounded() {
    let audio = compile_and_render("tempo: 1.0\n~drums $ saw 110\n~bass $ sine 55", 1.0);
    let r = rms(&audio);
    let p = peak(&audio);
    assert!(r > 0.05, "auto-sum of several buses should be audible, got RMS {r:.5}");
    assert!(
        r < 0.45,
        "auto-summed buses must be bounded by headroom, not blast, got RMS {r:.5}"
    );
    assert!(p < 0.999, "auto-sum must not clip, got peak {p:.5}");
}

/// Guardrail: an explicit `out $ ...` is unaffected by the headroom — it routes
/// at exactly the gain the user asked for.
#[test]
fn test_explicit_out_unaffected_by_headroom() {
    let audio = compile_and_render("tempo: 1.0\n~bass $ sine 55\nout $ ~bass * 0.3", 1.0);
    let r = rms(&audio);
    // sine at unity is ~0.707 RMS; * 0.3 => ~0.212. Must NOT be additionally
    // attenuated by the auto-sum headroom (that would give ~0.053).
    assert!(
        r > 0.15,
        "explicit `out $ ~bass * 0.3` must route at the user's gain (~0.21 RMS), got {r:.5}"
    );
}

/// Guardrail: the Tidal-style `dN` auto-route bus still reaches the speakers at
/// unity (it is an explicit speaker route, not the plain-bus fallback).
#[test]
fn test_dn_autoroute_still_audible_at_unity() {
    let audio = compile_and_render("tempo: 1.0\nd1 $ sine 110 * 0.3", 1.0);
    let r = rms(&audio);
    assert!(
        r > 0.15,
        "`d1` auto-route bus must still be audible at its own gain, got RMS {r:.5}"
    );
}

/// Guardrail: an explicit `~master` bus still reaches the speakers at unity.
#[test]
fn test_master_bus_still_audible_at_unity() {
    let audio = compile_and_render("tempo: 1.0\n~master $ sine 110 * 0.3", 1.0);
    let r = rms(&audio);
    assert!(
        r > 0.15,
        "`~master` bus must still be audible at its own gain, got RMS {r:.5}"
    );
}
