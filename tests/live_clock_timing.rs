//! Regression tests for T1 / pt-F1 + pt-F2: the live clock must advance by
//! **samples emitted** and rebase tempo without teleporting.
//!
//! Two distinct bugs are covered:
//!
//! * **pt-F1 (onset clustering):** the old live path recomputed the buffer-start
//!   cycle from wall-clock time on every buffer. Because the synth thread renders
//!   ahead into a ring buffer (filling ~1 s in a few ms at startup / after an
//!   underrun), the wall clock barely advances between those back-to-back renders,
//!   so `buffer_start_cycle` collapses into a tiny overlapping band and the emitted
//!   audio no longer tracks the pattern's sample grid. The fix routes live
//!   rendering through `process_buffer_at` fed by a sample-advancing [`LiveClock`].
//!
//! * **pt-F2 (set_cps teleport):** `set_cps` changed tempo with no offset
//!   compensation, so in wall-clock mode the cycle position teleported by
//!   `elapsed * Δcps`. The fix rebases (capture position, reset origin, then change
//!   cps) exactly like `GlobalClock::set_cps`.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::{LiveClock, OutputMixMode, UnifiedSignalGraph};
use std::time::{Duration, Instant};

const SR: f32 = 44100.0;
const CHUNK: usize = 512;

fn compile(code: &str) -> UnifiedSignalGraph {
    let (_, statements) = parse_program(code).expect("parse");
    let mut graph = compile_program(statements, SR, None).expect("compile");
    graph.set_output_mix_mode(OutputMixMode::None);
    graph.preload_samples();
    graph
}

/// Simple energy-rise onset detector (matches the style used in
/// `tests/test_sample_accuracy.rs`): an onset is a sudden jump from near-silence.
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

/// Render `total_frames` of a graph the way the FIXED product synth loop does:
/// a sample-advancing `LiveClock` feeds `process_buffer_at`, and the clock is
/// advanced by exactly the number of frames emitted each chunk. No wall-clock is
/// consulted on the render path — so rendering "infinitely fast" (as the synth
/// thread does while filling the ring) still tiles the pattern on the sample grid.
fn render_via_live_clock(graph: &mut UnifiedSignalGraph, total_frames: usize) -> Vec<f32> {
    graph.enable_wall_clock_timing();
    let mut clock = LiveClock::new(SR, graph.get_cps(), graph.get_cycle_position());
    let mut buf = vec![0.0f32; CHUNK * 2];
    let mut out = Vec::with_capacity(total_frames);
    let chunks = total_frames / CHUNK;
    for _ in 0..chunks {
        buf.iter_mut().for_each(|s| *s = 0.0);
        let (start, incr, cps) = clock.advance_buffer(CHUNK);
        graph.process_buffer_at(&mut buf, start, incr, cps);
        for i in 0..CHUNK {
            out.push(buf[i * 2]);
        }
    }
    out
}

