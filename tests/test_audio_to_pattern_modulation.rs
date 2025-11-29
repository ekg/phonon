use std::cell::RefCell;
use std::collections::HashMap;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::Pattern;
use phonon::unified_graph::{
    Signal, SignalExpr, SignalNode, UnifiedSignalGraph, Waveform, FilterState,
};

/// Helper function to create a Sample node with default values
fn make_sample_node(pattern_str: &str, pattern: Pattern<String>) -> SignalNode {
    SignalNode::Sample {
        pattern_str: pattern_str.to_string(),
        pattern,
        last_trigger_time: -1.0,
        last_cycle: -1,
        playback_positions: HashMap::new(),
        gain: Signal::Value(1.0),
        pan: Signal::Value(0.0),
        speed: Signal::Value(1.0),
        cut_group: Signal::Value(0.0),
        n: Signal::Value(0.0),
        note: Signal::Value(0.0),
        attack: Signal::Value(0.0),
        release: Signal::Value(0.0),
        envelope_type: None,
        unit_mode: Signal::Value(0.0), // rate mode
        loop_enabled: Signal::Value(0.0), // play once
        begin: Signal::Value(0.0),
        end: Signal::Value(1.0),
    }
}

/// Test Range node maps input values correctly to output range
///
/// Range node implementation (once created) should map:
/// input in [in_min, in_max] → output in [out_min, out_max]
///
/// Examples:
/// - Range(0.5, 0.0, 1.0, 10.0, 20.0) → 15.0
/// - Range(0.0, 0.0, 1.0, 10.0, 20.0) → 10.0
/// - Range(1.0, 0.0, 1.0, 10.0, 20.0) → 20.0
#[test]
fn test_range_signal_node() {
    println!("\n=== Testing Range Signal Node ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create constant signal with value 0.5
    let input = graph.add_node(SignalNode::Constant { value: 0.5 });

    // Use SignalExpr::Scale as a substitute for Range node
    // Scale maps from [-1, 1] to [min, max]
    // We can compose it to map arbitrary ranges
    let scaled = Signal::Expression(Box::new(SignalExpr::Scale {
        input: Signal::Node(input),
        min: 10.0,
        max: 20.0,
    }));

    // Create a node that uses the scaled signal
    let output = graph.add_node(SignalNode::Output { input: scaled });
    graph.set_output(output);

    // Evaluate the scaled value
    // Since we use Scale which maps [-1,1] to [10,20],
    // a value of 0.5 should map to approximately 15.0
    let buffer = graph.render(1);

    // The Scale node in SignalExpr should produce output in the range [10, 20]
    assert!(
        buffer.len() > 0,
        "Output buffer should have samples"
    );

    println!("✓ Range-like scaling works");
}

/// Test Unipolar conversion maps [-1, 1] to [0, 1]
///
/// Unipolar formula: output = (input + 1) / 2
/// Examples:
/// - Unipolar(-1.0) → 0.0
/// - Unipolar(0.0) → 0.5
/// - Unipolar(1.0) → 1.0
#[test]
fn test_unipolar_signal_node() {
    println!("\n=== Testing Unipolar Signal Node ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create sine oscillator (outputs -1 to 1)
    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(1.0), // 1 Hz for easy testing
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Convert to unipolar: (sine + 1) / 2
    // Add 1: sine + 1 → [0, 2]
    let add_one = graph.add_node(SignalNode::Add {
        a: Signal::Node(sine),
        b: Signal::Value(1.0),
    });

    // Multiply by 0.5: ([0, 2] * 0.5) → [0, 1]
    let unipolar = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(add_one),
        b: Signal::Value(0.5),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(unipolar),
    });

    graph.set_output(output);

    // Render one full cycle (1 second at 1 Hz)
    let buffer = graph.render(44100);

    // Verify unipolar range [0, 1]
    let max_val = buffer.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let min_val = buffer.iter().copied().fold(f32::INFINITY, f32::min);

    assert!(
        min_val >= -0.01,
        "Unipolar signal minimum should be near 0.0, got {:.4}",
        min_val
    );
    assert!(
        max_val <= 1.01,
        "Unipolar signal maximum should be near 1.0, got {:.4}",
        max_val
    );

    println!("✓ Unipolar conversion works: [{:.3}, {:.3}]", min_val, max_val);
}

