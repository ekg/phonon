#!/usr/bin/env cargo +nightly -Zscript
//! Demo of the Phonon Modular Synthesis DSL
//! 
//! Run with: cargo run --bin demo

use fermion::dsl_osc_handler::DslOscHandler;
use fermion::enhanced_parser::EnhancedParser;
use fermion::signal_executor::AudioBuffer;
use rosc::{OscMessage, OscType};
use std::time::{Duration, Instant};
use std::thread;

fn main() {
    println!("🎵 Phonon Modular Synthesis DSL - Live Demo 🎵\n");
    println!("=" . repeat(50));
    
    // Create DSL handler
    let mut handler = DslOscHandler::new(44100.0, 512);
    
    // Demo 1: Simple oscillator
    println!("\n📻 Demo 1: Simple Sine Wave");
    println!("-" . repeat(30));
    
    let patch1 = r#"
~osc: sine(440)
out: ~osc * 0.3
"#;
    
    println!("Loading patch:");
    println!("{}", patch1);
    
    let msg = OscMessage {
        addr: "/dsl/load".to_string(),
        args: vec![OscType::String(patch1.to_string())],
    };
    
    handler.handle_message(&msg).unwrap();
    
    // Process and analyze audio
    let audio = handler.process_block();
    analyze_audio(&audio, "Sine wave at 440 Hz");
    
    thread::sleep(Duration::from_millis(1000));
    
    // Demo 2: LFO Modulation
    println!("\n🌊 Demo 2: LFO Modulated Filter");
    println!("-" . repeat(30));
    
    let patch2 = r#"
~lfo: sine(0.5) * 0.5 + 0.5
~osc: saw(110)
~filtered: ~osc >> lpf(~lfo * 2000 + 500, 0.7)
out: ~filtered * 0.3
"#;
    
    println!("Loading patch:");
    println!("{}", patch2);
    
    let msg2 = OscMessage {
        addr: "/dsl/load".to_string(),
        args: vec![OscType::String(patch2.to_string())],
    };
    
    handler.handle_message(&msg2).unwrap();
    
    // Process multiple blocks to show modulation
    println!("\nProcessing with LFO modulation:");
    for i in 0..5 {
        let audio = handler.process_block();
        print!("  Block {}: ", i + 1);
        analyze_audio(&audio, "");
    }
    
    thread::sleep(Duration::from_millis(1000));
    
    // Demo 3: Cross-modulation
    println!("\n🔄 Demo 3: Cross-Modulation");
    println!("-" . repeat(30));
    
    let patch3 = r#"
// Bass synthesis
~bass_env: perc(0.01, 0.3)
~bass: saw(55) * ~bass_env >> lpf(800, 0.8)

// Extract bass level
~bass_rms: ~bass >> rms(0.05)

// Hi-hats modulated by bass level
~hats: "hh*8" >> hpf(~bass_rms * 5000 + 2000, 0.8) >> gain(0.3)

// Kick pattern
~kick: "bd ~ ~ bd"
~kick_transient: ~kick >> transient

// Sidechain compression
~bass_ducked: ~bass * (1 - ~kick_transient * 0.5)

// Mix
out: ~kick * 0.5 + ~bass_ducked * 0.3 + ~hats * 0.2
"#;
    
    println!("Loading cross-modulation patch:");
    println!("  - Bass ducked by kick (sidechain)");
    println!("  - Hi-hats filtered by bass level");
    println!("  - Pattern integration");
    
    let msg3 = OscMessage {
        addr: "/dsl/load".to_string(),
        args: vec![OscType::String(patch3.to_string())],
    };
    
    handler.handle_message(&msg3).unwrap();
    
    let audio = handler.process_block();
    analyze_audio(&audio, "Cross-modulated mix");
    
    thread::sleep(Duration::from_millis(1000));
    
    // Demo 4: Live Parameter Control
    println!("\n🎛️  Demo 4: Live Parameter Control");
    println!("-" . repeat(30));
    
    let patch4 = r#"
~mod: 0.0
~freq: 220 + ~mod * 220
~osc: saw(~freq)
~filtered: ~osc >> lpf(1000 + ~mod * 1000, 0.7)
out: ~filtered * 0.3
"#;
    
    println!("Loading controllable patch:");
    println!("{}", patch4);
    
    let msg4 = OscMessage {
        addr: "/dsl/load".to_string(),
        args: vec![OscType::String(patch4.to_string())],
    };
    
    handler.handle_message(&msg4).unwrap();
    
    println!("\nModulating parameters live:");
    for i in 0..5 {
        let value = i as f32 / 4.0;
        
        let set_msg = OscMessage {
            addr: "/dsl/bus/set".to_string(),
            args: vec![
                OscType::String("~mod".to_string()),
                OscType::Float(value),
            ],
        };
        
        handler.handle_message(&set_msg).unwrap();
        
        let audio = handler.process_block();
        print!("  ~mod = {:.2}: ", value);
        analyze_audio(&audio, &format!("Freq: {} Hz", 220.0 + value * 220.0));
    }
    
    thread::sleep(Duration::from_millis(1000));
    
    // Demo 5: Complex Modular Patch
    println!("\n🎼 Demo 5: Complete Modular Synthesis");
    println!("-" . repeat(30));
    
    let patch5 = r#"
// === LFOs ===
~lfo_slow: sine(0.25) * 0.5 + 0.5
~lfo_fast: sine(6) * 0.3

// === Bass ===
~bass_env: perc(0.01, 0.3)
~bass_osc: saw(55) * ~bass_env
~bass: ~bass_osc >> lpf(~lfo_slow * 2000 + 500, 0.8)

// === Lead ===
~lead_freq: 440 + ~lfo_fast * 20
~lead: square(~lead_freq) * 0.2
~lead_delayed: ~lead >> delay(0.375) >> lpf(3000, 0.5)

// === Drums ===
~kick: "bd ~ ~ bd" >> gain(1.0)
~snare: "~ sn ~ sn" >> gain(0.7)
~hats: "hh*16" >> gain(0.3)

// === Effects ===
~reverb_send: (~lead * 0.3) + (~lead_delayed * 0.5)
~reverb: ~reverb_send >> reverb(0.7, 0.8)

// === Master Mix ===
~drums: ~kick * 0.5 + ~snare * 0.3 + ~hats * 0.2
~mix: ~bass * 0.3 + ~drums * 0.4 + ~lead * 0.2 + ~reverb * 0.1
~master: ~mix >> compress(0.3, 4) >> limit(0.95)

out: ~master
"#;
    
    println!("Loading complete modular patch with:");
    println!("  ✓ Multiple LFOs");
    println!("  ✓ Bass synthesis with envelope");
    println!("  ✓ Lead with delay");
    println!("  ✓ Drum patterns");
    println!("  ✓ Reverb send");
    println!("  ✓ Master compression");
    
    let msg5 = OscMessage {
        addr: "/dsl/load".to_string(),
        args: vec![OscType::String(patch5.to_string())],
    };
    
    handler.handle_message(&msg5).unwrap();
    
    println!("\nProcessing complete patch:");
    for i in 0..3 {
        let audio = handler.process_block();
        print!("  Frame {}: ", i + 1);
        analyze_audio(&audio, "Full mix");
    }
    
    println!("\n" + &"=" . repeat(50));
    println!("✨ Demo Complete!");
    println!("\nThe DSL supports:");
    println!("  • Signal routing with ~buses");
    println!("  • Arithmetic operations (+, -, *, /)");
    println!("  • Effect chains with >>");
    println!("  • Pattern integration");
    println!("  • Cross-modulation");
    println!("  • Live parameter control");
    println!("  • Audio analysis (RMS, pitch, transients)");
    println!("  • Hot-swappable patches");
    println!("\n🎵 Ready for live coding! 🎵");
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

// Make this a standalone demo program
#[cfg(not(test))]
fn main() {
    // The actual main is above
}