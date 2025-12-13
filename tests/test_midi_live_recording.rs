//! MIDI Live Recording Integration Tests
//!
//! Tests the complete MIDI recording workflow:
//! 1. Fake MIDI events → MidiRecorder
//! 2. Pattern generation (notes, n-offsets, velocity, legato)
//! 3. Pattern alignment with cycles
//! 4. Multi-cycle recording with slow wrapper

use phonon::midi_input::MidiRecorder;

/// Simulate playing a C major chord and recording it
#[test]
fn test_record_chord_to_pattern() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(16); // 16th note quantization
    recorder.start();

    // Simulate playing C major chord on beat 0
    // At 120 BPM, 1 beat = 500,000 microseconds
    recorder.record_event_at(60, 100, 0);       // C4 - note on
    recorder.record_event_at(64, 80, 0);        // E4 - note on
    recorder.record_event_at(67, 90, 0);        // G4 - note on

    // Note offs after half a beat (250ms)
    recorder.record_event_at(60, 0, 250_000);   // C4 - note off
    recorder.record_event_at(64, 0, 250_000);   // E4 - note off
    recorder.record_event_at(67, 0, 250_000);   // G4 - note off

    // Generate pattern (4 beats per cycle)
    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Should produce a chord notation
    assert!(recorded.notes.contains('['), "Should have chord: {}", recorded.notes);
    assert!(recorded.notes.contains("c4"), "Should contain c4: {}", recorded.notes);
    assert!(recorded.notes.contains("e4"), "Should contain e4: {}", recorded.notes);
    assert!(recorded.notes.contains("g4"), "Should contain g4: {}", recorded.notes);

    // N-offsets should be [0 4 7] (C major triad)
    assert!(recorded.n_offsets.contains('0'), "Should have offset 0: {}", recorded.n_offsets);
    assert!(recorded.n_offsets.contains('4'), "Should have offset 4: {}", recorded.n_offsets);
    assert!(recorded.n_offsets.contains('7'), "Should have offset 7: {}", recorded.n_offsets);

    // Base note should be C4 (MIDI 60)
    assert_eq!(recorded.base_note, 60);
    assert_eq!(recorded.base_note_name, "c4");

    println!("Notes: {}", recorded.notes);
    println!("N-offsets: {}", recorded.n_offsets);
    println!("Velocities: {}", recorded.velocities);
    println!("Legato: {}", recorded.legato);
}

/// Simulate playing a melody across one cycle
#[test]
fn test_record_melody_single_cycle() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4); // Quarter note quantization
    recorder.start();

    // At 120 BPM, 1 beat = 500,000 us, 4 beats per cycle = 2,000,000 us
    // Play C, E, G, C (one octave up) on each beat

    // Beat 0: C4
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);

    // Beat 1: E4
    recorder.record_event_at(64, 90, 500_000);
    recorder.record_event_at(64, 0, 900_000);

    // Beat 2: G4
    recorder.record_event_at(67, 80, 1_000_000);
    recorder.record_event_at(67, 0, 1_400_000);

    // Beat 3: C5
    recorder.record_event_at(72, 110, 1_500_000);
    recorder.record_event_at(72, 0, 1_900_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Should have exactly 4 notes, no rests
    assert_eq!(recorded.notes, "c4 e4 g4 c5", "Pattern: {}", recorded.notes);
    assert_eq!(recorded.n_offsets, "0 4 7 12", "N-offsets: {}", recorded.n_offsets);
    assert_eq!(recorded.cycle_count, 1);

    println!("Single cycle melody: {}", recorded.notes);
}

/// Simulate playing across multiple cycles
#[test]
fn test_record_multi_cycle() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4); // Quarter notes
    recorder.start();

    // At 120 BPM: 1 beat = 500,000 us
    // 4 beats per cycle = 2,000,000 us per cycle

    // Cycle 1 (0-2s): C4, E4, G4, C5
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(64, 100, 500_000);
    recorder.record_event_at(64, 0, 900_000);
    recorder.record_event_at(67, 100, 1_000_000);
    recorder.record_event_at(67, 0, 1_400_000);
    recorder.record_event_at(72, 100, 1_500_000);
    recorder.record_event_at(72, 0, 1_900_000);

    // Cycle 2 (2s-4s): Same pattern, starting at 2,000,000 us
    recorder.record_event_at(60, 100, 2_000_000);
    recorder.record_event_at(60, 0, 2_400_000);
    recorder.record_event_at(64, 100, 2_500_000);
    recorder.record_event_at(64, 0, 2_900_000);
    recorder.record_event_at(67, 100, 3_000_000);
    recorder.record_event_at(67, 0, 3_400_000);
    recorder.record_event_at(72, 100, 3_500_000);
    recorder.record_event_at(72, 0, 3_900_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Should span 2 cycles
    assert_eq!(recorded.cycle_count, 2, "Should be 2 cycles");

    // Should have 8 notes total
    let note_count = recorded.notes.split_whitespace()
        .filter(|s| !s.starts_with('~'))
        .count();
    assert_eq!(note_count, 8, "Should have 8 notes: {}", recorded.notes);

    // The pattern with slow wrapper would be:
    // slow 2 $ n "c4 e4 g4 c5 c4 e4 g4 c5"
    println!("Multi-cycle pattern: slow {} $ n \"{}\"", recorded.cycle_count, recorded.notes);
}

