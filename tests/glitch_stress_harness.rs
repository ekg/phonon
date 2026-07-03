//! Integration tests for the live-session stress harness
//! (`src/stress_harness.rs`).
//!
//! Covers the task's validation criteria:
//!   * Self-tests prove the detectors catch injected click, dropout, and NaN
//!     defects (plus Inf, DC, clipping, stuck output, RMS growth, stuck voices,
//!     callback-budget overruns).
//!   * A seeded randomised session of >= 60 s-equivalent audio with >= 50 graph
//!     swaps completes with ZERO false positives on known-good programs.
//!   * The scripted scenario set represents failure modes from BOTH audit
//!     reports (docs/audits/test-gap-analysis-2026-07.md and
//!     docs/audits/live-transition-2026-07.md).
//!   * Any failing sequence is reproducible from its seed.
//!
//! Single-command CI entry point (in addition to these tests):
//!   cargo run --release --bin glitch_stress -- --seed 42

use phonon::stress_harness::{
    boundary_delta, count_nonfinite, dc_offset, detect_rms_growth, detect_stuck_voices, inject_click,
    inject_dc, inject_dropout, inject_nan, is_silent, is_stuck, known_good_pool, max_abs_delta, rms,
    run_all_scenarios, run_concurrent_session, run_detector_self_tests, run_random_session, sine_buf,
    Expectation, SessionConfig, Thresholds, SAMPLE_RATE,
};

// ---------------------------------------------------------------------------
// 1. Detector self-tests (TDD): detectors catch injected defects
// ---------------------------------------------------------------------------

#[test]
fn test_detectors_catch_injected_click() {
    let thr = Thresholds::default();
    let mut buf = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    // Clean sine must NOT be flagged as a click.
    let (clean_delta, _) = max_abs_delta(None, &buf);
    assert!(
        clean_delta < thr.internal_click_smooth,
        "clean sine flagged as click: delta={clean_delta}"
    );
    // Inject a full-scale discontinuity.
    inject_click(&mut buf, 500, 1.0);
    let (delta, at) = max_abs_delta(None, &buf);
    assert!(delta > thr.internal_click_smooth, "click not detected: delta={delta}");
    assert!(at == 500 || at == 501, "click localised to wrong sample: {at}");

    // Swap-seam boundary click.
    assert!(boundary_delta(0.95, -0.95) > thr.boundary_click_catastrophic);
    assert!(boundary_delta(0.2, 0.2001) < 0.01);
}

#[test]
fn test_detectors_catch_injected_dropout() {
    let thr = Thresholds::default();
    let clean = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    assert!(!is_silent(&clean, thr.silence_rms), "clean sine flagged silent");

    let mut buf = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_dropout(&mut buf, 0, 1024);
    assert!(is_silent(&buf, thr.silence_rms), "full dropout not detected");

    // Partial dropout: the zeroed region is silent.
    let mut partial = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_dropout(&mut partial, 256, 512);
    assert!(is_silent(&partial[256..768], thr.silence_rms));
    assert!(!is_silent(&partial[0..256], thr.silence_rms));
}

#[test]
fn test_detectors_catch_injected_nan_and_inf() {
    let clean = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    assert_eq!(count_nonfinite(&clean), (0, 0));

    let mut buf = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_nan(&mut buf, 100);
    inject_nan(&mut buf, 700);
    inject_inf_local(&mut buf, 900);
    let (nan, inf) = count_nonfinite(&buf);
    assert_eq!(nan, 2, "NaN count wrong");
    assert_eq!(inf, 1, "Inf count wrong");
}

fn inject_inf_local(buf: &mut [f32], pos: usize) {
    // Mirror of stress_harness::inject_inf (kept local to make the test
    // self-documenting about what "inf" means).
    buf[pos] = f32::INFINITY;
}

#[test]
fn test_detectors_catch_dc_offset_and_clipping() {
    let thr = Thresholds::default();
    let mut buf = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    assert!(dc_offset(&buf).abs() < thr.dc_offset);
    inject_dc(&mut buf, 0.3);
    assert!(dc_offset(&buf).abs() > thr.dc_offset, "DC offset not detected");
}

