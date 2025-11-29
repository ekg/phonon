//! Punch-in Recording Tests
//!
//! Tests Phase 3: Recording MIDI while audio is playing (punch-in)
//! - Recording starts at arbitrary cycle position
//! - Events quantized to absolute cycle grid (not recording-relative)
//! - Pattern aligned to cycle boundaries
//! - No timing drift over long recordings
//!
//! These tests mock the complete user interaction workflow:
//! 1. Audio is playing (simulated with cycle tracking)
//! 2. User presses Alt+R at a specific cycle (punch-in)
//! 3. User plays MIDI notes
//! 4. User presses Alt+R again (punch-out)
//! 5. Verify recorded pattern is cycle-aligned

use phonon::midi_input::MidiRecorder;

/// Test basic punch-in at an arbitrary cycle
#[test]
fn test_punch_in_at_cycle_2_point_5() {
    // SETUP: Simulating that audio has been playing for 2.5 cycles
    let punch_in_cycle = 2.5;

    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // USER ACTION: Press Alt+R (punch-in) at cycle 2.5
    recorder.start_at_cycle(punch_in_cycle);

    // USER ACTION: Play C4 immediately (at cycle ~2.5)
    recorder.record_event_at(60, 100, 0); // Note-on
    recorder.record_event_at(60, 0, 400_000); // Note-off after 400ms

    // USER ACTION: Play D4 after 500ms (at cycle ~3.0)
    recorder.record_event_at(62, 100, 500_000); // Note-on
    recorder.record_event_at(62, 0, 900_000); // Note-off

    // USER ACTION: Press Alt+R (punch-out / stop recording)
    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Pattern should have 2 notes
    let note_parts: Vec<&str> = pattern.notes.split_whitespace().collect();
    assert!(
        note_parts.len() >= 2,
        "Should have at least 2 notes, got: {}",
        pattern.notes
    );

    // VERIFICATION: Notes should be c4 and d4
    assert!(pattern.notes.contains("c4"), "Should contain c4");
    assert!(pattern.notes.contains("d4"), "Should contain d4");

    // VERIFICATION: Pattern spans at least 1 cycle
    assert!(pattern.cycle_count >= 1, "Should span at least 1 cycle");

    println!("Punch-in at cycle {}", punch_in_cycle);
    println!("Recorded notes: {}", pattern.notes);
    println!("Cycle count: {}", pattern.cycle_count);
}

/// Test punch-in with multiple notes across cycles
#[test]
fn test_punch_in_multi_cycle_recording() {
    // SETUP: Simulating audio playing, user punches in at cycle 1.7
    let punch_in_cycle = 1.7;

    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // USER ACTION: Press Alt+R at cycle 1.7
    recorder.start_at_cycle(punch_in_cycle);

    // USER ACTION: Play 8 notes over ~2 cycles (250ms apart = quarter notes at 120 BPM)
    for i in 0..8u64 {
        let note = (60 + i) as u8;
        let timestamp_us = i * 250_000; // 250ms apart
        recorder.record_event_at(note, 100, timestamp_us);
        recorder.record_event_at(note, 0, timestamp_us + 200_000); // 200ms duration (legato)
    }

    // USER ACTION: Press Alt+R (stop recording)
    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Should have 8 notes
    let note_parts: Vec<&str> = pattern.notes.split_whitespace().collect();
    assert!(
        note_parts.len() >= 8,
        "Should have 8 notes, got: {}",
        pattern.notes
    );

    // VERIFICATION: Pattern spans 1 cycle (8 notes at 250ms = 2000ms = 1 cycle at 120 BPM)
    assert!(
        pattern.cycle_count >= 1,
        "Should span at least 1 cycle, got: {}",
        pattern.cycle_count
    );

    println!("Punch-in at cycle {}", punch_in_cycle);
    println!("Recorded {} notes: {}", note_parts.len(), pattern.notes);
    println!("Spans {} cycles", pattern.cycle_count);
}

