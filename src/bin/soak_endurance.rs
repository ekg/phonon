//! Soak / endurance harness — simulated multi-hour live-coding session.
//!
//! Improvement-plan I3 / test-gap P1-A: an *endurance regression net*.
//!
//! Waves 1+2 fixed a whole class of **accumulation** bugs:
//!   * P2 — `Box::leak` on every parse (fixed),
//!   * G4 — voice-pool heap growth (fixed),
//!   * T2 — `f32` trigger-time precision cliff at ~10 h (widened to `f64`).
//!
//! Nothing *guards* those over a real multi-hour set, and the feature surface is
//! now large. This harness simulates a long session (compressed wall-clock via
//! many cycles) and asserts **no degradation**:
//!   * resident memory does not grow unboundedly (bounded resource proxy —
//!     node count / voice-pool size / active-voice count stays bounded across
//!     repeated graph swaps),
//!   * trigger phase does not drift (onset positions stay sample-accurate late
//!     in the run — validates the T2 `f64` trigger clock),
//!   * no voiceless windows / stuck-silence / stuck-loud over the run,
//!   * RMS stays in range (no slow blow-up or decay),
//!   * a **noise-heavy** scenario stays deterministic (doubles as the regression
//!     assertion for `wave3-noise-rng-hotpath`: per-node seeded PRNG, no
//!     per-sample `thread_rng()`).
//!
//! It drives the *public* engine API (`UnifiedSignalGraph::render` /
//! `process_buffer`, `set_cycle_position`, `node_count`, `voice_pool_size`,
//! `active_voice_count`) and reuses `stress_harness` detectors **read-only**
//! (imported public items — this file never edits `stress_harness.rs`).
//!
//! ## Binary usage
//! ```text
//! soak_endurance                 # default: ~2 simulated hours, all scenarios
//! soak_endurance --hours 8       # opt-in longer run
//! soak_endurance --cycles 500000 # by cycle budget instead of hours
//! soak_endurance --seed 42 --scenario noise
//! ```
//! The CI test (`tests/soak_endurance.rs`) runs a short bounded, deterministic
//! pass of the same code.

use phonon::stress_harness as sh;
use phonon::unified_graph::UnifiedSignalGraph;

/// Session sample rate (matches the modal editor / live paths).
pub const SR: f32 = 44100.0;

// ===========================================================================
// Soak-specific pure detectors (NOT in stress_harness; unit-testable)
// ===========================================================================

/// Sample-accurate onset positions in a mono buffer.
///
/// Rising edge of short-window RMS, with a refractory long enough to collapse a
/// single percussive hit's body into ONE onset. `refractory_samples` must be
/// shorter than the spacing of the pattern under test so distinct hits stay
/// separate. Deterministic: the same buffer always yields the same positions,
/// so early-vs-late comparison isolates *engine* drift, not detector noise.
pub fn onset_positions(buf: &[f32], sr: f32, threshold: f32, refractory_samples: usize) -> Vec<usize> {
    let win = ((sr * 0.005) as usize).max(1); // 5 ms energy window
    let hop = (win / 4).max(1);
    let mut prev = 0.0f32;
    let mut last: Option<usize> = None;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + win <= buf.len() {
        let w = &buf[i..i + win];
        let mut sum_sq = 0.0f32;
        for &s in w {
            if s.is_finite() {
                sum_sq += s * s;
            }
        }
        let r = (sum_sq / win as f32).sqrt();
        if (r - prev).max(0.0) > threshold {
            let past = last.map(|l| i - l >= refractory_samples).unwrap_or(true);
            if past {
                out.push(i);
                last = Some(i);
            }
        }
        prev = r;
        i += hop;
    }
    out
}

/// Inter-onset intervals (sample counts between consecutive onsets).
pub fn inter_onset_intervals(onsets: &[usize]) -> Vec<i64> {
    onsets.windows(2).map(|w| w[1] as i64 - w[0] as i64).collect()
}

/// Collapse a per-block RMS series into a windowed-RMS series: each output is the
/// RMS over `window_blocks` consecutive blocks (`sqrt(mean(block_rms^2))`).
///
/// This is the right granularity for judging loudness of *sparse percussion*:
/// most individual blocks between hits are near-silent, so the per-block median
/// hugs zero, but a ~1 s window always straddles several hits and reports the
/// true program energy. Non-overlapping windows.
pub fn windowed_rms_series(block_rms: &[f32], window_blocks: usize) -> Vec<f32> {
    let w = window_blocks.max(1);
    let mut out = Vec::with_capacity(block_rms.len() / w + 1);
    let mut start = 0;
    while start + w <= block_rms.len() {
        let mut sum_sq = 0.0f32;
        let mut n = 0usize;
        for &r in &block_rms[start..start + w] {
            if r.is_finite() {
                sum_sq += r * r;
                n += 1;
            }
        }
        out.push(if n == 0 { 0.0 } else { (sum_sq / n as f32).sqrt() });
        start += w;
    }
    out
}

