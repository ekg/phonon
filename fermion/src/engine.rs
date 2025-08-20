//! Proper audio engine with scheduling and sample management
//! Similar to how Strudel/SuperCollider handle timing

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};
use std::time::{Instant, Duration};
use tracing::{info, error, warn, debug};
use std::path::{Path, PathBuf};

pub struct AudioEngine {
    sample_rate: u32,
    scheduler: Arc<Mutex<Scheduler>>,
    sample_bank: Arc<Mutex<SampleBank>>,
    _stream: cpal::Stream,
    start_time: Instant,
}

struct Scheduler {
    events: VecDeque<ScheduledEvent>,
    voices: Vec<Voice>,
    current_frame: u64,
    sample_rate: u32,
    lookahead_frames: u32,
}

struct ScheduledEvent {
    trigger_frame: u64,
    sample_id: String,
    speed: f32,
    gain: f32,
}

struct Voice {
    samples: Arc<Vec<f32>>,
    position: f32,
    speed: f32,
    gain: f32,
    active: bool,
}

pub struct SampleBank {
    samples: HashMap<String, Arc<Vec<f32>>>,
    dirt_samples_dir: PathBuf,
}

impl AudioEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Get audio host and device
        let host = cpal::default_host();
        info!("Audio host: {:?}", host.id());
        
        let device = host.default_output_device()
            .ok_or("No audio output device found")?;
        info!("Audio device: {}", device.name()?);
        
        let config = device.default_output_config()?;
        info!("Sample format: {:?}, Rate: {} Hz, Channels: {}", 
              config.sample_format(), 
              config.sample_rate().0,
              config.channels());
        
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        
        // Initialize sample bank
        let sample_bank = Arc::new(Mutex::new(SampleBank::new()));
        
        // Initialize scheduler with 100ms lookahead
        let scheduler = Arc::new(Mutex::new(Scheduler {
            events: VecDeque::new(),
            voices: Vec::with_capacity(32), // Pre-allocate 32 voices
            current_frame: 0,
            sample_rate,
            lookahead_frames: sample_rate / 10, // 100ms lookahead
        }));
        
        let scheduler_clone = scheduler.clone();
        let sample_bank_clone = sample_bank.clone();
        
        // Build output stream
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), scheduler_clone, sample_bank_clone, channels)
            },
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), scheduler_clone, sample_bank_clone, channels)
            },
            _ => return Err("Unsupported sample format".into()),
        }?;
        
        stream.play()?;
        info!("Audio engine started at {} Hz", sample_rate);
        
        Ok(Self {
            sample_rate,
            scheduler,
            sample_bank,
            _stream: stream,
            start_time: Instant::now(),
        })
    }
    
    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        scheduler: Arc<Mutex<Scheduler>>,
        sample_bank: Arc<Mutex<SampleBank>>,
        channels: usize,
    ) -> Result<cpal::Stream, Box<dyn std::error::Error>>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut scheduler = scheduler.lock().unwrap();
                let sample_bank = sample_bank.lock().unwrap();
                scheduler.process_audio(data, channels, &sample_bank);
            },
            |err| error!("Audio stream error: {}", err),
            None,
        )?;
        
        Ok(stream)
    }
    
    /// Schedule a sample to play at a specific time (in seconds from engine start)
    pub fn schedule_sample(&self, sample_id: &str, time: f64, speed: f32, gain: f32) {
        let trigger_frame = (time * self.sample_rate as f64) as u64;
        
        let mut scheduler = self.scheduler.lock().unwrap();
        scheduler.events.push_back(ScheduledEvent {
            trigger_frame,
            sample_id: sample_id.to_string(),
            speed,
            gain,
        });
        
        // Keep events sorted by trigger time
        scheduler.events.make_contiguous().sort_by_key(|e| e.trigger_frame);
        
        debug!("Scheduled {} at frame {} ({}s)", sample_id, trigger_frame, time);
    }
    
    /// Play a sample immediately
    pub fn play_sample(&self, sample_id: &str, speed: f32, gain: f32) {
        // Lazy load sample if not already loaded
        self.ensure_sample_loaded(sample_id);
        
        let elapsed = self.start_time.elapsed().as_secs_f64();
        // Add small latency to ensure we don't miss the trigger
        self.schedule_sample(sample_id, elapsed + 0.01, speed, gain);
    }
    
    /// Play synthesized audio immediately
    pub fn play_synth(&self, samples: Vec<f32>, gain: f32) {
        // Generate a unique ID for this synth sound
        let synth_id = format!("synth_{}", self.start_time.elapsed().as_millis());
        
        // Store the synthesized samples in the bank
        {
            let mut bank = self.sample_bank.lock().unwrap();
            bank.samples.insert(synth_id.clone(), Arc::new(samples));
        }
        
        // Play it immediately
        let elapsed = self.start_time.elapsed().as_secs_f64();
        self.schedule_sample(&synth_id, elapsed + 0.01, 1.0, gain);
    }
    
    /// Ensure a sample is loaded (lazy loading)
    fn ensure_sample_loaded(&self, sample_id: &str) {
        let mut bank = self.sample_bank.lock().unwrap();
        
        // Check if this specific sample (with index) is already loaded
        if bank.samples.contains_key(sample_id) {
            return;
        }
        
        // Parse sample name and index (e.g., "bd:0" -> ("bd", 0))
        let parts: Vec<&str> = sample_id.split(':').collect();
        let base_name = parts[0];
        let index = if parts.len() > 1 {
            parts[1].parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        
        // Try to load from dirt-samples
        let sample_paths = [
            ("bd", "bd/BT0A0A7.wav"),
            ("sn", "sn/ST0T0S0.wav"),
            ("hh", "hh/000_hh3closedhh.wav"),
            ("cp", "cp/HANDCLP0.wav"),
            ("arpy", "arpy/arpy01.wav"),
            ("bass", "bass/000_bass1.wav"),
            ("kick", "kick/kick01.wav"),
            ("808", "808/BD.WAV"),
        ];
        
        for (name, path) in sample_paths {
            if name == base_name && index == 0 {  // Only use hardcoded path for index 0
                let full_path = bank.dirt_samples_dir.join(path);
                if full_path.exists() {
                    if let Err(e) = bank.load_sample(sample_id, &full_path) {
                        warn!("Failed to lazy-load {}: {}", sample_id, e);
                    } else {
                        info!("Lazy-loaded sample: {}", sample_id);
                    }
                    return;
                }
            }
        }
        
        // Try generic path pattern
        let sample_dir = bank.dirt_samples_dir.join(base_name);
        if sample_dir.exists() && sample_dir.is_dir() {
            // Find WAV files in directory and sort them
            if let Ok(entries) = std::fs::read_dir(&sample_dir) {
                let mut wav_files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        let ext = p.extension().and_then(|s| s.to_str());
                        ext == Some("wav") || ext == Some("WAV")
                    })
                    .collect();
                
                // Sort files alphabetically/numerically
                wav_files.sort();
                
                // Get the file at the specified index
                if let Some(path) = wav_files.get(index) {
                    if let Err(e) = bank.load_sample(sample_id, path) {
                        warn!("Failed to lazy-load {}: {}", sample_id, e);
                    } else {
                        info!("Lazy-loaded sample: {} from {:?}", sample_id, path);
                    }
                    return;
                } else {
                    warn!("Sample index {} out of range for {} (has {} files)", 
                          index, base_name, wav_files.len());
                }
            }
        }
        
        warn!("Sample not found for lazy loading: {}", sample_id);
    }
    
    /// Load a sample from disk
    pub fn load_sample(&self, name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut bank = self.sample_bank.lock().unwrap();
        bank.load_sample(name, path)
    }
    
    /// Pre-load common samples
    pub fn load_default_samples(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut bank = self.sample_bank.lock().unwrap();
        bank.load_dirt_samples()
    }
    
    pub fn get_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

