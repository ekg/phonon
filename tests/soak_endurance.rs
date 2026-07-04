//! Soak / endurance regression net (improvement-plan I3 / test-gap P1-A).
//!
//! Bounded, deterministic CI pass of the soak harness in `src/bin/soak_endurance.rs`.
//! The shared soak logic lives in that binary and is pulled in read-only here via
//! `#[path]` so the CI test and the opt-in `--bin` long run exercise the SAME code.
//! `stress_harness` detectors are reused read-only (imported public items); this
//! test never edits `stress_harness.rs` or `unified_graph.rs`.
//!
//! Coverage against the task's validation checklist:
//!   * `test_soak_no_accumulation_drift` — failing-test-first: the accumulation /
//!     drift / stuck detectors BITE on synthetic degraded input and pass on clean.
//!   * Level 1 (pattern-query): per-cycle event counts are constant from cycle 0
//!     to a very late cycle (no drift / doubling / dropping).
//!   * Level 2 (onset-detection): onset positions stay sample-accurate in a late
//!     window vs an early window (validates the T2 `f64` trigger clock).
//!   * Level 3 (audio): RMS bounded & stationary, no NaN/Inf, no stuck-silent /
//!     stuck-loud window, and the bounded resource proxy does not grow across
//!     repeated swaps.
//!   * Noise-heavy scenario is deterministic (regresses if `wave3-noise-rng-hotpath`
//!     is reverted) and clean.

#[path = "../src/bin/soak_endurance.rs"]
#[allow(dead_code)]
mod soak;

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::stress_harness::Thresholds;
use soak::*;
use std::collections::HashMap;

const SR: f32 = 44100.0;

fn query_cycle_event_count(pat: &phonon::pattern::Pattern<String>, cycle: i64) -> usize {
    let state = State {
        span: TimeSpan::new(Fraction::new(cycle, 1), Fraction::new(cycle + 1, 1)),
        controls: HashMap::new(),
    };
    pat.query(&state).len()
}

