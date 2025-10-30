//! Phonon Live with Polling - More reliable file watching
//!
//! Run with: cargo run --example phonon_poll [filename.phonon]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

struct LiveState {
    graph: Option<UnifiedSignalGraph>,
    current_file: PathBuf,
    last_modified: Option<SystemTime>,
    last_content: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Phonon Live (Polling) 🎵");
    println!("===========================");

    // Get filename from args or use default
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).map(|s| s.as_str()).unwrap_or("live.phonon");
    let filepath = PathBuf::from(filename);

    if !filepath.exists() {
        println!("Creating {}", filename);
        fs::write(&filepath, DEFAULT_CONTENT)?;
    }

    // Setup audio
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No audio output device found")?;

    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;

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
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), state_clone)?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    println!("✏️  Edit {} and save to hear changes", filename);
    println!("🎹 Press Ctrl+C to stop");
    println!();

    // Poll for changes
    let mut tick = 0;
    loop {
        std::thread::sleep(Duration::from_millis(100)); // Poll every 100ms
        tick += 1;

        // Check for file changes
        check_and_reload(&state, sample_rate);

        // Show we're alive every 5 seconds
        if tick % 50 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }
}

fn check_and_reload(state: &Arc<Mutex<LiveState>>, sample_rate: f32) {
    let filepath = {
        let state = state.lock().unwrap();
        state.current_file.clone()
    };

    // Check if file has changed
    if let Ok(metadata) = fs::metadata(&filepath) {
        if let Ok(modified) = metadata.modified() {
            let mut state_lock = state.lock().unwrap();

            // Check modification time
            let should_reload = match state_lock.last_modified {
                None => true,
                Some(last) => modified > last,
            };

            if should_reload {
                state_lock.last_modified = Some(modified);
                drop(state_lock);

                // Also check content to avoid false positives
                if let Ok(content) = fs::read_to_string(&filepath) {
                    let mut state_lock = state.lock().unwrap();
                    if content != state_lock.last_content {
                        state_lock.last_content = content.clone();
                        drop(state_lock);

                        println!("\n🔄 File changed, reloading...");

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
        pending_freq: None,
        last_sample: 0.0,
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

/// Parse a parameter that can be a number, pattern, or bus reference
fn parse_parameter(
    graph: &mut UnifiedSignalGraph,
    param_str: &str,
    buses: &std::collections::HashMap<String, phonon::unified_graph::NodeId>,
    default_value: f32,
) -> Signal {
    let param_str = param_str.trim();

    // Check if it's a pattern in quotes
    if param_str.starts_with('"') && param_str.ends_with('"') {
        let pattern_str = &param_str[1..param_str.len() - 1];

        let pattern = parse_mini_notation(pattern_str);
        let pattern_node = graph.add_node(SignalNode::Pattern {
            pattern_str: pattern_str.to_string(),
            pattern,
            last_value: default_value,
            last_trigger_time: -1.0,
        });
        Signal::Node(pattern_node)
    }
    // Check if it's a bus reference
    else if let Some(&node_id) = buses.get(param_str) {
        Signal::Node(node_id)
    }
    // Otherwise parse as number
    else if let Ok(value) = param_str.parse::<f32>() {
        Signal::Value(value)
    } else {
        Signal::Value(default_value)
    }
}

/// Parse simple expressions
fn parse_expression(
    graph: &mut UnifiedSignalGraph,
    expr: &str,
    buses: &std::collections::HashMap<String, phonon::unified_graph::NodeId>,
) -> Option<phonon::unified_graph::NodeId> {
    let expr = expr.trim();

    // Pattern in quotes: "bd ~ sn ~"
    if expr.starts_with('"') && expr.ends_with('"') {
        let pattern_str = &expr[1..expr.len() - 1];
        let pattern = parse_mini_notation(pattern_str);
        return Some(graph.add_node(SignalNode::Pattern {
            pattern_str: pattern_str.to_string(),
            pattern,
            last_value: 0.0,
            last_trigger_time: -1.0,
        }));
    }

    // Number
    if let Ok(value) = expr.parse::<f32>() {
        return Some(graph.add_node(SignalNode::Constant { value }));
    }

    // Oscillator: sine(440), sine("100 200 300"), sine(lfo)
    if expr.starts_with("sine(") || expr.starts_with("sin(") {
        if let Some(freq_str) = expr
            .strip_prefix("sine(")
            .or(expr.strip_prefix("sin("))
            .and_then(|s| s.strip_suffix(")"))
        {
            let freq_signal = parse_parameter(graph, freq_str, buses, 440.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: freq_signal,
                waveform: Waveform::Sine,
                phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
            }));
        }
    }

    if expr.starts_with("saw(") {
        if let Some(freq_str) = expr.strip_prefix("saw(").and_then(|s| s.strip_suffix(")")) {
            let freq_signal = parse_parameter(graph, freq_str, buses, 110.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: freq_signal,
                waveform: Waveform::Saw,
                phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
            }));
        }
    }

    if expr.starts_with("square(") || expr.starts_with("sq(") {
        if let Some(freq_str) = expr
            .strip_prefix("square(")
            .or(expr.strip_prefix("sq("))
            .and_then(|s| s.strip_suffix(")"))
        {
            let freq_signal = parse_parameter(graph, freq_str, buses, 220.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: freq_signal,
                waveform: Waveform::Square,
                phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
            }));
        }
    }

    if expr.starts_with("tri(") {
        if let Some(freq_str) = expr.strip_prefix("tri(").and_then(|s| s.strip_suffix(")")) {
            let freq_signal = parse_parameter(graph, freq_str, buses, 330.0);
            return Some(graph.add_node(SignalNode::Oscillator {
                freq: freq_signal,
                waveform: Waveform::Triangle,
                phase: 0.0,
        pending_freq: None,
        last_sample: 0.0,
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
            let params = &parts[1][..parts[1].len() - 1];
            let param_parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

            if let Some(input) = parse_expression(graph, input_expr, buses) {
                // Parse cutoff - can be a number, pattern, or reference
                let cutoff_signal = if let Some(cutoff_str) = param_parts.get(0) {
                    parse_parameter(graph, cutoff_str, buses, 1000.0)
                } else {
                    Signal::Value(1000.0)
                };

                // Q can also be a pattern, reference, or number
                let q_signal = if let Some(q_str) = param_parts.get(1) {
                    parse_parameter(graph, q_str, buses, 1.0)
                } else {
                    Signal::Value(1.0)
                };

                return Some(graph.add_node(SignalNode::LowPass {
                    input: Signal::Node(input),
                    cutoff: cutoff_signal,
                    q: q_signal,
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
            let params = &parts[1][..parts[1].len() - 1];
            let param_parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

            if let Some(input) = parse_expression(graph, input_expr, buses) {
                // Parse cutoff - can be a number, pattern, or reference
                let cutoff_signal = if let Some(cutoff_str) = param_parts.get(0) {
                    parse_parameter(graph, cutoff_str, buses, 1000.0)
                } else {
                    Signal::Value(1000.0)
                };

                // Q can also be a pattern, reference, or number
                let q_signal = if let Some(q_str) = param_parts.get(1) {
                    parse_parameter(graph, q_str, buses, 1.0)
                } else {
                    Signal::Value(1.0)
                };

                return Some(graph.add_node(SignalNode::HighPass {
                    input: Signal::Node(input),
                    cutoff: cutoff_signal,
                    q: q_signal,
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
                parse_expression(graph, parts[1], buses),
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
                parse_expression(graph, parts[1], buses),
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
