//! Working drum beat without reference modulation

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("🥁 Generating drum beat (no reference modulation)...\n");

    // Create a drum beat without using references as modulation parameters
    let code = r#"
        ~kick: impulse 4 >> mul 80 >> lpf 150 0.9
        ~snare: impulse 2 >> delay 0.25 0.0 >> mul 40 >> noise >> mul 0.5 >> hpf 1000 0.7
        ~hihat: impulse 8 >> mul 20 >> noise >> mul 0.3 >> hpf 5000 0.8
        out: ~kick + ~snare + ~hihat >> lpf 3000 0.6
    "#;

    println!("DSP Code:");
    println!("{}", code);

    // Render 4 seconds of audio
    match render_dsp_to_audio_simple(code, 44100.0, 4.0) {
        Ok(buffer) => {
            let peak = buffer.peak();
            let rms = buffer.rms();

            if peak < 0.0001 {
                println!("\n❌ Generated silence! Something is still broken.");
                println!(
                    "   First 10 samples: {:?}",
                    &buffer.data[..10.min(buffer.data.len())]
                );
            } else {
                println!("\n✅ Audio generated successfully!");
                println!("   Peak: {:.3}", peak);
                println!("   RMS: {:.3}", rms);

                // Save to file
                let path = "/tmp/working_beat.wav";
                buffer.write_wav(path).unwrap();
                println!("\n📁 Saved to: {}", path);
                println!("   Play with: play {}", path);
            }
        }
        Err(e) => {
            println!("\n❌ Failed: {}", e);
        }
    }

    // Also test simpler beats
    println!("\n--- Simple Kick Pattern ---");
    let simple_kick = "out: impulse 4 >> mul 60 >> lpf 100 0.8";
    match render_dsp_to_audio_simple(simple_kick, 44100.0, 2.0) {
        Ok(buffer) => {
            println!("Kick: Peak={:.3}, RMS={:.3}", buffer.peak(), buffer.rms());
            buffer.write_wav("/tmp/simple_kick.wav").unwrap();
        }
        Err(e) => println!("Failed: {}", e),
    }

    println!("\n--- Kick + Snare ---");
    let kick_snare = r#"
        ~kick: impulse 4 >> mul 50 >> lpf 100 0.9
        ~snare: impulse 2 >> delay 0.25 0.0 >> mul 30 >> hpf 500 0.7
        out: ~kick + ~snare
    "#;
    match render_dsp_to_audio_simple(kick_snare, 44100.0, 2.0) {
        Ok(buffer) => {
            println!(
                "Kick+Snare: Peak={:.3}, RMS={:.3}",
                buffer.peak(),
                buffer.rms()
            );
            buffer.write_wav("/tmp/kick_snare.wav").unwrap();
        }
        Err(e) => println!("Failed: {}", e),
    }
}
