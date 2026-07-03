//! Regression tests for parallel-render voice continuity across seek/chunk boundaries.
//!
//! Background (task fix-parallel-render):
//! The default CLI render path (`phonon render`, realtime=true parallel=true)
//! splits the timeline into 512-sample blocks, clones the graph per rayon
//! thread, and calls `seek_to_sample()` before each block. Two bugs truncated
//! sample voices after the first hit:
//!   1. Thread-chunk boundary: a voice triggered near the end of one thread's
//!      block range only played to that range's end; the next thread's fresh
//!      clone never triggered it, so its tail rendered silence.
//!   2. Cross-cycle hit: a hit at cycle position 0.0 of cycle N>0 got a
//!      near-instant auto-release (delta collapsed to ~0.001 cycles at the
//!      integer boundary), truncating the voice to a ~2ms blip.
//!
//! These tests replicate the CLI parallel render path using the public graph
//! API (clone-per-chunk + warmup + per-block seek + process_buffer) and assert
//! that every hit renders its full sample body, matching the clean sequential
//! `graph.render()` reference.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::UnifiedSignalGraph;

const BLOCK_SIZE: usize = 512;

/// Replicate the CLI parallel render: split blocks into per-thread chunks, clone
/// the graph per chunk, warm up each clone by rendering the blocks preceding its
/// range (discarded), then render its assigned range. Runs the chunks
/// sequentially — the truncation bug comes from the clone-per-chunk structure,
/// not from concurrency, so this is a faithful, deterministic reproduction.
fn render_parallel_style(
    graph: &mut UnifiedSignalGraph,
    total_samples: usize,
    num_threads: usize,
) -> Vec<f32> {
    let num_blocks = total_samples.div_ceil(BLOCK_SIZE);
    let blocks_per_thread = num_blocks.div_ceil(num_threads);

    let warmup_samples = graph.compute_parallel_warmup_samples(total_samples);
    let warmup_blocks = warmup_samples.div_ceil(BLOCK_SIZE);

    let mut output = vec![0.0f32; total_samples];

    for thread_idx in 0..num_threads {
        let start_block = thread_idx * blocks_per_thread;
        let end_block = ((thread_idx + 1) * blocks_per_thread).min(num_blocks);
        if start_block >= end_block {
            continue;
        }

        let mut my_graph = graph.clone();

        // Warmup: render preceding blocks so still-sounding voices are active.
        let warmup_start = start_block.saturating_sub(warmup_blocks);
        for wb in warmup_start..start_block {
            my_graph.seek_to_sample(wb * BLOCK_SIZE);
            let mut warm = vec![0.0f32; BLOCK_SIZE * 2];
            my_graph.process_buffer(&mut warm);
        }

        for block_idx in start_block..end_block {
            let block_start = block_idx * BLOCK_SIZE;
            let block_samples = (total_samples - block_start).min(BLOCK_SIZE);
            my_graph.seek_to_sample(block_start);
            let mut stereo = vec![0.0f32; block_samples * 2];
            my_graph.process_buffer(&mut stereo);
            for i in 0..block_samples {
                output[block_start + i] = stereo[i * 2]; // left channel (mono)
            }
        }
    }

    output
}

/// Measure the nonzero span of a hit starting at (or just after) `pos`, searching
/// forward up to `window` samples. Returns (onset, span) where span is the number
/// of samples from the first to the last nonzero sample.
fn hit_span(audio: &[f32], pos: usize, window: usize, thr: f32) -> (usize, usize) {
    let mut onset = None;
    let mut last = None;
    let end = (pos + window).min(audio.len());
    for i in pos..end {
        if audio[i].abs() > thr {
            if onset.is_none() {
                onset = Some(i);
            }
            last = Some(i);
        }
    }
    match (onset, last) {
        (Some(o), Some(l)) => (o, l - o + 1),
        _ => (0, 0),
    }
}

fn compile(code: &str, sample_rate: f32) -> UnifiedSignalGraph {
    let (_rest, stmts) = parse_program(code).expect("parse");
    compile_program(stmts, sample_rate, None).expect("compile")
}

