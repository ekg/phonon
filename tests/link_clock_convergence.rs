//! End-to-end verification of the network-tempo-sync (Ableton Link) sub-graph:
//! the **convergence / no-teleport** guarantee that makes a Phonon `LiveClock`
//! *follow* a shared network tempo+phase without ever violating the T1 timing
//! invariant.
//!
//! This is the marquee test called for by
//! `docs/audits/design-ableton-link-2026-07.md` §6 (test plan) and §9. It drives
//! the *real* `LiveClock` (`src/unified_graph.rs`) from a mock `TempoSource`
//! (`src/link_clock.rs`) with a fixed phase offset and asserts the three
//! properties of the design's §4.3 phase-correction scheme:
//!
//! - **(a) `position()` is monotonic non-decreasing every steady buffer** — the
//!   accumulated clock never teleports backwards; folding an external clock is a
//!   controlled varispeed, never a per-buffer snap (invariant **T1**,
//!   `src/unified_graph.rs` `LiveClock`).
//! - **(b) phase error decays to ~0 within N buffers** — the bounded (≤0.5 %)
//!   varispeed nudge (`link_clock::nudged_cps`) actually locks onto the network.
//! - **(c) `set_position` is called ONLY at join / large-error, never per steady
//!   buffer** — the T1 guard for the Link path. A steady buffer corrects phase by
//!   folding the error into `set_cps`, never by a `set_position` jump.
//!
//! It also pins the **C1** render-owner invariant (the graph stays `Send`-only,
//! never `Sync`) for the Link work specifically, and — under `--features link` —
//! drives the same fold over the real `rusty_link` backend so the native lane is
//! exercised end to end. The default build below has **zero** native dependency.

use phonon::link_clock::{
    needs_hard_reseek, nudged_cps, snapshot_from_source, LinkSnapshot, MockTempoSource,
    TempoSource, DEFAULT_BEATS_PER_CYCLE,
};
use phonon::unified_graph::{LiveClock, UnifiedSignalGraph};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Render config for the simulated buffers — realistic values (48 kHz, 512-frame
/// buffers). Only the ratio `FRAMES / SAMPLE_RATE` (the buffer duration) matters
/// to the math.
const SAMPLE_RATE: f32 = 48_000.0;
const FRAMES: usize = 512;
const BEATS_PER_CYCLE: f64 = DEFAULT_BEATS_PER_CYCLE; // one Link bar == one cycle
const BASE_CPS: f32 = 0.5; // 120 BPM @ 4 beats/cycle

/// Why a `set_position` (reseek) happened. Reseeks are *deliberate* jumps and may
/// occur ONLY for these two reasons — never on a steady buffer (design §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reseek {
    /// First lock onto the network (regime 1).
    Join,
    /// Phase error past the hard-reseek threshold; a soft slew would be audibly
    /// long, so fall back to a single hard reseek (regime 3).
    LargeError,
}

/// Faithful in-test model of the control→render *fold* that both live frontends
/// perform once per buffer (`src/main.rs` render loop / `phonon-audio`
/// `link_reader_step`, design §4.3 and §5).
///
/// The frontends themselves live in binary crates, so they cannot be linked from
/// an integration test; this reconstructs their per-buffer step from the same
/// public `link_clock` primitives (`snapshot_from_source`, `nudged_cps`,
/// `needs_hard_reseek`) driving the same public `LiveClock`, and *instruments*
/// every reseek so the "no per-buffer teleport" property is machine-checkable.
struct LinkFollowerSim {
    /// Whether we have performed the one-time join reseek yet.
    joined: bool,
    /// Publisher generation counter (see `LinkSnapshot::epoch`).
    epoch: u64,
    /// Every `set_position` call: `(buffer_index, reason)`. Its length is the
    /// total `set_position` count; it must never contain a steady buffer.
    reseeks: Vec<(usize, Reseek)>,
    /// Number of steady-state buffers that folded phase error into `set_cps`
    /// (regime 2). Used to prove a stopped transport does *not* nudge tempo.
    steady_set_cps: usize,
}

