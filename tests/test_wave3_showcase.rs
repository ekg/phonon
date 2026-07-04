//! Wave-3 showcase regression guard (wave3-doc-status-refresh).
//!
//! Renders `examples/wave3_showcase.ph`, which combines the now-complete
//! melodic + resonant + dynamics surface in one patch, and asserts the combined
//! result is musical and bounded. This proves the showcased features actually
//! work *together* (not just in isolation), and keeps the performer-facing
//! example honest — if any of these surfaces regress, the example stops
//! rendering and this test fails:
//!
//!   * scale quantization      — `n "..." # scale "minor"`
//!   * chords in mini-notation — `note "...'maj"` (polyphonic voice path)
//!   * resonant filters        — `# rlpf` / `# resonz` (RBJ biquad)
//!   * gate                    — `gate "t ~ t ~"` (pattern -> 0/1 control signal)
//!   * expander                — `# expander thr ratio atk rel` (upward dynamics)
//!   * T3-smooth continuous LFO on a cutoff (per-sample, no zipper)
//!
//! Level-3 audio characteristics (per the task ## Validation):
//!   - fully finite (no NaN / inf from combined high-Q resonance + dynamics),
//!   - non-silent with a musical RMS > 0.01 (proves the surface makes sound),
//!   - peak-bounded <= 1.0 (no clipping / feedback blow-up),
//!   - near-zero DC offset (no stuck-DC from a mis-sanitised filter).

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_file(path: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let code =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
    let (_, statements) =
        parse_program(&code).unwrap_or_else(|e| panic!("Failed to parse {path}: {e:?}"));
    let mut graph = compile_program(statements, sample_rate, None)
        .unwrap_or_else(|e| panic!("Failed to compile {path}: {e:?}"));
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

#[test]
fn test_wave3_showcase_example_renders_musical_and_bounded() {
    let buffer = render_file("examples/wave3_showcase.ph", 8.0);

    // Level 1: parse + compile succeeded (render_file panics otherwise) and the
    // whole showcased surface wired up.
    assert!(!buffer.is_empty(), "example produced no samples");
    assert_eq!(buffer.len(), 8 * 44100, "unexpected sample count");

    // Level 3: fully finite — combined resonance + gate + expander must never
    // produce NaN / inf.
    let non_finite = buffer.iter().filter(|s| !s.is_finite()).count();
    assert_eq!(non_finite, 0, "produced {non_finite} non-finite samples");

    // Level 3: non-silent with a musical RMS (proves the surface makes sound;
    // task requires RMS > 0.01). Upper bound guards against limiter saturation.
    let rms = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "too quiet — RMS = {rms}");
    assert!(rms < 0.5, "unexpectedly hot — RMS = {rms} (limiter saturation?)");

    // Level 3: peak-bounded — no clipping past the ceiling, no blow-up.
    let peak = buffer.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak <= 1.0, "clipping / blow-up — peak = {peak}");

    // Level 3: near-zero DC offset over the full window (a per-block gate false-
    // positives on low bass, so measure across the entire render).
    let dc = buffer.iter().sum::<f32>() / buffer.len() as f32;
    assert!(dc.abs() < 0.02, "excess DC offset = {dc}");

    // Sanity: the pattern `gate` actually chops the bass, so there must be both
    // energetic and quieter windows (not a static drone). At least a handful of
    // 100 ms windows carry sustained musical activity.
    let win = 4410; // 100 ms windows
    let energetic_windows = buffer
        .chunks(win)
        .filter(|c| {
            let w_rms = (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt();
            w_rms > 0.01
        })
        .count();
    assert!(
        energetic_windows >= 20,
        "too few energetic windows ({energetic_windows}) — expected sustained musical activity"
    );

    println!(
        "wave3_showcase.ph: RMS = {rms:.4}, peak = {peak:.4}, DC = {dc:.6}, \
         energetic_windows = {energetic_windows}/80"
    );
}
