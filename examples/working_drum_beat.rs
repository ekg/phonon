//! Working example that actually generates a drum beat with filtering

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("🥁 Generating filtered drum beat...\n");

    // Create a simple kick + clap beat with filter sweep
    let code = r#"
        ~kick: impulse 4 >> mul 60 >> lpf 100 0.9 >> mul 2
        ~clap_trig: impulse 2 >> delay 0.25 0.0
        ~clap: noise >> env 0.001 0.02 0.0 0.03 >> hpf 1500 0.8
        ~clap_sound: ~clap_trig * ~clap
        ~drums: ~kick + ~clap_sound
        ~lfo: sin 0.25 >> mul 1500 >> add 2000
        out: ~drums >> lpf ~lfo 0.6
    "#;

    println!("DSP Code:");
    println!("{}", code);

    // Render 4 seconds of audio
    match render_dsp_to_audio_simple(code, 44100.0, 4.0) {
        Ok(buffer) => {
            println!("\n✅ Audio generated successfully!");
            println!("   Samples: {}", buffer.data.len());
            println!("   Duration: {:.2}s", buffer.data.len() as f32 / 44100.0);
            println!("   Peak: {:.3}", buffer.peak());
            println!("   RMS: {:.3}", buffer.rms());

            // Save to file
            let output_path = "/tmp/drum_beat_output.wav";
            match buffer.write_wav(output_path) {
                Ok(_) => {
                    println!("\n📁 Saved to: {}", output_path);
                    println!("   Play with: play {}", output_path);
                }
                Err(e) => println!("❌ Failed to save WAV: {}", e),
            }

            // Show first few samples to prove we generated audio
            println!("\n🔊 First 10 samples:");
            for (i, sample) in buffer.data.iter().take(10).enumerate() {
                println!("   [{}]: {:.6}", i, sample);
            }
        }
        Err(e) => {
            println!("❌ Failed to generate audio: {}", e);
            println!("\nThis tells us what's not working in the DSP parser/executor");
        }
    }
}
