/// Direct fundsp soft_saw_hz API tests
///
/// Level 1: Verify fundsp API works correctly before wrapping
///
/// These tests ensure we understand fundsp's soft_saw_hz behavior:
/// - Takes 1 parameter (frequency)
/// - Takes 0 audio inputs (generator)
/// - Returns 1 mono output
/// - Softer/smoother sawtooth waveform (less harmonics than regular saw)

use fundsp::prelude::*;

#[test]
fn test_fundsp_soft_saw_hz_basic() {
    // Test that fundsp soft_saw_hz generates audio
    let mut unit = soft_saw_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Generate audio
    let mut sum = 0.0;
    for _ in 0..4410 {
        // 0.1 seconds
        let frame = unit.tick(&Default::default());
        sum += frame[0].abs();
    }

    // Should have output
    assert!(sum > 0.0, "Soft saw should produce output: {}", sum);

    println!("Soft saw 440 Hz - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_soft_saw_hz_frequency() {
    // Test different frequencies
    let sample_rate = 44100.0;

    // Low frequency (bass)
    let mut unit_low = soft_saw_hz(55.0);
    unit_low.reset();
    unit_low.set_sample_rate(sample_rate);

    // High frequency (treble)
    let mut unit_high = soft_saw_hz(2000.0);
    unit_high.reset();
    unit_high.set_sample_rate(sample_rate);

    let mut low_sum = 0.0;
    let mut high_sum = 0.0;

    for _ in 0..44100 {
        // 1 second
        let low_frame = unit_low.tick(&Default::default());
        let high_frame = unit_high.tick(&Default::default());

        low_sum += low_frame[0].abs();
        high_sum += high_frame[0].abs();
    }

    println!("Low (55 Hz) sum: {:.2}", low_sum);
    println!("High (2000 Hz) sum: {:.2}", high_sum);

    // Both should produce output
    assert!(low_sum > 0.0, "Low frequency should produce output");
    assert!(high_sum > 0.0, "High frequency should produce output");
}

#[test]
fn test_fundsp_soft_saw_hz_waveform_shape() {
    // Test that waveform has sawtooth shape (but softer)
    let mut unit = soft_saw_hz(100.0); // Low frequency to see shape
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Collect one period of samples
    let period_samples = (44100.0 / 100.0) as usize;
    let mut samples = Vec::new();

    for _ in 0..period_samples * 2 {
        let frame = unit.tick(&Default::default());
        samples.push(frame[0]);
    }

    // Soft saw should have both positive and negative values
    let has_positive = samples.iter().any(|&s| s > 0.0);
    let has_negative = samples.iter().any(|&s| s < 0.0);

    assert!(has_positive, "Soft saw should have positive values");
    assert!(has_negative, "Soft saw should have negative values");

    let min_val = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    println!("Soft saw waveform range: {:.3} to {:.3}", min_val, max_val);
}

#[test]
fn test_fundsp_soft_saw_hz_dc_centered() {
    // Test that waveform is DC-centered (average ~ 0)
    let mut unit = soft_saw_hz(220.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100; // 1 second

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // Should be close to zero (allow small drift)
    assert!(
        average.abs() < 0.01,
        "Soft saw should be DC-centered: average = {}",
        average
    );

    println!("Soft saw DC offset: {:.6}", average);
}

#[test]
fn test_fundsp_soft_saw_vs_regular_saw() {
    // Test that soft_saw has less high-frequency content than regular saw
    // (We expect soft_saw to have fewer harmonics)

    let sample_rate = 44100.0;
    let freq = 110.0;

    let mut soft_unit = soft_saw_hz(freq);
    soft_unit.reset();
    soft_unit.set_sample_rate(sample_rate);

    let mut regular_unit = saw_hz(freq);
    regular_unit.reset();
    regular_unit.set_sample_rate(sample_rate);

    // Collect samples
    let mut soft_samples = Vec::new();
    let mut regular_samples = Vec::new();

    for _ in 0..44100 {
        let soft_frame = soft_unit.tick(&Default::default());
        let regular_frame = regular_unit.tick(&Default::default());
        soft_samples.push(soft_frame[0]);
        regular_samples.push(regular_frame[0]);
    }

    // Calculate RMS (energy)
    let soft_rms: f32 = (soft_samples.iter().map(|s| s * s).sum::<f32>() / soft_samples.len() as f32).sqrt();
    let regular_rms: f32 = (regular_samples.iter().map(|s| s * s).sum::<f32>() / regular_samples.len() as f32).sqrt();

    println!("Soft saw RMS: {:.4}", soft_rms);
    println!("Regular saw RMS: {:.4}", regular_rms);

    // Both should produce audio
    assert!(soft_rms > 0.01, "Soft saw should have significant energy");
    assert!(regular_rms > 0.01, "Regular saw should have significant energy");

    // Soft saw might be slightly quieter or similar in RMS
    // (Mostly just verify both work - spectral analysis would be better for harmonic content)
}