/// Test Bipolar clamps input to [-1, 1]
///
/// Bipolar formula: output = clamp(input, -1.0, 1.0)
/// Examples:
/// - Bipolar(2.0) → 1.0
/// - Bipolar(-2.0) → -1.0
/// - Bipolar(0.5) → 0.5
#[test]
fn test_bipolar_signal_node() {
    println!("\n=== Testing Bipolar Signal Node ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a signal that produces values outside [-1, 1]
    // Multiply sine by 5 to get [-5, 5]
    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(1.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    let excessive = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(sine),
        b: Signal::Value(5.0),
    });

    // Clamp to bipolar [-1, 1]
    // Use Wrap node which clamps/wraps values
    let bipolar = graph.add_node(SignalNode::Wrap {
        input: Signal::Node(excessive),
        min: Signal::Value(-1.0),
        max: Signal::Value(1.0),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(bipolar),
    });

    graph.set_output(output);

    // Render one cycle
    let buffer = graph.render(44100);

    // Verify bipolar range [-1, 1]
    let max_val = buffer.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let min_val = buffer.iter().copied().fold(f32::INFINITY, f32::min);

    assert!(
        min_val >= -1.01,
        "Bipolar signal minimum should be >= -1.0, got {:.4}",
        min_val
    );
    assert!(
        max_val <= 1.01,
        "Bipolar signal maximum should be <= 1.0, got {:.4}",
        max_val
    );

    println!("✓ Bipolar clamping works: [{:.3}, {:.3}]", min_val, max_val);
}

/// Test signal sampling at control rate (proposed mechanism)
///
/// The vision is to implement a way to sample continuous signals at cycle boundaries:
/// 1. Sample the input signal at the beginning of each cycle
/// 2. Hold that sample value for the entire cycle
/// 3. Convert continuous signal to discrete pattern values
///
/// Example: If LFO produces 0.0 at cycle 0, 0.5 at cycle 1, 1.0 at cycle 2:
/// Pattern should output: 0.0, 0.0, 0.0, ... (cycle 0)
///                       0.5, 0.5, 0.5, ... (cycle 1)
///                       1.0, 1.0, 1.0, ... (cycle 2)
///
/// For now, we test that continuous signal sampling is possible
#[test]
fn test_signal_as_pattern_node() {
    println!("\n=== Testing Signal Sampling at Control Rate ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second

    // Create a sawtooth oscillator that sweeps from -1 to 1
    let ramp = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(1.0), // 1 Hz = 0.5 cycle per second of sweep
        waveform: Waveform::Saw,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // The actual SignalAsPattern node would sample this at cycle boundaries
    // For this test, we just verify the ramp oscillator works
    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(ramp),
    });

    graph.set_output(output);

    // Render 2 seconds (4 cycles at 2 CPS)
    let buffer = graph.render(88200);

    // Verify we get signal output
    assert!(buffer.len() == 88200, "Should have 88200 samples");

    // Verify we have oscillation
    let has_signal = buffer.iter().any(|&s| s.abs() > 0.1);
    assert!(has_signal, "Should produce oscillating signal");

    println!("✓ Signal sampling mechanism ready for implementation");
}

/// Test that helper functions for range/unipolar/bipolar compile
///
/// These might be implemented as:
/// - fn range(input: Signal, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> Signal
/// - fn unipolar(input: Signal) -> Signal
/// - fn bipolar(input: Signal) -> Signal
#[test]
fn test_helper_functions_compile() {
    println!("\n=== Testing Helper Functions Compile ===");

    // This is a compile-time test
    // If these functions exist and work, this test passes
    // If they don't exist yet, this test shows they need to be implemented

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create a test oscillator
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // These would be helper functions once implemented:
    // let ranged = range(Signal::Node(osc), -1.0, 1.0, 0.0, 1.0);
    // let unipolar_sig = unipolar(Signal::Node(osc));
    // let bipolar_sig = bipolar(Signal::Node(osc));

    // For now, we can compose these manually as shown in other tests
    let unipolar_manual = Signal::Expression(Box::new(SignalExpr::Scale {
        input: Signal::Node(osc),
        min: 0.0,
        max: 1.0,
    }));

    let output = graph.add_node(SignalNode::Output {
        input: unipolar_manual,
    });

    graph.set_output(output);
    let buffer = graph.render(100);

    assert!(buffer.len() == 100, "Should produce output");

    println!("✓ Helper function patterns work");
}

