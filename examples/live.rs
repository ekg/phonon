//! Phonon Live - Interactive audio patches
//!
//! Run with: cargo run --example live
//! Then modify this file and recompile to hear changes

use std::cell::RefCell;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::unified_graph::{Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Phonon Live 🎵");
    println!("=================");
    println!();

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

    // Create the graph - EDIT THIS SECTION!
    let graph = Arc::new(Mutex::new(create_patch(sample_rate)));

    // Setup audio stream
    let graph_clone = Arc::clone(&graph);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), graph_clone)?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    println!("🎹 Playing... Press Ctrl+C to stop");
    println!();
    println!("To change the sound:");
    println!("1. Edit the create_patch() function below");
    println!("2. Recompile with: cargo run --example live");
    println!();

    // Keep playing
    std::thread::park();
    Ok(())
}

/// EDIT THIS FUNCTION TO CHANGE THE SOUND!
fn create_patch(sample_rate: f32) -> UnifiedSignalGraph {
    let mut graph = UnifiedSignalGraph::new(sample_rate);

    // ==========================================
    // CHOOSE A PATCH (uncomment one)
    // ==========================================

    // patch_1_basic_lfo(&mut graph);
    // patch_2_pattern_drums(&mut graph);
    patch_3_bass_sidechain(&mut graph);
    // patch_4_fm_synthesis(&mut graph);
    // patch_5_ambient_pad(&mut graph);

    graph
}

/// Patch 1: Basic LFO modulating filter
fn patch_1_basic_lfo(graph: &mut UnifiedSignalGraph) {
    graph.set_cps(0.5);

    // LFO
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.5), // Try: 0.1, 1.0, 2.0
        waveform: Waveform::Sine, // Try: Triangle, Saw
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Oscillator
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0), // Try: 55, 220, 440
        waveform: Waveform::Saw,    // Try: Square, Triangle
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Modulated filter cutoff
    let cutoff = graph.add_node(SignalNode::Add {
        a: Signal::Value(1000.0), // Base cutoff
        b: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(lfo),
            Signal::Value(1500.0), // Modulation depth
        ))),
    });

    // Filter
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(osc),
        cutoff: Signal::Node(cutoff),
        q: Signal::Value(3.0), // Try: 1.0 to 10.0
        state: Default::default(),
    });

    // Output with volume
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(filtered),
        b: Signal::Value(0.3),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);
}

/// Patch 2: Pattern-driven drums
fn patch_2_pattern_drums(graph: &mut UnifiedSignalGraph) {
    graph.set_cps(2.0); // Tempo

    // Kick pattern
    let kick_pattern = parse_mini_notation("bd ~ ~ bd"); // Try: "bd bd ~ bd", "bd ~ bd ~"
    let kick_trig = graph.add_node(SignalNode::Pattern {
        pattern_str: "bd ~ ~ bd".to_string(),
        pattern: kick_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Kick oscillator
    let kick = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(60.0), // Try: 50, 80
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Kick envelope
    let kick_env = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(kick),
        trigger: Signal::Node(kick_trig),
        attack: Signal::Value(0.001),
        decay: Signal::Value(0.15), // Try: 0.05 to 0.3
        sustain: Signal::Value(0.0),
        release: Signal::Value(0.05),
        state: Default::default(),
    });

    // Snare pattern
    let snare_pattern = parse_mini_notation("~ sn ~ sn"); // Try: "~ sn", "sn sn ~ sn"
    let snare_trig = graph.add_node(SignalNode::Pattern {
        pattern_str: "~ sn ~ sn".to_string(),
        pattern: snare_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Snare (noise)
    let noise = graph.add_node(SignalNode::Noise { seed: 12345 });
    let snare_filtered = graph.add_node(SignalNode::HighPass {
        input: Signal::Node(noise),
        cutoff: Signal::Value(2000.0), // Try: 1000 to 5000
        q: Signal::Value(2.0),
        state: Default::default(),
    });

    // Snare envelope
    let snare_env = graph.add_node(SignalNode::Envelope {
        input: Signal::Node(snare_filtered),
        trigger: Signal::Node(snare_trig),
        attack: Signal::Value(0.001),
        decay: Signal::Value(0.05),
        sustain: Signal::Value(0.0),
        release: Signal::Value(0.02),
        state: Default::default(),
    });

    // Mix drums
    let mixed = graph.add_node(SignalNode::Add {
        a: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(kick_env),
            Signal::Value(0.8),
        ))),
        b: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(snare_env),
            Signal::Value(0.3),
        ))),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(mixed),
    });

    graph.set_output(output);
}

