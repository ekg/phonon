/// Comprehensive tests for fundsp moog_hz integration
///
/// Following the three-level testing methodology:
/// - Level 1: Not applicable (filters don't use pattern queries)
/// - Level 2: Not applicable (filters are continuous)
/// - Level 3: Audio characteristics (signal quality verification)
/// - Level 4: Comparative testing (fundsp vs custom moogLadder)
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;

    // Parse the DSL code
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");

    // Compile to signal graph
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");

    // Calculate number of samples
    let num_samples = (duration * sample_rate) as usize;

    // Render audio
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
fn test_moog_hz_level3_basic_filtering() {
    // Test that moog_hz filters a saw wave
    let code = "out: saw 220 # moog_hz 1000 0.7";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have filtered output with reasonable amplitude
    assert!(rms > 0.01, "RMS too low: {}", rms);
    assert!(rms < 1.0, "RMS too high: {}", rms);
    assert!(peak > rms, "Peak should be higher than RMS");

    println!(
        "moog_hz basic filtering - RMS: {:.4}, Peak: {:.4}",
        rms, peak
    );
}

#[test]
fn test_moog_hz_level3_cutoff_sweep() {
    // Test different cutoff frequencies
    let cutoffs = vec![100, 500, 1000, 5000, 10000];
    let mut rms_values = Vec::new();

    for cutoff in &cutoffs {
        let code = format!("out: saw 220 # moog_hz {} 0.5", cutoff);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);
        rms_values.push(rms);

        // All should have energy
        assert!(rms > 0.01, "Cutoff {} Hz: RMS too low", cutoff);

        println!("Cutoff {} Hz - RMS: {:.4}", cutoff, rms);
    }

    // Higher cutoffs should generally pass more signal
    // (100 Hz cutoff should attenuate 220 Hz saw more than 5000 Hz cutoff)
    assert!(
        rms_values[3] > rms_values[0],
        "5000 Hz cutoff should pass more signal than 100 Hz cutoff"
    );
}

#[test]
fn test_moog_hz_level3_resonance_sweep() {
    // Test different resonance values
    let resonances = vec![0.0, 0.3, 0.5, 0.7, 0.9];

    for res in &resonances {
        let code = format!("out: saw 220 # moog_hz 1000 {}", res);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);
        let peak = calculate_peak(&audio);

        // All should produce output
        assert!(rms > 0.01, "Resonance {}: RMS too low", res);

        println!("Resonance {} - RMS: {:.4}, Peak: {:.4}", res, rms, peak);
    }
}

#[test]
fn test_moog_hz_level3_pattern_modulation() {
    // Test Phonon's killer feature: pattern modulation at audio rate!
    let code = "
        tempo: 2.0
        ~lfo: sine 0.5
        ~cutoff: ~lfo * 2000 + 1000
        out: saw 110 # moog_hz ~cutoff 0.7
    ";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Modulated signal should have energy
    assert!(rms > 0.01, "Modulated RMS too low: {}", rms);
    assert!(rms < 1.0, "Modulated RMS too high: {}", rms);

    // Compare to static cutoff
    let code_static = "out: saw 110 # moog_hz 1000 0.7";
    let audio_static = render_dsl(code_static, 2.0);
    let rms_static = calculate_rms(&audio_static);

    // Should have similar energy (within 50%)
    let ratio = rms / rms_static;
    assert!(
        ratio > 0.5 && ratio < 1.5,
        "Modulated/static ratio out of range: {:.2}",
        ratio
    );

    println!(
        "Pattern modulation - RMS: {:.4}, Peak: {:.4}, Static RMS: {:.4}, Ratio: {:.2}",
        rms, peak, rms_static, ratio
    );
}

#[test]
fn test_moog_hz_level3_silence_comparison() {
    // Test that moog_hz actually produces sound (not silence)
    let code_moog = "out: saw 220 # moog_hz 1000 0.5";
    let code_silence = "out: sine 0 * 0";

    let audio_moog = render_dsl(code_moog, 1.0);
    let audio_silence = render_dsl(code_silence, 1.0);

    let rms_moog = calculate_rms(&audio_moog);
    let rms_silence = calculate_rms(&audio_silence);

    // moog_hz should have significantly more energy than silence
    assert!(
        rms_moog > rms_silence * 100.0,
        "moog_hz not producing enough sound vs silence"
    );

    println!(
        "moog_hz RMS: {:.4}, silence RMS: {:.6}",
        rms_moog, rms_silence
    );
}

#[test]
fn test_moog_hz_level3_low_cutoff() {
    // Test very low cutoff frequency
    let code = "out: saw 220 # moog_hz 50 0.5";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Very low cutoff should heavily attenuate 220 Hz saw
    // But should still produce some output
    assert!(
        rms > 0.001,
        "Very low cutoff should still produce some output"
    );
    assert!(rms < 0.5, "Very low cutoff should attenuate signal");

    println!("Very low cutoff (50 Hz) - RMS: {:.4}", rms);
}

