//! Audio live-edit glitch harness
//!
//! Reproducible harness for the reported issue: audio sometimes becomes corrupted
//! or degraded after editing and relaunching/reloading phonon code while the
//! process stays alive.
//!
//! # What this tests
//!
//! Exercises the same engine state transitions as the modal editor's `load_code()` path:
//!   parse → compile → enable_wall_clock → state_transfer → preload_samples → graph_swap → render
//!
//! 30 deterministic reload cycles are performed across five scenario categories:
//!   - Oscillator frequency jumps
//!   - Tempo (CPS) changes
//!   - Effect chain add/remove
//!   - New buses added
//!   - Minimal constant program
//!
//! # Metrics collected per cycle
//!
//! - NaN / Inf sample count
//! - Clipping  (|sample| > 1.0)
//! - Silence   (RMS < 0.001 when signal expected)
//! - DC offset (|mean| > 0.1)
//! - RMS jump  (ratio between last pre-swap and first post-swap buffer)
//! - Max discontinuity (largest sample-to-sample delta at transition boundary)
//! - Stuck output (post-swap buffer identical to pre-swap tail)
//!
//! # Exit behaviour
//!
//! The test fails (panics) if any of the following are found across all cycles:
//!   - Any NaN or Inf sample in any rendered buffer
//!   - Severe clipping (>5% of samples in a buffer above 1.0)
//!   - Unexpected silence in a buffer where sound is expected
//!   - Stuck output (new graph producing bit-identical output to old graph's tail)
//!
//! Run:
//!   cargo test audio_live_edit_glitch_harness -- --nocapture

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Sample rate used throughout (matches modal editor default).
const SAMPLE_RATE: f32 = 44100.0;

/// Buffer length in f32 elements.  The DAG renderer treats this as stereo-
/// interleaved, so `BUFFER_LEN / 2` = 512 frames ≈ 11.6 ms per buffer.
const BUFFER_LEN: usize = 1024;

/// Buffers rendered before each reload (captures pre-transition baseline).
const PRE_BUFFERS: usize = 8;

/// Buffers rendered after each reload (captures post-transition audio).
const POST_BUFFERS: usize = 8;

/// Total reload cycles exercised.
const NUM_CYCLES: usize = 30;

/// Fraction of samples in a single buffer that may clip before we flag it.
const CLIP_FRACTION_THRESHOLD: f32 = 0.05;

/// Minimum expected RMS for a buffer that should contain sound.
const SILENCE_RMS_THRESHOLD: f32 = 0.001;

/// Maximum absolute DC offset allowed.
const DC_OFFSET_THRESHOLD: f32 = 0.1;

/// Maximum sample-to-sample delta allowed at the transition boundary.
/// 0.5 corresponds to a large click / pop.
const DISCONTINUITY_THRESHOLD: f32 = 0.5;

// ---------------------------------------------------------------------------
// Scenario pairs: (description, code_before, code_after)
// ---------------------------------------------------------------------------

