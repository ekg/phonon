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

/// Test that plugin scanning finds real installed plugins
/// This test verifies the full scan pipeline works
#[test]
fn test_real_plugin_scanning() {
    let mut registry = PluginRegistry::new();
    let count = registry.scan().unwrap();

    // Should find some plugins if any are installed
    // (gearmulator, Surge XT, etc.)
    if count > 0 {
        // Verify search works
        let plugins = registry.list();
        assert!(!plugins.is_empty());

        // Each plugin should have valid info
        for plugin in plugins {
            assert!(!plugin.id.name.is_empty());
            assert!(!plugin.path.is_empty());
        }
    }
}

/// Test plugin manager with real scanned plugins
#[test]
fn test_manager_with_real_plugins() {
    let mut manager = PluginInstanceManager::new();
    manager.initialize(44100.0, 512).unwrap();

    // Scan for real plugins
    let _ = manager.registry_mut().scan();

    let plugins = manager.list_plugins();
    if !plugins.is_empty() {
        // Try to create an instance of the first plugin
        let first_plugin_name = plugins[0].id.name.clone();
        let result = manager.create_instance(&first_plugin_name);

        // Should succeed in creating an instance
        assert!(result.is_ok(), "Failed to create instance: {:?}", result);

        let instance_name = result.unwrap();
        assert!(!instance_name.is_empty());

        // Should be able to retrieve the instance
        assert!(manager.get_instance(&instance_name).is_some());
    }
}

/// Test MockPluginInstance integration with audio engine
/// Verifies that PluginInstance nodes with "MockSynth" actually generate audio
#[test]
fn test_audio_engine_mock_plugin_integration() {
    use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
    use std::collections::HashMap;
    use std::cell::RefCell;

    // Create a signal graph
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a PluginInstance node with MockSynth
    // Using note_pattern with A4 (MIDI note 69)
    let note_pattern = phonon::pattern::Pattern::pure(69.0);
    let plugin_node = SignalNode::PluginInstance {
        plugin_id: "MockSynth".to_string(),
        audio_inputs: vec![],
        params: HashMap::new(),
        note_pattern: Some(note_pattern),
        note_pattern_str: Some("69".to_string()),
        last_note_cycle: std::cell::Cell::new(-1),
        triggered_notes: std::cell::RefCell::new(std::collections::HashSet::new()),
        cached_note_events: std::cell::RefCell::new(Vec::new()),
        instance: RefCell::new(None),
    };

    // Add node to graph and get ID
    let node_id = graph.add_node(plugin_node);

    // Set output to the plugin node
    graph.set_output(node_id);

    // Render audio using buffer-based DAG processing
    let buffer_size = 512;
    let mut output = vec![0.0f32; buffer_size];
    let cycle_start = 0.0;
    let sample_increment = 1.0 / 44100.0;

    graph.process_buffer_dag(&mut output, cycle_start, sample_increment);

    // Verify audio was generated (not silence)
    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / buffer_size as f32).sqrt();
    assert!(
        rms > 0.01,
        "Expected audio output from MockSynth, got RMS={}",
        rms
    );
}

/// Test MockPluginInstance generates different pitches for different notes
#[test]
fn test_audio_engine_mock_plugin_pitch() {
    use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
    use std::collections::HashMap;
    use std::cell::RefCell;

    let buffer_size = 1024;

    // Helper function to render audio for a given note
    let render_note = |note: u8| -> Vec<f32> {
        let mut graph = UnifiedSignalGraph::new(44100.0);

        let note_pattern = phonon::pattern::Pattern::pure(note as f64);
        let plugin_node = SignalNode::PluginInstance {
            plugin_id: "MockSynth".to_string(),
            audio_inputs: vec![],
            params: HashMap::new(),
            note_pattern: Some(note_pattern),
            note_pattern_str: Some(format!("{}", note)),
            last_note_cycle: std::cell::Cell::new(-1),
            triggered_notes: std::cell::RefCell::new(std::collections::HashSet::new()),
            cached_note_events: std::cell::RefCell::new(Vec::new()),
            instance: RefCell::new(None),
        };

        let node_id = graph.add_node(plugin_node);
        graph.set_output(node_id);

        let mut output = vec![0.0f32; buffer_size];
        graph.process_buffer_dag(&mut output, 0.0, 1.0 / 44100.0);
        output
    };

    // Count zero crossings to estimate frequency
    let count_zero_crossings = |samples: &[f32]| -> usize {
        let mut count = 0;
        for i in 1..samples.len() {
            if (samples[i - 1] < 0.0 && samples[i] >= 0.0)
                || (samples[i - 1] >= 0.0 && samples[i] < 0.0)
            {
                count += 1;
            }
        }
        count
    };

    // Render A4 (69) and A5 (81) - octave apart
    let a4_output = render_note(69);
    let a5_output = render_note(81);

    let a4_crossings = count_zero_crossings(&a4_output);
    let a5_crossings = count_zero_crossings(&a5_output);

    // A5 should have ~twice as many zero crossings as A4 (octave = 2x frequency)
    let ratio = a5_crossings as f32 / a4_crossings as f32;
    assert!(
        ratio > 1.8 && ratio < 2.2,
        "Expected ~2x frequency ratio for octave, got {} (A4={}, A5={})",
        ratio,
        a4_crossings,
        a5_crossings
    );
}