// ===========================================================================
// FAILING-TEST-FIRST: prove every accumulation / drift / stuck detector BITES.
//
// The engine bugs this net guards (P2 parse-leak, G4 voice-pool growth, T2 f32
// trigger cliff) are already FIXED and this file must not edit the engine to
// re-break them. So we demonstrate the detectors are not vacuous the rigorous
// way: feed each detector a synthetically DEGRADED signal (the exact shape the
// regression would produce) and assert it fires, then feed it the CLEAN shape
// and assert it stays silent. If any detector were a no-op, this test fails.
// ===========================================================================
#[test]
fn test_soak_no_accumulation_drift() {
    // --- (1) resource-growth detector (P2 parse-leak / G4 voice-pool growth) ---
    // A healthy run cycles through a fixed program pool: the same counts recur, so
    // early-window max == late-window max → no growth.
    let healthy: Vec<usize> = (0..100).map(|i| [1usize, 6, 11, 12][i % 4]).collect();
    assert!(
        detect_count_growth(&healthy, 64).is_none(),
        "resource-growth detector false-fired on a bounded oscillating series"
    );
    // A leak makes the count climb monotonically with every swap.
    let leaking: Vec<usize> = (0..100).map(|i| 8 + i).collect();
    let bite = detect_count_growth(&leaking, 64);
    assert!(bite.is_some(), "resource-growth detector MISSED a monotonic leak");
    let (early, late) = bite.unwrap();
    assert!(late > early, "leak detector reported late<=early ({late} <= {early})");

    // The voice-pool proxy uses a tight slack — a single-slot growth trend fires.
    let pool_leak: Vec<usize> = (0..100).map(|i| 512 + i / 4).collect();
    assert!(
        detect_count_growth(&pool_leak, 8).is_some(),
        "voice-pool growth detector MISSED a slow pool climb (G4 signature)"
    );
    let pool_stable: Vec<usize> = vec![512; 100];
    assert!(
        detect_count_growth(&pool_stable, 8).is_none(),
        "voice-pool detector false-fired on a constant 512-slot pool"
    );

    // --- (2) onset-drift detector (T2 f32 trigger cliff) ---
    // Clean grid: onsets exactly `GRID` apart → zero deviation.
    let grid = ONSET_PROBE_GRID;
    let clean: Vec<usize> = (0..8).map(|k| (k as i64 * grid) as usize).collect();
    let clean_dev = max_ioi_deviation(&inter_onset_intervals(&clean), grid);
    assert!(clean_dev <= 2, "clean grid reported drift {clean_dev} (should be ~0)");
    // The f32 trigger cliff at a huge cycle count quantises trigger times to a
    // coarse grid whose step exceeds the note spacing (at ~5e6 cycles the f32 ULP
    // is ~0.5 cycle ≈ 22050 samples), collapsing/merging the quarter-note grid.
    // Reconstruct that and show the deviation explodes far past any sane tolerance.
    let f32_ulp_grid: i64 = 22050; // ~0.5-cycle ULP: coarser than the 11025 spacing
    let drifted: Vec<usize> = (0..8)
        .map(|k| {
            let ideal = k as i64 * grid;
            let snapped = (ideal + f32_ulp_grid / 2) / f32_ulp_grid * f32_ulp_grid;
            snapped as usize
        })
        .collect();
    let drift_dev = max_ioi_deviation(&inter_onset_intervals(&drifted), grid);
    assert!(
        drift_dev > 512,
        "onset-drift detector MISSED an f32-quantised trigger grid (dev={drift_dev}, series={drifted:?})"
    );

    // --- (3) voiceless-window (stuck-silence) detector ---
    let sounding: Vec<f32> = (0..300).map(|i| if i % 20 == 0 { 0.5 } else { 0.0 }).collect();
    assert_eq!(
        count_voiceless_windows(&sounding, 40, 0.001, 0),
        0,
        "voiceless detector false-fired on a series with a hit in every window"
    );
    let dead: Vec<f32> = vec![0.0; 300];
    assert!(
        count_voiceless_windows(&dead, 40, 0.001, 0) > 0,
        "voiceless detector MISSED an all-silent (dead-synth) series"
    );

    // --- (4) stuck-loud (limiter-railed feedback blow-up) detector ---
    let normal: Vec<f32> = (0..1024).map(|i| 0.3 * (i as f32 * 0.05).sin()).collect();
    assert!(
        !is_stuck_loud(&normal, 0.95, 0.9),
        "stuck-loud detector false-fired on normal-level audio"
    );
    let railed: Vec<f32> = (0..1024).map(|i| if i % 2 == 0 { 0.95 } else { -0.95 }).collect();
    assert!(
        is_stuck_loud(&railed, 0.95, 0.9),
        "stuck-loud detector MISSED a limiter-railed buffer"
    );

    // --- and the REAL bounded soak run is clean under all of the above ---
    let thr = Thresholds::default();
    let report = run_soak(&SoakConfig::ci(1));
    assert!(
        report.is_clean(&thr),
        "bounded soak run should be clean; {}",
        report.summary(&thr)
    );
}

// ===========================================================================
// LEVEL 1 — pattern-query: event counts per cycle stay constant, early to late.
// ===========================================================================
#[test]
fn test_soak_level1_event_count_constant_early_to_late() {
    let pat = parse_mini_notation("bd*4");
    let expected = query_cycle_event_count(&pat, 0);
    assert_eq!(expected, 4, "bd*4 should yield 4 events in cycle 0");

    // Constant across nearby cycles ...
    for c in 0..64 {
        assert_eq!(
            query_cycle_event_count(&pat, c),
            expected,
            "event count drifted at cycle {c}"
        );
    }
    // ... and still constant a *very* long way into the session (T2 territory).
    for &c in &[10_000i64, 100_000, 1_000_000, 5_000_000, 36_000_000] {
        assert_eq!(
            query_cycle_event_count(&pat, c),
            expected,
            "event count drifted/doubled/dropped at late cycle {c}"
        );
    }

    // A denser pattern behaves the same (no accumulation in the query path).
    let beat = parse_mini_notation("bd sn hh*4 cp");
    let beat0 = query_cycle_event_count(&beat, 0);
    for &c in &[0i64, 1, 1_000_000, 10_000_000] {
        assert_eq!(query_cycle_event_count(&beat, c), beat0, "beat count drifted at cycle {c}");
    }
}

