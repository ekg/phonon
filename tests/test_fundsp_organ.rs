/// Test fundsp organ_hz integration
///
/// This is the first fundsp UGen wrapper test following the Study → Implement → Test workflow
use fundsp::prelude::*;

#[test]
fn test_fundsp_organ_basic() {
    // Create a simple fundsp organ oscillator at 440 Hz
    let mut unit = organ_hz(440.0);

    // fundsp units need sample rate
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Process a few samples
    // fundsp's tick() takes a Frame input and returns a Frame output
    let mut sum = 0.0;
    for _ in 0..44100 {
        // organ_hz has 0 inputs, so we pass an empty frame
        let output_frame = unit.tick(&Default::default());
        // Extract the first (and only) output
        sum += output_frame[0].abs();
    }

    // Should produce non-zero output
    assert!(sum > 0.0, "organ_hz should produce sound");
}

#[test]
fn test_fundsp_organ_inputs_outputs() {
    let unit = organ_hz(440.0);

    // Check the input/output configuration
    println!("organ_hz inputs: {}", unit.inputs());
    println!("organ_hz outputs: {}", unit.outputs());

    // organ_hz should be a generator (0 inputs, 1 output)
    assert_eq!(unit.inputs(), 0, "organ_hz is a generator with no inputs");
    assert_eq!(unit.outputs(), 1, "organ_hz produces mono output");
}

#[test]
fn test_fundsp_organ_rms() {
    let mut unit = organ_hz(440.0);
    unit.reset();
    unit.set_sample_rate(44100.0);

    // Generate 1 second of audio
    let mut samples = Vec::new();

    for _ in 0..44100 {
        let output_frame = unit.tick(&Default::default());
        samples.push(output_frame[0]);
    }

    // Calculate RMS
    let rms = (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt();

    // Should have reasonable amplitude (between 0.1 and 1.0)
    assert!(rms > 0.1, "RMS too low: {}", rms);
    assert!(rms < 1.0, "RMS too high: {}", rms);

    println!("organ_hz(440.0) RMS: {:.4}", rms);
}
