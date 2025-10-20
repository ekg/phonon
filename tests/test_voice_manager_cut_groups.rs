/// Direct test of VoiceManager cut group functionality
use phonon::voice_manager::VoiceManager;
use std::sync::Arc;

#[test]
fn test_voice_manager_cut_group_stops_previous() {
    let mut vm = VoiceManager::new();

    // Create a simple sample
    let sample1 = Arc::new(vec![0.1; 1000]); // 1000 samples of 0.1
    let sample2 = Arc::new(vec![0.2; 1000]); // 1000 samples of 0.2

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
    vm.trigger_sample_with_cut_group(sample2.clone(), 1.0, 0.0, 1.0, Some(1));

    println!(
        "After second trigger (same cut group): {} voices",
        vm.active_voice_count()
    );
    assert_eq!(
        vm.active_voice_count(),
        1,
        "Should have only 1 voice - second stopped first"
    );

    // Process and verify
    for _ in 0..10 {
        let _ = vm.process();
    }

    println!(
        "After processing 10 more samples: {} voices",
        vm.active_voice_count()
    );
    assert_eq!(vm.active_voice_count(), 1, "Should still have only 1 voice");
}

#[test]
fn test_voice_manager_different_cut_groups_dont_interfere() {
    let mut vm = VoiceManager::new();

    let sample1 = Arc::new(vec![0.1; 1000]);
    let sample2 = Arc::new(vec![0.2; 1000]);

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

    let sample1 = Arc::new(vec![0.1; 1000]);
    let sample2 = Arc::new(vec![0.2; 1000]);

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
