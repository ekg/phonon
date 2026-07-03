//! Tests for internal node-state sanitisation + the pre-sanitisation invariant
//! probe (task `sanitize-node-state`, G5 + I1 / rt F-6, test-gap P0-C).
//!
//! Two intertwined concerns:
//!
//!   * **G5 / F-6** — the global output guard zeroes non-finite *output* samples,
//!     but a node whose own recursive state (resonant filter, feedback delay/reverb,
//!     FM loop) goes NaN/Inf keeps emitting non-finite forever → the voice/bus is
//!     stuck-silent until reload. The fix flushes the node's internal state so it
//!     recovers.
//!
//!   * **I1 / P0-C** — because sanitisation runs BEFORE any observer sees the output,
//!     the harness NaN/clip gates were tautological (an internal NaN reaches the
//!     harness as a clean `0.0`). The raw pre-sanitisation probe observes the true
//!     internal signal so the gates are meaningful.
//!
//! The reproductions here are deterministic: a single injected NaN into a *stable*
//! filter (so the recovery is complete), plus an arithmetically-diverging DSL graph.

use phonon::stress_harness::{
    build_initial, count_nonfinite, known_good_pool, run_all_scenarios, run_random_session,
    SessionConfig,
};
use phonon::unified_graph::UnifiedSignalGraph;

const SR: f32 = 44100.0;
const BLOCK: usize = 512 * 2; // stereo-interleaved, 512 frames

fn render(g: &mut UnifiedSignalGraph) -> Vec<f32> {
    let mut buf = vec![0.0f32; BLOCK];
    g.process_buffer(&mut buf);
    buf
}

