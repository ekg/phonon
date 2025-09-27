//! MIDI output module for sending patterns to MIDI devices
//!
//! This module provides real-time MIDI output functionality,
//! allowing patterns to be sent to hardware or software synthesizers.

use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use crate::pattern_midi::MidiMessage;
use crate::pattern_tonal::note_to_midi;
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// MIDI output handler
pub struct MidiOutputHandler {
    connection: Option<MidiOutputConnection>,
    sender: Option<Sender<MidiCommand>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

/// Commands sent to MIDI thread
#[derive(Debug, Clone)]
enum MidiCommand {
    Message(MidiMessage),
    Stop,
}

/// MIDI device info
pub struct MidiDevice {
    pub name: String,
    pub port: MidiOutputPort,
}

impl MidiOutputHandler {
    /// Create a new MIDI output handler
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            connection: None,
            sender: None,
            thread_handle: None,
        })
    }

    /// List available MIDI output devices
    pub fn list_devices() -> Result<Vec<MidiDevice>, Box<dyn std::error::Error>> {
        let midi_out = MidiOutput::new("Phonon MIDI Scanner")?;
        let ports = midi_out.ports();

        let mut devices = Vec::new();
        for port in ports {
            let name = midi_out.port_name(&port)?;
            devices.push(MidiDevice { name, port });
        }

        Ok(devices)
    }

    /// Connect to a MIDI device by name
    pub fn connect(&mut self, device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let devices = Self::list_devices()?;

        let device = devices
            .into_iter()
            .find(|d| d.name.contains(device_name))
            .ok_or_else(|| format!("MIDI device '{device_name}' not found"))?;

        self.connect_to_port(device.port)
    }

    /// Connect to a specific MIDI port
    pub fn connect_to_port(
        &mut self,
        port: MidiOutputPort,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Stop any existing connection
        self.stop();

        let midi_out = MidiOutput::new("Phonon MIDI Output")?;
        let (sender, receiver) = channel();

        // Create connection in separate thread
        let connection = midi_out.connect(&port, "phonon-output")?;
        let connection = Arc::new(Mutex::new(connection));
        let connection_clone = connection.clone();

        // Start MIDI output thread
        let handle = thread::spawn(move || {
            midi_output_thread(receiver, connection_clone);
        });

        self.sender = Some(sender);
        self.thread_handle = Some(handle);

        Ok(())
    }

    /// Send a MIDI message
    pub fn send(&self, msg: MidiMessage) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sender) = &self.sender {
            sender.send(MidiCommand::Message(msg))?;
        } else {
            return Err("Not connected to MIDI device".into());
        }
        Ok(())
    }

    /// Stop MIDI output
    pub fn stop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(MidiCommand::Stop);
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Play a pattern to MIDI
    pub fn play_pattern<T: Clone + Send + Sync + 'static>(
        &self,
        pattern: &Pattern<T>,
        tempo_bpm: f32,
        duration_beats: f32,
        midi_converter: impl Fn(&T) -> Option<MidiMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let beat_duration = 60.0 / tempo_bpm;
        let total_duration = duration_beats * beat_duration;
        let start_time = Instant::now();

        // Track note-offs needed
        let mut pending_note_offs: Vec<(Instant, MidiMessage)> = Vec::new();

        // Sample resolution (events per beat)
        let resolution = 16;
        let step_duration = beat_duration / resolution as f32;

        let mut current_beat = 0.0;

        while current_beat < duration_beats {
            let elapsed = start_time.elapsed().as_secs_f32();
            let target_time = current_beat * beat_duration;

            // Sleep if we're ahead
            if elapsed < target_time {
                thread::sleep(Duration::from_secs_f32(target_time - elapsed));
            }

            // Send pending note-offs
            let now = Instant::now();
            pending_note_offs.retain(|(off_time, msg)| {
                if now >= *off_time {
                    let _ = self.send(msg.clone());
                    false
                } else {
                    true
                }
            });

            // Query pattern at current position
            let state = State {
                span: TimeSpan::new(
                    Fraction::from_float(current_beat as f64),
                    Fraction::from_float((current_beat + 1.0 / resolution as f32) as f64),
                ),
                controls: HashMap::new(),
            };

            let events = pattern.query(&state);

            for event in events {
                if let Some(midi_msg) = midi_converter(&event.value) {
                    // Send note-on
                    self.send(midi_msg.clone())?;

                    // Schedule note-off
                    if let MidiMessage::NoteOn {
                        channel,
                        note,
                        velocity,
                    } = midi_msg
                    {
                        let duration = (event.part.end - event.part.begin).to_float();
                        let off_time =
                            now + Duration::from_secs_f32(duration as f32 * beat_duration);
                        pending_note_offs.push((
                            off_time,
                            MidiMessage::NoteOff {
                                channel,
                                note,
                                velocity: 0,
                            },
                        ));
                    }
                }
            }

            current_beat += 1.0 / resolution as f32;
        }

        // Send remaining note-offs
        for (_, msg) in pending_note_offs {
            self.send(msg)?;
        }

        Ok(())
    }
}