/// Median of an `i64` slice (0 when empty). Non-destructive.
pub fn median_i64(xs: &[i64]) -> i64 {
    if xs.is_empty() {
        return 0;
    }
    let mut v = xs.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

/// The worst deviation (in samples) of any inter-onset interval from the
/// expected grid spacing. A large value = the trigger clock has drifted or
/// quantised (the T2 `f32` failure mode at high cycle counts).
///
/// Returns `0` for an empty interval list (no onsets to compare — the caller
/// must separately assert onsets were present).
pub fn max_ioi_deviation(iois: &[i64], expected: i64) -> i64 {
    iois.iter().map(|&x| (x - expected).abs()).max().unwrap_or(0)
}

/// Detect an upward trend (leak signature) in a bounded resource-count series.
///
/// Compares the maximum of an early window (first 20 %) to the maximum of a late
/// window (last 20 %). A healthy soak cycling through a fixed program pool sees
/// the *same* multiset of counts in both windows, so early and late maxima match
/// and this returns `None`. A genuine leak (node count or voice-pool size that
/// climbs with every swap) makes the late max exceed the early max by more than
/// `slack`, returning `Some((early_max, late_max))`.
pub fn detect_count_growth(series: &[usize], slack: usize) -> Option<(usize, usize)> {
    if series.len() < 10 {
        return None;
    }
    let w = (series.len() / 5).max(2); // 20 % windows
    let early_max = series[..w].iter().copied().max().unwrap_or(0);
    let late_max = series[series.len() - w..].iter().copied().max().unwrap_or(0);
    if late_max > early_max + slack {
        Some((early_max, late_max))
    } else {
        None
    }
}

/// Count *voiceless windows*: contiguous, non-overlapping windows of
/// `window_blocks` per-block RMS values whose maximum stays below `silence`.
///
/// Per-block silence is meaningless for sparse percussion (there is real silence
/// between hits), so voicelessness is judged over a window wide enough to contain
/// several expected events: a healthy sounding program always has at least one
/// hit — hence one loud block — inside a ~1 s window. A window that is silent
/// throughout is a genuine stuck-silence / dead-synth signature. Windows before
/// `warmup_blocks` are skipped (initial priming).
pub fn count_voiceless_windows(
    series: &[f32],
    window_blocks: usize,
    silence: f32,
    warmup_blocks: usize,
) -> usize {
    let w = window_blocks.max(1);
    let mut count = 0usize;
    let mut start = warmup_blocks;
    while start + w <= series.len() {
        let win = &series[start..start + w];
        let peak = win.iter().copied().filter(|x| x.is_finite()).fold(0.0f32, f32::max);
        if peak < silence {
            count += 1;
        }
        start += w;
    }
    count
}

/// True when a block is *stuck loud*: nearly every sample is pinned within 1 %
/// of the master-limiter ceiling. This is the signature of a runaway feedback
/// bus whose blow-up the limiter clamps to a near-DC full-scale rail (see the
/// "audio NaN blow-up modes" note: feedback-bus blow-up goes stuck-LOUD, IIR
/// blow-up goes stuck-SILENT). `ceiling` is the master limiter ceiling
/// (default 0.95).
pub fn is_stuck_loud(buf: &[f32], ceiling: f32, fraction: f32) -> bool {
    if buf.is_empty() {
        return false;
    }
    let rail = ceiling * 0.99;
    let pinned = buf.iter().filter(|s| s.is_finite() && s.abs() >= rail).count();
    pinned as f32 / buf.len() as f32 >= fraction
}

// ===========================================================================
// Resource-proxy tracking (bounded-memory guard)
// ===========================================================================

/// Per-observation snapshot of the engine's bounded resource proxies. A true
/// resident-memory counter would need a custom global allocator; the task
/// accepts a proxy — these three are the leak-relevant ones the public API
/// exposes.
#[derive(Clone, Debug, Default)]
pub struct ResourceTrack {
    /// Live `SignalNode` count of the current graph (parse-leak / graph-bloat proxy).
    pub node_counts: Vec<usize>,
    /// Voice-manager pool size (G4 heap-growth proxy — must stay bounded).
    pub voice_pools: Vec<usize>,
    /// Active (sounding) voice count (G4 stuck-voice proxy).
    pub active_voices: Vec<usize>,
}

impl ResourceTrack {
    pub fn observe(&mut self, g: &UnifiedSignalGraph) {
        self.node_counts.push(g.node_count());
        self.voice_pools.push(g.voice_pool_size());
        self.active_voices.push(g.active_voice_count());
    }
}

// ===========================================================================
// Scenario programs
// ===========================================================================

/// One program the soak session renders and swaps between.
#[derive(Clone, Debug)]
pub struct SoakProgram {
    pub name: &'static str,
    pub code: &'static str,
    /// True for the noise-heavy programs (the `wave3-noise-rng-hotpath` regressors).
    pub noise: bool,
    /// True when the waveform is intentionally near-silent (never used here, but
    /// kept for the silent-gap guard's symmetry with `stress_harness`).
    pub expect_silent: bool,
}

const fn prog(name: &'static str, code: &'static str, noise: bool) -> SoakProgram {
    SoakProgram { name, code, noise, expect_silent: false }
}

/// A pool of known-good programs spanning the feature surface a long set
/// exercises: percussion (sample voice path — the T2 trigger clock), tonal
/// synthesis + filters/fx (accumulating IIR state), and **noise-heavy** buses
/// (the per-node seeded PRNG hot path). Every one renders cleanly; the session
/// proves zero degradation as it accumulates.
pub fn soak_pool() -> Vec<SoakProgram> {
    vec![
        // Percussion — the sample voice path with a hot trigger clock.
        prog("kick4", "tempo: 1.0\nout $ s \"bd*4\" * 0.7", false),
        prog("beat", "tempo: 1.0\nout $ s \"bd sn hh cp\" * 0.7", false),
        prog("kick-hats", "tempo: 1.0\nout $ s \"bd*2 hh*4\" * 0.6", false),
        // Tonal synthesis + accumulating filter/fx state.
        prog("saw-lpf", "tempo: 1.0\nout $ saw 110 # lpf 1500 0.6 * 0.25", false),
        prog("sine-delay", "tempo: 1.0\nout $ sine 220 # delay 0.25 0.4 0.3 * 0.22", false),
        prog("saw-reverb", "tempo: 1.0\nout $ saw 110 # lpf 1200 0.6 # reverb 0.4 0.3 * 0.18", false),
        // Noise-heavy — the seeded-PRNG hot path (wave3-noise-rng-hotpath regressors).
        prog("white-lpf", "tempo: 1.0\nout $ noise # lpf 3000 0.7 * 0.4", true),
        prog("pink", "tempo: 1.0\nout $ pink * 0.4", true),
        prog(
            "noise-bed",
            "tempo: 1.0\n~a $ noise * 0.25\n~b $ pink * 0.2\nout $ (~a + ~b) # lpf 2500 0.6",
            true,
        ),
        prog(
            "noise-perc",
            "tempo: 1.0\n~n $ noise # hpf 4000 0.6 * 0.3\nout $ s \"bd*4\" * 0.6 + ~n",
            true,
        ),
    ]
}

/// The default noise-heavy program used by the determinism regression assertion.
pub const NOISE_DET_CODE: &str =
    "tempo: 1.0\n~a $ noise * 0.3\n~b $ pink * 0.2\nout $ (~a + ~b) # lpf 3000 0.6";

/// The default percussion program used by the onset-drift (T2) probe: 4 evenly
/// spaced kicks per cycle. At cps = 1 and 44.1 kHz the grid spacing is
/// 44100 / 4 = 11025 samples.
pub const ONSET_PROBE_CODE: &str = "tempo: 1.0\nout $ s \"bd*4\"";
pub const ONSET_PROBE_GRID: i64 = 11025;

// ===========================================================================
// Deterministic offline swap (carries the accumulating clock forward)
// ===========================================================================

/// Compile a fresh graph and carry the *accumulating* cycle clock forward from
/// the old graph, staying in deterministic offline mode.
///
/// This is the soak analogue of the live swap path: unlike a fresh render, the
/// clock keeps climbing across every swap, so the trigger phase must survive
/// arbitrarily large cycle positions (the T2 stressor). Offline mode keeps the
/// run reproducible (no wall-clock dependence).
pub fn offline_swap(old: &UnifiedSignalGraph, new_code: &str, sr: f32) -> Result<UnifiedSignalGraph, String> {
    let carried = old.get_cycle_position();
    let mut ng = sh::compile_graph(new_code, sr)?;
    ng.preload_samples();
    ng.enable_raw_probe(true);
    ng.set_cycle_position(carried); // keep the global clock continuous & climbing
    Ok(ng)
}

/// Build the initial offline graph (deterministic; NOT wall-clock).
pub fn build_offline(code: &str, sr: f32) -> Result<UnifiedSignalGraph, String> {
    let mut g = sh::compile_graph(code, sr)?;
    g.preload_samples();
    g.enable_raw_probe(true);
    Ok(g)
}

// ===========================================================================
// Soak session config + report
// ===========================================================================

#[derive(Clone, Debug)]
pub struct SoakConfig {
    pub seed: u64,
    pub sample_rate: f32,
    pub block_frames: usize,
    pub channels: usize,
    /// Cycles per second (tempo). The compressed clock climbs at this rate.
    pub cps: f32,
    /// Total simulated cycles to render (the endurance budget). At cps = 1 a
    /// cycle ~ one bar; a multi-hour set is ~cps * 3600 * hours cycles.
    pub target_cycles: f64,
    /// Swap the live graph every this many cycles (exercise the swap path).
    pub swap_every_cycles: f64,
    pub thresholds: sh::Thresholds,
    pub verbose: bool,
    /// Restrict the pool to noise-heavy programs (the `--scenario noise` lane).
    pub noise_only: bool,
}

impl SoakConfig {
    /// Short, deterministic, bounded CI configuration. Renders ~6 s of audio at
    /// cps=2 with a swap every half-cycle (~12 swaps) so the swap path and the
    /// accumulation detectors get real coverage while `cargo test` stays fast in
    /// a debug build. The `--bin` scales this up via [`hours`](Self::hours) /
    /// [`cycles`](Self::cycles) for a true multi-hour run.
    pub fn ci(seed: u64) -> Self {
        SoakConfig {
            seed,
            sample_rate: SR,
            block_frames: 512,
            channels: 2,
            cps: 2.0,
            target_cycles: 12.0, // ~6 s of audio at cps=2
            swap_every_cycles: 1.0, // swap every ~0.5 s → ~12 swaps
            thresholds: sh::Thresholds::default(),
            verbose: false,
            noise_only: false,
        }
    }

    /// A longer opt-in run scaled from a wall-clock hour budget (compressed:
    /// `cps` cycles per simulated second).
    pub fn hours(seed: u64, hours: f64) -> Self {
        let mut c = SoakConfig::ci(seed);
        c.cps = 1.0;
        c.target_cycles = hours * 3600.0 * c.cps as f64;
        c.swap_every_cycles = 8.0;
        c
    }

    pub fn cycles(seed: u64, cycles: f64) -> Self {
        let mut c = SoakConfig::ci(seed);
        c.cps = 1.0;
        c.target_cycles = cycles;
        c.swap_every_cycles = 8.0;
        c
    }
}

/// Everything the soak observed. Prints its own seed so any failure reproduces.
#[derive(Clone, Debug, Default)]
pub struct SoakReport {
    pub seed: u64,
    pub blocks_rendered: usize,
    pub swaps: usize,
    pub audio_seconds: f64,
    pub final_cycle: f64,

    // Level-3 audio integrity across the whole run.
    pub nan_samples: usize,
    pub inf_samples: usize,
    pub raw_nonfinite_samples: usize,
    pub max_raw_peak: f32,
    pub clip_blocks: usize,
    pub voiceless_windows: usize,
    pub stuck_loud_blocks: usize,
    pub stuck_output_events: usize,
    pub rms_growth_detected: bool,
    /// True when both the early and late windowed median RMS sit inside a sane
    /// band (neither decayed toward silence nor railed toward the limiter).
    pub rms_in_band: bool,
    pub early_rms: f32,
    pub late_rms: f32,
    pub min_block_rms: f32,
    pub max_block_rms: f32,

    // Bounded-resource proxy (memory guard).
    pub node_growth: Option<(usize, usize)>,
    pub voice_pool_growth: Option<(usize, usize)>,
    pub max_node_count: usize,
    pub max_voice_pool: usize,
    pub max_active_voices: usize,
    pub stuck_voice_detected: bool,

    pub first_defect: Option<String>,
    pub swap_sequence: Vec<String>,
}

impl SoakReport {
    fn note(&mut self, d: String) {
        if self.first_defect.is_none() {
            self.first_defect = Some(d);
        }
    }

    /// Hard defects — things a healthy multi-hour session must NEVER do.
    pub fn defects(&self, thr: &sh::Thresholds) -> Vec<String> {
        let mut v = Vec::new();
        if self.nan_samples > 0 {
            v.push(format!("{} NaN samples", self.nan_samples));
        }
        if self.inf_samples > 0 {
            v.push(format!("{} Inf samples", self.inf_samples));
        }
        if self.raw_nonfinite_samples > 0 {
            v.push(format!(
                "{} RAW non-finite samples (internal blow-up masked as silence)",
                self.raw_nonfinite_samples
            ));
        }
        if self.clip_blocks > 0 {
            v.push(format!("{} severely-clipped blocks", self.clip_blocks));
        }
        if self.voiceless_windows > 0 {
            v.push(format!("{} voiceless (stuck-silent) windows", self.voiceless_windows));
        }
        if !self.rms_in_band {
            v.push(format!(
                "RMS out of band (early={:.4} late={:.4}; expected 0.02..0.95)",
                self.early_rms, self.late_rms
            ));
        }
        if self.stuck_loud_blocks > 0 {
            v.push(format!("{} stuck-loud (limiter-railed) windows", self.stuck_loud_blocks));
        }
        if self.stuck_output_events > 0 {
            v.push(format!("{} stuck-output events (swap replayed old tail)", self.stuck_output_events));
        }
        if self.rms_growth_detected {
            v.push(format!("unbounded RMS growth (early={:.4} late={:.4})", self.early_rms, self.late_rms));
        }
        if let Some((e, l)) = self.node_growth {
            v.push(format!("unbounded node-count growth (early_max={e} late_max={l})"));
        }
        if let Some((e, l)) = self.voice_pool_growth {
            v.push(format!("unbounded voice-pool growth (early_max={e} late_max={l}) — G4 regression"));
        }
        if self.stuck_voice_detected {
            v.push(format!("stuck voices (peak {} > {})", self.max_active_voices, thr.voice_ceiling));
        }
        v
    }

    pub fn is_clean(&self, thr: &sh::Thresholds) -> bool {
        self.defects(thr).is_empty()
    }

    pub fn summary(&self, thr: &sh::Thresholds) -> String {
        let defects = self.defects(thr);
        let status = if defects.is_empty() { "CLEAN".to_string() } else { format!("DEFECTS: {defects:?}") };
        format!(
            "seed={} blocks={} swaps={} audio={:.1}s final_cycle={:.0} | NaN={} Inf={} raw_nf={} clip={} \
             voiceless={} stuck_loud={} stuck_out={} rms[early={:.4} late={:.4} min={:.4} max={:.4} in_band={}] rms_growth={} \
             nodes[max={} growth={:?}] voice_pool[max={} growth={:?}] active_voices[max={}] => {}",
            self.seed, self.blocks_rendered, self.swaps, self.audio_seconds, self.final_cycle,
            self.nan_samples, self.inf_samples, self.raw_nonfinite_samples, self.clip_blocks,
            self.voiceless_windows, self.stuck_loud_blocks, self.stuck_output_events,
            self.early_rms, self.late_rms, self.min_block_rms, self.max_block_rms, self.rms_in_band, self.rms_growth_detected,
            self.max_node_count, self.node_growth, self.max_voice_pool, self.voice_pool_growth,
            self.max_active_voices, status,
        )
    }
}

// ===========================================================================
// The long-run driver
// ===========================================================================

/// Drive a compressed multi-hour session: render block-by-block, swap graphs
/// every `swap_every_cycles` (carrying the accumulating clock), and analyse
/// every block for accumulation defects using the read-only `stress_harness`
/// detectors plus the soak-specific resource/stuck-loud detectors.
pub fn run_soak(cfg: &SoakConfig) -> SoakReport {
    let mut pool = soak_pool();
    if cfg.noise_only {
        pool.retain(|p| p.noise);
    }
    assert!(!pool.is_empty(), "program pool must not be empty");

    let mut rng = sh::Rng::new(cfg.seed);
    let block_len = cfg.block_frames * cfg.channels;
    let thr = &cfg.thresholds;
    let cycles_per_block = cfg.cps as f64 * cfg.block_frames as f64 / cfg.sample_rate as f64;
    let total_blocks = ((cfg.target_cycles / cycles_per_block).ceil() as usize).max(40);
    let swap_every_blocks = (cfg.swap_every_cycles / cycles_per_block).ceil().max(1.0) as usize;
    let warmup = 3usize;

    let mut report = SoakReport { seed: cfg.seed, ..Default::default() };
    report.min_block_rms = f32::INFINITY;

    let mut current = rng.choose(&pool).clone();
    let mut graph = match build_offline(current.code, cfg.sample_rate) {
        Ok(g) => g,
        Err(e) => {
            report.note(format!("initial compile of '{}' failed: {e}", current.name));
            return report;
        }
    };
    // Set the tempo (compile already honours `tempo:`; assert cps for safety).
    graph.set_cps(cfg.cps);
    report.swap_sequence.push(current.name.to_string());

    let mut res = ResourceTrack::default();
    let mut rms_series: Vec<f32> = Vec::with_capacity(total_blocks);
    let mut prev_buf: Vec<f32> = Vec::new();
    let mut prev_code: &str = current.code;

    for block_idx in 0..total_blocks {
        // --- periodic swap (carry the climbing clock) ---
        let mut just_swapped = false;
        if block_idx > 0 && block_idx % swap_every_blocks == 0 {
            let target = rng.choose(&pool).clone();
            match offline_swap(&graph, target.code, cfg.sample_rate) {
                Ok(mut ng) => {
                    ng.set_cps(cfg.cps);
                    graph = ng;
                    current = target;
                    report.swaps += 1;
                    report.swap_sequence.push(current.name.to_string());
                    just_swapped = true;
                }
                Err(e) => report.note(format!("block {block_idx}: swap to '{}' failed: {e}", target.name)),
            }
        }

        // --- render one block ---
        let mut buf = vec![0.0f32; block_len];
        graph.process_buffer(&mut buf);

        // --- Level-3 per-block integrity (read-only stress_harness detectors) ---
        let (nan, inf) = sh::count_nonfinite(&buf);
        report.nan_samples += nan;
        report.inf_samples += inf;
        if nan > 0 || inf > 0 {
            report.note(format!("block {block_idx} ({}): {nan} NaN, {inf} Inf", current.name));
        }

        let probe = graph.last_raw_probe();
        if probe.raw_peak > report.max_raw_peak {
            report.max_raw_peak = probe.raw_peak;
        }
        if probe.raw_nonfinite > 0 {
            report.raw_nonfinite_samples += probe.raw_nonfinite;
            report.note(format!("block {block_idx} ({}): {} RAW non-finite", current.name, probe.raw_nonfinite));
        }

        let clipped = sh::count_clipped(&buf);
        if clipped as f32 > block_len as f32 * thr.clip_fraction {
            report.clip_blocks += 1;
            report.note(format!("block {block_idx} ({}): {clipped} clipped samples", current.name));
        }

        let block_rms = sh::rms(&buf);
        rms_series.push(block_rms);
        if block_rms < report.min_block_rms {
            report.min_block_rms = block_rms;
        }
        if block_rms > report.max_block_rms {
            report.max_block_rms = block_rms;
        }

        // (Stuck-silence is judged post-hoc over ~1 s windows — see below — because
        // sparse percussion legitimately goes quiet between hits within a block.)

        // Stuck-loud: output railed at the limiter ceiling (feedback blow-up).
        if is_stuck_loud(&buf, graph.get_master_limiter_ceiling(), 0.9) {
            report.stuck_loud_blocks += 1;
            report.note(format!("block {block_idx} ({}): stuck-loud (limiter-railed)", current.name));
        }

        // Stuck-output: a swap that silently did not take effect (new graph
        // replayed the old graph's exact tail). Guard on a real code change.
        if just_swapped
            && current.code != prev_code
            && !prev_buf.is_empty()
            && sh::is_stuck(&prev_buf, &buf)
            && block_rms >= thr.silence_rms
        {
            report.stuck_output_events += 1;
            report.note(format!("block {block_idx} ({}): stuck output (bit-identical to prior block)", current.name));
        }

        // --- bounded-resource proxy (sample once per block) ---
        res.observe(&graph);

        prev_buf = buf;
        prev_code = current.code;

        if cfg.verbose && block_idx % 2048 == 0 {
            eprintln!(
                "  [block {block_idx}/{total_blocks}] cycle={:.0} prog={} rms={block_rms:.4} nodes={} voices={}",
                graph.get_cycle_position(), current.name, graph.node_count(), graph.active_voice_count()
            );
        }
    }

    // --- whole-session aggregates ---
    report.blocks_rendered = total_blocks;
    report.audio_seconds = (total_blocks * cfg.block_frames) as f64 / cfg.sample_rate as f64;
    report.final_cycle = graph.get_cycle_position();

    // RMS stationarity — judged on the ~1 s WINDOWED RMS (sparse-percussion safe;
    // the per-block median hugs zero because most inter-hit blocks are quiet).
    let window_blocks = (cfg.sample_rate as usize / cfg.block_frames).max(1); // ~1 s
    let wrms = windowed_rms_series(&rms_series, window_blocks);
    let w = (wrms.len() / 5).max(1);
    report.early_rms = median_f32(&wrms[..w.min(wrms.len())]);
    report.late_rms = median_f32(&wrms[wrms.len().saturating_sub(w)..]);
    if sh::detect_rms_growth(&wrms, thr.rms_growth_ratio).is_some() {
        report.rms_growth_detected = true;
        report.note(format!("unbounded RMS growth: early={:.4} late={:.4}", report.early_rms, report.late_rms));
    }
    // "Stays in range (no slow blow-up or decay)": both windows must sit inside a
    // sane band. Below the floor = decayed toward silence; above the ceiling =
    // railed toward the limiter.
    const RMS_FLOOR: f32 = 0.02;
    const RMS_CEIL: f32 = 0.95;
    report.rms_in_band = report.early_rms >= RMS_FLOOR
        && report.early_rms <= RMS_CEIL
        && report.late_rms >= RMS_FLOOR
        && report.late_rms <= RMS_CEIL;
    if !report.rms_in_band {
        report.note(format!("RMS out of band: early={:.4} late={:.4}", report.early_rms, report.late_rms));
    }
    if report.min_block_rms.is_infinite() {
        report.min_block_rms = 0.0;
    }

    // Voiceless windows (stuck-silence) over ~1 s windows — robust to the silent
    // gaps between sparse percussion hits.
    report.voiceless_windows =
        count_voiceless_windows(&rms_series, window_blocks, thr.silence_rms, warmup);
    if report.voiceless_windows > 0 {
        report.note(format!("{} voiceless windows (~1s each)", report.voiceless_windows));
    }

    // Bounded-resource proxy verdicts.
    report.max_node_count = res.node_counts.iter().copied().max().unwrap_or(0);
    report.max_voice_pool = res.voice_pools.iter().copied().max().unwrap_or(0);
    report.max_active_voices = res.active_voices.iter().copied().max().unwrap_or(0);
    // node count varies per program; allow generous slack (any real leak trends
    // strictly upward and blows past this). Voice pool must be near-constant.
    report.node_growth = detect_count_growth(&res.node_counts, 64);
    report.voice_pool_growth = detect_count_growth(&res.voice_pools, 8);
    if let Some((e, l)) = report.node_growth {
        report.note(format!("node-count growth early_max={e} late_max={l}"));
    }
    if let Some((e, l)) = report.voice_pool_growth {
        report.note(format!("voice-pool growth early_max={e} late_max={l}"));
    }
    if sh::detect_stuck_voices(&res.active_voices, thr.voice_ceiling).is_some() {
        report.stuck_voice_detected = true;
        report.note(format!("stuck voices peak {} > {}", report.max_active_voices, thr.voice_ceiling));
    }

    report
}

/// Median of an `f32` slice (0.0 when empty). Non-destructive.
pub fn median_f32(xs: &[f32]) -> f32 {
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

// ===========================================================================
// Focused probes (used directly by the CI tests)
// ===========================================================================

/// Result of the onset-drift (T2) probe.
#[derive(Clone, Debug)]
pub struct OnsetDriftResult {
    pub early_onsets: usize,
    pub late_onsets: usize,
    pub early_iois: Vec<i64>,
    pub late_iois: Vec<i64>,
    pub early_median_ioi: i64,
    pub late_median_ioi: i64,
    /// Worst deviation of any late-window IOI from the expected grid (samples).
    pub late_max_deviation: i64,
    /// Worst deviation of any early-window IOI from the expected grid (samples).
    pub early_max_deviation: i64,
}

/// Render a percussive program at an early cycle and again at a *late* cycle
/// (fast-forwarded via `set_cycle_position`, deterministic offline mode), then
/// compare onset timing. The late window validates the T2 `f64` trigger clock:
/// with `f64`, onsets stay locked to the grid at arbitrarily large cycle counts;
/// with the reverted `f32` they quantise to a coarse (cycle-ULP) grid and this
/// deviation explodes.
pub fn onset_drift_probe(
    code: &str,
    expected_grid: i64,
    late_cycle: f64,
    window_secs: f32,
    sr: f32,
) -> OnsetDriftResult {
    let n = (window_secs * sr) as usize;
    let refractory = (expected_grid as f32 * 0.6) as usize; // < grid spacing
    let thresh = 0.05f32;

    let mut early_g = build_offline(code, sr).expect("compile early");
    let early = early_g.render(n);
    let eo = onset_positions(&early, sr, thresh, refractory);
    let eio = inter_onset_intervals(&eo);

    let mut late_g = build_offline(code, sr).expect("compile late");
    late_g.set_cycle_position(late_cycle);
    let late = late_g.render(n);
    let lo = onset_positions(&late, sr, thresh, refractory);
    let lio = inter_onset_intervals(&lo);

    OnsetDriftResult {
        early_onsets: eo.len(),
        late_onsets: lo.len(),
        early_median_ioi: median_i64(&eio),
        late_median_ioi: median_i64(&lio),
        early_max_deviation: max_ioi_deviation(&eio, expected_grid),
        late_max_deviation: max_ioi_deviation(&lio, expected_grid),
        early_iois: eio,
        late_iois: lio,
    }
}

/// Result of the noise-determinism probe.
#[derive(Clone, Debug)]
pub struct NoiseDetResult {
    pub bit_identical: bool,
    pub max_abs_diff: f32,
    pub rms: f32,
    pub finite: bool,
}

/// Render the same noise program on two independently-compiled graphs and
/// compare. A per-node *seeded* PRNG (the fixed state) makes them bit-identical;
/// a per-sample `thread_rng()` (the reverted `wave3-noise-rng-hotpath` state)
/// makes them diverge. Also confirms the output is finite and audible.
pub fn noise_determinism_probe(code: &str, samples: usize, sr: f32) -> NoiseDetResult {
    let mut a = build_offline(code, sr).expect("compile a");
    let mut b = build_offline(code, sr).expect("compile b");
    let ba = a.render(samples);
    let bb = b.render(samples);
    let bit_identical =
        ba.len() == bb.len() && ba.iter().zip(&bb).all(|(x, y)| x.to_bits() == y.to_bits());
    let max_abs_diff = ba.iter().zip(&bb).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
    let finite = ba.iter().all(|x| x.is_finite());
    NoiseDetResult { bit_identical, max_abs_diff, rms: sh::rms(&ba), finite }
}

// ===========================================================================
// CLI
// ===========================================================================

fn parse_arg<T: std::str::FromStr>(args: &[String], flag: &str) -> Option<T> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = parse_arg(&args, "--seed").unwrap_or(0x50AC_0000);
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    let scenario = parse_arg::<String>(&args, "--scenario").unwrap_or_else(|| "all".to_string());

    let mut cfg = if let Some(h) = parse_arg::<f64>(&args, "--hours") {
        SoakConfig::hours(seed, h)
    } else if let Some(c) = parse_arg::<f64>(&args, "--cycles") {
        SoakConfig::cycles(seed, c)
    } else if let Some(s) = parse_arg::<f64>(&args, "--seconds") {
        let mut c = SoakConfig::ci(seed);
        c.cps = 1.0;
        c.target_cycles = s;
        c
    } else {
        // Default: a modest ~5-minute run so a bare invocation returns promptly.
        // For a genuine multi-hour soak pass `--hours N` (build `--release` — a
        // debug build renders near 1x real time, so hours of audio take hours).
        SoakConfig::cycles(seed, 300.0)
    };
    cfg.verbose = verbose;
    cfg.noise_only = scenario == "noise";

    let thr = cfg.thresholds.clone();

    println!(
        "soak_endurance: seed={} target_cycles={:.0} cps={} swap_every={:.0}c scenario={} block={}x{}",
        cfg.seed, cfg.target_cycles, cfg.cps, cfg.swap_every_cycles, scenario, cfg.block_frames, cfg.channels
    );

    let mut hard_fail = false;

    // Main endurance run.
    let report = run_soak(&cfg);
    println!("SOAK      {}", report.summary(&thr));
    if !report.is_clean(&thr) {
        hard_fail = true;
    }

    // T2 onset-drift probe (late = the run's final cycle, at least 1e6 to stress f64).
    let late_cycle = report.final_cycle.max(1_000_000.0);
    let od = onset_drift_probe(ONSET_PROBE_CODE, ONSET_PROBE_GRID, late_cycle, 2.0, cfg.sample_rate);
    let od_ok = od.late_onsets >= 3
        && od.late_max_deviation <= 512
        && (od.late_median_ioi - od.early_median_ioi).abs() <= 256;
    println!(
        "ONSET-T2  late_cycle={:.0} early[n={} med_ioi={} dev={}] late[n={} med_ioi={} dev={}] => {}",
        late_cycle, od.early_onsets, od.early_median_ioi, od.early_max_deviation,
        od.late_onsets, od.late_median_ioi, od.late_max_deviation, if od_ok { "OK" } else { "DRIFT" }
    );
    if !od_ok {
        hard_fail = true;
    }

    // Noise-determinism probe (wave3-noise-rng-hotpath regression).
    let nd = noise_determinism_probe(NOISE_DET_CODE, cfg.sample_rate as usize, cfg.sample_rate);
    let nd_ok = nd.bit_identical && nd.finite && nd.rms > 0.01;
    println!(
        "NOISE     bit_identical={} max_diff={:.2e} rms={:.4} finite={} => {}",
        nd.bit_identical, nd.max_abs_diff, nd.rms, nd.finite, if nd_ok { "OK" } else { "NONDETERMINISTIC" }
    );
    if !nd_ok {
        hard_fail = true;
    }

    if hard_fail {
        eprintln!("soak_endurance: FAIL");
        std::process::exit(1);
    }
    println!("soak_endurance: PASS");
}
