//! # Link clock — source-agnostic tempo/phase adapter
//!
//! Pure, dependency-free adapter that maps a shared *tempo + beat-phase* (the
//! Ableton Link model) onto Phonon's `cps` / `cycle_position`. See
//! `docs/audits/design-ableton-link-2026-07.md` §4 (mapping) and §5
//! (thread-safety) for the full rationale.
//!
//! ## What this module is (and is not)
//!
//! - It is the **pure math + trait boundary**: BPM↔cps and beat↔cycle
//!   conversions, a bounded varispeed phase-correction, and a hard-reseek
//!   decision. All `f64` end-to-end so it inherits the no-drift guarantee of the
//!   f64 trigger timekeeping (`src/unified_graph.rs:1080`, invariant **T2**).
//! - It carries **no native dependency** and does **no I/O**. The real Ableton
//!   Link backend (`rusty_link`) and the OSC leader/follower backend implement
//!   the [`TempoSource`] trait elsewhere, behind a Cargo feature; this file only
//!   depends on `std`.
//! - It does **not** touch the render-owner clock. The frontends fold a
//!   [`LinkSnapshot`] into the *existing* `LiveClock::set_cps` /
//!   `LiveClock::set_position` (main) or `Cmd::SetTempo` / `Cmd::SetCycle`
//!   (phonon-audio) paths. This keeps `src/render_swap.rs` and
//!   `src/unified_graph.rs` out of the Link work entirely (design §7).
//!
//! ## The mapping (design §4.1)
//!
//! Choosing `beats_per_cycle` (default [`DEFAULT_BEATS_PER_CYCLE`]) so that one
//! Phonon cycle == one Link bar of `quantum = beats_per_cycle` beats:
//!
//! ```text
//! cps          = tempo_bpm / 60 / beats_per_cycle
//! target_cycle = link_beat / beats_per_cycle          // absolute, f64
//! ```
//!
//! ## Phase correction (design §4.3)
//!
//! Link supplies an *absolute* phase; Phonon *accumulates* position from
//! samples. Snapping to Link's phase every buffer would re-create the pt-F1
//! onset clustering (invariant **T1**). Instead:
//!
//! 1. **Join / explicit resync** — a single deliberate `set_position` reseek.
//! 2. **Steady state** — [`phase_nudge`] returns a tiny `cps` *factor* (clamped
//!    to [`MAX_PHASE_NUDGE`], ≤ 0.5 %) folded into `set_cps`, so position stays
//!    accumulated and monotonic while it converges on the network.
//! 3. **Large-error fallback** — [`needs_hard_reseek`] is true past
//!    [`HARD_RESEEK_THRESHOLD_CYCLES`] (half a cycle), where a soft slew would be
//!    audibly long; the frontend falls back to a single hard reseek.

use std::time::Instant;

/// Default beats per Phonon cycle (one Link *bar* == one Phonon cycle).
///
/// Four beats to a bar is the common-time default. It is configurable: every
/// conversion below takes `beats_per_cycle` explicitly so a session can map,
/// e.g., a 3-beat or 7-beat bar onto one cycle.
pub const DEFAULT_BEATS_PER_CYCLE: f64 = 4.0;

/// Maximum fractional `cps` change a single steady-state phase correction may
/// apply, as a fraction of the current rate (`0.005` == 0.5 %).
///
/// A rate change below this is inaudible, so the clock can track the network
/// without any perceptible pitch/tempo wobble (design §4.3).
pub const MAX_PHASE_NUDGE: f64 = 0.005;

/// Proportional gain mapping phase error (in cycles) to a `cps` correction
/// before clamping. With this gain the correction saturates at
/// [`MAX_PHASE_NUDGE`] once `|err|` reaches `MAX_PHASE_NUDGE / PHASE_NUDGE_GAIN`
/// (== 0.1 cycle), so small errors get a proportional nudge and larger errors
/// correct at the maximum safe rate.
pub const PHASE_NUDGE_GAIN: f64 = 0.05;

/// Phase-error magnitude (in cycles) beyond which a soft varispeed correction
/// would take too long / be audible, so the frontend should perform a single
/// hard `set_position` reseek instead (design §4.3, regime 3). Half a cycle.
pub const HARD_RESEEK_THRESHOLD_CYCLES: f64 = 0.5;

/// A source of a shared tempo + beat timeline (the Ableton Link model).
///
/// Implemented by the real `rusty_link` backend, the zero-dep OSC
/// leader/follower backend, and the in-process [`MockTempoSource`] used by
/// tests. The adapter in this module is generic over it, so no backend detail
/// leaks into the cps/phase math.
pub trait TempoSource {
    /// Latest shared tempo, in beats per minute.
    fn tempo_bpm(&self) -> f64;

