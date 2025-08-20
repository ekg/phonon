//! Test actual audio generation
//! Run with: cargo run --example test_audio

use fermion::enhanced_parser::EnhancedParser;
use fermion::signal_executor::SignalExecutor;
use std::fs;

fn main() {
    println!("Testing audio generation...\n");
    
    // Test 1: Simple sine wave
    test_sine_wave();
    
    // Test 2: Filtered saw wave
    test_filtered_saw();
    
    // Test 3: Mixed signals
    test_mixed_signals();
}

fn test_sine_wave() {
    println!("Test 1: Sine wave at 440 Hz");
    
    let dsl = r#"
~osc: sine(440)
out: ~osc * 0.3
"#;
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(dsl).expect("Failed to parse");
    
    println!("  Nodes created: {}", graph.nodes.len());
    println!("  Connections: {}", graph.connections.len());
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    let output = executor.process_block().expect("Failed to process");
    
    println!("  RMS: {:.3}", output.rms());
    println!("  Peak: {:.3}", output.peak());
    
    // Write to WAV file
    let wav_path = "/tmp/test_sine.wav";
    output.write_wav(wav_path).expect("Failed to write WAV");
    println!("  Wrote WAV to: {}", wav_path);
    
    // Analyze the WAV
    let data = fs::read(wav_path).expect("Failed to read WAV");
    println!("  WAV file size: {} bytes", data.len());
    
    assert!(output.peak() > 0.0, "Should generate audio!");
    println!("  ✅ PASSED\n");
}

fn test_filtered_saw() {
    println!("Test 2: Filtered saw wave");
    
    let dsl = r#"
~osc: saw(110)
~filtered: ~osc >> lpf(500, 0.7)
out: ~filtered * 0.3
"#;
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(dsl).expect("Failed to parse");
    
    println!("  Nodes created: {}", graph.nodes.len());
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    let output = executor.process_block().expect("Failed to process");
    
    println!("  RMS: {:.3}", output.rms());
    println!("  Peak: {:.3}", output.peak());
    
    let wav_path = "/tmp/test_filtered_saw.wav";
    output.write_wav(wav_path).expect("Failed to write WAV");
    println!("  Wrote WAV to: {}", wav_path);
    
    assert!(output.peak() > 0.0, "Should generate audio!");
    println!("  ✅ PASSED\n");
}

fn test_mixed_signals() {
    println!("Test 3: Mixed signals");
    
    let dsl = r#"
~osc1: sine(440)
~osc2: saw(220)
~mixed: ~osc1 * 0.3 + ~osc2 * 0.2
out: ~mixed
"#;
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(dsl).expect("Failed to parse");
    
    println!("  Nodes created: {}", graph.nodes.len());
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    let output = executor.process_block().expect("Failed to process");
    
    println!("  RMS: {:.3}", output.rms());
    println!("  Peak: {:.3}", output.peak());
    
    let wav_path = "/tmp/test_mixed.wav";
    output.write_wav(wav_path).expect("Failed to write WAV");
    println!("  Wrote WAV to: {}", wav_path);
    
    assert!(output.peak() > 0.0, "Should generate audio!");
    println!("  ✅ PASSED\n");
}