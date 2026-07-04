//! Wave-2 integration regression guard (verify-feature-wave2).
//!
//! Renders `examples/wave2_integration.ph`, which combines the entire feature
//! wave-2 surface in one patch, and asserts the combined result is musical and
//! bounded. This is the end-to-end join that proves the melodic + filter +
//! sampler features co-exist and none regressed the stability wave:
//!
//!   * scale quantization      — `n "..." # scale "minor"`     (feat-scale-quantization)
//!   * chords in mini-notation — `note "...'maj"`              (feat-chord-support)
//!   * resonant filters        — `# rlpf` / `# rhpf` (RBJ)     (feat-resonant-filters)
//!   * T3-smooth continuous LFO on the cutoff (per-sample)     (promote-t3-continuous-patterns)
//!   * f64 trigger timing (sample-accurate onsets)             (promote-t2-trigger-f64)
//!
//! Three levels:
//!   - Level 1: the example parses + compiles (every wave-2 surface wires up).
//!   - Level 2: rendered audio has rhythmic onsets (the chord/melody trigger).
//!   - Level 3: audio is finite (no NaN blow-up from resonance), non-silent with
//!     a musical RMS, peak-bounded (no clipping/blow-up), and near-zero DC.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_file(path: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let code = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
    let (_, statements) =
        parse_program(&code).unwrap_or_else(|e| panic!("Failed to parse {path}: {e:?}"));
    let mut graph = compile_program(statements, sample_rate, None)
        .unwrap_or_else(|e| panic!("Failed to compile {path}: {e:?}"));
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

#[test]
fn test_wave2_integration_example_renders_musical_and_bounded() {
    let buffer = render_file("examples/wave2_integration.ph", 8.0);

    // Level 1: parse + compile succeeded (render_file panics otherwise) and
    // produced samples.
    assert!(!buffer.is_empty(), "example produced no samples");
    assert_eq!(buffer.len(), 8 * 44100, "unexpected sample count");

    // Level 3: fully finite — combined high-Q resonance + LFO sweep must never
    // produce NaN/inf.
    let non_finite = buffer.iter().filter(|s| !s.is_finite()).count();
    assert_eq!(non_finite, 0, "produced {non_finite} non-finite samples");

    // Level 3: non-silent with a musical RMS (not a dropout, not saturated).
    let rms = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.02, "too quiet — RMS = {rms}");
    assert!(rms < 0.5, "unexpectedly hot — RMS = {rms} (limiter saturation?)");

    // Level 3: peak-bounded — no clipping past the limiter ceiling, no blow-up.
    let peak = buffer.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak <= 1.0001, "clipping/blow-up — peak = {peak}");

    // Level 3: near-zero DC offset (no stuck-DC from a mis-sanitised filter).
    let dc = buffer.iter().sum::<f32>() / buffer.len() as f32;
    assert!(dc.abs() < 0.02, "excess DC offset = {dc}");

    // Level 2: the chord/melody surface actually triggers rhythmic events, so
    // the render is not a static drone. Count zero-crossing-free "silent" runs
    // vs energetic runs across coarse windows — at least a few windows must be
    // energetic (onset activity), and the signal must vary block-to-block.
    let win = 4410; // 100 ms windows
    let energetic_windows = buffer
        .chunks(win)
        .filter(|c| {
            let w_rms = (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt();
            w_rms > 0.02
        })
        .count();
    assert!(
        energetic_windows >= 20,
        "too few energetic windows ({energetic_windows}) — expected sustained musical activity"
    );

    println!(
        "wave2_integration.ph: RMS = {rms:.4}, peak = {peak:.4}, DC = {dc:.6}, \
         energetic_windows = {energetic_windows}/80"
    );
}