    /// Beat position on the shared timeline at wall-clock instant `at` — the
    /// Link "beat": a monotonic `f64` count of beats since the session epoch.
    fn beat_at(&self, at: Instant) -> f64;

    /// Bar length in beats (the Link "quantum"). Maps to one Phonon cycle when
    /// `beats_per_cycle == quantum`; used for bar-phase alignment. It does *not*
    /// enter the cps/target-cycle conversion, which uses `beats_per_cycle`.
    fn quantum(&self) -> f64;

    /// Whether the shared transport is currently running.
    fn is_playing(&self) -> bool;
}

/// A lock-free-publishable snapshot of the derived Phonon clock state.
///
/// `Copy` and plain-old-data so it can be published through an
/// `ArcSwap<LinkSnapshot>` by the control-side Link reader thread and `.load()`ed
/// once per buffer by the render thread — no lock on the render path (design
/// §5). This module never shares it across threads itself; it only builds it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkSnapshot {
    /// Cycles per second derived from the shared tempo.
    pub cps: f64,
    /// Absolute Phonon cycle position the shared phase currently points at.
    pub target_cycle: f64,
    /// Monotonic generation counter; bumped by the publisher on every update so
    /// a reader can tell a fresh snapshot from a repeat.
    pub epoch: u64,
    /// Whether the shared transport is playing.
    pub playing: bool,
}

/// Convert a shared tempo (BPM) to Phonon `cps` for a given cycle length.
///
/// `cps = bpm / 60 / beats_per_cycle` (design §4.1).
#[inline]
pub fn bpm_to_cps(bpm: f64, beats_per_cycle: f64) -> f64 {
    bpm / 60.0 / beats_per_cycle
}

/// Inverse of [`bpm_to_cps`]: `bpm = cps * 60 * beats_per_cycle`.
#[inline]
pub fn cps_to_bpm(cps: f64, beats_per_cycle: f64) -> f64 {
    cps * 60.0 * beats_per_cycle
}

/// Convert an absolute Link beat to an absolute Phonon cycle.
///
/// `cycle = beat / beats_per_cycle` (design §4.1).
#[inline]
pub fn beat_to_cycle(beat: f64, beats_per_cycle: f64) -> f64 {
    beat / beats_per_cycle
}

/// Inverse of [`beat_to_cycle`]: `beat = cycle * beats_per_cycle`.
#[inline]
pub fn cycle_to_beat(cycle: f64, beats_per_cycle: f64) -> f64 {
    cycle * beats_per_cycle
}

/// Bounded proportional phase correction, returned as a multiplicative `cps`
/// *factor* near `1.0` (design §4.3, regime 2).
///
/// `err` is the phase error in cycles: `target_cycle - live_position`. A
/// positive `err` means the network is ahead (we are behind) so the factor is
/// `> 1.0` to speed up; a negative `err` yields a factor `< 1.0`. The deviation
/// from `1.0` always has the same sign as `err`, is monotone non-decreasing in
/// `err`, and its magnitude is clamped to [`MAX_PHASE_NUDGE`] (≤ 0.5 %).
#[inline]
pub fn phase_nudge(err: f64) -> f64 {
    // Proportional term, then clamp its magnitude so the going-forward rate
    // change is inaudible. clamp() of a linear term is monotone in `err`, its
    // deviation from 1.0 keeps `err`'s sign, and `err == 0` yields exactly 1.0.
    let correction = (PHASE_NUDGE_GAIN * err).clamp(-MAX_PHASE_NUDGE, MAX_PHASE_NUDGE);
    1.0 + correction
}

/// Apply a [`phase_nudge`] to a `cps` value: `cps * phase_nudge(err)`.
#[inline]
pub fn nudged_cps(cps: f64, err: f64) -> f64 {
    cps * phase_nudge(err)
}

/// Whether a phase error is too large for a soft varispeed correction and needs
/// a single hard `set_position` reseek instead (design §4.3, regime 3).
///
/// True only when `|err|` is strictly greater than
/// [`HARD_RESEEK_THRESHOLD_CYCLES`].
#[inline]
pub fn needs_hard_reseek(err: f64) -> bool {
    err.abs() > HARD_RESEEK_THRESHOLD_CYCLES
}

