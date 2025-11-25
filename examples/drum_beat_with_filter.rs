//! Example: Create a drum beat and filter it with DSP
//!
//! Demonstrates creating a "bd*4 cp" type beat and applying DSP effects

use phonon::mini_notation::parse_mini_notation;
use phonon::pattern::{Fraction, State, TimeSpan};
use phonon::simple_dsp_executor::render_dsp_to_audio_simple;
use std::collections::HashMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🎵 Drum Beat with DSP Filtering Demo\n");

    // Example 1: Basic kick pattern with low-pass filter sweep
    println!("1. Four-on-the-floor kick with filter sweep:");
    let code1 = r#"
        ~kick: impulse 4 >> mul 50 >> lpf 80 0.9 >> mul 2
        ~lfo: sin 0.5 >> mul 400 >> add 600
        out: ~kick >> lpf ~lfo 0.7
    "#;

    render_and_save(code1, "drum_beat_kick_filtered.wav", 4.0)?;

    // Example 2: Kick and clap pattern with filtering
    println!("\n2. Kick and clap with high-pass filtered clap:");
    let code2 = r#"
        ~kick: impulse 2 >> mul 60 >> lpf 100 0.8 >> mul 1.5
        ~clap: noise >> env 0.001 0.01 0.0 0.02 >> hpf 2000 0.9
        ~clap_trig: impulse 1 >> delay 0.25 0.0
        ~clap_sound: ~clap_trig * ~clap
        out: ~kick + ~clap_sound
    "#;

    render_and_save(code2, "drum_beat_kick_clap.wav", 4.0)?;

    // Example 3: More complex beat with multiple elements
    println!("\n3. Complex beat with kick, snare, and hi-hats:");
    let code3 = r#"
        ~kick: impulse 2 >> mul 80 >> lpf 120 0.85 >> mul 2
        ~snare: noise >> env 0.001 0.05 0.0 0.08 >> hpf 500 0.7 >> lpf 8000 0.6
        ~snare_trig: impulse 1 >> delay 0.5 0.0
        ~hihat: noise >> env 0.001 0.02 0.0 0.01 >> hpf 8000 0.9 >> mul 0.3
        ~hihat_trig: impulse 8
        ~drums: ~kick + (~snare_trig * ~snare) + (~hihat_trig * ~hihat)
        ~filter_lfo: sin 0.25 >> mul 2000 >> add 3000
        out: ~drums >> lpf ~filter_lfo 0.5
    "#;

    render_and_save(code3, "drum_beat_complex.wav", 4.0)?;

    // Example 4: Using pattern-inspired rhythm with DSP
    println!("\n4. Euclidean rhythm pattern with DSP:");
    let code4 = r#"
        ~trig1: impulse 3
        ~trig2: impulse 5
        ~bass: ~trig1 >> mul 50 >> saw 55 >> lpf 200 0.9
        ~perc: ~trig2 >> mul 30 >> triangle 220 >> env 0.001 0.05 0.0 0.02
        ~reverb_send: ~perc >> delay 0.1 0.4 >> lpf 2000 0.5
        out: ~bass * 0.7 + ~perc * 0.5 + ~reverb_send * 0.3
    "#;

    render_and_save(code4, "drum_beat_euclidean.wav", 4.0)?;

    // Example 5: Techno-style beat with sidechain compression effect
    println!("\n5. Techno beat with sidechain-style ducking:");
    let code5 = r#"
        ~kick: impulse 2 >> mul 100 >> lpf 150 0.9 >> mul 2
        ~kick_env: impulse 2 >> env 0.01 0.15 0.0 0.0
        ~bass: saw 55 >> lpf 800 0.7
        ~ducked_bass: ~bass * (1.0 - ~kick_env * 0.8)
        ~noise: noise >> hpf 10000 0.9 >> mul 0.1
        out: ~kick * 0.8 + ~ducked_bass * 0.6 + ~noise
    "#;

    render_and_save(code5, "drum_beat_techno_sidechain.wav", 4.0)?;

    println!("\n✅ All drum patterns generated successfully!");
    println!("📁 Check /tmp/ directory for WAV files");

    // Bonus: Show how to use pattern notation (even though we can't execute it yet)
    demonstrate_pattern_notation();

    Ok(())
}

fn render_and_save(code: &str, filename: &str, duration: f32) -> Result<(), Box<dyn Error>> {
    println!("  Generating: {}", filename);
    println!("  Code: {}", code.trim());

    let buffer = render_dsp_to_audio_simple(code, 44100.0, duration)?;
    let path = format!("/tmp/{}", filename);
    buffer.write_wav(&path)?;

    println!("  ✓ Saved to: {}", path);
    println!("  Peak: {:.3}, RMS: {:.3}", buffer.peak(), buffer.rms());

    Ok(())
}

fn demonstrate_pattern_notation() {
    println!("\n📝 Pattern Notation Examples (for reference):");
    println!("These show how you'd write patterns in mini-notation:");

    // Parse some example patterns
    let patterns = vec![
        ("bd*4", "Four kicks per cycle"),
        ("bd*4 cp", "Four kicks with clap on beat 2 and 4"),
        ("bd*4 [~ cp]", "Kicks with clap on off-beats"),
        ("bd*4 [~ cp] hh*8", "Full drum pattern"),
        ("bd(3,8)", "Euclidean rhythm: 3 hits in 8 steps"),
        ("bd(5,8) cp(3,8)", "Polyrhythmic pattern"),
    ];

    for (pattern_str, description) in patterns {
        println!("\n  Pattern: \"{}\"", pattern_str);
        println!("  Description: {}", description);

        // Parse and show the pattern structure
        let pattern = parse_mini_notation(pattern_str);
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let events = pattern.query(&state);
        println!("  Events in first cycle: {}", events.len());

        // Show timing of first few events
        for (i, event) in events.iter().take(4).enumerate() {
            println!(
                "    Event {}: {} at time {:.3}",
                i + 1,
                event.value,
                event.part.begin.to_float()
            );
        }
    }

    println!("\n💡 To combine patterns with DSP:");
    println!("   1. Generate triggers from patterns (impulse trains)");
    println!("   2. Use triggers to gate synthesized sounds");
    println!("   3. Apply DSP effects to the mixed output");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drum_generation() {
        // Test that we can generate a simple drum pattern
        let code = "out: impulse 4 >> mul 50 >> lpf 100 0.9";
        let buffer = render_dsp_to_audio_simple(code, 44100.0, 0.5).unwrap();
        assert!(buffer.peak() > 0.0);
        assert!(buffer.rms() > 0.0);
    }

    #[test]
    fn test_pattern_parsing() {
        let pattern = parse_mini_notation("bd*4 cp");
        let state = State {
            span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
            controls: HashMap::new(),
        };
        let events = pattern.query(&state);

        // Should have 5 events: 4 bd + 1 cp
        assert!(events.len() >= 4);
    }
}