impl Drop for MidiOutputHandler {
    fn drop(&mut self) {
        self.stop();
    }
}

/// MIDI output thread function
fn midi_output_thread(
    receiver: Receiver<MidiCommand>,
    connection: Arc<Mutex<MidiOutputConnection>>,
) {
    while let Ok(cmd) = receiver.recv() {
        match cmd {
            MidiCommand::Message(msg) => {
                let bytes = msg.to_bytes();
                if let Ok(mut conn) = connection.lock() {
                    let _ = conn.send(&bytes);
                }
            }
            MidiCommand::Stop => break,
        }
    }
}

/// Pattern scheduler for real-time MIDI playback
pub struct MidiScheduler {
    handler: MidiOutputHandler,
    tempo_bpm: f32,
    playing: Arc<Mutex<bool>>,
}

impl MidiScheduler {
    /// Create a new MIDI scheduler
    pub fn new(tempo_bpm: f32) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            handler: MidiOutputHandler::new()?,
            tempo_bpm,
            playing: Arc::new(Mutex::new(false)),
        })
    }

    /// Connect to MIDI device
    pub fn connect(&mut self, device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.handler.connect(device_name)
    }

    /// Play pattern continuously
    pub fn play_loop<T>(
        &mut self,
        pattern: Pattern<T>,
        midi_converter: impl Fn(&T) -> Option<MidiMessage> + Send + 'static,
    ) where
        T: Send + Sync + Clone + 'static,
    {
        let playing = self.playing.clone();
        *playing.lock().unwrap() = true;

        let tempo_bpm = self.tempo_bpm;
        let handler = self.handler.sender.clone();

        thread::spawn(move || {
            let beat_duration = 60.0 / tempo_bpm;
            let mut cycle = 0.0;

            while *playing.lock().unwrap() {
                let state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(cycle),
                        Fraction::from_float(cycle + 1.0),
                    ),
                    controls: HashMap::new(),
                };

                let events = pattern.query(&state);

                for event in events {
                    if let Some(midi_msg) = midi_converter(&event.value) {
                        if let Some(sender) = &handler {
                            let _ = sender.send(MidiCommand::Message(midi_msg));
                        }
                    }
                }

                thread::sleep(Duration::from_secs_f32(beat_duration));
                cycle += 1.0;
            }
        });
    }

    /// Stop playback
    pub fn stop(&mut self) {
        *self.playing.lock().unwrap() = false;
    }
}

/// Helper function to convert note strings to MIDI messages
pub fn note_to_midi_message(note_str: &str, channel: u8, velocity: u8) -> Option<MidiMessage> {
    // Handle special pattern names for drums
    let note = match note_str {
        "bd" | "kick" => 36,          // Bass drum
        "sn" | "sd" | "snare" => 38,  // Snare
        "cp" | "clap" => 39,          // Hand clap
        "hh" | "ch" | "hihat" => 42,  // Closed hi-hat
        "oh" | "open" => 46,          // Open hi-hat
        "lt" | "lowtom" => 43,        // Low tom
        "mt" | "midtom" => 47,        // Mid tom
        "ht" | "hightom" => 50,       // High tom
        "cy" | "crash" => 49,         // Crash cymbal
        "rd" | "ride" => 51,          // Ride cymbal
        "~" | "_" => return None,     // Rest/silence
        _ => note_to_midi(note_str)?, // Try to parse as note
    };
    Some(MidiMessage::NoteOn {
        channel,
        note,
        velocity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        // This will list actual MIDI devices if available
        match MidiOutputHandler::list_devices() {
            Ok(devices) => {
                println!("Available MIDI devices:");
                for device in devices {
                    println!("  - {}", device.name);
                }
            }
            Err(e) => {
                println!("Error listing MIDI devices: {}", e);
            }
        }
    }

    #[test]
    fn test_midi_message_conversion() {
        let msg = note_to_midi_message("C4", 0, 64);
        assert!(msg.is_some());

        if let Some(MidiMessage::NoteOn { note, .. }) = msg {
            assert_eq!(note, 60); // Middle C
        }
    }

    #[test]
    fn test_pattern_to_midi() {
        use crate::pattern::*;

        // Create a simple pattern
        let pattern = Pattern::cat(vec![
            Pattern::pure("C4".to_string()),
            Pattern::pure("E4".to_string()),
            Pattern::pure("G4".to_string()),
        ]);

        // Query the pattern
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        assert_eq!(events.len(), 3);

        // Convert to MIDI
        for event in events {
            let msg = note_to_midi_message(&event.value, 0, 64);
            assert!(msg.is_some());
        }
    }
}
