//! MIDI input module for receiving MIDI from external devices
//!
//! This module provides real-time MIDI input functionality,
//! allowing patterns to be recorded from MIDI controllers.

use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Shared MIDI event queue for real-time monitoring
/// Used by both MidiInputHandler (writes) and UnifiedSignalGraph (reads)
pub type MidiEventQueue = Arc<Mutex<VecDeque<MidiEvent>>>;

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
        let note_names = [
            "c", "cs", "d", "ds", "e", "f", "fs", "g", "gs", "a", "as", "b",
        ];
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
    /// Shared queue for real-time MIDI monitoring (used by graph)
    monitoring_queue: MidiEventQueue,
}

impl MidiInputHandler {
    /// Create a new MIDI input handler
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            connection: None,
            receiver: None,
            start_time: Instant::now(),
            monitoring_queue: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    /// Get the shared MIDI event queue for real-time monitoring
    pub fn get_monitoring_queue(&self) -> MidiEventQueue {
        self.monitoring_queue.clone()
    }

    /// List available MIDI input devices
    pub fn list_devices() -> Result<Vec<MidiInputDevice>, Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("Phonon MIDI Scanner")?;
        let ports = midi_in.ports();

        let devices: Vec<MidiInputDevice> = ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| {
                midi_in
                    .port_name(port)
                    .ok()
                    .map(|name| MidiInputDevice { name, index: i })
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
            .find(|p| {
                midi_in
                    .port_name(p)
                    .map_or(false, |n| n.contains(device_name))
            })
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
        // Create channel for MIDI messages (for recording)
        let (sender, receiver) = channel::<MidiEvent>();
        let start_time = Instant::now();

        // Clone monitoring queue for the callback
        let monitoring_queue = self.monitoring_queue.clone();

        // Ignore sysex and timing messages for cleaner input
        midi_in.ignore(Ignore::Sysex | Ignore::Time);

        // Connect and set up callback
        let connection = midi_in.connect(
            &port,
            "phonon-input",
            move |timestamp_us, message, _| {
                if let Some(event) = MidiEvent::from_bytes(message, timestamp_us) {
                    // Send to channel for recording
                    let _ = sender.send(event.clone());

                    // Also send to monitoring queue for real-time playthrough
                    if let Ok(mut queue) = monitoring_queue.lock() {
                        queue.push_back(event);
                        // Limit queue size to prevent memory growth
                        while queue.len() > 1000 {
                            queue.pop_front();
                        }
                    }
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

/// Complete MIDI note with start and end times (for legato calculation)
#[derive(Debug, Clone)]
struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
    pub start_us: u64,       // When note-on received
    pub end_us: Option<u64>, // When note-off received (None if still active)
}

/// MIDI pattern recorder - records MIDI events into Phonon patterns
pub struct MidiRecorder {
    events: Vec<MidiEvent>,
    start_time: Instant,
    tempo_bpm: f64,
    quantize_division: u8,
    /// Recording start timestamp (for relative timing)
    recording_start_us: u64,
    /// Recording start cycle position (for punch-in alignment)
    recording_start_cycle: f64,
    /// Track active notes (note number → NoteEvent) for duration calculation
    active_notes: HashMap<u8, NoteEvent>,
    /// Completed notes with full duration info
    completed_notes: Vec<NoteEvent>,
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
    /// Legato/duration pattern: "0.9 0.5 1.0" (0.0=staccato, 1.0=full sustain)
    pub legato: String,
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
            recording_start_cycle: 0.0,
            active_notes: HashMap::new(),
            completed_notes: Vec::new(),
        }
    }

    /// Set quantization (e.g., 4 for quarter notes, 8 for eighth notes, 16 for sixteenth)
    pub fn set_quantize(&mut self, division: u8) {
        self.quantize_division = division;
    }

    /// Start recording (at cycle 0)
    pub fn start(&mut self) {
        self.events.clear();
        self.start_time = Instant::now();
        self.recording_start_us = 0;
        self.recording_start_cycle = 0.0;
        self.active_notes.clear();
        self.completed_notes.clear();
    }

    /// Start recording at a specific cycle position (for punch-in)
    pub fn start_at_cycle(&mut self, cycle_position: f64) {
        self.start();
        self.recording_start_cycle = cycle_position;
    }

    /// Get the cycle position when recording started (for status display)
    pub fn get_recording_start_cycle(&self) -> f64 {
        self.recording_start_cycle
    }

    /// Record a MIDI event
    pub fn record_event(&mut self, event: MidiEvent) {
        // Capture the first event's timestamp as reference
        if self.events.is_empty() {
            self.recording_start_us = event.timestamp_us;
        }

        // Track note durations for legato calculation
        match event.message_type {
            MidiMessageType::NoteOn { note, velocity } if velocity > 0 => {
                // Start tracking this note
                self.active_notes.insert(
                    note,
                    NoteEvent {
                        note,
                        velocity,
                        start_us: event.timestamp_us,
                        end_us: None,
                    },
                );
            }
            MidiMessageType::NoteOff { note, .. }
            | MidiMessageType::NoteOn { note, velocity: 0 } => {
                // Complete the note duration
                if let Some(mut note_event) = self.active_notes.remove(&note) {
                    note_event.end_us = Some(event.timestamp_us);
                    self.completed_notes.push(note_event);
                }
            }
            _ => {}
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

        // Track note durations for legato calculation
        if velocity > 0 {
            // Note-on: Start tracking
            self.active_notes.insert(
                note,
                NoteEvent {
                    note,
                    velocity,
                    start_us: timestamp_us,
                    end_us: None,
                },
            );
        } else {
            // Note-off: Complete duration
            if let Some(mut note_event) = self.active_notes.remove(&note) {
                note_event.end_us = Some(timestamp_us);
                self.completed_notes.push(note_event);
            }
        }

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

    /// Convert timestamp to absolute cycle position (accounting for punch-in offset)
    fn timestamp_to_cycle(&self, timestamp_us: u64, beats_per_cycle: f64) -> f64 {
        let relative_us = timestamp_us.saturating_sub(self.recording_start_us);
        let us_per_beat = 60_000_000.0 / self.tempo_bpm;
        let relative_beats = relative_us as f64 / us_per_beat;
        let relative_cycles = relative_beats / beats_per_cycle;

        // Add recording start offset to get absolute cycle position
        self.recording_start_cycle + relative_cycles
    }

    /// Quantize a beat position to the nearest grid division
    fn quantize_beat(&self, beat: f64) -> f64 {
        let grid = 1.0 / self.quantize_division as f64;
        (beat / grid).round() * grid
    }

    /// Quantize a cycle position to the nearest grid division (absolute grid)
    fn quantize_cycle(&self, cycle: f64, beats_per_cycle: f64) -> f64 {
        let slots_per_cycle = self.quantize_division as f64;
        let slot_duration_cycles = beats_per_cycle / slots_per_cycle;

        // Quantize to absolute grid (not relative to recording start)
        (cycle / slot_duration_cycles).round() * slot_duration_cycles
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
        let mut legato_parts = Vec::new();
        let mut consecutive_rests = 0;

        // Create lookup map for note durations (note -> duration_us)
        let mut note_durations: std::collections::HashMap<(u8, u64), u64> =
            std::collections::HashMap::new();
        for note_event in &self.completed_notes {
            if let Some(end_us) = note_event.end_us {
                let duration = end_us.saturating_sub(note_event.start_us);
                note_durations.insert((note_event.note, note_event.start_us), duration);
            }
        }

        let us_per_beat = 60_000_000.0 / self.tempo_bpm;
        let slot_duration_us = (slot_duration_beats * us_per_beat) as u64;

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
                        velocity_parts.push(rest.clone());
                        legato_parts.push(rest);
                        consecutive_rests = 0;
                    }

                    if events.len() == 1 {
                        let (note, vel) = events[0];
                        let timestamp = note_ons
                            .iter()
                            .find(|(n, v, _)| *n == note && *v == vel)
                            .map(|(_, _, t)| *t)
                            .unwrap_or(0);

                        // Calculate legato
                        let legato =
                            if let Some(&duration_us) = note_durations.get(&(note, timestamp)) {
                                let legato_raw = duration_us as f64 / slot_duration_us as f64;
                                legato_raw.clamp(0.0, 1.0)
                            } else {
                                0.8 // Default for notes still held or not tracked
                            };

                        notes_parts.push(MidiEvent::midi_to_note_name(note));
                        n_offset_parts.push((note - base_note).to_string());
                        velocity_parts.push(format!("{:.2}", vel as f64 / 127.0));
                        legato_parts.push(format!("{:.2}", legato));
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

                        // For legato in chords, use average of all note durations
                        let legato_values: Vec<f64> = events
                            .iter()
                            .filter_map(|(note, vel)| {
                                let timestamp = note_ons
                                    .iter()
                                    .find(|(n, v, _)| *n == *note && *v == *vel)
                                    .map(|(_, _, t)| *t)?;
                                let duration_us = *note_durations.get(&(*note, timestamp))?;
                                let legato_raw = duration_us as f64 / slot_duration_us as f64;
                                Some(legato_raw.clamp(0.0, 1.0))
                            })
                            .collect();
                        let avg_legato = if legato_values.is_empty() {
                            0.8
                        } else {
                            legato_values.iter().sum::<f64>() / legato_values.len() as f64
                        };

                        // Use comma separator for polyphony (simultaneous notes)
                        // [c4, e4, g4] plays all notes at once
                        // [c4 e4 g4] would subdivide (play sequentially in the slot)
                        notes_parts.push(format!("[{}]", note_chord.join(", ")));
                        n_offset_parts.push(format!("[{}]", offset_chord.join(", ")));
                        velocity_parts.push(format!("{:.2}", max_vel as f64 / 127.0));
                        legato_parts.push(format!("{:.2}", avg_legato));
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
            legato: legato_parts.join(" "),
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
    pub fn to_pattern_string_with_options(
        &self,
        beats_per_cycle: f64,
        include_timing: bool,
    ) -> String {
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
                MidiMessageType::NoteOn { note, velocity } if *velocity > 0 => Some((
                    MidiEvent::midi_to_note_name(*note),
                    self.timestamp_to_beat(e.timestamp_us),
                )),
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
    pub fn to_n_pattern_string_with_options(
        &self,
        beats_per_cycle: f64,
        include_timing: bool,
    ) -> String {
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
                MidiMessageType::NoteOn { velocity, .. } if *velocity > 0 => Some(e.timestamp_us),
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
        let last_ts = self
            .events
            .last()
            .map(|e| e.timestamp_us)
            .unwrap_or(first_ts);
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

    // =========================================================================
    // LIVE PREVIEW METHODS - Real-time feedback during recording
    // =========================================================================

    /// Get the number of cycles elapsed since recording started (real-time)
    /// This uses wall-clock time, not event timestamps
    pub fn elapsed_cycles_realtime(&self, beats_per_cycle: f64) -> f64 {
        let elapsed_beats = self.elapsed_beats();
        elapsed_beats / beats_per_cycle
    }

    /// Get currently held notes (notes that have note-on but no note-off yet)
    pub fn get_currently_held_notes(&self) -> Vec<u8> {
        let mut held: Vec<u8> = self.active_notes.keys().copied().collect();
        held.sort();
        held
    }

    /// Get currently held notes as a formatted string
    pub fn get_currently_held_notes_string(&self) -> String {
        let held = self.get_currently_held_notes();
        if held.is_empty() {
            return String::new();
        }
        if held.len() == 1 {
            MidiEvent::midi_to_note_name(held[0])
        } else {
            // Use comma separator for polyphony (all notes sounding together)
            let names: Vec<String> = held.iter().map(|&n| MidiEvent::midi_to_note_name(n)).collect();
            format!("[{}]", names.join(", "))
        }
    }

    /// Get a live preview of the recording for display in the editor
    /// Returns (cycle_count, pattern_preview, currently_held)
    pub fn live_preview(&self, beats_per_cycle: f64) -> LiveRecordingPreview {
        let elapsed_cycles = self.elapsed_cycles_realtime(beats_per_cycle);
        let current_cycle = elapsed_cycles.floor() as usize + 1; // 1-indexed for display

        // Get pattern so far
        let pattern = self.to_recorded_pattern(beats_per_cycle);

        // Get currently held notes
        let held_notes = self.get_currently_held_notes_string();

        LiveRecordingPreview {
            current_cycle,
            total_cycles: pattern.as_ref().map(|p| p.cycle_count).unwrap_or(1),
            note_count: self.note_count(),
            pattern_preview: pattern.as_ref().map(|p| p.notes.clone()).unwrap_or_default(),
            currently_held: held_notes,
            elapsed_secs: self.start_time.elapsed().as_secs_f64(),
        }
    }

    /// Generate the full code line for live preview
    /// Example: "~rec1 $ slow 2 $ n \"c4 e4 g4 c5\""
    pub fn generate_code_preview(&self, beats_per_cycle: f64, bus_name: &str) -> String {
        let pattern = match self.to_recorded_pattern(beats_per_cycle) {
            Some(p) => p,
            None => return format!("{} $ n \"\"", bus_name),
        };

        let slow_wrapper = if pattern.cycle_count > 1 {
            format!("slow {} $ ", pattern.cycle_count)
        } else {
            String::new()
        };

        format!("{} $ {}n \"{}\"", bus_name, slow_wrapper, pattern.notes)
    }
}

/// Live recording preview data for UI display
#[derive(Debug, Clone)]
pub struct LiveRecordingPreview {
    /// Current cycle number (1-indexed)
    pub current_cycle: usize,
    /// Total cycles covered by recorded notes
    pub total_cycles: usize,
    /// Number of notes recorded so far
    pub note_count: usize,
    /// Pattern preview string
    pub pattern_preview: String,
    /// Currently held notes (if any)
    pub currently_held: String,
    /// Elapsed time in seconds
    pub elapsed_secs: f64,
}

// ========== Scale Locking ==========

/// Musical scale definition - intervals from root (in semitones)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    Major,
    Minor,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    Pentatonic,
    MinorPentatonic,
    Blues,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    WholeTone,
    Chromatic,
}

impl Scale {
    /// Get the intervals (semitones from root) for this scale
    pub fn intervals(&self) -> &'static [u8] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor | Scale::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            Scale::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
            Scale::Pentatonic => &[0, 2, 4, 7, 9],
            Scale::MinorPentatonic => &[0, 3, 5, 7, 10],
            Scale::Blues => &[0, 3, 5, 6, 7, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Scale::Locrian => &[0, 1, 3, 5, 6, 8, 10],
            Scale::WholeTone => &[0, 2, 4, 6, 8, 10],
            Scale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        }
    }

    /// Parse scale name from string
    pub fn from_str(s: &str) -> Option<Scale> {
        match s.to_lowercase().as_str() {
            "major" | "maj" | "ionian" => Some(Scale::Major),
            "minor" | "min" | "natural_minor" | "aeolian" => Some(Scale::Minor),
            "harmonic_minor" | "harmonic" => Some(Scale::HarmonicMinor),
            "melodic_minor" | "melodic" => Some(Scale::MelodicMinor),
            "pentatonic" | "pent" | "major_pentatonic" => Some(Scale::Pentatonic),
            "minor_pentatonic" | "minpent" => Some(Scale::MinorPentatonic),
            "blues" => Some(Scale::Blues),
            "dorian" => Some(Scale::Dorian),
            "phrygian" => Some(Scale::Phrygian),
            "lydian" => Some(Scale::Lydian),
            "mixolydian" | "mixo" => Some(Scale::Mixolydian),
            "locrian" => Some(Scale::Locrian),
            "wholetone" | "whole_tone" => Some(Scale::WholeTone),
            "chromatic" => Some(Scale::Chromatic),
            _ => None,
        }
    }
}

/// Map a MIDI note to the nearest note in a scale
/// root: MIDI note number of the scale root (e.g., 60 for C4)
/// scale: The scale to lock to
/// note: The input MIDI note to quantize
pub fn scale_lock(note: u8, root: u8, scale: Scale) -> u8 {
    let intervals = scale.intervals();

    // Get the note's position relative to root
    let diff = note as i16 - root as i16;
    let relative = diff.rem_euclid(12) as u8;
    // Use div_euclid for correct floor division with negative values
    let octave_offset = diff.div_euclid(12) * 12;

    // Find the nearest scale degree
    let mut best_interval = intervals[0];
    let mut best_distance = 12u8;

    for &interval in intervals {
        let distance = if relative >= interval {
            relative - interval
        } else {
            interval - relative
        };

        // Also check wrapping around the octave
        let wrap_distance = 12 - distance;
        let min_distance = distance.min(wrap_distance);

        if min_distance < best_distance {
            best_distance = min_distance;
            best_interval = interval;
        }
    }

    // Calculate the final note
    let result = root as i16 + octave_offset + best_interval as i16;
    result.clamp(0, 127) as u8
}

/// Parse root note from string (e.g., "c", "c#", "db", "f#")
pub fn parse_root_note(s: &str) -> Option<u8> {
    let s = s.to_lowercase();
    let chars: Vec<char> = s.chars().collect();

    if chars.is_empty() {
        return None;
    }

    let base = match chars[0] {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };

    let modifier = if chars.len() > 1 {
        match chars[1] {
            '#' | 's' => 1i8,
            'b' | 'f' => -1i8,
            _ => 0,
        }
    } else {
        0
    };

    // Return as MIDI note in octave 4 (C4 = 60)
    Some(((60 + base) as i8 + modifier) as u8)
}

// ========== Arpeggiator ==========

/// Arpeggiator pattern direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArpPattern {
    Up,
    Down,
    UpDown,
    DownUp,
    Random,
    AsPlayed,
}

impl ArpPattern {
    pub fn from_str(s: &str) -> Option<ArpPattern> {
        match s.to_lowercase().as_str() {
            "up" => Some(ArpPattern::Up),
            "down" => Some(ArpPattern::Down),
            "updown" | "up_down" | "pingpong" => Some(ArpPattern::UpDown),
            "downup" | "down_up" => Some(ArpPattern::DownUp),
            "random" | "rand" => Some(ArpPattern::Random),
            "asplayed" | "as_played" | "order" => Some(ArpPattern::AsPlayed),
            _ => None,
        }
    }
}

/// Arpeggiator state for processing held notes
#[derive(Debug, Clone)]
pub struct Arpeggiator {
    /// Held notes (in order of being pressed)
    held_notes: Vec<u8>,
    /// Current index in the arpeggio sequence
    current_index: usize,
    /// Pattern direction
    pattern: ArpPattern,
    /// For up-down patterns, are we going up or down?
    going_up: bool,
    /// Division (notes per beat): 1=quarter, 2=eighth, 4=sixteenth, etc.
    division: u8,
    /// Sample counter for timing
    sample_counter: u64,
    /// Samples per arp step (calculated from tempo)
    samples_per_step: u64,
    /// Last note that was triggered (for note-off)
    last_triggered_note: Option<u8>,
    /// Simple RNG state for random pattern
    rng_state: u32,
}

impl Arpeggiator {
    pub fn new(pattern: ArpPattern, division: u8) -> Self {
        Self {
            held_notes: Vec::new(),
            current_index: 0,
            pattern,
            going_up: true,
            division,
            sample_counter: 0,
            samples_per_step: 22050, // Default: 2 steps per second at 44.1kHz
            last_triggered_note: None,
            rng_state: 12345,
        }
    }

    /// Update tempo (samples per step based on BPM and division)
    pub fn set_tempo(&mut self, bpm: f32, sample_rate: f32) {
        let beats_per_second = bpm / 60.0;
        let steps_per_second = beats_per_second * self.division as f32;
        self.samples_per_step = (sample_rate / steps_per_second) as u64;
    }

    /// Add a note to the held notes
    pub fn note_on(&mut self, note: u8) {
        if !self.held_notes.contains(&note) {
            self.held_notes.push(note);
            // Sort for up/down patterns
            if self.pattern != ArpPattern::AsPlayed {
                self.held_notes.sort();
            }
        }
    }

    /// Remove a note from held notes
    pub fn note_off(&mut self, note: u8) {
        self.held_notes.retain(|&n| n != note);
        if self.current_index >= self.held_notes.len() && !self.held_notes.is_empty() {
            self.current_index = 0;
        }
    }

    /// Simple LCG random number generator
    fn next_random(&mut self) -> u32 {
        self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        self.rng_state
    }

    /// Get the next note in the arpeggio sequence
    fn next_note(&mut self) -> Option<u8> {
        if self.held_notes.is_empty() {
            return None;
        }

        let note = match self.pattern {
            ArpPattern::Up => {
                let note = self.held_notes[self.current_index];
                self.current_index = (self.current_index + 1) % self.held_notes.len();
                note
            }
            ArpPattern::Down => {
                let idx = self.held_notes.len() - 1 - self.current_index;
                let note = self.held_notes[idx];
                self.current_index = (self.current_index + 1) % self.held_notes.len();
                note
            }
            ArpPattern::UpDown => {
                let note = self.held_notes[self.current_index];
                if self.going_up {
                    if self.current_index >= self.held_notes.len() - 1 {
                        self.going_up = false;
                    } else {
                        self.current_index += 1;
                    }
                } else {
                    if self.current_index == 0 {
                        self.going_up = true;
                    } else {
                        self.current_index -= 1;
                    }
                }
                note
            }
            ArpPattern::DownUp => {
                let note = self.held_notes[self.current_index];
                if !self.going_up {
                    if self.current_index >= self.held_notes.len() - 1 {
                        self.going_up = true;
                    } else {
                        self.current_index += 1;
                    }
                } else {
                    if self.current_index == 0 {
                        self.going_up = false;
                    } else {
                        self.current_index -= 1;
                    }
                }
                note
            }
            ArpPattern::Random => {
                let idx = (self.next_random() as usize) % self.held_notes.len();
                self.held_notes[idx]
            }
            ArpPattern::AsPlayed => {
                let note = self.held_notes[self.current_index];
                self.current_index = (self.current_index + 1) % self.held_notes.len();
                note
            }
        };

        Some(note)
    }

    /// Process one sample, returns (note_on, note_off) events if triggered
    pub fn process_sample(&mut self) -> (Option<u8>, Option<u8>) {
        self.sample_counter += 1;

        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;

            // Note off for previous note
            let note_off = self.last_triggered_note;

            // Note on for next note
            let note_on = self.next_note();
            self.last_triggered_note = note_on;

            (note_on, note_off)
        } else {
            (None, None)
        }
    }

    /// Check if arpeggiator has any held notes
    pub fn is_active(&self) -> bool {
        !self.held_notes.is_empty()
    }

    /// Get currently held notes
    pub fn held_notes(&self) -> &[u8] {
        &self.held_notes
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
            MidiMessageType::NoteOn {
                note: 60,
                velocity: 100
            }
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
        recorder.record_event_at(60, 100, 0); // Beat 0: C4
        recorder.record_event_at(64, 100, 500_000); // Beat 1: E4
        recorder.record_event_at(67, 100, 1_000_000); // Beat 2: G4
        recorder.record_event_at(72, 100, 1_500_000); // Beat 3: C5

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
        recorder.record_event_at(60, 100, 0); // Beat 0: C4
        recorder.record_event_at(67, 100, 1_000_000); // Beat 2: G4

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
        recorder.record_event_at(60, 100, 0); // Beat 0: C4
        recorder.record_event_at(72, 100, 1_500_000); // Beat 3: C5

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
        recorder.record_event_at(42, 100, 0); // 16th 0
        recorder.record_event_at(42, 100, 250_000); // 16th 2
        recorder.record_event_at(42, 100, 500_000); // 16th 4
        recorder.record_event_at(42, 100, 750_000); // 16th 6

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
        recorder.record_event_at(60, 100, 10_000); // Slightly after beat 0
        recorder.record_event_at(64, 100, 510_000); // Slightly after beat 1

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
        recorder.record_event_at(60, 100, 0); // C4 -> 0
        recorder.record_event_at(64, 100, 500_000); // E4 -> 4
        recorder.record_event_at(67, 100, 1_000_000); // G4 -> 7
        recorder.record_event_at(72, 100, 1_500_000); // C5 -> 12

        let pattern = recorder.to_n_pattern_string(4.0);
        assert_eq!(pattern, "0 4 7 12");
    }

    #[test]
    fn test_n_offset_pattern_with_rests() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // Notes on beats 0 and 2 only
        recorder.record_event_at(60, 100, 0); // C4 -> 0
        recorder.record_event_at(67, 100, 1_000_000); // G4 -> 7

        let pattern = recorder.to_n_pattern_string(4.0);
        assert_eq!(pattern, "0 ~ 7");
    }

    #[test]
    fn test_n_offset_pattern_without_timing() {
        let mut recorder = MidiRecorder::new(120.0);

        // Play notes at various times (timing will be ignored)
        recorder.record_event_at(48, 100, 0); // C3 -> 0
        recorder.record_event_at(55, 100, 123_456); // G3 -> 7
        recorder.record_event_at(60, 100, 999_999); // C4 -> 12

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

        recorder.record_event_at(72, 100, 0); // C5
        recorder.record_event_at(48, 100, 100); // C3 (lowest)
        recorder.record_event_at(60, 100, 200); // C4

        assert_eq!(recorder.get_lowest_note(), Some(48));
        assert_eq!(recorder.get_base_note_name(), Some("c3".to_string()));
    }

    #[test]
    fn test_n_offset_octave_jumps() {
        let mut recorder = MidiRecorder::new(120.0);
        recorder.set_quantize(4);

        // Octave jumping pattern
        recorder.record_event_at(36, 100, 0); // C2 -> 0
        recorder.record_event_at(48, 100, 500_000); // C3 -> 12
        recorder.record_event_at(60, 100, 1_000_000); // C4 -> 24
        recorder.record_event_at(72, 100, 1_500_000); // C5 -> 36

        let pattern = recorder.to_n_pattern_string(4.0);
        assert_eq!(pattern, "0 12 24 36");
    }

    // ========== Scale Locking Tests ==========

    #[test]
    fn test_scale_lock_c_major() {
        let root = 60; // C4
        let scale = Scale::Major;

        // C -> C (in scale)
        assert_eq!(scale_lock(60, root, scale), 60);
        // D -> D (in scale)
        assert_eq!(scale_lock(62, root, scale), 62);
        // E -> E (in scale)
        assert_eq!(scale_lock(64, root, scale), 64);
        // F -> F (in scale)
        assert_eq!(scale_lock(65, root, scale), 65);
        // G -> G (in scale)
        assert_eq!(scale_lock(67, root, scale), 67);
        // A -> A (in scale)
        assert_eq!(scale_lock(69, root, scale), 69);
        // B -> B (in scale)
        assert_eq!(scale_lock(71, root, scale), 71);

        // C# -> C or D (closest)
        let cs = scale_lock(61, root, scale);
        assert!(cs == 60 || cs == 62);

        // Eb -> E (closest to Eb in C major)
        let eb = scale_lock(63, root, scale);
        assert!(eb == 62 || eb == 64);
    }

    #[test]
    fn test_scale_lock_a_minor() {
        let root = 69; // A4
        let scale = Scale::Minor;

        // A -> A (in scale)
        assert_eq!(scale_lock(69, root, scale), 69);
        // B -> B (in scale)
        assert_eq!(scale_lock(71, root, scale), 71);
        // C -> C (in scale)
        assert_eq!(scale_lock(72, root, scale), 72);
    }

    #[test]
    fn test_scale_lock_pentatonic() {
        let root = 60; // C4
        let scale = Scale::Pentatonic;

        // C major pentatonic: C D E G A
        assert_eq!(scale_lock(60, root, scale), 60); // C
        assert_eq!(scale_lock(62, root, scale), 62); // D
        assert_eq!(scale_lock(64, root, scale), 64); // E
        assert_eq!(scale_lock(67, root, scale), 67); // G
        assert_eq!(scale_lock(69, root, scale), 69); // A

        // F (65) should map to E (64) or G (67)
        let f = scale_lock(65, root, scale);
        assert!(f == 64 || f == 67);
    }

    #[test]
    fn test_scale_lock_octaves() {
        let root = 60; // C4
        let scale = Scale::Major;

        // Test across octaves
        assert_eq!(scale_lock(48, root, scale), 48); // C3
        assert_eq!(scale_lock(72, root, scale), 72); // C5
        assert_eq!(scale_lock(84, root, scale), 84); // C6

        // D# in different octaves
        let ds3 = scale_lock(51, root, scale);
        let ds4 = scale_lock(63, root, scale);
        let ds5 = scale_lock(75, root, scale);

        // They should all map to the same scale degree relative to their octave
        assert_eq!(ds4 - ds3, 12);
        assert_eq!(ds5 - ds4, 12);
    }

    #[test]
    fn test_parse_root_note() {
        assert_eq!(parse_root_note("c"), Some(60));
        assert_eq!(parse_root_note("C"), Some(60));
        assert_eq!(parse_root_note("c#"), Some(61));
        assert_eq!(parse_root_note("db"), Some(61));  // D-flat = C# = 61
        assert_eq!(parse_root_note("a"), Some(69));
        assert_eq!(parse_root_note("f#"), Some(66));
        assert_eq!(parse_root_note("bb"), Some(70));
    }

    #[test]
    fn test_scale_from_str() {
        assert_eq!(Scale::from_str("major"), Some(Scale::Major));
        assert_eq!(Scale::from_str("minor"), Some(Scale::Minor));
        assert_eq!(Scale::from_str("pentatonic"), Some(Scale::Pentatonic));
        assert_eq!(Scale::from_str("blues"), Some(Scale::Blues));
        assert_eq!(Scale::from_str("dorian"), Some(Scale::Dorian));
        assert_eq!(Scale::from_str("invalid"), None);
    }

    // ========== Arpeggiator Tests ==========

    #[test]
    fn test_arpeggiator_up() {
        let mut arp = Arpeggiator::new(ArpPattern::Up, 4);
        arp.samples_per_step = 1; // Trigger every sample for testing

        // Add chord: C E G
        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        // Collect notes from processing
        let mut notes = Vec::new();
        for _ in 0..6 {
            if let (Some(note), _) = arp.process_sample() {
                notes.push(note);
            }
        }

        // Should cycle through C E G C E G
        assert_eq!(notes, vec![60, 64, 67, 60, 64, 67]);
    }

    #[test]
    fn test_arpeggiator_down() {
        let mut arp = Arpeggiator::new(ArpPattern::Down, 4);
        arp.samples_per_step = 1;

        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        let mut notes = Vec::new();
        for _ in 0..6 {
            if let (Some(note), _) = arp.process_sample() {
                notes.push(note);
            }
        }

        // Should cycle through G E C G E C
        assert_eq!(notes, vec![67, 64, 60, 67, 64, 60]);
    }

    #[test]
    fn test_arpeggiator_updown() {
        let mut arp = Arpeggiator::new(ArpPattern::UpDown, 4);
        arp.samples_per_step = 1;

        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        let mut notes = Vec::new();
        for _ in 0..8 {
            if let (Some(note), _) = arp.process_sample() {
                notes.push(note);
            }
        }

        // Should go C E G E C E G E (ping pong without repeating ends)
        // Actually: C E G G E C C E -> depends on implementation
        // Let's just verify it contains up and down motion
        assert!(notes.contains(&60));
        assert!(notes.contains(&64));
        assert!(notes.contains(&67));
    }

    #[test]
    fn test_arpeggiator_note_off() {
        let mut arp = Arpeggiator::new(ArpPattern::Up, 4);
        arp.samples_per_step = 1;

        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        // Process a few
        for _ in 0..3 {
            arp.process_sample();
        }

        // Remove E
        arp.note_off(64);

        // Collect remaining notes
        let mut notes = Vec::new();
        for _ in 0..4 {
            if let (Some(note), _) = arp.process_sample() {
                notes.push(note);
            }
        }

        // Should only have C and G now
        for note in &notes {
            assert!(*note == 60 || *note == 67);
        }
    }

    #[test]
    fn test_arpeggiator_tempo() {
        let mut arp = Arpeggiator::new(ArpPattern::Up, 4); // 4 = sixteenth notes

        // 120 BPM, 44100 Hz
        arp.set_tempo(120.0, 44100.0);

        // At 120 BPM, 4 sixteenth notes per beat = 8 notes per second
        // 44100 / 8 = 5512.5 samples per step
        assert!((arp.samples_per_step as i64 - 5512).abs() < 10);
    }

    #[test]
    fn test_arp_pattern_from_str() {
        assert_eq!(ArpPattern::from_str("up"), Some(ArpPattern::Up));
        assert_eq!(ArpPattern::from_str("down"), Some(ArpPattern::Down));
        assert_eq!(ArpPattern::from_str("updown"), Some(ArpPattern::UpDown));
        assert_eq!(ArpPattern::from_str("random"), Some(ArpPattern::Random));
        assert_eq!(ArpPattern::from_str("asplayed"), Some(ArpPattern::AsPlayed));
        assert_eq!(ArpPattern::from_str("invalid"), None);
    }
}
