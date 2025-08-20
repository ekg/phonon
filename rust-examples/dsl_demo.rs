//! Demo of the Phonon Modular Synthesis DSL
//! 
//! Run with: cargo run --example dsl_demo

use fermion::enhanced_parser::EnhancedParser;
use fermion::signal_executor::SignalExecutor;
use fermion::engine::AudioEngine;
use std::time::Duration;
use std::thread;
use std::io::Write;

fn main() {
    println!("🎵 Phonon Modular Synthesis DSL - Live Demo 🎵\n");
    println!("{}", "=".repeat(50));
    
    // Initialize audio engine for live playback
    let engine = AudioEngine::new().expect("Failed to initialize audio engine");
    println!("✓ Audio engine initialized\n");
    
    demo_simple_oscillator();
    demo_lfo_modulation();
    demo_filter_sweep();
    demo_mixed_signals();
    demo_live_melody(&engine);
    
    println!("\n{}", "=".repeat(50));
    println!("✨ Demo Complete!");
    println!("\nThe DSL supports:");
    println!("  • Signal routing with ~buses");
    println!("  • Arithmetic operations (+, -, *, /)");
    println!("  • Effect chains with >>");
    println!("  • Oscillators: sine, saw, square");
    println!("  • Filters: lpf, hpf");
    println!("  • Live audio playback");
    println!("\n🎵 Ready for live coding! 🎵");
}

fn demo_simple_oscillator() {
    println!("📻 Demo 1: Simple Sine Wave");
    println!("{}", "-".repeat(30));
    
    let patch = r#"
~osc: sine(440)
out: ~osc * 0.3
"#;
    
    println!("Patch:");
    println!("{}", patch);
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(patch).expect("Failed to parse");
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    let output = executor.process_block().expect("Failed to process");
    analyze_audio(&output.data, "440Hz sine wave");
    
    // Save to file
    output.write_wav("/tmp/demo_sine.wav").expect("Failed to write WAV");
    println!("  → Saved to /tmp/demo_sine.wav");
    
    thread::sleep(Duration::from_millis(500));
}

fn demo_lfo_modulation() {
    println!("\n🌊 Demo 2: LFO Modulation");
    println!("{}", "-".repeat(30));
    
    let patch = r#"
~lfo: sine(5)
~carrier: sine(440 + ~lfo * 20)
out: ~carrier * 0.3
"#;
    
    println!("Vibrato patch:");
    println!("{}", patch);
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(patch).expect("Failed to parse");
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    // Process multiple blocks to show modulation
    println!("\nProcessing with vibrato:");
    let mut all_samples = Vec::new();
    for i in 0..5 {
        let output = executor.process_block().expect("Failed to process");
        print!("  Block {}: ", i + 1);
        analyze_audio(&output.data, "");
        all_samples.extend_from_slice(&output.data);
    }
    
    // Save combined output
    write_wav("/tmp/demo_vibrato.wav", &all_samples, 44100);
    println!("  → Saved to /tmp/demo_vibrato.wav");
    
    thread::sleep(Duration::from_millis(500));
}

fn demo_filter_sweep() {
    println!("\n🔄 Demo 3: Filter Sweep");
    println!("{}", "-".repeat(30));
    
    let patch = r#"
~osc: saw(110)
~filtered: ~osc >> lpf(500, 0.7)
out: ~filtered * 0.3
"#;
    
    println!("Filtered saw wave:");
    println!("{}", patch);
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(patch).expect("Failed to parse");
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    let output = executor.process_block().expect("Failed to process");
    analyze_audio(&output.data, "Filtered saw");
    
    output.write_wav("/tmp/demo_filtered.wav").expect("Failed to write WAV");
    println!("  → Saved to /tmp/demo_filtered.wav");
    
    thread::sleep(Duration::from_millis(500));
}