impl LinkFollowerSim {
    fn new() -> Self {
        Self {
            joined: false,
            epoch: 0,
            reseeks: Vec::new(),
            steady_set_cps: 0,
        }
    }

    /// Fold one snapshot (sampled at presentation time `at`) into `live` and
    /// advance exactly one buffer. Returns `live.position()` after advancing.
    ///
    /// This mirrors the design's three regimes precisely:
    /// 1. **not yet joined** → one deliberate `set_position` (join),
    /// 2. **|err| past threshold** → one deliberate `set_position` (hard reseek),
    /// 3. **steady** → bounded varispeed folded into `set_cps`, **no** reseek.
    ///
    /// A stopped transport is a no-op that leaves the clock untouched (mirrors the
    /// frontends' sentinel/stopped handling): no reseek, no nudge, no advance.
    fn step<S: TempoSource + ?Sized>(
        &mut self,
        live: &mut LiveClock,
        src: &S,
        at: Instant,
        buffer_idx: usize,
    ) -> f64 {
        let snap = snapshot_from_source(src, at, BEATS_PER_CYCLE, self.epoch);
        self.epoch += 1;

        if !snap.playing {
            // Sentinel / stopped transport: never touch the clock.
            return live.position();
        }

        let err = snap.target_cycle - live.position();

        if !self.joined {
            // Regime 1 — join: adopt the network tempo and phase once.
            live.set_cps(snap.cps as f32);
            live.set_position(snap.target_cycle);
            self.joined = true;
            self.reseeks.push((buffer_idx, Reseek::Join));
        } else if needs_hard_reseek(err) {
            // Regime 3 — large error: a single deliberate hard reseek.
            live.set_cps(snap.cps as f32);
            live.set_position(snap.target_cycle);
            self.reseeks.push((buffer_idx, Reseek::LargeError));
        } else {
            // Regime 2 — steady state: bounded varispeed, NEVER a set_position.
            // Position stays accumulated and monotonic (T1) while it converges.
            live.set_cps(nudged_cps(snap.cps, err) as f32);
            self.steady_set_cps += 1;
        }

        live.advance_buffer(FRAMES);
        live.position()
    }

    /// Total number of `set_position` calls (all reseeks).
    fn set_position_count(&self) -> usize {
        self.reseeks.len()
    }

    /// Number of hard reseeks triggered by a large phase error (regime 3).
    fn large_error_reseeks(&self) -> usize {
        self.reseeks
            .iter()
            .filter(|(_, r)| *r == Reseek::LargeError)
            .count()
    }
}

/// Presentation time of buffer `buffer_idx`, as a derived `Instant` — no real
/// sleeping, so the mock's constant-tempo timeline is exact and the test is
/// deterministic.
fn presentation_time(base: Instant, buffer_idx: usize) -> Instant {
    base + Duration::from_secs_f64(buffer_idx as f64 * FRAMES as f64 / SAMPLE_RATE as f64)
}

/// Phase error the follower would see at buffer `buffer_idx` given the clock's
/// current position — `target_cycle(at) − live.position()`.
fn phase_error<S: TempoSource>(src: &S, base: Instant, buffer_idx: usize, live: &LiveClock) -> f64 {
    let snap = snapshot_from_source(src, presentation_time(base, buffer_idx), BEATS_PER_CYCLE, 0);
    snap.target_cycle - live.position()
}

// ---------------------------------------------------------------------------
// (a)+(b)+(c) — the marquee test: steady-state convergence with no teleport.
// ---------------------------------------------------------------------------

