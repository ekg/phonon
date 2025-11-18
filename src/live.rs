#![allow(unused_assignments, unused_mut)]
//! Live coding module for Phonon
//!
//! Provides file watching and hot-reloading for live DSL editing with
//! high-performance ring buffer audio rendering.
//!
//! Architecture:
//! 1. File watcher: Detects changes and reloads graph
//! 2. Background synth thread: Continuously renders samples → ring buffer
//! 3. Audio callback: Just reads from ring buffer (FAST!)

use crate::compositional_compiler::compile_program;
use crate::compositional_parser::parse_program;
use crate::unified_graph::UnifiedSignalGraph;
use arc_swap::ArcSwap;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

// Newtype wrapper to impl Send+Sync for RefCell<UnifiedSignalGraph>
// SAFETY: Each GraphCell instance is only accessed by one thread at a time.
struct GraphCell(RefCell<UnifiedSignalGraph>);
unsafe impl Send for GraphCell {}
unsafe impl Sync for GraphCell {}

/// Live coding session with high-performance ring buffer audio
pub struct LiveSession {
    current_file: PathBuf,
    graph: Arc<ArcSwap<Option<GraphCell>>>,
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

        let default_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default config: {}", e))?;

        let sample_rate = default_config.sample_rate().0 as f32;
        let channels = default_config.channels() as usize;
        let sample_format = default_config.sample_format();

        // Create config - use default buffer size (ring buffer handles buffering)
        let config: cpal::StreamConfig = default_config.into();

        println!("🎵 Audio: {} Hz, {} channels",
                 sample_rate as u32, channels);
        println!("🔧 Using ring buffer architecture for parallel synthesis");