impl Scheduler {
    fn process_audio<T>(&mut self, output: &mut [T], channels: usize, sample_bank: &SampleBank)
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let frames = output.len() / channels;
        
        for frame_offset in 0..frames {
            let current_frame = self.current_frame + frame_offset as u64;
            
            // Check for events to trigger
            while let Some(event) = self.events.front() {
                if event.trigger_frame <= current_frame {
                    // Trigger this event
                    if let Some(samples) = sample_bank.get(&event.sample_id) {
                        // Find inactive voice or create new one
                        let voice_idx = self.voices.iter()
                            .position(|v| !v.active);
                        
                        let voice = if let Some(idx) = voice_idx {
                            &mut self.voices[idx]
                        } else {
                            self.voices.push(Voice {
                                samples: Arc::new(Vec::new()),
                                position: 0.0,
                                speed: 1.0,
                                gain: 1.0,
                                active: false,
                            });
                            self.voices.last_mut().unwrap()
                        };
                        
                        voice.samples = samples;
                        voice.position = 0.0;
                        voice.speed = event.speed;
                        voice.gain = event.gain;
                        voice.active = true;
                        
                        debug!("Triggered {} at frame {}", event.sample_id, current_frame);
                    } else {
                        warn!("Sample not found: {}", event.sample_id);
                    }
                    
                    self.events.pop_front();
                } else {
                    break; // Events are sorted, so we can stop here
                }
            }
            
            // Mix active voices
            let mut mixed = 0.0f32;
            
            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }
                
