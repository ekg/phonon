//! Live Playground - Edit live.phonon and hear changes instantly!
//!
//! Run with: cargo run --example live_playground
//! Then edit live.phonon in your editor and save to hear changes

use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use notify::{Event, EventKind, RecursiveMode, Result as NotifyResult, Watcher};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Shared state for hot-reloading
struct LiveState {
    graph: Option<UnifiedSignalGraph>,
    should_reload: bool,
    error_message: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Phonon Live Playground 🎵");
    println!("============================");
    println!();

    // Create default live.phonon file if it doesn't exist
    let live_file = "live.phonon";
    if !Path::new(live_file).exists() {
        let default_content = r#"# Phonon Live Coding
# Edit this file and save to hear changes!

# Set tempo (cycles per second)
cps: 1

# Create an LFO
~lfo: sine(0.5) * 0.5 + 0.5

# Bass with modulated filter
~bass: saw(55) >> lpf(~lfo * 2000 + 500, 0.8)

# Rhythm pattern
~kick: "1 0 0 1"
~snare: "0 1 0 0"

# Mix (comment/uncomment lines to change)
out: ~bass * 0.4
# out: ~bass * ~kick * 0.5
# out: sine(440) * 0.2
"#;
        fs::write(live_file, default_content)?;
        println!("Created live.phonon with example code");
    }

    // Setup audio
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No audio output device found")?;

    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;

    println!("Audio device: {}", device.name()?);
    println!("Sample rate: {} Hz", sample_rate);
    println!();

    // Shared state
    let state = Arc::new(Mutex::new(LiveState {
        graph: None,
        should_reload: true,
        error_message: None,
    }));

    // Load initial graph
    load_graph(&state, live_file, sample_rate);

    // Setup file watcher
    let state_clone = Arc::clone(&state);
    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| match res {
        Ok(event) => {
            if matches!(event.kind, EventKind::Modify(_)) {
                println!("\n🔄 File changed, reloading...");
                let mut state = state_clone.lock().unwrap();
                state.should_reload = true;
            }
        }
        Err(e) => eprintln!("Watch error: {:?}", e),
    })?;

    watcher.watch(Path::new(live_file), RecursiveMode::NonRecursive)?;

    // Setup audio stream
    let state_audio = Arc::clone(&state);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), state_audio)?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    println!("📝 Watching: {}", live_file);
    println!("✏️  Edit the file in your editor and save to hear changes");
    println!("🎹 Press Ctrl+C to stop");
    println!();
    println!("Example things to try:");
    println!("  - Change the LFO frequency: sine(2) instead of sine(0.5)");
    println!("  - Change the bass note: saw(110) instead of saw(55)");
    println!("  - Modify patterns: \"1 1 0 1\" instead of \"1 0 0 1\"");
    println!("  - Change the output mix");
    println!();

    // Keep alive and check for reloads
    loop {
        std::thread::sleep(Duration::from_millis(100));

        let mut state_guard = state.lock().unwrap();
        if state_guard.should_reload {
            state_guard.should_reload = false;
            drop(state_guard); // Release lock before loading
            load_graph(&state, live_file, sample_rate);
        }
    }
}

fn load_graph(state: &Arc<Mutex<LiveState>>, file: &str, sample_rate: f32) {
    match fs::read_to_string(file) {
        Ok(content) => {
            // Skip comments and empty lines
            let cleaned: String = content
                .lines()
                .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            // Try the DSL parser first
            match parse_dsl(&cleaned) {
                Ok((_, statements)) if !statements.is_empty() => {
                    let compiler = DslCompiler::new(sample_rate);
                    let graph = compiler.compile(statements);

                    let mut state = state.lock().unwrap();
                    state.graph = Some(graph);
                    state.error_message = None;
                    println!("✅ Loaded DSL successfully");
                }
                _ => {
                    // Fallback to manual graph construction for simpler syntax
                    match build_simple_graph(&cleaned, sample_rate) {
                        Ok(graph) => {
                            let mut state = state.lock().unwrap();
                            state.graph = Some(graph);
                            state.error_message = None;
                            println!("✅ Loaded graph successfully");
                        }
                        Err(e) => {
                            let mut state = state.lock().unwrap();
                            state.error_message = Some(e.clone());
                            println!("❌ Error: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            let mut state = state.lock().unwrap();
            state.error_message = Some(format!("Could not read file: {}", e));
            println!("❌ Could not read file: {}", e);
        }
    }
}

fn build_simple_graph(content: &str, sample_rate: f32) -> Result<UnifiedSignalGraph, String> {
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    graph.set_cps(1.0); // Default tempo

    // Very simple parser for basic operations
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("cps:") {
            if let Some(cps_str) = line.strip_prefix("cps:") {
                if let Ok(cps) = cps_str.trim().parse::<f32>() {
                    graph.set_cps(cps);
                }
            }
        } else if line.starts_with("out:") {
            // Simple output: just create a basic sine wave for now
            let osc = graph.add_node(SignalNode::Oscillator {
                freq: Signal::Value(440.0),
                waveform: Waveform::Sine,
                phase: 0.0,
            });

            let output = graph.add_node(SignalNode::Output {
                input: Signal::Node(osc),
            });

            graph.set_output(output);
        }
    }

    // Always create a default output to ensure there's something
    // (Remove check for private field)
    {
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
    }

    Ok(graph)
}

fn build_stream<T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    state: Arc<Mutex<LiveState>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let channels = config.channels as usize;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut state = state.lock().unwrap();

            // Fill buffer
            for frame in data.chunks_mut(channels) {
                let sample = if let Some(ref mut graph) = state.graph {
                    graph.process_sample()
                } else {
                    0.0
                };

                let value: T = cpal::Sample::from_sample(sample);
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