fn demo_mixed_signals() {
    println!("\n🎛️  Demo 4: Mixed Signals");
    println!("{}", "-".repeat(30));
    
    let patch = r#"
~osc1: sine(440)
~osc2: saw(220)
~mixed: ~osc1 * 0.3 + ~osc2 * 0.2
out: ~mixed
"#;
    
    println!("Mixed signals patch:");
    println!("{}", patch);
    
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(patch).expect("Failed to parse");
    
    println!("  Nodes: {}, Connections: {}", graph.nodes.len(), graph.connections.len());
    
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize().expect("Failed to initialize");
    
    let output = executor.process_block().expect("Failed to process");
    analyze_audio(&output.data, "Mixed sine+saw");
    
    output.write_wav("/tmp/demo_mixed.wav").expect("Failed to write WAV");
    println!("  → Saved to /tmp/demo_mixed.wav");
    
    thread::sleep(Duration::from_millis(500));
}

fn demo_live_melody(engine: &AudioEngine) {
    println!("\n🎼 Demo 5: Live Melody Playback");
    println!("{}", "-".repeat(30));
    println!("Playing a simple melody through the audio engine...");
    
    let notes = vec![
        (440.0, 0.25),  // A4
        (494.0, 0.25),  // B4  
        (523.0, 0.25),  // C5
        (587.0, 0.25),  // D5
        (659.0, 0.25),  // E5
        (587.0, 0.25),  // D5
        (523.0, 0.25),  // C5
        (440.0, 0.5),   // A4
    ];
    
    for (freq, duration) in notes {
        // Generate a note with the DSL
        let patch = format!(r#"
~osc: sine({})
out: ~osc * 0.3
"#, freq);
        
        let mut parser = EnhancedParser::new(44100.0);
        let graph = parser.parse(&patch).expect("Failed to parse");
        
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().expect("Failed to initialize");
        
        // Generate samples for this note
        let blocks = ((duration * 44100.0 / 512.0) as usize).max(1);
        let mut samples = Vec::new();
        for _ in 0..blocks {
            let output = executor.process_block().expect("Failed to process");
            samples.extend_from_slice(&output.data);
        }
        
        // Play through audio engine
        engine.play_synth(samples, 0.5);
        print!("♪ ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        thread::sleep(Duration::from_secs_f32(duration));
    }
    println!("\n  ✓ Melody complete!");
}

fn write_wav(filename: &str, samples: &[f32], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut writer = hound::WavWriter::create(filename, spec).expect("Failed to create WAV");
    
    for &sample in samples {
        let s = (sample * 32767.0) as i16;
        writer.write_sample(s).expect("Failed to write sample");
    }
    
    writer.finalize().expect("Failed to finalize WAV");
}

fn analyze_audio(audio: &[f32], label: &str) {
    if audio.is_empty() {
        println!("No audio generated");
        return;
    }
    
    // Calculate RMS
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    
    // Calculate peak
    let peak = audio.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    
    // Count zero crossings (rough frequency indicator)
    let mut zero_crossings = 0;
    for i in 1..audio.len() {
        if (audio[i-1] >= 0.0) != (audio[i] >= 0.0) {
            zero_crossings += 1;
        }
    }
    
    // Estimate frequency from zero crossings
    let estimated_freq = (zero_crossings as f32 * 44100.0) / (2.0 * audio.len() as f32);
    
    // Create a simple ASCII visualization
    let viz_width = 30;
    let mut viz = String::new();
    let step = audio.len() / viz_width;
    for i in 0..viz_width {
        let sample = audio.get(i * step).unwrap_or(&0.0);
        let level = (sample.abs() * 5.0).min(4.0) as usize;
        viz.push(match level {
            0 => '·',
            1 => '▁',
            2 => '▃',
            3 => '▅',
            4 => '█',
            _ => ' ',
        });
    }
    
    if label.is_empty() {
        println!("RMS: {:.3}, Peak: {:.3}, ~{}Hz [{}]", 
                 rms, peak, estimated_freq as i32, viz);
    } else {
        println!("{}: RMS: {:.3}, Peak: {:.3} [{}]", 
                 label, rms, peak, viz);
    }
}