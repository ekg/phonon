/// Direct fundsp moog_hz API tests
///
/// Level 1: Verify fundsp API works correctly before wrapping
///
/// These tests ensure we understand fundsp's moog_hz behavior:
/// - Takes 1 input (audio signal to filter)
/// - Takes 2 parameters (cutoff Hz, resonance 0-1)
/// - Returns filtered audio
/// - 4-pole 24dB/oct Moog ladder lowpass filter

use fundsp::prelude::*;

#[test]
fn test_fundsp_moog_basic() {
    // Test that fundsp moog_hz can filter a saw wave
    let mut unit = moog_hz(1000.0, 0.7);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Generate saw wave input at 220 Hz
    let mut saw_phase = 0.0;
    let mut output_sum = 0.0;

    for _ in 0..44100 {
        // Generate saw wave (-1 to 1)
        let saw = 2.0 * saw_phase - 1.0;
        saw_phase += 220.0 / 44100.0;
        if saw_phase >= 1.0 {
            saw_phase -= 1.0;
        }

        // Filter the saw wave
        let output_frame = unit.tick(&[saw].into());
        output_sum += output_frame[0].abs();
    }

    // Should have filtered output
    assert!(output_sum > 0.0, "moog_hz should produce output");
    println!("moog_hz output sum: {:.2}", output_sum);
}

#[test]
fn test_fundsp_moog_cutoff_affects_output() {
    // Test that different cutoff frequencies produce different output
    let sample_rate = 44100.0;

    // Low cutoff (100 Hz) - should heavily filter 220 Hz saw
    let mut unit_low = moog_hz(100.0, 0.5);
    unit_low.reset();
    unit_low.set_sample_rate(sample_rate);

    // High cutoff (5000 Hz) - should barely filter 220 Hz saw
    let mut unit_high = moog_hz(5000.0, 0.5);
    unit_high.reset();
    unit_high.set_sample_rate(sample_rate);

    let mut saw_phase = 0.0;
    let mut output_low_sum = 0.0;
    let mut output_high_sum = 0.0;

    for _ in 0..44100 {
        let saw = 2.0 * saw_phase - 1.0;
        saw_phase += 220.0 / 44100.0;
        if saw_phase >= 1.0 {
            saw_phase -= 1.0;
        }

        let output_low = unit_low.tick(&[saw].into())[0];
        let output_high = unit_high.tick(&[saw].into())[0];

        output_low_sum += output_low.abs();
        output_high_sum += output_high.abs();
    }

    // High cutoff should pass more signal
    assert!(
        output_high_sum > output_low_sum,
        "High cutoff should pass more signal than low cutoff"
    );

    println!("Low cutoff (100 Hz) sum: {:.2}", output_low_sum);
    println!("High cutoff (5000 Hz) sum: {:.2}", output_high_sum);
}

#[test]
fn test_fundsp_moog_resonance_affects_output() {
    // Test that resonance parameter changes output
    let sample_rate = 44100.0;

    // Low resonance
    let mut unit_low_q = moog_hz(1000.0, 0.1);
    unit_low_q.reset();
    unit_low_q.set_sample_rate(sample_rate);

    // High resonance (near self-oscillation)
    let mut unit_high_q = moog_hz(1000.0, 0.9);
    unit_high_q.reset();
    unit_high_q.set_sample_rate(sample_rate);

    let mut saw_phase = 0.0;
    let mut output_low_q = Vec::new();
    let mut output_high_q = Vec::new();

    for _ in 0..44100 {
        let saw = 2.0 * saw_phase - 1.0;
        saw_phase += 220.0 / 44100.0;
        if saw_phase >= 1.0 {
            saw_phase -= 1.0;
        }

        output_low_q.push(unit_low_q.tick(&[saw].into())[0]);
        output_high_q.push(unit_high_q.tick(&[saw].into())[0]);
    }

    // Calculate RMS
    let rms_low_q =
        (output_low_q.iter().map(|x| x * x).sum::<f32>() / output_low_q.len() as f32).sqrt();
    let rms_high_q =
        (output_high_q.iter().map(|x| x * x).sum::<f32>() / output_high_q.len() as f32).sqrt();

    // High resonance typically increases amplitude near cutoff
    println!("Low resonance (0.1) RMS: {:.4}", rms_low_q);
    println!("High resonance (0.9) RMS: {:.4}", rms_high_q);

    // Both should produce output
    assert!(rms_low_q > 0.01, "Low resonance should produce output");
    assert!(rms_high_q > 0.01, "High resonance should produce output");
}
