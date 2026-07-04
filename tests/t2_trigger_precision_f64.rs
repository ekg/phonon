//! T2 — `last_trigger_time` widened `f32` -> `f64` (audit pt-F3 /
//! `docs/audits/improvement-plan-2026-07.md` T2): kill long-session onset drift.
//!
//! `last_trigger_time` stores an ABSOLUTE cycle position and gates whether a
//! pattern event is a NEW trigger (dedup). As `f32` it is pinned to its ulp,
//! which grows with the cycle number: once the ulp reaches the event spacing the
//! dedup collapses — events are dropped (or, across buffer boundaries, doubled).
//! For `s "bd*4"` (quarter-cycle spacing) the ulp reaches 0.25 cycle at ~2^22
//! and 1.0 cycle at 2^23, so a long-running set loses most of its kicks. `f64`
//! keeps the timestamp exact for any realistic session length.
//!
//! Level 1 (per-node timestamp precision) lives as an in-crate unit test in
//! `src/unified_graph.rs` (`t2_trigger_precision_tests`) where the private field
//! is readable. This file covers Levels 2 (onset detection on rendered audio)
//! and 3 (audio characteristics). Both need real `bd` sample audio, resolved
//! from the dirt-samples search path.

use phonon::unified_graph::UnifiedSignalGraph;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

const SR: f32 = 44100.0;
const BUF: usize = 512;

fn build(code: &str) -> UnifiedSignalGraph {
    let (_, statements) = parse_dsl(code).expect("parse DSL");
    DslCompiler::new(SR).compile(statements)
}

/// Render `n_samples` mono, seeking first to absolute cycle `start_cycle`, then
/// processing realtime-sized 512-sample buffers (the boundary behaviour where
/// f32 dedup fails is exercised only by chunked, not one-shot, rendering).
fn render_at(g: &mut UnifiedSignalGraph, start_cycle: f64, n_samples: usize) -> Vec<f32> {
    g.set_cycle(start_cycle);
    let mut out = Vec::with_capacity(n_samples);
    let mut done = 0;
    while done < n_samples {
        let chunk = BUF.min(n_samples - done);
        let mut buf = vec![0.0f32; chunk * 2];
        g.process_buffer(&mut buf);
        for i in 0..chunk {
            out.push(buf[i * 2]);
        }
        done += chunk;
    }
    out
}

/// Count attack transients: rising edges of a short RMS envelope crossing
/// `thresh`, separated by at least `refractory` samples. Deterministic on the
/// offline render, and robust against the doubled/overlapping bursts f32 dedup
/// failure produces (they merge under the refractory gap rather than inflating
/// the count).
fn count_onsets(audio: &[f32], refractory: usize, thresh: f32) -> usize {
    let w = 64usize;
    let mut env = vec![0.0f32; audio.len()];
    let mut acc = 0.0f32;
    for i in 0..audio.len() {
        acc += audio[i] * audio[i];
        if i >= w {
            acc -= audio[i - w] * audio[i - w];
        }
        env[i] = (acc / w as f32).sqrt();
    }
    let mut count = 0;
    let mut last: Option<usize> = None;
    for i in 1..env.len() {
        let rising = env[i] > thresh && env[i] > env[i - 1] && env[i - 1] <= thresh;
        let far_enough = last.map_or(true, |l| i - l >= refractory);
        if rising && far_enough {
            count += 1;
            last = Some(i);
        }
    }
    count
}

fn rms(audio: &[f32]) -> f32 {
    (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt()
}

// Rendering parameters shared by the audio-level tests: `s "bd*4"` at 2 cps.
const CPS: f64 = 2.0;
const CYCLES: usize = 8;
const CODE: &str = "tempo: 2.0\nout $ s \"bd*4\"";
// 2^23: f32 cycle-position ulp is a full 1.0 cycle here, so sub-cycle trigger
// timing is entirely lost — the extreme end of the precision cliff.
const LARGE_OFFSET: f64 = 8_388_608.0;

fn n_samples() -> usize {
    (CYCLES as f64 / CPS * SR as f64).round() as usize
}

fn refractory() -> usize {
    // half the inter-kick spacing
    (SR as f64 / CPS / 4.0 / 2.0) as usize
}

/// Level 2 (onset detection): `s "bd*4"` rendered at a large cycle offset must
/// still fire exactly 4 kicks per cycle — no drops, no doubles.
///
/// Pre-fix (f32): the timestamp quantizes to whole cycles at 2^23, dedup
/// collapses, and only a handful of onsets survive (~1/cycle).
#[test]
fn level2_bd4_exact_onsets_at_large_offset() {
    let expected = CYCLES * 4; // 32

    // Baseline at cycle 0 is correct on both f32 and f64 — anchors the detector.
    let mut g0 = build(CODE);
    let base = render_at(&mut g0, 0.0, n_samples());
    let base_onsets = count_onsets(&base, refractory(), 0.02);
    assert!(
        (expected as i32 - base_onsets as i32).abs() <= 2,
        "detector sanity: baseline onsets={base_onsets}, expected ~{expected}"
    );

    // Same pattern, far into a long session.
    let mut g = build(CODE);
    let audio = render_at(&mut g, LARGE_OFFSET, n_samples());
    let onsets = count_onsets(&audio, refractory(), 0.02);

    assert!(
        (expected as i32 - onsets as i32).abs() <= 2,
        "onset drift at cycle {LARGE_OFFSET}: got {onsets} onsets, expected \
         ~{expected} (4/cycle over {CYCLES} cycles). f32 last_trigger_time drops \
         triggers at large cycle positions; f64 must keep 4/cycle."
    );
}

/// Regression guard across the precision cliff: at a range of large offsets the
/// onset count stays at 4/cycle. Pre-fix these degrade (≈24 at 2^22, ≈8 at 2^23).
#[test]
fn level2_bd4_onsets_stable_across_offsets() {
    let expected = CYCLES * 4;
    for &offset in &[1.0e5, 2.097_152e6 /* 2^21 */, 4.194_304e6 /* 2^22 */, LARGE_OFFSET] {
        let mut g = build(CODE);
        let audio = render_at(&mut g, offset, n_samples());
        let onsets = count_onsets(&audio, refractory(), 0.02);
        assert!(
            (expected as i32 - onsets as i32).abs() <= 2,
            "onset count {onsets} at offset {offset} deviates from 4/cycle \
             (expected ~{expected})"
        );
    }
}

/// Level 3 (audio characteristics): at a large offset the output stays finite
/// and its energy matches the cycle-0 baseline. Pre-fix, collapsed/overlapping
/// triggers inflate RMS well above baseline.
#[test]
fn level3_rms_stable_and_finite_at_large_offset() {
    let mut g0 = build(CODE);
    let base = render_at(&mut g0, 0.0, n_samples());
    let base_rms = rms(&base);
    assert!(base_rms > 0.05, "baseline should have energy, rms={base_rms}");

    let mut g = build(CODE);
    let audio = render_at(&mut g, LARGE_OFFSET, n_samples());
    assert!(
        audio.iter().all(|x| x.is_finite()),
        "output has NaN/Inf at large cycle offset"
    );
    let far_rms = rms(&audio);
    assert!(far_rms > 0.05, "should still have energy at large offset, rms={far_rms}");
    assert!(
        far_rms < base_rms * 1.5,
        "RMS inflated at cycle {LARGE_OFFSET}: {far_rms} vs baseline {base_rms} \
         (collapsed/doubled triggers from f32 dedup failure); f64 must keep it \
         steady"
    );
}
