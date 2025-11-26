//! Legato (note duration) capture tests
//!
//! Tests Phase 2: Note duration tracking and legato calculation
//! - Note-on → note-off duration tracking
//! - Legato value calculation (0.0 = staccato, 1.0 = full sustain)
//! - Pattern alignment with notes/velocities
//! - Mixed staccato/legato patterns

use phonon::midi_input::MidiRecorder;

/// Test staccato notes (short duration → low legato)
#[test]
fn test_staccato_notes() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // Record 4 staccato notes (very short duration)
    // At 120 BPM: 1 beat = 500ms = 500_000 us
    // Slot duration (quarter note) = 500_000 us
    // Short notes: 50_000 us (10% of slot) → legato ≈ 0.1

    // Note-on events
    recorder.record_event_at(60, 100, 0);         // C4 on at beat 0
    recorder.record_event_at(62, 100, 500_000);   // D4 on at beat 1
    recorder.record_event_at(64, 100, 1_000_000); // E4 on at beat 2
    recorder.record_event_at(65, 100, 1_500_000); // F4 on at beat 3

    // Note-off events (50ms after each note-on = 50_000 us)
    recorder.record_event_at(60, 0, 50_000);      // C4 off
    recorder.record_event_at(62, 0, 550_000);     // D4 off
    recorder.record_event_at(64, 0, 1_050_000);   // E4 off
    recorder.record_event_at(65, 0, 1_550_000);   // F4 off

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify notes captured correctly
    assert_eq!(recorded.notes, "c4 d4 e4 f4");

    // Verify legato values are low (staccato)
    let legato_values: Vec<f32> = recorded.legato
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(legato_values.len(), 4, "Should have 4 legato values");

    for (i, &legato) in legato_values.iter().enumerate() {
        assert!(legato < 0.2, "Staccato note {} should have low legato (< 0.2), got {}", i, legato);
        assert!(legato > 0.0, "Legato should be positive, got {}", legato);
    }

    println!("Staccato legato values: {:?}", legato_values);
}

/// Test legato notes (long duration → high legato)
#[test]
fn test_legato_notes() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // Record 4 legato notes (held nearly full duration)
    // At 120 BPM: 1 beat = 500ms = 500_000 us
    // Long notes: 450_000 us (90% of slot) → legato ≈ 0.9

    // Note-on events
    recorder.record_event_at(60, 100, 0);         // C4 on at beat 0
    recorder.record_event_at(62, 100, 500_000);   // D4 on at beat 1
    recorder.record_event_at(64, 100, 1_000_000); // E4 on at beat 2
    recorder.record_event_at(65, 100, 1_500_000); // F4 on at beat 3

    // Note-off events (450ms after each note-on = 450_000 us)
    recorder.record_event_at(60, 0, 450_000);      // C4 off
    recorder.record_event_at(62, 0, 950_000);      // D4 off
    recorder.record_event_at(64, 0, 1_450_000);    // E4 off
    recorder.record_event_at(65, 0, 1_950_000);    // F4 off

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify notes captured correctly
    assert_eq!(recorded.notes, "c4 d4 e4 f4");

    // Verify legato values are high (legato)
    let legato_values: Vec<f32> = recorded.legato
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(legato_values.len(), 4, "Should have 4 legato values");

    for (i, &legato) in legato_values.iter().enumerate() {
        assert!(legato > 0.8, "Legato note {} should have high legato (> 0.8), got {}", i, legato);
        assert!(legato <= 1.0, "Legato should be <= 1.0, got {}", legato);
    }

    println!("Legato values: {:?}", legato_values);
}

/// Test mixed staccato and legato notes
#[test]
fn test_mixed_articulation() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // Record alternating staccato/legato pattern
    // At 120 BPM: 1 beat = 500ms = 500_000 us

    // Note-on events
    recorder.record_event_at(60, 100, 0);         // C4 (staccato)
    recorder.record_event_at(62, 100, 500_000);   // D4 (legato)
    recorder.record_event_at(64, 100, 1_000_000); // E4 (staccato)
    recorder.record_event_at(65, 100, 1_500_000); // F4 (legato)

    // Note-off events
    recorder.record_event_at(60, 0, 75_000);      // C4 off (short: 15%)
    recorder.record_event_at(62, 0, 950_000);     // D4 off (long: 90%)
    recorder.record_event_at(64, 0, 1_100_000);   // E4 off (short: 20%)
    recorder.record_event_at(65, 0, 1_950_000);   // F4 off (long: 90%)

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify notes captured correctly
    assert_eq!(recorded.notes, "c4 d4 e4 f4");

    // Verify mixed legato values
    let legato_values: Vec<f32> = recorded.legato
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(legato_values.len(), 4, "Should have 4 legato values");

    // Note 0 (C4): staccato
    assert!(legato_values[0] < 0.3, "C4 should be staccato (< 0.3), got {}", legato_values[0]);

    // Note 1 (D4): legato
    assert!(legato_values[1] > 0.8, "D4 should be legato (> 0.8), got {}", legato_values[1]);

    // Note 2 (E4): staccato
    assert!(legato_values[2] < 0.3, "E4 should be staccato (< 0.3), got {}", legato_values[2]);

    // Note 3 (F4): legato
    assert!(legato_values[3] > 0.8, "F4 should be legato (> 0.8), got {}", legato_values[3]);

    println!("Mixed articulation legato values: {:?}", legato_values);
}

