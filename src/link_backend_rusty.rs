//! # rusty_link backend — the real Ableton Link [`TempoSource`]
//!
//! This module is compiled **only** under the off-by-default `link` Cargo
//! feature (`#[cfg(feature = "link")]` on the `mod` line in `src/lib.rs`). The
//! stock Phonon build never pulls the native Ableton Link library and stays
//! free of the GPLv2+ Link core. See
//! `docs/audits/design-ableton-link-2026-07.md` §3 (license) and §7 (matrix).
//!
//! ## What it provides
//!
//! [`RustyLinkTempoSource`] wraps a `rusty_link::AblLink` session and implements
//! the source-agnostic [`TempoSource`] trait from [`crate::link_clock`], so all
//! of the cps/phase math in `link_clock` (BPM↔cps, beat↔cycle, the bounded
//! varispeed nudge, the hard-reseek decision) works over a real network clock
//! with no backend detail leaking into the adapter. Tests and the OSC fallback
//! backend implement the same trait, so the frontends are written once against
//! the trait.
//!
//! ## Thread model — where this lives (design §5, does NOT reopen C1)
//!
//! Ableton Link owns its own realtime-safe network/timer threads internally;
//! that is independent of Phonon's `UnifiedSignalGraph`. The hard rule from the
//! render-owner migration is that the graph is `Send`-**only**, never `Sync`,
//! and no Link handle is ever attached to it. This backend honours that: it
//! lives on the **control-side Link reader thread**, which samples Link at a
//! cadence, derives a [`LinkSnapshot`], and publishes it through the existing
//! single-writer `ArcSwap` / render-owner command paths. The render thread only
//! does a lock-free `.load()` of the derived snapshot — it never touches this
//! backend or Link directly (design §5).
//!
//! Because the reader runs on the control thread (not the audio thread), the
//! [`TempoSource`] reads use `capture_app_session_state`, which is the correct
//! Link primitive for an application thread. (`capture_audio_session_state` is
//! documented "thread-safe: no" and "should ONLY be called in the audio thread";
//! using it here would be wrong.) A single reusable `SessionState`, created off
//! the audio thread as Link requires, is captured into on every read so no
//! allocation happens on the read path — the audio-conscious capture/commit
//! idiom. Writes (`set_tempo`) go through the matching `commit_app_session_state`
//! path.

use std::sync::Mutex;
use std::time::Instant;

use rusty_link::{AblLink, SessionState};

use crate::link_clock::{
    beat_to_cycle, bpm_to_cps, LinkSnapshot, TempoSource, DEFAULT_BEATS_PER_CYCLE,
};

/// Default Link quantum (bar length in beats). One Link bar == one Phonon cycle,
/// so this defaults to [`DEFAULT_BEATS_PER_CYCLE`] (four beats to the bar).
pub const DEFAULT_QUANTUM: f64 = DEFAULT_BEATS_PER_CYCLE;

/// A real Ableton Link [`TempoSource`] backed by `rusty_link`.
///
/// Construct it on the control side, [`set_enabled(true)`](Self::set_enabled) to
/// join the network session, then either read individual fields through the
/// [`TempoSource`] trait or capture a whole consistent [`LinkSnapshot`] per
/// buffer with [`capture_snapshot`](Self::capture_snapshot).
pub struct RustyLinkTempoSource {
    /// The Link session. `AblLink` is `Send + Sync` and owns Link's own network
    /// threads; we never attach it to the render graph (design §5).
    link: AblLink,
    /// A single reusable `SessionState`, created off the audio thread (Link
    /// requirement) and captured into on every read so no allocation happens on
    /// the read path. Behind a `Mutex` because the trait methods take `&self`
    /// and `capture_app_session_state` needs `&mut SessionState`; the lock lives
    /// on the control thread, never on the render path.
    session_state: Mutex<SessionState>,
    /// Bar length in beats used for the Link beat query. Maps one Phonon cycle
    /// to one Link bar.
    quantum: f64,
    /// Anchor for the `std::time::Instant` ↔ Link host-clock (microseconds)
    /// bridge, sampled at construction.
    epoch_instant: Instant,
    /// Link's `clock_micros()` at `epoch_instant`.
    epoch_link_micros: i64,
}

impl RustyLinkTempoSource {
    /// Construct a Link session at `initial_bpm` with the default quantum
    /// ([`DEFAULT_QUANTUM`]). Link starts **disabled** (no network activity)
    /// until [`set_enabled(true)`](Self::set_enabled) is called.
    pub fn new(initial_bpm: f64) -> Self {
        Self::with_quantum(initial_bpm, DEFAULT_QUANTUM)
    }

