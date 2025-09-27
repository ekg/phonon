//! Debug the complex drum pattern

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;

fn main() {
    println!("🔍 Debugging complex drum pattern\n");

    // Test delay node
    println!("Test: Delay node");
    let code = "out: impulse 2 >> delay 0.25 0.0";
    test_code(code);

    // Test noise with envelope
    println!("\nTest: Noise with envelope");
    let code = "out: noise >> env 0.001 0.02 0.0 0.03";
    test_code(code);

    // Test multiplication of signals
    println!("\nTest: Signal multiplication");
    let code = r#"
        ~trig: impulse 2
        ~sound: noise >> env 0.001 0.02 0.0 0.03
        out: ~trig * ~sound
    "#;
    test_code(code);

    // Simplified drum pattern
    println!("\nTest: Simplified drum pattern");
    let code = r#"
        ~kick: impulse 4 >> mul 60 >> lpf 100 0.9
        ~clap: impulse 2 >> mul 30
        out: ~kick + ~clap
    "#;
    test_code(code);
}

fn test_code(code: &str) {
    println!("  Code: {}", code.trim());

    match render_dsp_to_audio_simple(code, 44100.0, 0.5) {
        Ok(buffer) => {
            let peak = buffer.peak();
            let rms = buffer.rms();
            let non_zero = buffer.data.iter().filter(|&&x| x != 0.0).count();

            println!("  Peak: {:.6}, RMS: {:.6}", peak, rms);
            println!("  Non-zero samples: {} of {}", non_zero, buffer.data.len());

            if peak == 0.0 {
                println!("  ⚠️  SILENT!");
            } else {
                println!("  ✅ Audio generated");
            }
        }
        Err(e) => {
            println!("  ❌ Error: {}", e);
        }
    }
}