/// Build a [`LinkSnapshot`] from a [`TempoSource`] sampled at wall-clock instant
/// `at`, mapping through `beats_per_cycle`.
///
/// `epoch` is supplied by the caller (the publisher's generation counter). Pure:
/// the only impurity is whatever `at` the caller captured.
pub fn snapshot_from_source<S: TempoSource + ?Sized>(
    src: &S,
    at: Instant,
    beats_per_cycle: f64,
    epoch: u64,
) -> LinkSnapshot {
    LinkSnapshot {
        cps: bpm_to_cps(src.tempo_bpm(), beats_per_cycle),
        target_cycle: beat_to_cycle(src.beat_at(at), beats_per_cycle),
        epoch,
        playing: src.is_playing(),
    }
}

/// Deterministic in-process [`TempoSource`] for tests — no network, no native
/// dependency. Beats advance linearly from a base instant at the configured
/// tempo.
#[derive(Clone, Debug)]
pub struct MockTempoSource {
    bpm: f64,
    quantum: f64,
    playing: bool,
    base: Instant,
    base_beat: f64,
}

impl MockTempoSource {
    /// A playing source at `bpm`, quantum 4, whose beat timeline starts at 0 at
    /// the moment of construction.
    pub fn new(bpm: f64) -> Self {
        Self::with_origin(bpm, DEFAULT_BEATS_PER_CYCLE, Instant::now(), 0.0)
    }

    /// A source with an explicit beat origin: `base_beat` beats at instant
    /// `base`, advancing at `bpm`.
    pub fn with_origin(bpm: f64, quantum: f64, base: Instant, base_beat: f64) -> Self {
        Self {
            bpm,
            quantum,
            playing: true,
            base,
            base_beat,
        }
    }

    /// Set the tempo (does not move the beat origin).
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
    }

    /// Set the quantum (bar length in beats).
    pub fn set_quantum(&mut self, quantum: f64) {
        self.quantum = quantum;
    }

    /// Start/stop the transport.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }
}

