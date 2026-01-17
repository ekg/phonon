//! Test for note skipping in VST3 plugins
//!
//! Renders a 4-note pattern and verifies all notes are triggered.

#[cfg(feature = "vst3")]
fn main() {
    use phonon::plugin_host::{create_real_plugin_by_name, RealPluginScanner};
    use phonon::plugin_host::instance::MidiEvent;
    use std::path::PathBuf;

    println!("VST3 Note Skipping Test");
    println!("=======================\n");

    // Configuration
    let sample_rate = 44100.0f32;
    let block_size = 512;
    let cps = 0.5; // cycles per second (slow tempo for clear separation)
    let samples_per_cycle = (sample_rate / cps as f32) as usize;
    let num_cycles = 4;
    let total_samples = samples_per_cycle * num_cycles;

    println!("Sample rate: {} Hz", sample_rate);
    println!("CPS: {} (cycle = {} samples = {:.2}s)", cps, samples_per_cycle, samples_per_cycle as f32 / sample_rate);
    println!("Total: {} cycles, {} samples, {:.2}s\n", num_cycles, total_samples, total_samples as f32 / sample_rate);

    // Try to load Odin2
    let plugin_name = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .unwrap_or_else(|| "Odin2".to_string());

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

    // Initialize
    println!("Initializing plugin...");
    if let Err(e) = plugin.initialize(sample_rate, block_size) {
        eprintln!("Failed to initialize: {}", e);
        return;
    }
    println!("Plugin initialized!\n");

    // Define the 4-note pattern: c4 e4 g4 c5 (MIDI 60, 64, 67, 72)
    // Each note takes 1/4 of a cycle
    let notes = [60u8, 64, 67, 72]; // c4, e4, g4, c5
    let note_duration_samples = samples_per_cycle / 4;
    let note_names = ["c4", "e4", "g4", "c5"];

    println!("Pattern: {} {} {} {}", note_names[0], note_names[1], note_names[2], note_names[3]);
    println!("Note duration: {} samples ({:.3}s)\n", note_duration_samples, note_duration_samples as f32 / sample_rate);

    // Check flags
    let no_noteoff = std::env::args().any(|arg| arg == "--no-noteoff");
    let simple_mode = std::env::args().any(|arg| arg == "--simple");

    if no_noteoff {
        println!("*** Running WITHOUT note-offs ***\n");
    }
    if simple_mode {
        println!("*** SIMPLE MODE: One note per block, all at offset 0 ***\n");
    }

    // Generate all MIDI events for all cycles upfront
    let mut all_events: Vec<(usize, MidiEvent)> = Vec::new(); // (absolute_sample, event)

    if simple_mode {
        // Simple mode: put each note at the START of a block (offset 0)
        // Each note gets its own block to avoid any event queueing issues
        for cycle in 0..num_cycles {
            for (i, &note) in notes.iter().enumerate() {
                // Start each note at a block boundary
                let note_block = cycle * (samples_per_cycle / block_size) + i * (note_duration_samples / block_size);
                let note_start = note_block * block_size;
                all_events.push((note_start, MidiEvent::note_on(0, 0, note, 100)));
            }
        }
    } else {
        for cycle in 0..num_cycles {
            let cycle_start = cycle * samples_per_cycle;
            for (i, &note) in notes.iter().enumerate() {
                let note_start = cycle_start + i * note_duration_samples;
                let note_end = note_start + note_duration_samples - 100; // small gap before next note

                all_events.push((note_start, MidiEvent::note_on(0, 0, note, 100)));
                if !no_noteoff {
                    all_events.push((note_end, MidiEvent::note_off(0, 0, note)));
                }
            }
        }
    }

    // Sort by sample position
    all_events.sort_by_key(|(sample, _)| *sample);

    println!("Generated {} MIDI events ({} note-ons, {} note-offs)",
        all_events.len(), all_events.len() / 2, all_events.len() / 2);

    // Render audio
    let mut output_left = vec![0.0f32; total_samples];
    let mut output_right = vec![0.0f32; total_samples];

    let mut samples_processed = 0;
    let mut event_idx = 0;
    let mut notes_triggered = 0;

    println!("\nRendering audio...");

    while samples_processed < total_samples {
        let this_block = (total_samples - samples_processed).min(block_size);
        let block_end = samples_processed + this_block;

        // Collect MIDI events for this block
        let mut block_events: Vec<MidiEvent> = Vec::new();

        while event_idx < all_events.len() {
            let (event_sample, ref event) = all_events[event_idx];
            if event_sample >= samples_processed && event_sample < block_end {
                let mut ev = event.clone();
                // Check for --offset-zero flag to test if sample offsets are the issue
                let offset_zero = std::env::args().any(|arg| arg == "--offset-zero");
                ev.sample_offset = if offset_zero { 0 } else { event_sample - samples_processed };

                let event_type = if ev.is_note_on() { "ON " } else { "OFF" };
                println!("  Block {}: MIDI {} note {} @ offset {} (abs sample {})",
                    samples_processed / block_size, event_type, ev.data1, ev.sample_offset, event_sample);

                block_events.push(ev);

                if event.is_note_on() {
                    notes_triggered += 1;
                }
                event_idx += 1;
            } else if event_sample >= block_end {
                break;
            } else {
                event_idx += 1;
            }
        }

        // Process the block
        // Check for --one-at-a-time flag to send each event separately
        let one_at_a_time = std::env::args().any(|arg| arg == "--one-at-a-time");

        {
            let out_left = &mut output_left[samples_processed..block_end];
            let out_right = &mut output_right[samples_processed..block_end];

            if one_at_a_time && !block_events.is_empty() {
                // Process sample-by-sample, sending one event at a time
                for sample_idx in 0..this_block {
                    let events_for_sample: Vec<_> = block_events.iter()
                        .filter(|e| e.sample_offset == sample_idx)
                        .cloned()
                        .collect();

                    let mut out_l = [0.0f32; 1];
                    let mut out_r = [0.0f32; 1];
                    let mut outputs: Vec<&mut [f32]> = vec![&mut out_l[..], &mut out_r[..]];

                    if let Err(e) = plugin.process_with_midi(&events_for_sample, &mut outputs, 1) {
                        eprintln!("Process error: {}", e);
                    }

                    out_left[sample_idx] = out_l[0];
                    out_right[sample_idx] = out_r[0];
                }
            } else {
                let mut outputs: Vec<&mut [f32]> = vec![out_left, out_right];
                if let Err(e) = plugin.process_with_midi(&block_events, &mut outputs, this_block) {
                    eprintln!("Process error: {}", e);
                }
            }
        }

        samples_processed += this_block;
    }

    println!("Rendered {} samples", total_samples);
    println!("Notes triggered via MIDI: {}", notes_triggered);

    // Analyze audio by verifying frequencies at each note onset
    println!("\nVerifying note frequencies at each expected onset time...");

    // Helper function to estimate frequency using zero crossings
    fn estimate_frequency(samples: &[f32], sample_rate: f32, window_size: usize) -> f32 {
        if samples.len() < window_size {
            return 0.0;
        }
        let window = &samples[..window_size];
        let mut zero_crossings = 0;
        for i in 1..window.len() {
            if (window[i-1] < 0.0 && window[i] >= 0.0) || (window[i-1] >= 0.0 && window[i] < 0.0) {
                zero_crossings += 1;
            }
        }
        // Each complete cycle has 2 zero crossings
        let cycles = zero_crossings as f32 / 2.0;
        cycles * sample_rate / window_size as f32
    }

    // Expected frequencies for notes: c4=261.63, e4=329.63, g4=392.0, c5=523.25
    let expected_freqs = [261.63f32, 329.63, 392.0, 523.25];
    let freq_tolerance = 0.20; // 20% tolerance for frequency matching

    let mut notes_verified = 0;
    let mut notes_failed = 0;
    let window_size = 4096;

    println!("\n{:<6} {:<6} {:<10} {:<12} {:<12} {:<8}", "Cycle", "Note", "Sample", "Expected Hz", "Detected Hz", "Match?");
    println!("{}", "-".repeat(60));

    for cycle in 0..num_cycles {
        for (i, (&_note, &expected_freq)) in notes.iter().zip(expected_freqs.iter()).enumerate() {
            let note_start = cycle * samples_per_cycle + i * note_duration_samples;
            // Wait 1000 samples for note to stabilize
            let analysis_start = note_start + 1000;

            if analysis_start + window_size < total_samples {
                let detected_freq = estimate_frequency(
                    &output_left[analysis_start..analysis_start + window_size],
                    sample_rate,
                    window_size,
                );

                let match_ok = (detected_freq - expected_freq).abs() < expected_freq * freq_tolerance;
                let match_str = if match_ok { "YES" } else { "NO" };

                if match_ok {
                    notes_verified += 1;
                } else {
                    notes_failed += 1;
                }

                println!("{:<6} {:<6} {:<10} {:<12.1} {:<12.1} {:<8}",
                    cycle, note_names[i], note_start, expected_freq, detected_freq, match_str);
            }
        }
    }

    let expected_notes = num_cycles * notes.len();

    println!("\n=== RESULTS ===");
    println!("Expected notes: {}", expected_notes);
    println!("Notes verified (correct pitch): {}", notes_verified);
    println!("Notes failed (wrong pitch): {}", notes_failed);
    println!("MIDI note-ons sent: {}", notes_triggered);

    if notes_verified == expected_notes {
        println!("\n✅ PASS: All {} notes played at correct pitches!", expected_notes);
    } else if notes_failed > 0 {
        println!("\n❌ FAIL: {} notes had incorrect pitch (expected {}, verified {})",
            notes_failed, expected_notes, notes_verified);
    } else {
        println!("\n⚠️  WARN: Could not verify all notes (expected {}, verified {})",
            expected_notes, notes_verified);
    }

    // Save to WAV for manual inspection
    let output_path = PathBuf::from("/tmp/note_skipping_test.wav");
    println!("\nSaving audio to {}...", output_path.display());

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    match hound::WavWriter::create(&output_path, spec) {
        Ok(mut writer) => {
            for i in 0..total_samples {
                let _ = writer.write_sample(output_left[i]);
                let _ = writer.write_sample(output_right[i]);
            }
            let _ = writer.finalize();
            println!("Audio saved! Listen with: aplay {}", output_path.display());
        }
        Err(e) => {
            eprintln!("Failed to save WAV: {}", e);
        }
    }

    // Leak plugin to avoid cleanup crash
    plugin.leak();
}

#[cfg(not(feature = "vst3"))]
fn main() {
    eprintln!("VST3 feature not enabled. Rebuild with: cargo build --features vst3");
}