fn scenarios() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // --- 1. Oscillator frequency jump ---
        (
            "osc-freq-jump-110-220",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 1.0\nout $ sine 220 * 0.3",
        ),
        (
            "osc-freq-jump-220-440",
            "tempo: 1.0\nout $ sine 220 * 0.3",
            "tempo: 1.0\nout $ sine 440 * 0.3",
        ),
        (
            "osc-freq-jump-saw-110-330",
            "tempo: 1.0\nout $ saw 110 * 0.2",
            "tempo: 1.0\nout $ saw 330 * 0.2",
        ),
        (
            "osc-waveform-sine-to-saw",
            "tempo: 1.0\nout $ sine 220 * 0.3",
            "tempo: 1.0\nout $ saw 220 * 0.2",
        ),
        (
            "osc-waveform-saw-to-sine",
            "tempo: 1.0\nout $ saw 220 * 0.2",
            "tempo: 1.0\nout $ sine 220 * 0.3",
        ),
        // --- 2. Tempo (CPS) changes ---
        (
            "tempo-1.0-to-2.0",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 2.0\nout $ sine 110 * 0.3",
        ),
        (
            "tempo-2.0-to-0.5",
            "tempo: 2.0\nout $ sine 110 * 0.3",
            "tempo: 0.5\nout $ sine 110 * 0.3",
        ),
        (
            "tempo-0.5-to-1.0",
            "tempo: 0.5\nout $ sine 110 * 0.3",
            "tempo: 1.0\nout $ sine 110 * 0.3",
        ),
        (
            "tempo-1.0-to-3.0",
            "tempo: 1.0\nout $ sine 220 * 0.3",
            "tempo: 3.0\nout $ sine 220 * 0.3",
        ),
        (
            "tempo-3.0-to-1.0",
            "tempo: 3.0\nout $ sine 220 * 0.3",
            "tempo: 1.0\nout $ sine 220 * 0.3",
        ),
        // --- 3. Effect chain add/remove ---
        (
            "add-lpf",
            "tempo: 1.0\nout $ saw 110 * 0.2",
            "tempo: 1.0\nout $ saw 110 # lpf 1000 0.8 * 0.2",
        ),
        (
            "remove-lpf",
            "tempo: 1.0\nout $ saw 110 # lpf 1000 0.8 * 0.2",
            "tempo: 1.0\nout $ saw 110 * 0.2",
        ),
        (
            "add-hpf",
            "tempo: 1.0\nout $ saw 330 * 0.2",
            "tempo: 1.0\nout $ saw 330 # hpf 500 0.8 * 0.2",
        ),
        (
            "remove-hpf",
            "tempo: 1.0\nout $ saw 330 # hpf 500 0.8 * 0.2",
            "tempo: 1.0\nout $ saw 330 * 0.2",
        ),
        (
            "change-lpf-cutoff",
            "tempo: 1.0\nout $ saw 110 # lpf 500 0.8 * 0.2",
            "tempo: 1.0\nout $ saw 110 # lpf 2000 0.8 * 0.2",
        ),
        // --- 4. New buses ---
        (
            "add-bus",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 1.0\n~osc $ sine 110\nout $ ~osc * 0.3",
        ),
        (
            "remove-bus",
            "tempo: 1.0\n~osc $ sine 110\nout $ ~osc * 0.3",
            "tempo: 1.0\nout $ sine 110 * 0.3",
        ),
        (
            "add-second-bus",
            "tempo: 1.0\n~a $ sine 110\nout $ ~a * 0.3",
            "tempo: 1.0\n~a $ sine 110\n~b $ sine 220\nout $ ~a * 0.2 + ~b * 0.1",
        ),
        (
            "remove-second-bus",
            "tempo: 1.0\n~a $ sine 110\n~b $ sine 220\nout $ ~a * 0.2 + ~b * 0.1",
            "tempo: 1.0\n~a $ sine 110\nout $ ~a * 0.3",
        ),
        (
            "rename-bus",
            "tempo: 1.0\n~old $ sine 110\nout $ ~old * 0.3",
            "tempo: 1.0\n~new $ sine 110\nout $ ~new * 0.3",
        ),
        // --- 5. Minimal / constant programs ---
        (
            "constant-to-osc",
            "tempo: 1.0\nout $ 0.0",
            "tempo: 1.0\nout $ sine 110 * 0.3",
        ),
        (
            "osc-to-constant-silence",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 1.0\nout $ 0.0",
        ),
        (
            "gain-halve",
            "tempo: 1.0\nout $ sine 220 * 0.4",
            "tempo: 1.0\nout $ sine 220 * 0.2",
        ),
        (
            "gain-double",
            "tempo: 1.0\nout $ sine 220 * 0.2",
            "tempo: 1.0\nout $ sine 220 * 0.4",
        ),
        (
            "gain-to-edge",
            "tempo: 1.0\nout $ sine 220 * 0.2",
            "tempo: 1.0\nout $ sine 220 * 0.9",
        ),
        // --- Extra cycles to reach >= 30 ---
        (
            "multi-osc-merge",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 1.0\nout $ sine 110 * 0.15 + sine 220 * 0.15",
        ),
        (
            "multi-osc-split",
            "tempo: 1.0\nout $ sine 110 * 0.15 + sine 220 * 0.15",
            "tempo: 1.0\nout $ sine 110 * 0.3",
        ),
        (
            "lpf-sweep-cutoff",
            "tempo: 1.0\nout $ saw 110 # lpf 500 0.5 * 0.2",
            "tempo: 1.0\nout $ saw 110 # lpf 4000 0.5 * 0.2",
        ),
        (
            "lpf-resonance-change",
            "tempo: 1.0\nout $ saw 110 # lpf 1000 0.1 * 0.2",
            "tempo: 1.0\nout $ saw 110 # lpf 1000 0.9 * 0.2",
        ),
        (
            "tempo-and-osc-simultaneous",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 2.0\nout $ sine 220 * 0.3",
        ),
    ]
}

