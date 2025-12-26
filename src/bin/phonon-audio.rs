//! Phonon Audio Engine - Separate Process for Audio Synthesis
//!
//! This binary runs independently from the pattern engine (phonon edit).
//! It handles all audio synthesis and playback, receiving graph updates via IPC.
//!
//! Architecture:
//! - Pattern engine (phonon edit) compiles DSL → sends graph via Unix socket
//! - Audio engine (this) receives graph → synthesizes audio → outputs to speakers
//! - Separation ensures compilation NEVER blocks audio (< 30ms pattern swaps)
//!
//! Note: This binary is Unix-only (requires Unix domain sockets).

#[cfg(not(unix))]
fn main() {
    eprintln!("phonon-audio is only supported on Unix platforms (Linux, macOS)");
    std::process::exit(1);
}

#[cfg(unix)]
use arc_swap::ArcSwap;
#[cfg(unix)]
use clap::Parser;
#[cfg(unix)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(unix)]
use hound::{WavSpec, WavWriter};
#[cfg(unix)]
use phonon::compositional_compiler::compile_program;
#[cfg(unix)]
use phonon::compositional_parser::parse_program;
#[cfg(unix)]
use phonon::ipc::{AudioServer, IpcMessage};
#[cfg(unix)]
use phonon::unified_graph::UnifiedSignalGraph;
#[cfg(unix)]
use ringbuf::traits::{Consumer, Observer, Producer, Split};
#[cfg(unix)]
use ringbuf::HeapRb;
#[cfg(unix)]
use std::cell::RefCell;
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::BufWriter;
#[cfg(unix)]
use std::sync::{Arc, Mutex};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration as StdDuration;

/// Phonon Audio Engine - Real-time audio synthesis
#[cfg(unix)]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Record audio output to WAV file (for debugging)
    #[arg(short, long)]
    record: Option<String>,
}

// Audio buffer size in samples
// Can be overridden with PHONON_BUFFER_SIZE environment variable
// Smaller = lower latency but higher CPU usage
// Typical values: 64 (1.5ms), 128 (3ms), 256 (6ms), 512 (12ms)
#[cfg(unix)]
const DEFAULT_BUFFER_SIZE: usize = 128; // 3ms at 44.1kHz

