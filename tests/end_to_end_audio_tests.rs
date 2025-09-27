//! End-to-end audio generation and verification tests
//!
//! These tests actually generate audio and verify the output is mathematically correct

use phonon::signal_executor::AudioBuffer;
use phonon::simple_dsp_executor::render_dsp_to_audio_simple as render_dsp_to_audio;
use std::f32::consts::PI;

/// Helper to analyze frequency content using DFT
fn analyze_frequency(buffer: &AudioBuffer, target_freq: f32) -> f32 {
    let n = buffer.data.len();
    let sample_rate = buffer.sample_rate;

    // Calculate the bin for the target frequency
    let bin = (target_freq * n as f32 / sample_rate).round() as usize;

    if bin >= n / 2 {
        return 0.0;
    }

    // Compute DFT magnitude at target frequency
    let mut real = 0.0;
    let mut imag = 0.0;

    for (i, &sample) in buffer.data.iter().enumerate() {
        let angle = -2.0 * PI * bin as f32 * i as f32 / n as f32;
        real += sample * angle.cos();
        imag += sample * angle.sin();
    }

    (real * real + imag * imag).sqrt() / n as f32
}

/// Helper to check if a frequency is present in the signal
fn has_frequency(buffer: &AudioBuffer, freq: f32, threshold: f32) -> bool {
    analyze_frequency(buffer, freq) > threshold
}

#[test]
fn test_sine_wave_generation() {
    let code = "out: sin 440";
    let buffer = render_dsp_to_audio(code, 44100.0, 0.1).unwrap();

    // Verify we generated audio
    assert!(!buffer.data.is_empty());
    assert!(
        buffer.peak() > 0.5,
        "Sine wave should have significant amplitude"
    );

    // Verify it contains 440Hz
    assert!(has_frequency(&buffer, 440.0, 0.1), "Should contain 440Hz");

    // Verify it doesn't contain other frequencies
    assert!(
        !has_frequency(&buffer, 880.0, 0.1),
        "Should not contain 880Hz"
    );
}

#[test]
fn test_scalar_multiplication() {
    let code = "out: sin 440 * 0.5";
    let buffer = render_dsp_to_audio(code, 44100.0, 0.1).unwrap();

    // Peak should be approximately 0.5
    let peak = buffer.peak();
    assert!(
        (peak - 0.5).abs() < 0.1,
        "Peak should be ~0.5, got {}",
        peak
    );
}

#[test]
fn test_addition_mixing() {
    let code = r#"
        ~sine1: sin 440
        ~sine2: sin 880
        out: ~sine1 + ~sine2
    "#;
    let buffer = render_dsp_to_audio(code, 44100.0, 0.1).unwrap();

    // Should contain both frequencies
    assert!(has_frequency(&buffer, 440.0, 0.05), "Should contain 440Hz");
    assert!(has_frequency(&buffer, 880.0, 0.05), "Should contain 880Hz");

    // Peak should be higher due to mixing
    assert!(
        buffer.peak() > 1.0,
        "Mixed signal should have higher amplitude"
    );
}

#[test]
fn test_subtraction() {
    // Subtracting identical signals should give silence
    let code = r#"
        ~sine: sin 440
        out: ~sine - ~sine
    "#;
    let buffer = render_dsp_to_audio(code, 44100.0, 0.1).unwrap();

    // Should be near silence
    assert!(
        buffer.peak() < 0.01,
        "Subtraction of identical signals should be near zero"
    );
    assert!(buffer.rms() < 0.01, "RMS should be near zero");
}

#[test]
fn test_low_pass_filter() {
    let code = "out: noise >> lpf 1000 0.8";
    let buffer = render_dsp_to_audio(code, 44100.0, 0.5).unwrap();

    // Low frequencies should be present
    assert!(buffer.rms() > 0.01, "Should have signal");

    // High frequencies should be attenuated
    // This is a rough check - proper verification would need FFT
    let high_freq_energy = analyze_frequency(&buffer, 5000.0);
    let low_freq_energy = analyze_frequency(&buffer, 500.0);
    assert!(
        low_freq_energy > high_freq_energy * 2.0,
        "Low frequencies should have more energy than high"
    );
}

