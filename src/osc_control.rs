//! OSC (Open Sound Control) support for live coding
//!
//! This module provides OSC server and client functionality for
//! controlling Phonon in real-time during live performances.

use crate::mini_notation::parse_mini_notation;
use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use rosc::{OscMessage, OscPacket, OscType};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// OSC control commands
#[derive(Debug, Clone)]
pub enum OscCommand {
    /// Load a pattern
    LoadPattern {
        name: String,
        pattern: String,
    },

    /// Play/stop a pattern
    PlayPattern {
        name: String,
    },
    StopPattern {
        name: String,
    },

    /// Set tempo (BPM)
    SetTempo {
        bpm: f32,
    },

    /// Set control value
    SetControl {
        name: String,
        value: f32,
    },

    /// Mute/unmute a pattern
    Mute {
        name: String,
        muted: bool,
    },

    /// Solo a pattern
    Solo {
        name: String,
    },

    /// Clear solo
    ClearSolo,

    /// Set pattern volume
    SetVolume {
        name: String,
        volume: f32,
    },

    /// Apply an effect
    ApplyEffect {
        name: String,
        effect: String,
        params: Vec<f32>,
    },

    /// Sync/quantize to beat
    Sync,

    /// Stop all patterns
    StopAll,

    /// Query status
    GetStatus,
}

/// OSC server for receiving control messages
pub struct OscServer {
    socket: UdpSocket,
    sender: Sender<OscCommand>,
    running: Arc<Mutex<bool>>,
}

impl OscServer {
    /// Create a new OSC server
    pub fn new(port: u16) -> Result<(Self, Receiver<OscCommand>), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{port}"))?;
        socket.set_nonblocking(true)?;

        let (sender, receiver) = channel();

