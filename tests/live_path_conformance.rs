//! Live-path unification **conformance suite** (ENABLER I5 / test-gap P2-A).
//!
//! Design: `docs/audits/design-render-owner-swap-2026-07.md` §6.B.
//!
//! This suite runs the **same scenario matrix** against **every** frontend swap
//! path (`phonon live`, `phonon-audio`, modal editor) through the shared
//! concurrent primitive — `run_concurrent_session_model`, the path/model
//! extension of `run_concurrent_session_mode` (`src/stress_harness.rs:2017`) —
//! and asserts an **identical invariant vector** once the render-owner
//! migration lands.
//!
//! It runs in two modes, so it doubles as the regression gate:
//!
//!   * **BASELINE** (`SwapModel::SharedCellBaseline`) reproduces today's
//!     cross-thread `ArcSwap<RefCell>` swap and *demonstrates* that the R1/R2/R3
//!     windows are present (the retry ceiling that can give up, the transfer
//!     borrow that starves the synth, the voiceless-published window). This is
//!     what makes the POST result meaningful rather than vacuous.
//!   * **POST** (`SwapModel::RenderOwner`) models the render-thread-owned swap —
//!     single owner, SPSC command channel, in-thread buffer-boundary transfer,
//!     graveyard drop — under which **every** invariant is green and the
//!     invariant vector is **identical across all paths** for a given seed.
//!
//! The invariants asserted (design §6.B table):
//!   * No permanent silence / no synth-thread death  — C1
//!   * No "could not transfer state" give-up          — R1
//!   * No synth starvation attributable to a swap      — R2
//!   * No voiceless-published window                   — R3
//!   * Swap seam within the landed D3 crossfade envelope — D3
//!   * Identical invariant vector across every path    — I5

use phonon::stress_harness::{
    known_good_pool, run_concurrent_session_model, ConcurrentReport, ConformanceInvariants,
    LivePath, SessionConfig, SwapModel, Thresholds,
};

/// Swaps per session — enough for the baseline cross-thread contention to
/// exercise the R-windows repeatedly, small enough to keep the matrix quick.
const SWAPS: usize = 12;

/// Seam envelope: the largest sample step a swap boundary may show. The landed
/// D3 crossfade (`transfer_render_continuity`) keeps real seams far below this;
/// a value at/above the catastrophic-click threshold would be a genuine seam
/// regression (crossfade not firing on the post-swap buffer).
fn seam_envelope() -> f32 {
    Thresholds::default().boundary_click_catastrophic
}

/// Run one path under one model for a fixed swap count.
fn run(seed: u64, path: LivePath, model: SwapModel) -> ConcurrentReport {
    let cfg = SessionConfig::ci(seed);
    run_concurrent_session_model(&cfg, &known_good_pool(), SWAPS, path, model)
}

/// The all-green invariant vector every path must show under the render-owner
/// model.
fn all_green() -> ConformanceInvariants {
    ConformanceInvariants {
        synth_thread_alive: true,
        no_permanent_silence: true,
        nonfinite_free: true,
        r1_transfer_never_gave_up: true,
        r2_no_synth_starvation: true,
        r3_no_voiceless_window: true,
        seam_within_envelope: true,
    }
}

// ---------------------------------------------------------------------------
// BASELINE mode: the current protocol exposes the R1/R2/R3 windows.
// ---------------------------------------------------------------------------

