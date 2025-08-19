//! OSC server for receiving synthesis commands

use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::engine::AudioEngine;

pub struct OscServer {
    port: u16,
    engine: Arc<AudioEngine>,
}

impl OscServer {
    pub fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = AudioEngine::new()?;
        // Don't pre-load samples - they'll be lazy-loaded on demand
        
        Ok(Self {
            port,
            engine: Arc::new(engine),
        })
    }
    
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let socket = UdpSocket::bind(&addr)?;
        socket.set_nonblocking(true)?;
        
        info!("Fermion OSC server listening on {}", addr);
        
        let mut buf = [0u8; 65536];
        
        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    debug!("Received {} bytes from {}", size, addr);
                    
                    match rosc::decoder::decode_udp(&buf[..size]) {
                        Ok((_, packet)) => {
                            self.handle_packet(packet).await;
                        }
                        Err(e) => {
                            error!("Failed to decode OSC packet: {}", e);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available, sleep briefly
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
                Err(e) => {
                    error!("Socket error: {}", e);
                }
            }
        }
    }
    
    async fn handle_packet(&self, packet: OscPacket) {
        match packet {
            OscPacket::Message(msg) => {
                self.handle_message(msg).await;
            }
            OscPacket::Bundle(bundle) => {
                for packet in bundle.content {
                    Box::pin(self.handle_packet(packet)).await;
                }
            }
        }
    }
    
    async fn handle_message(&self, msg: OscMessage) {
        debug!("OSC message: {} with {} args", msg.addr, msg.args.len());
        
        match msg.addr.as_str() {
            "/play" => {
                // TODO: Implement synth playback
                warn!("Synth playback not yet implemented");
            }
            "/sample" => {
                self.handle_sample(msg).await;
            }
            _ => {
                warn!("Unknown OSC address: {}", msg.addr);
            }
        }
    }
    
    async fn handle_sample(&self, msg: OscMessage) {
        // Extract sample name, index, speed, and gain
        let mut sample_name = "bd".to_string();
        let mut index = 0usize;
        let mut speed = 1.0f32;
        let mut gain = 1.0f32;
        
        for (i, arg) in msg.args.iter().enumerate() {
            match arg {
                OscType::String(s) => {
                    if i == 0 {
                        sample_name = s.clone();
                    }
                }
                OscType::Int(val) => {
                    if i == 1 {
                        index = *val as usize;
                    } else if i == 2 {
                        speed = *val as f32;
                    }
                }
                OscType::Float(f) => {
                    if i == 2 {
                        speed = *f;
                    } else if i == 3 {
                        gain = *f;
                    }
                }
                _ => {}
            }
        }
        
        info!("Sample: {}:{} at speed {} gain {}", sample_name, index, speed, gain);
        
        // Format sample name for our engine
        let sample_id = if index > 0 {
            format!("{}:{}", sample_name, index)
        } else {
            sample_name.clone()
        };
        
        // Play through the audio engine (instant, low-latency)
        self.engine.play_sample(&sample_id, speed, gain);
        debug!("Triggered sample: {}", sample_id);
    }
}