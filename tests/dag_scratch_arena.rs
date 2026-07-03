//! WAVE-2 / rt F-5 / pt-F9 — DAG scratch-arena stress harness.
//!
//! `process_buffer_dag` used to pay, on EVERY buffer:
//!   * two+ `env::var` lookups (`DEBUG_DAG`, `DEBUG_VOICE_BUFFERS`, …) — each a
//!     glibc global-lock + linear `environ` scan;
//!   * a full rebuild of the DAG plan (`build_dag_dependencies` +
//!     `topological_order` + `parallel_batches`, `O(N^2)` for big patches);
//!   * a fresh `HashMap` + one `vec![0.0; buffer_size]` per node (+ a per-node
//!     `.clone()`), i.e. per-buffer heap allocation on the render path.
//!
//! The fix caches the env flags once at graph build, compiles the DAG plan once
//! (reused until the structure changes), and pools per-node scratch buffers keyed
//! through a reusable arena. Three process-global counters make the invariants
//! mechanically checkable:
//!   * [`ENV_FLAG_READS`]   — every module `env::var` routes through it;
//!   * [`DAG_PLAN_BUILDS`]  — bumped once per plan (re)build;
//!   * [`DAG_SCRATCH_ALLOCS`] — bumped once per freshly heap-allocated node buffer.
//!
//! Steady-state render must leave all three FLAT (zero per-buffer env reads, zero
//! plan rebuilds, zero scratch allocations). The pre-fix cost is reproduced by
//! flipping `set_dag_scratch_reuse(false)`, which rebuilds the plan and allocates
//! fresh buffers every buffer — and the optimised path is proven bit-for-bit
//! identical to that pre-fix path on a healthy program.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::{
    OutputMixMode, UnifiedSignalGraph, DAG_PLAN_BUILDS, DAG_SCRATCH_ALLOCS, ENV_FLAG_READS,
};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

const SR: f32 = 44100.0;
const CHUNK: usize = 512;

/// The counters are process-global atomics and integration tests in one file run
/// on multiple threads in the SAME process, so every test here serialises on this
/// lock to keep counter deltas uncontaminated.
static COUNTER_LOCK: Mutex<()> = Mutex::new(());

/// A deliberately non-trivial patch: an LFO modulating a pattern-swept filter,
/// plus two more filtered oscillator buses summed at the output. Pure synthesis
/// (no samples), so the render path stays in `process_buffer_dag`'s batch loop
/// and does not spin up voice/synthesis-voice allocation that the arena does not
/// (yet) pool.
const COMPLEX_PROGRAM: &str = r#"
~lfo # sine 0.5
~bass $ saw "55 110 82.5 55" # lpf (~lfo * 1500 + 800) 0.8
~lead $ square "220 330 440 330" # lpf 2500 0.6
~drone $ sine 55
out $ ~bass * 0.3 + ~lead * 0.15 + ~drone * 0.2
"#;

/// A self-referential (z^-1 feedback) patch. Plan caching fixes ONE particular
/// valid topological order; feedback correctness must be invariant to that choice,
/// so this is the important bit-for-bit case for the DAG plan.
const FEEDBACK_PROGRAM: &str = r#"
~fb $ sine 220 * 0.3 + ~fb * 0.4
out $ ~fb * 0.5
"#;

fn compile(code: &str) -> UnifiedSignalGraph {
    let (_, statements) = parse_program(code).expect("parse");
    let mut graph = compile_program(statements, SR, None).expect("compile");
    graph.set_output_mix_mode(OutputMixMode::None);
    graph.preload_samples();
    graph
}

/// Render `n` stereo buffers offline (sample-based timing → deterministic),
/// returning the mono left channel concatenated.
fn render_n(graph: &mut UnifiedSignalGraph, n: usize) -> Vec<f32> {
    let mut buf = vec![0.0f32; CHUNK * 2];
    let mut out = Vec::with_capacity(n * CHUNK);
    for _ in 0..n {
        buf.iter_mut().for_each(|s| *s = 0.0);
        graph.process_buffer(&mut buf);
        for i in 0..CHUNK {
            out.push(buf[i * 2]);
        }
    }
    out
}

fn rms(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    (signal.iter().map(|x| x * x).sum::<f32>() / signal.len() as f32).sqrt()
}

