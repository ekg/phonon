/// Direct fundsp noise API tests
///
/// Level 1: Verify fundsp API works correctly before wrapping
///
/// These tests ensure we understand fundsp's noise behavior:
/// - Takes 0 parameters
/// - Takes 0 audio inputs (generator)
/// - Returns 1 mono output
/// - White noise (equal energy across all frequencies)

use fundsp::prelude::*;

#[test]
fn test_fundsp_noise_basic() {
    // Test that fundsp noise generates audio
    let mut unit = noise();
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
    assert!(sum > 0.0, "Noise should produce output: {}", sum);

    println!("Noise - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_noise_range() {
    // Test that noise values are in range [-1, 1]
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut min_val = f32::INFINITY;
    let mut max_val = f32::NEG_INFINITY;

    for _ in 0..44100 {
        // 1 second
        let frame = unit.tick(&Default::default());
        let sample = frame[0];

        min_val = min_val.min(sample);
        max_val = max_val.max(sample);
    }

    // Should be roughly in [-1, 1] range
    assert!(
        min_val >= -1.0 && min_val < 0.0,
        "Noise should have negative values: {}",
        min_val
    );
    assert!(
        max_val > 0.0 && max_val <= 1.0,
        "Noise should have positive values: {}",
        max_val
    );

    println!("Noise range: {:.3} to {:.3}", min_val, max_val);
}

#[test]
fn test_fundsp_noise_distribution() {
    // Test that noise has roughly equal positive/negative samples
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut positive_count = 0;
    let mut negative_count = 0;

    for _ in 0..44100 {
        let frame = unit.tick(&Default::default());
        let sample = frame[0];

        if sample > 0.0 {
            positive_count += 1;
        } else if sample < 0.0 {
            negative_count += 1;
        }
    }

    let total = positive_count + negative_count;
    let positive_ratio = positive_count as f32 / total as f32;

    // Should be roughly 50/50 (allow 40-60% range for randomness)
    assert!(
        positive_ratio > 0.4 && positive_ratio < 0.6,
        "Noise should be roughly balanced: {:.2}% positive",
        positive_ratio * 100.0
    );

    println!(
        "Noise distribution: {:.1}% positive, {:.1}% negative",
        positive_ratio * 100.0,
        (1.0 - positive_ratio) * 100.0
    );
}

#[test]
fn test_fundsp_noise_dc_centered() {
    // Test that average is close to 0 (DC-centered)
    let mut unit = noise();
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut sum = 0.0;
    let num_samples = 44100; // 1 second

    for _ in 0..num_samples {
        let frame = unit.tick(&Default::default());
        sum += frame[0];
    }

    let average = sum / num_samples as f32;

    // DC offset should be very small (noise is random, but should average to ~0)
    assert!(
        average.abs() < 0.1,
        "Noise should be roughly DC-centered, average: {}",
        average
    );

    println!("Noise DC offset: {:.6}", average);
}
