//! Live coding module for Phonon
//!
//! Provides file watching and hot-reloading for live DSL editing

use crate::engine::AudioEngine;
use crate::enhanced_parser::EnhancedParser;
use crate::signal_executor::SignalExecutor;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

/// Messages for controlling the audio engine
#[derive(Debug, Clone)]
pub enum AudioMessage {
    LoadPatch(String),            // Load new DSL code
    UpdateParameter(String, f32), // Update a bus value
    Stop,                         // Stop playback
    Play,                         // Start playback
}

/// Live coding session
pub struct LiveSession {
    engine: Arc<AudioEngine>,
    current_file: PathBuf,
    current_executor: Arc<Mutex<Option<SignalExecutor>>>,
    last_modified: SystemTime,
    sample_rate: f32,
    block_size: usize,
    auto_reload: bool,
    pattern_mode: bool,
}

impl LiveSession {
    pub fn new(file_path: &Path) -> Result<Self, String> {
        // Initialize audio engine
        let engine =
            Arc::new(AudioEngine::new().map_err(|e| format!("Failed to init audio: {e}"))?);

        // Get initial file modification time
        let metadata = fs::metadata(file_path)
            .map_err(|e| format!("Cannot read file {}: {}", file_path.display(), e))?;
        let last_modified = metadata
            .modified()
            .map_err(|e| format!("Cannot get modification time: {e}"))?;

        Ok(Self {
            engine,
            current_file: file_path.to_path_buf(),
            current_executor: Arc::new(Mutex::new(None)),
            last_modified,
            sample_rate: 44100.0,
            block_size: 512,
            auto_reload: true,
            pattern_mode: false,
        })
    }

    /// Load and compile the DSL file
    pub fn load_file(&mut self) -> Result<(), String> {
        println!("🔄 Loading: {}", self.current_file.display());

        // Read the file
        let dsl_code = fs::read_to_string(&self.current_file)
            .map_err(|e| format!("Failed to read file: {e}"))?;

        // Parse the DSL
        let mut parser = EnhancedParser::new(self.sample_rate);
        let graph = parser.parse(&dsl_code)?;

        // Create new executor
        let mut executor = SignalExecutor::new(graph, self.sample_rate, self.block_size);
        executor.initialize()?;

        // Store the executor
        *self.current_executor.lock().unwrap() = Some(executor);

        // Update modification time
        let metadata = fs::metadata(&self.current_file)
            .map_err(|e| format!("Cannot read file metadata: {e}"))?;
        self.last_modified = metadata
            .modified()
            .map_err(|e| format!("Cannot get modification time: {e}"))?;

        println!("✅ Loaded successfully!");
        self.print_status();

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
    pub fn run(&mut self) -> Result<(), String> {
        // Initial load
        self.load_file()?;

        println!("\n🎵 Live coding session started!");
        println!("📝 Editing: {}", self.current_file.display());
        println!(
            "♻️  Auto-reload: {}",
            if self.auto_reload { "ON" } else { "OFF" }
        );
        println!("\nPress Ctrl+C to stop\n");

        // Main processing loop
        let mut last_process = std::time::Instant::now();
        let process_interval = Duration::from_millis(10);

        loop {
            // Check for file changes
            if self.auto_reload && self.check_file_modified() {
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

            // Process audio at regular intervals
            if last_process.elapsed() >= process_interval {
                if let Some(ref mut executor) = *self.current_executor.lock().unwrap() {
                    if let Ok(output) = executor.process_block() {
                        // Play the audio
                        let mut samples = Vec::with_capacity(self.block_size);
                        samples.extend_from_slice(&output.data);
                        self.engine.play_synth(samples.clone(), 1.0);
                    }
                }
                last_process = std::time::Instant::now();
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    /// Run in pattern mode (for Strudel-style patterns)
    pub fn run_pattern_mode(&mut self) -> Result<(), String> {
        self.pattern_mode = true;
        println!("🎼 Pattern mode enabled");

        // TODO: Implement pattern sequencing
        // This would parse pattern strings like "bd sn bd sn"
        // and trigger samples/synths accordingly

        self.run()
    }

    fn print_status(&self) {
        println!("──────────────────────────");
        println!("Sample rate: {} Hz", self.sample_rate as u32);
        println!("Block size:  {} samples", self.block_size);
        println!(
            "Latency:     ~{:.1} ms",
            (self.block_size as f32 / self.sample_rate * 1000.0)
        );
        println!("──────────────────────────");
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
pub struct LiveRepl {
    engine: AudioEngine,
    sample_rate: f32,
    block_size: usize,
}

impl LiveRepl {
    pub fn new() -> Result<Self, String> {
        let engine = AudioEngine::new().map_err(|e| format!("Failed to init audio: {e}"))?;

        Ok(Self {
            engine,
            sample_rate: 44100.0,
            block_size: 512,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        use std::io::{self, Write};

        println!("🎵 Phonon Live REPL");
        println!("==================");
        println!("Type DSL code and press Enter twice to evaluate");
        println!("Type 'exit' to quit\n");

        loop {
            print!("phonon> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            let mut empty_lines = 0;

            // Read until we get two empty lines or 'exit'
            loop {
                let mut line = String::new();
                io::stdin().read_line(&mut line).unwrap();

                if line.trim() == "exit" {
                    return Ok(());
                }

                if line.trim().is_empty() {
                    empty_lines += 1;
                    if empty_lines >= 2 {
                        break;
                    }
                } else {
                    empty_lines = 0;
                }

                input.push_str(&line);
            }

            if !input.trim().is_empty() {
                self.evaluate(&input);
            }
        }
    }

    fn evaluate(&mut self, dsl_code: &str) {
        // Parse the DSL
        let mut parser = EnhancedParser::new(self.sample_rate);
        match parser.parse(dsl_code) {
            Ok(graph) => {
                // Create executor
                let mut executor = SignalExecutor::new(graph, self.sample_rate, self.block_size);
                if let Err(e) = executor.initialize() {
                    println!("❌ Initialization error: {e}");
                    return;
                }

                // Generate a short sample
                println!("🔊 Playing...");
                let mut all_samples = Vec::new();
                for _ in 0..86 {
                    // ~1 second
                    if let Ok(output) = executor.process_block() {
                        all_samples.extend_from_slice(&output.data);
                    }
                }

                // Play it
                self.engine.play_synth(all_samples, 1.0);
                println!("✅ Done");
            }
            Err(e) => {
                println!("❌ Parse error: {e}");
            }
        }
    }
}