#[test]
fn steady_state_varispeed_converges_without_teleport() {
    let base = Instant::now();

    // A mock source with a FIXED phase offset: at t=base it sits exactly 0.05
    // cycle AHEAD of the clock's start (0.0). base_beat = 0.05 * beats_per_cycle
    // => target_cycle(base) = 0.05. It advances at 120 BPM (0.5 cps), the same
    // rate the clock runs at, so the 0.05 offset is the residual to be closed.
    let src = MockTempoSource::with_origin(120.0, BEATS_PER_CYCLE, base, 0.05 * BEATS_PER_CYCLE);
    let mut live = LiveClock::new(SAMPLE_RATE, BASE_CPS, 0.0);

    let mut sim = LinkFollowerSim::new();
    // This scenario exercises the STEADY state only — the join reseek is covered
    // by `join_reseeks_once_then_tracks_without_teleport`. Start already locked.
    sim.joined = true;

    let err0 = phase_error(&src, base, 0, &live).abs();
    assert!(
        (err0 - 0.05).abs() < 1e-9,
        "seed offset should be 0.05 cycle, got {err0}"
    );
    assert!(
        !needs_hard_reseek(err0),
        "seed offset must be in the soft band (else it would hard-reseek)"
    );

    // ~213 s of simulated audio: many time-constants of the deliberately gentle
    // (≤0.5 %) nudge. Pure arithmetic — runs instantly.
    let n = 20_000usize;
    let mut prev_pos = live.position();
    for i in 0..n {
        let at = presentation_time(base, i);
        let pos = sim.step(&mut live, &src, at, i);

        // (a) T1: accumulated position never runs backwards on a steady buffer.
        assert!(
            pos >= prev_pos - 1e-12,
            "position teleported backwards at buffer {i}: {prev_pos} -> {pos}"
        );
        prev_pos = pos;
    }

    // (c) The T1 guard: NOT ONE steady buffer performed a set_position.
    assert_eq!(
        sim.set_position_count(),
        0,
        "steady state must never reseek (set_position) — got {:?}",
        sim.reseeks
    );

    // (b) Phase error decayed to ~0 (locked onto the network).
    let err_n = phase_error(&src, base, n, &live).abs();
    assert!(
        err_n < err0 * 0.05,
        "phase error did not converge: {err0} -> {err_n}"
    );
    assert!(err_n < 1e-3, "residual phase error too large: {err_n}");
}

// ---------------------------------------------------------------------------
// Join: exactly one set_position (regime 1), then no-teleport tracking.
// ---------------------------------------------------------------------------

#[test]
fn join_reseeks_once_then_tracks_without_teleport() {
    let base = Instant::now();

    // Network far from the clock's start (target ~10.0 vs clock 0.0) so the join
    // is a genuine, observable reseek.
    let src = MockTempoSource::with_origin(120.0, BEATS_PER_CYCLE, base, 10.0 * BEATS_PER_CYCLE);
    let mut live = LiveClock::new(SAMPLE_RATE, BASE_CPS, 0.0);
    let mut sim = LinkFollowerSim::new();

    let n = 2_000usize;
    let mut prev_pos = f64::NEG_INFINITY;
    for i in 0..n {
        let at = presentation_time(base, i);
        let pos = sim.step(&mut live, &src, at, i);

        if i == 0 {
            // Buffer 0 is the join: the clock snapped forward onto the network.
            assert!(pos > 9.9, "join should snap to the network (~10.0), got {pos}");
        } else {
            // After the join, steady tracking: monotonic, never teleporting.
            assert!(
                pos >= prev_pos - 1e-12,
                "teleport after join at buffer {i}: {prev_pos} -> {pos}"
            );
        }
        prev_pos = pos;
    }

    // Exactly one reseek, and it was the JOIN at buffer 0 — nothing since.
    assert_eq!(
        sim.reseeks,
        vec![(0, Reseek::Join)],
        "must reseek exactly once (the join at buffer 0): {:?}",
        sim.reseeks
    );

    // The clock now tracks the network with ~0 residual.
    let err_n = phase_error(&src, base, n, &live).abs();
    assert!(err_n < 1e-3, "post-join tracking error too large: {err_n}");
}

// ---------------------------------------------------------------------------
// Large error: a single hard reseek (regime 3), then re-lock — no per-buffer snap.
// ---------------------------------------------------------------------------

