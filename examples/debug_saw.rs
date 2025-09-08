use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("Debugging saw wave rendering...\n");
    
    // Test direct saw output
    let test_cases = vec![
        ("Direct saw", "out: saw 220"),
        ("Direct saw with mul", "out: saw 220 >> mul 0.5"),
        ("Saw reference", "~osc: saw 220\nout: ~osc"),
        ("Saw ref with mul", "~osc: saw 220\nout: ~osc >> mul 0.5"),
        ("Saw with filter inline", "out: saw 220 >> lpf 1000 0.7"),
        ("Saw with filter ref", "~osc: saw 220\n~filt: ~osc >> lpf 1000 0.7\nout: ~filt"),
    ];
    
    for (name, code) in test_cases {
        println!("Test: {}", name);
        println!("Code:\n{}\n", code);
        
        match render_dsp_to_audio_simple(code, 44100.0, 0.01) {
            Ok(buffer) => {
                let peak = buffer.data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let rms = (buffer.data.iter().map(|s| s * s).sum::<f32>() / buffer.data.len() as f32).sqrt();
                
                println!("  Samples: {}", buffer.data.len());
                println!("  Peak: {:.6}", peak);
                println!("  RMS: {:.6}", rms);
                
                if peak > 0.001 {
                    println!("  ✓ Has audio output");
                } else {
                    println!("  ✗ No audio output!");
                    // Print first few samples to debug
                    println!("  First 10 samples: {:?}", &buffer.data[0..10.min(buffer.data.len())]);
                }
            }
            Err(e) => println!("  ✗ Error: {}", e),
        }
        println!();
    }
}