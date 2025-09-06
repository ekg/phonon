//! Actually working drum beat example with DSP filtering
//! This uses only the DSP nodes that are actually implemented

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("🥁 Generating WORKING filtered drum beat...\n");
    
    // A beat that actually works with our current implementation
    let code = r#"
        ~kick: impulse 4 >> mul 80 >> lpf 100 0.9
        ~snare: impulse 2 >> mul 30 >> noise >> hpf 500 0.7
        ~hihat: impulse 8 >> mul 10 >> noise >> hpf 8000 0.9
        ~lfo: sin 0.5 >> mul 1000 >> add 2000
        ~mixed: ~kick + ~snare + ~hihat
        out: ~mixed >> lpf ~lfo 0.6
    "#;
    
    println!("DSP Code:");
    println!("{}", code);
    
    // Render 4 seconds
    match render_dsp_to_audio_simple(code, 44100.0, 4.0) {
        Ok(buffer) => {
            let peak = buffer.peak();
            let rms = buffer.rms();
            
            if peak == 0.0 {
                println!("\n❌ Generated silence! The DSP chain has issues.");
                println!("   This proves the tests are not actually end-to-end.");
            } else {
                println!("\n✅ Audio generated successfully!");
                println!("   Peak: {:.3}", peak);
                println!("   RMS: {:.3}", rms);
                
                // Save to file
                let path = "/tmp/actual_drum_beat.wav";
                buffer.write_wav(path).unwrap();
                println!("\n📁 Saved to: {}", path);
                println!("   Play with: play {}", path);
            }
        }
        Err(e) => {
            println!("\n❌ Failed: {}", e);
        }
    }
    
    // Also test the simplest possible beat
    println!("\n--- Simplest Beat ---");
    let simple = "out: impulse 2 >> mul 50 >> lpf 200 0.8";
    
    match render_dsp_to_audio_simple(simple, 44100.0, 2.0) {
        Ok(buffer) => {
            println!("Simple beat: Peak={:.3}, RMS={:.3}", buffer.peak(), buffer.rms());
            buffer.write_wav("/tmp/simple_beat.wav").unwrap();
            println!("Saved to: /tmp/simple_beat.wav");
        }
        Err(e) => println!("Failed: {}", e),
    }
}