#[test]
fn large_phase_error_triggers_single_hard_reseek() {
    let base = Instant::now();

    // Two views of the same 120 BPM timeline that differ by a +0.8-cycle phase
    // step (a network jump / new peer): switching from `a` to `b` mid-run injects
    // an |err| ≈ 0.8 > 0.5 (the hard-reseek threshold) at one buffer only.
    let src_a = MockTempoSource::with_origin(120.0, BEATS_PER_CYCLE, base, 0.0);
    let src_b = MockTempoSource::with_origin(120.0, BEATS_PER_CYCLE, base, 0.8 * BEATS_PER_CYCLE);

    let mut live = LiveClock::new(SAMPLE_RATE, BASE_CPS, 0.0);
    let mut sim = LinkFollowerSim::new();

    let switch = 500usize;
    let n = 3_000usize;
    for i in 0..n {
        let at = presentation_time(base, i);
        let src: &MockTempoSource = if i < switch { &src_a } else { &src_b };
        sim.step(&mut live, src, at, i);
    }

    // First reseek is the join at buffer 0.
    assert_eq!(
        sim.reseeks.first(),
        Some(&(0, Reseek::Join)),
        "first reseek must be the join at buffer 0: {:?}",
        sim.reseeks
    );
    // Exactly ONE large-error hard reseek, and it lands on the phase-jump buffer.
    assert_eq!(
        sim.large_error_reseeks(),
        1,
        "the +0.8-cycle jump must cause exactly one hard reseek: {:?}",
        sim.reseeks
    );
    let hard = sim
        .reseeks
        .iter()
        .find(|(_, r)| *r == Reseek::LargeError)
        .copied()
        .unwrap();
    assert_eq!(hard.0, switch, "hard reseek must land on the jump buffer");

    // After the reseek, re-locked and tracking the new phase with ~0 residual.
    let err_n = phase_error(&src_b, base, n, &live).abs();
    assert!(err_n < 1e-3, "post-reseek tracking error too large: {err_n}");
}

// ---------------------------------------------------------------------------
// Stopped transport / sentinel: a pure no-op that leaves the clock untouched.
// ---------------------------------------------------------------------------

#[test]
fn stopped_transport_leaves_clock_untouched() {
    let base = Instant::now();

    // A stopped source whose phase is wildly ahead — a naive fold would teleport.
    let mut src = MockTempoSource::with_origin(120.0, BEATS_PER_CYCLE, base, 40.0 * BEATS_PER_CYCLE);
    src.set_playing(false);

    let start_pos = 1.234_f64;
    let mut live = LiveClock::new(SAMPLE_RATE, BASE_CPS, start_pos);
    let mut sim = LinkFollowerSim::new();

    for i in 0..100 {
        let at = presentation_time(base, i);
        sim.step(&mut live, &src, at, i);
    }

    assert_eq!(sim.set_position_count(), 0, "stopped transport must not reseek");
    assert_eq!(sim.steady_set_cps, 0, "stopped transport must not nudge tempo");
    assert!(!sim.joined, "must not 'join' a stopped transport");
    assert_eq!(
        live.position(),
        start_pos,
        "stopped transport must leave the clock position untouched"
    );
}

// ---------------------------------------------------------------------------
// C1 (regression) + Link publish-safety (design §5).
// ---------------------------------------------------------------------------

/// Autoref specialization probe: reports whether `T: Sync` at compile time
/// *without* requiring the bound (so it compiles for `!Sync` types too). The
/// inherent method (valid only for `T: Sync`) shadows the trait fallback; method
/// resolution picks it when the bound holds and falls back to `false` otherwise.
/// (Mirrors `tests/render_owner_graph_ownership.rs`.)
struct SyncProbe<T>(PhantomData<T>);
trait SyncFallback {
    fn probe_is_sync(&self) -> bool {
        false
    }
}
impl<T> SyncFallback for SyncProbe<T> {}
impl<T: Sync> SyncProbe<T> {
    fn probe_is_sync(&self) -> bool {
        true
    }
}

