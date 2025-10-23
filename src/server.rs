#![allow(unused_assignments, unused_mut)]
//! OSC server for receiving synthesis commands

use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};

use crate::engine::AudioEngine;

pub struct OscServer {
    port: u16,
    engine: Arc<AudioEngine>,
    synth_registry: Arc<RwLock<crate::synth_defs::SynthRegistry>>,
}

impl OscServer {
    pub fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = AudioEngine::new()?;
        // Don't pre-load samples - they'll be lazy-loaded on demand
        let synth_registry = crate::synth_defs::SynthRegistry::new();
        info!("Loaded {} synth definitions", synth_registry.count());
        
        Ok(Self {
            port,
            engine: Arc::new(engine),
            synth_registry: Arc::new(RwLock::new(synth_registry)),
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
            "/synth" => {
                self.handle_synth(msg).await;
            }
            "/synthdef" => {
                self.handle_synthdef(msg).await;
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
        // Always include index to ensure correct sample is loaded
        let sample_id = format!("{}:{}", sample_name, index);
        
        // Play through the audio engine (instant, low-latency)
        self.engine.play_sample(&sample_id, speed, gain);
        debug!("Triggered sample: {}", sample_id);
    }
    
    async fn handle_synth(&self, msg: OscMessage) {
        use crate::synth_defs::{parse_synth_def, compile_synth};
        
        // Extract synth name or definition string
        let mut synth_name = "sine(440)".to_string();
        let mut duration = 0.5f32;
        let mut gain = 0.5f32;
        
        for (i, arg) in msg.args.iter().enumerate() {
            match arg {
                OscType::String(s) => {
                    if i == 0 {
                        synth_name = s.clone();
                    }
                }
                OscType::Float(f) => {
                    if i == 1 {
                        duration = *f;
                    } else if i == 2 {
                        gain = *f;
                    }
                }
                _ => {}
            }
        }
        
        info!("Synth: {} for {}s at gain {}", synth_name, duration, gain);
        
        // First check if it's a named synth from the registry
        let registry = self.synth_registry.read().unwrap();
        if let Some(def) = registry.get(&synth_name) {
            info!("Using registered synth: {}", synth_name);
            let samples = compile_synth(&def, duration as f64);
            self.engine.play_synth(samples, gain);
        } else {
            drop(registry); // Release lock before parsing
            // Try to parse as inline definition
            match parse_synth_def(&synth_name) {
                Ok(def) => {
                    info!("Parsed inline synth definition: {:?}", def);
                    let samples = compile_synth(&def, duration as f64);
                    self.engine.play_synth(samples, gain);
                }
                Err(e) => {
                    warn!("Unknown synth '{}' (not in registry, failed to parse: {})", synth_name, e);
                }
            }
        }
    }
    
    async fn handle_synthdef(&self, msg: OscMessage) {
        use crate::synth_defs::parse_synth_def;
        
        // Extract name and definition
        let mut name = String::new();
        let mut definition = String::new();
        
        for (i, arg) in msg.args.iter().enumerate() {
            match arg {
                OscType::String(s) => {
                    if i == 0 {
                        name = s.clone();
                    } else if i == 1 {
                        definition = s.clone();
                    }
                }
                _ => {}
            }
        }
        
        if name.is_empty() || definition.is_empty() {
            warn!("Invalid synthdef message: missing name or definition");
            return;
        }
        
        info!("Registering synthdef: {} = {}", name, definition);
        
        // Parse the definition
        match parse_synth_def(&definition) {
            Ok(def) => {
                // Register the synthdef in our registry
                let mut registry = self.synth_registry.write().unwrap();
                registry.register(&name, def.clone());
                info!("Successfully registered synthdef '{}': {:?}", name, def);
            }
            Err(e) => {
                warn!("Failed to parse synthdef '{}': {}", name, e);
            }
        }
    }
}