/// Render `total_frames` the way the OLD live path did: `process_buffer` in
/// wall-clock mode, with the wall clock (near-)frozen between renders. This is the
/// deterministic worst case of "synth renders far ahead of real time": every
/// chunk re-anchors `buffer_start_cycle` to the same tiny band, so the emitted
/// audio stops tracking the pattern grid (pt-F1). Used to *demonstrate* the bug.
fn render_via_wall_clock_frozen(graph: &mut UnifiedSignalGraph, total_frames: usize) -> Vec<f32> {
    graph.enable_wall_clock_timing();
    let mut buf = vec![0.0f32; CHUNK * 2];
    let mut out = Vec::with_capacity(total_frames);
    let chunks = total_frames / CHUNK;
    for _ in 0..chunks {
        buf.iter_mut().for_each(|s| *s = 0.0);
        // Freeze the wall clock: reset the origin so `elapsed ≈ 0` every render.
        graph.session_start_time = Instant::now();
        graph.process_buffer(&mut buf);
        for i in 0..CHUNK {
            out.push(buf[i * 2]);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// pt-F1: LiveClock advances by samples emitted (grid-aligned), never wall-clock.
// ---------------------------------------------------------------------------

#[test]
fn test_live_clock_advances_on_sample_grid() {
    let cps = 1.7f32;
    let mut clock = LiveClock::new(SR, cps, 0.0);
    let per_buffer = CHUNK as f64 * cps as f64 / SR as f64;

    let mut prev_start: Option<f64> = None;
    for i in 0..200 {
        let (start, incr, got_cps) = clock.advance_buffer(CHUNK);
        assert_eq!(got_cps, cps);
        assert!((incr - cps as f64 / SR as f64).abs() < 1e-12);
        // Each buffer must begin exactly one buffer of cycle-time after the last —
        // no overlap, no gap, regardless of how fast we call it.
        let expected = i as f64 * per_buffer;
        assert!(
            (start - expected).abs() < 1e-9,
            "buffer {i}: start {start} != grid {expected}"
        );
        if let Some(p) = prev_start {
            assert!(
                (start - p - per_buffer).abs() < 1e-9,
                "consecutive buffer starts must differ by exactly one buffer of cycle-time"
            );
        }
        prev_start = Some(start);
    }
}

/// Rendering a running clock as fast as possible (ring-fill) must not consult the
/// wall clock: two clocks advanced the same number of samples but with wildly
/// different real time between calls land on the *same* position.
#[test]
fn test_live_clock_independent_of_wall_clock() {
    let cps = 2.0f32;
    let mut fast = LiveClock::new(SR, cps, 0.0);
    let mut slow = LiveClock::new(SR, cps, 0.0);
    for _ in 0..50 {
        fast.advance_buffer(CHUNK);
    }
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(1));
        slow.advance_buffer(CHUNK);
    }
    assert!(
        (fast.position() - slow.position()).abs() < 1e-9,
        "clock position must depend only on samples emitted, not wall time: {} vs {}",
        fast.position(),
        slow.position()
    );
}

// ---------------------------------------------------------------------------
// pt-F1: onset timing — sample-advancing live rendering is grid-spaced; the old
// wall-clock re-anchoring path clusters / drops onsets.
// ---------------------------------------------------------------------------

#[test]
fn test_live_ringfill_onsets_are_grid_spaced() {
    // Four kicks per cycle at 1 cps → onsets at 0.00, 0.25, 0.50, 0.75 s.
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let frames = SR as usize; // 1 second
    let expected = [0.0, 0.25, 0.5, 0.75];

    // FIXED path: sample-advancing LiveClock + process_buffer_at.
    let mut g = compile(code);
    let audio = render_via_live_clock(&mut g, frames);
    let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.005, "live-clock render produced (near) silence, rms={rms}");

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

#[test]
fn test_wall_clock_reanchor_regresses_onset_grid() {
    // Documents the pt-F1 bug: with wall-clock re-anchoring under ring-fill, the
    // emitted onsets do NOT land on the 0/0.25/0.5/0.75 grid the fixed path yields.
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let frames = SR as usize;
    let expected = [0.0, 0.25, 0.5, 0.75];

    let mut g = compile(code);
    let audio = render_via_wall_clock_frozen(&mut g, frames);
    let onsets = detect_onsets(&audio, SR, 0.001);

    // The bug: with the wall clock (near-)frozen while the synth renders ahead, the
    // buffer-start cycle never advances, so the pattern stalls at cycle 0 — the four
    // grid onsets collapse to a single onset at the start instead of spanning the
    // second. This is exactly the pt-F1 startup clustering the fix removes.
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
    // Compared with the fixed path, which yields all four spread across the second:
    for e in expected.iter().skip(1) {
        assert!(
            !onsets.iter().any(|&o| (o - e).abs() < 0.02),
            "buggy path unexpectedly hit grid position {e}s"
        );
    }
}

/// Reload continuity: swapping to a new graph mid-stream must continue from the
/// live clock position (the synth loop seeds the swapped-in graph from the clock),
/// not restart the pattern. Mirrors the product swap path: compile B, run
/// `transfer_session_timing` (which seeds B off the OLD graph's wall clock — here
/// ~0 because rendering is faster than real time), then seed B from the clock.
#[test]
fn test_reload_continuity_from_clock() {
    let code = "tempo: 1.0\nout $ s \"bd bd bd bd\"";
    let chunks_a = (0.6 * SR as f64 / CHUNK as f64) as usize; // ~0.6 s on graph A

    let mut a = compile(code);
    a.enable_wall_clock_timing();
    let mut clock = LiveClock::new(SR, a.get_cps(), a.get_cycle_position());

    let mut out = Vec::new();
    let mut buf = vec![0.0f32; CHUNK * 2];
    let mut render = |g: &mut UnifiedSignalGraph, clock: &mut LiveClock, out: &mut Vec<f32>| {
        buf.iter_mut().for_each(|s| *s = 0.0);
        let (start, incr, cps) = clock.advance_buffer(CHUNK);
        g.process_buffer_at(&mut buf, start, incr, cps);
        for i in 0..CHUNK {
            out.push(buf[i * 2]);
        }
    };

    for _ in 0..chunks_a {
        render(&mut a, &mut clock, &mut out);
    }

    // --- Swap to graph B, exactly like the product synth loop on reload. ---
    let mut b = compile(code);
    b.enable_wall_clock_timing();
    b.transfer_session_timing(&a); // seeds B off A's (near-zero) wall clock
    clock.set_cps(b.get_cps()); // rebase tempo (unchanged here)
    b.set_cycle_position(clock.position()); // seed B from the live clock — the fix

    let total_chunks = (SR as f64 / CHUNK as f64) as usize; // 1 s total
    for _ in chunks_a..total_chunks {
        render(&mut b, &mut clock, &mut out);
    }

    let onsets = detect_onsets(&out, SR, 0.001);
    // Continuous: the four kicks land on the grid across the A→B boundary with no
    // restart-at-zero burst from B.
    let expected = [0.0, 0.25, 0.5, 0.75];
    assert_eq!(
        onsets.len(),
        4,
        "reload should keep 4 continuous onsets, got {onsets:?}"
    );
    for (o, e) in onsets.iter().zip(expected.iter()) {
        assert!(
            (o - e).abs() < 0.02,
            "onset {o:.4}s should stay on grid {e:.4}s across reload"
        );
    }
}

// ---------------------------------------------------------------------------
// pt-F2: set_cps must not teleport the cycle position.
// ---------------------------------------------------------------------------

/// Compute the graph's live cycle position from its public timing fields, using the
/// exact wall-clock formula the render path uses. Independent of any new method, so
/// this same helper demonstrates the teleport on `main`.
fn wall_clock_position(g: &UnifiedSignalGraph) -> f64 {
    g.session_start_time.elapsed().as_secs_f64() * g.cps as f64 + g.cycle_offset
}

#[test]
fn test_set_cps_does_not_teleport_graph_position() {
    let mut g = compile("tempo: 2.0\nout $ s \"bd\"");
    g.enable_wall_clock_timing();
    // Simulate being an hour into the session at 2.0 cps.
    g.session_start_time = Instant::now() - Duration::from_secs(3600);
    g.cycle_offset = 0.0;

    let before = wall_clock_position(&g);
    assert!(before > 7000.0, "sanity: ~3600*2.0 cycles in, got {before}");

    // Change tempo 2.0 -> 2.5. On `main` (no rebase) this teleports the position by
    // elapsed*Δcps ≈ 3600*0.5 = 1800 cycles.
    g.set_cps(2.5);
    let after = wall_clock_position(&g);

    assert!(
        (after - before).abs() < 0.1,
        "set_cps must preserve cycle position (pt-F2): {before:.3} -> {after:.3} \
         (teleport of {:.1} cycles)",
        after - before
    );
    // And the new tempo must actually be in effect going forward.
    assert!((g.get_cps() - 2.5).abs() < 1e-6);
}

#[test]
fn test_live_clock_set_cps_is_continuous() {
    let mut clock = LiveClock::new(SR, 2.0, 0.0);
    // Advance ~1000 buffers so the position is far from zero.
    for _ in 0..1000 {
        clock.advance_buffer(CHUNK);
    }
    let before = clock.position();
    clock.set_cps(2.5);
    let after = clock.position();
    assert!(
        (after - before).abs() < 1e-9,
        "LiveClock::set_cps must preserve position: {before} -> {after}"
    );
    assert!((clock.cps() - 2.5).abs() < 1e-6);
    // Increment must now reflect the new tempo.
    assert!((clock.sample_increment() - 2.5 / SR as f64).abs() < 1e-12);
}

/// Offline (`set_cps` before rendering, sample-based mode) must be unaffected by
/// the rebase: this is the pattern hundreds of existing tests use.
#[test]
fn test_set_cps_offline_still_sets_tempo() {
    let mut g = compile("out $ s \"bd\"");
    // Fresh graph is in sample-based mode (use_wall_clock == false).
    g.set_cps(2.0);
    assert!((g.get_cps() - 2.0).abs() < 1e-6);
    assert!(
        g.get_cycle_position().abs() < 1e-9,
        "offline set_cps must not move the position from 0"
    );
}
