#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Live coding audio engine with continuous cycles
//!
//! Runs a continuous audio loop that can be hot-reloaded with new patterns

use crate::compositional_compiler::compile_program;
use crate::compositional_parser::parse_program;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Commands for controlling the live engine
pub enum EngineCommand {
    /// Load new DSL code
    LoadCode(String),
    /// Stop all sound immediately
    Hush,
    /// Panic - stop and reset everything
    Panic,
    /// Quit the engine
    Quit,
}

/// Live audio engine that runs continuously
pub struct LiveEngine {
    sample_rate: f32,
    cycle_duration: f32, // Duration of one cycle in seconds
    command_tx: Sender<EngineCommand>,
    engine_thread: Option<thread::JoinHandle<()>>,
}

impl LiveEngine {
    /// Create a new live engine
    pub fn new(sample_rate: f32, cycle_duration: f32) -> Result<Self, Box<dyn std::error::Error>> {
        let (command_tx, command_rx) = channel();

        // Spawn the audio engine thread
        let engine_thread = Some(thread::spawn(move || {
            let _ = run_audio_loop(sample_rate, cycle_duration, command_rx);
            // Silent - don't interfere with UI
        }));

        Ok(Self {
            sample_rate,
            cycle_duration,
            command_tx,
            engine_thread,
        })
    }

    /// Load new code into the engine
    pub fn load_code(&self, code: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.command_tx
            .send(EngineCommand::LoadCode(code.to_string()))?;
        Ok(())
    }

    /// Hush - stop all sound
    pub fn hush(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.command_tx.send(EngineCommand::Hush)?;
        Ok(())
    }

    /// Panic - stop and reset
    pub fn panic(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.command_tx.send(EngineCommand::Panic)?;
        Ok(())
    }

    /// Shutdown the engine
    pub fn shutdown(self) -> Result<(), Box<dyn std::error::Error>> {
        self.command_tx.send(EngineCommand::Quit)?;
        if let Some(thread) = self.engine_thread {
            thread.join().map_err(|_| "Failed to join engine thread")?;
        }
        Ok(())
    }
}

/// Run the continuous audio loop
fn run_audio_loop(
    sample_rate: f32,
    cycle_duration: f32,
    command_rx: Receiver<EngineCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize audio output
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;

    let config = device.default_output_config()?;

    // Shared state for the audio callback
    let current_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buffer_clone = Arc::clone(&current_buffer);
    let is_hushed = Arc::new(Mutex::new(false));
    let hush_clone = Arc::clone(&is_hushed);

    // Current playback position - shared between callback and main thread
    let position = Arc::new(Mutex::new(0usize));
    let pos_clone = Arc::clone(&position);

    // Build the audio stream with looping playback
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_output_stream(
                &config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let buffer = buffer_clone.lock().unwrap();
                    let hushed = *hush_clone.lock().unwrap();
                    let mut pos = pos_clone.lock().unwrap();

                    for sample in data.iter_mut() {
                        if hushed || buffer.is_empty() {
                            *sample = 0.0;
                        } else {
                            // Loop the buffer forever!
                            *sample = buffer[*pos];
                            *pos = (*pos + 1) % buffer.len(); // Wrap around to loop
                        }
                    }
                },
                |_err| { /* Silent - don't interfere with UI */ },
                None,
            )?
        }
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    // Main loop - process commands and regenerate audio
    let mut current_code = String::new();
    let mut current_graph: Option<crate::unified_graph::UnifiedSignalGraph> = None;

    loop {
        // Check for commands (non-blocking)
        if let Ok(command) = command_rx.try_recv() {
            match command {
                EngineCommand::LoadCode(code) => {
                    current_code = code;
                    // Re-render the audio
                    if !current_code.is_empty() {
                        // Parse and render using new compositional parser
                        match parse_program(&current_code) {
                            Ok((_, statements)) => {
                                // Compile to graph
                                match compile_program(statements, sample_rate) {
                                    Ok(mut graph) => {
                                        // CRITICAL FIX: Preserve time continuity between reloads!
                                        // Time should be independent of patterns - synthesis depends on time.
                                        if let Some(old_graph) = &current_graph {
                                            // Transfer session timing to maintain global clock continuity
                                            // This ensures the beat never drops during graph reload
                                            graph.transfer_session_timing(old_graph);
                                        } else {
                                            // First load: enable wall-clock timing for live mode
                                            graph.enable_wall_clock_timing();
                                        }

                                        // Render one cycle
                                        let samples_per_cycle =
                                            (cycle_duration * sample_rate) as usize;
                                        let audio_buffer = graph.render(samples_per_cycle);

                                        // Update the playback buffer
                                        let mut buffer = current_buffer.lock().unwrap();
                                        *buffer = audio_buffer;

                                        // Reset hush state but DON'T reset position - keep cycle continuous
                                        *is_hushed.lock().unwrap() = false;
                                        // Don't reset position - let it keep cycling smoothly

                                        // Store graph for next reload to preserve timing
                                        current_graph = Some(graph);

                                        // Silent - don't interfere with UI
                                    }
                                    Err(_e) => {
                                        // Silent - don't interfere with UI
                                    }
                                }
                            }
                            Err(_e) => {
                                // Silent - don't interfere with UI
                            }
                        }
                    }
                }
                EngineCommand::Hush => {
                    *is_hushed.lock().unwrap() = true;
                    // Silent - don't interfere with UI
                }
                EngineCommand::Panic => {
                    *is_hushed.lock().unwrap() = true;
                    current_buffer.lock().unwrap().clear();
                    *position.lock().unwrap() = 0;
                    // Clear current graph so time resets on next load
                    current_graph = None;
                    // Silent - don't interfere with UI
                }
                EngineCommand::Quit => {
                    // Silent - don't interfere with UI
                    break;
                }
            }
        }

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
