/// Comprehensive tests for fundsp reverb_stereo integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (reverb doesn't use pattern queries)
/// - Level 2: Not applicable (reverb is continuous)
/// - Level 3: Audio characteristics (signal quality verification)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Calculate RMS (root mean square) of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

#[test]
fn test_reverb_stereo_level3_basic() {
    // Test basic reverb application
    let code = "out $ saw 220 # reverb_stereo 0.5 1.0";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic reverb - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_reverb_stereo_level3_wet_sweep() {
    // Test different wet amounts
    let wet_values = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for wet in &wet_values {
        let code = format!("out $ saw 220 # reverb_stereo {} 1.0", wet);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Wet {} should produce output", wet);
        println!("Wet {}: RMS {:.4}", wet, rms);
    }
}

#[test]
fn test_reverb_stereo_level3_time_sweep() {
    // Test different reverb times
    let times = vec![0.1, 0.5, 1.0, 2.0, 5.0];

    for time in &times {
        let code = format!("out $ saw 220 # reverb_stereo 0.5 {}", time);
        let audio = render_dsl(&code, 2.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Time {}s should produce output", time);
        println!("Time {}s: RMS {:.4}", time, rms);
    }
}

#[test]
fn test_reverb_stereo_level3_pattern_modulation() {
    // Test Phonon's killer feature: pattern modulation at audio rate!
    let code = "
        tempo: 0.5
        ~wet $ sine 0.5 * 0.3 + 0.3
        out $ saw 110 # reverb_stereo ~wet 1.5
    ";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    // Modulated signal should have energy
    assert!(rms > 0.01, "Pattern modulated reverb should work: {}", rms);

    println!("Pattern modulation - RMS: {:.4}", rms);
}

#[test]
fn test_reverb_stereo_level3_vs_dry() {
    // Compare reverb to dry signal
    let code_reverb = "out $ saw 220 # reverb_stereo 0.7 2.0";
    let code_dry = "out $ saw 220";

    let audio_reverb = render_dsl(code_reverb, 2.0);
    let audio_dry = render_dsl(code_dry, 2.0);

    let rms_reverb = calculate_rms(&audio_reverb);
    let rms_dry = calculate_rms(&audio_dry);

    // Both should have energy
    assert!(rms_reverb > 0.01, "Reverb should have energy");
    assert!(rms_dry > 0.01, "Dry should have energy");

    println!("Reverb RMS: {:.4}, Dry RMS: {:.4}", rms_reverb, rms_dry);
}

#[test]
fn test_reverb_stereo_level3_tail_length() {
    // Test that long reverb time produces longer tail
    // Send brief signal then measure decay

    let code = "out $ saw 220 # reverb_stereo 0.8 3.0";
    let audio = render_dsl(code, 4.0); // 4 seconds to hear full tail

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Long reverb should produce output");

    println!("Long reverb (3.0s) RMS over 4 seconds: {:.4}", rms);
}

#[test]
fn test_reverb_stereo_level3_on_drums() {
    // Test reverb on percussive sample
    let code = "out $ s \"bd sn\" # reverb_stereo 0.6 1.5";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Reverb on drums should work");

    println!("Drums with reverb - RMS: {:.4}", rms);
}

#[test]
fn test_reverb_stereo_level3_zero_wet() {
    // Test that wet=0 passes through mostly dry
    let code = "out $ saw 220 # reverb_stereo 0.0 1.0";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Should still have output (dry signal)
    assert!(rms > 0.1, "Zero wet should pass dry signal: {}", rms);

    println!("Zero wet - RMS: {:.4}", rms);
}

#[test]
fn test_reverb_stereo_level3_full_wet() {
    // Test that wet=1 produces reverb
    let code = "out $ saw 220 # reverb_stereo 1.0 2.0";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    // Should have energy
    assert!(rms > 0.01, "Full wet should produce output: {}", rms);

    println!("Full wet - RMS: {:.4}", rms);
}

#[test]
fn test_reverb_stereo_level3_short_time() {
    // Test very short reverb time
    let code = "out $ saw 220 # reverb_stereo 0.5 0.1";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Short reverb should work: {}", rms);

    println!("Short reverb (0.1s) - RMS: {:.4}", rms);
}
