#![allow(unused_assignments, unused_mut)]
//! Synthesis engine using FunDSP

use fundsp::hacker::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub struct SynthEngine {
    sample_rate: f64,
    samples: HashMap<String, Vec<f32>>,
    sample_dir: PathBuf,
    dirt_samples_dir: PathBuf,
}

impl Default for SynthEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthEngine {
    pub fn new() -> Self {
        let base_dir =
            PathBuf::from(std::env::var("HOME").unwrap_or(".".to_string())).join("phonon");

        let sample_dir = base_dir.join("samples");
        let dirt_samples_dir = base_dir.join("dirt-samples");

        // Create sample directory if it doesn't exist
        std::fs::create_dir_all(&sample_dir).ok();

        let mut engine = Self {
            sample_rate: 44100.0,
            samples: HashMap::new(),
            sample_dir,
            dirt_samples_dir,
        };

        // Load default samples
        engine.load_default_samples();
        engine
    }

    pub fn play_test(
        &mut self,
        freq: f32,
        duration: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join("fermion_test.wav");
        self.render_sine(&path, freq, duration)?;

        // Play with mplayer (or aplay as fallback)
        if std::process::Command::new("which")
            .arg("mplayer")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            std::process::Command::new("mplayer")
                .arg(&path)
                .arg("-really-quiet")
                .spawn()?;
        } else {
            std::process::Command::new("aplay")
                .arg("-q")
                .arg(&path)
                .spawn()?;
        }

