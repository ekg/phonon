//! Example of MIDI output with Phonon patterns

use phonon::midi_output::MidiOutputHandler;
use phonon::mini_notation::parse_mini_notation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Phonon MIDI Output Example");
    println!("==========================");

    // List available MIDI devices
    println!("\nAvailable MIDI devices:");
    let devices = MidiOutputHandler::list_devices()?;

    if devices.is_empty() {
        println!("No MIDI devices found!");
        println!("Please connect a MIDI device or start a virtual MIDI port.");
        return Ok(());
    }

    for (i, device) in devices.iter().enumerate() {
        println!("  [{}] {}", i, device.name);
    }

    // For demo, we'll create patterns but not actually connect
    // (to avoid requiring a specific MIDI device for testing)

    println!("\nExample patterns that could be sent to MIDI:");

    // Pattern 1: Simple melody
    let _melody = parse_mini_notation("c4 e4 g4 c5");
    println!("\n1. Simple melody: c4 e4 g4 c5");

    // Pattern 2: Chord progression
    let _chords = parse_mini_notation("<c4'maj e4'min g4'maj>");
    println!("2. Chord progression: <c4'maj e4'min g4'maj>");

    // Pattern 3: Drum pattern
    let _drums = parse_mini_notation("bd*4 [~ cp] hh*8");
    println!("3. Drum pattern: bd*4 [~ cp] hh*8");

    // Pattern 4: Euclidean rhythm
    let _euclidean = parse_mini_notation("bd(3,8) cp(5,8)");
    println!("4. Euclidean rhythm: bd(3,8) cp(5,8)");

    // Pattern 5: Complex pattern with effects
    let _complex = parse_mini_notation("<[c4 e4] [g4 b4]> . fast(2) . sometimes(degrade)");
    println!("5. Complex pattern: <[c4 e4] [g4 b4]> . fast(2) . sometimes(degrade)");

    // To actually play to MIDI (commented out to avoid requiring device):
    /*
    // Connect to first device
    let mut handler = MidiOutputHandler::new()?;
    handler.connect(&devices[0].name)?;

    // Play melody
    println!("\nPlaying melody...");
    handler.play_pattern(
        &melody,
        120.0,  // BPM
        4.0,    // Duration in beats
        |note_str| note_to_midi_message(note_str, 0, 64)
    )?;

    thread::sleep(Duration::from_secs(2));
    */

    println!("\nTo actually play these patterns:");
    println!("1. Connect a MIDI device or virtual MIDI port");
    println!("2. Uncomment the MIDI playback code in this example");
    println!("3. Run: cargo run --example midi_example");

    Ok(())
}
