//! OSC server for receiving synthesis commands

use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::synth::SynthEngine;

pub struct OscServer {
    port: u16,
    engine: Arc<RwLock<SynthEngine>>,
}

impl OscServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            engine: Arc::new(RwLock::new(SynthEngine::new())),
        }
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
                self.handle_play(msg).await;
            }
            "/sample" => {
                self.handle_sample(msg).await;
            }
            "/sine" => {
                self.handle_sine(msg).await;
            }
            "/fm" => {
                self.handle_fm(msg).await;
            }
            "/chord" => {
                self.handle_chord(msg).await;
            }
            _ => {
                warn!("Unknown OSC address: {}", msg.addr);
            }
        }
    }
    
    async fn handle_play(&self, msg: OscMessage) {
        // Extract parameters
        let mut freq = 440.0f32;
        let mut duration = 0.25f32;
        
        for (i, arg) in msg.args.iter().enumerate() {
            match arg {
                OscType::Float(f) => {
                    if i == 0 {
                        freq = *f;
                    } else if i == 1 {
                        duration = *f;
                    }
                }
                OscType::Int(val) => {
                    if i == 0 {
                        freq = *val as f32;
                    }
                }
                _ => {}
            }
        }
        
        info!("Play: {} Hz for {} seconds", freq, duration);
        
        let engine = self.engine.read().await;
        let path = std::env::temp_dir().join(format!("fermion_{}.wav", freq));
        
        if let Err(e) = engine.render_sine(&path, freq, duration) {
            error!("Failed to render: {}", e);
            return;
        }
        
        // Play async
        tokio::spawn(async move {
            let _ = std::process::Command::new("mplayer")
                .arg(&path)
                .arg("-really-quiet")
                .output();
        });
    }
    
    async fn handle_sine(&self, msg: OscMessage) {
        if msg.args.len() < 2 {
            warn!("Sine requires freq and duration");
            return;
        }
        
        self.handle_play(msg).await;
    }
    
    async fn handle_fm(&self, msg: OscMessage) {
        if msg.args.len() < 3 {
            warn!("FM requires carrier, modulator, duration");
            return;
        }
        
        let carrier = match &msg.args[0] {
            OscType::Float(f) => *f,
            OscType::Int(i) => *i as f32,
            _ => return,
        };
        
        let modulator = match &msg.args[1] {
            OscType::Float(f) => *f,
            OscType::Int(i) => *i as f32,
            _ => return,
        };
        
        let duration = match &msg.args[2] {
            OscType::Float(f) => *f,
            OscType::Int(i) => *i as f32,
            _ => 0.25,
        };
        
        info!("FM: carrier={} mod={} dur={}", carrier, modulator, duration);
        
        let engine = self.engine.read().await;
        let path = std::env::temp_dir().join("fermion_fm.wav");
        
        if let Err(e) = engine.render_fm(&path, carrier, modulator, duration) {
            error!("Failed to render FM: {}", e);
            return;
        }
        
        tokio::spawn(async move {
            let _ = std::process::Command::new("mplayer")
                .arg(&path)
                .arg("-really-quiet")
                .output();
        });
    }
    
    async fn handle_sample(&self, msg: OscMessage) {
        // Extract sample name, index, and speed
        let mut sample_name = "bd".to_string();
        let mut index = 0usize;
        let mut speed = 1.0f32;
        
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
                    }
                }
                _ => {}
            }
        }
        
        info!("Sample: {}:{} at speed {}", sample_name, index, speed);
        
        let mut engine = self.engine.write().await;
        let path = std::env::temp_dir().join(format!("fermion_{}_{}.wav", sample_name, index));
        
        if let Err(e) = engine.play_sample(&path, &sample_name, index, speed) {
            error!("Failed to play sample: {}", e);
            return;
        }
        
        // Play async
        tokio::spawn(async move {
            let _ = std::process::Command::new("mplayer")
                .arg(&path)
                .arg("-really-quiet")
                .output();
        });
    }
    
    async fn handle_chord(&self, msg: OscMessage) {
        let mut freqs = Vec::new();
        let mut duration = 1.0f32;
        
        for arg in &msg.args {
            match arg {
                OscType::Float(f) => freqs.push(*f),
                OscType::Int(i) => freqs.push(*i as f32),
                _ => {}
            }
        }
        
        if freqs.is_empty() {
            warn!("Chord requires frequencies");
            return;
        }
        
        // Last value might be duration
        if freqs.len() > 1 && freqs.last().unwrap() < &10.0 {
            duration = freqs.pop().unwrap();
        }
        
        info!("Chord: {:?} for {} seconds", freqs, duration);
        
        let engine = self.engine.read().await;
        let path = std::env::temp_dir().join("fermion_chord.wav");
        
        if let Err(e) = engine.render_chord(&path, &freqs, duration) {
            error!("Failed to render chord: {}", e);
            return;
        }
        
        tokio::spawn(async move {
            let _ = std::process::Command::new("mplayer")
                .arg(&path)
                .arg("-really-quiet")
                .output();
        });
    }
}