/// Patch 3: Bass with sidechain compression
fn patch_3_bass_sidechain(graph: &mut UnifiedSignalGraph) {
    graph.set_cps(2.0);

    // Kick pattern for sidechain
    let kick_pattern = parse_mini_notation("1 0 0 0"); // Try: "1 0 1 0", "1 0 0 1"
    let kick_trig = graph.add_node(SignalNode::Pattern {
        pattern_str: "1 0 0 0".to_string(),
        pattern: kick_pattern,
        last_value: 0.0,
        last_trigger_time: -1.0,
    });

    // Bass notes
    let bass_pattern = parse_mini_notation("55 55 82.5 55"); // Try: "110 110 82.5 110"
    let bass_freq = graph.add_node(SignalNode::Pattern {
        pattern_str: "55 55 82.5 55".to_string(),
        pattern: bass_pattern,
        last_value: 55.0,
        last_trigger_time: -1.0,
    });

    // Bass oscillator
    let bass = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(bass_freq),
        waveform: Waveform::Saw,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Sidechain envelope (inverted kick)
    let sidechain = graph.add_node(SignalNode::Add {
        a: Signal::Value(1.0),
        b: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(kick_trig),
            Signal::Value(-0.7), // Sidechain depth
        ))),
    });

    // Apply sidechain
    let bass_compressed = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(bass),
        b: Signal::Node(sidechain),
    });

    // Filter with LFO
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.25),
        waveform: Waveform::Triangle,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let cutoff = graph.add_node(SignalNode::Add {
        a: Signal::Value(800.0),
        b: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(lfo),
            Signal::Value(700.0),
        ))),
    });

    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(bass_compressed),
        cutoff: Signal::Node(cutoff),
        q: Signal::Value(3.0),
        state: Default::default(),
    });

    // Add some delay
    let delayed = graph.add_node(SignalNode::Delay {
        input: Signal::Node(filtered),
        time: Signal::Value(0.375), // Dotted eighth
        feedback: Signal::Value(0.4),
        mix: Signal::Value(0.3),
        buffer: vec![0.0; 33075],
        write_idx: 0,
    });

    // Output
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(delayed),
        b: Signal::Value(0.4),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);
}

/// Patch 4: FM Synthesis
fn patch_4_fm_synthesis(graph: &mut UnifiedSignalGraph) {
    graph.set_cps(0.5);

    // Modulator
    let mod_freq = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0), // Try: 55, 220
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let modulator = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(mod_freq),
        b: Signal::Value(200.0), // Modulation index
    });

    // Carrier with FM
    let carrier_freq = graph.add_node(SignalNode::Add {
        a: Signal::Value(440.0), // Base frequency
        b: Signal::Node(modulator),
    });

    let carrier = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(carrier_freq),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Output
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(carrier),
        b: Signal::Value(0.2),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);
}

/// Patch 5: Ambient pad
fn patch_5_ambient_pad(graph: &mut UnifiedSignalGraph) {
    graph.set_cps(0.25);

    // Multiple detuned oscillators
    let osc1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Triangle,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let osc2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.5), // Slight detune
        waveform: Waveform::Triangle,
        semitone_offset: 0.0,
        phase: RefCell::new(0.25),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let osc3 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(330.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.5),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Mix oscillators
    let mix1 = graph.add_node(SignalNode::Add {
        a: Signal::Node(osc1),
        b: Signal::Node(osc2),
    });

    let mixed = graph.add_node(SignalNode::Add {
        a: Signal::Node(mix1),
        b: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(osc3),
            Signal::Value(0.3),
        ))),
    });

    // Slow filter sweep
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.1),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let cutoff = graph.add_node(SignalNode::Add {
        a: Signal::Value(1500.0),
        b: Signal::Expression(Box::new(SignalExpr::Multiply(
            Signal::Node(lfo),
            Signal::Value(1000.0),
        ))),
    });

    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(mixed),
        cutoff: Signal::Node(cutoff),
        q: Signal::Value(1.5),
        state: Default::default(),
    });

    // Long delay for space
    let delayed = graph.add_node(SignalNode::Delay {
        input: Signal::Node(filtered),
        time: Signal::Value(0.5),
        feedback: Signal::Value(0.6),
        mix: Signal::Value(0.5),
        buffer: vec![0.0; 44100],
        write_idx: 0,
    });

    // Output
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(delayed),
        b: Signal::Value(0.1),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(scaled),
    });

    graph.set_output(output);
}

fn build_stream<T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    graph: Arc<Mutex<UnifiedSignalGraph>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let channels = config.channels as usize;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut graph = graph.lock().unwrap();

            // Fill buffer
            for frame in data.chunks_mut(channels) {
                let sample = graph.process_sample();
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