impl TempoSource for MockTempoSource {
    fn tempo_bpm(&self) -> f64 {
        self.bpm
    }
    fn beat_at(&self, at: Instant) -> f64 {
        // Signed elapsed seconds from the base instant: `Instant` cannot express
        // a negative `Duration`, so compute the sign explicitly to keep the beat
        // timeline linear on both sides of `base`.
        let secs = if at >= self.base {
            at.duration_since(self.base).as_secs_f64()
        } else {
            -self.base.duration_since(at).as_secs_f64()
        };
        self.base_beat + secs * self.bpm / 60.0
    }
    fn quantum(&self) -> f64 {
        self.quantum
    }
    fn is_playing(&self) -> bool {
        self.playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const EPS: f64 = 1e-9;

    // ---- required: BPM <-> cps round-trip (f64 exact within 1e-9) ----------

    #[test]
    fn test_bpm_cps_roundtrip() {
        // Known value: 120 BPM, 4 beats/cycle -> 0.5 cps (120 BPM = 2 cps beats,
        // 4 beats per cycle -> 0.5 cycles/sec).
        assert!((bpm_to_cps(120.0, 4.0) - 0.5).abs() < EPS);
        // 174 BPM (DnB), 4/4 -> 0.725 cps.
        assert!((bpm_to_cps(174.0, 4.0) - 0.725).abs() < EPS);

        // Round-trip a spread of tempi and cycle lengths.
        for &bpm in &[60.0, 90.0, 120.0, 128.0, 140.0, 174.0, 200.0_f64] {
            for &bpc in &[1.0, 2.0, 3.0, 4.0, 7.0_f64] {
                let cps = bpm_to_cps(bpm, bpc);
                let back = cps_to_bpm(cps, bpc);
                assert!(
                    (back - bpm).abs() < EPS,
                    "bpm roundtrip {bpm}@{bpc}: got {back}"
                );
            }
        }
    }

    // ---- required: beat <-> cycle round-trip --------------------------------

    #[test]
    fn test_beat_cycle_roundtrip() {
        // 8 beats at 4 beats/cycle == cycle 2.
        assert!((beat_to_cycle(8.0, 4.0) - 2.0).abs() < EPS);
        assert!((cycle_to_beat(2.0, 4.0) - 8.0).abs() < EPS);

        for &beat in &[0.0, 0.25, 1.0, 3.5, 16.0, 1000.0, 123456.789_f64] {
            for &bpc in &[1.0, 2.0, 3.0, 4.0, 7.0_f64] {
                let cycle = beat_to_cycle(beat, bpc);
                let back = cycle_to_beat(cycle, bpc);
                assert!(
                    (back - beat).abs() < EPS,
                    "beat roundtrip {beat}@{bpc}: got {back}"
                );
            }
        }
    }

    // ---- required: phase_nudge clamped + monotone + same-sign --------------

    #[test]
    fn test_phase_nudge_clamped_and_monotone() {
        // Zero error -> exactly no correction.
        assert_eq!(phase_nudge(0.0), 1.0);

        // Small positive error -> factor > 1 but within the clamp.
        let small = phase_nudge(0.01);
        assert!(small > 1.0, "positive err should speed up: {small}");
        assert!(small <= 1.0 + MAX_PHASE_NUDGE + EPS);
        // Proportional region: 0.01 cycle * gain 0.05 = 5e-4 correction.
        assert!((small - (1.0 + PHASE_NUDGE_GAIN * 0.01)).abs() < EPS);

        // Small negative error -> factor < 1, symmetric magnitude.
        let small_neg = phase_nudge(-0.01);
        assert!(small_neg < 1.0, "negative err should slow down: {small_neg}");
        assert!((small_neg - (2.0 - small)).abs() < EPS, "symmetric around 1.0");

        // Clamp: a large error saturates at exactly +/- MAX_PHASE_NUDGE.
        assert!((phase_nudge(10.0) - (1.0 + MAX_PHASE_NUDGE)).abs() < EPS);
        assert!((phase_nudge(-10.0) - (1.0 - MAX_PHASE_NUDGE)).abs() < EPS);
        // Never exceeds the clamp regardless of input.
        for &err in &[0.0, 0.05, 0.1, 0.5, 1.0, 5.0, 1e6, -0.3, -2.0, -1e6_f64] {
            let f = phase_nudge(err);
            assert!(
                (f - 1.0).abs() <= MAX_PHASE_NUDGE + EPS,
                "clamp violated at err={err}: {f}"
            );
            // Same sign as err (deviation of factor from 1.0 tracks err's sign).
            assert!(
                (f - 1.0).signum() == err.signum() || err == 0.0,
                "sign mismatch at err={err}: {f}"
            );
        }

        // Monotone non-decreasing in err.
        let errs = [-1.0, -0.5, -0.2, -0.1, -0.05, 0.0, 0.05, 0.1, 0.2, 0.5, 1.0];
        let mut prev = f64::NEG_INFINITY;
        for &e in &errs {
            let f = phase_nudge(e);
            assert!(f >= prev - EPS, "not monotone at err={e}: {f} < {prev}");
            prev = f;
        }
    }

    // ---- required: hard-reseek threshold -----------------------------------

    #[test]
    fn test_hard_reseek_threshold() {
        // Below / at the threshold -> soft correction (no hard reseek).
        assert!(!needs_hard_reseek(0.0));
        assert!(!needs_hard_reseek(0.25));
        assert!(!needs_hard_reseek(-0.4));
        // Exactly at the threshold is NOT past it.
        assert!(!needs_hard_reseek(HARD_RESEEK_THRESHOLD_CYCLES));
        assert!(!needs_hard_reseek(-HARD_RESEEK_THRESHOLD_CYCLES));
        // Strictly past the threshold (either sign) -> hard reseek.
        assert!(needs_hard_reseek(HARD_RESEEK_THRESHOLD_CYCLES + 1e-6));
        assert!(needs_hard_reseek(-HARD_RESEEK_THRESHOLD_CYCLES - 1e-6));
        assert!(needs_hard_reseek(0.75));
        assert!(needs_hard_reseek(-1.0));
        assert!(needs_hard_reseek(50.0));
    }

    // ---- nudged_cps helper --------------------------------------------------

    #[test]
    fn test_nudged_cps_matches_factor() {
        let cps = 0.5;
        assert!((nudged_cps(cps, 0.0) - cps).abs() < EPS);
        assert!((nudged_cps(cps, 0.01) - cps * phase_nudge(0.01)).abs() < EPS);
        // Bounded: the applied cps never changes by more than MAX_PHASE_NUDGE.
        let hi = nudged_cps(cps, 1e6);
        assert!((hi - cps * (1.0 + MAX_PHASE_NUDGE)).abs() < EPS);
    }

    // ---- mock source + snapshot mapping ------------------------------------

    #[test]
    fn test_mock_source_beat_advances_linearly() {
        let base = Instant::now();
        let src = MockTempoSource::with_origin(120.0, 4.0, base, 0.0);
        // 120 BPM = 2 beats/sec. After 2s, 4 beats have elapsed.
        let two_s = base + Duration::from_secs(2);
        assert!((src.beat_at(two_s) - 4.0).abs() < 1e-9);
        assert!((src.beat_at(base) - 0.0).abs() < 1e-9);
        assert_eq!(src.tempo_bpm(), 120.0);
        assert_eq!(src.quantum(), 4.0);
        assert!(src.is_playing());
    }

    #[test]
    fn test_snapshot_from_mock_source() {
        let base = Instant::now();
        let src = MockTempoSource::with_origin(120.0, 4.0, base, 8.0);
        let snap = snapshot_from_source(&src, base, DEFAULT_BEATS_PER_CYCLE, 7);
        // cps = 120/60/4 = 0.5
        assert!((snap.cps - 0.5).abs() < EPS);
        // beat 8 -> cycle 2
        assert!((snap.target_cycle - 2.0).abs() < EPS);
        assert_eq!(snap.epoch, 7);
        assert!(snap.playing);

        // A snapshot 2s later: 4 more beats -> +1 cycle, same cps.
        let later = base + Duration::from_secs(2);
        let snap2 = snapshot_from_source(&src, later, DEFAULT_BEATS_PER_CYCLE, 8);
        assert!((snap2.cps - 0.5).abs() < EPS);
        assert!((snap2.target_cycle - 3.0).abs() < EPS);
        assert_eq!(snap2.epoch, 8);
    }

    #[test]
    fn test_quantum_independent_of_conversion() {
        // The cps/target_cycle mapping uses beats_per_cycle, NOT the source
        // quantum: changing the quantum leaves the derived clock untouched.
        let base = Instant::now();
        let mut src = MockTempoSource::with_origin(120.0, 4.0, base, 8.0);
        let a = snapshot_from_source(&src, base, 4.0, 0);
        src.set_quantum(8.0);
        let b = snapshot_from_source(&src, base, 4.0, 1);
        assert!((a.cps - b.cps).abs() < EPS);
        assert!((a.target_cycle - b.target_cycle).abs() < EPS);
    }

    #[test]
    fn test_snapshot_reflects_stopped_transport() {
        let base = Instant::now();
        let mut src = MockTempoSource::with_origin(100.0, 4.0, base, 0.0);
        src.set_playing(false);
        let snap = snapshot_from_source(&src, base, DEFAULT_BEATS_PER_CYCLE, 0);
        assert!(!snap.playing);
    }

    // ---- convergence: the varispeed nudge actually reduces phase error ------

    #[test]
    fn test_varispeed_converges_and_stays_monotone() {
        // Simulate the steady-state loop: a LiveClock-like position accumulating
        // by samples, corrected once per buffer by phase_nudge folded into cps.
        // The bounded (<=0.5 %) nudge is deliberately gentle, so this asserts the
        // controller *properties* rather than a fast time-to-lock:
        //   (a) position is monotonic non-decreasing every buffer (no teleport, T1);
        //   (b) the phase error shrinks monotonically toward zero (correctly
        //       signed, stable proportional control);
        //   (c) it does converge given enough buffers.
        let sample_rate = 48_000.0_f64;
        let buffer = 512.0_f64;
        let base_cps = 0.5_f64; // 120 BPM @ 4 bpc

        // Realistic residual offset a few percent of a cycle behind the network
        // (the join/large-error case is handled by the hard reseek, not here).
        let mut position = 0.0_f64;
        let mut network = 0.05_f64;
        let net_inc_per_buffer = base_cps * buffer / sample_rate;

        let err0 = (network - position).abs();
        let mut prev_pos = position;
        let mut prev_err = err0;
        // ~213 s of simulated time — many time-constants of the gentle nudge.
        for _ in 0..20_000 {
            let err = network - position;
            assert!(
                !needs_hard_reseek(err),
                "residual stays in the soft band, no hard reseek"
            );
            let cps = nudged_cps(base_cps, err);
            position += cps * buffer / sample_rate;
            network += net_inc_per_buffer;

            // T1: accumulated position never runs backwards.
            assert!(position >= prev_pos - EPS, "position teleported backwards");
            prev_pos = position;

            // The error magnitude never grows (stable, correctly-signed control).
            let cur_err = (network - position).abs();
            assert!(
                cur_err <= prev_err + EPS,
                "phase error grew: {prev_err} -> {cur_err}"
            );
            prev_err = cur_err;
        }
        // (c) Converged well below the starting offset.
        assert!(
            prev_err < err0 * 0.05,
            "phase error did not converge: {err0} -> {prev_err}"
        );
    }
}