        Ok(())
    }

    pub fn render_sine(
        &self,
        path: &Path,
        freq: f32,
        duration: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rendering sine wave: {} Hz for {} seconds", freq, duration);

        // Create stereo sine with envelope
        let graph = (sine_hz(freq) * 0.5) >> split::<U2>();
        self.render_graph(path, Box::new(graph), duration)
    }

    pub fn render_fm(
        &self,
        path: &Path,
        carrier: f32,
        modulator: f32,
        duration: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rendering FM: carrier={} mod={}", carrier, modulator);

        // Simple FM synthesis
        let graph = (sine_hz(carrier) * (sine_hz(modulator) * 200.0 + 1.0) * 0.5) >> split::<U2>();
        self.render_graph(path, Box::new(graph), duration)
    }

    pub fn render_chord(
        &self,
        path: &Path,
        freqs: &[f32],
        duration: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rendering chord: {:?}", freqs);

        if freqs.is_empty() {
            return Err("No frequencies provided".into());
        }

        // For now, just play the first frequency (simpler)
        // TODO: Implement proper chord synthesis
        self.render_sine(path, freqs[0], duration)
    }

    fn load_default_samples(&mut self) {
        // Generate some basic drum samples
        info!("Generating default samples");

        // Kick drum: low sine with envelope
        let kick = self.generate_kick();
        self.samples.insert("bd".to_string(), kick);
        self.samples
            .insert("kick".to_string(), self.samples["bd"].clone());

        // Snare: noise burst
        let snare = self.generate_snare();
        self.samples.insert("sd".to_string(), snare);
        self.samples
            .insert("snare".to_string(), self.samples["sd"].clone());

        // Hi-hat: short noise
        let hihat = self.generate_hihat();
        self.samples.insert("hh".to_string(), hihat);
        self.samples
            .insert("hihat".to_string(), self.samples["hh"].clone());

        info!("Loaded {} default samples", self.samples.len());
    }

    fn generate_kick(&self) -> Vec<f32> {
        let duration = 0.2;
        let samples = (self.sample_rate * duration) as usize;
        let mut buffer = Vec::with_capacity(samples);

        for i in 0..samples {
            let t = i as f64 / self.sample_rate;
            let env = (-(t * 20.0)).exp();
            let freq = 60.0 * (1.0 + (-(t * 50.0)).exp());
            let sample = (2.0 * std::f64::consts::PI * freq * t).sin() * env;
            buffer.push(sample as f32);
        }
        buffer
    }

    fn generate_snare(&self) -> Vec<f32> {
        let duration = 0.15;
        let samples = (self.sample_rate * duration) as usize;
        let mut buffer = Vec::with_capacity(samples);

        for i in 0..samples {
            let t = i as f64 / self.sample_rate;
            let env = (-(t * 30.0)).exp();
            let noise = (rand::random::<f64>() * 2.0 - 1.0) * 0.5;
            let tone = (2.0 * std::f64::consts::PI * 200.0 * t).sin() * 0.3;
            buffer.push((noise + tone) as f32 * env as f32);
        }
        buffer
    }

    fn generate_hihat(&self) -> Vec<f32> {
        let duration = 0.05;
        let samples = (self.sample_rate * duration) as usize;
        let mut buffer = Vec::with_capacity(samples);

        for i in 0..samples {
            let t = i as f64 / self.sample_rate;
            let env = (-(t * 100.0)).exp();
            let noise = (rand::random::<f64>() * 2.0 - 1.0) * env;
            buffer.push(noise as f32);
        }
        buffer
    }

    pub fn load_sample(
        &mut self,
        name: &str,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // For now, just support raw WAV loading
        // TODO: Implement proper WAV parsing
        info!("Loading sample {} from {:?}", name, path);
        self.samples.insert(name.to_string(), vec![]);
        Ok(())
    }

    pub fn load_dirt_sample(
        &mut self,
        name: &str,
        index: usize,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // Find the sample folder
        let sample_folder = self.dirt_samples_dir.join(name);

        if !sample_folder.exists() {
            // Try the regular samples directory
            let alt_folder = self.sample_dir.join(name);
            if !alt_folder.exists() {
                return Err(format!("Sample folder not found: {name}").into());
            }
        }

        // List WAV files in the folder
        let mut wav_files: Vec<_> = fs::read_dir(&sample_folder)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("wav"))
                    .unwrap_or(false)
            })
            .map(|entry| entry.path())
            .collect();

        // Sort files for consistent indexing
        wav_files.sort();

        if wav_files.is_empty() {
            return Err(format!("No WAV files found in {name}").into());
        }

        // Select the file by index (wrapping if necessary)
        let file_path = &wav_files[index % wav_files.len()];

        info!("Loading sample: {:?}", file_path);

        // For now, return empty vec - we'd need to implement WAV parsing
        // In production, we'd use the hound crate to read the WAV file
        Ok(vec![])
    }

    pub fn play_sample(
        &mut self,
        path: &Path,
        sample_name: &str,
        index: usize,
        speed: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if it's a cached sample
        let cache_key = format!("{sample_name}:{index}");

        let sample_data = if let Some(data) = self.samples.get(&cache_key) {
            data.clone()
        } else {
            // Try to load from dirt-samples
            match self.load_dirt_sample(sample_name, index) {
                Ok(data) if !data.is_empty() => {
                    // Cache the loaded sample
                    self.samples.insert(cache_key.clone(), data.clone());
                    data
                }
                _ => {
                    // Fall back to generated samples
                    if let Some(data) = self.samples.get(sample_name) {
                        data.clone()
                    } else {
                        return Err(format!("Sample not found: {sample_name}:{index}").into());
                    }
                }
            }
        };

        info!(
            "Playing sample: {}:{} at speed {}",
            sample_name, index, speed
        );

        // Resample if speed != 1.0
        let output = if (speed - 1.0).abs() > 0.01 {
            self.resample(&sample_data, speed)
        } else {
            sample_data
        };

        // Write WAV
        self.write_wav(path, &output)?;
        Ok(())
    }

    fn resample(&self, input: &[f32], rate: f32) -> Vec<f32> {
        let output_len = (input.len() as f32 / rate) as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_idx = (i as f32 * rate) as usize;
            if src_idx < input.len() {
                output.push(input[src_idx]);
            }
        }
        output
    }

    fn write_wav(&self, path: &Path, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        // WAV header
        file.write_all(b"RIFF")?;
        file.write_all(&((36 + samples.len() * 2) as u32).to_le_bytes())?;
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
        file.write_all(&((samples.len() * 2) as u32).to_le_bytes())?;

        for sample in samples {
            let s16 = (sample * 32767.0) as i16;
            file.write_all(&s16.to_le_bytes())?;
        }

        Ok(())
    }

    fn render_graph(
        &self,
        path: &Path,
        mut graph: Box<dyn AudioUnit>,
        duration: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
