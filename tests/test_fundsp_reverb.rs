/// Direct fundsp reverb_stereo API tests
///
/// Level 1: Verify fundsp API works correctly before wrapping
///
/// These tests ensure we understand fundsp's reverb_stereo behavior:
/// - Takes 1 mono input
/// - Takes 2 parameters (wet 0-1, time in seconds)
/// - Returns 2 stereo outputs (left and right)
/// - Has reverb tail that persists after input stops
use fundsp::prelude::*;

#[test]
fn test_fundsp_reverb_stereo_basic() {
    // Test that fundsp reverb_stereo processes audio
    let mut unit = reverb_stereo(0.5, 1.0, 0.5); // wet, time, diffusion
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Send continuous signal and measure output
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;

    for _ in 0..4410 {
        // 0.1 seconds
        let frame = unit.tick(&[1.0, 1.0].into());
        left_sum += frame[0].abs();
        right_sum += frame[1].abs();
    }

    // Should have output on both channels
    assert!(
        left_sum > 0.0,
        "Left channel should have output: {}",
        left_sum
    );
    assert!(
        right_sum > 0.0,
        "Right channel should have output: {}",
        right_sum
    );

    println!(
        "Basic reverb - L sum: {:.2}, R sum: {:.2}",
        left_sum, right_sum
    );
}

#[test]
fn test_fundsp_reverb_stereo_tail() {
    // Test reverb tail after impulse
    let mut unit = reverb_stereo(0.8, 2.0, 0.5); // High wet, 2 second time, diffusion
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Send brief impulse (10 samples) - stereo input
    for _ in 0..10 {
        unit.tick(&[1.0, 1.0].into());
    }

    // Measure reverb tail over 1 second
    let mut tail_sum = 0.0;
    for _ in 0..44100 {
        let frame = unit.tick(&[0.0, 0.0].into()); // No input
        tail_sum += frame[0].abs() + frame[1].abs();
    }

    // Should have significant reverb tail
    assert!(
        tail_sum > 1.0,
        "Reverb should have significant tail: {}",
        tail_sum
    );

    println!("Reverb tail sum (1 second after impulse): {:.2}", tail_sum);
}

#[test]
fn test_fundsp_reverb_stereo_wet_dry() {
    // Test that wet parameter affects mix
    let sample_rate = 44100.0;

    // Dry (wet=0.0) - should be mostly original signal
    let mut unit_dry = reverb_stereo(0.0, 1.0, 0.5);
    unit_dry.reset();
    unit_dry.set_sample_rate(sample_rate);

    // Wet (wet=1.0) - should be mostly reverb
    let mut unit_wet = reverb_stereo(1.0, 1.0, 0.5);
    unit_wet.reset();
    unit_wet.set_sample_rate(sample_rate);

    let mut dry_sum = 0.0;
    let mut wet_sum = 0.0;

    // Send continuous signal (stereo input)
    for _ in 0..4410 {
        // 0.1 seconds
        let dry_frame = unit_dry.tick(&[1.0, 1.0].into());
        let wet_frame = unit_wet.tick(&[1.0, 1.0].into());

        dry_sum += dry_frame[0].abs() + dry_frame[1].abs();
        wet_sum += wet_frame[0].abs() + wet_frame[1].abs();
    }

    println!("Dry (wet=0.0) sum: {:.2}", dry_sum);
    println!("Wet (wet=1.0) sum: {:.2}", wet_sum);

    // Both should produce output
    assert!(dry_sum > 0.0, "Dry should produce output");
    assert!(wet_sum > 0.0, "Wet should produce output");

    // They should be different
    assert!(
        (dry_sum - wet_sum).abs() / dry_sum > 0.1,
        "Wet and dry should differ significantly"
    );
}

#[test]
fn test_fundsp_reverb_stereo_time_parameter() {
    // Test that time parameter affects reverb length
    let sample_rate = 44100.0;

    // Short reverb
    let mut unit_short = reverb_stereo(0.5, 0.5, 0.5);
    unit_short.reset();
    unit_short.set_sample_rate(sample_rate);

    // Long reverb
    let mut unit_long = reverb_stereo(0.5, 3.0, 0.5);
    unit_long.reset();
    unit_long.set_sample_rate(sample_rate);

    // Send impulse to both (stereo input)
    unit_short.tick(&[1.0, 1.0].into());
    unit_long.tick(&[1.0, 1.0].into());

    // Measure tail after 1 second
    let mut short_tail = 0.0;
    let mut long_tail = 0.0;

    for _ in 0..44100 {
        let short_frame = unit_short.tick(&[0.0, 0.0].into());
        let long_frame = unit_long.tick(&[0.0, 0.0].into());

        short_tail += short_frame[0].abs() + short_frame[1].abs();
        long_tail += long_frame[0].abs() + long_frame[1].abs();
    }

    println!("Short reverb (0.5s) tail: {:.2}", short_tail);
    println!("Long reverb (3.0s) tail: {:.2}", long_tail);

    // Long reverb should have more tail remaining after 1 second
    assert!(
        long_tail > short_tail,
        "Longer time should have more tail: long={:.2}, short={:.2}",
        long_tail,
        short_tail
    );
}

#[test]
fn test_fundsp_reverb_stereo_stereo_output() {
    // Verify that left and right channels are different (stereo image)
    let mut unit = reverb_stereo(0.7, 1.5, 0.5);
    unit.reset();
    unit.set_sample_rate(44100.0);

    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    let mut difference_sum = 0.0;

    // Send signal (stereo input)
    for i in 0..4410 {
        // Varying input to get interesting reverb
        let input = ((i as f32 * 220.0 / 44100.0).sin() * 0.5 + 0.5).sin();
        let frame = unit.tick(&[input, input].into());

        left_sum += frame[0].abs();
        right_sum += frame[1].abs();
        difference_sum += (frame[0] - frame[1]).abs();
    }

    println!("Left channel sum: {:.2}", left_sum);
    println!("Right channel sum: {:.2}", right_sum);
    println!("L/R difference sum: {:.2}", difference_sum);

    // Both channels should have energy
    assert!(left_sum > 0.01, "Left channel should have energy");
    assert!(right_sum > 0.01, "Right channel should have energy");

    // Channels should differ (stereo image)
    assert!(
        difference_sum > 0.01,
        "Left and right should differ (stereo): {}",
        difference_sum
    );
}
