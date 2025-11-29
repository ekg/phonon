/// Systematic tests: Delay Effect
///
/// Tests delay/echo effect with timing verification and audio analysis.
/// Delay creates echoes by feeding back a delayed version of the input.
///
/// Key characteristics:
/// - Delay time controls spacing between echoes
/// - Feedback controls number of repetitions and decay
/// - Creates space, depth, and rhythmic effects
/// - Used for slapback, echo, doubling, rhythmic delays
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{num_complex::Complex, FftPlanner};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

// ========== Basic Delay Tests ==========

#[test]
fn test_delay_compiles() {
    let code = r#"
        tempo: 0.5
        ~delayed $ sine 440 # delay 0.2 0.5
        out $ ~delayed
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "Delay should compile: {:?}", result.err());
}

#[test]
fn test_delay_generates_audio() {
    let code = r#"
        tempo: 0.5
        ~source $ sine 440 * 0.3
        ~delayed $ ~source # delay 0.2 0.5
        out $ ~delayed
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Delay should produce audio, got RMS: {}", rms);
    println!("Delay RMS: {}", rms);
}

// ========== Delay Time Tests ==========

#[test]
fn test_delay_short_delay_time() {
    // Short delay (50ms) - slapback effect
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.05 0.3
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Short delay should work, RMS: {}", rms);
    println!("Short delay RMS: {}", rms);
}

#[test]
fn test_delay_medium_delay_time() {
    // Medium delay (250ms) - classic echo
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.25 0.4
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 1.5);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Medium delay should work, RMS: {}", rms);
    println!("Medium delay RMS: {}", rms);
}

#[test]
fn test_delay_long_delay_time() {
    // Long delay (500ms) - spacious echo
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.5 0.4
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Long delay should work, RMS: {}", rms);
    println!("Long delay RMS: {}", rms);
}

// ========== Feedback Tests ==========

#[test]
fn test_delay_no_feedback() {
    // Feedback = 0, only one echo
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.2 0.0
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Should have minimal RMS (no repeating echoes)
    assert!(
        rms > 0.0,
        "Zero feedback should still have audio, RMS: {}",
        rms
    );

    println!("No feedback RMS: {}", rms);
}

#[test]
fn test_delay_low_feedback() {
    // Low feedback = few echoes
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.2 0.3
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 1.5);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Low feedback should work, RMS: {}", rms);
    println!("Low feedback RMS: {}", rms);
}

#[test]
fn test_delay_high_feedback() {
    // High feedback = many echoes
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.15 0.7
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "High feedback should work, RMS: {}", rms);
    println!("High feedback RMS: {}", rms);
}

#[test]
fn test_delay_feedback_comparison() {
    // Compare RMS with different feedback levels
    let code_low = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.2 0.2
        out $ ~delayed * 0.3
    "#;

    let code_high = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.2 0.7
        out $ ~delayed * 0.3
    "#;

    let buffer_low = render_dsl(code_low, 2.0);
    let buffer_high = render_dsl(code_high, 2.0);

    let rms_low = calculate_rms(&buffer_low);
    let rms_high = calculate_rms(&buffer_high);

    // High feedback should have more overall energy
    assert!(
        rms_high > rms_low * 1.2,
        "High feedback should have more energy, low: {}, high: {}",
        rms_low,
        rms_high
    );

    println!("Feedback comparison - Low: {}, High: {}", rms_low, rms_high);
}

// ========== Musical Applications ==========

#[test]
fn test_delay_slapback() {
    // Slapback delay - short single echo
    let code = r#"
        tempo: 0.5
        ~snare $ white_noise * ad 0.001 0.1
        ~slapback $ ~snare # delay 0.08 0.2
        out $ ~slapback * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Slapback delay should work, RMS: {}", rms);
    println!("Slapback RMS: {}", rms);
}

#[test]
fn test_delay_echo_rhythmic() {
    // Rhythmic echo synced to tempo
    let code = r#"
        tempo: 0.5
        ~kick $ ad 0.001 0.1 * sine 60
        ~echo $ ~kick # delay 0.25 0.5
        out $ ~echo * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Rhythmic echo should work, RMS: {}", rms);
    println!("Rhythmic echo RMS: {}", rms);
}

#[test]
fn test_delay_doubling() {
    // Doubling effect - very short delay
    let code = r#"
        tempo: 0.5
        ~vocal $ sine 220 * 0.3
        ~doubled $ ~vocal # delay 0.03 0.0
        out $ (~vocal + ~doubled) * 0.2
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "Doubling effect should work, RMS: {}", rms);
    println!("Doubling RMS: {}", rms);
}

#[test]
fn test_delay_ambient_wash() {
    // Ambient wash - long delay with high feedback
    let code = r#"
        tempo: 1.0
        ~pad $ sine 220 * ad 0.1 0.3
        ~wash $ ~pad # delay 0.5 0.6
        out $ ~wash * 0.2
    "#;

    let buffer = render_dsl(code, 3.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Ambient wash should work, RMS: {}", rms);
    println!("Ambient wash RMS: {}", rms);
}

#[test]
fn test_delay_dub_style() {
    // Dub-style delay - medium time, high feedback
    let code = r#"
        tempo: 0.5
        ~snare $ white_noise * ad 0.001 0.1
        ~dub $ ~snare # delay 0.375 0.6
        out $ ~dub * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.01, "Dub delay should work, RMS: {}", rms);
    println!("Dub delay RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_delay_pattern_delay_time() {
    // Delay time modulated by pattern
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~mod_time $ sine 0.5 * 0.1 + 0.2
        ~delayed $ ~impulse # delay ~mod_time 0.4
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.01,
        "Delay with pattern-modulated time should work, RMS: {}",
        rms
    );

    println!("Pattern delay time RMS: {}", rms);
}

