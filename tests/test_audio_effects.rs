/// Tests for audio effects in UnifiedSignalGraph
use phonon::unified_graph::{
    BitCrushState, ChorusState, ReverbState, Signal, SignalNode, UnifiedSignalGraph, Waveform,
};

#[test]
fn test_reverb_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a short pulse
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let reverb = graph.add_node(SignalNode::Reverb {
        input: Signal::Node(osc),
        room_size: Signal::Value(0.8),
        damping: Signal::Value(0.5),
        mix: Signal::Value(0.5),
        state: ReverbState::new(44100.0),
    });

    graph.set_output(reverb);

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Reverb should produce audio
    assert!(rms > 0.1, "Reverb should produce audio, got RMS={}", rms);
}

#[test]
fn test_reverb_extends_sound() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Short pulse (100ms)
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let reverb = graph.add_node(SignalNode::Reverb {
        input: Signal::Node(osc),
        room_size: Signal::Value(0.9), // Large room
        damping: Signal::Value(0.3),   // Low damping = longer tail
        mix: Signal::Value(1.0),       // 100% wet
        state: ReverbState::new(44100.0),
    });

    graph.set_output(reverb);

    let buffer = graph.render(88200); // 2 seconds

    // Check that sound persists well beyond the input duration
    // First 100ms should have signal
    let early_rms: f32 = (buffer[..4410].iter().map(|x| x * x).sum::<f32>() / 4410.0).sqrt();

    // 1-2 second range should still have reverb tail
    let late_rms: f32 = (buffer[44100..88200].iter().map(|x| x * x).sum::<f32>() / 44100.0).sqrt();

    assert!(early_rms > 0.01, "Early reverb should have signal");
    assert!(
        late_rms > 0.001,
        "Reverb tail should persist, got late RMS={}",
        late_rms
    );
}

#[test]
fn test_distortion_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let distortion = graph.add_node(SignalNode::Distortion {
        input: Signal::Node(osc),
        drive: Signal::Value(10.0),
        mix: Signal::Value(1.0),
    });

    graph.set_output(distortion);

    let buffer = graph.render(4410); // 100ms
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Distortion should produce audio
    assert!(
        rms > 0.3,
        "Distortion should produce audio, got RMS={}",
        rms
    );

    // Check that clipping occurred (values near ±1.0)
    let max_val = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(
        max_val > 0.9,
        "Distortion should clip signal to near ±1, got max={}",
        max_val
    );
}

#[test]
fn test_distortion_changes_waveform() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Pure sine wave
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(100.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let distortion = graph.add_node(SignalNode::Distortion {
        input: Signal::Node(osc),
        drive: Signal::Value(20.0),
        mix: Signal::Value(1.0),
    });

    graph.set_output(distortion);

    let buffer = graph.render(4410); // 100ms

    // Check that distortion flattens the peaks (waveshaping)
    let peak_count = buffer.iter().filter(|&&x| x.abs() > 0.95).count();

    // Heavily distorted sine should have many samples near ±1.0
    assert!(
        peak_count > 50,
        "Distortion should clip/flatten peaks, got {} peak samples",
        peak_count
    );
}

#[test]
fn test_bitcrush_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let bitcrush = graph.add_node(SignalNode::BitCrush {
        input: Signal::Node(osc),
        bits: Signal::Value(4.0),        // 4-bit
        sample_rate: Signal::Value(4.0), // 1/4 sample rate
        state: BitCrushState::default(),
    });

    graph.set_output(bitcrush);

    let buffer = graph.render(4410); // 100ms
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Bitcrush should produce audio
    assert!(rms > 0.1, "Bitcrush should produce audio, got RMS={}", rms);
}

#[test]
fn test_bitcrush_reduces_resolution() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
    });

    let bitcrush = graph.add_node(SignalNode::BitCrush {
        input: Signal::Node(osc),
        bits: Signal::Value(3.0),        // 3-bit = 8 levels
        sample_rate: Signal::Value(1.0), // No rate reduction
        state: BitCrushState::default(),
    });

    graph.set_output(bitcrush);

    let buffer = graph.render(4410); // 100ms

    // Count unique values (should be limited by bit depth)
    let mut unique_values: Vec<f32> = buffer
        .iter()
        .map(|&x| (x * 1000.0).round() / 1000.0) // Round to 3 decimals for comparison
        .collect();
    unique_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    unique_values.dedup();

    // 3-bit = 8 levels (±1.0 range)
    // Should have roughly 8-16 unique values (accounting for quantization)
    assert!(
        unique_values.len() < 30,
        "Bitcrusher should reduce resolution, got {} unique values",
        unique_values.len()
    );
}

#[test]
fn test_chorus_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Saw,
        phase: 0.0,
    });

    let chorus = graph.add_node(SignalNode::Chorus {
        input: Signal::Node(osc),
        rate: Signal::Value(1.0),  // 1 Hz LFO
        depth: Signal::Value(0.8), // Strong modulation
        mix: Signal::Value(0.5),   // 50% wet
        state: ChorusState::new(44100.0),
    });

    graph.set_output(chorus);

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Chorus should produce audio
    assert!(rms > 0.1, "Chorus should produce audio, got RMS={}", rms);
}

#[test]
fn test_chorus_creates_modulation() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(220.0),
        waveform: Waveform::Saw,
        phase: 0.0,
    });

    let chorus = graph.add_node(SignalNode::Chorus {
        input: Signal::Node(osc),
        rate: Signal::Value(2.0),  // 2 Hz LFO
        depth: Signal::Value(1.0), // Maximum modulation
        mix: Signal::Value(1.0),   // 100% wet
        state: ChorusState::new(44100.0),
    });

    graph.set_output(chorus);

    let buffer = graph.render(44100); // 1 second

    // Analyze amplitude modulation (chorus should create beating)
    let chunk_size = 2205; // 50ms chunks
    let mut chunk_rms_values = Vec::new();
    for chunk in buffer.chunks(chunk_size) {
        let rms = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
        chunk_rms_values.push(rms);
    }

    // Calculate variance of RMS values
    let mean_rms = chunk_rms_values.iter().sum::<f32>() / chunk_rms_values.len() as f32;
    let variance = chunk_rms_values
        .iter()
        .map(|x| (x - mean_rms).powi(2))
        .sum::<f32>()
        / chunk_rms_values.len() as f32;

    // Chorus should create amplitude variation
    assert!(
        variance > 0.0003,
        "Chorus should create amplitude modulation, got variance={}",
        variance
    );
}

#[test]
fn test_effects_chain() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Oscillator -> Distortion -> Chorus -> Reverb
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        phase: 0.0,
    });

    let distortion = graph.add_node(SignalNode::Distortion {
        input: Signal::Node(osc),
        drive: Signal::Value(5.0),
        mix: Signal::Value(0.5),
    });

    let chorus = graph.add_node(SignalNode::Chorus {
        input: Signal::Node(distortion),
        rate: Signal::Value(1.5),
        depth: Signal::Value(0.6),
        mix: Signal::Value(0.4),
        state: ChorusState::new(44100.0),
    });

    let reverb = graph.add_node(SignalNode::Reverb {
        input: Signal::Node(chorus),
        room_size: Signal::Value(0.7),
        damping: Signal::Value(0.5),
        mix: Signal::Value(0.3),
        state: ReverbState::new(44100.0),
    });

    graph.set_output(reverb);

    let buffer = graph.render(22050); // 0.5 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Full effects chain should produce audio
    assert!(
        rms > 0.1,
        "Effects chain should produce audio, got RMS={}",
        rms
    );
}
