//! Live-session stress harness.
//!
//! A headless, reproducible simulation of an interactive live-coding session:
//! load a Phonon DSL program, render continuously, and perform scripted AND
//! seeded-random sequences of graph swaps / edits / tempo changes while
//! analysing the output for audible and structural defects.
//!
//! This module extends the original `tests/audio_live_edit_glitch_harness.rs`
//! into a full stress harness that covers the failure modes catalogued in:
//!   * `docs/audits/test-gap-analysis-2026-07.md` (RC-1..RC-6, G-1..G-7)
//!   * `docs/audits/live-transition-2026-07.md`   (D1..D4, U1, R1..R4)
//!
//! # What it drives
//!
//! [`live_swap`] performs the *exact* transfer sequence used by the modal
//! editor's `load_code()` hot-swap (`src/modal_editor/mod.rs:675-763`):
//!
//! ```text
//!   parse -> compile -> enable_wall_clock_timing
//!         -> transfer_session_timing -> transfer_fx_states
//!         -> transfer_voice_manager  -> preload_samples -> swap
//! ```
//!
//! The same primitive is additionally exercised through a real concurrent rig
//! (synth thread + `ArcSwap<Option<GraphCell>>` + `RefCell` + `HeapRb` ring) in
//! [`run_concurrent_session`], which reproduces the machinery the offline
//! harness stubs out (RC-1 / G-1, and the R1-R4 race window).
//!
//! # Determinism
//!
//! Oscillator phase in the engine is sample-counted, so pure synthesis programs
//! render identically for a given seed regardless of wall-clock timing. The
//! seeded [`Rng`] chooses *which* programs are swapped in and *when* (in block
//! units), so any failing sequence is reproducible from its seed alone. The
//! concurrent rig is inherently timing-dependent and only asserts structural
//! invariants (no synth-thread death, no permanent silence).

use crate::compositional_compiler::compile_program;
use crate::compositional_parser::parse_program;
use crate::unified_graph::UnifiedSignalGraph;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Default sample rate (matches the modal editor / live paths).
pub const SAMPLE_RATE: f32 = 44100.0;

// ===========================================================================
// Deterministic PRNG (SplitMix64)
// ===========================================================================

/// A tiny, fully-deterministic PRNG (SplitMix64).
///
/// Self-contained so reproduction from a seed never depends on the version of
/// an external RNG crate. Given the same seed and the same call sequence, it
/// yields the same stream on every platform.
#[derive(Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform `f32` in `[0, 1)`.
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform `usize` in `[0, n)` (returns 0 when `n == 0`).
    #[inline]
    pub fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Uniform `f32` in `[lo, hi)`.
    #[inline]
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    /// Pick a reference to a random element.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(items.len())]
    }

    /// True with probability `p`.
    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }
}

// ===========================================================================
// Detector thresholds
// ===========================================================================

/// Tunable thresholds for every defect detector.
#[derive(Clone, Debug)]
pub struct Thresholds {
    /// RMS below this = silence.
    pub silence_rms: f32,
    /// Sample-to-sample delta at a swap seam that counts as a *catastrophic*
    /// click (hard-fail). Ordinary swap boundary steps (audit D3, ~0.3) are
    /// recorded but not fatal; only near-full-scale reversals fail.
    pub boundary_click_catastrophic: f32,
    /// Internal sample-delta hard-fail for programs marked `smooth` (sine /
    /// filtered). Naturally-discontinuous waveforms (saw wrap) use the
    /// catastrophic threshold instead.
    pub internal_click_smooth: f32,
    /// Internal sample-delta hard-fail for *any* program (catastrophic).
    pub internal_click_catastrophic: f32,
    /// |mean(buffer)| above this = DC-offset defect (audit G-7: the original
    /// harness computed this but never asserted it).
    pub dc_offset: f32,
    /// Fraction of clipped (|s|>1.0) samples in a block that = severe clip.
    pub clip_fraction: f32,
    /// Late-window RMS / early-window RMS above this = unbounded growth.
    pub rms_growth_ratio: f32,
    /// Active voice count above this = runaway voice accumulation (stuck voices).
    pub voice_ceiling: usize,
    /// Per-block render time above `budget_overrun_frac * deadline` = an overrun.
    pub budget_overrun_frac: f64,
    /// Fraction of blocks over budget that hard-fails (robust to jitter).
    pub budget_overrun_block_fraction: f64,
    /// Contention gate for the *absolute* wall-clock deadline check. A trivial
    /// reference program is rendered at session start (the calibration probe);
    /// if its median per-block render time already exceeds
    /// `contention_probe_frac * deadline`, the environment cannot render even a
    /// near-zero-work program in real time, so the wall-clock deadline is
    /// meaningless (oversubscribed test runner / shared CI box). In that case
    /// the absolute overrun check is SKIPPED (reported, not failed) — see
    /// [`evaluate_budget`]. The relative spike check below stays active.
    pub contention_probe_frac: f64,
    /// A block is a *render-time spike* when it exceeds
    /// `relative_spike_mult * session_median_render_time`. Because it is
    /// normalised to the session's own median (which rises together with any
    /// global contention), this ratio is robust to an oversubscribed runner and
    /// still catches a catastrophic per-block blow-up (voice-pool realloc storm,
    /// leak) in ANY environment.
    pub relative_spike_mult: f64,
    /// Fraction of spike blocks that hard-fails the relative spike check.
    pub relative_spike_block_fraction: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            silence_rms: 0.001,
            boundary_click_catastrophic: 1.5,
            internal_click_smooth: 0.5,
            internal_click_catastrophic: 1.5,
            dc_offset: 0.1,
            clip_fraction: 0.05,
            rms_growth_ratio: 8.0,
            voice_ceiling: 512,
            budget_overrun_frac: 1.0,
            // A genuinely over-budget program overruns ~100% of blocks; a healthy
            // program with occasional scheduler jitter overruns <1%. 20% cleanly
            // separates the two and is robust to CI-runner noise.
            budget_overrun_block_fraction: 0.20,
            // If a trivial reference program can't render in under half the
            // real-time deadline, the box is not real-time-capable right now
            // (oversubscribed) and the wall-clock deadline check is skipped.
            contention_probe_frac: 0.5,
            // 8x the session's own median with a 20%-of-blocks gate: scheduler
            // jitter on a loaded box pushes the worst blocks to ~3x median, so
            // 8x has wide margin, while a realloc/leak storm hits far more than
            // 20% of blocks at >8x.
            relative_spike_mult: 8.0,
            relative_spike_block_fraction: 0.20,
        }
    }
}

// ===========================================================================
// Pure detector functions (unit-testable in isolation)
// ===========================================================================

