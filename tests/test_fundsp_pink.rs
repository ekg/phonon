/// Direct fundsp pink noise API tests
///
/// Level 1: Verify fundsp pink() generates pink noise correctly
use fundsp::prelude::*;

#[test]
fn test_fundsp_pink_basic() {
    // Test that fundsp pink generates audio
    let mut unit = pink::<f32>();
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
    assert!(sum > 0.0, "Pink noise should produce output: {}", sum);

    println!("Pink noise - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_pink_range() {
    // Test that pink noise values are in reasonable range
    let mut unit = pink::<f32>();
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

    // Pink noise should be roughly in [-2, 2] range (wider than white noise)
    assert!(
        min_val >= -3.0 && min_val < 0.0,
        "Pink noise should have negative values: {}",
        min_val
    );
    assert!(
        max_val > 0.0 && max_val <= 3.0,
        "Pink noise should have positive values: {}",
        max_val
    );

    println!("Pink noise range: {:.3} to {:.3}", min_val, max_val);
}

#[test]
fn test_fundsp_pink_distribution() {
    // Test that pink noise has roughly equal positive/negative samples
    let mut unit = pink::<f32>();
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
        "Pink noise should be roughly balanced: {:.2}% positive",
        positive_ratio * 100.0
    );

    println!(
        "Pink noise distribution: {:.1}% positive, {:.1}% negative",
        positive_ratio * 100.0,
        (1.0 - positive_ratio) * 100.0
    );
}

#[test]
fn test_fundsp_pink_dc_centered() {
    // Test that average is close to 0 (DC-centered)
    let mut unit = pink::<f32>();
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
        "Pink noise should be roughly DC-centered, average: {}",
        average
    );

    println!("Pink noise DC offset: {:.6}", average);
}