        // Graph for background synthesis thread (lock-free swap)
        let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));

        // Ring buffer: background synth writes, audio callback reads
        // Size: 1 second of audio = smooth playback even if synth lags briefly
        let ring_buffer_size = sample_rate as usize;
        let ring = HeapRb::<f32>::new(ring_buffer_size);
        let (mut ring_producer, mut ring_consumer) = ring.split();

        // Background synthesis thread: continuously renders samples into ring buffer
        // This enables parallel synthesis using all CPU cores!
        let graph_clone_synth = Arc::clone(&graph);
        thread::spawn(move || {
            let mut buffer = [0.0f32; 512]; // Render in chunks of 512 samples
            let mut total_buffers = 0usize;
            let mut total_time = std::time::Duration::ZERO;
            let profile = std::env::var("PROFILE_LIVE").is_ok();

            if profile {
                eprintln!("🔧 Background synthesis thread started with profiling");
            }

            loop {
                // Check if we have space in ring buffer
                let space = ring_producer.vacant_len();

                if space >= buffer.len() {
                    // Render a chunk of audio
                    let graph_snapshot = graph_clone_synth.load();

                    if let Some(ref graph_cell) = **graph_snapshot {
                        // Synthesize samples using optimized buffer processing
                        let start = if profile { Some(std::time::Instant::now()) } else { None };
                        graph_cell.0.borrow_mut().process_buffer(&mut buffer);

                        if let Some(start) = start {
                            let elapsed = start.elapsed();
                            total_time += elapsed;
                            total_buffers += 1;

                            if total_buffers % 10 == 0 || elapsed.as_millis() > 15 {
                                let avg_ms = total_time.as_secs_f64() * 1000.0 / total_buffers as f64;
                                let target_ms = 512.0 / 44.1; // 11.61ms
                                let this_ms = elapsed.as_secs_f64() * 1000.0;
                                eprintln!("🎵 Live #{}: {:.2}ms (avg: {:.2}ms, target: {:.2}ms, {:.0}% CPU) {}",
                                    total_buffers, this_ms, avg_ms, target_ms, (avg_ms / target_ms) * 100.0,
                                    if this_ms > target_ms { "⚠️ SLOW" } else { "✅" });
                            }
                        }

                        // Write to ring buffer
                        let written = ring_producer.push_slice(&buffer);
                        if written < buffer.len() {
                            eprintln!("⚠️  Ring buffer full, dropped {} samples", buffer.len() - written);
                        }
                    } else {
                        // No graph yet, write silence
                        ring_producer.push_slice(&buffer);
                    }
                } else {
                    // Ring buffer is full, sleep briefly
                    thread::sleep(Duration::from_micros(100));
                }
            }
        });

        // Audio callback: just reads from ring buffer (FAST!)
        // No synthesis, no processing, just copy pre-rendered samples
        let err_fn = |err| {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/phonon_audio_errors.log")
            {
                let _ = writeln!(file, "[{}] Audio stream error: {}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    err);
            }
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Read from ring buffer - MUCH faster than synthesis!
                        let available = ring_consumer.occupied_len();

                        if available >= data.len() {
                            // Ring buffer has enough samples, read them
                            ring_consumer.pop_slice(data);
                        } else {
                            // Underrun: not enough samples in buffer
                            // Read what we have, fill rest with silence
                            let read = ring_consumer.pop_slice(data);
                            for sample in data[read..].iter_mut() {
                                *sample = 0.0;
                            }

                            // Warn about underrun
                            static mut UNDERRUN_COUNT: usize = 0;
                            unsafe {
                                UNDERRUN_COUNT += 1;
                                if UNDERRUN_COUNT % 100 == 0 {
                                    eprintln!("⚠️  Audio underrun #{} (synth can't keep up)", UNDERRUN_COUNT);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let available = ring_consumer.occupied_len();

                        if available >= data.len() {
                            // Read from ring buffer and convert to i16
                            let mut temp = vec![0.0f32; data.len()];
                            ring_consumer.pop_slice(&mut temp);
                            for (dst, src) in data.iter_mut().zip(temp.iter()) {
                                *dst = (*src * 32767.0) as i16;
                            }
                        } else {
                            // Underrun
                            let mut temp = vec![0.0f32; available];
                            ring_consumer.pop_slice(&mut temp);
                            for (i, dst) in data.iter_mut().enumerate() {
                                if i < temp.len() {
                                    *dst = (temp[i] * 32767.0) as i16;
                                } else {
                                    *dst = 0;
                                }
                            }

                            static mut UNDERRUN_COUNT: usize = 0;
                            unsafe {
                                UNDERRUN_COUNT += 1;
                                if UNDERRUN_COUNT % 100 == 0 {
                                    eprintln!("⚠️  Audio underrun #{}", UNDERRUN_COUNT);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
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

        // Compile into a graph
        // Note: compile_program sets CPS from tempo:/bpm: statements in the file
        // Default is 0.5 CPS if not specified
        let mut new_graph = compile_program(statements, self.sample_rate)
            .map_err(|e| format!("Compile error: {}", e))?;

        // CRITICAL: Enable wall-clock timing for live mode
        new_graph.enable_wall_clock_timing();

        // CRITICAL: Transfer session timing to preserve global clock
        // Wall-clock based timing ensures the beat NEVER drops during reload
        let current_graph = self.graph.load();
        if let Some(ref old_graph_cell) = **current_graph {
            new_graph.transfer_session_timing(&old_graph_cell.0.borrow());
            // Also transfer VoiceManager for click-free reloads
            new_graph.transfer_voice_manager(old_graph_cell.0.borrow_mut().take_voice_manager());
        }

        // Hot-swap the graph atomically using lock-free ArcSwap
        // Background synthesis thread will pick up new graph on next render
        self.graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));

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
    /// Audio is rendered by background thread and read by audio callback
    pub fn run(&mut self) -> Result<(), String> {
        // Initial load
        self.load_file()?;

        println!("\n🎵 Live coding session started!");
        println!("📝 Editing: {}", self.current_file.display());
        println!("🚀 Audio: ring buffer + background synthesis (parallel!)");
        println!("\nPress Ctrl+C to stop\n");

        // File watching loop (audio runs independently)
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
