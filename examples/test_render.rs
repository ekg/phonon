use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("Testing render functionality...\n");
    
    // Test 1: Simple sine wave
    let code1 = r#"
~osc: sin 440
out: ~osc >> mul 0.5
"#;
    
    println!("Test 1 - Sine wave:");
    println!("{}", code1);
    
    match render_dsp_to_audio_simple(code1, 44100.0, 0.1) {
        Ok(buffer) => {
            println!("✓ Rendered {} samples", buffer.data.len());
            let peak = buffer.data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            println!("  Peak amplitude: {}", peak);
            if peak > 0.0 {
                println!("  ✓ Has audio output");
            } else {
                println!("  ✗ No audio output!");
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
    
    // Test 2: Saw wave with filter (the failing test)
    let code2 = r#"
~osc: saw 220
~filtered: ~osc >> lpf 1000 0.7
out: ~filtered >> mul 0.3
"#;
    
    println!("\nTest 2 - Filtered saw wave:");
    println!("{}", code2);
    
    match render_dsp_to_audio_simple(code2, 44100.0, 0.1) {
        Ok(buffer) => {
            println!("✓ Rendered {} samples", buffer.data.len());
            let peak = buffer.data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            println!("  Peak amplitude: {}", peak);
            if peak > 0.0 {
                println!("  ✓ Has audio output");
            } else {
                println!("  ✗ No audio output!");
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
    
    // Test 3: Direct output (simpler)
    let code3 = r#"
out: sin 440 >> mul 0.5
"#;
    
    println!("\nTest 3 - Direct output:");
    println!("{}", code3);
    
    match render_dsp_to_audio_simple(code3, 44100.0, 0.1) {
        Ok(buffer) => {
            println!("✓ Rendered {} samples", buffer.data.len());
            let peak = buffer.data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            println!("  Peak amplitude: {}", peak);
            if peak > 0.0 {
                println!("  ✓ Has audio output");
            } else {
                println!("  ✗ No audio output!");
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
}