/// Test velocity pattern captures dynamics
#[test]
fn test_velocity_pattern_dynamics() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Crescendo: p → mp → mf → f
    recorder.record_event_at(60, 40, 0);           // p
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(62, 70, 500_000);     // mp
    recorder.record_event_at(62, 0, 900_000);
    recorder.record_event_at(64, 100, 1_000_000);  // mf
    recorder.record_event_at(64, 0, 1_400_000);
    recorder.record_event_at(65, 127, 1_500_000);  // f
    recorder.record_event_at(65, 0, 1_900_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Parse velocities
    let velocities: Vec<f32> = recorded.velocities
        .split_whitespace()
        .filter(|s| !s.starts_with('~'))
        .map(|v| v.parse().unwrap())
        .collect();

    assert_eq!(velocities.len(), 4);

    // Should be increasing
    for i in 0..3 {
        assert!(velocities[i] < velocities[i + 1],
            "Velocity should increase: {:?}", velocities);
    }

    println!("Velocity pattern: {}", recorded.velocities);
}

/// Test pattern with rests (syncopation)
#[test]
fn test_pattern_with_rests() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Play on beats 0 and 2 only (rest on 1 and 3)
    recorder.record_event_at(60, 100, 0);           // Beat 0
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(67, 100, 1_000_000);   // Beat 2
    recorder.record_event_at(67, 0, 1_400_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Should have pattern with rest in between
    assert!(recorded.notes.contains('~'), "Should have rest: {}", recorded.notes);
    assert_eq!(recorded.notes, "c4 ~ g4", "Pattern: {}", recorded.notes);

    println!("Syncopated pattern: {}", recorded.notes);
}

/// Test legato tracking (note duration)
#[test]
fn test_legato_pattern() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Short note (staccato) - 10% of beat duration
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 50_000);  // Short!

    // Long note (legato) - 90% of beat duration
    recorder.record_event_at(64, 100, 500_000);
    recorder.record_event_at(64, 0, 950_000);  // Long!

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Parse legato values
    let legato: Vec<f32> = recorded.legato
        .split_whitespace()
        .filter(|s| !s.starts_with('~'))
        .map(|v| v.parse().unwrap())
        .collect();

    assert_eq!(legato.len(), 2);

    // First note should be short (staccato)
    assert!(legato[0] < 0.5, "First should be staccato: {}", legato[0]);

    // Second note should be long (legato)
    assert!(legato[1] > 0.5, "Second should be legato: {}", legato[1]);

    println!("Legato pattern: {}", recorded.legato);
}

/// Test punch-in at cycle boundary
#[test]
fn test_punch_in_recording() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // Start recording at cycle 2.5 (halfway through cycle 3)
    recorder.start_at_cycle(2.5);

    // Record notes relative to start time
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(64, 100, 500_000);
    recorder.record_event_at(64, 0, 900_000);

    let start_cycle = recorder.get_recording_start_cycle();
    assert!((start_cycle - 2.5).abs() < 0.01, "Start cycle should be 2.5");

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();
    assert_eq!(recorded.notes, "c4 e4");

    println!("Punch-in from cycle {}: {}", start_cycle, recorded.notes);
}

/// Test 16th note hi-hat pattern
#[test]
fn test_16th_note_pattern() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(16); // 16th note resolution
    recorder.start();

    // At 120 BPM: 1 beat = 500,000 us
    // 1/16th note = 125,000 us
    // Hi-hat pattern: x-x-x-x-x-x-x-x (every other 16th)

    let hi_hat = 42u8; // MIDI note for closed hi-hat
    for i in 0..8 {
        let timestamp = (i * 2) * 125_000; // Every other 16th
        recorder.record_event_at(hi_hat, 100, timestamp as u64);
        recorder.record_event_at(hi_hat, 0, timestamp as u64 + 100_000);
    }

    let recorded = recorder.to_recorded_pattern(1.0).unwrap(); // 1 beat per cycle

    // Should have alternating notes and rests
    let parts: Vec<&str> = recorded.notes.split_whitespace().collect();

    // Verify we got 8 notes with rests between them
    let note_count = parts.iter().filter(|&&s| s == "fs2").count(); // MIDI 42 = F#2
    assert_eq!(note_count, 8, "Should have 8 hi-hats: {:?}", parts);

    println!("16th note hi-hat: {}", recorded.notes);
}