// ===========================================================================
// LEVEL 2 — onset-detection: onset positions stay sample-accurate late in the
// run vs early (validates the T2 f64 trigger clock over ~10 h of cycles).
// ===========================================================================
#[test]
fn test_soak_level2_onset_sample_accurate_late_vs_early() {
    // 1,000,000 cycles at cps=1 ≈ 11.5 simulated days — far past the ~10 h T2
    // cliff. With f64 the trigger grid is exact; with the reverted f32 it would
    // quantise to a ~5512-sample grid and the deviation below would explode.
    let r = onset_drift_probe(ONSET_PROBE_CODE, ONSET_PROBE_GRID, 1_000_000.0, 2.0, SR);

    assert!(r.early_onsets >= 4, "early window found too few onsets: {r:?}");
    assert!(r.late_onsets >= 3, "late window found too few onsets: {r:?}");

    // Every inter-onset interval sits on the 11025-sample grid within one
    // detector hop of slack, both early AND late.
    let tol = 300i64; // ~7 ms; the detector hop is ~55 samples, drift would be ~5000+
    assert!(
        r.early_max_deviation <= tol,
        "early onsets already off-grid (dev={}) — detector problem: {r:?}",
        r.early_max_deviation
    );
    assert!(
        r.late_max_deviation <= tol,
        "LATE onsets drifted off the grid (dev={}) — T2 f64 trigger clock regressed: {r:?}",
        r.late_max_deviation
    );
    // Late median spacing matches early to within a hop — no phase drift.
    assert!(
        (r.late_median_ioi - r.early_median_ioi).abs() <= 128,
        "late/early median IOI diverged: early={} late={}",
        r.early_median_ioi,
        r.late_median_ioi
    );
    // And both match the theoretical grid.
    assert!(
        (r.late_median_ioi - ONSET_PROBE_GRID).abs() <= 128,
        "late median IOI {} far from grid {}",
        r.late_median_ioi,
        ONSET_PROBE_GRID
    );

    // Even further out (5,000,000 cycles ≈ 58 days) it must still hold.
    let r2 = onset_drift_probe(ONSET_PROBE_CODE, ONSET_PROBE_GRID, 5_000_000.0, 2.0, SR);
    assert!(
        r2.late_onsets >= 3 && r2.late_max_deviation <= tol,
        "onsets drifted at 5e6 cycles (dev={}): {r2:?}",
        r2.late_max_deviation
    );
}

// ===========================================================================
// LEVEL 3 — audio characteristics: RMS bounded & stationary, no NaN/Inf, no
// stuck-silent / stuck-loud, bounded resource proxy across repeated swaps.
// ===========================================================================
#[test]
fn test_soak_level3_audio_clean_and_resource_bounded() {
    let thr = Thresholds::default();
    // A couple of seeds so a single lucky swap sequence can't hide a defect.
    for seed in [2u64, 40, 123] {
        let report = run_soak(&SoakConfig::ci(seed));
        assert!(report.swaps >= 4, "soak did not exercise enough swaps: {}", report.summary(&thr));

        // No non-finite output, raw or sanitised.
        assert_eq!(report.nan_samples, 0, "NaN in output: {}", report.summary(&thr));
        assert_eq!(report.inf_samples, 0, "Inf in output: {}", report.summary(&thr));
        assert_eq!(
            report.raw_nonfinite_samples, 0,
            "raw internal blow-up: {}",
            report.summary(&thr)
        );

        // No stuck-silent / stuck-loud / clipped windows.
        assert_eq!(report.voiceless_windows, 0, "voiceless window: {}", report.summary(&thr));
        assert_eq!(report.stuck_loud_blocks, 0, "stuck-loud window: {}", report.summary(&thr));
        assert_eq!(report.clip_blocks, 0, "severe clipping: {}", report.summary(&thr));
        assert_eq!(report.stuck_output_events, 0, "stuck output on swap: {}", report.summary(&thr));

        // RMS bounded & roughly stationary (no slow blow-up or decay).
        assert!(report.rms_in_band, "RMS out of band: {}", report.summary(&thr));
        assert!(!report.rms_growth_detected, "RMS growth: {}", report.summary(&thr));

        // Bounded resource proxy across every swap.
        assert!(report.node_growth.is_none(), "node-count grew: {}", report.summary(&thr));
        assert!(report.voice_pool_growth.is_none(), "voice-pool grew (G4): {}", report.summary(&thr));
        assert!(!report.stuck_voice_detected, "stuck voices: {}", report.summary(&thr));
        assert!(
            report.max_voice_pool <= thr.voice_ceiling,
            "voice pool {} exceeded ceiling {}",
            report.max_voice_pool,
            thr.voice_ceiling
        );

        assert!(report.is_clean(&thr), "soak not clean: {}", report.summary(&thr));
    }
}

