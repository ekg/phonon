//! T3 — Continuous signal patterns evaluated per-sample (kill LFO zipper).
//!
//! Regression coverage for audit finding pt-F5 / T3
//! (`docs/audits/pattern-timing-2026-07.md` §7): continuous "signal" patterns
//! (`Pattern::sine_wave/saw_wave/tri_wave/...`) return a single hap spanning the
//! query, valued at `span.begin`. When such a pattern reaches
//! `SignalNode::Pattern`, the per-buffer `pattern_event_cache` froze that one
//! value for the whole 512-sample buffer — a ~86 Hz stairstep at 44.1 kHz
//! (audible zipper noise on modulated params). This directly contradicts
//! Phonon's headline ("patterns ARE control signals, evaluated at sample rate").
//!
//! The fix evaluates continuous patterns per-sample while keeping the fast
//! cached path for discrete step patterns.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::pattern::Pattern;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph};

const SR: f32 = 44100.0;
const BUF: usize = 512;

/// Build a `SignalNode::Pattern` holding a continuous `sine_wave` (stringified,
/// as the runtime stores control patterns as `Pattern<String>`).
fn continuous_sine_pattern_node(g: &mut UnifiedSignalGraph) -> phonon::unified_graph::NodeId {
    let pat = Pattern::<f64>::sine_wave().fmap(|v| format!("{}", v));
    g.add_node(SignalNode::Pattern {
        pattern_str: "sine_wave".to_string(),
        pattern: pat,
        last_value: 0.0,
        last_trigger_time: 0.0,
    })
}

/// Render `chunks` fixed-size 512-sample buffers, like real-time playback (mono).
fn render_chunks(g: &mut UnifiedSignalGraph, chunks: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(chunks * BUF);
    for _ in 0..chunks {
        let mut buf = vec![0.0f32; BUF * 2];
        g.process_buffer(&mut buf);
        for i in 0..BUF {
            out.push(buf[i * 2]);
        }
    }
    out
}

/// Largest absolute deviation from the first sample within a single 512-buffer.
fn max_within_buffer_dev(samples: &[f32], chunk_idx: usize) -> f32 {
    let seg = &samples[chunk_idx * BUF..(chunk_idx + 1) * BUF];
    let first = seg[0];
    seg.iter()
        .map(|&x| (x - first).abs())
        .fold(0.0f32, f32::max)
}

/// Count of distinct consecutive values within a single 512-buffer.
fn distinct_steps(samples: &[f32], chunk_idx: usize) -> usize {
    let seg = &samples[chunk_idx * BUF..(chunk_idx + 1) * BUF];
    let mut n = 1usize;
    let mut prev = seg[0];
    for &x in &seg[1..] {
        if (x - prev).abs() > 1e-9 {
            n += 1;
            prev = x;
        }
    }
    n
}

// ---------------------------------------------------------------------------
// Level 1 — pattern-query granularity
// ---------------------------------------------------------------------------

/// A continuous signal pattern queried at sample granularity must return
/// distinct values within one buffer — NOT one frozen constant.
#[test]
fn level1_continuous_pattern_varies_within_buffer() {
    let mut g = UnifiedSignalGraph::new(SR);
    let n = continuous_sine_pattern_node(&mut g);
    g.set_output(n);

    let samples = render_chunks(&mut g, 4);

    for c in 0..3 {
        let dev = max_within_buffer_dev(&samples, c);
        let steps = distinct_steps(&samples, c);
        // Pre-fix: dev == 0.0 and steps == 1 (frozen to buffer-start value).
        assert!(
            dev > 0.02,
            "buffer {c}: continuous pattern frozen within buffer (max_dev={dev}); \
             expected smooth per-sample variation"
        );
        assert!(
            steps > 400,
            "buffer {c}: only {steps} distinct values in 512 samples — \
             continuous pattern is a stairstep, not sample-rate smooth"
        );
    }
}

/// Regression guard: `PatternEvaluator` (sine/cosine/saw/tri no-arg) was already
/// per-sample; keep it that way.
#[test]
fn level1_pattern_evaluator_stays_smooth() {
    let mut g = UnifiedSignalGraph::new(SR);
    let n = g.add_node(SignalNode::PatternEvaluator {
        pattern: Pattern::<f64>::sine_wave(),
    });
    g.set_output(n);

    let samples = render_chunks(&mut g, 4);
    for c in 0..3 {
        assert!(
            distinct_steps(&samples, c) > 400,
            "buffer {c}: PatternEvaluator regressed to a stairstep"
        );
    }
}