/// A single sample per cycle over 4 cycles. Every hit lands at cycle position
/// 0.0 of its cycle — the exact case bug #2 (cross-cycle auto-release collapse)
/// truncated to a ~128-sample blip, and bug #1 (chunk boundary) truncated at
/// each thread's chunk end. Assert every hit renders its full sample body.
#[test]
fn test_parallel_render_no_voice_truncation_multicycle() {
    let sample_rate = 44100.0;
    // tempo 2.0 -> cps=2 -> 1 cycle = 0.5s = 22050 samples. 2s render = 4 cycles.
    let code = "tempo: 2.0\n\nout $ s \"bd\"\n";
    let total_samples = 88_200; // 2 seconds

    // Clean reference: single whole-buffer render does not seek/clone, so it does
    // not truncate. This is the gold standard the parallel path must match.
    let mut ref_graph = compile(code, sample_rate);
    let reference = ref_graph.render(total_samples);

    // The full sample body length, measured from the (untruncated) first hit.
    let (_, full_span) = hit_span(&reference, 0, 22050, 0.005);
    assert!(
        full_span > 3000,
        "sanity: bd sample body should be a few thousand samples, got {full_span}"
    );

    // Render via the parallel/chunked path with multiple threads to force chunk
    // boundaries to fall between hits.
    let mut par_graph = compile(code, sample_rate);
    let parallel = render_parallel_style(&mut par_graph, total_samples, 16);

    let period = 22_050usize;
    for cycle in 0..4 {
        let pos = cycle * period;
        let (onset, span) = hit_span(&parallel, pos, period, 0.005);
        assert!(
            span as f32 >= full_span as f32 * 0.9,
            "cycle {cycle} hit @ {pos} truncated: span={span} (onset={onset}), \
             expected ~{full_span} (>=90%). Voice did not survive the seek/chunk boundary."
        );
    }
}

/// The parallel/chunked render must match the clean sequential reference within
/// a tight tolerance (identical modulo tiny float ordering differences).
#[test]
fn test_parallel_render_matches_sequential() {
    let sample_rate = 44100.0;
    let code = "tempo: 2.0\n\nout $ s \"bd\"\n";
    let total_samples = 88_200;

    let mut ref_graph = compile(code, sample_rate);
    let reference = ref_graph.render(total_samples);

    let mut par_graph = compile(code, sample_rate);
    let parallel = render_parallel_style(&mut par_graph, total_samples, 16);

    assert_eq!(reference.len(), parallel.len());
    let mut max_diff = 0.0f32;
    for (a, b) in reference.iter().zip(parallel.iter()) {
        max_diff = max_diff.max((a - b).abs());
    }
    assert!(
        max_diff < 1e-3,
        "parallel output diverges from sequential reference: max_diff={max_diff}"
    );
}

/// Dense 4-hits-per-cycle pattern rendered in parallel: every hit (including the
/// ones straddled by a chunk boundary) must render its full body.
#[test]
fn test_parallel_render_dense_pattern_full_hits() {
    let sample_rate = 44100.0;
    // tempo 0.5 -> cps=0.5 -> 1 cycle = 2s = 88200 samples. bd*4 -> hits every 0.5s.
    let code = "tempo: 0.5\n\nout $ s \"bd*4\"\n";
    let total_samples = 88_200; // exactly one cycle, 4 hits

    let mut ref_graph = compile(code, sample_rate);
    let reference = ref_graph.render(total_samples);
    let (_, full_span) = hit_span(&reference, 0, 22050, 0.005);

    let mut par_graph = compile(code, sample_rate);
    let parallel = render_parallel_style(&mut par_graph, total_samples, 16);

    for (i, pos) in [0usize, 22_050, 44_100, 66_150].iter().enumerate() {
        let (onset, span) = hit_span(&parallel, *pos, 22_050, 0.005);
        assert!(
            span as f32 >= full_span as f32 * 0.9,
            "hit {i} @ {pos} truncated at chunk boundary: span={span} (onset={onset}), \
             expected ~{full_span}"
        );
    }
}
