/// Regression guard for the resonant-filter musical example.
///
/// `examples/resonant_filter_sweep.ph` exercises all three resonant filters
/// (RLPF / RHPF / Resonz) driven by pattern LFOs on their cutoff / frequency.
/// Nothing else globs `examples/*.ph`, so this test renders the actual file and
/// asserts the combined pattern-modulated sweep stays musical and bounded:
///
/// - Level 1: the example parses + compiles (all three filters wire up).
/// - Level 2/3: rendered audio is non-silent (RMS > 0.01), fully finite
///   (no NaN blow-up from high-Q self-oscillation), and peak-bounded (the
///   limiter/clamp keeps output well under a hard blow-up threshold).
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_file(path: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let code = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
    let (_, statements) = parse_program(&code)
        .unwrap_or_else(|e| panic!("Failed to parse {path}: {e:?}"));
    let mut graph = compile_program(statements, sample_rate, None)
        .unwrap_or_else(|e| panic!("Failed to compile {path}: {e:?}"));
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

#[test]
fn test_resonant_filter_example_renders_bounded_audio() {
    let buffer = render_file("examples/resonant_filter_sweep.ph", 8.0);

    assert!(!buffer.is_empty(), "example produced no samples");

    // Level 3: fully finite — high-Q resonance must not produce NaN/inf.
    let non_finite = buffer.iter().filter(|s| !s.is_finite()).count();
    assert_eq!(non_finite, 0, "example produced {non_finite} non-finite samples");

    // Level 3: non-silent.
    let rms = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(rms > 0.01, "example too quiet, RMS = {rms}");

    // Level 3: bounded — self-oscillation at high Q stays clamped, no blow-up.
    let peak = buffer.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(
        peak < 4.0,
        "example peak {peak} indicates an unbounded blow-up"
    );

    println!("resonant_filter_sweep.ph: RMS = {rms:.4}, peak = {peak:.4}");
}