#[test]
fn link_path_keeps_graph_send_not_sync_and_snapshot_is_publishable() {
    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_copy<T: Copy>() {}

    // The probe must actually discriminate, else the !Sync assertion is vacuous.
    assert!(
        SyncProbe::<i32>(PhantomData).probe_is_sync(),
        "probe broken: i32 is Sync but probe said !Sync"
    );
    assert!(
        !SyncProbe::<Rc<i32>>(PhantomData).probe_is_sync(),
        "probe broken: Rc<i32> is !Sync but probe said Sync"
    );

    // C1: the graph is `Send` (moved control-thread -> render-thread through the
    // render_swap channel) but must stay NOT `Sync`. The Link work folds updates
    // into the *clock*, never re-sharing the graph — so this must not regress.
    assert_send::<UnifiedSignalGraph>();
    assert!(
        !SyncProbe::<UnifiedSignalGraph>(PhantomData).probe_is_sync(),
        "UnifiedSignalGraph is Sync — the C1-root `unsafe impl Sync` has been \
         reintroduced. The Link path must fold updates into the clock via the \
         existing single-writer paths, never re-share the graph across threads \
         (design-ableton-link-2026-07.md §5)."
    );

    // Link publish-safety (design §5): the derived snapshot is `Copy` POD and
    // `Send + Sync`, so the control-side reader can hand it to the render thread
    // through a lock-free `ArcSwap<LinkSnapshot>` — no lock on the render path.
    assert_send_sync::<LinkSnapshot>();
    assert_copy::<LinkSnapshot>();
}

// ---------------------------------------------------------------------------
// `--features link`: the real rusty_link backend drives a LiveClock end to end.
// Compiled ONLY under `--features link`; the default build has zero native dep.
// ---------------------------------------------------------------------------

#[cfg(feature = "link")]
#[test]
fn rusty_link_backend_drives_liveclock_without_teleport() {
    use phonon::link_backend_rusty::RustyLinkTempoSource;

    /// Force `playing = true` so the fold runs regardless of Link's StartStopSync
    /// default; delegates every other query to the real backend.
    struct ForcePlaying<S>(S);
    impl<S: TempoSource> TempoSource for ForcePlaying<S> {
        fn tempo_bpm(&self) -> f64 {
            self.0.tempo_bpm()
        }
        fn beat_at(&self, at: Instant) -> f64 {
            self.0.beat_at(at)
        }
        fn quantum(&self) -> f64 {
            self.0.quantum()
        }
        fn is_playing(&self) -> bool {
            true
        }
    }

    let base = Instant::now();
    // Disabled session: no network, constant 120 BPM — a real `TempoSource`.
    let src = ForcePlaying(RustyLinkTempoSource::new(120.0));
    let mut live = LiveClock::new(SAMPLE_RATE, BASE_CPS, 0.0);
    let mut sim = LinkFollowerSim::new();

    let n = 3_000usize;
    let mut prev_pos = f64::NEG_INFINITY;
    for i in 0..n {
        let at = presentation_time(base, i);
        let pos = sim.step(&mut live, &src, at, i);
        if i > 0 {
            assert!(
                pos >= prev_pos - 1e-9,
                "teleport at buffer {i} over the real backend: {prev_pos} -> {pos}"
            );
        }
        prev_pos = pos;
    }

    // Join once, then track a constant tempo with no hard reseek.
    assert_eq!(
        sim.reseeks.first().map(|(_, r)| *r),
        Some(Reseek::Join),
        "first reseek must be the join: {:?}",
        sim.reseeks
    );
    assert_eq!(
        sim.large_error_reseeks(),
        0,
        "constant-tempo backend must not need a hard reseek: {:?}",
        sim.reseeks
    );

    // The real backend maps 120 BPM -> 0.5 cps, and the clock tracks it.
    let snap = snapshot_from_source(&src, presentation_time(base, n), BEATS_PER_CYCLE, 0);
    assert!((snap.cps - 0.5).abs() < 1e-6, "real backend cps {}", snap.cps);
    let err_n = (snap.target_cycle - live.position()).abs();
    assert!(err_n < 1e-2, "real backend tracking error too large: {err_n}");
}