// ---------------------------------------------------------------------------
// Audio analysis helpers
// ---------------------------------------------------------------------------

/// Per-buffer health metrics.
#[derive(Debug, Clone)]
struct BufferMetrics {
    rms: f32,
    nan_count: usize,
    inf_count: usize,
    clip_count: usize,
    dc_offset: f32,
    /// Maximum |sample[i] - sample[i-1]| within this buffer.
    max_internal_discontinuity: f32,
}

impl BufferMetrics {
    fn is_silent(&self, expect_sound: bool) -> bool {
        expect_sound && self.rms < SILENCE_RMS_THRESHOLD
    }

    fn has_nan_or_inf(&self) -> bool {
        self.nan_count > 0 || self.inf_count > 0
    }

    fn has_severe_clip(&self, buf_len: usize) -> bool {
        self.clip_count > (buf_len as f32 * CLIP_FRACTION_THRESHOLD) as usize
    }
}

fn analyze_buffer(buf: &[f32]) -> BufferMetrics {
    let mut sum_sq = 0.0f32;
    let mut nan_count = 0usize;
    let mut inf_count = 0usize;
    let mut clip_count = 0usize;
    let mut sum = 0.0f32;
    let mut max_disc = 0.0f32;
    let mut prev = 0.0f32;

    for (i, &s) in buf.iter().enumerate() {
        if s.is_nan() {
            nan_count += 1;
        } else if s.is_infinite() {
            inf_count += 1;
        } else {
            sum_sq += s * s;
            sum += s;
            if s.abs() > 1.0 {
                clip_count += 1;
            }
            if i > 0 {
                let disc = (s - prev).abs();
                if disc > max_disc {
                    max_disc = disc;
                }
            }
            prev = s;
        }
    }

    let valid = buf.len() - nan_count - inf_count;
    let rms = if valid > 0 {
        (sum_sq / valid as f32).sqrt()
    } else {
        0.0
    };
    let dc_offset = if valid > 0 { sum / valid as f32 } else { 0.0 };

    BufferMetrics {
        rms,
        nan_count,
        inf_count,
        clip_count,
        dc_offset,
        max_internal_discontinuity: max_disc,
    }
}

/// Sample-to-sample delta between the last sample of `tail` and first sample of `head`.
fn boundary_discontinuity(tail: &[f32], head: &[f32]) -> f32 {
    match (tail.last(), head.first()) {
        (Some(&a), Some(&b)) if a.is_finite() && b.is_finite() => (b - a).abs(),
        _ => 0.0,
    }
}

/// Returns true if two slices are bit-identical (stuck output).
fn is_stuck(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
}

// ---------------------------------------------------------------------------
// Reload simulation (mirrors load_code() in modal_editor/mod.rs)
// ---------------------------------------------------------------------------

fn compile_graph(code: &str) -> phonon::unified_graph::UnifiedSignalGraph {
    let (rest, statements) = parse_program(code).expect("parse_program failed");
    assert!(
        rest.trim().is_empty(),
        "Parser left unconsumed input: {:?}",
        rest
    );
    compile_program(statements, SAMPLE_RATE, None).expect("compile_program failed")
}

/// Perform a full live-reload transition from `old_graph` to a new graph compiled
/// from `new_code`.  Returns the new graph and the elapsed reload time.
///
/// Mirrors the sequence in `ModalEditor::load_code()`:
///   1. parse + compile
///   2. enable_wall_clock_timing
///   3. transfer_session_timing
///   4. transfer_fx_states
///   5. transfer_voice_manager
///   6. preload_samples
fn live_reload(
    old_graph: &mut phonon::unified_graph::UnifiedSignalGraph,
    new_code: &str,
) -> (phonon::unified_graph::UnifiedSignalGraph, u64) {
    let t0 = Instant::now();

    let mut new_graph = compile_graph(new_code);

    // Step 2 – same as load_code: enable wall-clock timing first
    new_graph.enable_wall_clock_timing();

    // Step 3-5 – state transfer (old graph still exclusively owned by render "thread")
    new_graph.transfer_session_timing(old_graph);
    new_graph.transfer_fx_states(old_graph);
    new_graph.transfer_voice_manager(old_graph.take_voice_manager());

    // Step 6 – preload all statically-known samples before swapping
    new_graph.preload_samples();

    let elapsed_us = t0.elapsed().as_micros() as u64;
    (new_graph, elapsed_us)
}

