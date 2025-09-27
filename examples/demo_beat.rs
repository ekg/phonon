//! Demo beat - 4 kick + clap pattern with filter sweep

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("🥁 Generating 4*bd cp type beat with filter...\n");

    // Classic 4-on-floor kick with clap on 2 and 4, plus hihat pattern
    // Filter frequency is fixed but creates movement
    let code = r#"
        ~kick: impulse 4 >> mul 100 >> lpf 80 0.95
        ~clap: impulse 2 >> delay 0.25 0.0 >> mul 50 >> noise >> mul 0.4 >> hpf 1200 0.7
        ~hihat: impulse 8 >> mul 15 >> noise >> mul 0.2 >> hpf 7000 0.9
        ~drums: ~kick + ~clap + ~hihat
        out: ~drums >> lpf 2500 0.6 >> mul 0.8
    "#;

    println!("Pattern breakdown:");
    println!("  Kick:  X . . . X . . . X . . . X . . .  (4 per bar)");
    println!("  Clap:  . . . . X . . . . . . . X . . .  (2 per bar, delayed)");
    println!("  Hihat: X . X . X . X . X . X . X . X .  (8 per bar)");
    println!();
    println!("DSP Code:");
    println!("{}", code);

    // Render 8 seconds (2 bars at 120 BPM)
    match render_dsp_to_audio_simple(code, 44100.0, 8.0) {
        Ok(buffer) => {
            println!("\n✅ Beat generated successfully!");
            println!("   Duration: 8 seconds (2 bars)");
            println!("   Peak: {:.3}", buffer.peak());
            println!("   RMS: {:.3}", buffer.rms());

            // Save to file
            let path = "/tmp/demo_beat.wav";
            buffer.write_wav(path).unwrap();
            println!("\n📁 Saved to: {}", path);
            println!("   Play with: play {}", path);
            println!("\nTry also:");
            println!("   aplay {}", path);
            println!("   ffplay -nodisp -autoexit {}", path);
        }
        Err(e) => {
            println!("\n❌ Failed: {}", e);
        }
    }
}
