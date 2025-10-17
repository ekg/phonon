use phonon::voice_manager::{Voice, VoiceManager};
use std::sync::Arc;

#[test]
fn test_voice_center_pan() {
    // Test that pan=0.0 (center) produces equal left and right channels
    let mut voice = Voice::new();

    let sample_data = vec![1.0, 0.8, 0.6, 0.4, 0.2];
    let sample = Arc::new(sample_data.clone());

    voice.trigger(sample, 1.0, 0.0); // gain=1.0, pan=0.0 (center)

    // Process samples and check left/right are equal
    for expected in &sample_data {
        let (left, right) = voice.process_stereo();

        // Center pan should have equal power in both channels
        // With equal-power panning: left = right = value * sqrt(0.5)
        let expected_channel = expected * (0.5_f32).sqrt();

        assert!(
            (left - expected_channel).abs() < 0.001,
            "Left channel should be {} at center pan, got {}",
            expected_channel,
            left
        );
        assert!(
            (right - expected_channel).abs() < 0.001,
            "Right channel should be {} at center pan, got {}",
            expected_channel,
            right
        );
    }
}

#[test]
fn test_voice_hard_left_pan() {
    // Test that pan=-1.0 (hard left) produces left channel only
    let mut voice = Voice::new();

    let sample_data = vec![1.0, 0.8, 0.6];
    let sample = Arc::new(sample_data.clone());

    voice.trigger(sample, 1.0, -1.0); // gain=1.0, pan=-1.0 (hard left)

    for expected in &sample_data {
        let (left, right) = voice.process_stereo();

        assert!(
            (left - expected).abs() < 0.001,
            "Left channel should be {}, got {}",
            expected,
            left
        );
        assert!(
            right.abs() < 0.001,
            "Right channel should be 0.0 at hard left, got {}",
            right
        );
    }
}

#[test]
fn test_voice_hard_right_pan() {
    // Test that pan=1.0 (hard right) produces right channel only
    let mut voice = Voice::new();

    let sample_data = vec![1.0, 0.8, 0.6];
    let sample = Arc::new(sample_data.clone());

    voice.trigger(sample, 1.0, 1.0); // gain=1.0, pan=1.0 (hard right)

    for expected in &sample_data {
        let (left, right) = voice.process_stereo();

        assert!(
            left.abs() < 0.001,
            "Left channel should be 0.0 at hard right, got {}",
            left
        );
        assert!(
            (right - expected).abs() < 0.001,
            "Right channel should be {}, got {}",
            expected,
            right
        );
    }
}

#[test]
fn test_voice_partial_pan() {
    // Test that pan=0.5 (halfway right) produces correct balance
    let mut voice = Voice::new();

    let sample_data = vec![1.0];
    let sample = Arc::new(sample_data);

    voice.trigger(sample, 1.0, 0.5); // gain=1.0, pan=0.5 (halfway right)

    let (left, right) = voice.process_stereo();

    // At pan=0.5, right should be louder than left
    assert!(
        right > left,
        "Right channel should be louder than left at pan=0.5"
    );

    // Equal-power panning preserves total energy
    let total_power = left * left + right * right;
    assert!(
        (total_power - 1.0).abs() < 0.1,
        "Total power should be ~1.0 for equal-power panning, got {}",
        total_power
    );
}

#[test]
fn test_voice_manager_stereo_mixing() {
    // Test that VoiceManager correctly mixes multiple voices in stereo
    let mut vm = VoiceManager::new();

    // Trigger two samples with different pan positions
    let sample1 = Arc::new(vec![1.0, 0.5, 0.0]);
    let sample2 = Arc::new(vec![0.8, 0.4, 0.0]);

    vm.trigger_sample_with_pan(sample1, 1.0, -1.0); // Hard left
    vm.trigger_sample_with_pan(sample2, 1.0, 1.0); // Hard right

    // First sample should have left=1.0, right=0.0 from voice 1
    // and left=0.0, right=0.8 from voice 2
    let (left, right) = vm.process_stereo();

    println!("Mixed output: left={}, right={}", left, right);

    assert!(
        left > 0.9,
        "Left channel should have sample1 (1.0), got {}",
        left
    );
    assert!(
        right > 0.7,
        "Right channel should have sample2 (0.8), got {}",
        right
    );
}

#[test]
fn test_pan_with_gain() {
    // Test that pan and gain work together correctly
    let mut voice = Voice::new();

    let sample_data = vec![1.0];
    let sample = Arc::new(sample_data);

    voice.trigger(sample, 0.5, -1.0); // gain=0.5, pan=-1.0 (hard left)

    let (left, right) = voice.process_stereo();

    assert!(
        (left - 0.5).abs() < 0.001,
        "Left channel should be 0.5 (gain applied), got {}",
        left
    );
    assert!(
        right.abs() < 0.001,
        "Right channel should be 0.0, got {}",
        right
    );
}