                let idx = voice.position as usize;
                if idx >= voice.samples.len() {
                    voice.active = false;
                    continue;
                }
                
                // Linear interpolation for sub-sample accuracy
                let sample = if voice.speed != 1.0 {
                    let frac = voice.position - idx as f32;
                    let s1 = voice.samples[idx];
                    let s2 = voice.samples.get(idx + 1).copied().unwrap_or(0.0);
                    s1 * (1.0 - frac) + s2 * frac
                } else {
                    voice.samples[idx]
                };
                
                mixed += sample * voice.gain;
                voice.position += voice.speed;
            }
            
            // Soft clipping
            mixed = mixed.tanh() * 0.9;
            
            // Write to all channels
            let base_idx = frame_offset * channels;
            for ch in 0..channels {
                output[base_idx + ch] = T::from_sample(mixed);
            }
        }
        
        self.current_frame += frames as u64;
    }
}

impl SampleBank {
    fn new() -> Self {
        let base_dir = PathBuf::from(std::env::var("HOME").unwrap_or(".".to_string()))
            .join("phonon");
        
        Self {
            samples: HashMap::new(),
            dirt_samples_dir: base_dir.join("dirt-samples"),
        }
    }
    
    fn load_sample(&mut self, name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if self.samples.contains_key(name) {
            return Ok(()); // Already loaded
        }
        
        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        
        // Convert to mono f32
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>()
                    .map(|s| s.unwrap_or(0.0))
                    .collect()
            },
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader.samples::<i32>()
                    .map(|s| s.unwrap_or(0) as f32 / max_val)
                    .collect()
            },
        };
        
        // Convert to mono if stereo
        let mono_samples = if spec.channels == 2 {
            samples.chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).copied().unwrap_or(0.0)) * 0.5)
                .collect()
        } else {
            samples
        };
        
        info!("Loaded sample '{}' ({} samples)", name, mono_samples.len());
        self.samples.insert(name.to_string(), Arc::new(mono_samples));
        
        Ok(())
    }
    
    fn load_dirt_samples(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load some common drum samples
        let drums = [
            ("bd", "bd/BT0A0A7.wav"),
            ("sn", "sn/ST0T0S0.wav"),
            ("hh", "hh/000_hh3closedhh.wav"),
            ("cp", "cp/HANDCLP0.wav"),
        ];
        
        for (name, path) in drums {
            let full_path = self.dirt_samples_dir.join(path);
            if full_path.exists() {
                if let Err(e) = self.load_sample(name, &full_path) {
                    warn!("Failed to load {}: {}", name, e);
                }
            }
        }
        
        Ok(())
    }
    
    fn get(&self, name: &str) -> Option<Arc<Vec<f32>>> {
        // Try exact match first
        if let Some(samples) = self.samples.get(name) {
            return Some(samples.clone());
        }
        
        // Try to parse "sample:index" format (e.g., "bd:0")
        if let Some((base, _index)) = name.split_once(':') {
            if let Some(samples) = self.samples.get(base) {
                return Some(samples.clone());
            }
        }
        
        None
    }
}