/// Get audio buffer size from environment variable or use default
/// Returns value clamped to reasonable bounds (32-2048 samples)
#[cfg(unix)]
fn get_buffer_size() -> usize {
    std::env::var("PHONON_BUFFER_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BUFFER_SIZE)
        .clamp(32, 2048) // Reasonable bounds: 0.7ms - 46ms
}

/// Global Clock - THE SINGLE SOURCE OF TIMING TRUTH
///
/// This struct owns all timing state. Graphs do NOT own timing.
/// When rendering, timing is passed TO the graph as a parameter.
///
/// This eliminates race conditions between threads because:
/// - Only the synthesis thread reads the clock
/// - Only the IPC thread writes the clock (tempo changes)
/// - No state needs to be transferred between graphs
///
/// TEMPO CHANGE HANDLING:
/// When tempo changes, we don't want timing to jump. So we:
/// 1. Save current position as base_cycle_position
/// 2. Save current time as base_time
/// 3. Future positions = base_position + (now - base_time) * new_cps
#[cfg(unix)]
struct GlobalClock {
    /// Time at last tempo change (or session start)
    base_time: std::time::Instant,
    /// Cycle position at last tempo change (or 0 at start)
    base_cycle_position: f64,
    /// Current cycles per second (tempo)
    cps: f32,
    /// Sample rate for calculating per-sample increment
    sample_rate: f32,
}

#[cfg(unix)]
impl GlobalClock {
    fn new(sample_rate: f32) -> Self {
        Self {
            base_time: std::time::Instant::now(),
            base_cycle_position: 0.0,
            cps: 0.5, // Default tempo
            sample_rate,
        }
    }

    /// Get current cycle position from wall-clock
    /// Position = base_position + (now - base_time) * cps
    fn get_position(&self) -> f64 {
        let elapsed = self.base_time.elapsed().as_secs_f64();
        self.base_cycle_position + elapsed * self.cps as f64
    }

    /// Get cycle increment per sample
    fn get_sample_increment(&self) -> f64 {
        self.cps as f64 / self.sample_rate as f64
    }

    /// Set tempo - MAINTAINS TIMING CONTINUITY!
    /// Before changing cps, we save the current position as the new base.
    /// This ensures no timing jump when tempo changes.
    fn set_cps(&mut self, new_cps: f32) {
        if (self.cps - new_cps).abs() < 0.0001 {
            return; // No change needed
        }
        // Save current position as new base BEFORE changing tempo
        let current_pos = self.get_position();
        self.base_cycle_position = current_pos;
        self.base_time = std::time::Instant::now();
        self.cps = new_cps;
    }

    /// Get current CPS
    #[allow(dead_code)]
    fn get_cps(&self) -> f32 {
        self.cps
    }

    /// Get buffer timing info atomically (position AND increment together)
    /// Returns (buffer_start_cycle, sample_increment, cps)
    /// This ensures consistent values even if tempo is changing
    fn get_buffer_timing(&self) -> (f64, f64, f32) {
        let position = self.get_position();
        let increment = self.get_sample_increment();
        (position, increment, self.cps)
    }
}

// Newtype wrapper to impl Send+Sync for RefCell<UnifiedSignalGraph>
// SAFETY: Each GraphCell instance is only accessed by one thread at a time.
#[cfg(unix)]
struct GraphCell(RefCell<UnifiedSignalGraph>);
#[cfg(unix)]
unsafe impl Send for GraphCell {}
#[cfg(unix)]
unsafe impl Sync for GraphCell {}

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("🎵 Phonon Audio Engine starting...");

    if let Some(ref record_path) = args.record {
        eprintln!("🔴 Recording to: {}", record_path);
    }

    // Create Unix socket server
    let server = AudioServer::new()?;
    eprintln!("📡 Waiting for pattern engine to connect...");

    // Wait for pattern engine connection
    let mut stream = server.accept()?;

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

    // Get configurable buffer size
    let buffer_size = get_buffer_size();
    let latency_ms = buffer_size as f32 / sample_rate * 1000.0;

    let mut config: cpal::StreamConfig = default_config.into();
    // Set buffer size explicitly for low latency
    config.buffer_size = cpal::BufferSize::Fixed(buffer_size as u32);

    eprintln!("🎵 Audio: {} Hz, {} channels", sample_rate as u32, channels);
    eprintln!(
        "🔧 Buffer size: {} samples ({:.1}ms latency)",
        buffer_size, latency_ms
    );
    eprintln!("🔧 Using ring buffer architecture for parallel synthesis");

    // Create WAV writer for recording (if requested)
    let wav_writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>> =
        if let Some(ref record_path) = args.record {
            let spec = WavSpec {
                channels: channels as u16,
                sample_rate: sample_rate as u32,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let writer = WavWriter::create(record_path, spec)?;
            eprintln!("✅ WAV writer created for recording");
            Arc::new(Mutex::new(Some(writer)))
        } else {
            Arc::new(Mutex::new(None))
        };

    // GLOBAL CLOCK - THE SINGLE SOURCE OF TIMING TRUTH
    // This is the ONLY thing that tracks timing. Graphs don't own timing.
    // Synthesis thread reads from it, IPC thread can update tempo.
    let global_clock = Arc::new(Mutex::new(GlobalClock::new(sample_rate)));
    eprintln!("⏰ Global clock initialized (single source of timing truth)");

    // Graph for background synthesis thread (lock-free swap)
    let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));

    // Ring buffer: background synth writes, audio callback reads
    // Size: 2 seconds of audio = smooth playback even with synthesis spikes
    let ring_buffer_size = (sample_rate as usize) * 2;
    let ring = HeapRb::<f32>::new(ring_buffer_size);
    let (mut ring_producer, mut ring_consumer) = ring.split();

    // Background synthesis thread: continuously renders samples into ring buffer
    let graph_clone_synth = Arc::clone(&graph);
    let clock_clone_synth = Arc::clone(&global_clock);
    thread::spawn(move || {
        // Use configurable buffer size (can't use array with runtime size, use Vec)
        let mut buffer = vec![0.0f32; buffer_size];

        loop {
            // Check if we have space in ring buffer
            let space = ring_producer.vacant_len();

            if space >= buffer.len() {
                // CRITICAL: Get timing from GlobalClock ONCE per buffer
                // This is THE SINGLE SOURCE OF TRUTH for timing
                let (buffer_start_cycle, sample_increment, cps) = {
                    let clock = clock_clone_synth.lock().unwrap();
                    clock.get_buffer_timing()
                };

                // Render a chunk of audio
                let graph_snapshot = graph_clone_synth.load();

                if let Some(ref graph_cell) = **graph_snapshot {
                    // DEBUG: Log buffer timing (enable with DEBUG_BUFFER_TIMING=1)
                    static DEBUG_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                    let counter = DEBUG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if std::env::var("DEBUG_BUFFER_TIMING").is_ok() && counter % 100 == 0 {
                        eprintln!("📊 Buffer {}: cycle={:.4}, incr={:.8}, cps={:.2}",
                            counter, buffer_start_cycle, sample_increment, cps);
                    }

                    // Synthesize samples using optimized buffer processing
                    // CRITICAL: Pass timing FROM GlobalClock TO the graph
                    // The graph does NOT calculate timing - it receives it as a parameter
                    graph_cell.0.borrow_mut().process_buffer_at(
                        &mut buffer,
                        buffer_start_cycle,
                        sample_increment,
                        cps,
                    );

                    // Write to ring buffer
                    let written = ring_producer.push_slice(&buffer);
                    if written < buffer.len() {
                        eprintln!(
                            "⚠️  Ring buffer full, dropped {} samples",
                            buffer.len() - written
                        );
                    }
                } else {
                    // No graph (hushed/panic) - write silence
                    buffer.fill(0.0);
                    ring_producer.push_slice(&buffer);
                }
            } else {
                // Ring buffer is full, sleep briefly
                thread::sleep(StdDuration::from_micros(100));
            }
        }
    });

    // Audio callback error handler
    let err_fn = |err| {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/phonon_audio_errors.log")
        {
            let _ = writeln!(
                file,
                "[{}] Audio stream error: {}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                err
            );
        }
    };

    // Clone wav_writer for audio callbacks
    let wav_writer_f32 = Arc::clone(&wav_writer);
    let wav_writer_i16 = Arc::clone(&wav_writer);

    // Build audio stream based on sample format
    let stream_result = match sample_format {
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
                        let read = ring_consumer.pop_slice(data);
                        for sample in data[read..].iter_mut() {
                            *sample = 0.0;
                        }

                        static mut UNDERRUN_COUNT: usize = 0;
                        unsafe {
                            UNDERRUN_COUNT += 1;
                            if UNDERRUN_COUNT % 100 == 0 {
                                eprintln!("⚠️  Audio underrun #{}", UNDERRUN_COUNT);
                            }
                        }
                    }

                    // Write to WAV file if recording
                    if let Ok(mut writer_lock) = wav_writer_f32.lock() {
                        if let Some(ref mut writer) = *writer_lock {
                            for &sample in data.iter() {
                                let _ = writer.write_sample(sample);
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

                        // Write f32 samples to WAV file if recording
                        if let Ok(mut writer_lock) = wav_writer_i16.lock() {
                            if let Some(ref mut writer) = *writer_lock {
                                for &sample in temp.iter() {
                                    let _ = writer.write_sample(sample);
                                }
                            }
                        }

                        // Convert to i16 for audio output
                        for (dst, src) in data.iter_mut().zip(temp.iter()) {
                            *dst = (*src * 32767.0) as i16;
                        }
                    } else {
                        // Underrun
                        let mut temp = vec![0.0f32; available];
                        ring_consumer.pop_slice(&mut temp);

                        // Write f32 samples to WAV file if recording (with zero padding)
                        if let Ok(mut writer_lock) = wav_writer_i16.lock() {
                            if let Some(ref mut writer) = *writer_lock {
                                for &sample in temp.iter() {
                                    let _ = writer.write_sample(sample);
                                }
                                // Write zeros for underrun portion
                                for _ in temp.len()..data.len() {
                                    let _ = writer.write_sample(0.0f32);
                                }
                            }
                        }

                        // Convert to i16 for audio output
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
        _ => return Err("Unsupported sample format".into()),
    };

    let audio_stream = stream_result.map_err(|e| format!("Failed to build stream: {}", e))?;

    audio_stream
        .play()
        .map_err(|e| format!("Failed to play stream: {}", e))?;

    eprintln!("✅ Audio engine ready");

    // Send Ready message to pattern engine
    IpcMessage::Ready.send(&mut stream)?;

    // IPC message loop - receive graph updates from pattern engine
    // Use receive_coalesced to automatically drain stale UpdateGraph messages
    loop {
        match IpcMessage::receive_coalesced(&mut stream) {
            Ok(msg) => match msg {
                IpcMessage::UpdateGraph { code } => {
                    eprintln!("📦 Received code update ({} bytes)", code.len());

                    // Parse the DSL code
                    match parse_program(&code) {
                        Ok((rest, statements)) => {
                            if !rest.trim().is_empty() {
                                eprintln!("⚠️  Failed to parse entire code, remaining: {}", rest);
                                continue;
                            }

                            // Compile into a graph
                            match compile_program(statements, sample_rate, None) {
                                Ok(new_graph) => {
                                    // CRITICAL: Update GlobalClock's tempo if it changed
                                    // GlobalClock.set_cps() handles timing continuity automatically!
                                    // No need for cycle_offset calculation - GlobalClock tracks position.
                                    let (old_pos, new_pos, old_cps) = {
                                        let mut clock = global_clock.lock().unwrap();
                                        let old_cps = clock.get_cps();
                                        let old_pos = clock.get_position();
                                        clock.set_cps(new_graph.cps);
                                        let new_pos = clock.get_position();
                                        (old_pos, new_pos, old_cps)
                                    };

                                    // DEBUG: Log timing continuity during graph swap
                                    eprintln!("🔄 Graph swap: pos={:.4} -> {:.4} (delta={:.6}), cps={:.2} -> {:.2}",
                                        old_pos, new_pos, new_pos - old_pos, old_cps, new_graph.cps);

                                    // Swap in new graph (atomic, lock-free)
                                    // Graph does NOT own timing - it receives timing from GlobalClock
                                    graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));
                                }
                                Err(e) => {
                                    eprintln!("❌ Compile error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Parse error: {}", e);
                        }
                    }
                }

                IpcMessage::Hush => {
                    eprintln!("🔇 Hush - silencing all outputs");
                    graph.store(Arc::new(None));
                }

                IpcMessage::Panic => {
                    eprintln!("🛑 Panic - stopping all synthesis");
                    graph.store(Arc::new(None));
                }

                IpcMessage::SetTempo { cps } => {
                    eprintln!("⏱️  Setting tempo to {} CPS", cps);
                    // Update GlobalClock (THE SINGLE SOURCE OF TRUTH)
                    // GlobalClock.set_cps() handles timing continuity automatically
                    let mut clock = global_clock.lock().unwrap();
                    clock.set_cps(cps);
                }

                IpcMessage::Shutdown => {
                    eprintln!("👋 Shutdown requested");
                    break;
                }

                _ => {
                    eprintln!("⚠️  Unexpected message from pattern engine: {:?}", msg);
                }
            },
            Err(e) => {
                eprintln!("❌ IPC error: {}", e);
                break;
            }
        }
    }

    eprintln!("🛑 Audio engine shutting down");

    // Finalize WAV recording if active
    if let Ok(mut writer_lock) = wav_writer.lock() {
        if let Some(writer) = writer_lock.take() {
            match writer.finalize() {
                Ok(_) => eprintln!("✅ Recording finalized successfully"),
                Err(e) => eprintln!("⚠️  Error finalizing recording: {}", e),
            }
        }
    }

    Ok(())
}

#[cfg(all(unix, test))]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to prevent parallel test execution (env vars are global)
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_buffer_size_default() {
        let _lock = TEST_LOCK.lock().unwrap();
        std::env::remove_var("PHONON_BUFFER_SIZE");
        assert_eq!(get_buffer_size(), DEFAULT_BUFFER_SIZE);
    }

    #[test]
    fn test_buffer_size_from_env() {
        let _lock = TEST_LOCK.lock().unwrap();
        std::env::set_var("PHONON_BUFFER_SIZE", "64");
        assert_eq!(get_buffer_size(), 64);
        std::env::remove_var("PHONON_BUFFER_SIZE");
    }

    #[test]
    fn test_buffer_size_clamped_min() {
        let _lock = TEST_LOCK.lock().unwrap();
        std::env::set_var("PHONON_BUFFER_SIZE", "16"); // Too small
        assert_eq!(get_buffer_size(), 32); // Clamped to minimum
        std::env::remove_var("PHONON_BUFFER_SIZE");
    }

    #[test]
    fn test_buffer_size_clamped_max() {
        let _lock = TEST_LOCK.lock().unwrap();
        std::env::set_var("PHONON_BUFFER_SIZE", "4096"); // Too large
        assert_eq!(get_buffer_size(), 2048); // Clamped to maximum
        std::env::remove_var("PHONON_BUFFER_SIZE");
    }

    #[test]
    fn test_buffer_size_invalid_falls_back_to_default() {
        let _lock = TEST_LOCK.lock().unwrap();
        std::env::set_var("PHONON_BUFFER_SIZE", "not_a_number");
        assert_eq!(get_buffer_size(), DEFAULT_BUFFER_SIZE);
        std::env::remove_var("PHONON_BUFFER_SIZE");
    }

    #[test]
    fn test_buffer_size_negative_falls_back_to_default() {
        let _lock = TEST_LOCK.lock().unwrap();
        std::env::set_var("PHONON_BUFFER_SIZE", "-100");
        assert_eq!(get_buffer_size(), DEFAULT_BUFFER_SIZE);
        std::env::remove_var("PHONON_BUFFER_SIZE");
    }
}