/// Across every frontend path, the current shared-cell swap protocol must
/// STRUCTURALLY expose the concurrency hazards the render-owner migration
/// closes — otherwise "all green under render-owner" would prove nothing.
///
/// It must, however, already survive (the `try_borrow_mut`+skip fix keeps the
/// synth thread alive and the ring flowing); the migration removes the
/// *windows*, not a live panic.
#[test]
fn test_baseline_exposes_r1_r2_r3_windows_on_every_path() {
    let seeds = [1u64, 42, 2024];
    // Aggregated harm/window totals (timing-dependent; asserted at matrix level
    // so no single seed can make the suite flaky).
    let mut total_transfer_windows = 0usize; // R2/R3 structural windows opened
    let mut total_voiceless_opened = 0usize; // R3 windows opened
    let mut total_synth_skips = 0usize; // R2 harm (synth starved)
    let mut total_give_ups = 0usize; // R1 harm (transfer gave up)

    for path in LivePath::ALL {
        for &seed in &seeds {
            let r = run(seed, path, SwapModel::SharedCellBaseline);
            let tag = format!("[{}] seed {seed}", path.label());

            // The fixed baseline still runs: no death, no permanent silence, no
            // non-finite output. (The migration is a concurrency refactor, not a
            // crash fix — the crash was already patched.)
            assert!(r.synth_thread_alive, "{tag}: baseline synth thread died: {:?}", r.notes);
            assert!(!r.permanent_silence, "{tag}: baseline went permanently silent");
            assert_eq!(r.nonfinite_in_output, 0, "{tag}: non-finite reached the device");

            // --- Deterministic structural facts (load-robust) --------------
            // R1 structural window: every swap went through the bounded
            // cross-thread retry loop that CAN give up. This is the exact
            // mechanism render-owner deletes (retry_loop_swaps == 0 there).
            assert_eq!(
                r.retry_loop_swaps, r.swaps,
                "{tag}: every baseline swap must use the cross-thread retry ceiling (R1 window)"
            );
            // Each swap is either a completed cross-thread transfer (opening the
            // R2/R3 windows) or a give-up (R1 harm) — accounting must be exact.
            assert_eq!(
                r.transfer_windows + r.could_not_transfer,
                r.retry_loop_swaps,
                "{tag}: transfer/give-up accounting must cover every swap"
            );
            // R3 is structural: a voiceless-published window opens on exactly
            // the swaps whose transfer succeeded.
            assert_eq!(
                r.voiceless_window_opened, r.transfer_windows,
                "{tag}: every completed transfer opens a voiceless-published window (R3)"
            );
            // The suite can observe the R-windows on every session.
            assert!(r.exposes_r_windows(), "{tag}: baseline must expose the R-windows: {r:?}");

            total_transfer_windows += r.transfer_windows;
            total_voiceless_opened += r.voiceless_window_opened;
            total_synth_skips += r.synth_borrow_skips;
            total_give_ups += r.could_not_transfer;
        }
    }

    eprintln!(
        "BASELINE matrix totals: transfer/R2+R3 windows={total_transfer_windows}, \
         R3 voiceless windows={total_voiceless_opened}, R2 synth skips={total_synth_skips}, \
         R1 give-ups={total_give_ups}"
    );

    // R2/R3 windows must actually open somewhere in the matrix (the cross-thread
    // transfer runs at least once): proves the windows are real, not just
    // reachable in principle.
    assert!(
        total_transfer_windows > 0,
        "baseline must open the R2/R3 cross-thread transfer window somewhere in the matrix"
    );
    assert_eq!(
        total_voiceless_opened, total_transfer_windows,
        "R3 window count must track the transfer count across the matrix"
    );
    // R2 harm bites: the synth thread was starved by a transfer borrow at least
    // once (or, under pathological load, the transfer gave up — R1 harm). Either
    // way the cross-thread contention the render-owner model removes is real.
    assert!(
        total_synth_skips + total_give_ups > 0,
        "baseline must exhibit cross-thread contention harm (R2 synth skips and/or R1 give-ups)"
    );
}

// ---------------------------------------------------------------------------
// POST mode: render-owner closes every window, on every path.
// ---------------------------------------------------------------------------

/// Under the render-owner model every invariant is green on every path, and the
/// R1/R2/R3 windows are structurally absent (zero, by construction).
#[test]
fn test_render_owner_all_invariants_green_on_every_path() {
    let seeds = [42u64];
    let env = seam_envelope();

    for path in LivePath::ALL {
        for &seed in &seeds {
            let r = run(seed, path, SwapModel::RenderOwner);
            let tag = format!("[{}] seed {seed}", path.label());

            // C1
            assert!(r.synth_thread_alive, "{tag}: synth thread died: {r:?}");
            assert!(!r.permanent_silence, "{tag}: permanent silence: {r:?}");
            assert_eq!(r.nonfinite_in_output, 0, "{tag}: non-finite output: {r:?}");
            // R1 — no retry ceiling exists, so nothing can give up.
            assert_eq!(r.retry_loop_swaps, 0, "{tag}: render-owner has no retry loop");
            assert_eq!(r.could_not_transfer, 0, "{tag}: R1 give-up must not occur");
            // R2 — no cross-thread borrow, so the synth is never starved.
            assert_eq!(r.transfer_windows, 0, "{tag}: no cross-thread transfer borrow");
            assert_eq!(r.synth_borrow_skips, 0, "{tag}: R2 synth starvation must not occur");
            // R3 — take+install+swap is one render-thread step.
            assert_eq!(r.voiceless_window_opened, 0, "{tag}: R3 window must not open");
            assert_eq!(r.voiceless_window_blocks, 0, "{tag}: no voiceless render blocks");
            // D3 — the seam stays within the crossfade envelope.
            assert!(
                r.max_swap_boundary_delta < env,
                "{tag}: swap seam {:.4} exceeded the D3 envelope {env:.2}",
                r.max_swap_boundary_delta
            );
            // Every swap that was issued was applied.
            assert_eq!(r.swaps, SWAPS, "{tag}: not all swaps were handed to the render thread");

            // The distilled invariant vector is fully green.
            assert_eq!(
                r.invariant_vector(env),
                all_green(),
                "{tag}: invariant vector not all-green: {:?}",
                r.invariant_vector(env)
            );
        }
    }
}

