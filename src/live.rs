#![allow(unused_assignments, unused_mut)]
//! Live coding module for Phonon
//!
//! Provides file watching and hot-reloading for live DSL editing with
//! callback-driven audio rendering for perfect timing.

use crate::compositional_compiler::compile_program;
use crate::compositional_parser::parse_program;
use crate::unified_graph::UnifiedSignalGraph;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

/// Live coding session with callback-driven audio
pub struct LiveSession {
    current_file: PathBuf,
    graph: Arc<Mutex<Option<UnifiedSignalGraph>>>,
    last_modified: SystemTime,
    sample_rate: f32,
    _stream: cpal::Stream, // Keep stream alive
}

impl LiveSession {
    pub fn new(file_path: &Path) -> Result<Self, String> {
        // Get audio device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default config: {}", e))?;

        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;

        println!("🎵 Audio: {} Hz, {} channels", sample_rate as u32, channels);

        // Shared graph (starts as None, will be loaded)
        let graph = Arc::new(Mutex::new(None));
        let graph_clone = graph.clone();

        // Build audio stream with callback
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => Self::build_stream::<f32>(
                &device,
                &config.into(),
                graph_clone,
                channels,
            ),
            cpal::SampleFormat::I16 => Self::build_stream::<i16>(
                &device,
                &config.into(),
                graph_clone,
                channels,
            ),
            _ => return Err("Unsupported sample format".to_string()),
        }
        .map_err(|e| format!("Failed to build stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;

        // Get initial file modification time
        let metadata = fs::metadata(file_path)
            .map_err(|e| format!("Cannot read file {}: {}", file_path.display(), e))?;
        let last_modified = metadata
            .modified()
            .map_err(|e| format!("Cannot get modification time: {e}"))?;

        Ok(Self {
            current_file: file_path.to_path_buf(),
            graph,
            last_modified,
            sample_rate,
            _stream: stream,
        })
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        graph: Arc<Mutex<Option<UnifiedSignalGraph>>>,
        channels: usize,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                // Audio callback - called by hardware when it needs samples
                let mut graph_lock = graph.lock().unwrap();

                if let Some(graph) = graph_lock.as_mut() {
                    // Generate samples directly from graph
                    for frame in data.chunks_mut(channels) {
                        let sample = graph.process_sample();

                        // Write to all channels (mono to stereo)
                        for channel_sample in frame.iter_mut() {
                            *channel_sample = T::from_sample(sample);
                        }
                    }
                } else {
                    // No graph loaded yet - output silence
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0);
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )
    }

    /// Load and compile the DSL file
    pub fn load_file(&mut self) -> Result<(), String> {
        println!("🔄 Loading: {}", self.current_file.display());

        // Read the file
        let dsl_code = fs::read_to_string(&self.current_file)
            .map_err(|e| format!("Failed to read file: {e}"))?;

        // Parse and compile
        let (rest, statements) = parse_program(&dsl_code)
            .map_err(|e| format!("Parse error: {}", e))?;

        if !rest.trim().is_empty() {
            return Err(format!("Failed to parse entire file, remaining: {}", rest));
        }

        let mut new_graph = compile_program(statements, self.sample_rate)
            .map_err(|e| format!("Compile error: {}", e))?;

        // Set tempo from DSL (default 1.0)
        new_graph.set_cps(1.0); // This will be overridden if tempo: is in the file

        // Hot-swap the graph atomically
        *self.graph.lock().unwrap() = Some(new_graph);

        // Update modification time
        let metadata = fs::metadata(&self.current_file)
            .map_err(|e| format!("Cannot read file metadata: {e}"))?;
        self.last_modified = metadata
            .modified()
            .map_err(|e| format!("Cannot get modification time: {e}"))?;

        println!("✅ Loaded successfully!");

        Ok(())
    }

    /// Check if file has been modified
    fn check_file_modified(&self) -> bool {
        if let Ok(metadata) = fs::metadata(&self.current_file) {
            if let Ok(modified) = metadata.modified() {
                return modified > self.last_modified;
            }
        }
        false
    }

    /// Start the live session with file watching
    /// Audio is rendered by the callback (driven by hardware clock)
    pub fn run(&mut self) -> Result<(), String> {
        // Initial load
        self.load_file()?;

        println!("\n🎵 Live coding session started!");
        println!("📝 Editing: {}", self.current_file.display());
        println!("🔊 Audio: callback-driven (hardware-timed)");
        println!("\nPress Ctrl+C to stop\n");

        // File watching loop (audio runs independently in callback)
        loop {
            // Check for file changes
            if self.check_file_modified() {
                println!("\n📝 File changed, reloading...");
                match self.load_file() {
                    Ok(_) => {
                        println!("✅ Reloaded successfully!");
                    }
                    Err(e) => {
                        println!("❌ Reload failed: {e}");
                        println!("   Fix the error and save again to retry");
                    }
                }
            }

            // Sleep to avoid busy-waiting
            thread::sleep(Duration::from_millis(100));
        }
    }
}

/// Watch multiple files and hot-reload on changes
pub struct MultiFileWatcher {
    sessions: Vec<LiveSession>,
}

impl Default for MultiFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiFileWatcher {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: &Path) -> Result<(), String> {
        let session = LiveSession::new(path)?;
        self.sessions.push(session);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), String> {
        if self.sessions.is_empty() {
            return Err("No files to watch".to_string());
        }

        // For now, just run the first session
        // TODO: Support multiple simultaneous sessions
        self.sessions[0].run()
    }
}

/// Simple REPL for live DSL evaluation
/// Note: This still uses the old scheduling-based engine and may have timing issues
/// TODO: Rewrite to use callback-driven rendering like LiveSession
pub struct LiveRepl {
    sample_rate: f32,
}

impl LiveRepl {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            sample_rate: 44100.0,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        println!("🎵 Phonon Live REPL");
        println!("==================");
        println!("⚠️  Warning: REPL mode may have timing issues");
        println!("   Use 'phonon live file.ph' for accurate playback");
        println!("\nType 'exit' to quit\n");

        Err("REPL mode temporarily disabled - use 'phonon live file.ph' instead".to_string())
    }
}
