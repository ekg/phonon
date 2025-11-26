//! Phonon Audio Engine - Separate Process for Audio Synthesis
//!
//! This binary runs independently from the pattern engine (phonon edit).
//! It handles all audio synthesis and playback, receiving graph updates via IPC.
//!
//! Architecture:
//! - Pattern engine (phonon edit) compiles DSL → sends graph via Unix socket
//! - Audio engine (this) receives graph → synthesizes audio → outputs to speakers
//! - Separation ensures compilation NEVER blocks audio (< 30ms pattern swaps)

use arc_swap::ArcSwap;
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::ipc::{AudioServer, IpcMessage};
use phonon::unified_graph::UnifiedSignalGraph;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;

/// Phonon Audio Engine - Real-time audio synthesis
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
const DEFAULT_BUFFER_SIZE: usize = 128; // 3ms at 44.1kHz

/// Get audio buffer size from environment variable or use default
/// Returns value clamped to reasonable bounds (32-2048 samples)
fn get_buffer_size() -> usize {
    std::env::var("PHONON_BUFFER_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BUFFER_SIZE)
        .clamp(32, 2048) // Reasonable bounds: 0.7ms - 46ms
}

// Newtype wrapper to impl Send+Sync for RefCell<UnifiedSignalGraph>
// SAFETY: Each GraphCell instance is only accessed by one thread at a time.
struct GraphCell(RefCell<UnifiedSignalGraph>);
unsafe impl Send for GraphCell {}
unsafe impl Sync for GraphCell {}

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
    eprintln!("🔧 Buffer size: {} samples ({:.1}ms latency)", buffer_size, latency_ms);
    eprintln!("🔧 Using ring buffer architecture for parallel synthesis");

    // Create WAV writer for recording (if requested)
    let wav_writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>> = if let Some(ref record_path) = args.record {
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

    // Shared session start time - ALL graphs use this same timing reference
    // This ensures timing continuity when swapping graphs during live coding
    let session_start_time = std::time::Instant::now();
    eprintln!("⏰ Session timing initialized");

    // Graph for background synthesis thread (lock-free swap)
    let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));

    // Ring buffer: background synth writes, audio callback reads
    // Size: 2 seconds of audio = smooth playback even with synthesis spikes
    let ring_buffer_size = (sample_rate as usize) * 2;
    let ring = HeapRb::<f32>::new(ring_buffer_size);
    let (mut ring_producer, mut ring_consumer) = ring.split();

    // Background synthesis thread: continuously renders samples into ring buffer
    let graph_clone_synth = Arc::clone(&graph);
    thread::spawn(move || {
        // Use configurable buffer size (can't use array with runtime size, use Vec)
        let mut buffer = vec![0.0f32; buffer_size];

        loop {
            // Check if we have space in ring buffer
            let space = ring_producer.vacant_len();

            if space >= buffer.len() {
                // Render a chunk of audio
                let graph_snapshot = graph_clone_synth.load();

                if let Some(ref graph_cell) = **graph_snapshot {
                    // Synthesize samples using optimized buffer processing
                    graph_cell.0.borrow_mut().process_buffer(&mut buffer);

                    // Write to ring buffer
                    let written = ring_producer.push_slice(&buffer);
                    if written < buffer.len() {
                        eprintln!("⚠️  Ring buffer full, dropped {} samples", buffer.len() - written);
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

    let audio_stream = stream_result
        .map_err(|e| format!("Failed to build stream: {}", e))?;

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
                                Ok(mut new_graph) => {
                                    // Set the shared session start time on the new graph
                                    // This ensures timing continuity - ALL graphs use the same wall-clock reference!
                                    new_graph.session_start_time = session_start_time;
                                    new_graph.use_wall_clock = true;
                                    new_graph.cycle_offset = 0.0; // No offset needed - all graphs share session_start_time

                                    // NOTE: Node timing initialization removed!
                                    // Graph now initializes nodes on first buffer processing
                                    // This ensures timing is based on ACTUAL buffer start time,
                                    // not IPC message arrival time (which can differ!)

                                    eprintln!("🔄 Graph updated (nodes will initialize on first buffer)");

                                    // Swap in new graph (atomic, lock-free)
                                    // Timing is continuous because all graphs share the same session_start_time!
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
                    let current_graph = graph.load();
                    if let Some(ref graph_cell) = **current_graph {
                        if let Ok(mut g) = graph_cell.0.try_borrow_mut() {
                            g.set_cps(cps);
                        }
                    }
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

#[cfg(test)]
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