        Ok((
            Self {
                socket,
                sender,
                running: Arc::new(Mutex::new(false)),
            },
            receiver,
        ))
    }

    /// Start the OSC server
    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        *self.running.lock().unwrap() = true;

        let socket = self.socket.try_clone()?;
        let sender = self.sender.clone();
        let running = self.running.clone();

        thread::spawn(move || {
            let mut buf = [0u8; 1024];

            while *running.lock().unwrap() {
                match socket.recv_from(&mut buf) {
                    Ok((size, _addr)) => {
                        if let Ok(packet) = rosc::decoder::decode_udp(&buf[..size]) {
                            if let Some(cmd) = Self::parse_osc_packet(packet.1) {
                                let _ = sender.send(cmd);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        eprintln!("OSC server error: {e}");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the OSC server
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }

    /// Parse OSC packet into command
    fn parse_osc_packet(packet: OscPacket) -> Option<OscCommand> {
        match packet {
            OscPacket::Message(msg) => Self::parse_osc_message(msg),
            OscPacket::Bundle(bundle) => {
                // Process first message in bundle for simplicity
                bundle.content.into_iter().find_map(Self::parse_osc_packet)
            }
        }
    }

    /// Parse OSC message into command
    fn parse_osc_message(msg: OscMessage) -> Option<OscCommand> {
        match msg.addr.as_str() {
            "/pattern/load" => {
                if msg.args.len() >= 2 {
                    if let (OscType::String(name), OscType::String(pattern)) =
                        (&msg.args[0], &msg.args[1])
                    {
                        return Some(OscCommand::LoadPattern {
                            name: name.clone(),
                            pattern: pattern.clone(),
                        });
                    }
                }
            }
            "/pattern/play" => {
                if let Some(OscType::String(name)) = msg.args.first() {
                    return Some(OscCommand::PlayPattern { name: name.clone() });
                }
            }
            "/pattern/stop" => {
                if let Some(OscType::String(name)) = msg.args.first() {
                    return Some(OscCommand::StopPattern { name: name.clone() });
                }
            }
            "/tempo" => {
                if let Some(OscType::Float(bpm)) = msg.args.first() {
                    return Some(OscCommand::SetTempo { bpm: *bpm });
                }
            }
            "/control" => {
                if msg.args.len() >= 2 {
                    if let (OscType::String(name), OscType::Float(value)) =
                        (&msg.args[0], &msg.args[1])
                    {
                        return Some(OscCommand::SetControl {
                            name: name.clone(),
                            value: *value,
                        });
                    }
                }
            }
            "/mute" => {
                if msg.args.len() >= 2 {
                    if let (OscType::String(name), OscType::Bool(muted)) =
                        (&msg.args[0], &msg.args[1])
                    {
                        return Some(OscCommand::Mute {
                            name: name.clone(),
                            muted: *muted,
                        });
                    }
                }
            }
            "/solo" => {
                if let Some(OscType::String(name)) = msg.args.first() {
                    return Some(OscCommand::Solo { name: name.clone() });
                }
            }
            "/solo/clear" => {
                return Some(OscCommand::ClearSolo);
            }
            "/volume" => {
                if msg.args.len() >= 2 {
                    if let (OscType::String(name), OscType::Float(volume)) =
                        (&msg.args[0], &msg.args[1])
                    {
                        return Some(OscCommand::SetVolume {
                            name: name.clone(),
                            volume: *volume,
                        });
                    }
                }
            }
            "/sync" => {
                return Some(OscCommand::Sync);
            }
            "/stop/all" => {
                return Some(OscCommand::StopAll);
            }
            "/status" => {
                return Some(OscCommand::GetStatus);
            }
            _ => {}
        }
        None
    }
}

/// OSC client for sending messages
pub struct OscClient {
    socket: UdpSocket,
    target: SocketAddr,
}

impl OscClient {
    /// Create a new OSC client
    pub fn new(target: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let target = target.parse()?;

        Ok(Self { socket, target })
    }

    /// Send an OSC message
    pub fn send(
        &self,
        address: &str,
        args: Vec<OscType>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg = OscMessage {
            addr: address.to_string(),
            args,
        };

        let packet = OscPacket::Message(msg);
        let buf = rosc::encoder::encode(&packet)?;

        self.socket.send_to(&buf, self.target)?;
        Ok(())
    }

    /// Send pattern load command
    pub fn load_pattern(
        &self,
        name: &str,
        pattern: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send(
            "/pattern/load",
            vec![
                OscType::String(name.to_string()),
                OscType::String(pattern.to_string()),
            ],
        )
    }

    /// Send play pattern command
    pub fn play_pattern(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.send("/pattern/play", vec![OscType::String(name.to_string())])
    }

    /// Send stop pattern command
    pub fn stop_pattern(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.send("/pattern/stop", vec![OscType::String(name.to_string())])
    }

    /// Set tempo
    pub fn set_tempo(&self, bpm: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.send("/tempo", vec![OscType::Float(bpm)])
    }

    /// Set control value
    pub fn set_control(&self, name: &str, value: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.send(
            "/control",
            vec![OscType::String(name.to_string()), OscType::Float(value)],
        )
    }
}

/// Pattern state for OSC control
#[derive(Clone)]
pub struct PatternState {
    pub pattern: Pattern<String>,
    pub playing: bool,
    pub muted: bool,
    pub volume: f32,
}

/// OSC-controlled pattern engine
pub struct OscPatternEngine {
    patterns: Arc<Mutex<HashMap<String, PatternState>>>,
    tempo_bpm: Arc<Mutex<f32>>,
    controls: Arc<Mutex<HashMap<String, f64>>>,
    solo_pattern: Arc<Mutex<Option<String>>>,
    osc_server: Option<OscServer>,
    osc_receiver: Option<Receiver<OscCommand>>,
}

impl OscPatternEngine {
    /// Create a new OSC pattern engine
    pub fn new(osc_port: Option<u16>) -> Result<Self, Box<dyn std::error::Error>> {
        let (osc_server, osc_receiver) = if let Some(port) = osc_port {
            let (server, receiver) = OscServer::new(port)?;
            server.start()?;
            (Some(server), Some(receiver))
        } else {
            (None, None)
        };

        Ok(Self {
            patterns: Arc::new(Mutex::new(HashMap::new())),
            tempo_bpm: Arc::new(Mutex::new(120.0)),
            controls: Arc::new(Mutex::new(HashMap::new())),
            solo_pattern: Arc::new(Mutex::new(None)),
            osc_server,
            osc_receiver,
        })
    }

    /// Process OSC commands
    pub fn process_osc_commands(&mut self) {
        // Collect commands first to avoid borrow issues
        let commands: Vec<OscCommand> = if let Some(receiver) = &self.osc_receiver {
            let mut cmds = Vec::new();
            while let Ok(cmd) = receiver.try_recv() {
                cmds.push(cmd);
            }
            cmds
        } else {
            Vec::new()
        };

        // Then handle them
        for cmd in commands {
            self.handle_osc_command(cmd);
        }
    }

    /// Handle OSC command
    fn handle_osc_command(&mut self, cmd: OscCommand) {
        match cmd {
            OscCommand::LoadPattern { name, pattern } => {
                let parsed = parse_mini_notation(&pattern);
                let state = PatternState {
                    pattern: parsed,
                    playing: false,
                    muted: false,
                    volume: 1.0,
                };
                self.patterns.lock().unwrap().insert(name, state);
            }
            OscCommand::PlayPattern { name } => {
                if let Some(state) = self.patterns.lock().unwrap().get_mut(&name) {
                    state.playing = true;
                }
            }
            OscCommand::StopPattern { name } => {
                if let Some(state) = self.patterns.lock().unwrap().get_mut(&name) {
                    state.playing = false;
                }
            }
            OscCommand::SetTempo { bpm } => {
                *self.tempo_bpm.lock().unwrap() = bpm;
            }
            OscCommand::SetControl { name, value } => {
                self.controls.lock().unwrap().insert(name, value as f64);
            }
            OscCommand::Mute { name, muted } => {
                if let Some(state) = self.patterns.lock().unwrap().get_mut(&name) {
                    state.muted = muted;
                }
            }
            OscCommand::Solo { name } => {
                *self.solo_pattern.lock().unwrap() = Some(name);
            }
            OscCommand::ClearSolo => {
                *self.solo_pattern.lock().unwrap() = None;
            }
            OscCommand::SetVolume { name, volume } => {
                if let Some(state) = self.patterns.lock().unwrap().get_mut(&name) {
                    state.volume = volume;
                }
            }
            OscCommand::StopAll => {
                for state in self.patterns.lock().unwrap().values_mut() {
                    state.playing = false;
                }
            }
            _ => {}
        }
    }

    /// Get active patterns for current beat
    pub fn get_active_patterns(&self, beat: f64) -> Vec<(String, Vec<String>)> {
        let patterns = self.patterns.lock().unwrap();
        let solo = self.solo_pattern.lock().unwrap();

        let mut result = Vec::new();

        for (name, state) in patterns.iter() {
            // Check if pattern should play
            let should_play =
                state.playing && !state.muted && (solo.is_none() || solo.as_ref() == Some(name));

            if should_play {
                let query_state = State {
                    span: TimeSpan::new(
                        Fraction::from_float(beat),
                        Fraction::from_float(beat + 0.125), // 1/8 beat resolution
                    ),
                    controls: self.controls.lock().unwrap().clone(),
                };

                let events = state.pattern.query(&query_state);
                let values: Vec<String> = events.into_iter().map(|e| e.value).collect();

                if !values.is_empty() {
                    result.push((name.clone(), values));
                }
            }
        }

        result
    }

    /// Get current tempo
    pub fn get_tempo(&self) -> f32 {
        *self.tempo_bpm.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_server_creation() {
        let result = OscServer::new(9999);
        assert!(result.is_ok());
    }

    #[test]
    fn test_osc_message_parsing() {
        let msg = OscMessage {
            addr: "/pattern/load".to_string(),
            args: vec![
                OscType::String("drums".to_string()),
                OscType::String("bd*4 cp hh*8".to_string()),
            ],
        };

        let cmd = OscServer::parse_osc_message(msg);
        assert!(matches!(cmd, Some(OscCommand::LoadPattern { .. })));
    }

    #[test]
    fn test_pattern_engine() {
        let mut engine = OscPatternEngine::new(None).unwrap();

        // Load a pattern
        engine.handle_osc_command(OscCommand::LoadPattern {
            name: "test".to_string(),
            pattern: "c4 e4 g4".to_string(),
        });

        // Play it
        engine.handle_osc_command(OscCommand::PlayPattern {
            name: "test".to_string(),
        });

        // Get active patterns
        let active = engine.get_active_patterns(0.0);
        assert!(!active.is_empty());
    }
}