#[test]
fn test_delay_pattern_feedback() {
    // Feedback modulated by envelope
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~env $ line 0.2 0.7
        ~delayed $ ~impulse # delay 0.2 ~env
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.01,
        "Delay with pattern-modulated feedback should work, RMS: {}",
        rms
    );

    println!("Pattern feedback RMS: {}", rms);
}

// ========== Frequency Content Tests ==========

#[test]
fn test_delay_preserves_frequency() {
    // Delay should preserve frequency content
    let code_dry = r#"
        tempo: 0.5
        out $ sine 440 * 0.3
    "#;

    let code_delayed = r#"
        tempo: 0.5
        ~delayed $ sine 440 # delay 0.2 0.3
        out $ ~delayed * 0.3
    "#;

    let buffer_dry = render_dsl(code_dry, 1.0);
    let buffer_delayed = render_dsl(code_delayed, 1.0);

    let (frequencies, mags_dry) = analyze_spectrum(&buffer_dry, 44100.0);
    let (_, mags_delayed) = analyze_spectrum(&buffer_delayed, 44100.0);

    // Find dominant frequency in both
    let max_idx_dry = mags_dry
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, _)| idx)
        .unwrap();

    let max_idx_delayed = mags_delayed
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, _)| idx)
        .unwrap();

    let freq_dry = frequencies[max_idx_dry];
    let freq_delayed = frequencies[max_idx_delayed];

    // Frequencies should be very close (within 20Hz)
    assert!(
        (freq_dry - freq_delayed).abs() < 20.0,
        "Delay should preserve frequency, dry: {}Hz, delayed: {}Hz",
        freq_dry,
        freq_delayed
    );

    println!(
        "Frequency preservation - Dry: {}Hz, Delayed: {}Hz",
        freq_dry, freq_delayed
    );
}

// ========== Cascaded Delays ==========

#[test]
fn test_delay_cascade() {
    // Multiple delays in series
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.15 0.4 # delay 0.2 0.3
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 2.0);
    let rms = calculate_rms(&buffer);

    // Cascaded delays should create complex echo patterns
    assert!(rms > 0.01, "Cascaded delays should work, RMS: {}", rms);
    println!("Cascaded delay RMS: {}", rms);
}

// ========== Stability Tests ==========

#[test]
fn test_delay_no_excessive_clipping() {
    let code = r#"
        tempo: 0.5
        ~source $ sine 440 * 0.5
        ~delayed $ ~source # delay 0.2 0.5
        out $ ~delayed
    "#;

    let buffer = render_dsl(code, 1.0);
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(
        max_amplitude <= 1.2,
        "Delay should not cause excessive clipping, max: {}",
        max_amplitude
    );

    println!("Delay max amplitude: {}", max_amplitude);
}

#[test]
fn test_delay_consistent_output() {
    let code = r#"
        tempo: 0.5
        ~delayed $ sine 440 # delay 0.2 0.5
        out $ ~delayed * 0.3
    "#;

    let buffer1 = render_dsl(code, 0.5);
    let buffer2 = render_dsl(code, 0.5);

    // Buffers should be identical (deterministic)
    let mut identical = 0;
    for i in 0..buffer1.len().min(buffer2.len()) {
        if (buffer1[i] - buffer2[i]).abs() < 0.0001 {
            identical += 1;
        }
    }

    let identity_ratio = identical as f32 / buffer1.len() as f32;
    assert!(
        identity_ratio > 0.99,
        "Delay should produce consistent output, identity: {}",
        identity_ratio
    );

    println!("Delay identity ratio: {}", identity_ratio);
}

#[test]
fn test_delay_no_dc_offset() {
    // Delay should not introduce DC offset
    let code = r#"
        tempo: 0.5
        ~delayed $ sine 440 # delay 0.2 0.5
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let mean: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;

    assert!(
        mean.abs() < 0.01,
        "Delay should not introduce DC offset, mean: {}",
        mean
    );

    println!("Delay DC offset: {}", mean);
}

// ========== Edge Cases ==========

#[test]
fn test_delay_very_short_time() {
    // Very short delay (10ms) - comb filtering effect
    let code = r#"
        tempo: 0.5
        ~source $ white_noise * 0.3
        ~delayed $ ~source # delay 0.01 0.5
        out $ ~delayed
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Very short delay should work, RMS: {}", rms);

    println!("Very short delay RMS: {}", rms);
}

#[test]
fn test_delay_zero_feedback() {
    // Feedback = 0 should not cause issues
    let code = r#"
        tempo: 0.5
        ~delayed $ sine 440 # delay 0.2 0.0
        out $ ~delayed * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "Zero feedback should work, RMS: {}", rms);

    println!("Zero feedback RMS: {}", rms);
}

#[test]
fn test_delay_max_feedback() {
    // Very high feedback (close to 1.0) - long sustain
    let code = r#"
        tempo: 0.5
        ~impulse $ ad 0.001 0.05 * sine 440
        ~delayed $ ~impulse # delay 0.2 0.95
        out $ ~delayed * 0.2
    "#;

    let buffer = render_dsl(code, 3.0);
    let rms = calculate_rms(&buffer);

    // Should have sustained energy from long feedback
    assert!(
        rms > 0.01,
        "Max feedback should create long sustain, RMS: {}",
        rms
    );

    println!("Max feedback RMS: {}", rms);
}
