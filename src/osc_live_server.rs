#![allow(unused_assignments, unused_mut)]
//! OSC Live Server for Phonon
//!
//! Listens on port 7770 for OSC messages to control live coding session
//! Handles: /eval, /hush, /panic

use crate::unified_graph::UnifiedSignalGraph;
use crate::unified_graph_parser::DslCompiler;
use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Commands from OSC server to audio engine
#[derive(Debug, Clone)]
pub enum LiveCommand {
    /// Evaluate new Phonon code
    Eval { code: String },
    /// Stop all audio (graceful fade)
    Hush,
    /// Emergency stop (immediate silence)
    Panic,
}

/// OSC Live Server
pub struct OscLiveServer {
    port: u16,
    running: Arc<Mutex<bool>>,
    command_sender: Option<std::sync::mpsc::Sender<LiveCommand>>,
}

impl OscLiveServer {
    /// Create a new OSC live server
    pub fn new(
        port: u16,
    ) -> Result<(Self, std::sync::mpsc::Receiver<LiveCommand>), Box<dyn std::error::Error>> {
        let (tx, rx) = std::sync::mpsc::channel();

        Ok((
            Self {
                port,
                running: Arc::new(Mutex::new(false)),
                command_sender: Some(tx),
            },
            rx,
        ))
    }

    /// Start the OSC server in a background thread
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let port = self.port;
        let running = self.running.clone();
        let sender = self.command_sender.clone().unwrap();

        *running.lock().unwrap() = true;

        thread::spawn(move || {
            if let Err(e) = Self::server_loop(port, running, sender) {
                error!("OSC server error: {}", e);
            }
        });

