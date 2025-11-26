//! MIDI input module for receiving MIDI from external devices
//!
//! This module provides real-time MIDI input functionality,
//! allowing patterns to be recorded from MIDI controllers.

use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Parsed MIDI event with timing
#[derive(Debug, Clone)]
pub struct MidiEvent {
    /// MIDI message bytes
    pub message: Vec<u8>,
    /// Timestamp when received (microseconds from start)
    pub timestamp_us: u64,
    /// Channel (0-15)
    pub channel: u8,
    /// Message type
    pub message_type: MidiMessageType,
}

/// Type of MIDI message
#[derive(Debug, Clone, PartialEq)]
pub enum MidiMessageType {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8, velocity: u8 },
    ControlChange { controller: u8, value: u8 },
    ProgramChange { program: u8 },
    PitchBend { value: i16 },
    Other,
}

impl MidiEvent {
    /// Parse raw MIDI bytes into a MidiEvent
    pub fn from_bytes(bytes: &[u8], timestamp_us: u64) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        let status = bytes[0];
        let channel = status & 0x0F;
        let message_type = match status & 0xF0 {
            0x90 if bytes.len() >= 3 && bytes[2] > 0 => MidiMessageType::NoteOn {
                note: bytes[1],
                velocity: bytes[2],
            },
            0x90 if bytes.len() >= 3 => MidiMessageType::NoteOff {
                note: bytes[1],
                velocity: 0,
            },
            0x80 if bytes.len() >= 3 => MidiMessageType::NoteOff {
                note: bytes[1],
                velocity: bytes[2],
            },
            0xB0 if bytes.len() >= 3 => MidiMessageType::ControlChange {
                controller: bytes[1],
                value: bytes[2],
            },
            0xC0 if bytes.len() >= 2 => MidiMessageType::ProgramChange { program: bytes[1] },
            0xE0 if bytes.len() >= 3 => {
                let lsb = bytes[1] as i16;
                let msb = bytes[2] as i16;
                let value = ((msb << 7) | lsb) - 8192;
                MidiMessageType::PitchBend { value }
            }
            _ => MidiMessageType::Other,
        };

        Some(Self {
            message: bytes.to_vec(),
            timestamp_us,
            channel,
            message_type,
        })
    }

    /// Convert MIDI note number to note name
    pub fn midi_to_note_name(note: u8) -> String {
        let note_names = ["c", "cs", "d", "ds", "e", "f", "fs", "g", "gs", "a", "as", "b"];
        let octave = (note / 12) as i32 - 1;
        let note_index = (note % 12) as usize;
        format!("{}{}", note_names[note_index], octave)
    }
}

/// MIDI input device info
#[derive(Debug, Clone)]
pub struct MidiInputDevice {
    pub name: String,
    pub index: usize,
}

/// MIDI input handler for receiving messages
pub struct MidiInputHandler {
    connection: Option<MidiInputConnection<()>>,
    receiver: Option<Receiver<MidiEvent>>,
    start_time: Instant,
}

