//! Plugin Integration Tests
//!
//! End-to-end tests for the plugin hosting system.
//! Uses MockPluginInstance for deterministic testing.

use phonon::plugin_host::{
    MockPluginInstance, NamedPluginInstance, PluginFormat, PluginId,
    PluginInstanceManager, PluginRegistry, PluginSettings,
};
use phonon::plugin_host::instance::MidiEvent;
use std::path::PathBuf;
use tempfile::tempdir;

/// Test that plugin registry scanning finds plugins
#[test]
fn test_plugin_registry_scanning() {
    let mut registry = PluginRegistry::new();

    // Scan should succeed even if no plugins found
    let result = registry.scan();
    assert!(result.is_ok());

    // Registry should be marked as scanned
    assert!(registry.is_scanned());
}

/// Test plugin instance manager with mock plugin
#[test]
fn test_plugin_instance_manager_lifecycle() {
    let mut manager = PluginInstanceManager::new();

    // Add mock plugin to registry
    let mock_info = MockPluginInstance::mock_plugin_info();
    manager.registry_mut().add_plugin(mock_info.clone());

    // Initialize manager
    manager.initialize(44100.0, 512).unwrap();

    // Create instance with auto-generated name
    let instance_name = manager.create_instance("MockSynth").unwrap();
    assert!(instance_name.starts_with("mocksynth:"));

    // Verify instance exists
    assert!(manager.get_instance(&instance_name).is_some());

    // Create named instance
    manager
        .create_named_instance("MockSynth", "my_synth")
        .unwrap();
    assert!(manager.get_instance("my_synth").is_some());
    assert!(manager.get_instance("~my_synth").is_some()); // With ~ prefix

    // List instances
    let instances = manager.list_instances();
    assert_eq!(instances.len(), 2);

    // Remove instance
    assert!(manager.remove_instance("my_synth"));
    assert!(manager.get_instance("my_synth").is_none());
}

/// Test plugin settings persistence
#[test]
fn test_plugin_settings_persistence() {
    let temp_dir = tempdir().unwrap();
    let settings_path = temp_dir.path().join("test.ph.plugins");

    // Create and save settings
    let mut settings = PluginSettings::new();
    settings.instances.push(NamedPluginInstance {
        name: "test_synth:1".to_string(),
        plugin_id: PluginId {
            format: PluginFormat::Vst3,
            identifier: "test".to_string(),
            name: "TestSynth".to_string(),
        },
        param_values: vec![0.5, 0.7],
        preset_name: Some("Init".to_string()),
        notes: "Test instance".to_string(),
    });

    settings.save(&settings_path).unwrap();

    // Load settings
    let loaded = PluginSettings::load(&settings_path).unwrap();
    assert_eq!(loaded.instances.len(), 1);
    assert_eq!(loaded.instances[0].name, "test_synth:1");
    assert_eq!(loaded.instances[0].param_values, vec![0.5, 0.7]);
}

/// Test mock plugin audio generation
#[test]
fn test_mock_plugin_audio_generation() {
    let mut plugin = MockPluginInstance::new();
    plugin.initialize(44100.0, 512).unwrap();

    // Play a note
    let note_on = MidiEvent::note_on(0, 0, 69, 100); // A4 = 440 Hz

    let mut left = vec![0.0f32; 1024];
    let mut right = vec![0.0f32; 1024];

    {
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
        plugin
            .process_with_midi(&[note_on], &mut outputs, 1024)
            .unwrap();
    }

    // Verify audio was generated
    let rms: f32 = (left.iter().map(|s| s * s).sum::<f32>() / 1024.0).sqrt();
    assert!(rms > 0.01, "Expected audio output, got RMS={}", rms);

    // Verify it's not just noise (has periodic structure)
    // For A4 at 44100 Hz, period is ~100 samples
    // Check correlation with shifted version
    let shift = 100; // ~440 Hz period
    let mut correlation = 0.0f32;
    for i in 0..(1024 - shift) {
        correlation += left[i] * left[i + shift];
    }
    correlation /= (1024 - shift) as f32;

    // Periodic signal should have positive correlation
    assert!(
        correlation > 0.1,
        "Expected periodic signal, got correlation={}",
        correlation
    );
}

