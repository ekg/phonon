/// Test musical notation (note names, scales, chords) with audio verification
///
/// This test suite ensures that note names (c4, d#4, etc.) are correctly
/// parsed and converted to the proper frequencies in the audio output.
/// We use FFT analysis to verify that the generated audio contains
/// the expected frequency components.
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern_tonal::{midi_to_freq, note_to_midi};
use phonon::unified_graph::{Signal, SignalNode, UnifiedSignalGraph, Waveform};
use std::collections::HashMap;
use std::f32::consts::PI;

/// FFT-based frequency detector
/// Returns the dominant frequency in Hz
fn detect_frequency(samples: &[f32], sample_rate: f32) -> Option<f32> {
    use rustfft::{num_complex::Complex, FftPlanner};

    if samples.len() < 1024 {
        return None;
    }

    // Take first power-of-2 samples
    let n = 1024.min(samples.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    // Apply Hann window and prepare complex input
    let mut buffer: Vec<Complex<f32>> = samples[..n]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - ((2.0 * PI * i as f32) / (n as f32 - 1.0)).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut buffer);

    // Find peak frequency (skip DC component)
    let mut max_magnitude = 0.0f32;
    let mut max_index = 0;

    for (i, complex) in buffer.iter().enumerate().skip(1).take(n / 2) {
        let magnitude = complex.norm();
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            max_index = i;
        }
    }

    if max_magnitude < 0.001 {
        return None;
    }

    let freq = (max_index as f32 * sample_rate) / n as f32;
    Some(freq)
}

/// Helper to render a pattern as a frequency pattern for an oscillator
fn render_note_pattern(pattern_str: &str) -> Vec<f32> {
    let mut graph = UnifiedSignalGraph::new(44100.0);
    graph.set_cps(2.0); // 2 cycles per second for testing

    // Parse the pattern
    let pattern = parse_mini_notation(pattern_str);

    // Create a Pattern node that will convert note names to frequencies
    let pattern_node = graph.add_node(SignalNode::Pattern {
        pattern_str: pattern_str.to_string(),
        pattern,
        last_value: 440.0,
        last_trigger_time: -1.0,
    });

    // Create oscillator controlled by the pattern
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(pattern_node),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0, 
    });

    // Scale down amplitude
    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Value(0.2),
    });

    graph.set_output(scaled);

    // Render one cycle (0.5 seconds at 2 CPS)
    graph.render(22050)
}

#[test]
fn test_note_to_midi_conversion() {
    // Basic note names
    assert_eq!(note_to_midi("c4"), Some(60));
    assert_eq!(note_to_midi("a4"), Some(69));
    assert_eq!(note_to_midi("c#4"), Some(61));
    assert_eq!(note_to_midi("cs4"), Some(61));
    assert_eq!(note_to_midi("df4"), Some(61)); // D-flat = C# enharmonic
    assert_eq!(note_to_midi("d4"), Some(62));
    assert_eq!(note_to_midi("e4"), Some(64));
    assert_eq!(note_to_midi("f4"), Some(65));
    assert_eq!(note_to_midi("g4"), Some(67));
    assert_eq!(note_to_midi("b4"), Some(71));

    // Different octaves
    assert_eq!(note_to_midi("c0"), Some(12));
    assert_eq!(note_to_midi("c3"), Some(48));
    assert_eq!(note_to_midi("c5"), Some(72));
    assert_eq!(note_to_midi("c6"), Some(84));

    // Frequency conversions
    let a4_freq = midi_to_freq(69);
    assert!((a4_freq - 440.0).abs() < 0.1, "A4 should be 440 Hz");

    let c4_freq = midi_to_freq(60);
    assert!((c4_freq - 261.63).abs() < 1.0, "C4 should be ~261.63 Hz");
}

#[test]
fn test_single_note_audio_c4() {
    // Test that c4 produces the correct frequency (261.63 Hz)
    let buffer = render_note_pattern("c4");

    // Detect frequency
    let detected = detect_frequency(&buffer, 44100.0);
    assert!(detected.is_some(), "Should detect a frequency");

    let freq = detected.unwrap();
    let expected = midi_to_freq(60) as f32; // C4 = 261.63 Hz

    println!("C4: Expected {:.2} Hz, Got {:.2} Hz", expected, freq);
    assert!(
        (freq - expected).abs() < 15.0,
        "C4 frequency should be ~{:.2} Hz, got {:.2} Hz (tolerance: 15 Hz for FFT resolution)",
        expected,
        freq
    );
}

#[test]
fn test_single_note_audio_a4() {
    // Test that a4 produces 440 Hz
    let buffer = render_note_pattern("a4");

    let detected = detect_frequency(&buffer, 44100.0);
    assert!(detected.is_some(), "Should detect a frequency");

    let freq = detected.unwrap();
    let expected = 440.0; // A4 = 440 Hz

    println!("A4: Expected {:.2} Hz, Got {:.2} Hz", expected, freq);
    assert!(
        (freq - expected).abs() < 15.0,
        "A4 frequency should be ~{:.2} Hz, got {:.2} Hz (tolerance: 15 Hz for FFT resolution)",
        expected,
        freq
    );
}

