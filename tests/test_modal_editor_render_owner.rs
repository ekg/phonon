//! Regression tests for the **modal editor** render-owner migration
//! (`migrate-modal-editor-render-owner`).
//!
//! The modal editor no longer shares its graph across threads through
//! `ArcSwap<Option<GraphCell(RefCell<UnifiedSignalGraph>)>>` + `unsafe impl
//! Sync`. The background synth thread is the SOLE owner of the graph; the
//! control thread hands compiled graphs and hush / panic over the render-owner
//! command channel (`src/render_swap.rs`), and the swap's state transfer runs on
//! the render thread at a buffer boundary (design §4).
//!
//! These tests drive the editor through its **real keymap** (Ctrl+X to evaluate,
//! Ctrl+H to hush) via `EditorTestHarness`, so they exercise the migrated
//! load_code → command-channel → render-owner path end to end — not a CLI/unit
//! substitute — and assert the behavior is preserved:
//!   * a compiled patch produces audio,
//!   * Ctrl+H (hush, routed as `Cmd::Hush`) silences it, and
//!   * re-evaluating (Ctrl+X, routed as a swap) resumes audio.

use crossterm::event::KeyCode;
use phonon::modal_editor::test_harness::EditorTestHarness;

const SR: f32 = 44100.0;
/// ~0.5 s of 512-sample stereo chunks (44100 / 256 mono frames per chunk).
const CHUNKS: usize = (SR as usize / 256) / 2;

fn rms(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    (signal.iter().map(|x| x * x).sum::<f32>() / signal.len() as f32).sqrt()
}

/// Evaluate the current buffer with Ctrl+X (the real keystroke), the way a live
/// user hot-swaps a patch — this routes through `load_code` → the render-owner
/// swap channel.
fn eval(harness: &mut EditorTestHarness, code: &str) {
    harness.set_content(code);
    harness.ctrl_x();
    assert!(harness.has_graph(), "graph should be loaded after Ctrl+X");
}

#[test]
fn test_render_owner_first_load_produces_audio() {
    let mut harness = EditorTestHarness::new().expect("headless harness");
    eval(&mut harness, "tempo: 1.0\nout $ s \"bd bd bd bd\"");

    let audio = harness
        .process_audio_chunks_capture(CHUNKS)
        .expect("render audio");
    assert!(
        rms(&audio) > 0.005,
        "first load through the render-owner channel should produce audio (rms={})",
        rms(&audio)
    );
}

#[test]
fn test_render_owner_hush_via_channel_silences_then_reload_resumes() {
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let mut harness = EditorTestHarness::new().expect("headless harness");
    eval(&mut harness, code);

    // Sounding before hush.
    let before = harness
        .process_audio_chunks_capture(CHUNKS)
        .expect("render before hush");
    assert!(
        rms(&before) > 0.005,
        "patch should be sounding before hush (rms={})",
        rms(&before)
    );

    // Ctrl+H → hush, routed through the render-owner channel as `Cmd::Hush`
    // (no `graph.store(None)`, no cross-thread borrow). The render owner applies
    // `hush_all` to its owned graph at the next boundary.
    harness.send_key_with_modifiers(KeyCode::Char('h'), crossterm::event::KeyModifiers::CONTROL);

    let hushed = harness
        .process_audio_chunks_capture(CHUNKS)
        .expect("render after hush");
    assert!(
        rms(&hushed) < 1e-4,
        "hush routed through the render-owner channel should silence output (rms={})",
        rms(&hushed)
    );

    // Re-evaluate (Ctrl+X) → a fresh graph swaps in over the channel; the fresh
    // graph has no hushed channels, so audio resumes (matching "C-r to reload").
    eval(&mut harness, code);
    let resumed = harness
        .process_audio_chunks_capture(CHUNKS)
        .expect("render after reload");
    assert!(
        rms(&resumed) > 0.005,
        "re-evaluating after hush should resume audio (rms={})",
        rms(&resumed)
    );
}
