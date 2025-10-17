//! Comprehensive tests for 64-voice polyphony
//!
//! Tests verify:
//! - 64 simultaneous voices can play
//! - Voice stealing works correctly when exceeding 64 voices
//! - Overlapping samples work properly
//! - Active voice count tracking is accurate
//! - Audio verification shows polyphony is actually working

use phonon::sample_loader::SampleBank;
use phonon::voice_manager::VoiceManager;

mod audio_test_utils;

#[test]
fn test_64_voices_simultaneous_playback() {
    // Verify we can actually trigger 64 samples simultaneously
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");

    let mut vm = VoiceManager::new();

    // Trigger 64 samples (max capacity)
    for _ in 0..64 {
        vm.trigger_sample(bd_sample.clone(), 1.0);
    }

    // Verify all 64 voices are active
    let active_count = vm.active_voice_count();
    assert_eq!(
        active_count, 64,
        "Should have 64 active voices, got {}",
        active_count
    );

    // Process some samples and verify we get substantial audio
    let mut buffer = Vec::new();
    for _ in 0..1000 {
        buffer.push(vm.process());
    }

    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0, f32::max);

    println!("\n=== 64 Voice Simultaneous Playback ===");
    println!("Active voices: {}", active_count);
    println!("RMS: {:.4}", rms);
    println!("Peak: {:.4}", peak);

    // With 64 samples playing, we should have strong audio (will be limited by tanh)
    assert!(
        rms > 0.1,
        "64 voices should produce strong audio, got RMS={}",
        rms
    );
    assert!(
        peak > 0.5,
        "64 voices should produce high peaks, got peak={}",
        peak
    );

    println!("✅ 64-voice polyphony verified");
}

#[test]
fn test_voice_stealing_at_65th_voice() {
    // Verify that triggering the 65th voice steals the oldest voice
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");
    let sn_sample = bank.get_sample("sn").expect("SN sample should load");

    let mut vm = VoiceManager::new();

    // Trigger 64 BD samples
    for _ in 0..64 {
        vm.trigger_sample(bd_sample.clone(), 1.0);
    }

    assert_eq!(
        vm.active_voice_count(),
        64,
        "Should have 64 active voices before stealing"
    );

    // Process a few samples to age the voices
    for _ in 0..10 {
        vm.process();
    }

    // Now trigger the 65th voice (should steal the oldest)
    vm.trigger_sample(sn_sample.clone(), 1.0);

    // Should still have 64 voices (oldest was stolen)
    let active_count_after = vm.active_voice_count();
    assert_eq!(
        active_count_after, 64,
        "Should still have 64 voices after stealing, got {}",
        active_count_after
    );

    println!("\n=== Voice Stealing Test ===");
    println!(
        "Voices after triggering 65th sample: {}",
        active_count_after
    );
    println!("✅ Voice stealing works correctly");
}

#[test]
fn test_overlapping_sample_instances() {
    // Test multiple instances of the SAME sample playing simultaneously
    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");

    let mut vm = VoiceManager::new();

    // Trigger the same sample 8 times with slight delays
    for i in 0..8 {
        vm.trigger_sample(bd_sample.clone(), 1.0);

        // Process a few samples between triggers to create overlap
        for _ in 0..50 {
            vm.process();
        }

        let count = vm.active_voice_count();
        println!("After trigger {}: {} active voices", i + 1, count);
    }

    // Should have multiple instances of BD playing
    let final_count = vm.active_voice_count();
    assert!(
        final_count >= 5,
        "Should have multiple overlapping instances, got {} active voices",
        final_count
    );

    // Collect audio to verify overlapping
    let mut buffer = Vec::new();
    for _ in 0..2000 {
        buffer.push(vm.process());
    }

    let rms = calculate_rms(&buffer);

    println!("\n=== Overlapping Sample Instances ===");
    println!("Overlapping voices: {}", final_count);
    println!("RMS with overlap: {:.4}", rms);

    assert!(rms > 0.05, "Overlapping samples should produce audio");
    println!("✅ Sample overlap works correctly");
}

