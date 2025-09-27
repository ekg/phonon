//! Example of OSC control for Phonon patterns

use phonon::osc_control::{OscClient, OscPatternEngine};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Phonon OSC Control Example");
    println!("==========================\n");

    // Start OSC pattern engine on port 9000
    let mut engine = OscPatternEngine::new(Some(9000))?;
    println!("✅ OSC server started on port 9000");
    println!("   Listening for OSC messages...\n");

    // Create OSC client to send test messages
    let client = OscClient::new("127.0.0.1:9000")?;

    // Load some patterns
    println!("📦 Loading patterns via OSC:");

    client.load_pattern("drums", "bd*4 [~ cp] hh*8")?;
    println!("   - Loaded 'drums': bd*4 [~ cp] hh*8");

    client.load_pattern("bass", "c2 ~ e2 g2")?;
    println!("   - Loaded 'bass': c2 ~ e2 g2");

    client.load_pattern("melody", "c4 e4 g4 c5")?;
    println!("   - Loaded 'melody': c4 e4 g4 c5");

    thread::sleep(Duration::from_millis(100));

    // Process commands
    engine.process_osc_commands();

    // Set tempo
    println!("\n⏱️  Setting tempo to 140 BPM");
    client.set_tempo(140.0)?;

    thread::sleep(Duration::from_millis(100));
    engine.process_osc_commands();

    // Play patterns
    println!("\n▶️  Starting patterns:");

    client.play_pattern("drums")?;
    println!("   - Playing 'drums'");

    client.play_pattern("bass")?;
    println!("   - Playing 'bass'");

    thread::sleep(Duration::from_millis(100));
    engine.process_osc_commands();

    // Simulate a few beats
    println!("\n🎵 Pattern output for 4 beats:");
    let tempo = engine.get_tempo();
    let beat_duration = 60.0 / tempo;

    for beat in 0..16 {
        // 16 steps = 4 beats (at 1/4 resolution)
        let beat_time = beat as f64 * 0.25;
        let active = engine.get_active_patterns(beat_time);

        if !active.is_empty() {
            print!("   Beat {:.2}: ", beat_time);
            for (name, values) in active {
                print!("{}: {:?}  ", name, values);
            }
            println!();
        }

        thread::sleep(Duration::from_secs_f32(beat_duration * 0.25));
    }

    // Demonstrate muting
    println!("\n🔇 Muting 'drums'...");
    client.send(
        "/mute",
        vec![
            rosc::OscType::String("drums".to_string()),
            rosc::OscType::Bool(true),
        ],
    )?;

    thread::sleep(Duration::from_millis(100));
    engine.process_osc_commands();

    // Stop all
    println!("\n⏹️  Stopping all patterns");
    client.send("/stop/all", vec![])?;

    thread::sleep(Duration::from_millis(100));
    engine.process_osc_commands();

    println!("\n✅ OSC control demo complete!");
    println!("\nOSC Message Reference:");
    println!("----------------------");
    println!("/pattern/load  <name:string> <pattern:string>  - Load a pattern");
    println!("/pattern/play  <name:string>                   - Play a pattern");
    println!("/pattern/stop  <name:string>                   - Stop a pattern");
    println!("/tempo         <bpm:float>                     - Set tempo");
    println!("/control       <name:string> <value:float>     - Set control value");
    println!("/mute          <name:string> <muted:bool>      - Mute/unmute");
    println!("/solo          <name:string>                   - Solo a pattern");
    println!("/solo/clear                                    - Clear solo");
    println!("/volume        <name:string> <volume:float>    - Set volume");
    println!("/sync                                          - Sync to beat");
    println!("/stop/all                                      - Stop all patterns");
    println!("/status                                        - Get status");

    Ok(())
}