// ===========================================================================
// NOISE-HEAVY — deterministic (regresses if wave3-noise-rng-hotpath reverts to
// per-sample thread_rng()) and clean over a soak.
// ===========================================================================
#[test]
fn test_soak_noise_heavy_deterministic_and_clean() {
    // Determinism: two independently-compiled graphs of the same noise program
    // must produce BIT-IDENTICAL output. A per-sample thread_rng() would diverge.
    let nd = noise_determinism_probe(NOISE_DET_CODE, SR as usize, SR);
    assert!(
        nd.bit_identical,
        "noise output not reproducible across graphs (max_diff={:.3e}) — \
         per-node seeded PRNG regressed to thread_rng()?",
        nd.max_abs_diff
    );
    assert!(nd.finite, "noise output has non-finite samples");
    assert!(nd.rms > 0.01, "noise output too quiet (rms={:.4})", nd.rms);

    // The same holds for a filtered pink+white bed and for hpf noise-perc.
    for code in [
        "tempo: 1.0\nout $ pink * 0.4",
        "tempo: 1.0\n~n $ noise # hpf 4000 0.6 * 0.3\nout $ s \"bd*4\" * 0.6 + ~n",
        "tempo: 1.0\nout $ noise # lpf 3000 0.7 * 0.4",
    ] {
        let d = noise_determinism_probe(code, (SR / 2.0) as usize, SR);
        assert!(d.bit_identical, "noise program not reproducible: {code:?} (max_diff={:.3e})", d.max_abs_diff);
        assert!(d.finite, "noise program produced non-finite output: {code:?}");
    }

    // A noise-only soak stays clean over its whole run.
    let thr = Thresholds::default();
    let mut cfg = SoakConfig::ci(9);
    cfg.noise_only = true;
    let report = run_soak(&cfg);
    assert!(report.swaps >= 3, "noise soak did not swap enough: {}", report.summary(&thr));
    assert_eq!(report.nan_samples + report.inf_samples, 0, "noise soak non-finite: {}", report.summary(&thr));
    assert_eq!(report.raw_nonfinite_samples, 0, "noise soak raw blow-up: {}", report.summary(&thr));
    assert!(report.rms_in_band, "noise soak RMS out of band: {}", report.summary(&thr));
    assert!(report.voice_pool_growth.is_none(), "noise soak voice-pool grew: {}", report.summary(&thr));
    assert!(report.is_clean(&thr), "noise soak not clean: {}", report.summary(&thr));
}

// ===========================================================================
// The CI harness must be deterministic: same seed → identical report.
// ===========================================================================
#[test]
fn test_soak_is_deterministic() {
    let mut cfg = SoakConfig::ci(77);
    cfg.target_cycles = 8.0; // keep this one extra short
    let a = run_soak(&cfg);
    let b = run_soak(&cfg);
    assert_eq!(a.swap_sequence, b.swap_sequence, "swap sequence not reproducible");
    assert_eq!(a.blocks_rendered, b.blocks_rendered);
    assert_eq!(a.nan_samples, b.nan_samples);
    assert_eq!(a.max_node_count, b.max_node_count);
    assert_eq!(a.max_active_voices, b.max_active_voices);
    assert_eq!(a.early_rms.to_bits(), b.early_rms.to_bits(), "early RMS not bit-reproducible");
    assert_eq!(a.late_rms.to_bits(), b.late_rms.to_bits(), "late RMS not bit-reproducible");
}
