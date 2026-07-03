//! Regression tests for T1 / pt-F1 in the **modal editor** live surface.
//!
//! The modal editor's background synth thread (src/modal_editor/mod.rs) used to
//! render via `graph.process_buffer()` in wall-clock mode — the exact same pt-F1
//! onset-clustering bug that `unify-live-clock` fixed for `phonon live`
//! (src/main.rs) and `LiveSession` (src/live.rs). Because the synth thread
//! renders ahead into the ring buffer far faster than real time (ring-fill), the
//! wall clock barely advances between back-to-back renders, so
//! `buffer_start_cycle` collapses into a tiny band and the emitted audio stops
//! tracking the pattern's sample grid.
//!
//! These tests drive the modal editor's own compile + `GraphCell` path (via
//! `EditorTestHarness`) and assert:
//!   * the FIXED path (sample-advancing `LiveClock` + `process_buffer_at`) lands
//!     four kicks on the 0/0.25/0.5/0.75 grid, and
//!   * the OLD wall-clock re-anchoring path clusters / stalls the onsets.
//!
//! Onset-detector style mirrors `tests/live_clock_timing.rs`.

use phonon::modal_editor::test_harness::EditorTestHarness;

const SR: f32 = 44100.0;
/// Mono frames emitted per rendered chunk. The harness renders into a 512-sample
/// **stereo-interleaved** buffer (matching the product synth thread), so each
/// chunk advances the clock — and emits — 512/2 = 256 mono frames.
const FRAMES_PER_CHUNK: usize = 256;
/// Chunks to render ~1 second, staying just under a full cycle at 1 cps (so the
/// kick at cycle 1.0 doesn't fire): 44100/256 = 172 → 172*256 = 44032 ≈ 0.9985 s.
const CHUNKS_PER_SECOND: usize = SR as usize / FRAMES_PER_CHUNK;

/// Simple energy-rise onset detector (matches the style used in
/// `tests/live_clock_timing.rs` / `tests/test_sample_accuracy.rs`): an onset is a
/// sudden jump from near-silence.
fn detect_onsets(signal: &[f32], sample_rate: f32, threshold: f32) -> Vec<f64> {
    let mut onsets = Vec::new();
    let window = (sample_rate * 0.01) as usize; // 10 ms
    let mut prev = 0.0f32;
    for (i, chunk) in signal.chunks(window).enumerate() {
        let energy: f32 = chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32;
        if energy > threshold && energy > prev * 3.0 && prev < threshold {
            onsets.push((i * window) as f64 / sample_rate as f64);
        }
        prev = energy;
    }
    onsets
}

/// Load `code` into a headless modal editor exactly the way a live user does:
/// set the buffer content and evaluate with Ctrl+X (compile + hot-swap), then
/// enable wall-clock timing (as the editor does for every live graph).
fn load_editor(code: &str) -> EditorTestHarness {
    let mut harness = EditorTestHarness::new().expect("create headless harness");
    harness.set_content(code);
    harness.ctrl_x();
    assert!(harness.has_graph(), "graph should load after Ctrl+X");
    harness
        .enable_wall_clock_timing()
        .expect("enable wall-clock timing");
    harness
}

// ---------------------------------------------------------------------------
// pt-F1 (FIX): the migrated synth path renders on the sample grid.
// ---------------------------------------------------------------------------

#[test]
fn test_modal_editor_ringfill_onsets_are_grid_spaced() {
    // Four kicks per cycle at 1 cps → onsets at 0.00, 0.25, 0.50, 0.75 s.
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let expected = [0.0, 0.25, 0.5, 0.75];

    let harness = load_editor(code);

    // Render via the editor's synth code path (LiveClock + process_buffer_at),
    // as fast as possible (ring-fill). This is the FIXED behavior.
    let audio = harness
        .process_audio_chunks_capture(CHUNKS_PER_SECOND)
        .expect("render ring-fill audio");

    let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.005, "render produced (near) silence, rms={rms}");

    let onsets = detect_onsets(&audio, SR, 0.001);
    assert_eq!(
        onsets.len(),
        4,
        "expected 4 grid-spaced onsets, got {}: {:?}",
        onsets.len(),
        onsets
    );
    for (o, e) in onsets.iter().zip(expected.iter()) {
        assert!(
            (o - e).abs() < 0.02,
            "onset {o:.4}s should be near grid position {e:.4}s (±20ms)"
        );
    }
}

