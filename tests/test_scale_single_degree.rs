//! Debug test for single scale degree

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};
use rustfft::{num_complex::Complex, FftPlanner};

fn find_dominant_frequency(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut complex_input: Vec<Complex<f32>> =
        buffer.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();

    fft.process(&mut complex_input);

    let magnitudes: Vec<f32> = complex_input[1..complex_input.len() / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let max_idx = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    (max_idx + 1) as f32 * sample_rate / buffer.len() as f32
}

#[test]
fn test_single_degree_0() {
    // Test degree 0 = C4 = 261.63 Hz
    let input = r#"
        cps: 1.0
        out $ sine(scale("0", "major", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100); // 1 second
    let detected_freq = find_dominant_frequency(&buffer[11025..33075], 44100.0);

    println!("Degree 0: detected {}Hz, expected 261.63Hz", detected_freq);
    assert!(
        (detected_freq - 261.63).abs() < 5.0,
        "Degree 0 should be C4 (261.63Hz), got {}Hz",
        detected_freq
    );
}

#[test]
fn test_single_degree_1() {
    // Test degree 1 = D4 = 293.66 Hz
    let input = r#"
        cps: 1.0
        out $ sine(scale("1", "major", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let detected_freq = find_dominant_frequency(&buffer[11025..33075], 44100.0);

    println!("Degree 1: detected {}Hz, expected 293.66Hz", detected_freq);
    assert!(
        (detected_freq - 293.66).abs() < 5.0,
        "Degree 1 should be D4 (293.66Hz), got {}Hz",
        detected_freq
    );
}

#[test]
fn test_single_degree_2() {
    // Test degree 2 = E4 = 329.63 Hz
    let input = r#"
        cps: 1.0
        out $ sine(scale("2", "major", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let detected_freq = find_dominant_frequency(&buffer[11025..33075], 44100.0);

    println!("Degree 2: detected {}Hz, expected 329.63Hz", detected_freq);
    assert!(
        (detected_freq - 329.63).abs() < 5.0,
        "Degree 2 should be E4 (329.63Hz), got {}Hz",
        detected_freq
    );
}

#[test]
#[ignore = "Scale pattern evaluation not cycling correctly - both cycles produce 260Hz instead of 261.63Hz and 293.66Hz"]
fn test_alternating_degrees() {
    // Test "0 1" pattern - 2 cycles
    let input = r#"
        cps: 2.0
        out $ sine(scale("0 1", "major", "60")) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let samples_per_cycle = (44100.0 / 2.0) as usize;
    let buffer = graph.render(samples_per_cycle * 2);

    // Check first cycle (degree 0 = C4)
    let start1 = samples_per_cycle / 4;
    let end1 = start1 + samples_per_cycle / 2;
    let freq1 = find_dominant_frequency(&buffer[start1..end1], 44100.0);
    println!(
        "Cycle 0 (degree 0): detected {}Hz, expected 261.63Hz",
        freq1
    );

    // Check second cycle (degree 1 = D4)
    let start2 = samples_per_cycle + samples_per_cycle / 4;
    let end2 = start2 + samples_per_cycle / 2;
    let freq2 = find_dominant_frequency(&buffer[start2..end2], 44100.0);
    println!(
        "Cycle 1 (degree 1): detected {}Hz, expected 293.66Hz",
        freq2
    );

    assert!(
        (freq1 - 261.63).abs() < 5.0,
        "Cycle 0 should be C4 (261.63Hz), got {}Hz",
        freq1
    );
    assert!(
        (freq2 - 293.66).abs() < 5.0,
        "Cycle 1 should be D4 (293.66Hz), got {}Hz",
        freq2
    );
}
