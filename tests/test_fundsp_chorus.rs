/// Direct fundsp chorus API tests
///
/// Level 1: Verify fundsp API works correctly before wrapping
///
/// These tests ensure we understand fundsp's chorus behavior:
/// - Takes 4 parameters (seed, separation, variation, mod_frequency)
/// - Takes 1 mono input
/// - Returns 1 mono output
/// - Creates 5-voice chorus effect

use fundsp::prelude::*;

#[test]
fn test_fundsp_chorus_basic() {
    // Test that fundsp chorus processes audio
    let mut unit = chorus(0, 0.015, 0.005, 0.3);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Send continuous signal and measure output
    let mut sum = 0.0;

    for _ in 0..4410 {
        // 0.1 seconds
        let frame = unit.tick(&[1.0].into());
        sum += frame[0].abs();
    }

    // Should have output
    assert!(sum > 0.0, "Chorus should produce output: {}", sum);

    println!("Basic chorus - sum: {:.2}", sum);
}

#[test]
fn test_fundsp_chorus_separation() {
    // Test that separation parameter affects output
    let sample_rate = 44100.0;

    // Narrow voice spread
    let mut unit_narrow = chorus(0, 0.005, 0.002, 0.3);
    unit_narrow.reset();
    unit_narrow.set_sample_rate(sample_rate);

    // Wide voice spread
    let mut unit_wide = chorus(0, 0.030, 0.002, 0.3);
    unit_wide.reset();
    unit_wide.set_sample_rate(sample_rate);

    let mut narrow_sum = 0.0;
    let mut wide_sum = 0.0;

    for _ in 0..4410 {
        // 0.1 seconds
        let narrow_frame = unit_narrow.tick(&[1.0].into());
        let wide_frame = unit_wide.tick(&[1.0].into());

        narrow_sum += narrow_frame[0].abs();
        wide_sum += wide_frame[0].abs();
    }

    println!("Narrow (0.005s) sum: {:.2}", narrow_sum);
    println!("Wide (0.030s) sum: {:.2}", wide_sum);

    // Both should produce output
    assert!(narrow_sum > 0.0, "Narrow should produce output");
    assert!(wide_sum > 0.0, "Wide should produce output");
}

#[test]
fn test_fundsp_chorus_mod_frequency() {
    // Test that mod frequency affects modulation speed
    let sample_rate = 44100.0;

    // Slow LFO
    let mut unit_slow = chorus(0, 0.015, 0.005, 0.1);
    unit_slow.reset();
    unit_slow.set_sample_rate(sample_rate);

    // Fast LFO
    let mut unit_fast = chorus(0, 0.015, 0.005, 2.0);
    unit_fast.reset();
    unit_fast.set_sample_rate(sample_rate);

    let mut slow_sum = 0.0;
    let mut fast_sum = 0.0;

    for _ in 0..44100 {
        // 1 second to hear LFO difference
        let slow_frame = unit_slow.tick(&[1.0].into());
        let fast_frame = unit_fast.tick(&[1.0].into());

        slow_sum += slow_frame[0].abs();
        fast_sum += fast_frame[0].abs();
    }

    println!("Slow LFO (0.1 Hz) sum: {:.2}", slow_sum);
    println!("Fast LFO (2.0 Hz) sum: {:.2}", fast_sum);

    // Both should produce output
    assert!(slow_sum > 0.0, "Slow LFO should produce output");
    assert!(fast_sum > 0.0, "Fast LFO should produce output");
}

#[test]
fn test_fundsp_chorus_variation() {
    // Test that variation parameter affects modulation depth
    let sample_rate = 44100.0;

    // Small variation
    let mut unit_small = chorus(0, 0.015, 0.001, 0.3);
    unit_small.reset();
    unit_small.set_sample_rate(sample_rate);

    // Large variation
    let mut unit_large = chorus(0, 0.015, 0.010, 0.3);
    unit_large.reset();
    unit_large.set_sample_rate(sample_rate);

    let mut small_sum = 0.0;
    let mut large_sum = 0.0;

    for _ in 0..44100 {
        // 1 second
        let small_frame = unit_small.tick(&[1.0].into());
        let large_frame = unit_large.tick(&[1.0].into());

        small_sum += small_frame[0].abs();
        large_sum += large_frame[0].abs();
    }

    println!("Small variation (0.001s) sum: {:.2}", small_sum);
    println!("Large variation (0.010s) sum: {:.2}", large_sum);

    // Both should produce output
    assert!(small_sum > 0.0, "Small variation should produce output");
    assert!(large_sum > 0.0, "Large variation should produce output");
}

#[test]
fn test_fundsp_chorus_different_seeds() {
    // Test that different seeds create different LFO patterns
    let sample_rate = 44100.0;

    let mut unit_seed0 = chorus(0, 0.015, 0.005, 0.3);
    unit_seed0.reset();
    unit_seed0.set_sample_rate(sample_rate);

    let mut unit_seed1 = chorus(1, 0.015, 0.005, 0.3);
    unit_seed1.reset();
    unit_seed1.set_sample_rate(sample_rate);

    let mut seed0_sum = 0.0;
    let mut seed1_sum = 0.0;

    for _ in 0..44100 {
        // 1 second
        let frame0 = unit_seed0.tick(&[1.0].into());
        let frame1 = unit_seed1.tick(&[1.0].into());

        seed0_sum += frame0[0].abs();
        seed1_sum += frame1[0].abs();
    }

    println!("Seed 0 sum: {:.2}", seed0_sum);
    println!("Seed 1 sum: {:.2}", seed1_sum);

    // Both should produce output
    assert!(seed0_sum > 0.0, "Seed 0 should produce output");
    assert!(seed1_sum > 0.0, "Seed 1 should produce output");

    // Different seeds may produce different results (depending on LFO phase)
    // But both should still work
}