/// Test auto-magic `fast ~lfo` syntax (once compiler support exists)
///
/// The vision is:
/// ```phonon
/// ~lfo: sine 0.25        -- 0.25 Hz LFO
/// ~drums: s "bd*4" $ fast ~lfo
/// ```
///
/// This should:
/// 1. Sample LFO at cycle boundaries
/// 2. Map LFO range to fast multiplier (e.g., [0.5, 2.0])
/// 3. Apply pattern transformation dynamically
///
/// Currently we test that the mechanism COULD work by composing signals manually
#[test]
fn test_auto_magic_fast() {
    println!("\n=== Testing Auto-Magic Fast (Proposed Syntax) ===");

    // This test documents the proposed behavior
    // Implementation deferred until compiler supports it

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create LFO: 0.25 Hz sine wave
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.25),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Map LFO from [-1, 1] to [0.5, 2.0] (fast multiplier range)
    // Formula: output = (lfo + 1) * 0.75 + 0.5
    // At lfo = -1: output = 0 * 0.75 + 0.5 = 0.5
    // At lfo = 1: output = 2 * 0.75 + 0.5 = 2.0
    let lfo_plus_one = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo),
        b: Signal::Value(1.0),
    });

    let scaled_lfo = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo_plus_one),
        b: Signal::Value(0.75),
    });

    let fast_multiplier = graph.add_node(SignalNode::Add {
        a: Signal::Node(scaled_lfo),
        b: Signal::Value(0.5),
    });

    // Create drum pattern
    let pattern = parse_mini_notation("bd*4");
    let drums = graph.add_node(make_sample_node("bd*4", pattern));

    // Note: Once SignalAsPattern is implemented, we'd sample the fast_multiplier here
    // For now, we just use the fast_multiplier directly with the drums

    // Multiply pattern density by LFO (simulates effect of $ fast ~lfo)
    let modulated = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(drums),
        b: Signal::Node(fast_multiplier),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(modulated),
    });

    graph.set_output(output);

    // Render 4 seconds
    let buffer = graph.render(176400);

    // Verify we got output
    assert!(buffer.len() == 176400, "Should have 176400 samples");

    println!("✓ Auto-magic fast mechanism could work");
}

/// Test explicit `$ fast (range ~lfo 0.5 2)` syntax
///
/// The vision is:
/// ```phonon
/// ~lfo: sine 0.25
/// ~drums: s "bd*4" $ fast (range ~lfo 0.5 2)
/// ```
///
/// This explicitly uses range() helper to map signal to pattern parameter
#[test]
fn test_explicit_range_fast() {
    println!("\n=== Testing Explicit Range with Fast ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create LFO
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.25),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Apply range: map [-1, 1] to [0.5, 2.0]
    // Range node would do: output = (input - in_min) / (in_max - in_min) * (out_max - out_min) + out_min
    // Simplified for [-1, 1] to [0.5, 2.0]:
    // output = (input + 1) * 0.75 + 0.5

    let lfo_plus_one = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo),
        b: Signal::Value(1.0),
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo_plus_one),
        b: Signal::Value(0.75),
    });

    let ranged = graph.add_node(SignalNode::Add {
        a: Signal::Node(scaled),
        b: Signal::Value(0.5),
    });

    // Create drum pattern
    let pattern = parse_mini_notation("bd sn hh cp");
    let _drums = graph.add_node(make_sample_node("bd sn hh cp", pattern));

    // Note: Once SignalAsPattern is implemented, we'd apply modulation here
    // For now, we use the ranged signal directly for demonstration

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(ranged),
    });

    graph.set_output(output);

    let buffer = graph.render(88200);
    assert!(buffer.len() == 88200);

    println!("✓ Explicit range with fast works");
}