/// Test heavy polyphonic input - many simultaneous notes
/// This validates that the recording system handles 10+ notes at once
#[test]
fn test_heavy_polyphonic_input() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Play a 10-note cluster chord (like slamming fist on keyboard)
    // All notes start at the same time
    let cluster_notes: Vec<u8> = vec![60, 62, 64, 65, 67, 69, 71, 72, 74, 76]; // C4 to E5
    let velocities: Vec<u8> = vec![100, 90, 80, 95, 85, 92, 88, 110, 78, 105];

    // All notes start at beat 0
    for (&note, &vel) in cluster_notes.iter().zip(velocities.iter()) {
        recorder.record_event_at(note, vel, 0);
    }

    // All notes release at beat 2
    for &note in cluster_notes.iter() {
        recorder.record_event_at(note, 0, 1_000_000);
    }

    // Second chord at beat 2 - different voicing
    let chord2: Vec<u8> = vec![55, 59, 62, 67, 71]; // G3 major 9
    for &note in chord2.iter() {
        recorder.record_event_at(note, 100, 1_000_000);
    }
    for &note in chord2.iter() {
        recorder.record_event_at(note, 0, 1_900_000);
    }

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // First chord should be a 10-note cluster
    assert!(recorded.notes.contains('['), "Should have chord notation: {}", recorded.notes);

    // Count opening brackets - should have 2 chords
    let chord_count = recorded.notes.matches('[').count();
    assert_eq!(chord_count, 2, "Should have 2 chords, got: {}", recorded.notes);

    // Verify all notes from first chord are present
    assert!(recorded.notes.contains("c4"), "Should have c4: {}", recorded.notes);
    assert!(recorded.notes.contains("e5"), "Should have e5: {}", recorded.notes);

    // Verify n_offsets capture the intervals
    println!("Heavy polyphonic test:");
    println!("  Notes: {}", recorded.notes);
    println!("  N-offsets: {}", recorded.n_offsets);
    println!("  Chord count: {}", chord_count);
}

/// Test rapid-fire polyphonic input (fast arpeggios across multiple octaves)
#[test]
fn test_rapid_arpeggio_polyphony() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(16); // 16th note for fast patterns
    recorder.start();

    // Simulate a very fast arpeggio spanning 3 octaves
    // At 120 BPM, 16th = 125,000 us
    // Each note overlaps the previous (legato arpeggio)
    let arpeggio: Vec<u8> = vec![
        48, 52, 55, 60, 64, 67, 72, 76, 79, 84, // C3 to C6 (major arpeggio)
        79, 76, 72, 67, 64, 60, 55, 52, 48,     // Back down
    ];

    let mut time = 0u64;
    let step = 62_500u64; // 32nd note intervals for overlap

    for (i, &note) in arpeggio.iter().enumerate() {
        // Note on
        recorder.record_event_at(note, 100, time);

        // Note off after 2 steps (creates overlap)
        if i > 0 {
            let prev_note = arpeggio[i - 1];
            recorder.record_event_at(prev_note, 0, time);
        }

        time += step;
    }

    // Final note off
    recorder.record_event_at(*arpeggio.last().unwrap(), 0, time);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Should have many notes
    let note_count: usize = recorded.notes
        .split_whitespace()
        .filter(|s| !s.starts_with('~') && !s.starts_with('[') && !s.starts_with(']'))
        .count();

    assert!(note_count >= 10, "Should have 10+ notes, got {}: {}", note_count, recorded.notes);

    println!("Rapid arpeggio test:");
    println!("  Notes: {}", recorded.notes);
    println!("  Total note events: {}", arpeggio.len());
}

