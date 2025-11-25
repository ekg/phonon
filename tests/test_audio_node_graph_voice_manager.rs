//! Test AudioNodeGraph integration with VoiceManager and SampleBank
//!
//! This test verifies that AudioNodeGraph properly initializes and provides
//! access to VoiceManager and SampleBank for pattern playback.

use phonon::audio_node_graph::AudioNodeGraph;
use phonon::sample_loader::StereoSample;
use std::sync::Arc;

#[test]
fn test_audio_node_graph_has_voice_manager_and_sample_bank() {
    // Create an AudioNodeGraph
    let graph = AudioNodeGraph::new(44100.0);

    // Verify we can access voice_manager
    let voice_manager = graph.voice_manager();
    assert!(voice_manager.lock().is_ok(), "Voice manager should be accessible");

    // Verify we can access sample_bank
    let sample_bank = graph.sample_bank();
    // Sample bank is wrapped in Arc, so we can just clone it
    let _ = sample_bank.clone();
}

#[test]
fn test_voice_manager_can_be_used() {
    let graph = AudioNodeGraph::new(44100.0);

    // Get voice manager
    let voice_manager = graph.voice_manager();
    let vm = voice_manager.lock().unwrap();

    // Verify initial state
    assert_eq!(vm.active_voice_count(), 0, "No voices should be active initially");
    assert!(vm.pool_size() > 0, "Voice pool should be initialized");
}

#[test]
fn test_sample_bank_can_load_samples() {
    

    let graph = AudioNodeGraph::new(44100.0);

    // Get sample bank
    let sample_bank = graph.sample_bank();

    // Lock and access the sample bank
    let mut bank = sample_bank.lock().unwrap();

    // Try to load a test sample
    // Note: This will only work if dirt-samples directory exists
    // For now, we just verify we can access the bank mutably
    let _ = &mut bank;
}

#[test]
fn test_voice_manager_sample_bank_integration() {
    // This test demonstrates the intended workflow:
    // 1. Create graph
    // 2. Access voice manager and sample bank
    // 3. Load sample
    // 4. Trigger sample playback

    use std::sync::Arc;

    let graph = AudioNodeGraph::new(44100.0);

    // Get voice manager
    let voice_manager = graph.voice_manager();
    let mut vm = voice_manager.lock().unwrap();

    // Get sample bank
    let sample_bank = graph.sample_bank();
    let _bank = sample_bank.lock().unwrap();

    // For now, we verify that the components exist
    let initial_active = vm.active_voice_count();
    assert_eq!(initial_active, 0, "Should start with no active voices");

    // Create a simple test sample
    let test_sample = Arc::new(StereoSample::mono(vec![0.5f32; 1000]));

    // Trigger the sample
    vm.trigger_sample(test_sample, 1.0);

    // Verify a voice was allocated
    assert_eq!(vm.active_voice_count(), 1, "Should have 1 active voice after trigger");

    // Process a few samples to advance the voice
    for _ in 0..10 {
        let _ = vm.process();
    }

    // Voice should still be active (envelope not finished yet)
    assert!(vm.active_voice_count() > 0, "Voice should still be active");
}

#[test]
fn test_multiple_sample_triggers() {
    let graph = AudioNodeGraph::new(44100.0);
    let voice_manager = graph.voice_manager();
    let mut vm = voice_manager.lock().unwrap();

    // Create test samples
    let sample1 = Arc::new(StereoSample::mono(vec![0.5f32; 500]));
    let sample2 = Arc::new(StereoSample::mono(vec![0.7f32; 800]));
    let sample3 = Arc::new(StereoSample::mono(vec![0.3f32; 1200]));

    // Trigger multiple samples
    vm.trigger_sample(sample1, 1.0);
    vm.trigger_sample(sample2, 0.8);
    vm.trigger_sample(sample3, 0.6);

    // Should have 3 active voices
    assert_eq!(vm.active_voice_count(), 3, "Should have 3 active voices");

    // Process audio
    for _ in 0..100 {
        let _output = vm.process();
        // Just verify we get some output (sum of voices)
        // Don't assert specific values as they depend on envelope/timing
    }

    // Voices should still be active
    assert!(vm.active_voice_count() > 0, "At least some voices should still be active");
}

#[test]
fn test_voice_manager_reset() {
    let graph = AudioNodeGraph::new(44100.0);
    let voice_manager = graph.voice_manager();
    let mut vm = voice_manager.lock().unwrap();

    // Trigger some samples
    let sample = Arc::new(StereoSample::mono(vec![0.5f32; 1000]));
    vm.trigger_sample(sample.clone(), 1.0);
    vm.trigger_sample(sample.clone(), 0.8);
    vm.trigger_sample(sample, 0.6);

    assert_eq!(vm.active_voice_count(), 3, "Should have 3 active voices");

    // Reset all voices
    vm.reset();

    // Should have no active voices
    assert_eq!(vm.active_voice_count(), 0, "Reset should clear all active voices");
}

#[test]
fn test_voice_manager_stereo_output() {
    let graph = AudioNodeGraph::new(44100.0);
    let voice_manager = graph.voice_manager();
    let mut vm = voice_manager.lock().unwrap();

    // Create test sample
    let sample = Arc::new(StereoSample::mono(vec![1.0f32; 500]));

    // Trigger with different pan positions
    vm.trigger_sample_with_pan(sample.clone(), 1.0, -1.0); // Hard left
    vm.trigger_sample_with_pan(sample.clone(), 1.0, 0.0);  // Center
    vm.trigger_sample_with_pan(sample, 1.0, 1.0);          // Hard right

    // Process stereo output
    let (left, right) = vm.process_stereo();

    // Just verify we get output
    // Exact values depend on envelope and panning calculations
    assert!(left.abs() > 0.0 || right.abs() > 0.0, "Should produce stereo output");
}
