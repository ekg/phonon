/// Direct fundsp saw_hz API tests
///
/// Level 1: Verify fundsp API works correctly before wrapping
///
/// These tests ensure we understand fundsp's saw_hz behavior:
/// - Takes 1 parameter (frequency)
/// - Takes 0 audio inputs (generator)
/// - Returns 1 mono output
/// - Bandlimited sawtooth waveform
use fundsp::prelude::*;

#[test]
fn test_fundsp_saw_hz_basic() {
    // Test that fundsp saw_hz generates audio
    let mut unit = saw_hz(440.0);
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
    assert!(sum > 0.0, "Saw should produce output: {}", sum);

    println!("Saw 440 Hz - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_saw_hz_frequency() {
    // Test different frequencies
    let sample_rate = 44100.0;

    // Low frequency (bass)
    let mut unit_low = saw_hz(55.0);
    unit_low.reset();
    unit_low.set_sample_rate(sample_rate);

    // High frequency (treble)
    let mut unit_high = saw_hz(2000.0);
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
fn test_fundsp_saw_hz_waveform_shape() {
    // Test that waveform has sawtooth shape
    let mut unit = saw_hz(100.0); // Low frequency to see shape
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Collect one period of samples
    let period_samples = (44100.0 / 100.0) as usize;
    let mut samples = Vec::new();

    for _ in 0..period_samples * 2 {
        let frame = unit.tick(&Default::default());
        samples.push(frame[0]);
    }

    // Sawtooth should have both positive and negative values
    let has_positive = samples.iter().any(|&s| s > 0.0);
    let has_negative = samples.iter().any(|&s| s < 0.0);

    assert!(has_positive, "Sawtooth should have positive values");
    assert!(has_negative, "Sawtooth should have negative values");

    let min_val = samples.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    println!("Saw waveform range: {:.3} to {:.3}", min_val, max_val);
}

#[test]
fn test_fundsp_saw_hz_dc_centered() {
    // Test that waveform is DC-centered (average ~ 0)
    let mut unit = saw_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100; // 1 second

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // DC offset should be very small (close to 0)
    assert!(
        average.abs() < 0.01,
        "Saw should be DC-centered, average: {}",
        average
    );

    println!("Saw DC offset: {:.6}", average);
}