// ---------------------------------------------------------------------------
// Per-cycle result
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CycleResult {
    scenario: &'static str,
    reload_time_us: u64,

    // Pre-transition aggregate
    pre_rms_avg: f32,
    pre_nan: usize,
    pre_inf: usize,
    pre_clip_severe: bool,

    // Transition boundary
    boundary_disc: f32,

    // Post-transition aggregate
    post_rms_avg: f32,
    post_nan: usize,
    post_inf: usize,
    post_clip_severe: bool,
    post_silent: bool,
    post_stuck: bool,

    // Derived
    rms_jump_ratio: f32,
}

impl CycleResult {
    fn has_any_nan_or_inf(&self) -> bool {
        self.pre_nan > 0 || self.pre_inf > 0 || self.post_nan > 0 || self.post_inf > 0
    }

    fn has_severe_clipping(&self) -> bool {
        self.pre_clip_severe || self.post_clip_severe
    }
}

// ---------------------------------------------------------------------------
// Main harness
// ---------------------------------------------------------------------------

// `after_is_silent`: true when `code_after` produces intentionally silent output.
fn run_cycle(
    scenario: &'static str,
    code_before: &str,
    code_after: &str,
    after_is_silent: bool,
) -> CycleResult {
    // ---- Build initial graph ----
    let mut graph = compile_graph(code_before);
    graph.enable_wall_clock_timing();
    graph.preload_samples();

    // ---- Pre-transition rendering ----
    let mut pre_buffers: Vec<Vec<f32>> = Vec::with_capacity(PRE_BUFFERS);
    for _ in 0..PRE_BUFFERS {
        let mut buf = vec![0.0f32; BUFFER_LEN];
        graph.process_buffer(&mut buf);
        pre_buffers.push(buf);
    }

    let pre_metrics: Vec<BufferMetrics> = pre_buffers.iter().map(|b| analyze_buffer(b)).collect();
    let pre_rms_avg = pre_metrics.iter().map(|m| m.rms).sum::<f32>() / PRE_BUFFERS as f32;
    let pre_nan: usize = pre_metrics.iter().map(|m| m.nan_count).sum();
    let pre_inf: usize = pre_metrics.iter().map(|m| m.inf_count).sum();
    let pre_clip_severe = pre_metrics
        .iter()
        .any(|m| m.has_severe_clip(BUFFER_LEN));

    // ---- Live reload ----
    let (mut new_graph, reload_time_us) = live_reload(&mut graph, code_after);

    // ---- Post-transition rendering ----
    let mut post_buffers: Vec<Vec<f32>> = Vec::with_capacity(POST_BUFFERS);
    for _ in 0..POST_BUFFERS {
        let mut buf = vec![0.0f32; BUFFER_LEN];
        new_graph.process_buffer(&mut buf);
        post_buffers.push(buf);
    }

    let post_metrics: Vec<BufferMetrics> =
        post_buffers.iter().map(|b| analyze_buffer(b)).collect();
    let post_rms_avg = post_metrics.iter().map(|m| m.rms).sum::<f32>() / POST_BUFFERS as f32;
    let post_nan: usize = post_metrics.iter().map(|m| m.nan_count).sum();
    let post_inf: usize = post_metrics.iter().map(|m| m.inf_count).sum();
    let post_clip_severe = post_metrics
        .iter()
        .any(|m| m.has_severe_clip(BUFFER_LEN));
    let post_silent = !after_is_silent
        && post_metrics
            .iter()
            .all(|m| m.is_silent(true));
    let post_stuck = is_stuck(
        pre_buffers.last().unwrap(),
        post_buffers.first().unwrap(),
    );

    // ---- Transition boundary discontinuity ----
    let boundary_disc = boundary_discontinuity(
        pre_buffers.last().unwrap(),
        post_buffers.first().unwrap(),
    );

    // ---- RMS jump ratio ----
    let pre_last_rms = pre_metrics.last().map(|m| m.rms).unwrap_or(0.0);
    let post_first_rms = post_metrics.first().map(|m| m.rms).unwrap_or(0.0);
    let rms_jump_ratio = if pre_last_rms > 1e-6 {
        (post_first_rms / pre_last_rms).max(pre_last_rms / post_first_rms.max(1e-6))
    } else {
        1.0
    };

    CycleResult {
        scenario,
        reload_time_us,
        pre_rms_avg,
        pre_nan,
        pre_inf,
        pre_clip_severe,
        boundary_disc,
        post_rms_avg,
        post_nan,
        post_inf,
        post_clip_severe,
        post_silent,
        post_stuck,
        rms_jump_ratio,
    }
}