impl MidiInputHandler {
    /// Create a new MIDI input handler
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            connection: None,
            receiver: None,
            start_time: Instant::now(),
        })
    }

    /// List available MIDI input devices
    pub fn list_devices() -> Result<Vec<MidiInputDevice>, Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("Phonon MIDI Scanner")?;
        let ports = midi_in.ports();

        let devices: Vec<MidiInputDevice> = ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| {
                midi_in.port_name(port).ok().map(|name| MidiInputDevice {
                    name,
                    index: i,
                })
            })
            .collect();

        Ok(devices)
    }

    /// Connect to a MIDI input device by name
    pub fn connect(&mut self, device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("Phonon MIDI Input")?;
        let ports = midi_in.ports();

        let port = ports
            .iter()
            .find(|p| midi_in.port_name(p).map_or(false, |n| n.contains(device_name)))
            .ok_or_else(|| format!("MIDI device '{}' not found", device_name))?;

        self.connect_to_port_internal(midi_in, port.clone())
    }

    /// Connect to a MIDI input device by index
    pub fn connect_by_index(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("Phonon MIDI Input")?;
        let ports = midi_in.ports();

        let port = ports
            .get(index)
            .ok_or_else(|| format!("MIDI device index {} not found", index))?;

        self.connect_to_port_internal(midi_in, port.clone())
    }

    /// Internal connection helper
    fn connect_to_port_internal(
        &mut self,
        mut midi_in: MidiInput,
        port: MidiInputPort,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create channel for MIDI messages
        let (sender, receiver) = channel::<MidiEvent>();
        let start_time = Instant::now();

        // Ignore sysex and timing messages for cleaner input
        midi_in.ignore(Ignore::Sysex | Ignore::Time);

        // Connect and set up callback
        let connection = midi_in.connect(
            &port,
            "phonon-input",
            move |timestamp_us, message, _| {
                if let Some(event) = MidiEvent::from_bytes(message, timestamp_us) {
                    let _ = sender.send(event);
                }
            },
            (),
        )?;

        self.connection = Some(connection);
        self.receiver = Some(receiver);
        self.start_time = start_time;

        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Try to receive a MIDI event (non-blocking)
    pub fn try_recv(&self) -> Option<MidiEvent> {
        self.receiver.as_ref()?.try_recv().ok()
    }

    /// Receive all pending MIDI events (non-blocking)
    pub fn recv_all(&self) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        if let Some(ref receiver) = self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }
        events
    }

    /// Disconnect from current device
    pub fn disconnect(&mut self) {
        self.connection = None;
        self.receiver = None;
    }
}

impl Default for MidiInputHandler {
    fn default() -> Self {
        Self::new().expect("Failed to create MIDI input handler")
    }
}

/// MIDI pattern recorder - records MIDI events into Phonon patterns
pub struct MidiRecorder {
    events: Vec<MidiEvent>,
    start_time: Instant,
    tempo_bpm: f64,
    quantize_division: u8,
    /// Recording start timestamp (for relative timing)
    recording_start_us: u64,
}

/// Full recorded pattern with notes, velocities, and timing
#[derive(Debug, Clone)]
pub struct RecordedPattern {
    /// Note names pattern: "c4 e4 g4"
    pub notes: String,
    /// N-offsets pattern: "0 4 7" (semitones from lowest)
    pub n_offsets: String,
    /// Velocity/gain pattern: "0.8 1.0 0.6" (normalized 0-1)
    pub velocities: String,
    /// Base note (lowest note played)
    pub base_note: u8,
    /// Base note name
    pub base_note_name: String,
    /// Number of cycles the pattern spans
    pub cycle_count: usize,
    /// Quantization division used
    pub quantize_division: u8,
}

impl MidiRecorder {
    /// Create a new MIDI recorder
    pub fn new(tempo_bpm: f64) -> Self {
        Self {
            events: Vec::new(),
            start_time: Instant::now(),
            tempo_bpm,
            quantize_division: 16, // Default to 16th notes
            recording_start_us: 0,
        }
    }

    /// Set quantization (e.g., 4 for quarter notes, 8 for eighth notes, 16 for sixteenth)
    pub fn set_quantize(&mut self, division: u8) {
        self.quantize_division = division;
    }

    /// Start recording
    pub fn start(&mut self) {
        self.events.clear();
        self.start_time = Instant::now();
        self.recording_start_us = 0;
    }

    /// Record a MIDI event
    pub fn record_event(&mut self, event: MidiEvent) {
        // Capture the first event's timestamp as reference
        if self.events.is_empty() {
            self.recording_start_us = event.timestamp_us;
        }
        self.events.push(event);
    }

