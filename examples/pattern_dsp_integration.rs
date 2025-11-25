//! Example demonstrating advanced Pattern-DSP integration

use phonon::glicol_pattern_bridge::{parse_enhanced, PatternDspEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎹 Phonon Pattern-DSP Integration Demo");
    println!("=======================================\n");

    // Create pattern-DSP engine
    let mut engine = PatternDspEngine::new(120.0);

    println!("1️⃣  Pure Patterns");
    println!("   ---------------");
    engine.parse_hybrid("bd*4 [~ cp] hh*8")?;
    println!("   ✓ Loaded drum pattern: bd*4 [~ cp] hh*8");

    engine.parse_hybrid("c4 e4 g4 c5")?;
    println!("   ✓ Loaded melody: c4 e4 g4 c5");

    println!("\n2️⃣  Pattern-Triggered Synthesis");
    println!("   ----------------------------");
    engine.create_voice("bass", "c2 ~ e2 g2", "saw(55) >> lpf(1000, 0.8)")?;
    println!("   ✓ Created bass voice with saw wave + filter");

    engine.create_voice(
        "lead",
        "c4 e4 g4 b4",
        "square(440) >> lpf(2000, 0.5) >> delay(0.25, 0.3)",
    )?;
    println!("   ✓ Created lead voice with square wave + delay");

    println!("\n3️⃣  Pattern Modulation");
    println!("   -------------------");
    engine.add_modulation("bass", "cutoff", "0.2 0.5 0.8 1.0", (200.0, 2000.0))?;
    println!("   ✓ Added cutoff modulation to bass: 0.2 0.5 0.8 1.0");

    println!("\n4️⃣  Pattern Processing");
    println!("   -------------------");
    engine.parse_hybrid("bd*4 >> reverb(0.3)")?;
    println!("   ✓ Drums through reverb: bd*4 >> reverb(0.3)");

    engine.parse_hybrid("hh*16 >> hpf(5000, 0.8)")?;
    println!("   ✓ Hi-hats through high-pass: hh*16 >> hpf(5000, 0.8)");

    println!("\n5️⃣  Control Routing");
    println!("   ----------------");
    engine.parse_hybrid("0 0.25 0.5 0.75 1 >> ~lfo")?;
    println!("   ✓ Pattern to LFO control: 0 0.25 0.5 0.75 1 >> ~lfo");

    println!("\n6️⃣  Query Pattern Events");
    println!("   ----------------------");

    for beat in 0..4 {
        let events = engine.query(beat as f64);
        if !events.is_empty() {
            println!("   Beat {}: ", beat);
            for (name, values) in events {
                println!("      {}: {:?}", name, values);
            }
        }
    }

    println!("\n7️⃣  Advanced Notation Examples");
    println!("   ---------------------------");

    // Parse enhanced notation
    let examples = vec![
        ("Embedded synth", "bd [sine(440):0.1] cp"),
        ("Pattern with scale", "{c4 e4 g4}'maj"),
        ("Euclidean + effect", "bd(3,8) >> delay(0.1, 0.5)"),
        ("Polyrhythm + filter", "{bd*4, cp*3} >> lpf(800, 0.7)"),
        ("Alternation + reverb", "<bd sn> hh >> reverb(0.4)"),
        ("Speed + distortion", "bd*4 . fast(2) >> distortion(2.0)"),
    ];

    for (desc, pattern) in examples {
        match parse_enhanced(pattern) {
            Ok(_) => println!("   ✓ {}: {}", desc, pattern),
            Err(e) => println!("   ✗ {}: {} ({})", desc, pattern, e),
        }
    }

    println!("\n8️⃣  DSP Chain Syntax");
    println!("   -----------------");

    // Glicol-style DSP chains
    let dsp_examples = vec![
        "sine(440) >> mul(0.5)",
        "saw(110) >> lpf(500, 0.8) >> delay(0.25, 0.3)",
        "noise() >> hpf(2000, 0.7) >> reverb(0.5, 0.8)",
        "square(220) >> lpf(~lfo * 1000 + 500, 0.8)",
    ];

    for chain in dsp_examples {
        println!("   • {}", chain);
    }

    println!("\n9️⃣  Reference Chains (~)");
    println!("   --------------------");
    println!("   ~lfo: sine(0.5) * 0.5 + 0.5");
    println!("   ~env: adsr(0.01, 0.1, 0.7, 0.2)");
    println!("   ~bass: saw(55) >> lpf(~lfo * 2000 + 500, 0.8)");
    println!("   ~drums: sp(\"bd\") >> reverb(0.2, 0.5)");
    println!("   out: ~bass * 0.4 + ~drums * 0.6");

    println!("\n✅ Integration demo complete!");
    println!("\nKey Features:");
    println!("• Pattern sequences (TidalCycles-style)");
    println!("• DSP synthesis (Glicol-style)");
    println!("• Pattern-triggered synthesis");
    println!("• Pattern modulation of DSP parameters");
    println!("• DSP processing of pattern output");
    println!("• Unified notation for both paradigms");

    Ok(())
}