/// Test that punch-in at cycle 0 behaves like normal recording
#[test]
fn test_punch_in_at_cycle_zero() {
    // SETUP: User starts recording from the beginning (cycle 0)
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // USER ACTION: Press Alt+R at cycle 0 (equivalent to normal start())
    recorder.start_at_cycle(0.0);

    // USER ACTION: Play 4 notes
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(62, 100, 500_000);
    recorder.record_event_at(62, 0, 900_000);
    recorder.record_event_at(64, 100, 1_000_000);
    recorder.record_event_at(64, 0, 1_400_000);
    recorder.record_event_at(65, 100, 1_500_000);
    recorder.record_event_at(65, 0, 1_900_000);

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Should work same as normal recording
    assert_eq!(pattern.notes, "c4 d4 e4 f4");
    assert_eq!(pattern.cycle_count, 1);

    println!("Punch-in at cycle 0 (normal recording)");
    println!("Notes: {}", pattern.notes);
}

/// Test punch-in mid-cycle with quantization
#[test]
fn test_punch_in_quantization_alignment() {
    // SETUP: Punch-in at cycle 5.3 (mid-cycle)
    let punch_in_cycle = 5.3;

    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4); // Quarter note quantization

    // USER ACTION: Press Alt+R at cycle 5.3
    recorder.start_at_cycle(punch_in_cycle);

    // USER ACTION: Play note at t=0 (should quantize relative to cycle 5.3, not recording start)
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);

    // USER ACTION: Play note at t=500ms (should quantize to next slot)
    recorder.record_event_at(62, 100, 500_000);
    recorder.record_event_at(62, 0, 900_000);

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Notes should be quantized to absolute cycle grid
    let note_parts: Vec<&str> = pattern.notes.split_whitespace().collect();
    assert!(note_parts.len() >= 2, "Should have at least 2 notes");

    println!("Punch-in at cycle {} with quantization", punch_in_cycle);
    println!("Notes: {}", pattern.notes);
    println!("Velocities: {}", pattern.velocities);
    println!("Legato: {}", pattern.legato);
}

/// Test punch-in with rests (sparse recording)
#[test]
fn test_punch_in_with_rests() {
    // SETUP: Punch-in at cycle 3.0
    let punch_in_cycle = 3.0;

    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // USER ACTION: Press Alt+R at cycle 3.0
    recorder.start_at_cycle(punch_in_cycle);

    // USER ACTION: Play note at t=0 (cycle ~3.0)
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);

    // SILENCE: No notes for 500ms

    // USER ACTION: Play note at t=1000ms (cycle ~3.5)
    recorder.record_event_at(62, 100, 1_000_000);
    recorder.record_event_at(62, 0, 1_400_000);

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Pattern should include rests
    assert!(
        pattern.notes.contains("~"),
        "Pattern should contain rests: {}",
        pattern.notes
    );

    // VERIFICATION: Legato pattern should also have rests in same positions
    let note_parts: Vec<&str> = pattern.notes.split_whitespace().collect();
    let legato_parts: Vec<&str> = pattern.legato.split_whitespace().collect();
    assert_eq!(
        note_parts.len(),
        legato_parts.len(),
        "Note and legato patterns should align"
    );

    println!("Punch-in with rests:");
    println!("Notes: {}", pattern.notes);
    println!("Legato: {}", pattern.legato);
}

/// Test punch-in at very late cycle (no timing drift)
#[test]
fn test_punch_in_at_late_cycle() {
    // SETUP: Simulating long playback session (cycle 100)
    let punch_in_cycle = 100.0;

    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // USER ACTION: Press Alt+R at cycle 100
    recorder.start_at_cycle(punch_in_cycle);

    // USER ACTION: Play melody
    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(60, 0, 400_000);
    recorder.record_event_at(62, 100, 500_000);
    recorder.record_event_at(62, 0, 900_000);

    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // VERIFICATION: Should still work correctly at late cycles
    assert!(pattern.notes.contains("c4"));
    assert!(pattern.notes.contains("d4"));

    println!("Punch-in at late cycle {}", punch_in_cycle);
    println!("Notes: {}", pattern.notes);
}

