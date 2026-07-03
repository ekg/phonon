//! GlobalClock lock hardening tests (rt-safety audit F-3 follow-up).
//!
//! Follow-up from `render_lock_hardening.rs` / harden-render-locks, which owned
//! `src/unified_graph.rs` and explicitly left the GlobalClock mutex in
//! `src/bin/phonon-audio.rs` out of scope.
//!
//! The synth render thread reads timing from the GlobalClock ONCE per buffer.
//! Doing so behind `.lock().unwrap()` is unsafe on the audio path:
//!   * a poisoned mutex makes `.unwrap()` panic → the synth thread dies → the
//!     ring buffer drains → permanent silence, and
//!   * a plain `Mutex` is a priority-inversion point between the render thread
//!     and the (lower-priority) IPC thread that updates tempo.
//!
//! The fix publishes the clock as a lock-free double-buffered snapshot
//! (`ArcSwap<GlobalClock>`): the render thread reads it with `.load()` (no lock,
//! no allocation, always a consistent snapshot) and the IPC thread swaps in a
//! new snapshot on tempo change. These invariants are enforced structurally.

use std::fs;

fn phonon_audio_src() -> String {
    // Integration tests run with CWD = crate root.
    fs::read_to_string("src/bin/phonon-audio.rs").expect("read src/bin/phonon-audio.rs")
}

/// F-3: the GlobalClock must not sit behind a `Mutex` that the render thread has
/// to lock every buffer. It is published as a lock-free `ArcSwap` snapshot.
#[test]
fn test_globalclock_not_behind_mutex() {
    let src = phonon_audio_src();
    assert!(
        !src.contains("Mutex::new(GlobalClock"),
        "GlobalClock must not be wrapped in a Mutex on the render path; \
         publish a lock-free ArcSwap snapshot instead"
    );
    assert!(
        src.contains("ArcSwap::from_pointee(GlobalClock"),
        "GlobalClock should be published via a lock-free ArcSwap double-buffered snapshot"
    );
}

/// F-3: no `.lock().unwrap()` may remain on the clock render/update path. A panic
/// anywhere poisons the mutex; `.unwrap()` on a poisoned lock then panics the
/// render thread, draining the ring and stopping audio permanently.
#[test]
fn test_no_clock_lock_unwrap_on_render_path() {
    let src = phonon_audio_src();
    // The synth render thread reads the clock via `clock_clone_synth`; the IPC
    // tempo-update paths use `global_clock`. Neither may take a poisonable lock.
    assert!(
        !src.contains("clock_clone_synth.lock()"),
        "render thread must read the clock lock-free (.load()), not .lock()"
    );
    assert!(
        !src.contains("global_clock.lock()"),
        "tempo-update path must not .lock() the clock (use a lock-free snapshot swap)"
    );
    // Positively assert the render thread reads a lock-free snapshot.
    assert!(
        src.contains("clock_clone_synth.load()"),
        "render thread should read the clock via a lock-free ArcSwap .load()"
    );
}

/// harden-render-locks added `UnifiedSignalGraph::preload_plugins()` plus a
/// one-time `ensure_prepared()` render-init fallback. The ideal is to call
/// `preload_plugins()` from the compile/reload path so the FIRST rendered buffer
/// never blocks on plugin disk load. Enforce that the reload path wires it.
#[test]
fn test_reload_preloads_plugins_off_render_thread() {
    let src = phonon_audio_src();
    assert!(
        src.contains("preload_plugins()"),
        "compile/reload path must call preload_plugins() so plugins load fully \
         off the render thread (never block the first rendered buffer)"
    );
}