    /// Record a MIDI event with explicit timestamp (for testing)
    pub fn record_event_at(&mut self, note: u8, velocity: u8, timestamp_us: u64) {
        if self.events.is_empty() {
            self.recording_start_us = timestamp_us;
        }
        let event = MidiEvent {
            message: vec![0x90, note, velocity],
            timestamp_us,
            channel: 0,
            message_type: if velocity > 0 {
                MidiMessageType::NoteOn { note, velocity }
            } else {
                MidiMessageType::NoteOff { note, velocity: 0 }
            },
        };
        self.events.push(event);
    }

    /// Get elapsed time in beats
    fn elapsed_beats(&self) -> f64 {
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        elapsed_secs * (self.tempo_bpm / 60.0)
    }

    /// Convert timestamp to beat position (0-based)
    fn timestamp_to_beat(&self, timestamp_us: u64) -> f64 {
        let relative_us = timestamp_us.saturating_sub(self.recording_start_us);
        let us_per_beat = 60_000_000.0 / self.tempo_bpm;
        relative_us as f64 / us_per_beat
    }

    /// Quantize a beat position to the nearest grid division
    fn quantize_beat(&self, beat: f64) -> f64 {
        let grid = 1.0 / self.quantize_division as f64;
        (beat / grid).round() * grid
    }

    /// Convert recorded events to full pattern data
    pub fn to_recorded_pattern(&self, beats_per_cycle: f64) -> Option<RecordedPattern> {
        if self.events.is_empty() {
            return None;
        }

        // Collect note-on events with timing AND velocity
        let note_ons: Vec<_> = self
            .events
            .iter()
            .filter_map(|e| match &e.message_type {
                MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => {
                    Some((*note, *velocity, e.timestamp_us))
                }
                _ => None,
            })
            .collect();

        if note_ons.is_empty() {
            return None;
        }

        // Find lowest note for n-offsets
        let base_note = note_ons.iter().map(|(n, _, _)| *n).min().unwrap();
        let base_note_name = MidiEvent::midi_to_note_name(base_note);

        // Calculate grid
        let slots_per_cycle = self.quantize_division as usize;
        let slot_duration_beats = beats_per_cycle / slots_per_cycle as f64;
        let last_beat = self.timestamp_to_beat(note_ons.last().unwrap().2);
        let num_cycles = ((last_beat / beats_per_cycle).ceil() as usize).max(1);
        let total_slots = slots_per_cycle * num_cycles;

        // Grid stores (notes, velocities) per slot
        let mut grid: Vec<Option<Vec<(u8, u8)>>> = vec![None; total_slots];

        for (note, velocity, timestamp) in &note_ons {
            let beat = self.timestamp_to_beat(*timestamp);
            let quantized_beat = self.quantize_beat(beat);
            let slot = ((quantized_beat / slot_duration_beats).round() as usize)
                .min(total_slots.saturating_sub(1));

            if let Some(ref mut events) = grid[slot] {
                events.push((*note, *velocity));
            } else {
                grid[slot] = Some(vec![(*note, *velocity)]);
            }
        }

        // Build pattern strings
        let mut notes_parts = Vec::new();
        let mut n_offset_parts = Vec::new();
        let mut velocity_parts = Vec::new();
        let mut consecutive_rests = 0;

        for slot_events in grid.iter() {
            match slot_events {
                Some(events) => {
                    // Add accumulated rests
                    if consecutive_rests > 0 {
                        let rest = if consecutive_rests == 1 {
                            "~".to_string()
                        } else {
                            format!("~@{}", consecutive_rests)
                        };
                        notes_parts.push(rest.clone());
                        n_offset_parts.push(rest.clone());
                        velocity_parts.push(rest);
                        consecutive_rests = 0;
                    }

                    if events.len() == 1 {
                        let (note, vel) = events[0];
                        notes_parts.push(MidiEvent::midi_to_note_name(note));
                        n_offset_parts.push((note - base_note).to_string());
                        velocity_parts.push(format!("{:.2}", vel as f64 / 127.0));
                    } else {
                        // Chord - multiple notes
                        let note_chord: Vec<_> = events
                            .iter()
                            .map(|(n, _)| MidiEvent::midi_to_note_name(*n))
                            .collect();
                        let offset_chord: Vec<_> = events
                            .iter()
                            .map(|(n, _)| (n - base_note).to_string())
                            .collect();
                        // For velocity in chords, use average or max
                        let max_vel = events.iter().map(|(_, v)| *v).max().unwrap();

                        notes_parts.push(format!("[{}]", note_chord.join(" ")));
                        n_offset_parts.push(format!("[{}]", offset_chord.join(" ")));
                        velocity_parts.push(format!("{:.2}", max_vel as f64 / 127.0));
                    }
                }
                None => {
                    consecutive_rests += 1;
                }
            }
        }

        Some(RecordedPattern {
            notes: notes_parts.join(" "),
            n_offsets: n_offset_parts.join(" "),
            velocities: velocity_parts.join(" "),
            base_note,
            base_note_name,
            cycle_count: num_cycles,
            quantize_division: self.quantize_division,
        })
    }

