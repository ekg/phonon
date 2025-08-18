//! Synthesis engine using FunDSP

use fundsp::hacker::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::{debug, info};

pub struct SynthEngine {
    sample_rate: f64,
}

impl SynthEngine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
        }
    }
    
    pub fn play_test(&mut self, freq: f32, duration: f32) -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join("fermion_test.wav");
        self.render_sine(&path, freq, duration)?;
        
        // Play with mplayer
        std::process::Command::new("mplayer")
            .arg(&path)
            .arg("-really-quiet")
            .spawn()?;
        
        Ok(())
    }
    
    pub fn render_sine(&self, path: &Path, freq: f32, duration: f32) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rendering sine wave: {} Hz for {} seconds", freq, duration);
        
        // Create stereo sine with envelope
        let graph = (sine_hz(freq) * 0.5) >> split::<U2>();
        self.render_graph(path, Box::new(graph), duration)
    }
    
    pub fn render_fm(&self, path: &Path, carrier: f32, modulator: f32, duration: f32) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rendering FM: carrier={} mod={}", carrier, modulator);
        
        // Simple FM synthesis
        let graph = (sine_hz(carrier) * (sine_hz(modulator) * 200.0 + 1.0) * 0.5) >> split::<U2>();
        self.render_graph(path, Box::new(graph), duration)
    }
    
    pub fn render_chord(&self, path: &Path, freqs: &[f32], duration: f32) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rendering chord: {:?}", freqs);
        
        if freqs.is_empty() {
            return Err("No frequencies provided".into());
        }
        
        // For now, just play the first frequency (simpler)
        // TODO: Implement proper chord synthesis
        self.render_sine(path, freqs[0], duration)
    }
    
    fn render_graph(&self, path: &Path, mut graph: Box<dyn AudioUnit>, duration: f32) -> Result<(), Box<dyn std::error::Error>> {
        let samples = (self.sample_rate * duration as f64) as usize;
        
        graph.reset();
        graph.set_sample_rate(self.sample_rate);
        
        let mut buffer = Vec::with_capacity(samples);
        let mut output = [0.0f32; 2];
        let input = [0.0f32; 0];
        
        for _ in 0..samples {
            graph.tick(&input, &mut output);
            buffer.push((output[0] * 32767.0) as i16);
        }
        
        // Write WAV
        let mut file = File::create(path)?;
        
        // WAV header
        file.write_all(b"RIFF")?;
        file.write_all(&((36 + buffer.len() * 2) as u32).to_le_bytes())?;
        file.write_all(b"WAVE")?;
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?; // PCM
        file.write_all(&1u16.to_le_bytes())?; // Mono
        file.write_all(&44100u32.to_le_bytes())?;
        file.write_all(&88200u32.to_le_bytes())?;
        file.write_all(&2u16.to_le_bytes())?;
        file.write_all(&16u16.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&((buffer.len() * 2) as u32).to_le_bytes())?;
        
        for sample in buffer {
            file.write_all(&sample.to_le_bytes())?;
        }
        
        debug!("Rendered {} samples to {:?}", samples, path);
        Ok(())
    }
}