/// Regression guard: DISCRETE step patterns keep step semantics — a step value
/// is held (constant) between event boundaries, not turned into a ramp (that is
/// the separate, deferred T6 "slew" item). Within a buffer wholly inside one
/// step the value must stay constant.
#[test]
fn level1_discrete_step_pattern_preserved() {
    let mut g = UnifiedSignalGraph::new(SR);
    // cps 0.5 -> one cycle = 2 s = 88200 samples; 4 steps/cycle => each step is
    // 22050 samples wide, far larger than a 512 buffer, so the first buffer sits
    // entirely inside step 0.
    // Small values so the master output limiter (~0.95 ceiling) does not clamp
    // and hide the step value.
    let pat = phonon::mini_notation_v3::parse_mini_notation("0.1 0.2 0.3 0.4");
    let n = g.add_node(SignalNode::Pattern {
        pattern_str: "0.1 0.2 0.3 0.4".to_string(),
        pattern: pat,
        last_value: 0.0,
        last_trigger_time: 0.0,
    });
    g.set_output(n);

    let samples = render_chunks(&mut g, 1);
    assert!(
        (samples[0] - 0.1).abs() < 1e-3,
        "first step should read ~0.1, got {}",
        samples[0]
    );
    assert_eq!(
        max_within_buffer_dev(&samples, 0),
        0.0,
        "discrete step must be held constant within its slot (no accidental slew)"
    );
}

// ---------------------------------------------------------------------------
// Level 2 — filter-cutoff modulation: no ~86 Hz stairstep
// ---------------------------------------------------------------------------

/// Build a saw whose LPF cutoff is driven by a continuous `sine_wave` pattern
/// bus, and verify the cutoff CONTROL varies smoothly within every buffer
/// (i.e. the modulation is not a per-buffer stairstep).
#[test]
fn level2_continuous_pattern_filter_cutoff_no_stairstep() {
    use phonon::unified_graph::Waveform;

    // Control-only graph: route the (scaled) continuous LFO straight to output so
    // we can inspect the exact signal that drives the filter cutoff.
    let mut ctrl = UnifiedSignalGraph::new(SR);
    let lfo = continuous_sine_pattern_node(&mut ctrl);
    let scaled = ctrl.add_multiply_node(Signal::Node(lfo), Signal::Value(0.5));
    ctrl.set_output(scaled);
    let control = render_chunks(&mut ctrl, 8);

    // Every buffer must show smooth within-buffer motion (no freeze).
    for c in 0..8 {
        assert!(
            max_within_buffer_dev(&control, c) > 0.01,
            "buffer {c}: filter-cutoff control frozen within buffer (stairstep)"
        );
    }

    // Stairstep signature check: with a freeze, ALL motion happens at the 512
    // block boundaries and none inside. Compare total within-buffer motion to
    // total boundary jumps. Smooth modulation => within-buffer motion dominates.
    let mut within_motion = 0.0f32;
    let mut boundary_motion = 0.0f32;
    for i in 1..control.len() {
        let d = (control[i] - control[i - 1]).abs();
        if i % BUF == 0 {
            boundary_motion += d;
        } else {
            within_motion += d;
        }
    }
    assert!(
        within_motion > boundary_motion * 20.0,
        "modulation looks like a per-buffer stairstep: within-buffer motion \
         {within_motion} vs boundary motion {boundary_motion}"
    );

    // Full audio graph: saw 55 through LPF with the same continuous-pattern cutoff.
    let mut g = UnifiedSignalGraph::new(SR);
    let saw = g.add_oscillator(Signal::Value(55.0), Waveform::Saw);
    let lfo = continuous_sine_pattern_node(&mut g);
    let mul = g.add_multiply_node(Signal::Node(lfo), Signal::Value(2000.0));
    let cutoff = g.add_node(SignalNode::Add {
        a: Signal::Node(mul),
        b: Signal::Value(2500.0),
    });
    let lpf = g.add_lowpass_node(Signal::Node(saw), Signal::Node(cutoff), Signal::Value(3.0));
    g.set_output(lpf);

    let audio = render_chunks(&mut g, 8);
    let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.01, "filtered audio should have energy, rms={rms}");
    assert!(audio.iter().all(|x| x.is_finite()), "audio has NaN/Inf");
}

/// Regression guard for the task's literal example. `sine 0.5` compiles to an
/// oscillator LFO (already per-sample); confirm the DSL path stays smooth and
/// healthy after the fix.
#[test]
fn level2_dsl_oscillator_filter_mod_smooth() {
    let code = "out $ saw 55 # lpf (sine 0.5 * 2000 + 500) 0.8";
    let (_, stmts) = parse_program(code).expect("parse");
    let mut g = compile_program(stmts, SR, None).expect("compile");
    let audio = render_chunks(&mut g, 8);

    let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "oscillator-modulated filter should sound, rms={rms}"
    );
    assert!(audio.iter().all(|x| x.is_finite()), "audio has NaN/Inf");
}

// ---------------------------------------------------------------------------
// Level 3 — audio characteristics
// ---------------------------------------------------------------------------

#[test]
fn level3_continuous_pattern_audio_finite_and_energetic() {
    let mut g = UnifiedSignalGraph::new(SR);
    let n = continuous_sine_pattern_node(&mut g);
    // Scale to audible amplitude and route out.
    let scaled = g.add_multiply_node(Signal::Node(n), Signal::Value(0.3));
    g.set_output(scaled);

    let audio = render_chunks(&mut g, 8);
    assert!(audio.iter().all(|x| x.is_finite()), "NaN/Inf in output");
    let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "continuous LFO output should have energy, rms={rms}"
    );
    // Sanity: bounded (sine * 0.3 stays within [-0.3, 0.3]).
    assert!(
        audio.iter().all(|&x| x.abs() <= 0.31),
        "continuous sine LFO exceeded expected bounds"
    );
}