#[test]
fn test_moog_hz_level3_high_resonance() {
    // Test high resonance (near self-oscillation)
    let code = "out: saw 220 # moog_hz 1000 0.95";
    let audio = render_dsl(code, 0.5);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // High resonance can increase amplitude near cutoff
    assert!(rms > 0.01, "High resonance should produce output");
    // Allow higher peak due to resonance boost
    assert!(peak < 5.0, "Peak should not be excessive: {}", peak);

    println!("High resonance (0.95) - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

// ========== LEVEL 4: COMPARATIVE TESTING (fundsp vs custom) ==========
// NOTE: These tests are ignored because custom moogLadder doesn't exist yet

#[test]
#[ignore = "custom moogLadder not implemented yet"]
fn test_moog_hz_level4_vs_custom_moog_ladder() {
    // Compare fundsp moog_hz to our custom moogLadder implementation
    // This validates both implementations!

    // fundsp implementation
    let code_fundsp = "out: saw 220 # moog_hz 1000 0.7";
    let audio_fundsp = render_dsl(code_fundsp, 1.0);

    // Our custom implementation
    let code_custom = "out: saw 220 # moogLadder 1000 0.7";
    let audio_custom = render_dsl(code_custom, 1.0);

    let rms_fundsp = calculate_rms(&audio_fundsp);
    let rms_custom = calculate_rms(&audio_custom);
    let peak_fundsp = calculate_peak(&audio_fundsp);
    let peak_custom = calculate_peak(&audio_custom);

    // Should have similar amplitude (within 50%)
    let ratio = rms_fundsp / rms_custom;
    assert!(
        ratio > 0.5 && ratio < 1.5,
        "fundsp/custom RMS ratio too different: {:.2}",
        ratio
    );

    println!(
        "fundsp moog_hz - RMS: {:.4}, Peak: {:.4}",
        rms_fundsp, peak_fundsp
    );
    println!(
        "custom moogLadder - RMS: {:.4}, Peak: {:.4}",
        rms_custom, peak_custom
    );
    println!("RMS ratio (fundsp/custom): {:.2}", ratio);
}

#[test]
#[ignore = "custom moogLadder not implemented yet"]
fn test_moog_hz_level4_cutoff_comparison() {
    // Compare behavior across different cutoff frequencies
    let cutoffs = vec![500, 1000, 2000, 4000];

    for cutoff in cutoffs {
        let code_fundsp = format!("out: saw 220 # moog_hz {} 0.5", cutoff);
        let code_custom = format!("out: saw 220 # moogLadder {} 0.5", cutoff);

        let audio_fundsp = render_dsl(&code_fundsp, 0.5);
        let audio_custom = render_dsl(&code_custom, 0.5);

        let rms_fundsp = calculate_rms(&audio_fundsp);
        let rms_custom = calculate_rms(&audio_custom);

        let ratio = rms_fundsp / rms_custom;

        // Should have similar behavior at all cutoffs (within 50%)
        assert!(
            ratio > 0.5 && ratio < 1.5,
            "Cutoff {} Hz: ratio too different: {:.2}",
            cutoff,
            ratio
        );

        println!(
            "Cutoff {} Hz - fundsp RMS: {:.4}, custom RMS: {:.4}, ratio: {:.2}",
            cutoff, rms_fundsp, rms_custom, ratio
        );
    }
}

#[test]
#[ignore = "custom moogLadder not implemented yet"]
fn test_moog_hz_level4_resonance_comparison() {
    // Compare behavior across different resonance values
    let resonances = vec![0.1, 0.3, 0.5, 0.7, 0.9];

    for res in resonances {
        let code_fundsp = format!("out: saw 220 # moog_hz 1000 {}", res);
        let code_custom = format!("out: saw 220 # moogLadder 1000 {}", res);

        let audio_fundsp = render_dsl(&code_fundsp, 0.5);
        let audio_custom = render_dsl(&code_custom, 0.5);

        let rms_fundsp = calculate_rms(&audio_fundsp);
        let rms_custom = calculate_rms(&audio_custom);
        let peak_fundsp = calculate_peak(&audio_fundsp);
        let peak_custom = calculate_peak(&audio_custom);

        println!(
            "Resonance {} - fundsp (RMS: {:.4}, Peak: {:.4}), custom (RMS: {:.4}, Peak: {:.4})",
            res, rms_fundsp, peak_fundsp, rms_custom, peak_custom
        );

        // Both should produce output
        assert!(
            rms_fundsp > 0.01,
            "fundsp should produce output at Q={}",
            res
        );
        assert!(
            rms_custom > 0.01,
            "custom should produce output at Q={}",
            res
        );
    }
}

#[test]
#[ignore = "custom moogLadder not implemented yet"]
fn test_moog_hz_level4_pattern_modulation_comparison() {
    // Compare pattern modulation behavior
    let code_fundsp = "
        tempo: 2.0
        ~lfo: sine 0.5
        ~cutoff: ~lfo * 2000 + 1000
        out: saw 110 # moog_hz ~cutoff 0.7
    ";

    let code_custom = "
        tempo: 2.0
        ~lfo: sine 0.5
        ~cutoff: ~lfo * 2000 + 1000
        out: saw 110 # moogLadder ~cutoff 0.7
    ";

    let audio_fundsp = render_dsl(code_fundsp, 2.0);
    let audio_custom = render_dsl(code_custom, 2.0);

    let rms_fundsp = calculate_rms(&audio_fundsp);
    let rms_custom = calculate_rms(&audio_custom);

    let ratio = rms_fundsp / rms_custom;

    // Pattern modulation should work similarly for both (within 50%)
    assert!(
        ratio > 0.5 && ratio < 1.5,
        "Pattern modulation ratio too different: {:.2}",
        ratio
    );

    println!(
        "Pattern modulation - fundsp RMS: {:.4}, custom RMS: {:.4}, ratio: {:.2}",
        rms_fundsp, rms_custom, ratio
    );
}