/// FIX invariant: the steady-state render path performs ZERO per-buffer `env::var`
/// lookups. (Pre-fix, `process_buffer_dag` read `DEBUG_DAG`/`DEBUG_VOICE_BUFFERS`
/// etc. every buffer, so this delta would be > 0.)
#[test]
fn test_render_path_has_zero_per_buffer_env_var() {
    let _g = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut graph = compile(COMPLEX_PROGRAM);

    // Warm up (first buffers do one-time init).
    let _ = render_n(&mut graph, 20);

    let before = ENV_FLAG_READS.load(Ordering::Relaxed);
    let audio = render_n(&mut graph, 40);
    let after = ENV_FLAG_READS.load(Ordering::Relaxed);

    assert!(rms(&audio) > 1e-4, "test patch must actually produce audio");
    assert_eq!(
        after - before,
        0,
        "render path must perform ZERO per-buffer env::var reads (delta over 40 buffers)"
    );
}

/// FIX invariant: after warm-up the render path rebuilds NO DAG plan and
/// allocates NO fresh node scratch buffer per buffer.
#[test]
fn test_render_path_no_per_buffer_plan_rebuild_or_alloc() {
    let _g = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut graph = compile(COMPLEX_PROGRAM);

    // Warm up generously so the plan is compiled and the scratch pool is full.
    let _ = render_n(&mut graph, 40);

    let plan_before = DAG_PLAN_BUILDS.load(Ordering::Relaxed);
    let alloc_before = DAG_SCRATCH_ALLOCS.load(Ordering::Relaxed);
    let _ = render_n(&mut graph, 60);
    let plan_after = DAG_PLAN_BUILDS.load(Ordering::Relaxed);
    let alloc_after = DAG_SCRATCH_ALLOCS.load(Ordering::Relaxed);

    assert_eq!(
        plan_after - plan_before,
        0,
        "DAG plan must be compiled once, not rebuilt per buffer"
    );
    assert_eq!(
        alloc_after - alloc_before,
        0,
        "render path must not heap-allocate node scratch per buffer (pool must be reused)"
    );
}

/// PRE-FIX cost reproduction: with scratch reuse disabled, `process_buffer_dag`
/// rebuilds the plan and allocates fresh node buffers every buffer — exactly the
/// cost the fix removes. This is the "failing" side: asserting the FIX invariant
/// (delta == 0) against this path would FAIL.
#[test]
fn test_pre_fix_path_rebuilds_plan_and_allocs_every_buffer() {
    let _g = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut graph = compile(COMPLEX_PROGRAM);
    graph.set_dag_scratch_reuse(false);

    let _ = render_n(&mut graph, 5); // warm-up cannot help the pre-fix path

    let plan_before = DAG_PLAN_BUILDS.load(Ordering::Relaxed);
    let alloc_before = DAG_SCRATCH_ALLOCS.load(Ordering::Relaxed);
    const N: u64 = 30;
    let _ = render_n(&mut graph, N as usize);
    let plan_delta = DAG_PLAN_BUILDS.load(Ordering::Relaxed) - plan_before;
    let alloc_delta = DAG_SCRATCH_ALLOCS.load(Ordering::Relaxed) - alloc_before;

    assert_eq!(
        plan_delta, N,
        "pre-fix path rebuilds the DAG plan on every buffer"
    );
    assert!(
        alloc_delta >= N,
        "pre-fix path allocates fresh node scratch every buffer (got {alloc_delta} over {N})"
    );
}

/// BIT-FOR-BIT: the optimised (reuse on) and pre-fix (reuse off) render paths must
/// produce byte-identical audio for a healthy program. This is the non-negotiable
/// "preserve output bit-for-bit" constraint. Checked for both a plain synthesis
/// patch and a self-referential (z^-1 feedback) patch.
#[test]
fn test_scratch_reuse_is_bit_for_bit_identical() {
    let _g = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    for program in [COMPLEX_PROGRAM, FEEDBACK_PROGRAM] {
        let mut graph_reuse = compile(program);
        graph_reuse.set_dag_scratch_reuse(true);
        let audio_reuse = render_n(&mut graph_reuse, 200);

        let mut graph_fresh = compile(program);
        graph_fresh.set_dag_scratch_reuse(false);
        let audio_fresh = render_n(&mut graph_fresh, 200);

        assert_eq!(audio_reuse.len(), audio_fresh.len());
        assert!(rms(&audio_reuse) > 1e-4, "patch must produce audio");

        for (i, (a, b)) in audio_reuse.iter().zip(audio_fresh.iter()).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "sample {i} differs: reuse={a} fresh={b} — scratch arena changed output"
            );
        }
    }
}