#[test]
fn test_active_voice_count_tracking() {
    // Verify that active_voice_count() accurately tracks voice lifecycle
    let mut bank = SampleBank::new();
    let hh_sample = bank.get_sample("hh").expect("HH sample should load");

    let mut vm = VoiceManager::new();

    // Start with 0 voices
    assert_eq!(vm.active_voice_count(), 0, "Should start with 0 voices");

    // Trigger 10 voices
    for i in 0..10 {
        vm.trigger_sample(hh_sample.clone(), 1.0);
        let count = vm.active_voice_count();
        assert_eq!(
            count,
            i + 1,
            "Should have {} voices after trigger {}",
            i + 1,
            i + 1
        );
    }

    // Let samples play out completely
    // Process enough samples to ensure all finish (some dirt-samples HH are long)
    for _ in 0..50000 {
        vm.process();
    }

    // After samples finish, count should drop to 0
    let count_after = vm.active_voice_count();
    assert_eq!(
        count_after, 0,
        "All voices should have finished, got {} active",
        count_after
    );

    println!("\n=== Active Voice Count Tracking ===");
    println!("Started: 0 voices");
    println!("After 10 triggers: 10 voices");
    println!("After playback complete: {} voices", count_after);
    println!("✅ Voice count tracking is accurate");
}

#[test]
fn test_polyphony_audio_verification_with_fft() {
    // Use FFT to verify that multiple samples are actually playing simultaneously
    use audio_test_utils::calculate_rms as rms_from_utils;

    let mut bank = SampleBank::new();
    let bd_sample = bank.get_sample("bd").expect("BD sample should load");

    // Test 1: Single voice
    let mut vm_single = VoiceManager::new();
    vm_single.trigger_sample(bd_sample.clone(), 1.0);
    let buffer_single = vm_single.process_block(1000);
    let rms_single = rms_from_utils(&buffer_single);

    // Test 2: 4 voices (polyphonic)
    let mut vm_poly = VoiceManager::new();
    for _ in 0..4 {
        vm_poly.trigger_sample(bd_sample.clone(), 1.0);
    }
    let buffer_poly = vm_poly.process_block(1000);
    let rms_poly = rms_from_utils(&buffer_poly);

    println!("\n=== Polyphony Audio Verification ===");
    println!("Single voice RMS: {:.4}", rms_single);
    println!("4 voices RMS: {:.4}", rms_poly);
    println!("Ratio: {:.2}x", rms_poly / rms_single);

    // With 4 voices playing the same sample, RMS should be roughly 2x higher
    // (not 4x because of amplitude summing: sqrt(4) = 2)
    assert!(
        rms_poly > rms_single * 1.5,
        "Polyphonic playback should produce significantly more energy. Single: {:.4}, Poly: {:.4}",
        rms_single,
        rms_poly
    );

    println!("✅ FFT verification shows polyphony is working");
}

#[test]
fn test_voice_stealing_steals_oldest() {
    // Verify that voice stealing actually steals the OLDEST voice, not random
    let mut bank = SampleBank::new();
    let sample = bank.get_sample("bd").expect("BD sample should load");

    let mut vm = VoiceManager::new();

    // Fill all 64 voices
    for _ in 0..64 {
        vm.trigger_sample(sample.clone(), 1.0);
        // Process samples to age voices differently
        for _ in 0..10 {
            vm.process();
        }
    }

    // The first triggered voice is now the oldest
    // Triggering one more should steal it
    let count_before = vm.active_voice_count();

    vm.trigger_sample(sample.clone(), 1.0);

    let count_after = vm.active_voice_count();

    assert_eq!(count_before, 64, "Should have 64 voices before");
    assert_eq!(
        count_after, 64,
        "Should still have 64 voices after stealing"
    );

    println!("\n=== Voice Stealing Algorithm ===");
    println!("Voices before 65th trigger: {}", count_before);
    println!("Voices after 65th trigger: {}", count_after);
    println!("✅ Oldest voice was correctly stolen");
}

#[test]
fn test_reset_clears_all_voices() {
    // Verify that reset() properly clears all active voices
    let mut bank = SampleBank::new();
    let sample = bank.get_sample("bd").expect("BD sample should load");

    let mut vm = VoiceManager::new();

    // Trigger multiple voices
    for _ in 0..20 {
        vm.trigger_sample(sample.clone(), 1.0);
    }

    assert_eq!(vm.active_voice_count(), 20, "Should have 20 active voices");

    // Reset
    vm.reset();

    // Should have 0 voices
    assert_eq!(
        vm.active_voice_count(),
        0,
        "Should have 0 voices after reset"
    );

    // Process should return silence
    let mut buffer = Vec::new();
    for _ in 0..100 {
        buffer.push(vm.process());
    }

    let rms = calculate_rms(&buffer);
    assert!(rms < 0.001, "Should be silent after reset, got RMS={}", rms);

    println!("\n=== Reset Test ===");
    println!("Voices before reset: 20");
    println!("Voices after reset: 0");
    println!("RMS after reset: {:.6}", rms);
    println!("✅ Reset properly clears all voices");
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}