    /// Construct a Link session at `initial_bpm` with an explicit `quantum` (bar
    /// length in beats). Starts disabled.
    pub fn with_quantum(initial_bpm: f64, quantum: f64) -> Self {
        let link = AblLink::new(initial_bpm);
        // The SessionState must be created off the audio thread (Link
        // requirement); construction here satisfies that and we reuse it.
        let session_state = SessionState::new();
        // Anchor the Instant<->Link-clock bridge, reading the two clocks back to
        // back so the offset between them is captured to within a few micros.
        let epoch_link_micros = link.clock_micros();
        let epoch_instant = Instant::now();
        Self {
            link,
            session_state: Mutex::new(session_state),
            quantum,
            epoch_instant,
            epoch_link_micros,
        }
    }

    /// Borrow the underlying `AblLink` (for callbacks, peer counts, etc.).
    pub fn link(&self) -> &AblLink {
        &self.link
    }

    /// Enable or disable the Link network session. Disabled means no discovery
    /// and no peers — useful for tests and offline play.
    pub fn set_enabled(&self, enabled: bool) {
        self.link.enable(enabled);
    }

    /// Whether the Link network session is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.link.is_enabled()
    }

    /// Number of peers currently connected in the Link session.
    pub fn num_peers(&self) -> u64 {
        self.link.num_peers()
    }

    /// Configured quantum (bar length in beats).
    pub fn quantum_beats(&self) -> f64 {
        self.quantum
    }

    /// Convert a `std::time::Instant` to Link's host-clock microseconds via the
    /// anchor captured at construction. `Instant` cannot express a negative
    /// `Duration`, so the sign is computed explicitly (mirrors
    /// `MockTempoSource::beat_at` in `link_clock`).
    fn instant_to_link_micros(&self, at: Instant) -> i64 {
        let delta_us: i64 = if at >= self.epoch_instant {
            at.duration_since(self.epoch_instant).as_micros() as i64
        } else {
            -(self.epoch_instant.duration_since(at).as_micros() as i64)
        };
        self.epoch_link_micros.saturating_add(delta_us)
    }

    /// Capture ONE consistent [`LinkSnapshot`] from a single session-state
    /// capture: tempo→cps, beat→target_cycle, and the transport state all read
    /// from the same instant. This is what the control-side reader thread should
    /// call once per buffer — cheaper and more consistent than the per-field
    /// [`TempoSource`] methods, which each capture independently.
    ///
    /// `at` is the wall-clock instant the snapshot targets (e.g. the buffer's
    /// presentation time); `beats_per_cycle` maps beats to cycles; `epoch` is the
    /// publisher's generation counter (see [`LinkSnapshot::epoch`]).
    pub fn capture_snapshot(&self, at: Instant, beats_per_cycle: f64, epoch: u64) -> LinkSnapshot {
        let time = self.instant_to_link_micros(at);
        let mut ss = self.session_state.lock().expect("link session_state poisoned");
        self.link.capture_app_session_state(&mut ss);
        LinkSnapshot {
            cps: bpm_to_cps(ss.tempo(), beats_per_cycle),
            target_cycle: beat_to_cycle(ss.beat_at_time(time, self.quantum), beats_per_cycle),
            epoch,
            playing: ss.is_playing(),
        }
    }

    /// Commit a new session tempo (in BPM), effective now. Uses the app-thread
    /// commit path (`commit_app_session_state`) so the change is broadcast to all
    /// peers. Realtime-safe: no — call from the control thread only.
    pub fn set_tempo(&self, bpm: f64) {
        let now_micros = self.link.clock_micros();
        let mut ss = self.session_state.lock().expect("link session_state poisoned");
        self.link.capture_app_session_state(&mut ss);
        ss.set_tempo(bpm, now_micros);
        self.link.commit_app_session_state(&ss);
    }
}

impl TempoSource for RustyLinkTempoSource {
    fn tempo_bpm(&self) -> f64 {
        let mut ss = self.session_state.lock().expect("link session_state poisoned");
        self.link.capture_app_session_state(&mut ss);
        ss.tempo()
    }

    fn beat_at(&self, at: Instant) -> f64 {
        let time = self.instant_to_link_micros(at);
        let mut ss = self.session_state.lock().expect("link session_state poisoned");
        self.link.capture_app_session_state(&mut ss);
        ss.beat_at_time(time, self.quantum)
    }

