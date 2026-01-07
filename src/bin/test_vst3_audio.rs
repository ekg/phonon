//! Test VST3 Plugin Audio Generation
//!
//! Loads a VST3 plugin, plays a note, and saves the audio to a file.

#[cfg(feature = "vst3")]
fn main() {
    use phonon::plugin_host::{create_real_plugin_by_name, RealPluginScanner};
    use std::path::PathBuf;

    println!("VST3 Plugin Audio Test");
    println!("======================\n");

    // Configuration
    let sample_rate = 44100.0f32;
    let block_size = 512;
    let duration_secs = 2.0;
    let total_samples = (sample_rate * duration_secs) as usize;

    // Try to create a plugin instance
    // Use Surge XT as it's a known working plugin
    let plugin_name = std::env::args().nth(1).unwrap_or_else(|| "Surge XT".to_string());

    println!("Loading plugin: {}...", plugin_name);

    let mut plugin = match create_real_plugin_by_name(&plugin_name) {
        Ok(p) => {
            println!("Plugin loaded: {}", p.info().id.name);
            p
        }
        Err(e) => {
            eprintln!("Failed to load plugin: {}", e);
            eprintln!("\nAvailable plugins:");
            if let Ok(scanner) = RealPluginScanner::new() {
                if let Ok(plugins) = scanner.scan() {
                    for p in &plugins {
                        println!("  - {}", p.name);
                    }
                }
            }
            return;
        }
    };

    // Initialize the plugin
    println!("Initializing plugin at {} Hz, {} samples...", sample_rate, block_size);
    if let Err(e) = plugin.initialize(sample_rate, block_size) {
        eprintln!("Failed to initialize plugin: {}", e);
        return;
    }
    println!("Plugin initialized successfully!\n");

    // Print parameter info
    let param_count = plugin.parameter_count();
    println!("Plugin has {} parameters", param_count);
    if param_count > 0 {
        println!("First 5 parameters:");
        for i in 0..param_count.min(5) {
            if let Ok(info) = plugin.parameter_info(i) {
                if let Ok(value) = plugin.get_parameter(i) {
                    println!("  {}: {} = {:.3} (range: {:.3} - {:.3})",
                        i, info.name, value, info.min_value, info.max_value);
                }
            }
        }
        println!();
    }

    // Generate audio
    println!("Generating {} seconds of audio...", duration_secs);

    let mut output_left = vec![0.0f32; total_samples];
    let mut output_right = vec![0.0f32; total_samples];

    // Create MIDI note-on event
    use phonon::plugin_host::instance::MidiEvent;
    let note_on = MidiEvent::note_on(0, 0, 60, 100); // Middle C, velocity 100

    let mut samples_processed = 0;
    let note_off_sample = (sample_rate * 0.5) as usize; // Note off after 0.5 seconds

    while samples_processed < total_samples {
        let this_block = (total_samples - samples_processed).min(block_size);

        // Collect MIDI events for this block
        let mut midi_events = Vec::new();

        // Note on at sample 0
        if samples_processed == 0 {
            midi_events.push(note_on.clone());
        }

        // Note off at appropriate time
        if samples_processed <= note_off_sample && samples_processed + this_block > note_off_sample {
            let offset = note_off_sample - samples_processed;
            midi_events.push(MidiEvent::note_off(offset, 0, 60));
        }

        // Process the block
        {
            let out_slice_left = &mut output_left[samples_processed..samples_processed + this_block];
            let out_slice_right = &mut output_right[samples_processed..samples_processed + this_block];
            let mut outputs: Vec<&mut [f32]> = vec![out_slice_left, out_slice_right];

            if let Err(e) = plugin.process_with_midi(&midi_events, &mut outputs, this_block) {
                eprintln!("Error processing audio: {}", e);
                break;
            }
        }

        samples_processed += this_block;
    }

    // Calculate RMS
    let rms_left: f32 = (output_left.iter().map(|s| s * s).sum::<f32>() / total_samples as f32).sqrt();
    let rms_right: f32 = (output_right.iter().map(|s| s * s).sum::<f32>() / total_samples as f32).sqrt();

    println!("Audio generated!");
    println!("  Left channel RMS: {:.6}", rms_left);
    println!("  Right channel RMS: {:.6}", rms_right);

    // Check if we got any audio
    if rms_left > 0.001 || rms_right > 0.001 {
        println!("\nPlugin is generating audio successfully!");

        // Save to WAV file
        let output_path = PathBuf::from(format!("/tmp/{}_test.wav", plugin_name.replace(' ', "_")));

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        match hound::WavWriter::create(&output_path, spec) {
            Ok(mut writer) => {
                for i in 0..total_samples {
                    writer.write_sample(output_left[i]).unwrap();
                    writer.write_sample(output_right[i]).unwrap();
                }
                writer.finalize().unwrap();
                println!("\nSaved to: {}", output_path.display());
            }
            Err(e) => {
                eprintln!("Failed to save WAV: {}", e);
            }
        }
    } else {
        println!("\nWARNING: Plugin produced silence!");
        println!("This may be expected for some plugins that need specific initialization.");
    }

    // Leak the plugin to prevent double-free crash on exit
    // This is a workaround for VST3 SDK cleanup issues
    println!("\nLeaking plugin to prevent cleanup crash...");
    plugin.leak();
}

#[cfg(not(feature = "vst3"))]
fn main() {
    println!("VST3 support not enabled. Build with: cargo run --bin test_vst3_audio --features vst3");
}
