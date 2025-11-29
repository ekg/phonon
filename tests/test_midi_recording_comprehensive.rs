//! Comprehensive MIDI recording tests
//!
//! Tests the full MIDI recording workflow including:
//! - Note capture with timing
//! - Velocity capture
//! - N-offset pattern generation
//! - Multi-cycle recording
//! - Integration with pattern playback

use phonon::midi_input::{MidiRecorder, RecordedPattern};

/// Test velocity capture preserves dynamics
#[test]
fn test_velocity_capture_soft_to_loud() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4); // Quarter notes

    // Crescendo: gradually increasing velocity
    recorder.record_event_at(60, 30, 0); // Beat 0: ppp
    recorder.record_event_at(62, 60, 500_000); // Beat 1: mp
    recorder.record_event_at(64, 90, 1_000_000); // Beat 2: mf
    recorder.record_event_at(65, 127, 1_500_000); // Beat 3: fff

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Verify we captured all notes
    assert_eq!(recorded.notes, "c4 d4 e4 f4");

    // Verify velocities are normalized (0-127 -> 0.0-1.0)
    let velocities: Vec<f32> = recorded
        .velocities
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(velocities.len(), 4);
    assert!(velocities[0] < velocities[1], "Velocity should increase");
    assert!(velocities[1] < velocities[2], "Velocity should increase");
    assert!(velocities[2] < velocities[3], "Velocity should increase");

    // Check approximate values (30/127 ≈ 0.24, 127/127 = 1.0)
    assert!(
        velocities[0] < 0.3,
        "First note should be soft: {}",
        velocities[0]
    );
    assert!(
        velocities[3] > 0.95,
        "Last note should be loud: {}",
        velocities[3]
    );
}

/// Test velocity with chords - each note can have different velocity
/// TODO: Implement per-note velocity in chords
#[test]
#[ignore] // Not yet implemented: chord velocities need individual tracking
fn test_velocity_chord_different_dynamics() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // Chord with bass note loud, treble soft (common piano technique)
    recorder.record_event_at(48, 127, 0); // C3 - loud bass
    recorder.record_event_at(60, 80, 0); // C4 - medium
    recorder.record_event_at(64, 50, 0); // E4 - soft

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // All notes should be in a chord bracket
    assert!(recorded.notes.contains('['), "Should have chord notation");
    assert!(recorded.notes.contains(']'), "Should have chord notation");

    // Velocities should also be in chord bracket
    assert!(
        recorded.velocities.contains('['),
        "Velocities should have chord notation"
    );

    // Parse velocities from chord
    let vel_str = recorded
        .velocities
        .trim_start_matches('[')
        .trim_end_matches(']');
    let velocities: Vec<f32> = vel_str
        .split(',')
        .map(|v| v.trim().parse::<f32>().unwrap())
        .collect();

    assert_eq!(velocities.len(), 3, "Should have 3 velocities for chord");
    assert!(
        velocities[0] > 0.95,
        "Bass should be loud: {}",
        velocities[0]
    );
    assert!(
        velocities[2] < 0.5,
        "Treble should be soft: {}",
        velocities[2]
    );
}

/// Test velocity pattern with rests maintains alignment
#[test]
fn test_velocity_pattern_with_rests() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // Pattern: note, rest, note, rest
    recorder.record_event_at(60, 100, 0); // Beat 0: C4
    recorder.record_event_at(67, 80, 1_000_000); // Beat 2: G4 (beat 1 is rest)

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Pattern ends with a note, so no trailing rest
    assert_eq!(recorded.notes, "c4 ~ g4");

    // Velocity pattern should match note pattern structure
    let vel_parts: Vec<&str> = recorded.velocities.split_whitespace().collect();
    let note_parts: Vec<&str> = recorded.notes.split_whitespace().collect();

    assert_eq!(
        vel_parts.len(),
        note_parts.len(),
        "Velocity pattern should have same structure as note pattern"
    );

    // Rests should be rests in both patterns
    assert_eq!(note_parts[1], "~");
    assert_eq!(vel_parts[1], "~");
}

/// Test n-offset pattern with velocity
#[test]
fn test_n_offset_with_velocity() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // C major triad with different velocities
    recorder.record_event_at(60, 100, 0); // C4 - loud
    recorder.record_event_at(64, 70, 500_000); // E4 - medium
    recorder.record_event_at(67, 50, 1_000_000); // G4 - soft

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // N-offsets should be 0, 4, 7 (C major triad)
    assert_eq!(recorded.n_offsets, "0 4 7");
    assert_eq!(recorded.base_note, 60);
    assert_eq!(recorded.base_note_name, "c4");

    // Velocities should match the dynamics
    let velocities: Vec<f32> = recorded
        .velocities
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(velocities.len(), 3);
    assert!(
        velocities[0] > velocities[1],
        "First note louder than second"
    );
    assert!(
        velocities[1] > velocities[2],
        "Second note louder than third"
    );
}