    fn quantum(&self) -> f64 {
        self.quantum
    }

    fn is_playing(&self) -> bool {
        let mut ss = self.session_state.lock().expect("link session_state poisoned");
        self.link.capture_app_session_state(&mut ss);
        ss.is_playing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_clock::{cps_to_bpm, snapshot_from_source};
    use std::time::Duration;

    const EPS: f64 = 1e-6;

    /// Smoke test (task validation): the backend constructs under `--features
    /// link` and reports its initial tempo through [`TempoSource`], with no
    /// network activity (constructed disabled).
    #[test]
    fn test_rusty_link_backend_constructs_disabled_and_reports_tempo() {
        let src = RustyLinkTempoSource::new(120.0);
        assert!(!src.is_enabled(), "must start disabled — no network until enabled");
        assert_eq!(src.num_peers(), 0);
        assert!((src.tempo_bpm() - 120.0).abs() < EPS, "tempo={}", src.tempo_bpm());
        assert!((src.quantum() - DEFAULT_QUANTUM).abs() < 1e-9);
        assert!((src.quantum_beats() - DEFAULT_QUANTUM).abs() < 1e-9);
    }

    /// The backend satisfies the `TempoSource` trait bound both statically (as a
    /// generic parameter) and as a `dyn` object.
    #[test]
    fn test_rusty_link_backend_is_tempo_source() {
        fn assert_tempo_source<T: TempoSource>(_: &T) {}
        let src = RustyLinkTempoSource::with_quantum(140.0, 3.0);
        assert_tempo_source(&src);
        let dynref: &dyn TempoSource = &src;
        assert!((dynref.tempo_bpm() - 140.0).abs() < EPS);
        assert!((dynref.quantum() - 3.0).abs() < 1e-9);
    }

    /// The Instant↔Link-clock bridge makes `beat_at` advance linearly with the
    /// queried time at the session tempo (120 BPM == 2 beats/sec). No real sleep:
    /// both instants are derived, so Link's constant-tempo timeline is exact.
    #[test]
    fn test_rusty_link_beat_advances_with_time() {
        let src = RustyLinkTempoSource::new(120.0);
        let t0 = Instant::now();
        let b0 = src.beat_at(t0);
        let t1 = t0 + Duration::from_millis(500);
        let b1 = src.beat_at(t1);
        assert!(b1 > b0, "beat must advance with time: {b0} -> {b1}");
        // 0.5 s @ 120 BPM ≈ 1 beat; generous tolerance for capture jitter.
        assert!(
            (b1 - b0 - 1.0).abs() < 0.2,
            "expected ~1 beat over 0.5s @120bpm, got {}",
            b1 - b0
        );
    }

    /// `capture_snapshot` maps the Link tempo to cps through `link_clock` and
    /// carries the epoch/transport fields.
    #[test]
    fn test_rusty_link_capture_snapshot_maps_bpm_to_cps() {
        let src = RustyLinkTempoSource::new(120.0);
        let snap = src.capture_snapshot(Instant::now(), DEFAULT_BEATS_PER_CYCLE, 7);
        // 120 BPM / 60 / 4 = 0.5 cps.
        assert!((snap.cps - 0.5).abs() < EPS, "cps={}", snap.cps);
        assert!((cps_to_bpm(snap.cps, DEFAULT_BEATS_PER_CYCLE) - 120.0).abs() < EPS);
        assert_eq!(snap.epoch, 7);
    }

    /// The generic `snapshot_from_source` in `link_clock` works over the real
    /// backend (proves no backend detail leaks into the adapter).
    #[test]
    fn test_rusty_link_generic_snapshot_from_source() {
        let src = RustyLinkTempoSource::new(174.0);
        let snap = snapshot_from_source(&src, Instant::now(), DEFAULT_BEATS_PER_CYCLE, 42);
        assert!((snap.cps - 174.0 / 60.0 / 4.0).abs() < EPS, "cps={}", snap.cps);
        assert_eq!(snap.epoch, 42);
    }

    /// The capture/commit round-trip: `set_tempo` commits a new tempo that the
    /// next capture reads back.
    #[test]
    fn test_rusty_link_set_tempo_commits() {
        let src = RustyLinkTempoSource::new(120.0);
        src.set_tempo(150.0);
        assert!(
            (src.tempo_bpm() - 150.0).abs() < EPS,
            "committed tempo not read back: {}",
            src.tempo_bpm()
        );
    }
}