/// I5 core: for a given seed the invariant vector is **identical** across all
/// three paths under the render-owner model — the paths are unified.
#[test]
fn test_render_owner_invariant_vector_identical_across_paths() {
    let env = seam_envelope();
    for seed in [1u64, 42, 2024] {
        let vectors: Vec<(LivePath, ConformanceInvariants)> = LivePath::ALL
            .iter()
            .map(|&p| (p, run(seed, p, SwapModel::RenderOwner).invariant_vector(env)))
            .collect();

        // Every path is all-green...
        for (p, v) in &vectors {
            assert_eq!(
                *v,
                all_green(),
                "seed {seed} [{}]: not all-green under render-owner: {v:?}",
                p.label()
            );
        }
        // ...and therefore identical to the first path's vector.
        let (first_path, first) = vectors[0];
        for (p, v) in &vectors[1..] {
            assert_eq!(
                *v, first,
                "seed {seed}: invariant vector differs between {} and {}: {v:?} vs {first:?}",
                p.label(),
                first_path.label()
            );
        }
    }
}

/// The improvement, pinned per seed/path: the shared-cell baseline exposes the
/// R-windows the render-owner model closes.
#[test]
fn test_render_owner_closes_windows_left_open_by_baseline() {
    let env = seam_envelope();
    for path in LivePath::ALL {
        for seed in [7u64] {
            let base = run(seed, path, SwapModel::SharedCellBaseline);
            let post = run(seed, path, SwapModel::RenderOwner);
            let tag = format!("[{}] seed {seed}", path.label());

            // Baseline uses the hazardous cross-thread retry mechanism on every
            // swap (deterministic); render-owner uses none of it.
            assert_eq!(
                base.retry_loop_swaps, base.swaps,
                "{tag}: baseline must engage the cross-thread retry ceiling (R1 window)"
            );
            assert!(
                base.exposes_r_windows(),
                "{tag}: baseline unexpectedly closed all R-windows: {base:?}"
            );
            assert!(
                !post.exposes_r_windows(),
                "{tag}: render-owner still exposes an R-window: retry={} transfer={} voiceless={}",
                post.retry_loop_swaps,
                post.transfer_windows,
                post.voiceless_window_opened
            );

            // Render-owner closes every window deterministically (== 0).
            assert_eq!(post.retry_loop_swaps, 0, "{tag}: R1 retry loop must be gone");
            assert_eq!(post.transfer_windows, 0, "{tag}: R2 transfer window must be gone");
            assert_eq!(post.voiceless_window_opened, 0, "{tag}: R3 window must be gone");
            assert_eq!(post.synth_borrow_skips, 0, "{tag}: R2 synth starvation must be gone");

            // The migration keeps the seam within the crossfade envelope — it
            // must not introduce a *new* click relative to the current behavior.
            assert!(
                post.max_swap_boundary_delta < env,
                "{tag}: render-owner seam {:.4} exceeded envelope {env:.2}",
                post.max_swap_boundary_delta
            );

            // The render-owner model is fully green where the baseline is not.
            assert_eq!(post.invariant_vector(env), all_green(), "{tag}: POST not all-green");
        }
    }
}

// ---------------------------------------------------------------------------
// Reproducibility: a failing conformance run must be reproducible from its seed.
// ---------------------------------------------------------------------------

/// The seed-driven structural metrics are reproducible across runs (the swap
/// schedule and program choices are pure functions of the seed). Contention
/// outcomes (which transfers won the borrow, how many blocks were skipped) are
/// timing-dependent and intentionally excluded.
#[test]
fn test_conformance_metrics_reproducible_from_seed() {
    for model in [SwapModel::SharedCellBaseline, SwapModel::RenderOwner] {
        let a = run(31337, LivePath::PhononLive, model);
        let b = run(31337, LivePath::PhononLive, model);
        assert_eq!(a.swaps, b.swaps, "{}: swap count not reproducible", model.label());
        assert_eq!(
            a.retry_loop_swaps, b.retry_loop_swaps,
            "{}: retry-loop count not reproducible",
            model.label()
        );
        assert_eq!(
            a.model_baseline, b.model_baseline,
            "{}: model tag not stable",
            model.label()
        );
        assert_eq!(a.path_label, b.path_label, "{}: path label not stable", model.label());
    }
}