/// Test multi-cycle recording with velocity
#[test]
fn test_multi_cycle_velocity_recording() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // Record pattern over 2 cycles (8 beats)
    // Cycle 1: C4, E4, G4, C5
    recorder.record_event_at(60, 80, 0);
    recorder.record_event_at(64, 90, 500_000);
    recorder.record_event_at(67, 100, 1_000_000);
    recorder.record_event_at(72, 110, 1_500_000);

    // Cycle 2: Same notes, different velocities (louder)
    recorder.record_event_at(60, 100, 2_000_000);
    recorder.record_event_at(64, 110, 2_500_000);
    recorder.record_event_at(67, 120, 3_000_000);
    recorder.record_event_at(72, 127, 3_500_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // Should span 2 cycles
    assert_eq!(recorded.cycle_count, 2);

    // Notes should repeat
    assert_eq!(recorded.notes, "c4 e4 g4 c5 c4 e4 g4 c5");

    // Second cycle should be louder than first
    let velocities: Vec<f32> = recorded
        .velocities
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(velocities.len(), 8);
    for i in 0..4 {
        assert!(
            velocities[i + 4] > velocities[i],
            "Second cycle note {} should be louder than first cycle",
            i
        );
    }
}

/// Test edge case: all notes same velocity
#[test]
fn test_uniform_velocity() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // All notes at max velocity (common for drum programming)
    recorder.record_event_at(60, 127, 0);
    recorder.record_event_at(62, 127, 500_000);
    recorder.record_event_at(64, 127, 1_000_000);
    recorder.record_event_at(65, 127, 1_500_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    // All velocities should be 1.0
    let velocities: Vec<f32> = recorded
        .velocities
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    for (i, vel) in velocities.iter().enumerate() {
        assert!(
            (*vel - 1.0).abs() < 0.01,
            "Velocity {} should be ~1.0, got {}",
            i,
            vel
        );
    }
}

/// Test edge case: very soft notes (velocity = 1)
#[test]
fn test_very_soft_velocity() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    // Very soft notes (minimum non-zero velocity)
    recorder.record_event_at(60, 1, 0);
    recorder.record_event_at(62, 1, 500_000);

    let recorded = recorder.to_recorded_pattern(4.0).unwrap();

    let velocities: Vec<f32> = recorded
        .velocities
        .split_whitespace()
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    for vel in velocities.iter() {
        assert!(*vel < 0.02, "Very soft velocity should be near 0: {}", vel);
        assert!(*vel > 0.0, "But should not be exactly 0");
    }
}

/// Test recording summary includes velocity info
#[test]
fn test_recording_summary() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(4);

    recorder.record_event_at(60, 100, 0);
    recorder.record_event_at(64, 80, 500_000);
    recorder.record_event_at(67, 60, 1_000_000);

    let summary = recorder.get_recording_summary(4.0);

    // Summary should mention note count and cycles
    assert!(
        summary.contains("3 notes"),
        "Summary should mention note count: {}",
        summary
    );
    assert!(
        summary.contains("1 cycle") || summary.contains("cycles"),
        "Summary should mention cycle count: {}",
        summary
    );
}

/// Test high-resolution timing (16th notes)
#[test]
fn test_16th_note_velocity_patterns() {
    let mut recorder = MidiRecorder::new(120.0);
    recorder.set_quantize(16); // Sixteenth notes

    // At 120 BPM: 1 beat = 500ms, 1/16th = 125ms = 125,000 us
    // Create a hi-hat pattern with accents (loud-soft-soft-soft)
    let velocities = vec![127, 60, 60, 60, 100, 60, 60, 60]; // Accents on 1 and 5

    for (i, vel) in velocities.iter().enumerate() {
        let timestamp = (i as u64) * 125_000;
        recorder.record_event_at(42, *vel, timestamp); // Closed hi-hat
    }

    let recorded = recorder.to_recorded_pattern(1.0).unwrap();

    // Parse velocities, skipping rests
    let vel_values: Vec<f32> = recorded
        .velocities
        .split_whitespace()
        .filter(|v| !v.starts_with('~')) // Skip rests
        .map(|v| v.parse::<f32>().unwrap())
        .collect();

    assert_eq!(vel_values.len(), 8, "Should have 8 velocity values");

    // Accents should be louder
    assert!(
        vel_values[0] > 0.9,
        "First accent should be loud: {}",
        vel_values[0]
    );
    assert!(
        vel_values[4] > 0.7,
        "Second accent should be loud: {}",
        vel_values[4]
    );

    // Non-accents should be softer
    assert!(
        vel_values[1] < 0.6,
        "Non-accent should be soft: {}",
        vel_values[1]
    );
    assert!(
        vel_values[2] < 0.6,
        "Non-accent should be soft: {}",
        vel_values[2]
    );
}