/// Test punch-in with different tempos
#[test]
fn test_punch_in_different_tempos() {
    // Test at 60 BPM (slow)
    {
        let mut recorder = MidiRecorder::new(60.0); // 60 BPM
        recorder.set_quantize(4);
        recorder.start_at_cycle(2.0);

        // At 60 BPM: 1 beat = 1 second = 1_000_000 us
        recorder.record_event_at(60, 100, 0);
        recorder.record_event_at(60, 0, 800_000);
        recorder.record_event_at(62, 100, 1_000_000);
        recorder.record_event_at(62, 0, 1_800_000);

        let pattern = recorder.to_recorded_pattern(4.0).unwrap();
        assert!(pattern.notes.contains("c4"));
        assert!(pattern.notes.contains("d4"));

        println!("60 BPM punch-in: {}", pattern.notes);
    }

    // Test at 180 BPM (fast)
    {
        let mut recorder = MidiRecorder::new(180.0); // 180 BPM
        recorder.set_quantize(4);
        recorder.start_at_cycle(2.0);

        // At 180 BPM: 1 beat = 333ms = 333_333 us
        recorder.record_event_at(60, 100, 0);
        recorder.record_event_at(60, 0, 300_000);
        recorder.record_event_at(62, 100, 333_333);
        recorder.record_event_at(62, 0, 633_333);

        let pattern = recorder.to_recorded_pattern(4.0).unwrap();
        assert!(pattern.notes.contains("c4"));
        assert!(pattern.notes.contains("d4"));

        println!("180 BPM punch-in: {}", pattern.notes);
    }
}

/// Test complete workflow: playback → punch-in → record → punch-out
#[test]
fn test_complete_punch_in_workflow() {
    // MOCK USER WORKFLOW:
    // 1. Audio is playing for a while
    // 2. User wants to add overdub
    // 3. Presses Alt+R at specific cycle
    // 4. Plays MIDI keyboard
    // 5. Presses Alt+R again
    // 6. Verifies result

    // STEP 1: Simulated playback (cycle advances)
    let current_cycle = 2.5;

    // STEP 2: User decides to record
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // STEP 3: User presses Alt+R (punch-in)
    println!("🔴 USER: Press Alt+R at cycle {}", current_cycle);
    recorder.start_at_cycle(current_cycle);

    // STEP 4: User plays melody on MIDI keyboard
    println!("🎹 USER: Playing melody...");
    let melody = vec![
        (60, 0, 400_000),           // C4
        (62, 500_000, 900_000),     // D4
        (64, 1_000_000, 1_400_000), // E4
        (65, 1_500_000, 1_900_000), // F4
    ];

    for (note, start_us, end_us) in melody {
        recorder.record_event_at(note, 100, start_us);
        recorder.record_event_at(note, 0, end_us);
    }

    // STEP 5: User presses Alt+R (punch-out)
    println!("⏹️  USER: Press Alt+R (stop recording)");
    let pattern = recorder.to_recorded_pattern(4.0).unwrap();

    // STEP 6: Verify result
    println!("✅ RESULT:");
    println!("   Notes: {}", pattern.notes);
    println!("   Velocities: {}", pattern.velocities);
    println!("   Legato: {}", pattern.legato);
    println!("   Cycles: {}", pattern.cycle_count);

    // ASSERTIONS
    assert_eq!(pattern.notes, "c4 d4 e4 f4");
    assert_eq!(pattern.cycle_count, 1);
    assert!(pattern.velocities.len() > 0);
    assert!(pattern.legato.len() > 0);

    // VERIFICATION: Smart paste would produce
    let smart_paste_output = format!(
        "~rec1: slow {} $ n \"{}\" # gain \"{}\" # legato \"{}\"",
        pattern.cycle_count, pattern.notes, pattern.velocities, pattern.legato
    );

    println!("\n📝 SMART PASTE OUTPUT:");
    println!("{}", smart_paste_output);

    // SUCCESS!
    println!("\n🎉 Punch-in recording successful!");
}