/// Test complete workflow: record → generate smart paste format
#[test]
fn test_smart_paste_format() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Record a simple 4-note pattern over exactly 1 cycle
    // At 120 BPM: 1 beat = 500,000 us, 4 beats per cycle = 2,000,000 us
    recorder.record_event_at(60, 80, 0);           // Beat 0: C4
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(64, 90, 500_000);     // Beat 1: E4
    recorder.record_event_at(64, 0, 900_000);
    recorder.record_event_at(67, 100, 1_000_000);  // Beat 2: G4
    recorder.record_event_at(67, 0, 1_400_000);
    recorder.record_event_at(72, 110, 1_500_000);  // Beat 3: C5
    recorder.record_event_at(72, 0, 1_900_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Build smart paste format (like insert_midi_smart_paste does)
    let slow_wrapper = if recorded.cycle_count > 1 {
        format!("slow {} $ ", recorded.cycle_count)
    } else {
        String::new()
    };

    let smart_paste = format!(
        "~rec1: {}n \"{}\" # gain \"{}\" # legato \"{}\"",
        slow_wrapper,
        recorded.notes,
        recorded.velocities,
        recorded.legato
    );

    // Single cycle - no slow wrapper needed
    assert_eq!(recorded.cycle_count, 1, "Should be exactly 1 cycle");
    assert!(smart_paste.contains("~rec1:"), "Should have bus name");
    assert!(smart_paste.contains("# gain"), "Should have gain pattern");
    assert!(smart_paste.contains("# legato"), "Should have legato pattern");
    assert!(smart_paste.contains("c4 e4 g4 c5"), "Should have 4 notes: {}", smart_paste);

    println!("Smart paste format:\n{}", smart_paste);
}

/// Test live preview with currently held notes
#[test]
fn test_live_preview_held_notes() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Record note-on without note-off (simulating held keys)
    recorder.record_event_at(60, 100, 0);  // C4 held
    recorder.record_event_at(64, 90, 0);   // E4 held
    recorder.record_event_at(67, 80, 0);   // G4 held

    // Get currently held notes (should show all 3)
    let held = recorder.get_currently_held_notes();
    assert_eq!(held.len(), 3, "Should have 3 held notes");
    assert!(held.contains(&60), "Should contain C4");
    assert!(held.contains(&64), "Should contain E4");
    assert!(held.contains(&67), "Should contain G4");

    // Test string format
    let held_str = recorder.get_currently_held_notes_string();
    assert!(held_str.contains('['), "Should be chord notation: {}", held_str);
    assert!(held_str.contains("c4"), "Should contain c4: {}", held_str);

    // Release one note
    recorder.record_event_at(64, 0, 100_000);  // Release E4
    let held2 = recorder.get_currently_held_notes();
    assert_eq!(held2.len(), 2, "Should have 2 held notes after release");
    assert!(!held2.contains(&64), "E4 should be released");

    println!("Held notes: {:?}", held);
    println!("Held string: {}", held_str);
}

/// Test live preview structure
#[test]
fn test_live_preview_structure() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Record a few notes
    recorder.record_event_at(60, 100, 0);      // C4
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(64, 90, 500_000); // E4
    recorder.record_event_at(64, 0, 900_000);

    let preview = recorder.live_preview(4.0);

    // Verify preview fields
    assert!(preview.current_cycle >= 1, "Should have current cycle");
    assert_eq!(preview.note_count, 2, "Should have 2 notes recorded");
    assert!(preview.pattern_preview.contains("c4"), "Preview should contain c4");
    assert!(preview.pattern_preview.contains("e4"), "Preview should contain e4");
    assert!(preview.elapsed_secs > 0.0, "Should have elapsed time");

    println!("Live preview: {:?}", preview);
}

/// Test code preview generation (for auto-insert)
#[test]
fn test_generate_code_preview() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);
    recorder.start();

    // Single cycle recording
    recorder.record_event_at(60, 100, 0);      // C4
    recorder.record_event_at(60, 0, 400_000);

    let code = recorder.generate_code_preview(4.0, "~rec1");
    assert!(code.starts_with("~rec1 $"), "Should start with bus name: {}", code);
    assert!(code.contains("n \""), "Should have n pattern: {}", code);
    assert!(!code.contains("slow"), "Single cycle should not have slow: {}", code);

    println!("Code preview (1 cycle): {}", code);

    // Multi-cycle recording - simulate 2+ cycles worth of beats
    // At 120 BPM, 4 beats per cycle = 2 seconds = 2,000,000 us
    recorder.record_event_at(64, 90, 2_500_000);  // Beat in cycle 2
    recorder.record_event_at(64, 0, 2_900_000);

    let code2 = recorder.generate_code_preview(4.0, "~rec2");
    assert!(code2.contains("slow"), "Multi-cycle should have slow: {}", code2);
    assert!(code2.contains("slow 2"), "Should be slow 2: {}", code2);

    println!("Code preview (2 cycles): {}", code2);
}
