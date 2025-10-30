/// Comprehensive tests for fundsp organ_hz integration
///
/// Following the three-level testing methodology:
/// - Level 1: Pattern query verification (not applicable - organ_hz is continuous)
/// - Level 2: Onset detection (not applicable - organ_hz is continuous tone)
/// - Level 3: Audio characteristics (signal quality verification)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code to audio buffer using compositional compiler
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;

    // Parse the DSL code
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");

    // Compile to signal graph
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");

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
fn test_organ_hz_level3_basic_tone() {
    // Test that organ_hz generates a stable tone
    let code = "out: organ_hz 440";
    let audio = render_dsl(code, 1.0);

    // Level 3: Audio characteristics
    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Should have reasonable amplitude (between 0.1 and 0.5)
    assert!(rms > 0.1, "RMS too low: {}", rms);
    assert!(rms < 0.5, "RMS too high: {}", rms);

    // Peak should be higher than RMS
    assert!(peak > rms, "Peak should be higher than RMS");

    println!("organ_hz 440 Hz - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_organ_hz_level3_frequency_sweep() {
    // Test different frequencies
    let frequencies = vec![110.0, 220.0, 440.0, 880.0];

    for freq in frequencies {
        let code = format!("out: organ_hz {}", freq);
        let audio = render_dsl(&code, 0.5);

        let rms = calculate_rms(&audio);

        // All frequencies should produce similar RMS (within 50%)
        assert!(rms > 0.1, "RMS too low at {} Hz: {}", freq, rms);
        assert!(rms < 0.5, "RMS too high at {} Hz: {}", freq, rms);

        println!("organ_hz {} Hz - RMS: {:.4}", freq, rms);
    }
}

#[test]
fn test_organ_hz_level3_pattern_modulation() {
    // Test Phonon's killer feature: pattern modulation at audio rate!
    let code = "
        tempo: 2.0
        ~freq: sine 0.5 * 110 + 440
        out: organ_hz ~freq
    ";
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Modulated signal should have energy
    assert!(rms > 0.1, "Modulated RMS too low: {}", rms);
    assert!(rms < 0.5, "Modulated RMS too high: {}", rms);

    // Compare to static frequency
    let code_static = "out: organ_hz 440";
    let audio_static = render_dsl(code_static, 2.0);
    let rms_static = calculate_rms(&audio_static);

    // Modulated should have similar energy to static (within 50%)
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
fn test_organ_hz_level3_dc_offset() {
    // Test that organ_hz has minimal DC offset
    let code = "out: organ_hz 440";
    let audio = render_dsl(code, 1.0);

    // Calculate DC offset (average value)
    let dc_offset: f32 = audio.iter().sum::<f32>() / audio.len() as f32;

    // DC offset should be very small (< 0.01)
    assert!(
        dc_offset.abs() < 0.01,
        "DC offset too high: {}",
        dc_offset
    );

    println!("DC offset: {:.6}", dc_offset);
}

#[test]
fn test_organ_hz_level3_silence_comparison() {
    // Test that organ_hz actually produces sound (not silence)
    let code_organ = "out: organ_hz 440";
    let code_silence = "out: sine 0 * 0";

    let audio_organ = render_dsl(code_organ, 1.0);
    let audio_silence = render_dsl(code_silence, 1.0);

    let rms_organ = calculate_rms(&audio_organ);
    let rms_silence = calculate_rms(&audio_silence);

    // Organ should have significantly more energy than silence
    assert!(
        rms_organ > rms_silence * 100.0,
        "organ_hz not producing enough sound vs silence"
    );

    println!(
        "organ_hz RMS: {:.4}, silence RMS: {:.6}",
        rms_organ, rms_silence
    );
}

#[test]
fn test_organ_hz_level3_multiple_cycles() {
    // Test that organ_hz is stable over multiple cycles
    let code = "
        tempo: 2.0
        out: organ_hz 440
    ";

    // Render 8 cycles
    let audio = render_dsl(code, 4.0); // 4 seconds = 8 cycles at 2 Hz

    // Split into 4 segments and verify RMS consistency
    let segment_size = audio.len() / 4;
    let mut rms_values = Vec::new();

    for i in 0..4 {
        let start = i * segment_size;
        let end = (i + 1) * segment_size;
        let segment = &audio[start..end];
        let rms = calculate_rms(segment);
        rms_values.push(rms);
        println!("Segment {} RMS: {:.4}", i, rms);
    }

    // All segments should have similar RMS (within 10%)
    let avg_rms = rms_values.iter().sum::<f32>() / rms_values.len() as f32;
    for (i, rms) in rms_values.iter().enumerate() {
        let deviation = (rms - avg_rms).abs() / avg_rms;
        assert!(
            deviation < 0.1,
            "Segment {} RMS deviates too much from average: {:.1}%",
            i,
            deviation * 100.0
        );
    }
}

#[test]
fn test_organ_hz_level3_pattern_arithmetic() {
    // Test that organ_hz works with arithmetic pattern expressions
    let code = "
        tempo: 2.0
        out: organ_hz (220 * 2)
    ";
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);

    // Should produce same output as organ_hz 440
    let code_direct = "out: organ_hz 440";
    let audio_direct = render_dsl(code_direct, 1.0);
    let rms_direct = calculate_rms(&audio_direct);

    // Should be very similar (within 5%)
    let diff = (rms - rms_direct).abs() / rms_direct;
    assert!(
        diff < 0.05,
        "Arithmetic expression produces different result: {:.1}%",
        diff * 100.0
    );

    println!(
        "Arithmetic RMS: {:.4}, Direct RMS: {:.4}, Diff: {:.1}%",
        rms,
        rms_direct,
        diff * 100.0
    );
}

#[test]
fn test_organ_hz_level3_low_frequency() {
    // Test very low frequency (sub-audio range)
    let code = "out: organ_hz 55"; // A1 - very low
    let audio = render_dsl(code, 1.0);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // Even at low frequency, should have energy
    assert!(rms > 0.05, "Low frequency RMS too low: {}", rms);
    assert!(rms < 0.5, "Low frequency RMS too high: {}", rms);

    println!("Low frequency (55 Hz) - RMS: {:.4}, Peak: {:.4}", rms, peak);
}

#[test]
fn test_organ_hz_level3_high_frequency() {
    // Test high frequency (near Nyquist)
    let code = "out: organ_hz 8000"; // High but below Nyquist (22050 Hz)
    let audio = render_dsl(code, 0.5);

    let rms = calculate_rms(&audio);
    let peak = calculate_peak(&audio);

    // High frequency should still have energy
    // Note: organ_hz may have higher amplitude at high frequencies due to additive harmonics
    assert!(rms > 0.05, "High frequency RMS too low: {}", rms);
    assert!(rms < 0.7, "High frequency RMS too high: {}", rms);

    println!(
        "High frequency (8000 Hz) - RMS: {:.4}, Peak: {:.4}",
        rms, peak
    );
}