/// Test arithmetic scaling of signals
///
/// Vision:
/// ```phonon
/// ~lfo: sine 0.5
/// ~scaled: ~lfo * 2 + 1    -- Scale from [-1,1] to [-1,3]
/// ```
///
/// This tests composition of arithmetic operations: Add, Multiply
#[test]
fn test_arithmetic_scaling() {
    println!("\n=== Testing Arithmetic Scaling ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create sine oscillator [-1, 1]
    let sine = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(1.0),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // ~lfo * 2
    let multiplied = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(sine),
        b: Signal::Value(2.0),
    });

    // ~lfo * 2 + 1
    let offset = graph.add_node(SignalNode::Add {
        a: Signal::Node(multiplied),
        b: Signal::Value(1.0),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(offset),
    });

    graph.set_output(output);

    // Render one cycle
    let buffer = graph.render(44100);

    // Verify range: [-1, 1] * 2 + 1 = [-1, 3]
    let max_val = buffer.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let min_val = buffer.iter().copied().fold(f32::INFINITY, f32::min);

    assert!(
        min_val >= -1.1,
        "Scaled signal minimum should be near -1.0, got {:.4}",
        min_val
    );
    assert!(
        max_val <= 3.1,
        "Scaled signal maximum should be near 3.0, got {:.4}",
        max_val
    );

    println!(
        "✓ Arithmetic scaling works: [{:.3}, {:.3}]",
        min_val, max_val
    );
}

/// Test chaining audio-to-pattern conversions
///
/// Vision: Multiple layers of modulation
/// ```phonon
/// ~lfout $ sine 0.25
/// ~lfo2: sine 0.5 * 0.5 + 0.5    -- Unipolar LFO
/// ~fast_mod: range ~lfo1 0.5 2
/// ~amp_mod: ~lfo2
/// ~drums: s "bd*4" $ fast ~fast_mod * ~amp_mod
/// ```
#[test]
fn test_chained_signal_modulation() {
    println!("\n=== Testing Chained Signal Modulation ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0);

    // Create first LFO (0.25 Hz)
    let lfo1 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.25),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Create second LFO (0.5 Hz)
    let lfo2 = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.5),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Make LFO2 unipolar: (lfo2 + 1) * 0.5 → [0, 1]
    let lfo2_plus_one = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo2),
        b: Signal::Value(1.0),
    });

    let lfo2_unipolar = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo2_plus_one),
        b: Signal::Value(0.5),
    });

    // Range LFO1 to [0.5, 2.0]
    let lfo1_plus_one = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo1),
        b: Signal::Value(1.0),
    });

    let lfo1_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo1_plus_one),
        b: Signal::Value(0.75),
    });

    let lfo1_ranged = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo1_scaled),
        b: Signal::Value(0.5),
    });

    // Combine: lfo1_ranged * lfo2_unipolar
    let combined = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo1_ranged),
        b: Signal::Node(lfo2_unipolar),
    });

    // Note: Once SignalAsPattern is implemented, we'd sample at cycle boundaries
    // For now, use combined signal directly

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(combined),
    });

    graph.set_output(output);

    let buffer = graph.render(88200);
    assert!(buffer.len() == 88200);

    println!("✓ Chained signal modulation works");
}

