//! Phonon Live - Watch and play .phonon files
//!
//! Run with: cargo run --example phonon_live [filename.phonon]
//! Defaults to live.phonon if no file specified

use phonon::unified_graph::{UnifiedSignalGraph, SignalNode, Signal, SignalExpr, Waveform};
use phonon::mini_notation_v3::parse_mini_notation;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SizedSample, FromSample};
use notify::{Watcher, RecursiveMode, Result as NotifyResult, Event, EventKind};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;

struct LiveState {
    graph: Option<UnifiedSignalGraph>,
    should_reload: bool,
    current_file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Phonon Live 🎵");
    println!("=================");

    // Get filename from args or use default
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).map(|s| s.as_str()).unwrap_or("live.phonon");
    let filepath = PathBuf::from(filename);

    // Create default files if they don't exist
    ensure_example_files()?;

    if !filepath.exists() {
        println!("Creating {}", filename);
        fs::write(&filepath, DEFAULT_CONTENT)?;
    }

    // Setup audio
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or("No audio output device found")?;

    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;

    println!("📂 Watching: {}", filepath.display());
    println!("🎧 Audio: {} @ {} Hz", device.name()?, sample_rate);
    println!();

    // Shared state
    let state = Arc::new(Mutex::new(LiveState {
        graph: None,
        should_reload: true,
        current_file: filepath.clone(),
    }));

    // Initial load
    load_file(&state, sample_rate);

    // File watcher
    let state_clone = Arc::clone(&state);
    let watched_path = filepath.clone();
    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| {
        match res {
            Ok(event) => {
                println!("📁 File event: {:?}", event.kind);

                // Check for any write/modify events
                let should_reload = matches!(
                    event.kind,
                    EventKind::Modify(_) |
                    EventKind::Create(_) |
                    EventKind::Remove(_) |
                    EventKind::Any
                );

                if should_reload {
                    // Check if it's our file
                    if event.paths.is_empty() || event.paths.iter().any(|p| {
                        p == &watched_path || p.file_name() == watched_path.file_name()
                    }) {
                        println!("🔄 Triggering reload for: {:?}", event.paths);
                        let mut state = state_clone.lock().unwrap();
                        state.should_reload = true;
                    }
                }
            }
            Err(e) => eprintln!("Watch error: {:?}", e),
        }
    })?;

    // Watch the current directory instead of just the file
    let watch_dir = filepath.parent().unwrap_or(Path::new("."));
    println!("👁️  Watching directory: {:?}", watch_dir);
    watcher.watch(watch_dir, RecursiveMode::NonRecursive)?;

    // Audio stream
    let state_clone = Arc::clone(&state);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            build_stream::<f32>(&device, &config.into(), state_clone)?
        }
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    println!("✏️  Edit {} and save to hear changes", filename);
    println!("🎹 Press Ctrl+C to stop");
    println!();
    println!("Try these example files:");
    println!("  cargo run --example phonon_live bass.phonon");
    println!("  cargo run --example phonon_live drums.phonon");
    println!("  cargo run --example phonon_live ambient.phonon");
    println!();

    // Main loop
    let state_for_reload = Arc::clone(&state);
    let mut tick = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        tick += 1;

        // Show we're alive every 2 seconds
        if tick % 40 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }

        let mut state = state_for_reload.lock().unwrap();
        if state.should_reload {
            state.should_reload = false;
            let file = state.current_file.clone();
            drop(state);

            println!("\n🔄 Reloading {}...", file.display());
            load_file(&state_for_reload, sample_rate);
        }
    }
}

