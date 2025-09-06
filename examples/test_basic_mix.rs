//! Test basic mixing of references

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("Testing basic mixing of references\n");
    
    // Test 1: Two simple references
    println!("Test 1: Mix two references");
    let code = r#"
        ~a: impulse 2 >> mul 50
        ~b: impulse 4 >> mul 30
        out: ~a + ~b
    "#;
    test_code(code);
    
    // Test 2: Reference with modulation
    println!("\nTest 2: Reference as modulation");
    let code = r#"
        ~lfo: sin 2 >> mul 500 >> add 1000
        out: impulse 4 >> mul 50 >> lpf ~lfo 0.7
    "#;
    test_code(code);
    
    // Test 3: The actual working beat
    println!("\nTest 3: Actual working beat");
    let code = r#"
        ~kick: impulse 4 >> mul 80 >> lpf 100 0.9
        ~snare: impulse 2 >> mul 30 >> noise >> hpf 500 0.7
        ~hihat: impulse 8 >> mul 10 >> noise >> hpf 8000 0.9
        ~lfo: sin 0.5 >> mul 1000 >> add 2000
        ~mixed: ~kick + ~snare + ~hihat
        out: ~mixed >> lpf ~lfo 0.6
    "#;
    test_code(code);
}

fn test_code(code: &str) {
    println!("  Code: {}", code.trim());
    
    match render_dsp_to_audio_simple(code, 44100.0, 0.5) {
        Ok(buffer) => {
            let peak = buffer.peak();
            let rms = buffer.rms();
            let non_zero = buffer.data.iter().filter(|&&x| x.abs() > 0.0001).count();
            
            println!("  Peak: {:.3}, RMS: {:.3}", peak, rms);
            println!("  Non-zero samples: {} of {}", non_zero, buffer.data.len());
            
            if peak < 0.0001 {
                println!("  ⚠️  SILENT!");
                // Show first few samples
                println!("  First 10 samples: {:?}", &buffer.data[..10.min(buffer.data.len())]);
            } else {
                println!("  ✅ Audio generated");
            }
        }
        Err(e) => {
            println!("  ❌ Error: {}", e);
        }
    }
}