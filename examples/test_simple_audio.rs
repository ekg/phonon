//! Test simple audio generation to debug issues

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("Testing basic audio generation...\n");
    
    // Test 1: Simple sine wave (should be clean)
    println!("Test 1: 440Hz sine wave");
    let code = "out: sin 440 >> mul 0.3";
    match render_dsp_to_audio_simple(code, 44100.0, 1.0) {
        Ok(buffer) => {
            let path = "/tmp/test_sine.wav";
            buffer.write_wav(path).unwrap();
            println!("  Saved to: {}", path);
            println!("  Peak: {:.3}, RMS: {:.3}", buffer.peak(), buffer.rms());
            
            // Check for DC offset
            let dc_offset = buffer.data.iter().sum::<f32>() / buffer.data.len() as f32;
            println!("  DC offset: {:.6}", dc_offset);
            
            // Check for clipping
            let clipped = buffer.data.iter().filter(|&&x| x.abs() >= 0.99).count();
            println!("  Clipped samples: {}", clipped);
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    // Test 2: Very simple kick (softer impulse)
    println!("\nTest 2: Soft kick");
    let code = "out: impulse 2 >> mul 0.5 >> lpf 100 0.7";
    match render_dsp_to_audio_simple(code, 44100.0, 1.0) {
        Ok(buffer) => {
            let path = "/tmp/test_kick.wav";
            buffer.write_wav(path).unwrap();
            println!("  Saved to: {}", path);
            println!("  Peak: {:.3}, RMS: {:.3}", buffer.peak(), buffer.rms());
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    // Test 3: Just noise (should be random)
    println!("\nTest 3: Filtered noise");
    let code = "out: noise >> mul 0.1 >> lpf 1000 0.5";
    match render_dsp_to_audio_simple(code, 44100.0, 1.0) {
        Ok(buffer) => {
            let path = "/tmp/test_noise.wav";
            buffer.write_wav(path).unwrap();
            println!("  Saved to: {}", path);
            println!("  Peak: {:.3}, RMS: {:.3}", buffer.peak(), buffer.rms());
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    println!("\nPlay with:");
    println!("  aplay /tmp/test_sine.wav");
    println!("  aplay /tmp/test_kick.wav");
    println!("  aplay /tmp/test_noise.wav");
}