        info!("🎛️  OSC server started on port {}", self.port);
        Ok(())
    }

    /// Stop the OSC server
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
        info!("OSC server stopped");
    }

    /// Main server loop
    fn server_loop(
        port: u16,
        running: Arc<Mutex<bool>>,
        sender: std::sync::mpsc::Sender<LiveCommand>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
        socket.set_nonblocking(true)?;

        info!("OSC server listening on 0.0.0.0:{}", port);

        let mut buf = [0u8; 65536]; // Large buffer for complex OSC messages

        while *running.lock().unwrap() {
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    debug!("Received {} bytes from {}", size, addr);

                    match rosc::decoder::decode_udp(&buf[..size]) {
                        Ok((_remaining, packet)) => {
                            if let Some(cmd) = Self::handle_packet(packet) {
                                if let Err(e) = sender.send(cmd) {
                                    error!("Failed to send command: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode OSC packet: {}", e);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available, sleep briefly
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    error!("Socket error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle OSC packet
    fn handle_packet(packet: OscPacket) -> Option<LiveCommand> {
        match packet {
            OscPacket::Message(msg) => Self::handle_message(msg),
            OscPacket::Bundle(bundle) => {
                // Process first message in bundle
                for content in bundle.content {
                    if let Some(cmd) = Self::handle_packet(content) {
                        return Some(cmd);
                    }
                }
                None
            }
        }
    }

    /// Handle OSC message
    fn handle_message(msg: OscMessage) -> Option<LiveCommand> {
        debug!("OSC message: {} with {} args", msg.addr, msg.args.len());

        match msg.addr.as_str() {
            "/eval" => {
                // Extract Phonon code from first string argument
                if let Some(OscType::String(code)) = msg.args.first() {
                    info!("📝 /eval: {} chars", code.len());
                    return Some(LiveCommand::Eval { code: code.clone() });
                } else {
                    warn!("/eval requires string argument with Phonon code");
                }
            }
            "/hush" => {
                info!("🔇 /hush: stopping all audio");
                return Some(LiveCommand::Hush);
            }
            "/panic" => {
                info!("🚨 /panic: emergency stop");
                return Some(LiveCommand::Panic);
            }
            _ => {
                debug!("Unknown OSC address: {}", msg.addr);
            }
        }

        None
    }
}

/// Process OSC commands and update graph
pub fn apply_command_to_graph(cmd: &LiveCommand, sample_rate: f32) -> Option<UnifiedSignalGraph> {
    match cmd {
        LiveCommand::Eval { code } => {
            // Compile the DSL code into a new graph
            info!("Compiling: {}", code);

            match crate::unified_graph_parser::parse_dsl(code) {
                Ok((_remaining, statements)) => {
                    let compiler = DslCompiler::new(sample_rate);
                    let graph = compiler.compile(statements);
                    info!("✅ Compiled successfully");
                    Some(graph)
                }
                Err(e) => {
                    error!("❌ Parse error: {:?}", e);
                    None
                }
            }
        }
        LiveCommand::Hush => {
            // Create empty graph (silence)
            info!("Creating empty graph (hush)");
            let mut graph = UnifiedSignalGraph::new(sample_rate);
            graph.set_cps(1.0);
            // Set output to constant 0 (silence)
            let silence_node =
                graph.add_node(crate::unified_graph::SignalNode::Constant { value: 0.0 });
            graph.set_output(silence_node);
            Some(graph)
        }
        LiveCommand::Panic => {
            // Create empty graph immediately (emergency stop)
            warn!("⚠️  PANIC: Creating empty graph");
            let mut graph = UnifiedSignalGraph::new(sample_rate);
            graph.set_cps(1.0);
            let silence_node =
                graph.add_node(crate::unified_graph::SignalNode::Constant { value: 0.0 });
            graph.set_output(silence_node);
            Some(graph)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server() {
        let result = OscLiveServer::new(7770);
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_command() {
        let msg = OscMessage {
            addr: "/eval".to_string(),
            args: vec![OscType::String("~d1 = sine(440) * 0.2".to_string())],
        };

        let cmd = OscLiveServer::handle_message(msg);
        assert!(cmd.is_some());

        if let Some(LiveCommand::Eval { code }) = cmd {
            assert_eq!(code, "~d1 = sine(440) * 0.2");
        } else {
            panic!("Expected Eval command");
        }
    }

    #[test]
    fn test_hush_command() {
        let msg = OscMessage {
            addr: "/hush".to_string(),
            args: vec![],
        };

        let cmd = OscLiveServer::handle_message(msg);
        assert!(cmd.is_some());
        assert!(matches!(cmd.unwrap(), LiveCommand::Hush));
    }

    #[test]
    fn test_panic_command() {
        let msg = OscMessage {
            addr: "/panic".to_string(),
            args: vec![],
        };

        let cmd = OscLiveServer::handle_message(msg);
        assert!(cmd.is_some());
        assert!(matches!(cmd.unwrap(), LiveCommand::Panic));
    }

    #[test]
    fn test_apply_eval_command() {
        let cmd = LiveCommand::Eval {
            code: "cps: 2.0\n~d1: sine(440)".to_string(),
        };

        let graph = apply_command_to_graph(&cmd, 44100.0);
        assert!(graph.is_some());

        let graph = graph.unwrap();
        assert_eq!(graph.get_cps(), 2.0);
    }

    #[test]
    fn test_apply_hush_command() {
        let cmd = LiveCommand::Hush;
        let graph = apply_command_to_graph(&cmd, 44100.0);
        assert!(graph.is_some());

        // Hush should create a graph that outputs silence
        let mut graph = graph.unwrap();
        let sample = graph.process_sample();
        assert_eq!(sample, 0.0);
    }

    #[test]
    fn test_apply_panic_command() {
        let cmd = LiveCommand::Panic;
        let graph = apply_command_to_graph(&cmd, 44100.0);
        assert!(graph.is_some());

        // Panic should create a graph that outputs silence
        let mut graph = graph.unwrap();
        let sample = graph.process_sample();
        assert_eq!(sample, 0.0);
    }
}
