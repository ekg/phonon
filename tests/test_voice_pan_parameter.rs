use phonon::sample_loader::StereoSample;
use phonon::voice_manager::{Voice, VoiceManager};
use std::sync::Arc;

// Helper: Process enough samples to get past the envelope attack phase
// Default attack is 1ms = ~44 samples at 44100Hz, so process 100 to be safe
fn skip_attack_phase(voice: &mut Voice, samples: usize) {
    for _ in 0..samples {
        let _ = voice.process_stereo();
    }
}

#[test]
fn test_voice_center_pan() {
    // Test that pan=0.0 (center) produces equal left and right channels
    let mut voice = Voice::new();

    // Use a longer constant sample to allow envelope to open fully
    let sample = Arc::new(StereoSample::mono(vec![1.0; 200]));

    voice.trigger(sample, 1.0, 0.0); // gain=1.0, pan=0.0 (center)

    // Skip attack phase
    skip_attack_phase(&mut voice, 100);

    // Now check - left and right should be equal at center pan
    let (left, right) = voice.process_stereo();

    // Center pan should have equal power in both channels
    assert!(
        (left - right).abs() < 0.001,
        "Left and right should be equal at center pan, got left={}, right={}",
        left,
        right
    );

    // Both should be positive (sample is playing)
    assert!(
        left > 0.5,
        "Left should have significant level, got {}",
        left
    );
    assert!(
        right > 0.5,
        "Right should have significant level, got {}",
        right
    );
}

#[test]
fn test_voice_hard_left_pan() {
    // Test that pan=-1.0 (hard left) produces left channel only
    let mut voice = Voice::new();

    let sample = Arc::new(StereoSample::mono(vec![1.0; 200]));

    voice.trigger(sample, 1.0, -1.0); // gain=1.0, pan=-1.0 (hard left)

    // Skip attack phase
    skip_attack_phase(&mut voice, 100);

    let (left, right) = voice.process_stereo();

    assert!(
        left > 0.9,
        "Left channel should be ~1.0 at hard left pan, got {}",
        left
    );
    assert!(
        right.abs() < 0.001,
        "Right channel should be 0.0 at hard left, got {}",
        right
    );
}

#[test]
fn test_voice_hard_right_pan() {
    // Test that pan=1.0 (hard right) produces right channel only
    let mut voice = Voice::new();

    let sample = Arc::new(StereoSample::mono(vec![1.0; 200]));

    voice.trigger(sample, 1.0, 1.0); // gain=1.0, pan=1.0 (hard right)

    // Skip attack phase
    skip_attack_phase(&mut voice, 100);

    let (left, right) = voice.process_stereo();

    assert!(
        left.abs() < 0.001,
        "Left channel should be 0.0 at hard right, got {}",
        left
    );
    assert!(
        right > 0.9,
        "Right channel should be ~1.0 at hard right pan, got {}",
        right
    );
}

#[test]
fn test_voice_partial_pan() {
    // Test that pan=0.5 (halfway right) produces correct balance
    let mut voice = Voice::new();

    let sample = Arc::new(StereoSample::mono(vec![1.0; 200]));

    voice.trigger(sample, 1.0, 0.5); // gain=1.0, pan=0.5 (halfway right)

    // Skip attack phase
    skip_attack_phase(&mut voice, 100);

    let (left, right) = voice.process_stereo();

    // At pan=0.5, right should be louder than left
    assert!(
        right > left,
        "Right channel should be louder than left at pan=0.5, got left={}, right={}",
        left,
        right
    );

    // Both channels should have some signal
    assert!(left > 0.1, "Left should have some signal at pan=0.5");
    assert!(
        right > 0.5,
        "Right should have significant signal at pan=0.5"
    );

    // Equal-power panning preserves total energy (approximately)
    let total_power = left * left + right * right;
    assert!(
        total_power > 0.8 && total_power < 1.2,
        "Total power should be ~1.0 for equal-power panning, got {}",
        total_power
    );
}

#[test]
fn test_voice_manager_stereo_mixing() {
    // Test that VoiceManager correctly mixes multiple voices in stereo
    let mut vm = VoiceManager::new();

    // Use longer samples
    let sample1 = Arc::new(StereoSample::mono(vec![1.0; 200]));
    let sample2 = Arc::new(StereoSample::mono(vec![0.8; 200]));

    vm.trigger_sample_with_pan(sample1, 1.0, -1.0); // Hard left
    vm.trigger_sample_with_pan(sample2, 1.0, 1.0); // Hard right

    // Skip attack phase
    for _ in 0..100 {
        let _ = vm.process_stereo();
    }

    let (left, right) = vm.process_stereo();

    println!("Mixed output: left={}, right={}", left, right);

    // Left channel should have sample1 (~1.0)
    assert!(
        left > 0.9,
        "Left channel should have sample1 (~1.0), got {}",
        left
    );
    // Right channel should have sample2 (~0.8)
    assert!(
        right > 0.7,
        "Right channel should have sample2 (~0.8), got {}",
        right
    );
}

#[test]
fn test_pan_with_gain() {
    // Test that pan and gain work together correctly
    let mut voice = Voice::new();

    let sample = Arc::new(StereoSample::mono(vec![1.0; 200]));

    voice.trigger(sample, 0.5, -1.0); // gain=0.5, pan=-1.0 (hard left)

    // Skip attack phase
    skip_attack_phase(&mut voice, 100);

    let (left, right) = voice.process_stereo();

    assert!(
        (left - 0.5).abs() < 0.05,
        "Left channel should be ~0.5 (gain applied), got {}",
        left
    );
    assert!(
        right.abs() < 0.001,
        "Right channel should be 0.0, got {}",
        right
    );
}