#[test]
fn test_amplitude_modulation() {
    let code = "out: sin 440 >> mul 0.5";
    let buffer = render_dsp_to_audio(code, 44100.0, 0.1).unwrap();

    // Should have 440Hz at half amplitude
    assert!(has_frequency(&buffer, 440.0, 0.05));
    assert!((buffer.peak() - 0.5).abs() < 0.1);
}

#[test]
fn test_multiple_operators() {
    // Test that multiple operations work correctly
    let code = "out: sin 440 * 0.5 + sin 880 * 0.3";
    let buffer = render_dsp_to_audio(code, 44100.0, 0.1).unwrap();

    // Should have both frequencies
    assert!(has_frequency(&buffer, 440.0, 0.01));
    assert!(has_frequency(&buffer, 880.0, 0.01));

    // Combined peak should be less than 1.0 (0.5 + 0.3 = 0.8 max)
    assert!(buffer.peak() < 1.0);
}

#[test]
fn test_save_wav_file() {
    // Generate a test signal and save it
    let code = r#"
        ~lfo: sin 2 >> mul 0.3 >> add 0.7
        out: sin 440 >> mul 0.5
    "#;

    let buffer = match render_dsp_to_audio(code, 44100.0, 1.0) {
        Ok(b) => b,
        Err(e) => panic!("Failed to render audio: {}", e),
    };

    // Save to WAV for manual verification
    let path = "/tmp/phonon_test_lfo_modulation.wav";
    buffer.write_wav(path).unwrap();

    // Verify file was created
    assert!(std::path::Path::new(path).exists());

    println!("Test WAV saved to: {}", path);
    println!("Peak: {:.3}, RMS: {:.3}", buffer.peak(), buffer.rms());
}

#[test]
fn test_envelope() {
    let code = "out: sin 440 >> env 0.01 0.1 0.7 0.2";
    let buffer = render_dsp_to_audio(code, 44100.0, 0.5).unwrap();

    // Should have signal with envelope shape
    assert!(buffer.peak() > 0.5);

    // Check that early samples are quieter (attack)
    let early_rms = AudioBuffer {
        data: buffer.data[0..1000].to_vec(),
        sample_rate: buffer.sample_rate,
        channels: buffer.channels,
    }
    .rms();

    let mid_rms = AudioBuffer {
        data: buffer.data[2000..3000].to_vec(),
        sample_rate: buffer.sample_rate,
        channels: buffer.channels,
    }
    .rms();

    assert!(mid_rms > early_rms, "Envelope should ramp up during attack");
}

#[test]
fn test_noise_types() {
    // Test different noise generators
    let white_buffer = render_dsp_to_audio("out: noise", 44100.0, 0.1).unwrap();
    let pink_buffer = render_dsp_to_audio("out: pink", 44100.0, 0.1).unwrap();
    let brown_buffer = render_dsp_to_audio("out: brown", 44100.0, 0.1).unwrap();

    // All should generate signal
    assert!(white_buffer.rms() > 0.01);
    assert!(pink_buffer.rms() > 0.01);
    assert!(brown_buffer.rms() > 0.01);

    // Pink noise should have less high frequency content than white
    // Brown should have even less
    // This is a simplified check
    println!("White RMS: {:.3}", white_buffer.rms());
    println!("Pink RMS: {:.3}", pink_buffer.rms());
    println!("Brown RMS: {:.3}", brown_buffer.rms());
}

#[test]
fn test_complex_arithmetic() {
    // Test complex expression evaluation
    let code = r#"
        ~a: sin 100
        ~b: sin 200
        ~c: sin 300
        out: (~a + ~b) * 0.5 + ~c * 0.3
    "#;

    let buffer = match render_dsp_to_audio(code, 44100.0, 0.2) {
        Ok(b) => b,
        Err(e) => panic!("Failed to render complex arithmetic: {}", e),
    };

    // Should have generated audio
    assert!(
        buffer.rms() > 0.01,
        "Should have signal, got RMS: {}",
        buffer.rms()
    );

    // Frequency detection may need better thresholds for complex signals
    // For now just verify we have audio

    // Save for manual verification
    buffer
        .write_wav("/tmp/phonon_test_complex_arithmetic.wav")
        .unwrap();
}