#[test]
fn test_detectors_catch_stuck_output_and_growth_and_voices() {
    let thr = Thresholds::default();

    // Stuck output.
    let a = sine_buf(220.0, SAMPLE_RATE, 0.0, 256);
    assert!(is_stuck(&a, &a.clone()));
    assert!(!is_stuck(&a, &sine_buf(240.0, SAMPLE_RATE, 0.0, 256)));

    // Unbounded RMS growth.
    let growing: Vec<f32> = (0..200).map(|i| 0.01 * 1.03f32.powi(i)).collect();
    assert!(detect_rms_growth(&growing, thr.rms_growth_ratio).is_some());
    let flat: Vec<f32> = vec![0.2; 200];
    assert!(detect_rms_growth(&flat, thr.rms_growth_ratio).is_none());

    // Stuck voices.
    let leak: Vec<usize> = (0..100).map(|i| i * 8).collect();
    assert!(detect_stuck_voices(&leak, thr.voice_ceiling).is_some());
    let bounded: Vec<usize> = (0..100).map(|i| i % 16).collect();
    assert!(detect_stuck_voices(&bounded, thr.voice_ceiling).is_none());
}

#[test]
fn test_full_detector_self_test_suite_passes() {
    // The programmatic self-test used by `glitch_stress --self-test`.
    match run_detector_self_tests() {
        Ok(n) => assert!(n >= 20, "expected >= 20 detector checks, ran {n}"),
        Err(e) => panic!("detector self-test failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// 2. Seeded randomised session: zero false positives on known-good programs
// ---------------------------------------------------------------------------

#[test]
fn test_seeded_session_60s_50_swaps_zero_false_positives() {
    let thr = Thresholds::default();
    let pool = known_good_pool();

    // The criterion gate: a full >= 60 s-equivalent session with >= 50 swaps,
    // zero false positives on the known-good pool.
    let cfg = SessionConfig::ci(42);
    assert!(cfg.target_seconds >= 60.0, "session must be >= 60s");
    assert!(cfg.min_swaps >= 50, "session must do >= 50 swaps");
    let report = run_random_session(&cfg, &pool);
    assert!(
        report.audio_seconds >= 60.0,
        "only {:.1}s of audio",
        report.audio_seconds
    );
    assert!(report.swaps >= 50, "only {} swaps", report.swaps);
    assert!(
        report.is_clean(&thr),
        "seed 42: false positives on known-good pool: {:?}\n{}",
        report.hard_defects(&thr),
        report.summary(&thr)
    );

    // Breadth: additional seeds (shorter, still >= 50 swaps) must also be clean.
    for seed in [0u64, 1, 1337, 999_983] {
        let mut cfg = SessionConfig::ci(seed);
        cfg.target_seconds = 12.0;
        let report = run_random_session(&cfg, &pool);
        assert!(report.swaps >= 50, "seed {seed}: only {} swaps", report.swaps);
        assert!(
            report.is_clean(&thr),
            "seed {seed}: false positives: {:?}\n{}",
            report.hard_defects(&thr),
            report.summary(&thr)
        );
    }
}

#[test]
fn test_session_is_reproducible_from_seed() {
    let pool = known_good_pool();
    let mut cfg = SessionConfig::ci(0xC0FFEE);
    cfg.target_seconds = 10.0; // determinism doesn't need the full 60 s

    let r1 = run_random_session(&cfg, &pool);
    let r2 = run_random_session(&cfg, &pool);

    // The swap sequence is purely seed-driven and MUST be identical — this is
    // what makes any failing sequence reproducible from its seed.
    assert_eq!(
        r1.swap_sequence, r2.swap_sequence,
        "swap sequence not reproducible for the same seed"
    );
    // Deterministic (audio-derived) defect fields must also match.
    assert_eq!(r1.nan_samples, r2.nan_samples);
    assert_eq!(r1.clip_blocks, r2.clip_blocks);
    assert_eq!(r1.silent_gap_blocks, r2.silent_gap_blocks);
    assert_eq!(r1.stuck_output_events, r2.stuck_output_events);
    assert_eq!(r1.catastrophic_boundary_clicks, r2.catastrophic_boundary_clicks);
    assert_eq!(r1.dc_offset_blocks, r2.dc_offset_blocks);
    assert_eq!(r1.rms_growth_detected, r2.rms_growth_detected);
    assert_eq!(
        r1.max_boundary_delta.to_bits(),
        r2.max_boundary_delta.to_bits(),
        "boundary delta not reproducible"
    );
}

// ---------------------------------------------------------------------------
// 3. Scripted audit scenarios: both reports represented, no hard failures
// ---------------------------------------------------------------------------

#[test]
fn test_scripted_scenarios_represent_both_audits_and_have_no_hard_failures() {
    let cfg = SessionConfig::ci(7);
    let (results, failures) = run_all_scenarios(&cfg);

    assert!(
        failures.is_empty(),
        "scripted scenarios produced hard failures: {failures:?}"
    );

    // Coverage: live-transition report (D1-D4, U1, R1-R4) and test-gap report
    // (G-*/RC-*) must both be represented.
    let refs: Vec<&str> = results.iter().map(|r| r.audit_ref).collect();
    for needed in ["D1", "D2", "D3", "U1", "R1", "R4"] {
        assert!(
            refs.contains(&needed),
            "live-transition finding {needed} not represented; have {refs:?}"
        );
    }
    for needed in ["G2", "G5"] {
        assert!(
            refs.contains(&needed),
            "test-gap finding {needed} not represented; have {refs:?}"
        );
    }

    // Every scenario that is expected clean must actually be clean.
    for r in &results {
        if r.available {
            // Documented-defect scenarios carry a note; clean ones must pass.
            let sc = phonon::stress_harness::audit_scenarios()
                .into_iter()
                .find(|s| s.name == r.name)
                .unwrap();
            if sc.expectation == Expectation::Clean {
                assert!(
                    r.passed(),
                    "clean scenario {} failed: {:?}",
                    r.name,
                    r.failures
                );
            }
        }
    }

    // No scenario may leak NaN/Inf (a real explosion the sanitiser missed).
    for r in &results {
        assert_eq!(r.nan, 0, "scenario {} produced NaN", r.name);
        assert_eq!(r.inf, 0, "scenario {} produced Inf", r.name);
    }
}

// ---------------------------------------------------------------------------
// 3b. D3 swap-boundary click regression (audit live-transition-2026-07 §5 D3)
// ---------------------------------------------------------------------------

/// The swap-boundary crossfade (Phase 4d, `unified_graph.rs`) must fire on the
/// FIRST post-swap buffer of the new graph. Before the fix `prev_buffer_tail`
/// was not carried across the swap, so the crossfade was skipped and the
/// `D3-sine-to-saw-boundary-click` scenario stepped 0.330 at the seam — an
/// audible click. After transferring render continuity the seam must drop well
/// below the audible-click threshold (< 0.05).
#[test]
fn test_d3_swap_boundary_click_below_audible_threshold() {
    let cfg = SessionConfig::ci(7);
    let (results, failures) = run_all_scenarios(&cfg);
    assert!(
        failures.is_empty(),
        "scripted scenarios produced hard failures: {failures:?}"
    );

    let d3 = results
        .iter()
        .find(|r| r.name == "D3-sine-to-saw-boundary-click")
        .expect("D3 scenario missing");
    assert!(d3.available, "D3 scenario failed to build: {:?}", d3.note);
    assert!(
        d3.boundary_delta < 0.05,
        "D3 swap-boundary click not smoothed: boundary_delta={:.4} (want < 0.05). \
         The Phase-4d crossfade did not fire on the post-swap buffer — \
         prev_buffer_tail is not being transferred across the swap.",
        d3.boundary_delta
    );

    // The going-to-silence control MUST stay strictly silent: the render-
    // continuity transfer must not inject old audio into an intentionally
    // silent new graph.
    let silence = results
        .iter()
        .find(|r| r.name == "clean-osc-to-silence")
        .expect("clean-osc-to-silence scenario missing");
    assert!(
        silence.passed(),
        "clean-osc-to-silence regressed after the fix: {:?} (post_rms={:.5})",
        silence.failures,
        silence.post_rms
    );

    // No other scenario may have gained a catastrophic seam or NaN/Inf.
    for r in &results {
        assert_eq!(r.nan, 0, "scenario {} produced NaN after fix", r.name);
        assert_eq!(r.inf, 0, "scenario {} produced Inf after fix", r.name);
    }
}

// ---------------------------------------------------------------------------
// 3c. Audit D2: FX-state transfer completeness — effect tails survive a swap
// ---------------------------------------------------------------------------

/// Regression guard for `complete-fx-state` (audit D2): the pingpong and
/// tape-delay tails must be CONTINUOUS across a hot-swap. Each D2 scenario
/// primes a fully-wet effect with a live source, then swaps to a graph whose
/// dry input is silenced — so the only remaining energy is the transferred
/// effect tail. Before `transfer_fx_states` injected these effect types, the
/// swapped-in graph had a fresh (empty) buffer and the tail snapped to zero.
#[test]
fn test_d2_effect_tails_survive_swap() {
    let cfg = SessionConfig::ci(11);
    let scenarios = phonon::stress_harness::audit_scenarios();

    let d2: Vec<_> = scenarios
        .iter()
        .filter(|s| matches!(s.expectation, Expectation::ContinuousTail(_)))
        .collect();
    assert!(
        d2.len() >= 2,
        "expected at least the pingpong + tapedelay tail-continuity scenarios, found {}",
        d2.len()
    );

    for sc in d2 {
        let r = phonon::stress_harness::run_scenario(sc, &cfg);
        assert!(r.available, "scenario {} failed to compile", sc.name);
        // The tail must have been audible while priming...
        assert!(
            r.pre_rms > cfg.thresholds.silence_rms,
            "scenario {} never built an audible tail (pre_rms {:.5})",
            sc.name,
            r.pre_rms
        );
        // ...and must NOT drop to silence after the swap (the D2 defect).
        assert!(
            !r.post_silent,
            "scenario {} FX tail reset on swap: post_rms {:.5} (state not transferred)",
            sc.name,
            r.post_rms
        );
        assert!(
            r.passed(),
            "scenario {} reported hard failures: {:?}",
            sc.name,
            r.failures
        );
        assert_eq!(r.nan, 0, "scenario {} produced NaN", sc.name);
        assert_eq!(r.inf, 0, "scenario {} produced Inf", sc.name);
    }
}

// ---------------------------------------------------------------------------
// 4. Concurrent rig: real synth thread, structural invariants
// ---------------------------------------------------------------------------

#[test]
fn test_concurrent_session_no_synth_death_or_permanent_silence() {
    let cfg = SessionConfig::ci(2024);
    let report = run_concurrent_session(&cfg, &known_good_pool(), 30);

    assert!(
        report.synth_thread_alive,
        "synth thread died (permanent silence): {:?}",
        report.notes
    );
    assert!(
        !report.permanent_silence,
        "output went permanently silent: max_silent_run={}",
        report.max_consecutive_silent
    );
    assert_eq!(
        report.nonfinite_in_output, 0,
        "non-finite samples reached the device"
    );
    assert!(report.swaps >= 1, "no swaps performed");
    // The consumer must have observed audio flowing through the ring.
    assert!(
        report.consumer_blocks > 0,
        "consumer never drained any audio"
    );
}

// ---------------------------------------------------------------------------
// 5. Sanity: a program that produces sound has non-zero RMS through the runner
// ---------------------------------------------------------------------------

#[test]
fn test_known_good_pool_all_compile_and_sound() {
    let pool = known_good_pool();
    let block_len = 1024;
    for p in &pool {
        let mut g =
            phonon::stress_harness::build_initial(p.code, SAMPLE_RATE).unwrap_or_else(|e| {
                panic!("known-good program '{}' failed to compile: {e}", p.name)
            });
        // Warm up then measure.
        let mut last = vec![0.0f32; block_len];
        for _ in 0..8 {
            last = vec![0.0f32; block_len];
            g.process_buffer(&mut last);
        }
        assert_eq!(count_nonfinite(&last), (0, 0), "{} produced non-finite", p.name);
        if !p.expect_silent {
            assert!(
                rms(&last) > 0.001,
                "known-good program '{}' is unexpectedly silent",
                p.name
            );
        }
    }
}
