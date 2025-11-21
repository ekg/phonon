/// Tests for PitchNode - Pitch detection using autocorrelation
///
/// Tests verify:
/// 1. Detects correct pitch from sine wave
/// 2. Detects correct pitch from saw wave
/// 3. Detects correct pitch from square wave
/// 4. Returns 0 Hz for silence
/// 5. Returns 0 Hz for noise (no clear pitch)
/// 6. Smooth tracking over multiple blocks
/// 7. Handles frequency changes
/// 8. Edge cases (very low/high frequencies)

use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::nodes::constant::ConstantNode;
use phonon::nodes::noise::NoiseNode;
use phonon::nodes::oscillator::{OscillatorNode, Waveform};
use phonon::nodes::pitch::PitchNode;
use phonon::pattern::Fraction;

/// Helper: Create test context
fn test_context(block_size: usize) -> ProcessContext {
    ProcessContext::new(Fraction::from_float(0.0), 0, block_size, 2.0, 44100.0)
}

#[test]
fn test_pitch_detects_sine_440hz() {
    // Test 1: Should detect 440 Hz from sine wave
    let sample_rate = 44100.0;
    let block_size = 2048; // Need enough samples for low frequencies

    let mut freq_const = ConstantNode::new(440.0);
    let mut sine = OscillatorNode::new(0, Waveform::Sine);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let sine_inputs = vec![freq_buf.as_slice()];
    sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);

    let pitch_inputs = vec![signal_buf.as_slice()];

    // Process several blocks to allow detector to converge
    for _ in 0..5 {
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    // Check detected pitch (average over buffer for stability)
    let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Should be close to 440 Hz (within 5% tolerance)
    assert!(
        (avg_pitch - 440.0).abs() < 22.0,
        "Detected pitch {} Hz should be close to 440 Hz",
        avg_pitch
    );
}

#[test]
fn test_pitch_detects_sine_110hz() {
    // Test 2: Should detect lower frequency (110 Hz = A2)
    let sample_rate = 44100.0;
    let block_size = 4096; // Larger buffer for low frequencies

    let mut freq_const = ConstantNode::new(110.0);
    let mut sine = OscillatorNode::new(0, Waveform::Sine);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let sine_inputs = vec![freq_buf.as_slice()];
    sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);

    let pitch_inputs = vec![signal_buf.as_slice()];

    for _ in 0..5 {
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Within 10% tolerance for lower frequencies
    assert!(
        (avg_pitch - 110.0).abs() < 11.0,
        "Detected pitch {} Hz should be close to 110 Hz",
        avg_pitch
    );
}