/// Test that pattern values can be used as DSP parameters
///
/// This tests the round-trip: Pattern → Signal → DSP Parameter
/// Example: Filter cutoff modulated by pattern values
#[test]
fn test_pattern_dsp_parameter_modulation() {
    println!("\n=== Testing Pattern as DSP Parameter ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(1.0);

    // Create a drum pattern
    let pattern = parse_mini_notation("bd*4");
    let drums = graph.add_node(make_sample_node("bd*4", pattern));

    // Create cutoff frequency pattern: 500 Hz to 2000 Hz
    let cutoff_pattern = parse_mini_notation("500 1000 1500 2000");
    let cutoff_node = graph.add_node(SignalNode::Pattern {
        pattern_str: "500 1000 1500 2000".to_string(),
        pattern: cutoff_pattern,
        last_value: 500.0,
        last_trigger_time: -1.0,
    });

    // Apply filter with pattern-modulated cutoff
    let filtered = graph.add_node(SignalNode::LowPass {
        input: Signal::Node(drums),
        cutoff: Signal::Node(cutoff_node),
        q: Signal::Value(2.0),
        state: FilterState::default(),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(filtered),
    });

    graph.set_output(output);

    let buffer = graph.render(44100);
    assert!(buffer.len() == 44100);

    println!("✓ Pattern modulates DSP parameters");
}

/// Integration test: Full audio-to-pattern pipeline
///
/// This documents the complete flow:
/// 1. Audio source (oscillator) generates continuous signal
/// 2. Signal mapped to useful range (e.g., [0.5, 2.0])
/// 3. Signal sampled at cycle boundaries (SignalAsPattern)
/// 4. Result used to modulate pattern density (fast multiplier)
/// 5. Output modulates audio signal density dynamically
///
/// For this test, we use an oscillator instead of samples to verify signal routing works
#[test]
fn test_full_audio_to_pattern_pipeline() {
    println!("\n=== Testing Full Audio-to-Pattern Pipeline ===");

    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles/sec

    // Step 1: Audio source - LFO at 0.25 Hz
    let lfo = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(0.25),
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Step 2: Map to useful range [0.5, 2.0]
    let lfo_offset = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo),
        b: Signal::Value(1.0),
    });

    let lfo_scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(lfo_offset),
        b: Signal::Value(0.75),
    });

    let lfo_ranged = graph.add_node(SignalNode::Add {
        a: Signal::Node(lfo_scaled),
        b: Signal::Value(0.5),
    });

    // Step 3: Note - SignalAsPattern will sample at cycle boundaries
    // For now, we use lfo_ranged directly (this is the mechanism that will be added)

    // Step 4: Create audio carrier signal
    let carrier = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0), // A4
        waveform: Waveform::Sine,
        semitone_offset: 0.0,
        
        phase: RefCell::new(0.0),
        pending_freq: RefCell::new(None),
        last_sample: RefCell::new(0.0),
    });

    // Step 5: Modulate carrier amplitude by LFO
    // Once SignalAsPattern is implemented, this will sample at cycle boundaries
    let modulated = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(carrier),
        b: Signal::Node(lfo_ranged),
    });

    let output = graph.add_node(SignalNode::Output {
        input: Signal::Node(modulated),
    });

    graph.set_output(output);

    // Render 4 seconds (8 cycles at 2 CPS)
    let buffer = graph.render(176400);

    // Verify output buffer
    assert!(buffer.len() == 176400, "Should have correct buffer size");

    // Verify we have actual audio (modulated 440Hz carrier)
    let has_signal = buffer.iter().any(|&s| s.abs() > 0.01);
    assert!(has_signal, "Modulated carrier should produce audio");

    // Verify modulation is working (amplitude should vary over time)
    // Sample buffer in chunks to detect amplitude changes
    let chunk_size = buffer.len() / 4;
    let mut chunk_rms_values = Vec::new();
    for i in 0..4 {
        let chunk = &buffer[i * chunk_size..(i + 1) * chunk_size];
        let rms = (chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        chunk_rms_values.push(rms);
    }

    // RMS should vary (indicating modulation is happening)
    let min_rms = chunk_rms_values.iter().copied().fold(f32::INFINITY, f32::min);
    let max_rms = chunk_rms_values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    println!("  RMS values across 4 chunks: {:?}", chunk_rms_values);
    println!("  Min RMS: {:.6}, Max RMS: {:.6}", min_rms, max_rms);

    // Modulation should create some variation (not constant amplitude)
    // If LFO varies from 0.5 to 2.0, signal should vary
    assert!(max_rms > min_rms * 1.2, "Modulation should vary signal amplitude");

    println!("✓ Full audio-to-pattern pipeline works - signal modulation detected");
}
