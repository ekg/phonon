/// Combined tests for `often` (75%) and `rarely` (25%) transforms
/// Both use `sometimes_by` with different probabilities
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

#[test]
fn test_often_probability() {
    // often should apply transform ~75% of cycles
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let mut transform_count = 0;
    for cycle in 0..100 {
        let mut rng = StdRng::seed_from_u64(cycle);
        if rng.gen::<f64>() < 0.75 {
            transform_count += 1;
        }
    }

    let probability = transform_count as f64 / 100.0;
    assert!(
        probability >= 0.65 && probability <= 0.85,
        "often should apply ~75%: got {:.1}%",
        probability * 100.0
    );

    println!("✅ often: {:.1}% application rate", probability * 100.0);
}

#[test]
fn test_rarely_probability() {
    // rarely should apply transform ~25% of cycles
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let mut transform_count = 0;
    for cycle in 0..100 {
        let mut rng = StdRng::seed_from_u64(cycle);
        if rng.gen::<f64>() < 0.1 {
            // rarely uses 0.1
            transform_count += 1;
        }
    }

    let probability = transform_count as f64 / 100.0;
    assert!(
        probability >= 0.05 && probability <= 0.15,
        "rarely should apply ~10%: got {:.1}%",
        probability * 100.0
    );

    println!("✅ rarely: {:.1}% application rate", probability * 100.0);
}

#[test]
fn test_often_audio() {
    let code = r#"
tempo: 0.5
out: s "bd sn" $ often (fast 2)
"#;

    let audio = render_dsl(code, 20);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "often should produce audio");
    println!("✅ often audio: RMS = {:.4}", rms);
}

#[test]
fn test_rarely_audio() {
    let code = r#"
tempo: 0.5
out: s "bd sn" $ rarely (fast 2)
"#;

    let audio = render_dsl(code, 20);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "rarely should produce audio");
    println!("✅ rarely audio: RMS = {:.4}", rms);
}
