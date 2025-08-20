//! Complete Phonon Demo - Patterns + Synthesis
//! 
//! This demonstrates the full power of Phonon:
//! Strudel-style patterns combined with modular synthesis!

use phonon::pattern::{Pattern, State, TimeSpan, Fraction};
use phonon::enhanced_parser::EnhancedParser;
use phonon::signal_executor::SignalExecutor;
use phonon::render::{RenderConfig, Renderer};
use std::collections::HashMap;
use std::path::Path;

fn main() {
    println!("🎵 Phonon Complete System Demo");
    println!("==============================\n");
    
    // Demo 1: Pure pattern operations
    demo_patterns();
    
    // Demo 2: Synthesis DSL
    demo_synthesis();
    
    // Demo 3: Integrated pattern + synthesis (the future!)
    demo_integrated();
    
    println!("\n✨ Phonon: The future of live coding!");
}

fn demo_patterns() {
    println!("📊 Pattern System (Rust implementation of Strudel)");
    println!("--------------------------------------------------");
    
    // Create a rhythm pattern
    let kick_pattern = Pattern::from_string("1 0 1 0");
    let snare_pattern = Pattern::from_string("0 1 0 1");
    
    // Stack them
    let drum_pattern = Pattern::stack(vec![kick_pattern, snare_pattern]);
    
    // Query for one cycle
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events = drum_pattern.query(&state);
    println!("  Drum pattern events in first cycle:");
    for event in events {
        println!("    {:?} at {:.2}", event.value, event.part.begin.to_float());
    }
    
    // Demonstrate pattern transformations
    let melody = Pattern::from_string("c4 e4 g4 c5")
        .fast(2.0)
        .every(4, |p| p.rev());
    
    println!("\n  Melody pattern (fast(2).every(4, rev)):");
    let melody_events = melody.query(&state);
    for event in melody_events.iter().take(4) {
        println!("    {} at {:.2}", event.value, event.part.begin.to_float());
    }
    
    // Euclidean rhythm
    let euclid = Pattern::<bool>::euclid(5, 8, 0);
    let euclid_events = euclid.query(&state);
    print!("  Euclidean(5,8): ");
    for event in euclid_events {
        if event.value {
            print!("● ");
        }
    }
    println!("\n");
}

fn demo_synthesis() {
    println!("🎛️  Synthesis DSL");
    println!("----------------");
    
    let dsl = r#"
~lfo: sine(2) * 100
~carrier: sine(440 + ~lfo)
~filtered: ~carrier >> lpf(1000, 0.7)
out: ~filtered * 0.5
"#;
    
    println!("  DSL Code:");
    for line in dsl.lines() {
        if !line.trim().is_empty() {
            println!("    {}", line);
        }
    }
    
    // Parse and render
    let config = RenderConfig {
        duration: 0.5,
        sample_rate: 44100,
        ..Default::default()
    };
    
    let renderer = Renderer::new(config);
    let output_path = Path::new("/tmp/phonon_synthesis_demo.wav");
    
    match renderer.render_to_file(dsl, output_path) {
        Ok(stats) => {
            println!("\n  ✓ Rendered synthesis:");
            println!("    Duration: {:.2}s", stats.duration);
            println!("    RMS: {:.3}", stats.rms);
            println!("    Peak: {:.3}", stats.peak);
        }
        Err(e) => {
            println!("  ❌ Render failed: {}", e);
        }
    }
}

fn demo_integrated() {
    println!("\n🚀 Integrated Pattern + Synthesis (The Vision!)");
    println!("-----------------------------------------------");
    
    println!("  Imagine writing this in Phonon:");
    println!();
    
    let future_code = r#"
// Rhythm pattern drives filter cutoff!
~rhythm: "1 0 1 0".fast(2)
~cutoff: ~rhythm * 1500 + 500

// Melody pattern
~notes: "c3 e3 g3 c4".slow(2).note()

// Synthesis with pattern modulation
~bass: saw(~notes) >> lpf(~cutoff, 2.0)
~lead: sine(~notes * 2) * 0.3

// Mix with effects
~verb: ~lead >> reverb(0.3, 0.7)
out: ~bass * 0.4 + ~verb * 0.3
"#;
    
    for line in future_code.lines() {
        if !line.trim().is_empty() {
            println!("    {}", line.trim());
        }
    }
    
    println!("\n  This would combine:");
    println!("    • Tidal/Strudel pattern operations");
    println!("    • Real-time synthesis");
    println!("    • Cross-modulation between patterns and audio");
    println!("    • All in pure Rust for maximum performance!");
    
    // Actually try to render a simple integrated example
    test_simple_integration();
}

fn test_simple_integration() {
    println!("\n  Testing basic integration:");
    
    // Create a pattern that generates frequencies
    let note_pattern = Pattern::from_string("220 330 440 330");
    
    // Query the pattern
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events = note_pattern.query(&state);
    
    // For each event, generate synthesis
    for (i, event) in events.iter().enumerate() {
        // Parse frequency from event
        let freq: f64 = event.value.parse().unwrap_or(440.0);
        
        // Generate DSL for this note
        let dsl = format!(r#"
~osc: sine({})
out: ~osc * 0.3
"#, freq);
        
        // Render it
        let config = RenderConfig {
            duration: 0.25, // Each note is 1/4 second
            sample_rate: 44100,
            fade_in: 0.01,
            fade_out: 0.01,
            ..Default::default()
        };
        
        let renderer = Renderer::new(config);
        let output_path = format!("/tmp/phonon_note_{}.wav", i);
        
        if let Ok(_) = renderer.render_to_file(&dsl, Path::new(&output_path)) {
            print!("♪ ");
        }
    }
    println!("\n  ✓ Generated 4 note files in /tmp/");
}