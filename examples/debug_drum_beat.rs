//! Debug why the drum beat isn't generating audio

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("🔍 Debugging drum beat generation\n");
    
    // Test 1: Simple sine wave
    println!("Test 1: Simple sine wave");
    let code1 = "out: sin 440";
    test_code(code1);
    
    // Test 2: Sine with multiplication
    println!("\nTest 2: Sine with amplitude");
    let code2 = "out: sin 440 >> mul 0.5";
    test_code(code2);
    
    // Test 3: Impulse train
    println!("\nTest 3: Impulse train");
    let code3 = "out: impulse 4";
    test_code(code3);
    
    // Test 4: Impulse with multiplication
    println!("\nTest 4: Impulse with gain");
    let code4 = "out: impulse 4 >> mul 100";
    test_code(code4);
    
    // Test 5: Reference chain
    println!("\nTest 5: Reference chain");
    let code5 = r#"
        ~test: sin 440
        out: ~test
    "#;
    test_code(code5);
    
    // Test 6: Mix two signals
    println!("\nTest 6: Mix two signals");
    let code6 = "out: sin 440 + sin 880";
    test_code(code6);
    
    // Test 7: The actual drum pattern piece by piece
    println!("\nTest 7: Just kick");
    let code7 = "out: impulse 4 >> mul 60 >> lpf 100 0.9";
    test_code(code7);
}

fn test_code(code: &str) {
    println!("  Code: {}", code.trim());
    
    match render_dsp_to_audio_simple(code, 44100.0, 0.1) {
        Ok(buffer) => {
            let peak = buffer.peak();
            let rms = buffer.rms();
            let non_zero = buffer.data.iter().filter(|&&x| x != 0.0).count();
            
            println!("  Peak: {:.6}, RMS: {:.6}", peak, rms);
            println!("  Non-zero samples: {} of {}", non_zero, buffer.data.len());
            
            if peak == 0.0 {
                println!("  ⚠️  SILENT - No audio generated!");
                
                // Show first few samples anyway
                println!("  First 5 samples: {:?}", &buffer.data[..5.min(buffer.data.len())]);
            } else {
                println!("  ✅ Audio generated");
            }
        }
        Err(e) => {
            println!("  ❌ Error: {}", e);
        }
    }
}