// ---------------------------------------------------------------------------
// Test entry point
// ---------------------------------------------------------------------------

#[test]
fn test_audio_live_edit_glitch_harness() {
    let scenarios = scenarios();
    assert!(
        scenarios.len() >= NUM_CYCLES,
        "Need at least {} scenarios, got {}",
        NUM_CYCLES,
        scenarios.len()
    );

    println!(
        "\n=== Audio Live-Edit Glitch Harness ({} cycles) ===",
        NUM_CYCLES
    );
    println!(
        "  Buffer: {} floats = {} stereo frames ≈ {:.1} ms",
        BUFFER_LEN,
        BUFFER_LEN / 2,
        (BUFFER_LEN / 2) as f64 / SAMPLE_RATE as f64 * 1000.0
    );
    println!(
        "  Render: {}×pre + reload + {}×post per cycle",
        PRE_BUFFERS, POST_BUFFERS
    );
    println!(
        "  Thresholds: clip>{:.0}%, silence_rms<{}, dc_offset>{}, disc>{}",
        CLIP_FRACTION_THRESHOLD * 100.0,
        SILENCE_RMS_THRESHOLD,
        DC_OFFSET_THRESHOLD,
        DISCONTINUITY_THRESHOLD
    );
    println!();

    let mut results: Vec<CycleResult> = Vec::with_capacity(NUM_CYCLES);

    for (i, &(name, before, after)) in scenarios[..NUM_CYCLES].iter().enumerate() {
        // Silence is intentional for the "osc-to-constant-silence" scenario.
        let after_is_silent = name.contains("silence");

        let result = run_cycle(name, before, after, after_is_silent);

        // Per-cycle console output
        let disc_flag = if result.boundary_disc > DISCONTINUITY_THRESHOLD {
            "  ⚠ DISC"
        } else {
            ""
        };
        let nan_flag = if result.has_any_nan_or_inf() {
            "  ✗ NaN/Inf"
        } else {
            ""
        };
        let clip_flag = if result.has_severe_clipping() {
            "  ✗ CLIP"
        } else {
            ""
        };
        let silent_flag = if result.post_silent { "  ✗ SILENT" } else { "" };
        let stuck_flag = if result.post_stuck { "  ✗ STUCK" } else { "" };
        let jump_flag = if result.rms_jump_ratio > 10.0 {
            "  ⚠ RMS_JUMP"
        } else {
            ""
        };

        println!(
            "  [{:02}] {:<40} reload={:5}µs | pre_rms={:.4} post_rms={:.4} | disc={:.4} rms_ratio={:.2}{}{}{}{}{}{}",
            i + 1,
            name,
            result.reload_time_us,
            result.pre_rms_avg,
            result.post_rms_avg,
            result.boundary_disc,
            result.rms_jump_ratio,
            nan_flag,
            clip_flag,
            silent_flag,
            stuck_flag,
            disc_flag,
            jump_flag,
        );

        results.push(result);
    }

    // ---- Aggregate summary ----
    println!("\n=== Summary ===");

    let total_nan: usize = results.iter().map(|r| r.pre_nan + r.post_nan).sum();
    let total_inf: usize = results.iter().map(|r| r.pre_inf + r.post_inf).sum();
    let clip_cycles: usize = results.iter().filter(|r| r.has_severe_clipping()).count();
    let silent_cycles: usize = results.iter().filter(|r| r.post_silent).count();
    let stuck_cycles: usize = results.iter().filter(|r| r.post_stuck).count();
    let disc_cycles: usize = results
        .iter()
        .filter(|r| r.boundary_disc > DISCONTINUITY_THRESHOLD)
        .count();
    let high_jump_cycles: usize = results.iter().filter(|r| r.rms_jump_ratio > 10.0).count();

    let avg_reload_us: f64 =
        results.iter().map(|r| r.reload_time_us as f64).sum::<f64>() / results.len() as f64;
    let max_reload_us: u64 = results.iter().map(|r| r.reload_time_us).max().unwrap_or(0);
    let max_disc: f32 = results
        .iter()
        .map(|r| r.boundary_disc)
        .fold(0.0f32, f32::max);

    println!("  Cycles:            {}", NUM_CYCLES);
    println!(
        "  Reload time:       avg={:.0}µs  max={}µs",
        avg_reload_us, max_reload_us
    );
    println!("  NaN samples:       {}", total_nan);
    println!("  Inf samples:       {}", total_inf);
    println!("  Severe-clip cycles:{}", clip_cycles);
    println!("  Silent cycles:     {}", silent_cycles);
    println!("  Stuck cycles:      {}", stuck_cycles);
    println!(
        "  Disc > threshold:  {} cycles  (max={:.4})",
        disc_cycles, max_disc
    );
    println!("  High-RMS-jump:     {} cycles", high_jump_cycles);
    println!();

    // ---- Hard failures ----
    let mut failures: Vec<String> = Vec::new();

    if total_nan > 0 {
        failures.push(format!(
            "NaN samples detected: {} total across {} cycles",
            total_nan, NUM_CYCLES
        ));
    }
    if total_inf > 0 {
        failures.push(format!(
            "Inf samples detected: {} total across {} cycles",
            total_inf, NUM_CYCLES
        ));
    }
    if clip_cycles > 0 {
        let names: Vec<&str> = results
            .iter()
            .filter(|r| r.has_severe_clipping())
            .map(|r| r.scenario)
            .collect();
        failures.push(format!(
            "Severe clipping in {} cycles: {:?}",
            clip_cycles, names
        ));
    }
    if silent_cycles > 0 {
        let names: Vec<&str> = results
            .iter()
            .filter(|r| r.post_silent)
            .map(|r| r.scenario)
            .collect();
        failures.push(format!(
            "Unexpected silence in {} cycles: {:?}",
            silent_cycles, names
        ));
    }
    if stuck_cycles > 0 {
        let names: Vec<&str> = results
            .iter()
            .filter(|r| r.post_stuck)
            .map(|r| r.scenario)
            .collect();
        failures.push(format!(
            "Stuck output (old audio replayed) in {} cycles: {:?}",
            stuck_cycles, names
        ));
    }

    // ---- Report and assert ----
    if failures.is_empty() {
        println!("✅ PASSED — no hard failures detected.");
        println!();
        if disc_cycles > 0 {
            println!(
                "  Note: {} cycles had boundary discontinuity > {:.2} (flagged but not fatal).",
                disc_cycles, DISCONTINUITY_THRESHOLD
            );
            println!("  This may indicate audible clicks/pops at the transition point.");
        }
        if high_jump_cycles > 0 {
            println!(
                "  Note: {} cycles had RMS jump ratio > 10× (large gain change at reload).",
                high_jump_cycles
            );
        }
    } else {
        println!("❌ FAILURES:");
        for f in &failures {
            println!("  - {}", f);
        }
        println!();
        panic!(
            "audio_live_edit_glitch_harness: {} failure(s) detected — see output above",
            failures.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Smoke check: the harness itself compiles and basic render works
// ---------------------------------------------------------------------------

#[test]
fn test_glitch_harness_smoke_single_reload() {
    // Minimal sanity: one reload cycle, one buffer, no assertion failures.
    let before = "tempo: 1.0\nout $ sine 110 * 0.3";
    let after  = "tempo: 1.0\nout $ sine 220 * 0.3";

    let mut graph = compile_graph(before);
    graph.enable_wall_clock_timing();
    graph.preload_samples();

    let mut pre_buf = vec![0.0f32; BUFFER_LEN];
    graph.process_buffer(&mut pre_buf);

    let pre_rms = {
        let ss: f32 = pre_buf.iter().map(|x| x * x).sum();
        (ss / pre_buf.len() as f32).sqrt()
    };
    assert!(
        pre_rms > SILENCE_RMS_THRESHOLD,
        "pre-reload buffer should not be silent, got RMS {}",
        pre_rms
    );

    let (mut new_graph, _) = live_reload(&mut graph, after);

    let mut post_buf = vec![0.0f32; BUFFER_LEN];
    new_graph.process_buffer(&mut post_buf);

    let post_rms = {
        let ss: f32 = post_buf.iter().map(|x| x * x).sum();
        (ss / post_buf.len() as f32).sqrt()
    };
    assert!(
        post_rms > SILENCE_RMS_THRESHOLD,
        "post-reload buffer should not be silent, got RMS {}",
        post_rms
    );

    // Ensure no NaN/Inf in either buffer
    for &s in pre_buf.iter().chain(post_buf.iter()) {
        assert!(
            s.is_finite(),
            "non-finite sample detected: {}",
            s
        );
    }
}
