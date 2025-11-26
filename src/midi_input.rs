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
    quantize_division: Option<u8>,
}

impl MidiRecorder {
    /// Create a new MIDI recorder
    pub fn new(tempo_bpm: f64) -> Self {
        Self {
            events: Vec::new(),
            start_time: Instant::now(),
            tempo_bpm,
            quantize_division: None,
        }
    }

    /// Set quantization (e.g., 4 for quarter notes, 8 for eighth notes, 16 for sixteenth)
    pub fn set_quantize(&mut self, division: u8) {
        self.quantize_division = Some(division);
    }

    /// Start recording
    pub fn start(&mut self) {
        self.events.clear();
        self.start_time = Instant::now();
    }

    /// Record a MIDI event
    pub fn record_event(&mut self, event: MidiEvent) {
        self.events.push(event);
    }

    /// Get elapsed time in beats
    fn elapsed_beats(&self) -> f64 {
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        elapsed_secs * (self.tempo_bpm / 60.0)
    }

    /// Convert recorded events to a Phonon pattern string
    pub fn to_pattern_string(&self, beats_per_cycle: f64) -> String {
        if self.events.is_empty() {
            return String::new();
        }

        // Find the total duration and collect note-on events
        let note_ons: Vec<_> = self
            .events
            .iter()
            .filter_map(|e| match &e.message_type {
                MidiMessageType::NoteOn { note, velocity: _ } => Some((*note, e.timestamp_us)),
                _ => None,
            })
            .collect();

        if note_ons.is_empty() {
            return String::new();
        }

        // Calculate beat positions
        let us_per_beat = 60_000_000.0 / self.tempo_bpm;
        let us_per_cycle = us_per_beat * beats_per_cycle;

        // Convert to note names at beat positions
        let mut pattern_elements: Vec<String> = Vec::new();
        let total_cycles = (note_ons.last().unwrap().1 as f64 / us_per_cycle).ceil() as usize;
        let total_cycles = total_cycles.max(1);

        // Group notes by time position within cycles
        for (note, timestamp) in &note_ons {
            let note_name = MidiEvent::midi_to_note_name(*note);
            pattern_elements.push(note_name);
        }

        // Join elements into pattern
        pattern_elements.join(" ")
    }

    /// Get the number of recorded events
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Clear recorded events
    pub fn clear(&mut self) {
        self.events.clear();
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
}