#[test]
fn test_sharp_note_cs4() {
    // Test that c#4 produces the correct frequency
    let buffer = render_note_pattern("cs4");

    let detected = detect_frequency(&buffer, 44100.0);
    assert!(detected.is_some(), "Should detect a frequency");

    let freq = detected.unwrap();
    let expected = midi_to_freq(61) as f32; // C#4 = 277.18 Hz

    println!("C#4: Expected {:.2} Hz, Got {:.2} Hz", expected, freq);
    // Note: Detected frequency may be off due to FFT bin quantization
    // and pattern evaluation timing. 25 Hz tolerance allows for these factors.
    assert!(
        (freq - expected).abs() < 25.0,
        "C#4 frequency should be ~{:.2} Hz, got {:.2} Hz (tolerance: 25 Hz)",
        expected,
        freq
    );
}

#[test]
fn test_octave_c3_vs_c4() {
    // Test that c3 is exactly one octave below c4
    let buffer_c3 = render_note_pattern("c3");
    let buffer_c4 = render_note_pattern("c4");

    let freq_c3 = detect_frequency(&buffer_c3, 44100.0).expect("Should detect C3");
    let freq_c4 = detect_frequency(&buffer_c4, 44100.0).expect("Should detect C4");

    println!("C3: {:.2} Hz, C4: {:.2} Hz", freq_c3, freq_c4);

    // C4 should be approximately 2x C3
    let ratio = freq_c4 / freq_c3;
    assert!(
        (ratio - 2.0).abs() < 0.05,
        "C4 should be one octave (2x) above C3, got ratio {:.3}",
        ratio
    );
}

#[test]
#[ignore] // Enable this once pattern iteration is working with note conversion
fn test_note_sequence() {
    // Test a sequence of notes: c4 e4 g4 (C major chord as melody)
    let buffer = render_note_pattern("c4 e4 g4");

    // We should detect the presence of each note in different parts of the buffer
    // For now, just verify the buffer has non-zero content
    let rms: f32 = buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32;
    let rms = rms.sqrt();

    println!("Note sequence RMS: {:.4}", rms);
    assert!(rms > 0.01, "Should have audible content");
}

#[test]
#[ignore] // Enable once UnifiedSignalGraph can handle note names
fn test_cli_note_parsing() {
    // Test that the CLI parser can handle note names in oscillator contexts
    // This will require updating the CLI parser to recognize note names

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Simulate parsing: sine "c4 e4 g4"
    // For now, we just verify the conversion function works
    let c4_freq = midi_to_freq(60);
    let e4_freq = midi_to_freq(64);
    let g4_freq = midi_to_freq(67);

    assert!((c4_freq - 261.63).abs() < 1.0);
    assert!((e4_freq - 329.63).abs() < 1.0);
    assert!((g4_freq - 392.00).abs() < 1.0);
}

#[test]
fn test_note_name_robustness() {
    // Test various note name formats
    assert_eq!(note_to_midi("C4"), Some(60)); // Uppercase
    assert_eq!(note_to_midi("c4"), Some(60)); // Lowercase
    assert_eq!(note_to_midi("c#4"), Some(61)); // Sharp
    assert_eq!(note_to_midi("cs4"), Some(61)); // Sharp (s notation)
    assert_eq!(note_to_midi("df4"), Some(61)); // Flat (enharmonic)
    assert_eq!(note_to_midi("df4"), Some(61)); // Flat (f notation)

    // Default octave (should be 4)
    assert_eq!(note_to_midi("c"), Some(60));
    assert_eq!(note_to_midi("a"), Some(69));
}

#[test]
fn test_frequency_accuracy_tolerance() {
    // Verify that our FFT detection is accurate enough
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Generate a pure 440 Hz sine wave
    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Value(440.0),
        waveform: Waveform::Sine,
        phase: 0.0,
        pending_freq: None,
        last_sample: 0.0, 
    });

    let scaled = graph.add_node(SignalNode::Multiply {
        a: Signal::Node(osc),
        b: Signal::Value(0.2),
    });

    graph.set_output(scaled);

    let buffer = graph.render(44100); // 1 second

    let detected = detect_frequency(&buffer, 44100.0);
    assert!(detected.is_some());

    let freq = detected.unwrap();
    println!("Pure 440 Hz test: detected {:.2} Hz", freq);
    // FFT resolution with 1024 samples at 44.1kHz = ~43 Hz per bin
    // Tolerance of 15 Hz allows for bin quantization error
    assert!(
        (freq - 440.0).abs() < 15.0,
        "FFT should detect ~440 Hz (got {:.2} Hz) - tolerance accounts for 43 Hz bin resolution",
        freq
    );
}

#[test]
#[ignore] // Enable once scale quantization is integrated
fn test_scale_quantization() {
    // Test scale("major") with numeric degrees
    // Input: 0 1 2 3 4 5 6 7 (scale degrees)
    // Output in C major: C D E F G A B C (60 62 64 65 67 69 71 72)

    // This will require implementing scale quantization in the pattern system
    // and integrating it with UnifiedSignalGraph
}

#[test]
#[ignore] // Enable once chord parsing is integrated
fn test_chord_notation() {
    // Test chord notation: c4'maj should produce C E G (60 64 67)
    // This requires implementing chord expansion in the pattern system
}
