//! Regression tests for G6 / pt-F6: inline `Signal::Pattern` must NOT be re-parsed
//! from mini-notation on every sample of the audio thread.
//!
//! Before the fix, `eval_signal_at_time` (and its note/chord siblings) called
//! `parse_mini_notation(pattern_str)` on *every* sample for any inline
//! `Signal::Pattern`, i.e. ~44_100 parses + allocations per second per such signal.
//! This is a real-time-safety violation (heap allocation on the synth thread) and a
//! large CPU cost. `SignalNode::Pattern` already avoids this via its event cache; the
//! inline branch did not.
//!
//! The fix memoizes the parsed `Pattern<String>` keyed by the pattern string, so the
//! parse happens at most once per distinct inline pattern regardless of how many
//! samples are rendered — while preserving identical query results / onset timing.

use phonon::mini_notation_v3::{mini_notation_parse_count, reset_mini_notation_parse_count};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::cell::RefCell;

/// Build a graph whose oscillator frequency is controlled by an INLINE `Signal::Pattern`
/// (not a `SignalNode::Pattern` node). This is exactly the shape that used to trigger a
/// per-sample re-parse.
fn build_inline_pattern_graph(pattern: &str) -> UnifiedSignalGraph {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Pattern(pattern.to_string()),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });
    graph.set_output(osc);
    graph
}

/// Core regression: rendering thousands of samples of an inline `Signal::Pattern`
/// control must NOT scale the mini-notation parse count with the sample count.
#[test]
fn test_inline_pattern_not_reparsed_per_sample() {
    let num_samples = 8820; // 0.2s @ 44.1kHz — spans multiple cycles at cps=2.0
    let mut graph = build_inline_pattern_graph("110 220 440 330");

    // Warm up one buffer first so any legitimate one-time setup parses are excluded,
    // then measure the *steady-state* parse count across a long render.
    let _ = graph.render(512);
    reset_mini_notation_parse_count();

    let buffer = graph.render(num_samples);

    let parses = mini_notation_parse_count();

    // Sanity: the render actually produced audio (the pattern drove the oscillator).
    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "inline-pattern-controlled oscillator produced silence (rms={rms})");

    // The heart of the regression. Pre-fix this was ~num_samples (thousands). Post-fix
    // the parsed pattern is cached, so steady-state parses stay at (a small constant) 0.
    // Allow a tiny slack for any incidental setup, but it MUST be far below the sample count.
    assert!(
        parses <= 2,
        "inline Signal::Pattern was re-parsed {parses} times while rendering {num_samples} \
         samples — expected the parsed pattern to be memoized (<=2 parses)"
    );
}

/// Onset-timing / value preservation: memoization must not change the sequence of values
/// the pattern produces. We compare the frequency the oscillator "sees" across a cycle by
/// detecting when the rendered sine's instantaneous behaviour changes — but simpler and
/// exact: render two graphs (fresh each time) and confirm identical output buffers, proving
/// the cached-pattern path yields bit-identical results to a single render.
#[test]
fn test_inline_pattern_query_results_unchanged() {
    // Two independent graphs with the same inline pattern must render identically.
    let mut a = build_inline_pattern_graph("110 220 440 330");
    let mut b = build_inline_pattern_graph("110 220 440 330");

    let buf_a = a.render(22050); // one full cycle at cps=2.0
    let buf_b = b.render(22050);

    assert_eq!(buf_a.len(), buf_b.len());
    for (i, (x, y)) in buf_a.iter().zip(buf_b.iter()).enumerate() {
        assert_eq!(
            x, y,
            "inline-pattern render diverged at sample {i}: {x} != {y}"
        );
    }

    // And the output must be non-trivial (the four distinct freqs actually play).
    let rms = (buf_a.iter().map(|x| x * x).sum::<f32>() / buf_a.len() as f32).sqrt();
    assert!(rms > 0.01, "expected audible output, got rms={rms}");
}

/// Level-1 style check that the *values* the inline pattern yields still transition at the
/// correct cycle boundaries. At cps=2.0 a 4-step pattern over one cycle (22050 samples)
/// changes value every 22050/4 = 5512 samples. We assert the oscillator frequency (recovered
/// via zero-crossing rate over each quarter) increases then decreases in the 110/220/440/330
/// shape, proving onset semantics are preserved by the caching change.
#[test]
fn test_inline_pattern_onset_transitions_preserved() {
    let mut graph = build_inline_pattern_graph("110 220 440 330");
    let buffer = graph.render(22050);

    // Count zero crossings per quarter-cycle as a proxy for frequency.
    let quarter = 22050 / 4;
    let mut zc = [0usize; 4];
    for q in 0..4 {
        let start = q * quarter;
        let end = start + quarter;
        for i in start + 1..end {
            if (buffer[i - 1] <= 0.0 && buffer[i] > 0.0)
                || (buffer[i - 1] >= 0.0 && buffer[i] < 0.0)
            {
                zc[q] += 1;
            }
        }
    }

    // Expected relative ordering: 110 < 220 < 440 > 330, and 330 > 110.
    assert!(zc[0] < zc[1], "expected freq step 0(110) < 1(220): {zc:?}");
    assert!(zc[1] < zc[2], "expected freq step 1(220) < 2(440): {zc:?}");
    assert!(zc[2] > zc[3], "expected freq step 2(440) > 3(330): {zc:?}");
    assert!(zc[3] > zc[0], "expected freq step 3(330) > 0(110): {zc:?}");
}