// ---------------------------------------------------------------------------
// pt-F1 (BUG): the old wall-clock re-anchoring path clusters / stalls onsets.
// This is the behavior on `main` (mod.rs synth loop used process_buffer).
// ---------------------------------------------------------------------------

#[test]
fn test_modal_editor_wall_clock_reanchor_regresses_onset_grid() {
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let expected = [0.0, 0.25, 0.5, 0.75];

    let harness = load_editor(code);

    // Render the way the OLD synth loop did: process_buffer in wall-clock mode
    // with the wall clock frozen between renders (deterministic ring-fill worst
    // case — the synth thread renders far ahead of real time).
    let audio = harness
        .render_ring_fill_wall_clock_frozen(CHUNKS_PER_SECOND)
        .expect("render frozen-wall-clock audio");

    let onsets = detect_onsets(&audio, SR, 0.001);

    // The bug: the buffer-start cycle never advances, so the pattern stalls at
    // cycle 0 — the four grid onsets collapse instead of spanning the second.
    let reaches_late_grid = onsets.iter().any(|&o| o > 0.4);
    assert!(
        !reaches_late_grid,
        "wall-clock re-anchoring unexpectedly tracked the pattern grid ({onsets:?}) — \
         the pt-F1 reproduction is no longer exercising the bug"
    );
    assert!(
        onsets.len() < 4,
        "buggy path should drop onsets (pattern stalls), but produced {onsets:?}"
    );
    for e in expected.iter().skip(1) {
        assert!(
            !onsets.iter().any(|&o| (o - e).abs() < 0.02),
            "buggy path unexpectedly hit grid position {e}s"
        );
    }
}

// ---------------------------------------------------------------------------
// Reload continuity: a C-x graph swap must continue from the live clock, with no
// re-trigger burst (the synth loop seeds the swapped-in graph from the clock).
// ---------------------------------------------------------------------------

#[test]
fn test_modal_editor_reload_continuity_no_retrigger_burst() {
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let expected = [0.0, 0.25, 0.5, 0.75];

    let mut harness = load_editor(code);

    // Render ~0.6 s on graph A, then C-x-swap to a freshly compiled graph B and
    // continue for the rest of the second — the editor's real swap path.
    let chunks_a = (0.6 * SR as f64 / FRAMES_PER_CHUNK as f64) as usize;
    let chunks_total = CHUNKS_PER_SECOND;

    let mut audio = harness
        .render_live_chunks(chunks_a)
        .expect("render graph A");

    // Re-evaluate the same code: compile B, transfer session timing, hot-swap.
    // The persistent live clock (held in the harness) seeds B from its current
    // position so the beat continues instead of restarting at cycle 0.
    harness.ctrl_x();
    assert!(harness.has_graph(), "graph B should load after re-eval");

    let audio_b = harness
        .render_live_chunks(chunks_total - chunks_a)
        .expect("render graph B");
    audio.extend_from_slice(&audio_b);

    let onsets = detect_onsets(&audio, SR, 0.001);
    assert_eq!(
        onsets.len(),
        4,
        "reload should keep 4 continuous onsets (no re-trigger burst), got {onsets:?}"
    );
    for (o, e) in onsets.iter().zip(expected.iter()) {
        assert!(
            (o - e).abs() < 0.02,
            "onset {o:.4}s should stay on grid {e:.4}s across the A→B swap"
        );
    }
}