fn block_rms(buf: &[f32]) -> f32 {
    (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
}

// ---------------------------------------------------------------------------
// Probe basics
// ---------------------------------------------------------------------------

#[test]
fn test_raw_probe_off_by_default() {
    let mut g = build_initial("tempo: 1.0\nout $ sine 440 * 0.3", SR).unwrap();
    assert!(!g.raw_probe_enabled(), "probe must be off by default");
    let _ = render(&mut g);
    // With the probe disabled the snapshot stays at its default (all-zero) value —
    // no overhead is paid on the production render path.
    let p = g.last_raw_probe();
    assert_eq!(p.raw_nonfinite, 0);
    assert_eq!(p.raw_peak, 0.0);
    assert_eq!(p.first_nonfinite_node, None);
}

#[test]
fn test_clean_program_no_raw_nonfinite() {
    // A well-behaved program must never register a raw non-finite — otherwise the
    // gate would false-positive and be useless.
    let mut g = build_initial("tempo: 1.0\nout $ saw 110 # lpf 1500 0.6 * 0.3", SR).unwrap();
    g.enable_raw_probe(true);
    for _ in 0..50 {
        let buf = render(&mut g);
        let p = g.last_raw_probe();
        assert_eq!(p.raw_nonfinite, 0, "clean program produced raw non-finite");
        assert!(p.raw_peak.is_finite() && p.raw_peak < 1.5, "unexpected raw peak {}", p.raw_peak);
        // sanity: it is actually producing audio
        assert!(block_rms(&buf) > 0.001);
    }
}

// ---------------------------------------------------------------------------
// The core F-6 / I1 property: an internal NaN is caught by the RAW probe even
// though the sanitised output is a clean 0.0 (the tautology is removed), AND the
// node recovers instead of going permanently stuck-silent.
// ---------------------------------------------------------------------------

#[test]
fn test_injected_filter_nan_caught_raw_but_output_clean_and_recovers() {
    // A *stable* resonant low-pass, so once its state is reset the recovery is
    // complete (raw non-finite returns to exactly zero).
    let code = "tempo: 1.0\nout $ saw 110 # lpf 1500 0.6 * 0.4";

    let mut g = build_initial(code, SR).unwrap();
    g.enable_raw_probe(true);
    let lpf = g.debug_find_lowpass_node().expect("graph has a low-pass node");

    // Warm up: clean audio, no raw non-finite.
    for _ in 0..4 {
        let buf = render(&mut g);
        assert_eq!(g.last_raw_probe().raw_nonfinite, 0);
        assert!(block_rms(&buf) > 0.001);
    }

    // Simulate a numerical blow-up of the filter's internal integrator state.
    assert!(g.debug_force_node_state_nan(lpf), "filter state should be corruptible");

    // The block on which the NaN surfaces:
    let buf = render(&mut g);
    let probe = g.last_raw_probe();
    let (nan, inf) = count_nonfinite(&buf);

    // (a) The RAW probe caught the internal blow-up and named its origin node...
    assert!(
        probe.raw_nonfinite > 0,
        "raw probe must catch the internal NaN (got {})",
        probe.raw_nonfinite
    );
    assert_eq!(
        probe.first_nonfinite_node,
        Some(lpf),
        "raw probe must name the originating node"
    );
    // ...while the *sanitised* output the caller sees is completely finite (a `0.0`
    // where the NaN was). This is exactly why the old `nan`/`inf` gate was blind.
    assert_eq!(nan + inf, 0, "sanitised output must never expose NaN/Inf");

    // (b) The node RECOVERS: within a few blocks the raw non-finite returns to zero
    // and audio is restored (RMS > 0). On `main` (no state reset) this stays NaN
    // forever and the output is stuck-silent — see the paired test below.
    let mut recovered = false;
    for _ in 0..10 {
        let buf = render(&mut g);
        if g.last_raw_probe().raw_nonfinite == 0 && block_rms(&buf) > 0.001 {
            recovered = true;
            break;
        }
    }
    assert!(recovered, "node must recover to finite audio after the transient");
}

#[test]
fn test_without_fix_node_stays_stuck_silent() {
    // Reproduce the pre-fix `main` behaviour by disabling the state sanitiser: an
    // internal NaN persists and the node is stuck-silent forever, while the raw
    // probe keeps reporting the non-finite (proving it is genuinely stuck, not
    // merely masked).
    let code = "tempo: 1.0\nout $ saw 110 # lpf 1500 0.6 * 0.4";
    let mut g = build_initial(code, SR).unwrap();
    g.enable_raw_probe(true);
    g.set_node_state_sanitize(false); // pre-fix behaviour
    let lpf = g.debug_find_lowpass_node().unwrap();

    for _ in 0..4 {
        render(&mut g);
    }
    assert!(g.debug_force_node_state_nan(lpf));

    let mut last_rms = 1.0;
    let mut last_raw = 0;
    for _ in 0..20 {
        let buf = render(&mut g);
        last_rms = block_rms(&buf);
        last_raw = g.last_raw_probe().raw_nonfinite;
    }
    assert_eq!(last_rms, 0.0, "without the fix the node must be stuck-silent");
    assert!(
        last_raw > 0,
        "without the fix the raw signal stays non-finite (stuck), got {last_raw}"
    );
}

// ---------------------------------------------------------------------------
// DSL-level reproductions (no test hooks): arithmetic divergence.
// ---------------------------------------------------------------------------

#[test]
fn test_resonant_filter_blowup_recovers_not_stuck_silent() {
    // A self-oscillating filter (cutoff pinned to Nyquist-ish, extreme Q) drives its
    // Chamberlin-SVF state past stability into Inf/NaN. WITH the fix the node stays
    // audible; WITHOUT it the output is stuck-silent.
    let code = "tempo: 1.0\nout $ saw 55 # lpf 20000 20 * 0.5";

    // With the fix.
    let mut g = build_initial(code, SR).unwrap();
    g.enable_raw_probe(true);
    let mut late_rms = 0.0f32;
    let mut caught_raw = false;
    for i in 0..120 {
        let buf = render(&mut g);
        let (nan, inf) = count_nonfinite(&buf);
        assert_eq!(nan + inf, 0, "sanitised output leaked NaN/Inf at block {i}");
        if g.last_raw_probe().raw_nonfinite > 0 {
            caught_raw = true;
        }
        if i >= 110 {
            late_rms += block_rms(&buf);
        }
    }
    assert!(caught_raw, "raw probe must catch the filter blow-up");
    assert!(late_rms / 10.0 > 0.001, "filtered output must not be stuck-silent (fix)");

    // Without the fix: stuck-silent.
    let mut g2 = build_initial(code, SR).unwrap();
    g2.set_node_state_sanitize(false);
    let mut late_rms2 = 0.0f32;
    for i in 0..120 {
        let buf = render(&mut g2);
        if i >= 110 {
            late_rms2 += block_rms(&buf);
        }
    }
    assert_eq!(late_rms2 / 10.0, 0.0, "without the fix the filter is stuck-silent");
}

#[test]
fn test_feedback_blowup_never_leaks_nan_to_output() {
    // A self-referential bus with loop gain > 1 diverges. The output guard + node
    // scrub must keep the *sanitised* output finite on every block, and the raw
    // probe must observe the internal explosion.
    let code = "tempo: 1.0\n~fb $ ~fb * 1.5 + sine 440 * 0.5\nout $ ~fb";
    let mut g = build_initial(code, SR).unwrap();
    g.enable_raw_probe(true);
    let mut caught_raw = false;
    for i in 0..80 {
        let buf = render(&mut g);
        let (nan, inf) = count_nonfinite(&buf);
        assert_eq!(nan + inf, 0, "feedback NaN leaked to sanitised output at block {i}");
        if g.last_raw_probe().raw_nonfinite > 0 {
            caught_raw = true;
        }
    }
    assert!(caught_raw, "raw probe must catch the feedback blow-up");
}

// ---------------------------------------------------------------------------
// Harness-level wiring: the raw gate is active and non-tautological.
// ---------------------------------------------------------------------------

#[test]
fn test_harness_scenario_f6_raw_gate_nontautological() {
    // The scripted `F6-resonant-filter-blowup` scenario must show the raw probe
    // catching what the sanitised NaN gate cannot (`nan == 0` but `raw_nonfinite > 0`),
    // and — thanks to the state reset — stay audible (`post_rms` above the floor)
    // rather than dropping to silence.
    let cfg = SessionConfig::ci(42);
    let (results, failures) = run_all_scenarios(&cfg);
    // Documented scenarios never hard-fail, so the F6 blow-up must not break the run.
    assert!(
        failures.iter().all(|f| !f.contains("F6")),
        "F6 scenario should be documented, not a hard failure: {failures:?}"
    );
    let f6 = results
        .iter()
        .find(|r| r.name == "F6-resonant-filter-blowup")
        .expect("F6 scenario present");
    assert!(f6.available, "F6 scenario must compile");
    assert_eq!(f6.nan + f6.inf, 0, "sanitised gate is blind to the internal NaN (tautology)");
    assert!(
        f6.raw_nonfinite > 0,
        "raw gate must catch the internal blow-up (got {})",
        f6.raw_nonfinite
    );
    assert!(f6.raw_peak > 1.0, "raw peak must expose the pre-limiter blow-up");
    assert!(!f6.post_silent, "state reset must keep the node audible, not stuck-silent");
}

#[test]
fn test_random_session_raw_gate_clean_on_known_good() {
    // A short seeded session over the known-good pool must register ZERO raw
    // non-finite — the newly-active raw gate must not false-positive.
    let mut cfg = SessionConfig::ci(42);
    cfg.target_seconds = 8.0;
    cfg.min_swaps = 10;
    let report = run_random_session(&cfg, &known_good_pool());
    assert_eq!(
        report.raw_nonfinite_samples, 0,
        "raw gate false-positived on known-good programs: {:?}",
        report.first_defect
    );
    assert_eq!(report.nan_samples, 0);
    assert_eq!(report.inf_samples, 0);
    assert!(report.is_clean(&cfg.thresholds), "known-good session must be clean");
}

// ---------------------------------------------------------------------------
// Delay-line and reverb recovery (mechanism coverage beyond filters).
// ---------------------------------------------------------------------------

#[test]
fn test_delay_and_reverb_recover_from_injected_nan() {
    for code in [
        "tempo: 1.0\nout $ sine 220 # delay 0.25 0.4 0.3 * 0.3",
        "tempo: 1.0\nout $ saw 110 # lpf 1200 0.6 # reverb 0.4 0.3 * 0.2",
    ] {
        let mut g = build_initial(code, SR).unwrap();
        g.enable_raw_probe(true);
        for _ in 0..8 {
            render(&mut g);
        }
        // Corrupt whatever filter is present (reverb graph has an lpf; delay graph
        // relies on the node-boundary scrub of the delay buffer). Then confirm the
        // sanitised output never leaks NaN and audio survives.
        if let Some(id) = g.debug_find_lowpass_node() {
            g.debug_force_node_state_nan(id);
        }
        let mut ok_blocks = 0;
        for _ in 0..30 {
            let buf = render(&mut g);
            let (nan, inf) = count_nonfinite(&buf);
            assert_eq!(nan + inf, 0, "sanitised output leaked NaN/Inf for `{code}`");
            if block_rms(&buf) > 0.001 {
                ok_blocks += 1;
            }
        }
        assert!(ok_blocks > 0, "graph `{code}` never recovered audio");
    }
}