/// Test mock plugin polyphony
#[test]
fn test_mock_plugin_polyphony() {
    let mut plugin = MockPluginInstance::new();
    plugin.initialize(44100.0, 512).unwrap();

    // Play C major chord
    let c = MidiEvent::note_on(0, 0, 60, 100);
    let e = MidiEvent::note_on(1, 0, 64, 100);
    let g = MidiEvent::note_on(2, 0, 67, 100);

    let mut left = vec![0.0f32; 512];
    let mut right = vec![0.0f32; 512];

    {
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
        plugin
            .process_with_midi(&[c, e, g], &mut outputs, 512)
            .unwrap();
    }

    // All three notes should be playing
    assert_eq!(plugin.active_voices(), 3);

    // Chord should be louder than single note
    let chord_rms: f32 = (left.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();

    // Reset and play single note
    let mut plugin2 = MockPluginInstance::new();
    plugin2.initialize(44100.0, 512).unwrap();

    let mut left2 = vec![0.0f32; 512];
    let mut right2 = vec![0.0f32; 512];

    {
        let mut outputs: Vec<&mut [f32]> = vec![&mut left2, &mut right2];
        let c_only = MidiEvent::note_on(0, 0, 60, 100);
        plugin2
            .process_with_midi(&[c_only], &mut outputs, 512)
            .unwrap();
    }

    let single_rms: f32 = (left2.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();

    // Chord should be louder (more voices adding energy)
    assert!(
        chord_rms > single_rms,
        "Expected chord louder: chord={}, single={}",
        chord_rms,
        single_rms
    );
}

/// Test mock plugin parameter changes
#[test]
fn test_mock_plugin_parameter_automation() {
    let mut plugin = MockPluginInstance::new();
    plugin.initialize(44100.0, 512).unwrap();

    // Set volume to maximum
    plugin.set_parameter(0, 1.0).unwrap();
    assert_eq!(plugin.get_parameter(0).unwrap(), 1.0);

    // Play note at max volume
    let note = MidiEvent::note_on(0, 0, 69, 127);
    let mut left_loud = vec![0.0f32; 512];
    let mut right_loud = vec![0.0f32; 512];

    {
        let mut outputs: Vec<&mut [f32]> = vec![&mut left_loud, &mut right_loud];
        plugin
            .process_with_midi(&[note.clone()], &mut outputs, 512)
            .unwrap();
    }

    let rms_loud: f32 = (left_loud.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();

    // Now set volume to minimum
    plugin.set_parameter(0, 0.0).unwrap();

    let mut left_quiet = vec![0.0f32; 512];
    let mut right_quiet = vec![0.0f32; 512];

    {
        let mut outputs: Vec<&mut [f32]> = vec![&mut left_quiet, &mut right_quiet];
        plugin
            .process_with_midi(&[], &mut outputs, 512) // Note already playing
            .unwrap();
    }

    let rms_quiet: f32 = (left_quiet.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();

    // Quiet should be much quieter (or silent)
    assert!(
        rms_quiet < rms_loud * 0.5,
        "Expected volume change: loud={}, quiet={}",
        rms_loud,
        rms_quiet
    );
}

/// Test full pipeline: registry -> manager -> instance -> audio
#[test]
fn test_full_plugin_pipeline() {
    // 1. Set up manager with mock plugin
    let mut manager = PluginInstanceManager::new();
    let mock_info = MockPluginInstance::mock_plugin_info();
    manager.registry_mut().add_plugin(mock_info);
    manager.initialize(44100.0, 512).unwrap();

    // 2. Create named instance
    manager
        .create_named_instance("MockSynth", "lead")
        .unwrap();

    // 3. Get instance
    let instance = manager.get_instance("lead").expect("Instance should exist");

    // 4. Use instance (this tests that our infrastructure works)
    let handle = instance.lock().unwrap();
    assert!(handle.is_initialized());
    assert_eq!(handle.info().id.name, "MockSynth");
    drop(handle);

    // 5. Create another instance to verify naming
    let auto_name = manager.create_instance("MockSynth").unwrap();
    assert!(auto_name.starts_with("mocksynth:"));

    // 6. List all instances
    let names = manager.list_instances();
    assert!(names.contains(&"lead".to_string()));
    assert!(names.iter().any(|n| n.starts_with("mocksynth:")));

    // 7. Cleanup
    manager.remove_instance("lead");
    assert!(manager.get_instance("lead").is_none());
}

/// Test that settings path derivation works correctly
#[test]
fn test_settings_path_derivation() {
    let ph_path = PathBuf::from("/home/user/music/song.ph");
    let settings_path = PluginInstanceManager::settings_path_for_ph_file(&ph_path);
    assert_eq!(
        settings_path,
        PathBuf::from("/home/user/music/song.ph.plugins")
    );

    let ph_path2 = PathBuf::from("untitled.ph");
    let settings_path2 = PluginInstanceManager::settings_path_for_ph_file(&ph_path2);
    assert_eq!(settings_path2, PathBuf::from("untitled.ph.plugins"));
}