/// Test legato pattern alignment with rests
#[test]
fn test_legato_pattern_alignment_with_rests() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // Record pattern with gaps (rests)
    // Pattern: C4 ~ D4 ~ (note, rest, note, rest)

    // Note-on events (only beats 0 and 2)
    recorder.record_event_at(60, 100, 0);         // C4 on at beat 0
    recorder.record_event_at(62, 100, 1_000_000); // D4 on at beat 2

    // Note-off events
    recorder.record_event_at(60, 0, 400_000);      // C4 off (80% duration)
    recorder.record_event_at(62, 0, 1_100_000);    // D4 off (20% duration)

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify notes pattern has rests (no trailing rest)
    assert_eq!(recorded.notes, "c4 ~ d4");

    // Verify velocities pattern matches (with rests, no trailing rest)
    let velocity_parts: Vec<&str> = recorded.velocities.split_whitespace().collect();
    assert_eq!(velocity_parts.len(), 3, "Should have 3 velocity entries (including rest)");
    assert_eq!(velocity_parts[1], "~", "Second entry should be rest");

    // Verify legato pattern matches (with rests, no trailing rest)
    let legato_parts: Vec<&str> = recorded.legato.split_whitespace().collect();
    assert_eq!(legato_parts.len(), 3, "Should have 3 legato entries (including rest)");
    assert_eq!(legato_parts[1], "~", "Second legato entry should be rest");

    // Verify non-rest legato values
    let legato_c4: f32 = legato_parts[0].parse().unwrap();
    let legato_d4: f32 = legato_parts[2].parse().unwrap();

    assert!(legato_c4 > 0.7, "C4 should be legato (> 0.7), got {}", legato_c4);
    assert!(legato_d4 < 0.3, "D4 should be staccato (< 0.3), got {}", legato_d4);

    println!("Notes: {}", recorded.notes);
    println!("Velocities: {}", recorded.velocities);
    println!("Legato: {}", recorded.legato);
}

/// Test legato with multi-cycle recording
#[test]
fn test_legato_multi_cycle() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // Record pattern spanning 2 cycles (8 beats)
    // Cycle 1: staccato quarter notes (beats 0-3)
    // Cycle 2: legato quarter notes (beats 4-7)

    // Cycle 1: Staccato notes (short duration)
    recorder.record_event_at(60, 100, 0);         // C4 on
    recorder.record_event_at(60, 0, 50_000);      // C4 off (short)
    recorder.record_event_at(62, 100, 500_000);   // D4 on
    recorder.record_event_at(62, 0, 550_000);     // D4 off (short)
    recorder.record_event_at(64, 100, 1_000_000); // E4 on
    recorder.record_event_at(64, 0, 1_050_000);   // E4 off (short)
    recorder.record_event_at(65, 100, 1_500_000); // F4 on
    recorder.record_event_at(65, 0, 1_550_000);   // F4 off (short)

    // Cycle 2: Legato notes (long duration)
    recorder.record_event_at(67, 100, 2_000_000); // G4 on
    recorder.record_event_at(67, 0, 2_450_000);   // G4 off (long)
    recorder.record_event_at(69, 100, 2_500_000); // A4 on
    recorder.record_event_at(69, 0, 2_950_000);   // A4 off (long)
    recorder.record_event_at(71, 100, 3_000_000); // B4 on
    recorder.record_event_at(71, 0, 3_450_000);   // B4 off (long)
    recorder.record_event_at(72, 100, 3_500_000); // C5 on
    recorder.record_event_at(72, 0, 3_950_000);   // C5 off (long)

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify we recorded 2 cycles
    assert_eq!(recorded.cycle_count, 2, "Should span 2 cycles");

    // Verify all 8 notes captured
    assert_eq!(recorded.notes, "c4 d4 e4 f4 g4 a4 b4 c5");

    // Verify legato pattern
    let legato_values: Vec<f32> = recorded.legato
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(legato_values.len(), 8, "Should have 8 legato values");

    // First 4 notes: staccato
    for i in 0..4 {
        assert!(legato_values[i] < 0.2, "Note {} should be staccato, got {}", i, legato_values[i]);
    }

    // Last 4 notes: legato
    for i in 4..8 {
        assert!(legato_values[i] > 0.8, "Note {} should be legato, got {}", i, legato_values[i]);
    }

    println!("Multi-cycle legato values: {:?}", legato_values);
}

/// Test legato calculation clamping (doesn't exceed 1.0)
#[test]
fn test_legato_clamping() {
    let mut recorder = MidiRecorder::new(120.0); // 120 BPM
    recorder.set_quantize(4); // Quarter notes

    // Record note held LONGER than slot duration
    // At 120 BPM: 1 beat = 500ms = 500_000 us
    // Note held for 600_000 us (120% of slot) → should clamp to 1.0

    recorder.record_event_at(60, 100, 0);       // C4 on at beat 0
    recorder.record_event_at(60, 0, 600_000);   // C4 off (held beyond next slot)

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify legato is clamped to 1.0
    let legato_values: Vec<f32> = recorded.legato
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(legato_values.len(), 1, "Should have 1 legato value");
    assert_eq!(legato_values[0], 1.0, "Legato should be clamped to 1.0 for overlong note");

    println!("Clamped legato value: {}", legato_values[0]);
}