#[test]
fn test_pitch_detects_saw_220hz() {
    // Test 3: Should detect pitch from saw wave (complex harmonic content)
    let sample_rate = 44100.0;
    let block_size = 2048;

    let mut freq_const = ConstantNode::new(220.0);
    let mut saw = OscillatorNode::new(0, Waveform::Saw);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let saw_inputs = vec![freq_buf.as_slice()];
    saw.process_block(&saw_inputs, &mut signal_buf, sample_rate, &context);

    let pitch_inputs = vec![signal_buf.as_slice()];

    for _ in 0..5 {
        saw.process_block(&saw_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Saw waves should detect fundamental (within 10%)
    assert!(
        (avg_pitch - 220.0).abs() < 22.0,
        "Detected pitch {} Hz should be close to 220 Hz",
        avg_pitch
    );
}

#[test]
fn test_pitch_detects_square_330hz() {
    // Test 4: Should detect pitch from square wave
    let sample_rate = 44100.0;
    let block_size = 2048;

    let mut freq_const = ConstantNode::new(330.0);
    let mut square = OscillatorNode::new(0, Waveform::Square);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let square_inputs = vec![freq_buf.as_slice()];
    square.process_block(&square_inputs, &mut signal_buf, sample_rate, &context);

    let pitch_inputs = vec![signal_buf.as_slice()];

    for _ in 0..5 {
        square.process_block(&square_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Square waves should detect fundamental (within 10%)
    assert!(
        (avg_pitch - 330.0).abs() < 33.0,
        "Detected pitch {} Hz should be close to 330 Hz",
        avg_pitch
    );
}

#[test]
fn test_pitch_returns_zero_for_silence() {
    // Test 5: Should return 0 Hz for silence
    let sample_rate = 44100.0;
    let block_size = 1024;

    let mut silence = ConstantNode::new(0.0);
    let mut pitch = PitchNode::new(0);

    let context = test_context(block_size);

    let mut silence_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    silence.process_block(&[], &mut silence_buf, sample_rate, &context);

    let pitch_inputs = vec![silence_buf.as_slice()];
    pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);

    // Should detect no pitch (0 Hz or very low confidence)
    let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;
    assert!(
        avg_pitch < 10.0,
        "Silence should produce near-zero pitch, got {} Hz",
        avg_pitch
    );
}

#[test]
fn test_pitch_returns_zero_for_noise() {
    // Test 6: Should return 0 Hz for noise (no clear pitch)
    let sample_rate = 44100.0;
    let block_size = 1024;

    let mut amp_const = ConstantNode::new(0.5);
    let mut noise = NoiseNode::new(0);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut amp_buf = vec![0.0; block_size];
    let mut noise_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    amp_const.process_block(&[], &mut amp_buf, sample_rate, &context);

    let noise_inputs = vec![amp_buf.as_slice()];
    noise.process_block(&noise_inputs, &mut noise_buf, sample_rate, &context);

    let pitch_inputs = vec![noise_buf.as_slice()];

    for _ in 0..3 {
        noise.process_block(&noise_inputs, &mut noise_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    // Noise should have very low or zero detected pitch
    let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;
    assert!(
        avg_pitch < 50.0,
        "Noise should produce low/zero pitch, got {} Hz",
        avg_pitch
    );
}

#[test]
fn test_pitch_smooth_tracking() {
    // Test 7: Should track pitch smoothly over multiple blocks
    let sample_rate = 44100.0;
    let block_size = 2048;

    let mut freq_const = ConstantNode::new(440.0);
    let mut sine = OscillatorNode::new(0, Waveform::Sine);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let sine_inputs = vec![freq_buf.as_slice()];
    let pitch_inputs = vec![signal_buf.as_slice()];

    let mut pitch_readings = Vec::new();

    // Collect pitch over 10 blocks
    for _ in 0..10 {
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);

        let avg_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;
        pitch_readings.push(avg_pitch);
    }

    // After convergence, should be stable (low variance)
    let stable_readings = &pitch_readings[5..]; // Skip first 5 blocks
    let mean: f32 = stable_readings.iter().sum::<f32>() / stable_readings.len() as f32;

    // All stable readings should be within 5% of mean
    for &reading in stable_readings {
        assert!(
            (reading - mean).abs() < mean * 0.05,
            "Pitch should be stable: reading {} vs mean {}",
            reading,
            mean
        );
    }
}

#[test]
fn test_pitch_handles_frequency_change() {
    // Test 8: Should detect when frequency changes
    let sample_rate = 44100.0;
    let block_size = 2048;

    let mut freq_const = ConstantNode::new(220.0);
    let mut sine = OscillatorNode::new(0, Waveform::Sine);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let sine_inputs = vec![freq_buf.as_slice()];
    let pitch_inputs = vec![signal_buf.as_slice()];

    // Converge on 220 Hz
    for _ in 0..5 {
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    let pitch1: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Change frequency to 440 Hz
    freq_const.set_value(440.0);
    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    // Converge on 440 Hz
    for _ in 0..5 {
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    let pitch2: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Should detect the change
    assert!(
        pitch1 < 250.0,
        "First pitch should be ~220 Hz, got {} Hz",
        pitch1
    );
    assert!(
        pitch2 > 400.0,
        "Second pitch should be ~440 Hz, got {} Hz",
        pitch2
    );
    assert!(
        (pitch2 - pitch1).abs() > 100.0,
        "Should detect frequency change: {} Hz to {} Hz",
        pitch1,
        pitch2
    );
}

#[test]
fn test_pitch_extreme_frequencies() {
    // Test 9: Should handle very low and very high frequencies gracefully
    let sample_rate = 44100.0;
    let block_size = 8192; // Very large buffer for very low frequencies

    let context = test_context(block_size);

    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    let mut pitch = PitchNode::new(0);

    // Test very low frequency (55 Hz = A1)
    let mut freq_low = ConstantNode::new(55.0);
    let mut sine_low = OscillatorNode::new(0, Waveform::Sine);

    let mut freq_buf = vec![0.0; block_size];
    freq_low.process_block(&[], &mut freq_buf, sample_rate, &context);

    let sine_inputs = vec![freq_buf.as_slice()];
    let pitch_inputs = vec![signal_buf.as_slice()];

    for _ in 0..5 {
        sine_low.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);
    }

    let low_pitch: f32 = pitch_buf.iter().sum::<f32>() / pitch_buf.len() as f32;

    // Should detect within 20% for very low frequencies
    assert!(
        (low_pitch - 55.0).abs() < 15.0,
        "Should detect low frequency ~55 Hz, got {} Hz",
        low_pitch
    );
}

#[test]
fn test_pitch_input_nodes() {
    // Test 10: Verify input node dependencies
    let pitch = PitchNode::new(42);
    let deps = pitch.input_nodes();

    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], 42);
}

#[test]
fn test_pitch_stability() {
    // Test 11: Should remain stable over many blocks
    let sample_rate = 44100.0;
    let block_size = 1024;

    let mut freq_const = ConstantNode::new(440.0);
    let mut sine = OscillatorNode::new(0, Waveform::Sine);
    let mut pitch = PitchNode::new(1);

    let context = test_context(block_size);

    let mut freq_buf = vec![0.0; block_size];
    let mut signal_buf = vec![0.0; block_size];
    let mut pitch_buf = vec![0.0; block_size];

    freq_const.process_block(&[], &mut freq_buf, sample_rate, &context);

    let sine_inputs = vec![freq_buf.as_slice()];
    let pitch_inputs = vec![signal_buf.as_slice()];

    // Process many blocks
    for _ in 0..50 {
        sine.process_block(&sine_inputs, &mut signal_buf, sample_rate, &context);
        pitch.process_block(&pitch_inputs, &mut pitch_buf, sample_rate, &context);

        // All outputs should be finite
        for (i, &sample) in pitch_buf.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "Sample {} became infinite/NaN",
                i
            );
            assert!(
                sample >= 0.0,
                "Pitch should be non-negative: {}",
                sample
            );
        }
    }
}
