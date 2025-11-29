/// Direct test of VoiceManager cut group functionality
use phonon::sample_loader::StereoSample;
use phonon::voice_manager::VoiceManager;
use std::sync::Arc;

#[test]
fn test_voice_manager_cut_group_stops_previous() {
    let mut vm = VoiceManager::new();

    // Create a simple sample (longer to ensure it doesn't naturally end)
    let sample1 = Arc::new(StereoSample::mono(vec![0.1; 10000])); // 10000 samples
    let sample2 = Arc::new(StereoSample::mono(vec![0.2; 10000]));

    // Trigger first voice with cut group 1
    vm.trigger_sample_with_cut_group(sample1.clone(), 1.0, 0.0, 1.0, Some(1));

    println!("After first trigger: {} voices", vm.active_voice_count());
    assert_eq!(
        vm.active_voice_count(),
        1,
        "Should have 1 voice after first trigger"
    );

    // Process a few samples
    for _ in 0..10 {
        let _ = vm.process();
    }

    println!(
        "After processing 10 samples: {} voices",
        vm.active_voice_count()
    );
    assert_eq!(vm.active_voice_count(), 1, "Should still have 1 voice");

    // Trigger second voice with SAME cut group - should stop first voice
    // Note: Cut group uses a 10ms fade-out (~441 samples at 44100Hz) to avoid clicks
    vm.trigger_sample_with_cut_group(sample2.clone(), 1.0, 0.0, 1.0, Some(1));

    // Immediately after trigger, both voices are active (old one is fading)
    println!(
        "Immediately after second trigger: {} voices (old voice fading)",
        vm.active_voice_count()
    );

    // Process enough samples to let the quick release finish (10ms @ 44100Hz = ~441 samples)
    for _ in 0..500 {
        let _ = vm.process();
    }

    println!("After fade-out period: {} voices", vm.active_voice_count());
    assert_eq!(
        vm.active_voice_count(),
        1,
        "Should have only 1 voice after fade-out completes"
    );
}

#[test]
fn test_voice_manager_different_cut_groups_dont_interfere() {
    let mut vm = VoiceManager::new();

    let sample1 = Arc::new(StereoSample::mono(vec![0.1; 10000]));
    let sample2 = Arc::new(StereoSample::mono(vec![0.2; 10000]));

    // Trigger with cut group 1
    vm.trigger_sample_with_cut_group(sample1.clone(), 1.0, 0.0, 1.0, Some(1));
    assert_eq!(vm.active_voice_count(), 1);

    // Trigger with cut group 2 - should NOT stop first voice
    vm.trigger_sample_with_cut_group(sample2.clone(), 1.0, 0.0, 1.0, Some(2));

    println!(
        "After triggers with different cut groups: {} voices",
        vm.active_voice_count()
    );
    assert_eq!(
        vm.active_voice_count(),
        2,
        "Different cut groups should not interfere"
    );
}

#[test]
fn test_voice_manager_no_cut_group_allows_overlap() {
    let mut vm = VoiceManager::new();

    let sample1 = Arc::new(StereoSample::mono(vec![0.1; 10000]));
    let sample2 = Arc::new(StereoSample::mono(vec![0.2; 10000]));

    // Trigger with no cut group (None)
    vm.trigger_sample_with_cut_group(sample1.clone(), 1.0, 0.0, 1.0, None);
    assert_eq!(vm.active_voice_count(), 1);

    // Trigger with no cut group - should allow overlap
    vm.trigger_sample_with_cut_group(sample2.clone(), 1.0, 0.0, 1.0, None);

    println!(
        "After triggers with no cut group: {} voices",
        vm.active_voice_count()
    );
    assert_eq!(
        vm.active_voice_count(),
        2,
        "No cut group should allow overlap"
    );
}