fn load_file(state: &Arc<Mutex<LiveState>>, sample_rate: f32) {
    let mut state_lock = state.lock().unwrap();
    let filepath = state_lock.current_file.clone();
    drop(state_lock);

    match fs::read_to_string(&filepath) {
        Ok(content) => {
            match parse_phonon_file(&content, sample_rate) {
                Ok(graph) => {
                    let mut state = state.lock().unwrap();
                    state.graph = Some(graph);
                    println!("✅ Loaded successfully");
                }
                Err(e) => {
                    println!("❌ Parse error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Could not read file: {}", e);
        }
    }
}

/// Parse a simplified .phonon file format
fn parse_phonon_file(content: &str, sample_rate: f32) -> Result<UnifiedSignalGraph, String> {
    let mut graph = UnifiedSignalGraph::new(sample_rate);
    let mut buses = std::collections::HashMap::new();

    // Default tempo
    graph.set_cps(1.0);

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        // Parse statements
        if line.starts_with("cps ") || line.starts_with("tempo ") {
            // Tempo setting: cps 2.0
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(cps) = parts[1].parse::<f32>() {
                    graph.set_cps(cps);
                }
            }
        } else if line.starts_with("out ") {
            // Output: out sine(440) * 0.2
            let expr = line.strip_prefix("out ").unwrap_or("").trim();
            if let Some(node_id) = parse_expression(&mut graph, expr, &buses) {
                let output = graph.add_node(SignalNode::Output {
                    input: Signal::Node(node_id),
                });
                graph.set_output(output);
            }
        } else if line.contains('=') {
            // Assignment: lfo = sine(0.5)
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim();
                let expr = parts[1].trim();

                if let Some(node_id) = parse_expression(&mut graph, expr, &buses) {
                    buses.insert(name.to_string(), node_id);
                    graph.add_bus(name.to_string(), node_id);
                }
            }
        }
    }

    // If no output defined, create default
    if !graph.has_output() {
        let osc = graph.add_node(SignalNode::Oscillator {
            freq: Signal::Value(440.0),
            waveform: Waveform::Sine,
            phase: 0.0,
        });

        let scaled = graph.add_node(SignalNode::Multiply {
            a: Signal::Node(osc),
            b: Signal::Value(0.1),
        });

        let output = graph.add_node(SignalNode::Output {
            input: Signal::Node(scaled),
        });

        graph.set_output(output);
    }

    Ok(graph)
}

