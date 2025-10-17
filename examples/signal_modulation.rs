//! Signal modulation examples using Phonon
//!
//! Demonstrates various signal processing techniques:
//! - Ring modulation
//! - Amplitude modulation (AM)
//! - Frequency modulation (FM)
//! - Pattern gating
//! - Complex signal multiplication

use phonon::glicol_parser::parse_glicol;
use phonon::simple_dsp_executor::SimpleDspExecutor;
use std::fs::File;
use std::io::Write;

fn save_wav(
    filename: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(filename, spec)?;
    for &sample in samples {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude)?;
    }
    writer.finalize()?;
    Ok(())
}

fn main() {
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);

    println!("=== Phonon Signal Modulation Examples ===\n");

    // Example 1: Ring Modulation
    println!("1. Ring Modulation (creates sidebands)");
    let ring_mod = r#"
        ~carrier: sin 440     // A4 note
        ~modulator: sin 50    // 50Hz modulator
        o: ~carrier * ~modulator >> mul 0.5
    "#;

    match parse_glicol(ring_mod) {
        Ok(env) => match executor.render(&env, 1.0) {
            Ok(audio) => {
                println!("   Generated ring modulation: 440Hz ± 50Hz sidebands");
                save_wav("ring_modulation.wav", &audio.data, sample_rate as u32).ok();
            }
            Err(e) => println!("   Error: {}", e),
        },
        Err(e) => println!("   Parse error: {}", e),
    }

    // Example 2: Amplitude Modulation (tremolo)
    println!("\n2. Amplitude Modulation (tremolo effect)");
    let am_synthesis = r#"
        ~carrier: sin 440
        ~lfo: sin 6 >> mul 0.3 >> add 0.7    // LFO varies between 0.4 and 1.0
        o: ~carrier * ~lfo
    "#;

    match parse_glicol(am_synthesis) {
        Ok(env) => match executor.render(&env, 1.0) {
            Ok(audio) => {
                println!("   Generated AM synthesis with 6Hz tremolo");
                save_wav("amplitude_modulation.wav", &audio.data, sample_rate as u32).ok();
            }
            Err(e) => println!("   Error: {}", e),
        },
        Err(e) => println!("   Parse error: {}", e),
    }

    // Example 3: Complex modulation with envelopes
    println!("\n3. Complex Modulation with Envelopes");
    let complex_mod = r#"
        ~carrier: saw 220 >> lpf 2000 0.5
        ~mod_env: sin 0.5 >> mul 0.5 >> add 0.5   // Slow envelope
        ~fast_mod: sin 15                          // Fast modulator
        o: ~carrier * ~mod_env * ~fast_mod >> mul 0.3
    "#;

    match parse_glicol(complex_mod) {
        Ok(env) => match executor.render(&env, 2.0) {
            Ok(audio) => {
                println!("   Generated complex modulation");
                save_wav("complex_modulation.wav", &audio.data, sample_rate as u32).ok();
            }
            Err(e) => println!("   Error: {}", e),
        },
        Err(e) => println!("   Parse error: {}", e),
    }

    // Example 4: Pattern-based gating
    println!("\n4. Pattern-based Gating (rhythmic modulation)");
    let pattern_gate = r#"
        ~source: saw 110 >> lpf 500 0.8
        ~pattern: s "bd ~ cp ~ bd bd ~ cp"
        ~smooth: lpf 10 0.5    // Smooth the pattern transitions
        o: ~source * ~pattern >> ~smooth >> mul 0.5
    "#;

    match parse_glicol(pattern_gate) {
        Ok(env) => match executor.render(&env, 2.0) {
            Ok(audio) => {
                println!("   Generated pattern-gated synthesis");
                save_wav("pattern_gating.wav", &audio.data, sample_rate as u32).ok();
            }
            Err(e) => println!("   Error: {}", e),
        },
        Err(e) => println!("   Parse error: {}", e),
    }

    // Example 5: Sidechain compression simulation
    println!("\n5. Sidechain Compression Effect");
    let sidechain = r#"
        ~bass: saw 55 >> lpf 200 0.7
        ~kick: sin 60 >> env 0.01 0.05 0.0 0.1
        ~kick_pattern: s "bd ~ ~ ~ bd ~ ~ ~"
        ~duck: mul 0.2 >> add 0.8    // Ducking envelope
        o: ~bass * (~kick_pattern >> ~duck) + (~kick_pattern * 0.5)
    "#;

    match parse_glicol(sidechain) {
        Ok(env) => match executor.render(&env, 2.0) {
            Ok(audio) => {
                println!("   Generated sidechain compression effect");
                save_wav("sidechain.wav", &audio.data, sample_rate as u32).ok();
            }
            Err(e) => println!("   Error: {}", e),
        },
        Err(e) => println!("   Parse error: {}", e),
    }

    // Example 6: Vocoder-like effect
    println!("\n6. Vocoder-like Effect (spectral multiplication)");
    let vocoder = r#"
        ~carrier: saw 100 >> lpf 2000 0.5
        ~modulator: noise >> bpf 1000 0.5
        ~envelope: sin 2 >> mul 0.5 >> add 0.5
        o: ~carrier * ~modulator * ~envelope >> mul 0.2
    "#;

    match parse_glicol(vocoder) {
        Ok(env) => match executor.render(&env, 2.0) {
            Ok(audio) => {
                println!("   Generated vocoder-like effect");
                save_wav("vocoder_effect.wav", &audio.data, sample_rate as u32).ok();
            }
            Err(e) => println!("   Error: {}", e),
        },
        Err(e) => println!("   Parse error: {}", e),
    }

    println!("\n=== Examples Complete ===");
    println!("Audio files saved (if hound crate is available):");
    println!("  - ring_modulation.wav");
    println!("  - amplitude_modulation.wav");
    println!("  - complex_modulation.wav");
    println!("  - pattern_gating.wav");
    println!("  - sidechain.wav");
    println!("  - vocoder_effect.wav");

    println!("\nYou can multiply any signals together using the * operator!");
    println!("This enables ring modulation, AM synthesis, gating, and more.");
}
