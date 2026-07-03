//! F-4 regression (task `preallocate-voice-pool`, rt-safety audit F-4).
//!
//! Steady-state dense sample triggering must not grow the voice pool on the
//! synth thread, and must not produce a per-buffer render-time spike
//! attributable to voice-pool growth.
//!
//! Before the fix, once all voices were busy a new trigger called
//! `grow_voice_pool`, which `push`ed `Voice::new()` in a loop (heap alloc + full
//! Vec realloc/memcpy of a large per-voice struct) **and `eprintln!`ed** the
//! growth — all on the synth thread, exactly when the engine was already
//! overloaded (→ render-time spike + stderr backpressure → underrun). See
//! `docs/audits/rt-safety-2026-07.md` §4 F-4.
//!
//! This test drives the exact hot path the defect lives in — voice allocation +
//! block render — under a saturating dense-trigger load (the `s "bd*16"`
//! layered, long-tail scenario from the audit) and checks it with the stress
//! harness's callback-budget detector.
//!
//! Determinism note: `cargo test` builds unoptimized, where rendering a full
//! pre-grown pool per block is far slower than the real-time deadline, so the
//! budget check uses a **relative** spike budget derived from the session's own
//! steady-state median rather than the absolute callback deadline. A
//! growth-induced realloc/memcpy + `eprintln!` backpressure would surface here
//! as an over-budget outlier block; a pre-grown pool renders uniform blocks.

use phonon::sample_loader::StereoSample;
use phonon::stress_harness::budget_overrun_fraction;
use phonon::voice_manager::VoiceManager;
use std::sync::Arc;
use std::time::Instant;

/// A ~0.18 s stereo sample so triggered voices overlap heavily across blocks
/// (long tails, no cut groups) — the audit's pool-exhaustion scenario.
fn long_tail_sample() -> Arc<StereoSample> {
    let n = 8000;
    let left: Vec<f32> = (0..n).map(|i| (i as f32 * 0.02).sin() * 0.4).collect();
    let right = left.clone();
    Arc::new(StereoSample::stereo(left, right))
}

#[test]
fn dense_triggering_no_pool_growth_and_no_render_spike() {
    let mut vm = VoiceManager::new();
    let ceiling = vm.voice_ceiling();

    // The product path pre-grows the pool to its ceiling at construction, off
    // the synth thread — so the synth thread never has to allocate under load.
    assert_eq!(
        vm.pool_size(),
        ceiling,
        "pool must be pre-grown to the ceiling at construction"
    );

    let sample = long_tail_sample();
    let block = 256usize;
    let warmup = 24usize;
    let blocks = 160usize;
    // 64 fresh voices per block, each sustained by its release tail across many
    // blocks, drives the active count well past the 512 ceiling → the steal
    // fallback is exercised on essentially every block.
    let triggers_per_block = 64usize;

    let mut render_s: Vec<f64> = Vec::with_capacity(blocks);

    for _ in 0..blocks {
        for _ in 0..triggers_per_block {
            vm.trigger_sample(sample.clone(), 0.4);
        }

        let t0 = Instant::now();
        let _ = vm.render_block(block);
        render_s.push(t0.elapsed().as_secs_f64());

        // Invariant on every block: the synth thread never grew the pool, and
        // the pool neither grows nor shrinks — it stays pinned at the ceiling.
        assert_eq!(
            vm.growth_event_count(),
            0,
            "no synth-thread voice-pool growth is allowed during rendering"
        );
        assert_eq!(
            vm.pool_size(),
            ceiling,
            "pool must stay at the ceiling (no growth, no periodic synth-thread shrink)"
        );
    }

    // Saturation beyond the ceiling was absorbed by stealing (the RT-safe
    // fallback), not by allocation — counted atomically for off-thread reporting.
    assert!(
        vm.steal_event_count() > 0,
        "dense load beyond the ceiling must steal voices (got {} steals)",
        vm.steal_event_count()
    );

    // Stress-harness budget detector: after warmup, no block may spike to a
    // large multiple of the steady-state median render time.
    let mut steady: Vec<f64> = render_s[warmup..].to_vec();
    steady.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = steady[steady.len() / 2];
    // Generous spike budget (6× median) so ordinary debug/CI scheduler jitter
    // never trips it; a growth realloc/memcpy + eprintln backpressure would.
    let spike_budget = (p50 * 6.0).max(1e-4);
    let overrun = budget_overrun_fraction(&render_s[warmup..], spike_budget, 1.0);
    assert_eq!(
        overrun, 0.0,
        "render-time spike detected: {:.1}% of blocks exceeded {:.0}us (p50 {:.0}us) — \
         unexpected for a pre-grown, non-allocating pool",
        overrun * 100.0,
        spike_budget * 1e6,
        p50 * 1e6
    );
}