/// Parse simple expressions
fn parse_expression(
    graph: &mut UnifiedSignalGraph,
    expr: &str,
    buses: &std::collections::HashMap<String, phonon::unified_graph::NodeId>
) -> Option<phonon::unified_graph::NodeId> {
    let expr = expr.trim();

    // Pattern in quotes: "bd ~ sn ~"
    if expr.starts_with('"') && expr.ends_with('"') {
        let pattern_str = &expr[1..expr.len()-1];
        let pattern = parse_mini_notation(pattern_str);
        return Some(graph.add_node(SignalNode::Pattern {
            pattern_str: pattern_str.to_string(),
            pattern,
            last_value: 0.0,
        }));
    }

    // Number
    if let Ok(value) = expr.parse::<f32>() {
        return Some(graph.add_node(SignalNode::Constant { value }));
    }

    // Oscillator: sine(440), saw(110)
    if expr.starts_with("sine(") || expr.starts_with("sin(") {
        if let Some(freq_str) = expr.strip_prefix("sine(").or(expr.strip_prefix("sin("))
            .and_then(|s| s.strip_suffix(")")) {
            let freq = freq_str.parse::<f32>().unwrap_or(440.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: Signal::Value(freq),
                waveform: Waveform::Sine,
                phase: 0.0,
            }));
        }
    }

    if expr.starts_with("saw(") {
        if let Some(freq_str) = expr.strip_prefix("saw(").and_then(|s| s.strip_suffix(")")) {
            let freq = freq_str.parse::<f32>().unwrap_or(110.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: Signal::Value(freq),
                waveform: Waveform::Saw,
                phase: 0.0,
            }));
        }
    }

    if expr.starts_with("square(") || expr.starts_with("sq(") {
        if let Some(freq_str) = expr.strip_prefix("square(").or(expr.strip_prefix("sq("))
            .and_then(|s| s.strip_suffix(")")) {
            let freq = freq_str.parse::<f32>().unwrap_or(220.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: Signal::Value(freq),
                waveform: Waveform::Square,
                phase: 0.0,
            }));
        }
    }

    if expr.starts_with("tri(") {
        if let Some(freq_str) = expr.strip_prefix("tri(").and_then(|s| s.strip_suffix(")")) {
            let freq = freq_str.parse::<f32>().unwrap_or(330.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: Signal::Value(freq),
                waveform: Waveform::Triangle,
                phase: 0.0,
            }));
        }
    }

    // Noise
    if expr == "noise" {
        return Some(graph.add_node(SignalNode::Noise { seed: 12345 }));
    }

    // Filters: lpf(input, cutoff, q)
    if expr.contains(" >> lpf(") {
        let parts: Vec<&str> = expr.splitn(2, " >> lpf(").collect();
        if parts.len() == 2 && parts[1].ends_with(")") {
            let input_expr = parts[0];
            let params = &parts[1][..parts[1].len()-1];
            let param_parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

            if let Some(input) = parse_expression(graph, input_expr, buses) {
                let cutoff = param_parts.get(0)
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1000.0);
                let q = param_parts.get(1)
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.0);

                return Some(graph.add_node(SignalNode::LowPass {
                    input: Signal::Node(input),
                    cutoff: Signal::Value(cutoff),
                    q: Signal::Value(q),
                    state: Default::default(),
                }));
            }
        }
    }

    // HPF
    if expr.contains(" >> hpf(") {
        let parts: Vec<&str> = expr.splitn(2, " >> hpf(").collect();
        if parts.len() == 2 && parts[1].ends_with(")") {
            let input_expr = parts[0];
            let params = &parts[1][..parts[1].len()-1];
            let param_parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

            if let Some(input) = parse_expression(graph, input_expr, buses) {
                let cutoff = param_parts.get(0)
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1000.0);
                let q = param_parts.get(1)
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.0);

                return Some(graph.add_node(SignalNode::HighPass {
                    input: Signal::Node(input),
                    cutoff: Signal::Value(cutoff),
                    q: Signal::Value(q),
                    state: Default::default(),
                }));
            }
        }
    }

    // Binary operations: a * b, a + b
    if expr.contains(" * ") {
        let parts: Vec<&str> = expr.splitn(2, " * ").collect();
        if parts.len() == 2 {
            if let (Some(left), Some(right)) = (
                parse_expression(graph, parts[0], buses),
                parse_expression(graph, parts[1], buses)
            ) {
                return Some(graph.add_node(SignalNode::Multiply {
                    a: Signal::Node(left),
                    b: Signal::Node(right),
                }));
            }
        }
    }

    if expr.contains(" + ") {
        let parts: Vec<&str> = expr.splitn(2, " + ").collect();
        if parts.len() == 2 {
            if let (Some(left), Some(right)) = (
                parse_expression(graph, parts[0], buses),
                parse_expression(graph, parts[1], buses)
            ) {
                return Some(graph.add_node(SignalNode::Add {
                    a: Signal::Node(left),
                    b: Signal::Node(right),
                }));
            }
        }
    }

    // Bus reference
    if let Some(&node_id) = buses.get(expr) {
        return Some(node_id);
    }

    None
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

# Simple sine wave
out sine(440) * 0.2
"#;

fn ensure_example_files() -> std::io::Result<()> {
    // Create example files if they don't exist

    if !Path::new("bass.phonon").exists() {
        fs::write("bass.phonon", r#"# Bass patch with filter sweep
tempo 2.0

lfo = sine(0.25) * 0.5 + 0.5
bass = saw(55)
# TODO: Implement filter syntax
# filtered = bass >> lpf(lfo * 2000 + 500)

out bass * 0.3
"#)?;
    }

    if !Path::new("drums.phonon").exists() {
        fs::write("drums.phonon", r#"# Drum patterns
tempo 2.0

kick = "1 0 0 1"
snare = "0 1 0 1"
# TODO: Implement drum synthesis
# kick_sound = sine(60) * kick
# snare_sound = noise * snare

out kick * 0.5
"#)?;
    }

    if !Path::new("ambient.phonon").exists() {
        fs::write("ambient.phonon", r#"# Ambient pad
tempo 0.5

# Detuned oscillators
osc1 = sine(220)
osc2 = sine(220.5)
osc3 = sine(330) * 0.3
mixed = osc1 + osc2 + osc3

out mixed * 0.1
"#)?;
    }

    Ok(())
}