//! VST3 Plugin Integration Tests
//!
//! Tests real VST3 plugins to ensure proper audio generation.
//! These tests require VST3 plugins to be installed:
//! - Surge XT (instrument with 2 inputs)
//! - Odin2 (instrument with 0 inputs)
//!
//! Tests are skipped if plugins are not available.

use phonon::plugin_host::{PluginRegistry, create_real_plugin_by_name};
use phonon::plugin_host::instance::MidiEvent;

/// Helper to check if a plugin is available
fn plugin_available(name: &str) -> bool {
    let mut registry = PluginRegistry::new();
    let _ = registry.scan();
    registry.find(name).is_some()
}

/// Test Surge XT (instrument with 2 input channels)
/// Surge XT accepts stereo input for effects processing mode
#[test]
fn test_vst3_surge_xt_produces_audio() {
    if !plugin_available("Surge XT") {
        eprintln!("Skipping test: Surge XT not installed");
        return;
    }

    let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
    plugin.initialize(44100.0, 512).unwrap();

    // Send note-on and process multiple buffers
    let note_on = MidiEvent::note_on(0, 0, 60, 100);

    let mut total_rms = 0.0f32;
    let num_buffers = 20;

    for i in 0..num_buffers {
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        let midi = if i == 0 { vec![note_on.clone()] } else { vec![] };
        plugin.process_with_midi(&midi, &mut outputs, 512).unwrap();

        let buffer_rms: f32 = (left.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();
        total_rms += buffer_rms;
    }

    let avg_rms = total_rms / num_buffers as f32;

    // Leak plugin to avoid VST3 cleanup crash (double-free bug in VST3 SDK)
    plugin.leak();

    assert!(
        avg_rms > 0.001,
        "Surge XT should produce audio, got avg RMS={}",
        avg_rms
    );
}

/// Test Odin2 (instrument with 0 input channels)
/// Odin2 is a pure synthesizer that expects no audio input
#[test]
fn test_vst3_odin2_produces_audio() {
    if !plugin_available("Odin2") {
        eprintln!("Skipping test: Odin2 not installed");
        return;
    }

    let mut plugin = create_real_plugin_by_name("Odin2").unwrap();
    plugin.initialize(44100.0, 512).unwrap();

    // Send note-on and process multiple buffers
    let note_on = MidiEvent::note_on(0, 0, 60, 100);

    let mut total_rms = 0.0f32;
    let num_buffers = 20;

    for i in 0..num_buffers {
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        let midi = if i == 0 { vec![note_on.clone()] } else { vec![] };
        plugin.process_with_midi(&midi, &mut outputs, 512).unwrap();

        let buffer_rms: f32 = (left.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();
        total_rms += buffer_rms;
    }

    let avg_rms = total_rms / num_buffers as f32;

    // Leak plugin to avoid VST3 cleanup crash (double-free bug in VST3 SDK)
    plugin.leak();

    assert!(
        avg_rms > 0.001,
        "Odin2 should produce audio, got avg RMS={}",
        avg_rms
    );
}

/// Test VST3 plugin loading and initialization
#[test]
fn test_vst3_plugin_loading() {
    // Try to load any available VST3 plugin
    let mut registry = PluginRegistry::new();
    let _ = registry.scan();

    let plugins = registry.list();
    if plugins.is_empty() {
        eprintln!("Skipping test: No VST3 plugins installed");
        return;
    }

    // Try to load the first plugin
    let first_plugin = &plugins[0];
    let result = create_real_plugin_by_name(&first_plugin.id.name);

    match result {
        Ok(mut plugin) => {
            // Initialize the plugin
            let init_result = plugin.initialize(44100.0, 512);
            assert!(
                init_result.is_ok(),
                "Plugin initialization failed: {:?}",
                init_result
            );

            // Verify parameter count is reasonable
            let param_count = plugin.parameter_count();
            assert!(
                param_count < 10000,
                "Unreasonable parameter count: {}",
                param_count
            );

            // Leak plugin to avoid VST3 cleanup crash
            plugin.leak();
        }
        Err(e) => {
            eprintln!("Plugin load failed (may be expected): {}", e);
        }
    }
}

/// Test VST3 input channel auto-detection
/// This verifies that our fallback mechanism works for both
/// 0-input and 2-input plugins
#[test]
fn test_vst3_input_channel_detection() {
    // Test Odin2 (0 inputs)
    if plugin_available("Odin2") {
        let mut plugin = create_real_plugin_by_name("Odin2").unwrap();
        plugin.initialize(44100.0, 512).unwrap();

        // Process with MIDI - should auto-detect 0 inputs
        let note_on = MidiEvent::note_on(0, 0, 60, 100);
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        let result = plugin.process_with_midi(&[note_on], &mut outputs, 512);
        plugin.leak(); // Leak before assert to avoid crash
        assert!(result.is_ok(), "Odin2 process should succeed: {:?}", result);
    }

    // Test Surge XT (2 inputs)
    if plugin_available("Surge XT") {
        let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
        plugin.initialize(44100.0, 512).unwrap();

        // Process with MIDI - should auto-detect 2 inputs
        let note_on = MidiEvent::note_on(0, 0, 60, 100);
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        let result = plugin.process_with_midi(&[note_on], &mut outputs, 512);
        plugin.leak(); // Leak before assert to avoid crash
        assert!(result.is_ok(), "Surge XT process should succeed: {:?}", result);
    }
}

/// Test multiple buffers of processing (ensures state is maintained)
#[test]
fn test_vst3_sustained_note() {
    if !plugin_available("Surge XT") {
        eprintln!("Skipping test: Surge XT not installed");
        return;
    }

    let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
    plugin.initialize(44100.0, 512).unwrap();

    let note_on = MidiEvent::note_on(0, 0, 60, 100);

    // Process multiple buffers
    let mut total_rms = 0.0f32;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        // Only send note-on in first buffer
        let midi = if i == 0 { vec![note_on.clone()] } else { vec![] };

        plugin.process_with_midi(&midi, &mut outputs, 512).unwrap();

        let buffer_rms: f32 = (left.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();
        total_rms += buffer_rms;
    }

    let avg_rms = total_rms / num_buffers as f32;

    // Leak plugin to avoid VST3 cleanup crash
    plugin.leak();

    assert!(
        avg_rms > 0.001,
        "Sustained note should produce audio over multiple buffers, got avg RMS={}",
        avg_rms
    );
}

/// Test note-off stops audio
#[test]
fn test_vst3_note_off() {
    if !plugin_available("Surge XT") {
        eprintln!("Skipping test: Surge XT not installed");
        return;
    }

    let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
    plugin.initialize(44100.0, 512).unwrap();

    let note_on = MidiEvent::note_on(0, 0, 60, 100);
    let note_off = MidiEvent::note_off(0, 0, 60);

    // Send note on
    let mut left = vec![0.0f32; 512];
    let mut right = vec![0.0f32; 512];
    let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
    plugin.process_with_midi(&[note_on], &mut outputs, 512).unwrap();

    // Send note off
    let mut left = vec![0.0f32; 512];
    let mut right = vec![0.0f32; 512];
    let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
    plugin.process_with_midi(&[note_off], &mut outputs, 512).unwrap();

    // Process a few more buffers - audio should decay
    let mut final_rms = 0.0f32;
    for _ in 0..20 {
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
        plugin.process_with_midi(&[], &mut outputs, 512).unwrap();
        final_rms = (left.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();
    }

    // Leak plugin to avoid VST3 cleanup crash
    plugin.leak();

    // After note-off and decay, audio should be very quiet
    // (Depends on synth release time, but should be much quieter than sustained)
    assert!(
        final_rms < 0.1,
        "After note-off, audio should decay, got RMS={}",
        final_rms
    );
}

/// Test that both 0-input and 2-input plugins work in same session
#[test]
fn test_vst3_mixed_input_plugins() {
    let odin_available = plugin_available("Odin2");
    let surge_available = plugin_available("Surge XT");

    if !odin_available && !surge_available {
        eprintln!("Skipping test: No test plugins installed");
        return;
    }

    if odin_available {
        let mut plugin = create_real_plugin_by_name("Odin2").unwrap();
        plugin.initialize(44100.0, 512).unwrap();

        let note_on = MidiEvent::note_on(0, 0, 60, 100);
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        let result = plugin.process_with_midi(&[note_on], &mut outputs, 512);
        plugin.leak(); // Leak before assert to avoid crash
        assert!(result.is_ok(), "Odin2 should process without error");
    }

    if surge_available {
        let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
        plugin.initialize(44100.0, 512).unwrap();

        let note_on = MidiEvent::note_on(0, 0, 60, 100);
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        let result = plugin.process_with_midi(&[note_on], &mut outputs, 512);
        plugin.leak(); // Leak before assert to avoid crash
        assert!(result.is_ok(), "Surge XT should process without error");
    }
}

/// Test polyphonic MIDI (multiple simultaneous notes)
#[test]
fn test_vst3_polyphonic_notes() {
    if !plugin_available("Surge XT") {
        eprintln!("Skipping test: Surge XT not installed");
        return;
    }

    let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
    plugin.initialize(44100.0, 512).unwrap();

    // Send C major chord
    let c4 = MidiEvent::note_on(0, 0, 60, 100);
    let e4 = MidiEvent::note_on(0, 0, 64, 100);
    let g4 = MidiEvent::note_on(0, 0, 67, 100);

    let mut total_rms = 0.0f32;
    let num_buffers = 10;

    for i in 0..num_buffers {
        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];

        // Send all notes in first buffer
        let midi = if i == 0 {
            vec![c4.clone(), e4.clone(), g4.clone()]
        } else {
            vec![]
        };

        plugin.process_with_midi(&midi, &mut outputs, 512).unwrap();

        let buffer_rms: f32 = (left.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();
        total_rms += buffer_rms;
    }

    let avg_rms = total_rms / num_buffers as f32;

    // Leak plugin to avoid VST3 cleanup crash
    plugin.leak();

    assert!(
        avg_rms > 0.001,
        "Polyphonic chord should produce audio, got avg RMS={}",
        avg_rms
    );
}

/// Test parameter changes
#[test]
fn test_vst3_parameter_changes() {
    if !plugin_available("Surge XT") {
        eprintln!("Skipping test: Surge XT not installed");
        return;
    }

    let mut plugin = create_real_plugin_by_name("Surge XT").unwrap();
    plugin.initialize(44100.0, 512).unwrap();

    // Get parameter count
    let param_count = plugin.parameter_count();
    assert!(param_count > 0, "Plugin should have parameters");

    // Try to read and write a parameter
    if let Ok(original_value) = plugin.get_parameter(0) {
        // Set to a different value
        let new_value = if original_value > 0.5 { 0.2 } else { 0.8 };
        let _ = plugin.set_parameter(0, new_value);

        // Read back
        if let Ok(read_back) = plugin.get_parameter(0) {
            // Leak plugin to avoid VST3 cleanup crash
            plugin.leak();

            // Should be approximately equal (some plugins quantize)
            assert!(
                (read_back - new_value).abs() < 0.1,
                "Parameter should update: set={}, got={}",
                new_value,
                read_back
            );
        } else {
            plugin.leak();
        }
    } else {
        plugin.leak();
    }
}
