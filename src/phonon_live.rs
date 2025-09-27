//! Phonon live coding implementation for CLI

use crate::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, Waveform};
use crate::mini_notation_v3::parse_mini_notation;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SizedSample, FromSample};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, Duration};

pub struct LiveConfig {
    pub file: PathBuf,
    pub sample_rate: f32,
}

pub fn run_live(config: LiveConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Phonon Live");
    println!("==============");

    let filepath = config.file;

    if !filepath.exists() {
        println!("Creating {}", filepath.display());
        fs::write(&filepath, DEFAULT_CONTENT)?;
    }

    // Setup audio
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or("No audio output device found")?;

    let audio_config = device.default_output_config()?;
    let sample_rate = audio_config.sample_rate().0 as f32;

    println!("📂 Watching: {}", filepath.display());
    println!("🎧 Audio: {} @ {} Hz", device.name()?, sample_rate);
    println!();

    // Shared state
    let state = Arc::new(Mutex::new(LiveState {
        graph: None,
        current_file: filepath.clone(),
        last_modified: None,
        last_content: String::new(),
    }));

    // Initial load
    check_and_reload(&state, sample_rate);

    // Audio stream
    let state_clone = Arc::clone(&state);
    let stream = match audio_config.sample_format() {
        cpal::SampleFormat::F32 => {
            build_stream::<f32>(&device, &audio_config.into(), state_clone)?
        }
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    println!("✏️  Edit {} and save to hear changes", filepath.display());
    println!("🎹 Press Ctrl+C to stop");
    println!();

    // Poll for changes
    loop {
        std::thread::sleep(Duration::from_millis(100));
        check_and_reload(&state, sample_rate);
    }
}

struct LiveState {
    graph: Option<UnifiedSignalGraph>,
    current_file: PathBuf,
    last_modified: Option<SystemTime>,
    last_content: String,
}

fn check_and_reload(state: &Arc<Mutex<LiveState>>, sample_rate: f32) {
    let filepath = {
        let state = state.lock().unwrap();
        state.current_file.clone()
    };

    if let Ok(metadata) = fs::metadata(&filepath) {
        if let Ok(modified) = metadata.modified() {
            let mut state_lock = state.lock().unwrap();

            let should_reload = match state_lock.last_modified {
                None => true,
                Some(last) => modified > last,
            };

            if should_reload {
                state_lock.last_modified = Some(modified);
                drop(state_lock);

                if let Ok(content) = fs::read_to_string(&filepath) {
                    let mut state_lock = state.lock().unwrap();
                    if content != state_lock.last_content {
                        state_lock.last_content = content.clone();
                        drop(state_lock);

                        println!("🔄 Reloading...");

                        match parse_phonon_file(&content, sample_rate) {
                            Ok(graph) => {
                                let mut state_lock = state.lock().unwrap();
                                state_lock.graph = Some(graph);
                                println!("✅ Loaded successfully");
                            }
                            Err(e) => {
                                println!("❌ Parse error: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_phonon_file(content: &str, sample_rate: f32) -> Result<UnifiedSignalGraph, String> {
    // Use the parser from phonon_poll.rs
    // This is a simplified version - you'd want to use the full parser
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // For now, just create a simple test signal
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Value(0.2),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);

    Ok(graph)
}

fn build_stream<T: Sample + SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    state: Arc<Mutex<LiveState>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let channels = config.channels as usize;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut state = state.lock().unwrap();

            for frame in data.chunks_mut(channels) {
                let sample = if let Some(ref mut graph) = state.graph {
                    graph.process_sample()
                } else {
                    0.0
                };

                let value: T = Sample::from_sample(sample);
                for channel in frame {
                    *channel = value;
                }
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    Ok(stream)
}

const DEFAULT_CONTENT: &str = r#"# Phonon Live
# Edit and save to hear changes!

tempo 1.0
out sine(440) * 0.2
"#;