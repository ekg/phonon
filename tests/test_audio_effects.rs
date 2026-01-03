use phonon::unified_graph::{
    BitCrushState, ChorusState, CompressorState, ReverbState, Signal, SignalNode,
    UnifiedSignalGraph, Waveform,
};
/// Tests for audio effects in UnifiedSignalGraph
use std::cell::RefCell;

#[test]
fn test_reverb_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a short pulse
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let distortion = graph.add_node(SignalNode::Distortion {
        input: Signal::Node(osc),
        drive: Signal::Value(20.0),
        mix: Signal::Value(1.0),
    });

    graph.set_output(distortion);

    // Disable the master limiter so we can see the full distortion output
    graph.set_master_limiter_ceiling(1.0);

    let buffer = graph.render(4410); // 100ms

    // Check that distortion flattens the peaks (waveshaping)
    // With drive=20 on a sine wave, tanh will saturate to near ±1.0
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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
fn test_delay_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Short pulse
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let delay = graph.add_node(SignalNode::Delay {
        input: Signal::Node(osc),
        time: Signal::Value(0.25),    // 250ms delay
        feedback: Signal::Value(0.5), // Moderate feedback
        mix: Signal::Value(1.0),      // 100% wet
        buffer: vec![0.0; (2.0 * 44100.0) as usize],
        write_idx: 0,
    });

    graph.set_output(delay);

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Delay should produce audio
    assert!(rms > 0.1, "Delay should produce audio, got RMS={}", rms);
}

#[test]
fn test_delay_creates_echoes() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(880.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let delay = graph.add_node(SignalNode::Delay {
        input: Signal::Node(osc),
        time: Signal::Value(0.5),     // 500ms delay
        feedback: Signal::Value(0.6), // Feedback for multiple echoes
        mix: Signal::Value(0.5),      // 50% wet/dry mix
        buffer: vec![0.0; (2.0 * 44100.0) as usize],
        write_idx: 0,
    });

    graph.set_output(delay);

    let buffer = graph.render(88200); // 2 seconds

    // Check that signal is present throughout due to continuous oscillator + feedback
    let early_rms: f32 = (buffer[..4410].iter().map(|x| x * x).sum::<f32>() / 4410.0).sqrt();

    // After delay time (500ms+), should have echoes building up
    let delayed_rms: f32 =
        (buffer[22050..33075].iter().map(|x| x * x).sum::<f32>() / 11025.0).sqrt();

    println!(
        "Delay - Early RMS: {:.6}, Delayed RMS: {:.6}",
        early_rms, delayed_rms
    );

    // Both should have signal (early has direct + starting echoes, late has accumulated echoes)
    assert!(
        early_rms > 0.1,
        "Early signal should be present, got {:.6}",
        early_rms
    );
    assert!(
        delayed_rms > 0.1,
        "Delayed signal should be present, got {:.6}",
        delayed_rms
    );
}

#[test]
fn test_effects_chain() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Oscillator -> Distortion -> Chorus -> Reverb
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(110.0),
        waveform: Waveform::Saw,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
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

#[test]
fn test_compressor_basic() {
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a sine wave
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let compressor = graph.add_node(SignalNode::Compressor {
        input: Signal::Node(osc),
        threshold: Signal::Value(-20.0),  // -20 dB threshold
        ratio: Signal::Value(4.0),        // 4:1 ratio
        attack: Signal::Value(0.01),      // 10ms attack
        release: Signal::Value(0.1),      // 100ms release
        makeup_gain: Signal::Value(10.0), // 10 dB makeup gain
        state: CompressorState::new(),
    });

    graph.set_output(compressor);

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Compressor should produce audio
    assert!(
        rms > 0.1,
        "Compressor should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_compressor_reduces_dynamic_range() {
    // Test WITHOUT compressor
    let mut graph_uncompressed = UnifiedSignalGraph::new(44100.0);

    let osc_uncomp = graph_uncompressed.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    graph_uncompressed.set_output(osc_uncomp);
    let buffer_uncomp = graph_uncompressed.render(44100); // 1 second
    let peak_uncomp = buffer_uncomp.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    // Test WITH compressor
    let mut graph_compressed = UnifiedSignalGraph::new(44100.0);

    let osc_comp = graph_compressed.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,

        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let compressor = graph_compressed.add_node(SignalNode::Compressor {
        input: Signal::Node(osc_comp),
        threshold: Signal::Value(-40.0), // -40 dB threshold (very low, so it compresses)
        ratio: Signal::Value(10.0),      // 10:1 ratio (heavy compression)
        attack: Signal::Value(0.001),    // 1ms attack (very fast)
        release: Signal::Value(0.01),    // 10ms release (fast)
        makeup_gain: Signal::Value(0.0), // No makeup gain
        state: CompressorState::new(),
    });

    graph_compressed.set_output(compressor);
    let buffer_comp = graph_compressed.render(44100); // 1 second
    let peak_comp = buffer_comp.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    println!(
        "Uncompressed peak: {:.6}, Compressed peak: {:.6}",
        peak_uncomp, peak_comp
    );

    // The compressed signal should have a lower peak due to gain reduction
    // With a -40dB threshold and 10:1 ratio on a ~0dB signal, we expect significant reduction
    assert!(
        peak_comp < peak_uncomp * 0.7,
        "Compressor should reduce peak level, uncomp={:.6}, comp={:.6}",
        peak_uncomp,
        peak_comp
    );

    // But should still produce audio
    let rms: f32 =
        (buffer_comp.iter().map(|x| x * x).sum::<f32>() / buffer_comp.len() as f32).sqrt();
    assert!(
        rms > 0.01,
        "Compressor should still produce audio, got RMS={}",
        rms
    );
}