    /// Convert recorded events to a Phonon pattern string with timing
    pub fn to_pattern_string(&self, beats_per_cycle: f64) -> String {
        self.to_recorded_pattern(beats_per_cycle)
            .map(|p| p.notes)
            .unwrap_or_default()
    }

    /// Convert recorded events to pattern string, optionally including timing
    pub fn to_pattern_string_with_options(&self, beats_per_cycle: f64, include_timing: bool) -> String {
        if self.events.is_empty() {
            return String::new();
        }

        if !include_timing {
            // Simple mode: just note names without timing grid
            return self
                .events
                .iter()
                .filter_map(|e| match &e.message_type {
                    MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => {
                        Some(MidiEvent::midi_to_note_name(*note))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
        }

        self.to_pattern_string(beats_per_cycle)
    }

    /// Get velocity pattern string (normalized 0-1)
    pub fn to_velocity_string(&self, beats_per_cycle: f64) -> String {
        self.to_recorded_pattern(beats_per_cycle)
            .map(|p| p.velocities)
            .unwrap_or_default()
    }

    /// Get the number of recorded events
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get note-on count
    pub fn note_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(&e.message_type, MidiMessageType::NoteOn { velocity, .. } if *velocity > 0))
            .count()
    }

    /// Clear recorded events
    pub fn clear(&mut self) {
        self.events.clear();
        self.recording_start_us = 0;
    }

    /// Get timing info for debugging
    pub fn get_timing_info(&self) -> Vec<(String, f64)> {
        self.events
            .iter()
            .filter_map(|e| match &e.message_type {
                MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => {
                    Some((
                        MidiEvent::midi_to_note_name(*note),
                        self.timestamp_to_beat(e.timestamp_us),
                    ))
                }
                _ => None,
            })
            .collect()
    }

    /// Convert recorded events to n-offset pattern string (semitone offsets from lowest note)
    ///
    /// Instead of note names like "c4 e4 g4 c5", produces offsets like "0 4 7 12"
    /// This is useful for creating `n` patterns that can be transposed easily.
    pub fn to_n_pattern_string(&self, beats_per_cycle: f64) -> String {
        self.to_recorded_pattern(beats_per_cycle)
            .map(|p| p.n_offsets)
            .unwrap_or_default()
    }

    /// Convert recorded events to n-offset pattern string, optionally including timing
    pub fn to_n_pattern_string_with_options(&self, beats_per_cycle: f64, include_timing: bool) -> String {
        if !include_timing {
            // Simple mode: just offsets without timing grid
            let note_ons: Vec<_> = self
                .events
                .iter()
                .filter_map(|e| match &e.message_type {
                    MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => Some(*note),
                    _ => None,
                })
                .collect();

            if note_ons.is_empty() {
                return String::new();
            }

            let lowest = note_ons.iter().min().unwrap();
            return note_ons
                .iter()
                .map(|n| (n - lowest).to_string())
                .collect::<Vec<_>>()
                .join(" ");
        }

        self.to_n_pattern_string(beats_per_cycle)
    }

    /// Get the lowest MIDI note from recorded events
    pub fn get_lowest_note(&self) -> Option<u8> {
        self.events
            .iter()
            .filter_map(|e| match &e.message_type {
                MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => Some(*note),
                _ => None,
            })
            .min()
    }

    /// Get the base note name for the n-offset pattern (useful for display)
    pub fn get_base_note_name(&self) -> Option<String> {
        self.get_lowest_note().map(MidiEvent::midi_to_note_name)
    }

    /// Get the number of cycles the recording spans
    pub fn get_cycle_count(&self, beats_per_cycle: f64) -> usize {
        let note_ons: Vec<_> = self
            .events
            .iter()
            .filter_map(|e| match &e.message_type {
                MidiMessageType::NoteOn { velocity, .. } if *velocity > 0 => {
                    Some(e.timestamp_us)
                }
                _ => None,
            })
            .collect();

        if note_ons.is_empty() {
            return 0;
        }

        let last_beat = self.timestamp_to_beat(*note_ons.last().unwrap());
        ((last_beat / beats_per_cycle).ceil() as usize).max(1)
    }

    /// Get recording duration in seconds
    pub fn get_duration_secs(&self) -> f64 {
        if self.events.is_empty() {
            return 0.0;
        }

        let first_ts = self.recording_start_us;
        let last_ts = self.events.last().map(|e| e.timestamp_us).unwrap_or(first_ts);
        (last_ts - first_ts) as f64 / 1_000_000.0
    }

    /// Get recording summary for display
    pub fn get_recording_summary(&self, beats_per_cycle: f64) -> String {
        let note_count = self.note_count();
        let cycle_count = self.get_cycle_count(beats_per_cycle);
        let duration = self.get_duration_secs();

        if note_count == 0 {
            return "No notes recorded".to_string();
        }

        format!(
            "{} notes over {} cycle{} ({:.1}s)",
            note_count,
            cycle_count,
            if cycle_count == 1 { "" } else { "s" },
            duration
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_note_to_name() {
        assert_eq!(MidiEvent::midi_to_note_name(60), "c4");
        assert_eq!(MidiEvent::midi_to_note_name(69), "a4");
        assert_eq!(MidiEvent::midi_to_note_name(72), "c5");
        assert_eq!(MidiEvent::midi_to_note_name(48), "c3");
    }

    #[test]
    fn test_parse_note_on() {
        let bytes = [0x90, 60, 100]; // Note on, channel 0, C4, velocity 100
        let event = MidiEvent::from_bytes(&bytes, 0).unwrap();
        assert_eq!(event.channel, 0);
        assert!(matches!(
            event.message_type,
            MidiMessageType::NoteOn { note: 60, velocity: 100 }
        ));
    }

    #[test]
    fn test_parse_note_off() {
        let bytes = [0x80, 60, 0]; // Note off, channel 0, C4
        let event = MidiEvent::from_bytes(&bytes, 0).unwrap();
        assert!(matches!(
            event.message_type,
            MidiMessageType::NoteOff { note: 60, .. }
        ));
    }

    #[test]
    fn test_parse_note_on_zero_velocity() {
        let bytes = [0x90, 60, 0]; // Note on with velocity 0 = note off
        let event = MidiEvent::from_bytes(&bytes, 0).unwrap();
        assert!(matches!(
            event.message_type,
            MidiMessageType::NoteOff { note: 60, .. }
        ));
    }

    // ========== Timing Tests ==========

    #[test]
    fn test_pattern_with_quarter_note_timing() {
        // At 120 BPM, 1 beat = 500,000 microseconds
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4); // Quarter notes

        // Play C major arpeggio on each beat
        recorder.record_event_at(60, 100, 0);          // Beat 0: C4
        recorder.record_event_at(64, 100, 500_000);    // Beat 1: E4
        recorder.record_event_at(67, 100, 1_000_000);  // Beat 2: G4
        recorder.record_event_at(72, 100, 1_500_000);  // Beat 3: C5

        // 4 beats per cycle, all notes on beat boundaries
        let pattern = recorder.to_pattern_string(4.0);
        assert_eq!(pattern, "c4 e4 g4 c5");
    }

    #[test]
    fn test_pattern_with_rests() {
        // At 120 BPM, 1 beat = 500,000 microseconds
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4); // Quarter notes

        // Play notes on beats 0 and 2 only (beats 1 and 3 are rests)
        recorder.record_event_at(60, 100, 0);          // Beat 0: C4
        recorder.record_event_at(67, 100, 1_000_000);  // Beat 2: G4

        let pattern = recorder.to_pattern_string(4.0);
        // Should have a rest between C4 and G4
        assert_eq!(pattern, "c4 ~ g4");
    }

    #[test]
    fn test_pattern_with_multiple_rests() {
        // At 120 BPM, 1 beat = 500,000 microseconds
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4); // Quarter notes

        // Play note only on beat 0 and beat 3
        recorder.record_event_at(60, 100, 0);          // Beat 0: C4
        recorder.record_event_at(72, 100, 1_500_000);  // Beat 3: C5

        let pattern = recorder.to_pattern_string(4.0);
        // Should have ~@2 for two consecutive rests
        assert_eq!(pattern, "c4 ~@2 c5");
    }