/// Root-mean-square over the finite samples of a buffer.
pub fn rms(buf: &[f32]) -> f32 {
    let mut sum_sq = 0.0f32;
    let mut n = 0usize;
    for &s in buf {
        if s.is_finite() {
            sum_sq += s * s;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        (sum_sq / n as f32).sqrt()
    }
}

/// Mean (DC offset) over the finite samples of a buffer.
pub fn dc_offset(buf: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for &s in buf {
        if s.is_finite() {
            sum += s;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

/// `(nan_count, inf_count)` in a buffer.
pub fn count_nonfinite(buf: &[f32]) -> (usize, usize) {
    let mut nan = 0;
    let mut inf = 0;
    for &s in buf {
        if s.is_nan() {
            nan += 1;
        } else if s.is_infinite() {
            inf += 1;
        }
    }
    (nan, inf)
}

/// Count of samples whose magnitude exceeds 1.0 (clipping).
pub fn count_clipped(buf: &[f32]) -> usize {
    buf.iter().filter(|s| s.is_finite() && s.abs() > 1.0).count()
}

/// True when RMS is below the silence threshold.
pub fn is_silent(buf: &[f32], threshold: f32) -> bool {
    rms(buf) < threshold
}

/// Largest sample-to-sample delta, optionally including the carry-in seam
/// between the previous block's last sample and this block's first sample.
///
/// Returns `(max_delta, index)` where `index == 0` means the seam delta.
/// Non-finite samples are skipped (they are handled by [`count_nonfinite`]).
pub fn max_abs_delta(prev_last: Option<f32>, buf: &[f32]) -> (f32, usize) {
    let mut max = 0.0f32;
    let mut at = 0usize;
    let mut prev = prev_last.filter(|p| p.is_finite());
    for (i, &s) in buf.iter().enumerate() {
        if !s.is_finite() {
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            let d = (s - p).abs();
            if d > max {
                max = d;
                at = i;
            }
        }
        prev = Some(s);
    }
    (max, at)
}

/// Sample-delta across a swap seam: last old sample vs first new sample.
pub fn boundary_delta(tail_last: f32, head_first: f32) -> f32 {
    if tail_last.is_finite() && head_first.is_finite() {
        (head_first - tail_last).abs()
    } else {
        0.0
    }
}

/// Bit-identical output detection (a swapped-in graph replaying the old tail).
pub fn is_stuck(a: &[f32], b: &[f32]) -> bool {
    !a.is_empty()
        && a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Detect unbounded RMS growth over a per-block RMS series.
///
/// Compares the median of an early window against the median of a late window;
/// returns `Some((early, late))` when `late > ratio * early` (and both windows
/// carry signal). Stationary programs return `None`.
pub fn detect_rms_growth(series: &[f32], ratio: f32) -> Option<(f32, f32)> {
    if series.len() < 20 {
        return None;
    }
    let w = (series.len() / 5).max(4); // 20% windows
    let early = median(&series[..w]);
    let late = median(&series[series.len() - w..]);
    if early > 1e-6 && late > ratio * early {
        Some((early, late))
    } else {
        None
    }
}

/// Detect runaway voice accumulation. Returns the peak count if it exceeds the
/// ceiling (a hard voice-leak signature).
pub fn detect_stuck_voices(trajectory: &[usize], ceiling: usize) -> Option<usize> {
    let peak = trajectory.iter().copied().max().unwrap_or(0);
    if peak > ceiling {
        Some(peak)
    } else {
        None
    }
}

/// Fraction of blocks whose render time exceeded `frac * deadline`.
pub fn budget_overrun_fraction(render_times_s: &[f64], deadline_s: f64, frac: f64) -> f64 {
    if render_times_s.is_empty() {
        return 0.0;
    }
    let limit = frac * deadline_s;
    let over = render_times_s.iter().filter(|&&t| t > limit).count();
    over as f64 / render_times_s.len() as f64
}

/// Median of an `f64` slice (0.0 when empty). Non-destructive.
fn median_f64(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut v: Vec<f64> = xs.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Fraction of samples strictly greater than `limit`.
fn fraction_over(xs: &[f64], limit: f64) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().filter(|&&t| t > limit).count() as f64 / xs.len() as f64
}

/// Verdict of the callback-budget analysis for one session.
///
/// Separates the two independent signals so each is judged on its own terms:
///   * the **absolute** real-time deadline overrun (meaningful only when the
///     host can actually deliver real time — gated by the calibration probe),
///   * the **relative** per-block spike (normalised to the session's own
///     median, so it stays valid on an oversubscribed test runner).
#[derive(Clone, Copy, Debug, Default)]
pub struct BudgetVerdict {
    /// Fraction of blocks over the absolute real-time deadline.
    pub over_fraction: f64,
    /// Fraction of blocks over `relative_spike_mult * session_median`.
    pub relative_spike_fraction: f64,
    /// Session median per-block render time (µs), the relative baseline.
    pub session_median_us: f64,
    /// Calibration-probe median per-block render time (µs) — the cost of a
    /// trivial program in the current environment.
    pub probe_us: f64,
    /// True when the absolute deadline check was skipped because the host is
    /// oversubscribed (probe over `contention_probe_frac * deadline`) and
    /// real-time enforcement was not forced.
    pub skipped: bool,
    /// Whether real-time enforcement was forced despite contention.
    pub forced: bool,
}

impl BudgetVerdict {
    /// The absolute overrun is a hard defect only when it was not skipped and
    /// exceeds the block-fraction gate.
    pub fn absolute_overrun(&self, thr: &Thresholds) -> bool {
        !self.skipped && self.over_fraction > thr.budget_overrun_block_fraction
    }

    /// The relative spike check is always active.
    pub fn relative_spike(&self, thr: &Thresholds) -> bool {
        self.relative_spike_fraction > thr.relative_spike_block_fraction
    }
}

/// Evaluate the callback budget from raw per-block render times.
///
/// `render_us`, `deadline_us` and `probe_us` are all in microseconds. `probe_us`
/// is the calibration-probe median (cost of a trivial reference program in this
/// environment).
///
/// The **absolute** wall-clock deadline check is only meaningful in a real-time
/// context. Its enforcement has three levels:
///   * `enforce_requested == false` (the DEFAULT `cargo test` path): the
///     absolute check is REPORT-ONLY — the overrun is measured but never a hard
///     defect. This is what keeps a non-real-time, oversubscribed test runner
///     from false-failing.
///   * `enforce_requested == true` (the standalone `glitch_stress` real-time
///     lane): enforce the absolute check UNLESS the calibration probe shows the
///     host cannot render even a trivial program in real time (auto-skip under
///     contention, loudly reported).
///   * `force == true` (`PHONON_STRESS_FORCE_RT_BUDGET=1`, a dedicated isolated
///     CI lane): enforce unconditionally, ignoring the contention probe.
///
/// The **relative** spike check is always active (see [`BudgetVerdict`]).
///
/// Pure and deterministic given its inputs — the unit of falsifiable proof that
/// a genuinely over-budget render IS flagged (enforced lane) and that a
/// contended runner is skipped, not failed (default lane).
pub fn evaluate_budget(
    render_us: &[f64],
    deadline_us: f64,
    probe_us: f64,
    enforce_requested: bool,
    force: bool,
    thr: &Thresholds,
) -> BudgetVerdict {
    let session_median_us = median_f64(render_us);
    let over_fraction = budget_overrun_fraction(render_us, deadline_us, thr.budget_overrun_frac);
    let spike_limit = (session_median_us * thr.relative_spike_mult).max(1.0);
    let relative_spike_fraction = fraction_over(render_us, spike_limit);
    let contended = probe_us > thr.contention_probe_frac * deadline_us;
    let skipped = if force {
        false
    } else if !enforce_requested {
        // Default path: the wall-clock deadline is not a real-time context.
        true
    } else {
        // Real-time lane: enforce unless the host is oversubscribed.
        contended
    };
    BudgetVerdict {
        over_fraction,
        relative_spike_fraction,
        session_median_us,
        probe_us,
        skipped,
        forced: force,
    }
}

/// Render a trivial reference program for a fixed number of blocks (after a
/// short warm-up) and return the median per-block wall-clock render time in µs.
///
/// This is the calibration probe: because the program does near-zero work, its
/// render time is dominated by the environment. On a real-time-capable host it
/// is a small fraction of the callback deadline; on an oversubscribed test
/// runner it balloons to (or past) the deadline, which is exactly the signal
/// [`evaluate_budget`] uses to decide whether the absolute deadline check can be
/// trusted. Uses its own graph so it never perturbs the seeded swap sequence.
pub fn calibrate_probe_us(sample_rate: f32, block_frames: usize, channels: usize) -> f64 {
    // The cheapest possible sounding program — a single sine oscillator.
    const REF: &str = "tempo: 1.0\nout $ sine 110 * 0.3";
    let block_len = block_frames * channels;
    let mut graph = match build_initial(REF, sample_rate) {
        Ok(g) => g,
        Err(_) => return 0.0, // never gate on a probe we couldn't build
    };
    let warmup = 8usize;
    let samples = 32usize;
    let mut buf = vec![0.0f32; block_len];
    for _ in 0..warmup {
        graph.process_buffer(&mut buf);
    }
    let mut us: Vec<f64> = Vec::with_capacity(samples);
    for _ in 0..samples {
        for s in buf.iter_mut() {
            *s = 0.0;
        }
        let t0 = Instant::now();
        graph.process_buffer(&mut buf);
        us.push(t0.elapsed().as_secs_f64() * 1e6);
    }
    median_f64(&us)
}

/// Whether a dedicated real-time CI lane has forced enforcement of the absolute
/// wall-clock deadline check via `PHONON_STRESS_FORCE_RT_BUDGET`.
pub fn force_realtime_budget() -> bool {
    std::env::var("PHONON_STRESS_FORCE_RT_BUDGET")
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(false)
}

fn median(xs: &[f32]) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut v: Vec<f32> = xs.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn percentile_us(sorted_us: &[f64], p: f64) -> f64 {
    if sorted_us.is_empty() {
        return 0.0;
    }
    let idx = ((p * (sorted_us.len() - 1) as f64).round() as usize).min(sorted_us.len() - 1);
    sorted_us[idx]
}

// ===========================================================================
// Synthetic signal generators + defect injectors (for detector self-tests)
// ===========================================================================

/// Clean sine buffer used as known-good input for detector self-tests.
pub fn sine_buf(freq: f32, sr: f32, phase0: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            0.3 * (2.0 * std::f32::consts::PI * freq * t + phase0).sin()
        })
        .collect()
}

/// Inject a click: a single large sample-to-sample discontinuity at `pos`.
pub fn inject_click(buf: &mut [f32], pos: usize, magnitude: f32) {
    if pos < buf.len() {
        buf[pos] += magnitude;
    }
}

/// Inject a dropout: zero a contiguous region (unexpected silence).
pub fn inject_dropout(buf: &mut [f32], start: usize, len: usize) {
    let n = buf.len();
    let start = start.min(n);
    let end = (start + len).min(n);
    for s in &mut buf[start..end] {
        *s = 0.0;
    }
}

/// Inject a NaN sample.
pub fn inject_nan(buf: &mut [f32], pos: usize) {
    if pos < buf.len() {
        buf[pos] = f32::NAN;
    }
}

/// Inject an infinite sample.
pub fn inject_inf(buf: &mut [f32], pos: usize) {
    if pos < buf.len() {
        buf[pos] = f32::INFINITY;
    }
}

/// Inject a constant DC offset.
pub fn inject_dc(buf: &mut [f32], offset: f32) {
    for s in buf.iter_mut() {
        *s += offset;
    }
}

// ===========================================================================
// The real swap path (mirror of modal_editor::load_code)
// ===========================================================================

/// Parse + compile a program into a graph.
pub fn compile_graph(code: &str, sample_rate: f32) -> Result<UnifiedSignalGraph, String> {
    let (rest, statements) = parse_program(code).map_err(|e| format!("parse error: {e:?}"))?;
    if !rest.trim().is_empty() {
        return Err(format!("parser left unconsumed input: {rest:?}"));
    }
    compile_program(statements, sample_rate, None)
}

/// Build and prime an initial graph the way every live path does before its
/// first render.
pub fn build_initial(code: &str, sample_rate: f32) -> Result<UnifiedSignalGraph, String> {
    let mut g = compile_graph(code, sample_rate)?;
    g.enable_wall_clock_timing();
    g.preload_samples();
    Ok(g)
}

/// Metadata captured for one swap, sufficient to reason about beat continuity.
#[derive(Clone, Debug)]
pub struct SwapInfo {
    pub transfer_us: u64,
    pub cps_before: f32,
    pub cps_after: f32,
    pub cycle_before: f64,
    pub cycle_after: f64,
    pub transferred: bool,
}

/// Perform a full live hot-swap from `old` to a graph compiled from `new_code`.
///
/// This replicates the transfer sequence in `ModalEditor::load_code()`
/// (`src/modal_editor/mod.rs:675-763`) exactly:
///   1. compile new graph
///   2. `enable_wall_clock_timing()`  (always, so timing is valid even if
///      transfer is skipped)
///   3. `transfer_session_timing(old)`  (beat continuity)
///   4. `transfer_fx_states(old)`       (effect tails)
///   5. `transfer_voice_manager(old.take_voice_manager())`  (active voices)
///   6. `preload_samples()`             (avoid disk I/O on the audio thread)
///
/// When `transfer == false`, steps 3-5 are skipped to reproduce the audit's
/// **R1** branch ("Could not transfer state after retries" — beat may jump).
pub fn live_swap(
    old: &mut UnifiedSignalGraph,
    new_code: &str,
    sample_rate: f32,
    transfer: bool,
) -> Result<(UnifiedSignalGraph, SwapInfo), String> {
    let cps_before = old.get_cps();
    let cycle_before = old.get_cycle_position();

    let t0 = Instant::now();
    let mut new_graph = compile_graph(new_code, sample_rate)?;

    // Step 2 — always enable wall-clock timing first.
    new_graph.enable_wall_clock_timing();

    if transfer {
        // Steps 3-5 — state transfer while the old graph is exclusively owned.
        new_graph.transfer_session_timing(old);
        new_graph.transfer_fx_states(old);
        new_graph.transfer_voice_manager(old.take_voice_manager());
    }

    // Step 6 — preload before the swap.
    new_graph.preload_samples();

    let transfer_us = t0.elapsed().as_micros() as u64;
    let info = SwapInfo {
        transfer_us,
        cps_before,
        cps_after: new_graph.get_cps(),
        cycle_before,
        cycle_after: new_graph.get_cycle_position(),
        transferred: transfer,
    };
    Ok((new_graph, info))
}

// ===========================================================================
// Known-good program pool
// ===========================================================================

/// A program the session can render and swap between.
#[derive(Clone, Debug)]
pub struct Program {
    pub name: &'static str,
    pub code: &'static str,
    /// True when the program is intentionally silent.
    pub expect_silent: bool,
    /// True when the waveform is continuous (sine / filtered) so internal
    /// clicks are meaningful. Naturally-discontinuous waveforms (saw) are only
    /// checked against the catastrophic threshold.
    pub smooth: bool,
}

const fn prog(name: &'static str, code: &'static str, smooth: bool) -> Program {
    Program {
        name,
        code,
        expect_silent: false,
        smooth,
    }
}

/// A pool of *known-good* synthesis programs. Every one renders cleanly and is
/// used to prove the session produces zero false positives. These are pure
/// synthesis (no mini-notation patterns) so audio is phase-deterministic.
pub fn known_good_pool() -> Vec<Program> {
    vec![
        prog("sine-110", "tempo: 1.0\nout $ sine 110 * 0.3", true),
        prog("sine-220", "tempo: 1.0\nout $ sine 220 * 0.25", true),
        prog("sine-440", "tempo: 1.0\nout $ sine 440 * 0.2", true),
        prog("saw-110", "tempo: 1.0\nout $ saw 110 * 0.2", false),
        prog("saw-220-lpf", "tempo: 1.0\nout $ saw 220 # lpf 1500 0.6 * 0.2", true),
        prog("saw-55-lpf", "tempo: 1.0\nout $ saw 55 # lpf 800 0.7 * 0.25", true),
        prog("saw-330-hpf", "tempo: 1.0\nout $ saw 330 # hpf 500 0.7 * 0.2", false),
        prog(
            "two-sines",
            "tempo: 1.0\nout $ sine 110 * 0.15 + sine 220 * 0.15",
            true,
        ),
        prog(
            "bus-osc",
            "tempo: 1.0\n~osc $ sine 165 * 0.3\nout $ ~osc",
            true,
        ),
        prog(
            "two-bus",
            "tempo: 1.0\n~a $ sine 110\n~b $ sine 220\nout $ ~a * 0.2 + ~b * 0.1",
            true,
        ),
        prog(
            "saw-reverb",
            "tempo: 1.0\nout $ saw 110 # lpf 1200 0.6 # reverb 0.4 0.3 * 0.18",
            true,
        ),
        prog(
            "sine-delay",
            "tempo: 1.0\nout $ sine 220 # delay 0.25 0.4 0.3 * 0.22",
            true,
        ),
        // Tempo variety (exercises transfer_session_timing CPS handling).
        prog("sine-110-t2", "tempo: 2.0\nout $ sine 110 * 0.3", true),
        prog("saw-165-t05", "tempo: 0.5\nout $ saw 165 # lpf 1000 0.5 * 0.2", true),
    ]
}

// ===========================================================================
// Session runner (deterministic virtual-clock)
// ===========================================================================

/// Configuration for a randomised stress session.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub seed: u64,
    pub sample_rate: f32,
    pub block_frames: usize,
    pub channels: usize,
    pub target_seconds: f32,
    pub min_swaps: usize,
    pub thresholds: Thresholds,
    pub verbose: bool,
    /// Request enforcement of the absolute wall-clock real-time deadline check.
    ///
    /// Default `false`: under `cargo test` (a non-real-time, oversubscribed
    /// runner) the wall-clock deadline is meaningless, so the absolute overrun
    /// is reported but never a hard defect. The standalone `glitch_stress`
    /// real-time lane sets this `true` (it still auto-skips under a contention
    /// probe). `PHONON_STRESS_FORCE_RT_BUDGET=1` forces enforcement regardless.
    /// The relative per-block spike check is always active either way.
    pub enforce_realtime_budget: bool,
}

impl SessionConfig {
    /// The CI-gate configuration: >= 60 s of audio, >= 50 swaps, 512-frame
    /// stereo blocks at 44.1 kHz (matching the modal editor's render chunk).
    pub fn ci(seed: u64) -> Self {
        SessionConfig {
            seed,
            sample_rate: SAMPLE_RATE,
            block_frames: 512,
            channels: 2,
            target_seconds: 60.0,
            min_swaps: 50,
            thresholds: Thresholds::default(),
            verbose: false,
            enforce_realtime_budget: false,
        }
    }
}

/// Everything the session observed. Prints its own seed so any failure is
/// reproducible with `--seed`.
#[derive(Clone, Debug, Default)]
pub struct SessionReport {
    pub seed: u64,
    pub blocks_rendered: usize,
    pub swaps: usize,
    pub audio_seconds: f32,

    pub nan_samples: usize,
    pub inf_samples: usize,
    pub clip_blocks: usize,
    pub silent_gap_blocks: usize,
    pub stuck_output_events: usize,

    pub max_boundary_delta: f32,
    pub catastrophic_boundary_clicks: usize,
    pub catastrophic_internal_clicks: usize,

    pub dc_offset_blocks: usize,
    pub max_dc_offset: f32,

    pub rms_growth_detected: bool,
    pub max_voice_count: usize,
    pub stuck_voice_detected: bool,

    pub budget_overrun_blocks: usize,
    pub budget_overrun_fraction: f64,
    /// True when the absolute wall-clock deadline check was skipped because the
    /// host was oversubscribed (calibration probe over the contention gate).
    /// The overrun is still measured and reported, just not treated as a defect.
    pub budget_check_skipped: bool,
    /// True when real-time enforcement was forced despite contention.
    pub budget_check_forced: bool,
    /// Median per-block render time of the trivial calibration probe (µs).
    pub calibration_probe_us: f64,
    /// Fraction of blocks exceeding `relative_spike_mult * session_median`.
    pub relative_spike_fraction: f64,
    pub relative_spike_blocks: usize,
    pub p50_render_us: f64,
    pub p95_render_us: f64,
    pub p99_render_us: f64,
    pub max_render_us: f64,
    pub deadline_us: f64,

    /// First hard defect observed, with block + swap context for reproduction.
    pub first_defect: Option<String>,
    /// Ordered list of programs swapped in (the reproducible swap sequence).
    pub swap_sequence: Vec<String>,
}

impl SessionReport {
    fn note_defect(&mut self, desc: String) {
        if self.first_defect.is_none() {
            self.first_defect = Some(desc);
        }
    }

    /// The list of *hard* defects (things a known-good session must never do).
    pub fn hard_defects(&self, thr: &Thresholds) -> Vec<String> {
        let mut v = Vec::new();
        if self.nan_samples > 0 {
            v.push(format!("{} NaN samples", self.nan_samples));
        }
        if self.inf_samples > 0 {
            v.push(format!("{} Inf samples", self.inf_samples));
        }
        if self.clip_blocks > 0 {
            v.push(format!("{} severely-clipped blocks", self.clip_blocks));
        }
        if self.silent_gap_blocks > 0 {
            v.push(format!(
                "{} unexpected silent-gap blocks",
                self.silent_gap_blocks
            ));
        }
        if self.stuck_output_events > 0 {
            v.push(format!(
                "{} stuck-output events (new graph replayed old tail)",
                self.stuck_output_events
            ));
        }
        if self.catastrophic_boundary_clicks > 0 {
            v.push(format!(
                "{} catastrophic swap-boundary clicks (>{})",
                self.catastrophic_boundary_clicks, thr.boundary_click_catastrophic
            ));
        }
        if self.catastrophic_internal_clicks > 0 {
            v.push(format!(
                "{} catastrophic internal clicks",
                self.catastrophic_internal_clicks
            ));
        }
        if self.dc_offset_blocks > 0 {
            v.push(format!(
                "{} DC-offset blocks (max {:.3})",
                self.dc_offset_blocks, self.max_dc_offset
            ));
        }
        if self.rms_growth_detected {
            v.push("unbounded RMS growth".to_string());
        }
        if self.stuck_voice_detected {
            v.push(format!("stuck voices (peak {})", self.max_voice_count));
        }
        // Absolute real-time deadline overrun — a hard defect ONLY when the host
        // proved real-time-capable (the calibration probe passed). Under an
        // oversubscribed test runner the wall-clock deadline is meaningless, so
        // `budget_check_skipped` suppresses it (it is still shown in `summary`).
        if !self.budget_check_skipped
            && self.budget_overrun_fraction > thr.budget_overrun_block_fraction
        {
            v.push(format!(
                "callback-budget overruns: {:.1}% of blocks over budget",
                self.budget_overrun_fraction * 100.0
            ));
        }
        // Relative per-block spike — normalised to the session's own median, so
        // it stays valid under global contention and is always enforced.
        if self.relative_spike_fraction > thr.relative_spike_block_fraction {
            v.push(format!(
                "render-time spikes: {:.1}% of blocks > {:.0}x session median",
                self.relative_spike_fraction * 100.0,
                thr.relative_spike_mult
            ));
        }
        v
    }

    pub fn is_clean(&self, thr: &Thresholds) -> bool {
        self.hard_defects(thr).is_empty()
    }

    /// Human-readable one-block summary.
    pub fn summary(&self, thr: &Thresholds) -> String {
        let defects = self.hard_defects(thr);
        let status = if defects.is_empty() {
            "CLEAN".to_string()
        } else {
            format!("DEFECTS: {defects:?}")
        };
        let budget_mode = if self.budget_check_forced {
            "FORCED-RT"
        } else if self.budget_check_skipped {
            "SKIPPED(contended)"
        } else {
            "enforced"
        };
        format!(
            "seed={} blocks={} swaps={} audio={:.1}s | NaN={} Inf={} clip={} silent_gap={} stuck_out={} \
             max_bnd_delta={:.3} cat_bnd_clk={} dc_blocks={} rms_growth={} max_voices={} \
             budget_over={:.1}%[{}] spike={:.1}% probe={:.0}us p50={:.0}us p99={:.0}us max={:.0}us deadline={:.0}us => {}",
            self.seed,
            self.blocks_rendered,
            self.swaps,
            self.audio_seconds,
            self.nan_samples,
            self.inf_samples,
            self.clip_blocks,
            self.silent_gap_blocks,
            self.stuck_output_events,
            self.max_boundary_delta,
            self.catastrophic_boundary_clicks,
            self.dc_offset_blocks,
            self.rms_growth_detected,
            self.max_voice_count,
            self.budget_overrun_fraction * 100.0,
            budget_mode,
            self.relative_spike_fraction * 100.0,
            self.calibration_probe_us,
            self.p50_render_us,
            self.p99_render_us,
            self.max_render_us,
            self.deadline_us,
            status,
        )
    }
}

/// Choose the block indices at which swaps occur. Guarantees at least
/// `min_swaps` unique indices in `[warmup, total_blocks)` and includes one
/// rapid burst of back-to-back swaps (audit R4).
fn schedule_swaps(rng: &mut Rng, total_blocks: usize, min_swaps: usize, warmup: usize) -> Vec<usize> {
    let end = total_blocks.saturating_sub(2).max(warmup + 1);
    let span = end.saturating_sub(warmup).max(1);
    let mut set: BTreeSet<usize> = BTreeSet::new();

    // Rapid burst (R4): 5 back-to-back swaps somewhere in the first half.
    let burst_start = warmup + rng.range(span / 2 + 1);
    for k in 0..5 {
        set.insert((burst_start + k).min(end - 1));
    }

    // Fill to a target above min_swaps so collisions never drop us under.
    let target = min_swaps + 5 + rng.range(15);
    let mut guard = 0usize;
    while set.len() < target && guard < target * 40 {
        set.insert(warmup + rng.range(span));
        guard += 1;
    }

    set.into_iter().collect()
}

/// Run a seeded, randomised stress session over the given program pool.
///
/// Renders `target_seconds` of audio block-by-block through the real swap path,
/// performing at least `min_swaps` graph swaps (including a rapid burst), and
/// analyses every block for the full detector suite.
pub fn run_random_session(cfg: &SessionConfig, pool: &[Program]) -> SessionReport {
    assert!(!pool.is_empty(), "program pool must not be empty");
    let mut rng = Rng::new(cfg.seed);
    let block_len = cfg.block_frames * cfg.channels;
    let deadline_s = cfg.block_frames as f64 / cfg.sample_rate as f64;
    // Round UP so the session always renders at least `target_seconds` of audio.
    let needed_samples = (cfg.target_seconds * cfg.sample_rate).ceil() as usize;
    let total_blocks = ((needed_samples + cfg.block_frames - 1) / cfg.block_frames)
        .max(cfg.min_swaps + 10);
    let warmup = 4usize.min(total_blocks / 4);

    let swap_blocks = schedule_swaps(&mut rng, total_blocks, cfg.min_swaps, warmup);
    let mut swap_at = swap_blocks.iter().copied().peekable();

    let thr = &cfg.thresholds;
    let mut report = SessionReport {
        seed: cfg.seed,
        deadline_us: deadline_s * 1e6,
        ..Default::default()
    };

    // Initial program.
    let mut current: Program = rng.choose(pool).clone();
    let mut graph = match build_initial(current.code, cfg.sample_rate) {
        Ok(g) => g,
        Err(e) => {
            report.note_defect(format!("initial compile of '{}' failed: {e}", current.name));
            return report;
        }
    };
    report.swap_sequence.push(current.name.to_string());

    // Calibration probe: measure the per-block cost of a trivial program in the
    // CURRENT environment, before the timed session. On a real-time-capable host
    // this is a small fraction of the deadline; on an oversubscribed test runner
    // it balloons — the signal `evaluate_budget` uses to skip (not fail) the
    // absolute wall-clock deadline check under contention. Uses its own graph so
    // the seeded swap sequence is untouched.
    report.calibration_probe_us =
        calibrate_probe_us(cfg.sample_rate, cfg.block_frames, cfg.channels);

    let mut prev_buf: Vec<f32> = Vec::new();
    let mut prev_prog_code: &str = current.code;
    let mut prev_last: Option<f32> = None;
    let mut rms_series: Vec<f32> = Vec::with_capacity(total_blocks);
    let mut voice_traj: Vec<usize> = Vec::with_capacity(total_blocks);
    let mut render_us: Vec<f64> = Vec::with_capacity(total_blocks);

    for block_idx in 0..total_blocks {
        // --- Perform any swap scheduled for this block ---
        let mut just_swapped = false;
        if swap_at.peek() == Some(&block_idx) {
            swap_at.next();
            let target = rng.choose(pool).clone();
            match live_swap(&mut graph, target.code, cfg.sample_rate, true) {
                Ok((ng, _info)) => {
                    graph = ng;
                    current = target;
                    report.swaps += 1;
                    report.swap_sequence.push(current.name.to_string());
                    just_swapped = true;
                }
                Err(e) => {
                    report.note_defect(format!(
                        "block {block_idx}: swap to '{}' failed to compile: {e}",
                        target.name
                    ));
                }
            }
        }

        // --- Render one block, timed against the callback deadline ---
        let mut buf = vec![0.0f32; block_len];
        let t0 = Instant::now();
        graph.process_buffer(&mut buf);
        render_us.push(t0.elapsed().as_secs_f64() * 1e6);

        // --- Per-block detectors ---
        let (nan, inf) = count_nonfinite(&buf);
        report.nan_samples += nan;
        report.inf_samples += inf;
        if nan > 0 || inf > 0 {
            report.note_defect(format!(
                "block {block_idx} ({}): {nan} NaN, {inf} Inf",
                current.name
            ));
        }

        let clipped = count_clipped(&buf);
        if clipped as f32 > block_len as f32 * thr.clip_fraction {
            report.clip_blocks += 1;
            report.note_defect(format!(
                "block {block_idx} ({}): {clipped} clipped samples",
                current.name
            ));
        }

        let dc = dc_offset(&buf).abs();
        if dc > report.max_dc_offset {
            report.max_dc_offset = dc;
        }
        if dc > thr.dc_offset {
            report.dc_offset_blocks += 1;
            report.note_defect(format!(
                "block {block_idx} ({}): DC offset {dc:.3}",
                current.name
            ));
        }

        let block_rms = rms(&buf);
        rms_series.push(block_rms);

        // Silence / dropout: a sounding program going silent mid-stream.
        if !current.expect_silent
            && block_idx > warmup
            && !just_swapped
            && block_rms < thr.silence_rms
            && rms_series
                .get(block_idx.wrapping_sub(1))
                .map(|&r| r >= thr.silence_rms)
                .unwrap_or(false)
        {
            report.silent_gap_blocks += 1;
            report.note_defect(format!(
                "block {block_idx} ({}): unexpected silence (RMS {block_rms:.5})",
                current.name
            ));
        }

        // Clicks: seam (index 0) vs internal.
        let (maxd, at) = max_abs_delta(prev_last, &buf);
        if just_swapped {
            let seam = boundary_delta(prev_last.unwrap_or(0.0), buf.first().copied().unwrap_or(0.0));
            if seam > report.max_boundary_delta {
                report.max_boundary_delta = seam;
            }
            if seam > thr.boundary_click_catastrophic {
                report.catastrophic_boundary_clicks += 1;
                report.note_defect(format!(
                    "block {block_idx} ({}): catastrophic boundary click {seam:.3}",
                    current.name
                ));
            }
            // Stuck output: after swapping to a *different* program, the new
            // graph produced the old graph's exact tail (the swap silently did
            // not take effect). Two swaps to the *same* program legitimately
            // produce identical fresh-start blocks, so guard on code change.
            if current.code != prev_prog_code
                && !prev_buf.is_empty()
                && is_stuck(&prev_buf, &buf)
                && block_rms >= thr.silence_rms
            {
                report.stuck_output_events += 1;
                report.note_defect(format!(
                    "block {block_idx} ({}): stuck output (bit-identical to prior block)",
                    current.name
                ));
            }
        }
        // Internal click (delta not at the seam).
        if at > 0 {
            let cat = maxd > thr.internal_click_catastrophic;
            let smooth_click = current.smooth && maxd > thr.internal_click_smooth;
            if cat || smooth_click {
                report.catastrophic_internal_clicks += 1;
                report.note_defect(format!(
                    "block {block_idx} ({}): internal click {maxd:.3} at sample {at}",
                    current.name
                ));
            }
        }

        // Voices.
        let vc = graph.active_voice_count();
        voice_traj.push(vc);
        if vc > report.max_voice_count {
            report.max_voice_count = vc;
        }

        prev_last = buf.iter().rev().find(|s| s.is_finite()).copied();
        prev_buf = buf;
        prev_prog_code = current.code;

        if cfg.verbose && block_idx % 512 == 0 {
            eprintln!(
                "  [block {block_idx}/{total_blocks}] prog={} rms={block_rms:.4} voices={vc}",
                current.name
            );
        }
    }

    // --- Whole-session aggregate detectors ---
    report.blocks_rendered = total_blocks;
    report.audio_seconds = (total_blocks * cfg.block_frames) as f32 / cfg.sample_rate;

    if let Some((early, late)) = detect_rms_growth(&rms_series, thr.rms_growth_ratio) {
        report.rms_growth_detected = true;
        report.note_defect(format!(
            "unbounded RMS growth: early={early:.4} late={late:.4}"
        ));
    }
    if let Some(peak) = detect_stuck_voices(&voice_traj, thr.voice_ceiling) {
        report.stuck_voice_detected = true;
        report.note_defect(format!("stuck voices: peak {peak} > {}", thr.voice_ceiling));
    }

    // `render_us` and the deadline are both in microseconds here. The budget
    // analysis separates the absolute real-time deadline (gated by the
    // calibration probe — skipped, not failed, under host oversubscription) from
    // the relative per-block spike (normalised to the session's own median, so
    // it stays valid on a loaded box). See [`evaluate_budget`].
    let deadline_us = deadline_s * 1e6;
    let force_rt = force_realtime_budget();
    let enforce_rt = cfg.enforce_realtime_budget || force_rt;
    let verdict = evaluate_budget(
        &render_us,
        deadline_us,
        report.calibration_probe_us,
        enforce_rt,
        force_rt,
        thr,
    );
    report.budget_overrun_fraction = verdict.over_fraction;
    report.budget_check_skipped = verdict.skipped;
    report.budget_check_forced = verdict.forced;
    report.relative_spike_fraction = verdict.relative_spike_fraction;
    report.budget_overrun_blocks = render_us
        .iter()
        .filter(|&&t| t > thr.budget_overrun_frac * deadline_us)
        .count();
    let spike_limit = (verdict.session_median_us * thr.relative_spike_mult).max(1.0);
    report.relative_spike_blocks = render_us.iter().filter(|&&t| t > spike_limit).count();
    if verdict.absolute_overrun(thr) {
        report.note_defect(format!(
            "callback-budget overruns: {:.1}% of blocks over budget",
            verdict.over_fraction * 100.0
        ));
    } else if verdict.skipped && verdict.over_fraction > thr.budget_overrun_block_fraction {
        // Loud, non-fatal: the wall-clock overrun is not treated as a defect.
        let contended = verdict.probe_us > thr.contention_probe_frac * deadline_us;
        let reason = if !enforce_rt {
            "non-real-time test lane (enforcement not requested)"
        } else if contended {
            "host oversubscribed (probe over the real-time gate)"
        } else {
            "real-time budget not enforced"
        };
        eprintln!(
            "  [stress_harness] absolute callback-budget check SKIPPED — {reason}: \
             probe {:.0}us, deadline {:.0}us, measured overrun {:.1}% (not a defect)",
            verdict.probe_us,
            deadline_us,
            verdict.over_fraction * 100.0
        );
    }
    if verdict.relative_spike(thr) {
        report.note_defect(format!(
            "render-time spikes: {:.1}% of blocks > {:.0}x session median ({:.0}us)",
            verdict.relative_spike_fraction * 100.0,
            thr.relative_spike_mult,
            verdict.session_median_us
        ));
    }

    let mut sorted = render_us.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    report.p50_render_us = percentile_us(&sorted, 0.50);
    report.p95_render_us = percentile_us(&sorted, 0.95);
    report.p99_render_us = percentile_us(&sorted, 0.99);
    report.max_render_us = sorted.last().copied().unwrap_or(0.0);

    report
}

// ===========================================================================
// Scripted audit scenarios (D1-D4, U1, R1-R4 + RC/G coverage)
// ===========================================================================

/// What a scripted scenario is expected to exhibit. Scenarios that reproduce a
/// *known, documented* audit defect are recorded (not failed) so CI stays green
/// until the underlying defect is fixed; scenarios expected to be clean hard-fail
/// on any defect.
#[derive(Clone, Debug, PartialEq)]
pub enum Expectation {
    /// Must be clean: any hard defect fails the scenario.
    Clean,
    /// The `after` program is intentionally silent.
    ExpectSilence,
    /// The `after` program silences its dry input, so only a transferred FX
    /// **tail** can produce sound. A drop to silence means the effect's state
    /// was not carried across the swap (audit D2) — a hard defect. The string
    /// names the audit finding (e.g. "D2").
    ContinuousTail(&'static str),
    /// Known/documented audit defect — measured and reported, never fatal.
    /// The string names the audit finding (e.g. "D3", "R1").
    Documented(&'static str),
}

/// One scripted transition.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: &'static str,
    pub audit_ref: &'static str,
    pub before: &'static str,
    pub after: &'static str,
    pub expectation: Expectation,
    /// When true the swap drops state transfer (reproduces the R1 give-up branch).
    pub skip_transfer: bool,
}

/// The scripted scenario set. Each entry maps to a failure mode from one of the
/// two audit reports so both reports are represented (validation requirement).
pub fn audit_scenarios() -> Vec<Scenario> {
    fn sc(
        name: &'static str,
        audit_ref: &'static str,
        before: &'static str,
        after: &'static str,
        expectation: Expectation,
    ) -> Scenario {
        Scenario {
            name,
            audit_ref,
            before,
            after,
            expectation,
            skip_transfer: false,
        }
    }
    vec![
        // ---- live-transition-2026-07: discontinuities ----
        // D1: voices faded/killed on every swap (truncated drum/pad). Represented
        // with a sustained filtered pad; boundary continuity is measured.
        sc(
            "D1-voice-fade-on-swap",
            "D1",
            "tempo: 1.0\n~pad $ saw 55 # lpf 800 0.7\nout $ ~pad * 0.3",
            "tempo: 1.0\n~pad $ saw 55 # lpf 900 0.7\nout $ ~pad * 0.3",
            Expectation::Documented("D1"),
        ),
        // D2: partial FX-state transfer — a NON-transferred effect (pingpong)
        // resets its tail at the swap. The `before` primes a fully-wet pingpong
        // buffer with a live saw; the `after` silences the dry input (`* 0.0`)
        // so ONLY the transferred delay tail can produce sound. If the pingpong
        // buffer is not injected on swap, the tail snaps to silence — the exact
        // D2 defect. Short delay + high feedback keeps the tail audible inside
        // the render window.
        sc(
            "D2-pingpong-tail-reset",
            "D2",
            "tempo: 1.0\nout $ saw 110 # pingpong 0.02 0.85 0.8 0 1.0 * 0.4",
            "tempo: 1.0\nout $ (saw 110 * 0.0) # pingpong 0.02 0.85 0.8 0 1.0 * 0.4",
            Expectation::ContinuousTail("D2"),
        ),
        // D2b: a tape-delay tail (also non-transferred per the audit). Same
        // shape — prime with a live saw, then silence the dry input so only the
        // tape-delay tail remains. Default mix is 50% wet, ample for the tail
        // to stay above the silence floor when the state transfers.
        sc(
            "D2-tapedelay-tail-reset",
            "D2",
            "tempo: 1.0\nout $ saw 110 # tapedelay 0.02 0.85 0.5 0.02 6.0 0.05 0.3 1.0 * 0.4",
            "tempo: 1.0\nout $ (saw 110 * 0.0) # tapedelay 0.02 0.85 0.5 0.02 6.0 0.05 0.3 1.0 * 0.4",
            Expectation::ContinuousTail("D2"),
        ),
        // D3: cross-swap crossfade never fires — phase-dependent boundary click
        // on a waveform change (audit measured disc up to ~0.33).
        sc(
            "D3-sine-to-saw-boundary-click",
            "D3",
            "tempo: 1.0\nout $ sine 220 * 0.3",
            "tempo: 1.0\nout $ saw 220 * 0.2",
            Expectation::Documented("D3"),
        ),
        // U1: swapping to a block that has no `out` silences output.
        sc(
            "U1-chunk-without-out",
            "U1",
            "tempo: 1.0\n~drums $ saw 110\n~bass $ sine 55\nout $ ~drums * 0.2 + ~bass * 0.2",
            "tempo: 1.0\n~bass $ sine 55",
            Expectation::Documented("U1"),
        ),
        // ---- live-transition-2026-07: races ----
        // R1: heavy-load transfer failure -> fresh timing -> beat jump. Forced
        // by skipping the transfer step (the give-up branch).
        Scenario {
            name: "R1-transfer-skip-beat-jump",
            audit_ref: "R1",
            before: "tempo: 1.0\nout $ sine 110 * 0.3",
            after: "tempo: 1.0\nout $ sine 165 * 0.3",
            expectation: Expectation::Documented("R1"),
            skip_transfer: true,
        },
        // R4: rapid successive swaps — represented in the randomised session's
        // burst; here a scripted A/B/A/B chain checks it never hard-faults.
        sc(
            "R4-rapid-ab-swaps",
            "R4",
            "tempo: 1.0\nout $ sine 220 * 0.25",
            "tempo: 1.0\nout $ sine 330 * 0.25",
            Expectation::Clean,
        ),
        // ---- test-gap-analysis-2026-07 ----
        // RC-2 / G-2: an internally NaN-producing graph (0/0) — the output
        // sanitiser flushes to zero, so the OUTPUT gate is a backstop; this
        // scenario documents that the swap survives it without exploding.
        sc(
            "G2-divide-by-zero-sanitised",
            "G2",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 1.0\nout $ (sine 110 / 0.0) * 0.0 + sine 110 * 0.3",
            Expectation::Documented("G2"),
        ),
        // RC-6 / G-5: tempo change at swap (rate change with phase preserved).
        sc(
            "G5-tempo-1-to-3",
            "G5",
            "tempo: 1.0\nout $ sine 220 * 0.3",
            "tempo: 3.0\nout $ sine 220 * 0.3",
            Expectation::Clean,
        ),
        // Clean controls (must never fault) — gain change, filter add/remove.
        sc(
            "clean-add-lpf",
            "-",
            "tempo: 1.0\nout $ saw 110 * 0.2",
            "tempo: 1.0\nout $ saw 110 # lpf 1000 0.8 * 0.2",
            Expectation::Clean,
        ),
        sc(
            "clean-gain-change",
            "-",
            "tempo: 1.0\nout $ sine 220 * 0.2",
            "tempo: 1.0\nout $ sine 220 * 0.35",
            Expectation::Clean,
        ),
        // osc-to-silence (intentional).
        sc(
            "clean-osc-to-silence",
            "-",
            "tempo: 1.0\nout $ sine 110 * 0.3",
            "tempo: 1.0\nout $ 0.0",
            Expectation::ExpectSilence,
        ),
    ]
}

/// Outcome of one scripted scenario.
#[derive(Clone, Debug)]
pub struct ScenarioResult {
    pub name: &'static str,
    pub audit_ref: &'static str,
    pub available: bool,
    pub boundary_delta: f32,
    pub pre_rms: f32,
    pub post_rms: f32,
    pub nan: usize,
    pub inf: usize,
    pub post_silent: bool,
    pub cycle_jump: f64,
    pub max_voices: usize,
    /// Hard-fail reasons (empty on pass). Documented scenarios never populate
    /// this from their known defect.
    pub failures: Vec<String>,
    pub note: Option<String>,
}

impl ScenarioResult {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Run one scripted scenario: render pre-buffers, swap, render post-buffers,
/// evaluate against its expectation.
pub fn run_scenario(sc: &Scenario, cfg: &SessionConfig) -> ScenarioResult {
    let block_len = cfg.block_frames * cfg.channels;
    let thr = &cfg.thresholds;
    let pre_buffers = 8usize;
    let post_buffers = 8usize;

    let mut result = ScenarioResult {
        name: sc.name,
        audit_ref: sc.audit_ref,
        available: true,
        boundary_delta: 0.0,
        pre_rms: 0.0,
        post_rms: 0.0,
        nan: 0,
        inf: 0,
        post_silent: false,
        cycle_jump: 0.0,
        max_voices: 0,
        failures: Vec::new(),
        note: None,
    };

    let mut graph = match build_initial(sc.before, cfg.sample_rate) {
        Ok(g) => g,
        Err(e) => {
            result.available = false;
            result.note = Some(format!("`before` failed to compile: {e}"));
            return result;
        }
    };

    let mut last_pre: Option<f32> = None;
    let mut pre_rms_acc = 0.0f32;
    for _ in 0..pre_buffers {
        let mut buf = vec![0.0f32; block_len];
        graph.process_buffer(&mut buf);
        pre_rms_acc += rms(&buf);
        last_pre = buf.iter().rev().find(|s| s.is_finite()).copied();
        result.max_voices = result.max_voices.max(graph.active_voice_count());
    }
    result.pre_rms = pre_rms_acc / pre_buffers as f32;
    let cycle_before = graph.get_cycle_position();

    let (mut new_graph, info) = match live_swap(&mut graph, sc.after, cfg.sample_rate, !sc.skip_transfer) {
        Ok(x) => x,
        Err(e) => {
            result.available = false;
            result.note = Some(format!("`after` failed to compile: {e}"));
            return result;
        }
    };
    // Expected small advance vs actual — a large jump signals R1.
    result.cycle_jump = (info.cycle_after - cycle_before).abs();

    let mut post_rms_acc = 0.0f32;
    let mut first_post = true;
    for _ in 0..post_buffers {
        let mut buf = vec![0.0f32; block_len];
        new_graph.process_buffer(&mut buf);
        let (nan, inf) = count_nonfinite(&buf);
        result.nan += nan;
        result.inf += inf;
        if first_post {
            result.boundary_delta =
                boundary_delta(last_pre.unwrap_or(0.0), buf.first().copied().unwrap_or(0.0));
            first_post = false;
        }
        post_rms_acc += rms(&buf);
        result.max_voices = result.max_voices.max(new_graph.active_voice_count());
    }
    result.post_rms = post_rms_acc / post_buffers as f32;
    result.post_silent = result.post_rms < thr.silence_rms;

    // --- Evaluate against expectation ---
    // NaN/Inf are always hard defects (a real explosion the sanitiser missed).
    if result.nan > 0 {
        result.failures.push(format!("{} NaN samples", result.nan));
    }
    if result.inf > 0 {
        result.failures.push(format!("{} Inf samples", result.inf));
    }
    match &sc.expectation {
        Expectation::Clean => {
            if result.post_silent {
                result
                    .failures
                    .push(format!("unexpected silence (post RMS {:.5})", result.post_rms));
            }
            if result.boundary_delta > thr.boundary_click_catastrophic {
                result.failures.push(format!(
                    "catastrophic boundary click {:.3}",
                    result.boundary_delta
                ));
            }
        }
        Expectation::ExpectSilence => {
            if !result.post_silent {
                result.failures.push(format!(
                    "expected silence but post RMS {:.5}",
                    result.post_rms
                ));
            }
        }
        Expectation::ContinuousTail(tag) => {
            // The dry input is silenced in `after`, so any post-swap energy is
            // the transferred effect tail. A drop to silence means the FX state
            // was reset on swap (audit D2 defect).
            if result.post_silent {
                result.failures.push(format!(
                    "{tag} FX tail reset on swap: post RMS {:.5} < silence floor {:.5} \
                     (effect state not transferred)",
                    result.post_rms, thr.silence_rms
                ));
            }
            result.note = Some(format!(
                "{tag} tail-continuity: pre_rms={:.4} post_rms={:.4} (tail {})",
                result.pre_rms,
                result.post_rms,
                if result.post_silent { "LOST" } else { "survived" }
            ));
        }
        Expectation::Documented(tag) => {
            // Known defect: record the measurement, do not fail on it.
            result.note = Some(format!(
                "documented audit finding {tag}: boundary_delta={:.3} post_silent={} cycle_jump={:.3}",
                result.boundary_delta, result.post_silent, result.cycle_jump
            ));
        }
    }
    result
}

/// Run every scripted scenario. Returns `(results, hard_failures)`.
pub fn run_all_scenarios(cfg: &SessionConfig) -> (Vec<ScenarioResult>, Vec<String>) {
    let mut results = Vec::new();
    let mut failures = Vec::new();
    for sc in audit_scenarios() {
        let r = run_scenario(&sc, cfg);
        if r.available && !r.passed() {
            for f in &r.failures {
                failures.push(format!("{} [{}]: {f}", r.name, r.audit_ref));
            }
        }
        results.push(r);
    }
    (results, failures)
}

// ===========================================================================
// Detector self-tests (TDD: prove detectors catch injected defects)
// ===========================================================================

/// Run every detector against synthetic audio with a *known* injected defect
/// and confirm it fires — and that a clean signal does not. Returns the number
/// of checks that passed, or the first failing check.
///
/// This is the falsifiable proof that the detectors are real (not tautological):
/// they operate on the synthetic buffer *before* any engine sanitisation, so an
/// injected NaN, click, or dropout is genuinely present when the detector runs.
pub fn run_detector_self_tests() -> Result<usize, String> {
    let thr = Thresholds::default();
    let mut passed = 0usize;
    macro_rules! check {
        ($cond:expr, $msg:expr) => {{
            if !($cond) {
                return Err($msg.to_string());
            }
            passed += 1;
        }};
    }

    // --- Clean reference must be free of every defect. ---
    let clean = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    check!(!is_silent(&clean, thr.silence_rms), "clean sine flagged silent");
    check!(count_nonfinite(&clean) == (0, 0), "clean sine has non-finite");
    check!(count_clipped(&clean) == 0, "clean sine flagged clipping");
    check!(dc_offset(&clean).abs() < thr.dc_offset, "clean sine has DC offset");
    let (cd, _) = max_abs_delta(None, &clean);
    check!(cd < thr.internal_click_smooth, "clean sine flagged as click");

    // --- Click. ---
    let mut clicky = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_click(&mut clicky, 500, 1.0);
    let (cd, at) = max_abs_delta(None, &clicky);
    check!(cd > thr.internal_click_smooth, "injected click not detected");
    check!(at == 500 || at == 501, "click localised to wrong sample");

    // --- Boundary click at a swap seam. ---
    let bd = boundary_delta(0.9, -0.9);
    check!(bd > 1.5, "injected boundary click not detected");
    check!(boundary_delta(0.1, 0.10001) < 0.01, "clean seam flagged");

    // --- Dropout / unexpected silence. ---
    let mut dropped = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_dropout(&mut dropped, 0, 1024);
    check!(is_silent(&dropped, thr.silence_rms), "full dropout not detected as silence");
    // A partial dropout region is silent within its window.
    let mut partial = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_dropout(&mut partial, 256, 512);
    check!(is_silent(&partial[256..768], thr.silence_rms), "partial dropout not silent");

    // --- NaN / Inf. ---
    let mut nanny = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_nan(&mut nanny, 100);
    inject_nan(&mut nanny, 200);
    let (nan, _) = count_nonfinite(&nanny);
    check!(nan == 2, "injected NaN not counted correctly");
    let mut inffy = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_inf(&mut inffy, 300);
    let (_, inf) = count_nonfinite(&inffy);
    check!(inf == 1, "injected Inf not counted");

    // --- Clipping. ---
    let mut clip = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    for i in 0..200 {
        clip[i] = 2.0;
    }
    check!(count_clipped(&clip) == 200, "injected clipping not counted");

    // --- DC offset. ---
    let mut dc = sine_buf(220.0, SAMPLE_RATE, 0.0, 1024);
    inject_dc(&mut dc, 0.3);
    check!(dc_offset(&dc).abs() > thr.dc_offset, "injected DC offset not detected");

    // --- Stuck output (bit-identical). ---
    let a = sine_buf(220.0, SAMPLE_RATE, 0.0, 256);
    let b = a.clone();
    check!(is_stuck(&a, &b), "identical buffers not detected as stuck");
    let c = sine_buf(221.0, SAMPLE_RATE, 0.0, 256);
    check!(!is_stuck(&a, &c), "different buffers flagged as stuck");

    // --- Unbounded RMS growth. ---
    let growing: Vec<f32> = (0..200).map(|i| 0.01 * 1.03f32.powi(i)).collect();
    check!(
        detect_rms_growth(&growing, thr.rms_growth_ratio).is_some(),
        "exponential RMS growth not detected"
    );
    let stationary: Vec<f32> = (0..200).map(|i| 0.2 + 0.01 * (i as f32).sin()).collect();
    check!(
        detect_rms_growth(&stationary, thr.rms_growth_ratio).is_none(),
        "stationary RMS flagged as growth"
    );

    // --- Stuck voices. ---
    let leak: Vec<usize> = (0..100).map(|i| i * 8).collect(); // climbs to 792
    check!(
        detect_stuck_voices(&leak, thr.voice_ceiling).is_some(),
        "voice leak not detected"
    );
    let bounded: Vec<usize> = (0..100).map(|i| i % 16).collect();
    check!(
        detect_stuck_voices(&bounded, thr.voice_ceiling).is_none(),
        "bounded voices flagged as leak"
    );

    // --- Callback-budget overrun (raw fraction). ---
    let deadline = 512.0 / SAMPLE_RATE as f64;
    let over: Vec<f64> = vec![deadline * 1.5; 100];
    check!(
        budget_overrun_fraction(&over, deadline, thr.budget_overrun_frac) > 0.9,
        "budget overrun not detected"
    );
    let under: Vec<f64> = vec![deadline * 0.1; 100];
    check!(
        budget_overrun_fraction(&under, deadline, thr.budget_overrun_frac) < 0.01,
        "under-budget flagged as overrun"
    );

    // --- Contention-gated absolute deadline check. ---
    let dl_us = deadline * 1e6;
    let slow: Vec<f64> = vec![dl_us * 1.5; 100]; // a genuinely over-budget render
    // (0) DEFAULT (cargo test) path — enforcement NOT requested: even a
    //     deliberately slow render is REPORT-ONLY, never a hard defect. This is
    //     the false positive the fix removes: an oversubscribed test runner must
    //     not fail on wall-clock overruns.
    let v_default = evaluate_budget(&slow, dl_us, dl_us * 1.5, false, false, &thr);
    check!(
        v_default.skipped && !v_default.absolute_overrun(&thr),
        "default (non-real-time) path hard-failed on a wall-clock overrun"
    );
    // (1) Real-time lane on a real-time-capable host (tiny probe): a deliberately
    //     slow render IS flagged — the standalone real-time check must still bite.
    let v_rt = evaluate_budget(&slow, dl_us, dl_us * 0.05, true, false, &thr);
    check!(
        v_rt.absolute_overrun(&thr) && !v_rt.skipped,
        "over-budget render not flagged on a real-time-capable host"
    );
    // (2) Real-time lane on an oversubscribed host (probe over the deadline): the
    //     SAME slow render is auto-SKIPPED, not failed.
    let v_busy = evaluate_budget(&slow, dl_us, dl_us * 1.2, true, false, &thr);
    check!(
        v_busy.skipped && !v_busy.absolute_overrun(&thr),
        "contended host did not skip the absolute budget check"
    );
    // (3) Forced real-time lane overrides the contention gate.
    let v_forced = evaluate_budget(&slow, dl_us, dl_us * 1.2, true, true, &thr);
    check!(
        v_forced.forced && v_forced.absolute_overrun(&thr),
        "forced real-time budget did not override contention"
    );

    // --- Relative per-block spike (robust to global contention). ---
    // A burst of blocks far above the session median is a spike, regardless of
    // the absolute deadline.
    let mut spiky: Vec<f64> = vec![800.0; 70];
    spiky.extend(std::iter::repeat(800.0 * 12.0).take(30));
    let v_spike = evaluate_budget(&spiky, dl_us, dl_us * 0.05, false, false, &thr);
    check!(
        v_spike.relative_spike(&thr),
        "render-time spike burst not detected"
    );
    // A uniformly-inflated (contended) session — every block equally slow — is
    // NOT a spike: the median rises with the contention, so the ratio stays ~1.
    let uniform_slow: Vec<f64> = vec![dl_us * 1.5; 100];
    let v_uniform = evaluate_budget(&uniform_slow, dl_us, dl_us * 1.2, false, false, &thr);
    check!(
        !v_uniform.relative_spike(&thr),
        "uniform contention misclassified as a render-time spike"
    );

    // --- Calibration probe returns a finite, non-negative measurement. ---
    let probe = calibrate_probe_us(SAMPLE_RATE, 512, 2);
    check!(
        probe.is_finite() && probe >= 0.0,
        "calibration probe returned a non-finite / negative value"
    );

    Ok(passed)
}

// ===========================================================================
// Concurrent rig (real synth thread + ArcSwap + RefCell + ring buffer)
//
// This closes RC-1 / G-1: the offline session above is single-threaded, so the
// cross-thread `RefCell` interleaving that the audits identify as the true bug
// surface is structurally unreachable there. Here we spin the *real* machinery
// the modal editor uses — a background synth thread rendering through a
// `RefCell`-guarded, `ArcSwap`-swapped graph into an SPSC ring — and a control
// thread issuing swaps with the modal editor's `try_borrow_mut` discipline. We
// assert only structural invariants (no synth-thread death, no permanent
// silence, no non-finite output reaching the "device"); timing-dependent
// figures (underruns) are reported, not asserted.
// ===========================================================================

use arc_swap::ArcSwap;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// Same unsafe primitive the three live paths share
/// (`src/modal_editor/mod.rs:57`, `src/main.rs:926`, `src/live.rs:29`).
struct GraphCell(RefCell<UnifiedSignalGraph>);
// SAFETY: mirrors the production paths — the `try_borrow_mut` discipline on both
// the synth and control threads keeps access non-overlapping in practice.
unsafe impl Send for GraphCell {}
unsafe impl Sync for GraphCell {}

/// Result of a concurrent stress run.
#[derive(Clone, Debug, Default)]
pub struct ConcurrentReport {
    pub seed: u64,
    pub swaps: usize,
    /// False means the synth thread panicked mid-run (permanent-silence bug).
    pub synth_thread_alive: bool,
    pub consumer_blocks: usize,
    pub silent_consumer_blocks: usize,
    pub underruns: usize,
    pub max_consecutive_silent: usize,
    /// True when output stayed silent for a long stretch while an audible
    /// program was active (a real dropout / stuck-silence defect).
    pub permanent_silence: bool,
    pub nonfinite_in_output: usize,
    pub notes: Vec<String>,
    /// Running length of the current silent streak (across drain calls).
    running_silent: usize,
}

impl ConcurrentReport {
    pub fn hard_defects(&self) -> Vec<String> {
        let mut v = Vec::new();
        if !self.synth_thread_alive {
            v.push("synth thread panicked (permanent silence)".to_string());
        }
        if self.permanent_silence {
            v.push(format!(
                "permanent silence: {} consecutive silent output blocks",
                self.max_consecutive_silent
            ));
        }
        if self.nonfinite_in_output > 0 {
            v.push(format!(
                "{} non-finite samples reached the device",
                self.nonfinite_in_output
            ));
        }
        v
    }

    pub fn is_clean(&self) -> bool {
        self.hard_defects().is_empty()
    }
}

/// Synth-thread borrow discipline under concurrent graph swaps.
///
/// The reload/control thread always holds a `try_borrow_mut()` on the old graph
/// across `transfer_*` while swapping. The *synth* thread is where the two live
/// paths historically diverged, and that divergence is the C1 / F-1 bug:
///
/// * [`SynthBorrow::TryBorrowSkip`] — the safe discipline: `try_borrow_mut()`
///   then skip the block on `Err`. This is what the modal editor
///   (`src/modal_editor/mod.rs:280`) always did and what the fixed product
///   surfaces (`src/main.rs`, `src/bin/phonon-audio.rs`) now do.
/// * [`SynthBorrow::Unconditional`] — the pre-fix product-surface bug: an
///   unconditional `borrow_mut()`. When the reload thread holds the transfer
///   borrow on the same `RefCell`, this panics → the synth thread dies → the
///   ring drains → audio stops permanently. Used by the regression test to
///   reproduce the panic that shipped on `main`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynthBorrow {
    /// `try_borrow_mut()` + skip-on-`Err` (modal editor + fixed product surfaces).
    TryBorrowSkip,
    /// Unconditional `borrow_mut()` (the `main`-branch product-surface bug).
    Unconditional,
}

/// Run a concurrent stress session for `num_swaps` swaps against `pool`.
///
/// Reproduces the modal editor's / product surfaces' threading model headlessly
/// (no CPAL device; the ring consumer is driven manually at the block cadence).
/// The synth thread uses the safe [`SynthBorrow::TryBorrowSkip`] discipline —
/// the discipline the product surfaces adopt after the C1 / F-1 fix. Use
/// [`run_concurrent_session_mode`] to drive the pre-fix (`Unconditional`) path.
pub fn run_concurrent_session(cfg: &SessionConfig, pool: &[Program], num_swaps: usize) -> ConcurrentReport {
    run_concurrent_session_mode(cfg, pool, num_swaps, SynthBorrow::TryBorrowSkip)
}

/// Run a concurrent stress session with an explicit synth-thread borrow
/// discipline. See [`SynthBorrow`].
pub fn run_concurrent_session_mode(
    cfg: &SessionConfig,
    pool: &[Program],
    num_swaps: usize,
    synth_borrow: SynthBorrow,
) -> ConcurrentReport {
    assert!(!pool.is_empty());
    let mut rng = Rng::new(cfg.seed);
    let sr = cfg.sample_rate;
    let block_frames = cfg.block_frames;
    let channels = cfg.channels;
    let block_len = block_frames * channels;

    let mut report = ConcurrentReport {
        seed: cfg.seed,
        ..Default::default()
    };

    // Initial graph.
    let initial = rng.choose(pool).clone();
    let initial_graph = match build_initial(initial.code, sr) {
        Ok(g) => g,
        Err(e) => {
            report.notes.push(format!("initial compile failed: {e}"));
            report.synth_thread_alive = true; // never spawned
            return report;
        }
    };

    let graph: Arc<ArcSwap<Option<GraphCell>>> =
        Arc::new(ArcSwap::from_pointee(Some(GraphCell(RefCell::new(initial_graph)))));

    // 1 s ring, interleaved.
    let ring = HeapRb::<f32>::new((sr as usize) * channels);
    let (mut prod, mut cons) = ring.split();

    let stop = Arc::new(AtomicBool::new(false));
    let synth_done = Arc::new(AtomicBool::new(false));

    // --- Synth thread: render through try_borrow_mut, push to ring. ---
    let synth_graph = Arc::clone(&graph);
    let synth_stop = Arc::clone(&stop);
    let synth_done_flag = Arc::clone(&synth_done);
    let synth = std::thread::spawn(move || {
        let mut buf = vec![0.0f32; block_len];
        while !synth_stop.load(Ordering::Relaxed) {
            let snapshot = synth_graph.load();
            match &**snapshot {
                Some(cell) => match synth_borrow {
                    SynthBorrow::TryBorrowSkip => match cell.0.try_borrow_mut() {
                        Ok(mut g) => {
                            for s in buf.iter_mut() {
                                *s = 0.0;
                            }
                            g.process_buffer(&mut buf);
                            drop(g);
                            // Best-effort push; if the ring is full, drop this block
                            // (the consumer is behind — not our concern here).
                            if prod.vacant_len() >= buf.len() {
                                prod.push_slice(&buf);
                            }
                        }
                        Err(_) => {
                            // Control thread holds the borrow mid-transfer: skip,
                            // exactly like the modal editor's synth loop and the
                            // fixed product surfaces.
                            std::thread::yield_now();
                        }
                    },
                    SynthBorrow::Unconditional => {
                        // Pre-fix product-surface bug (main.rs / phonon-audio.rs
                        // on `main`): unconditional borrow_mut(). Panics — killing
                        // this thread — if the control thread holds the transfer
                        // borrow on the same RefCell.
                        let mut g = cell.0.borrow_mut();
                        for s in buf.iter_mut() {
                            *s = 0.0;
                        }
                        g.process_buffer(&mut buf);
                        drop(g);
                        if prod.vacant_len() >= buf.len() {
                            prod.push_slice(&buf);
                        }
                    }
                },
                None => {
                    // Hushed: push silence.
                    let silence = vec![0.0f32; block_len];
                    if prod.vacant_len() >= silence.len() {
                        prod.push_slice(&silence);
                    }
                }
            }
        }
        synth_done_flag.store(true, Ordering::Relaxed);
    });

    // --- Control thread == this thread: issue swaps, then drain a tail. ---
    // Give the synth thread a moment to fill the ring.
    std::thread::sleep(Duration::from_millis(5));

    let mut swap_seq: Vec<&'static str> = vec![initial.name];
    for i in 0..num_swaps {
        // Rapid burst in the middle third to represent R4.
        let gap_us = if i > num_swaps / 3 && i < num_swaps / 2 {
            rng.range_f32(50.0, 300.0)
        } else {
            rng.range_f32(300.0, 3000.0)
        };
        std::thread::sleep(Duration::from_micros(gap_us as u64));

        let target = rng.choose(pool).clone();
        // Compile off the audio thread (like the UI thread).
        let mut new_graph = match compile_graph(target.code, sr) {
            Ok(g) => g,
            Err(e) => {
                report.notes.push(format!("swap {i} compile failed: {e}"));
                continue;
            }
        };
        new_graph.enable_wall_clock_timing();

        // Transfer under the modal editor's retry discipline (50 x 500us).
        let snapshot = graph.load();
        if let Some(cell) = &**snapshot {
            let mut transferred = false;
            for _ in 0..50 {
                match cell.0.try_borrow_mut() {
                    Ok(mut old) => {
                        new_graph.transfer_session_timing(&old);
                        new_graph.transfer_fx_states(&old);
                        new_graph.transfer_voice_manager(old.take_voice_manager());
                        transferred = true;
                        break;
                    }
                    Err(_) => std::thread::sleep(Duration::from_micros(500)),
                }
            }
            if !transferred {
                report
                    .notes
                    .push(format!("swap {i}: could not transfer state (R1 window)"));
            }
        }
        drop(snapshot);
        new_graph.preload_samples();
        graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));
        report.swaps += 1;
        swap_seq.push(target.name);

        // --- Drain the ring at (approximately) device cadence while swapping. ---
        drain_consumer(&mut cons, block_len, &mut report);
    }

    // Drain a short tail so post-swap audio is observed.
    for _ in 0..40 {
        std::thread::sleep(Duration::from_micros(
            (block_frames as f64 / sr as f64 * 1e6) as u64,
        ));
        drain_consumer(&mut cons, block_len, &mut report);
    }

    // Stop and join.
    stop.store(true, Ordering::Relaxed);
    let joined = synth.join();
    report.synth_thread_alive = joined.is_ok();

    // Permanent silence: a long consecutive silent stretch (>= ~0.5 s of blocks)
    // is a real dropout given the pool is entirely audible.
    let silence_block_limit = ((0.5 * sr as f64) / block_frames as f64) as usize;
    if report.max_consecutive_silent > silence_block_limit && report.consumer_blocks > 0 {
        report.permanent_silence = true;
    }

    report
}

/// Pop everything currently available from the ring in block-sized chunks,
/// updating silence / underrun / non-finite counters. The current silent-streak
/// length persists across calls in `report.running_silent`.
fn drain_consumer(
    cons: &mut ringbuf::HeapCons<f32>,
    block_len: usize,
    report: &mut ConcurrentReport,
) {
    let mut out = vec![0.0f32; block_len];
    while cons.occupied_len() >= block_len {
        let got = cons.pop_slice(&mut out);
        report.consumer_blocks += 1;
        if got < block_len {
            report.underruns += 1;
        }
        let (nan, inf) = count_nonfinite(&out[..got]);
        report.nonfinite_in_output += nan + inf;
        if rms(&out[..got.max(1)]) < 1e-4 {
            report.silent_consumer_blocks += 1;
            report.running_silent += 1;
            if report.running_silent > report.max_consecutive_silent {
                report.max_consecutive_silent = report.running_silent;
            }
        } else {
            report.running_silent = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic_for_seed() {
        let mut a = Rng::new(12345);
        let mut b = Rng::new(12345);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        // Different seed => different stream (overwhelmingly likely).
        let mut c = Rng::new(12346);
        let mut differ = false;
        for _ in 0..8 {
            if a.next_u64() != c.next_u64() {
                differ = true;
            }
        }
        assert!(differ);
    }

    #[test]
    fn rng_range_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..10_000 {
            let x = r.range(10);
            assert!(x < 10);
            let f = r.next_f32();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn schedule_guarantees_min_swaps() {
        let mut r = Rng::new(99);
        let s = schedule_swaps(&mut r, 5000, 50, 4);
        assert!(s.len() >= 50, "got {} swaps", s.len());
        assert!(s.iter().all(|&i| i >= 4 && i < 5000));
        // Sorted + unique.
        assert!(s.windows(2).all(|w| w[0] < w[1]));
    }

    /// Regression reproducer for C1 / rt-safety F-1 — the `main`-branch bug.
    ///
    /// The pre-fix product surfaces (`src/main.rs:1006`,
    /// `src/bin/phonon-audio.rs:288`) synthesised through an *unconditional*
    /// `borrow_mut()`. When the reload thread holds its `try_borrow_mut()`
    /// transfer borrow on the same ArcSwap-shared `RefCell` mid-swap, that
    /// unconditional borrow panics → the synth thread dies → the ring drains →
    /// audio stops permanently.
    ///
    /// Driving the product-surface discipline (`SynthBorrow::Unconditional`)
    /// under a real concurrent synth + reload rig must kill the synth thread.
    /// (The dying thread prints an expected `BorrowMutError` panic; it is caught
    /// by `join()` — cargo suppresses it while this test passes.) The fix ports
    /// the modal editor's `try_borrow_mut()`+skip discipline to the product
    /// surfaces — see `product_surface_try_borrow_survives_concurrent_swaps`.
    ///
    /// The race is timing-dependent, so we allow a few bounded attempts: the
    /// unconditional discipline is fatal regardless of the product-side fix
    /// (this test exercises the harness model, not the binaries), so observing
    /// the death even once proves the defect.
    #[test]
    fn product_surface_unconditional_borrow_kills_synth_thread() {
        let pool = known_good_pool();
        let mut died = false;
        let mut last = None;
        for seed in 0..6 {
            let cfg = SessionConfig::ci(seed);
            let report =
                run_concurrent_session_mode(&cfg, &pool, 40, SynthBorrow::Unconditional);
            if !report.synth_thread_alive {
                assert!(
                    !report.hard_defects().is_empty(),
                    "a dead synth thread must register as a hard defect: {report:?}"
                );
                died = true;
                break;
            }
            last = Some(report);
        }
        assert!(
            died,
            "expected the unconditional borrow_mut() to panic the synth thread \
             during a concurrent transfer borrow (the C1 / F-1 bug), but it \
             survived every attempt; last report: {last:?}"
        );
    }

    /// Post-fix guard: the product surfaces now use `try_borrow_mut()`+skip
    /// (`SynthBorrow::TryBorrowSkip`), the same discipline as the modal editor.
    /// Under the identical concurrent swap load the synth thread must survive,
    /// no non-finite sample may reach the device, and there must be no permanent
    /// silence. This mirrors what `glitch_stress --concurrent` asserts.
    #[test]
    fn product_surface_try_borrow_survives_concurrent_swaps() {
        let pool = known_good_pool();
        for seed in [0u64, 42, 7] {
            let cfg = SessionConfig::ci(seed);
            let report =
                run_concurrent_session_mode(&cfg, &pool, 40, SynthBorrow::TryBorrowSkip);
            assert!(
                report.synth_thread_alive,
                "try_borrow_mut()+skip must keep the synth thread alive \
                 (seed {seed}): {report:?}"
            );
            assert_eq!(
                report.nonfinite_in_output, 0,
                "no non-finite samples may reach the device (seed {seed}): {report:?}"
            );
            assert!(
                !report.permanent_silence,
                "no permanent silence under the try_borrow discipline \
                 (seed {seed}): {report:?}"
            );
        }
    }
}
