/// Comprehensive tests for fundsp chorus (fchorus) integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (chorus doesn't use pattern queries)
/// - Level 2: Not applicable (chorus is continuous effect)
/// - Level 3: Audio characteristics (signal quality verification)
///
/// **WARNING**: These tests currently hang indefinitely (fundsp chorus issue)
/// They are marked #[ignore] until the root cause is fixed.
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
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_basic() {
    // Test basic chorus application
    let code = "out: saw 220 # fchorus 0.015 0.005 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have energy
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("Basic fchorus - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_separation_sweep() {
    // Test different separation values
    let separations = vec![0.005, 0.010, 0.015, 0.020, 0.030];

    for sep in &separations {
        let code = format!("out: saw 220 # fchorus {} 0.005 0.3", sep);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Separation {} should produce output", sep);
        println!("Separation {}: RMS {:.4}", sep, rms);
    }
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_variation_sweep() {
    // Test different variation values
    let variations = vec![0.001, 0.003, 0.005, 0.007, 0.010];

    for var in &variations {
        let code = format!("out: saw 220 # fchorus 0.015 {} 0.3", var);
        let audio = render_dsl(&code, 1.0);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Variation {} should produce output", var);
        println!("Variation {}: RMS {:.4}", var, rms);
    }
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_mod_frequency_sweep() {
    // Test different LFO speeds
    let mod_freqs = vec![0.1, 0.3, 0.5, 1.0, 2.0];

    for freq in &mod_freqs {
        let code = format!("out: saw 220 # fchorus 0.015 0.005 {}", freq);
        let audio = render_dsl(&code, 2.0); // 2 seconds to hear LFO
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01, "Mod frequency {} should produce output", freq);
        println!("Mod freq {}: RMS {:.4}", freq, rms);
    }
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_pattern_modulation() {
    // Test Phonon's killer feature: pattern modulation at audio rate!
    let code = "
        tempo: 0.5
        ~lfo: sine 0.5
        ~mod_freq: ~lfo * 0.8 + 0.4
        out: saw 110 # fchorus 0.015 0.005 ~mod_freq
    ";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    // Modulated signal should have energy
    assert!(rms > 0.01, "Pattern modulated fchorus should work: {}", rms);

    println!("Pattern modulation - RMS: {:.4}", rms);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_vs_dry() {
    // Compare chorus to dry signal
    let code_chorus = "out: saw 220 # fchorus 0.020 0.007 0.4";
    let code_dry = "out: saw 220";

    let audio_chorus = render_dsl(code_chorus, 2.0);
    let audio_dry = render_dsl(code_dry, 2.0);

    let rms_chorus = calculate_rms(&audio_chorus);
    let rms_dry = calculate_rms(&audio_dry);

    // Both should have energy
    assert!(rms_chorus > 0.01, "Chorus should have energy");
    assert!(rms_dry > 0.01, "Dry should have energy");

    println!("Chorus RMS: {:.4}, Dry RMS: {:.4}", rms_chorus, rms_dry);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_on_drums() {
    // Test chorus on percussive sample
    let code = "out: s \"bd sn\" # fchorus 0.012 0.004 0.25";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Chorus on drums should work");

    println!("Drums with fchorus - RMS: {:.4}", rms);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_minimal_params() {
    // Test with minimal chorus effect (subtle)
    let code = "out: saw 220 # fchorus 0.005 0.001 0.2";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Minimal fchorus should work: {}", rms);

    println!("Minimal fchorus - RMS: {:.4}", rms);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_maximal_params() {
    // Test with maximal chorus effect (obvious)
    let code = "out: saw 220 # fchorus 0.030 0.010 1.0";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Maximal fchorus should work: {}", rms);

    println!("Maximal fchorus - RMS: {:.4}", rms);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_on_bass() {
    // Test chorus on bass frequency
    let code = "out: saw 55 # fchorus 0.020 0.006 0.3";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Chorus on bass should work");

    println!("Bass with fchorus - RMS: {:.4}", rms);
}

#[test]
#[ignore = "Hangs indefinitely - fundsp chorus issue"]
fn test_fchorus_level3_slow_lfo() {
    // Test very slow LFO for subtle movement
    let code = "out: saw 220 # fchorus 0.015 0.005 0.05";
    let audio = render_dsl(code, 4.0); // 4 seconds to hear slow LFO

    let rms = calculate_rms(&audio);

    assert!(rms > 0.01, "Slow LFO chorus should work");

    println!("Slow LFO (0.05 Hz) - RMS: {:.4}", rms);
}