    #[test]
    fn test_pattern_sixteenth_notes() {
        // At 120 BPM, 1 beat = 500,000 us, 1/16th = 125,000 us
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(16); // Sixteenth notes

        // Play hi-hat pattern: x . x . x . x . (every other 16th)
        recorder.record_event_at(42, 100, 0);          // 16th 0
        recorder.record_event_at(42, 100, 250_000);    // 16th 2
        recorder.record_event_at(42, 100, 500_000);    // 16th 4
        recorder.record_event_at(42, 100, 750_000);    // 16th 6

        let pattern = recorder.to_pattern_string(1.0); // 1 beat cycle
        // Should have 4 notes with rests between
        assert!(pattern.contains("fs2")); // MIDI 42 = F#2
        assert_eq!(recorder.note_count(), 4);
    }

    #[test]
    fn test_pattern_chord_simultaneous_notes() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // Play C major chord (all notes at same time)
        recorder.record_event_at(60, 100, 0); // C4
        recorder.record_event_at(64, 100, 0); // E4
        recorder.record_event_at(67, 100, 0); // G4

        let pattern = recorder.to_pattern_string(4.0);
        // Should produce a chord notation [c4 e4 g4]
        assert!(pattern.contains("["));
        assert!(pattern.contains("c4"));
        assert!(pattern.contains("e4"));
        assert!(pattern.contains("g4"));
    }

    #[test]
    fn test_timing_info() {
        let mut recorder = MidiRecorder::new(120.0);

        recorder.record_event_at(60, 100, 0);
        recorder.record_event_at(64, 100, 500_000); // 1 beat later

        let info = recorder.get_timing_info();
        assert_eq!(info.len(), 2);
        assert_eq!(info[0].0, "c4");
        assert!((info[0].1 - 0.0).abs() < 0.01);
        assert_eq!(info[1].0, "e4");
        assert!((info[1].1 - 1.0).abs() < 0.01); // Should be at beat 1
    }

    #[test]
    fn test_quantize_snaps_to_grid() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4); // Quarter notes

        // Play slightly off beat (10ms late)
        recorder.record_event_at(60, 100, 10_000);     // Slightly after beat 0
        recorder.record_event_at(64, 100, 510_000);    // Slightly after beat 1

        let pattern = recorder.to_pattern_string(4.0);
        // Should still snap to c4 e4 without extra rests
        assert_eq!(pattern, "c4 e4");
    }

    #[test]
    fn test_pattern_without_timing() {
        let mut recorder = MidiRecorder::new(120.0);

        recorder.record_event_at(60, 100, 0);
        recorder.record_event_at(64, 100, 1_000_000);
        recorder.record_event_at(67, 100, 5_000_000);

        // Without timing - just note names
        let pattern = recorder.to_pattern_string_with_options(4.0, false);
        assert_eq!(pattern, "c4 e4 g4");
    }

    // ========== N-Offset Tests ==========

    #[test]
    fn test_n_offset_pattern_c_major_arpeggio() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // C major arpeggio: C4 E4 G4 C5
        // Offsets from C4: 0, 4, 7, 12
        recorder.record_event_at(60, 100, 0);          // C4 -> 0
        recorder.record_event_at(64, 100, 500_000);    // E4 -> 4
        recorder.record_event_at(67, 100, 1_000_000);  // G4 -> 7
        recorder.record_event_at(72, 100, 1_500_000);  // C5 -> 12

        let pattern = recorder.to_n_pattern_string(4.0);
        assert_eq!(pattern, "0 4 7 12");
    }

    #[test]
    fn test_n_offset_pattern_with_rests() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // Notes on beats 0 and 2 only
        recorder.record_event_at(60, 100, 0);          // C4 -> 0
        recorder.record_event_at(67, 100, 1_000_000);  // G4 -> 7

        let pattern = recorder.to_n_pattern_string(4.0);
        assert_eq!(pattern, "0 ~ 7");
    }

    #[test]
    fn test_n_offset_pattern_without_timing() {
        let mut recorder = MidiRecorder::new(120.0);

        // Play notes at various times (timing will be ignored)
        recorder.record_event_at(48, 100, 0);          // C3 -> 0
        recorder.record_event_at(55, 100, 123_456);    // G3 -> 7
        recorder.record_event_at(60, 100, 999_999);    // C4 -> 12

        // Without timing - just offsets
        let pattern = recorder.to_n_pattern_string_with_options(4.0, false);
        assert_eq!(pattern, "0 7 12");
    }

    #[test]
    fn test_n_offset_chord() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // C major chord (all notes at same time)
        recorder.record_event_at(60, 100, 0); // C4 -> 0
        recorder.record_event_at(64, 100, 0); // E4 -> 4
        recorder.record_event_at(67, 100, 0); // G4 -> 7

        let pattern = recorder.to_n_pattern_string(4.0);
        // Should produce a chord notation [0 4 7]
        assert!(pattern.contains("["));
        assert!(pattern.contains("0"));
        assert!(pattern.contains("4"));
        assert!(pattern.contains("7"));
    }

    #[test]
    fn test_get_lowest_note() {
        let mut recorder = MidiRecorder::new(120.0);

        recorder.record_event_at(72, 100, 0);    // C5
        recorder.record_event_at(48, 100, 100);  // C3 (lowest)
        recorder.record_event_at(60, 100, 200);  // C4

        assert_eq!(recorder.get_lowest_note(), Some(48));
        assert_eq!(recorder.get_base_note_name(), Some("c3".to_string()));
    }

    #[test]
    fn test_n_offset_octave_jumps() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // Octave jumping pattern
        recorder.record_event_at(36, 100, 0);          // C2 -> 0
        recorder.record_event_at(48, 100, 500_000);    // C3 -> 12
        recorder.record_event_at(60, 100, 1_000_000);  // C4 -> 24
        recorder.record_event_at(72, 100, 1_500_000);  // C5 -> 36

        let pattern = recorder.to_n_pattern_string(4.0);
        assert_eq!(pattern, "0 12 24 36");
    }
}
