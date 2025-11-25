//! Live coding with .phonon files - watches for changes and auto-plays

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;
use std::env;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <file.phonon> [duration_seconds]", args[0]);
        println!("\nLive coding mode - edit the file and it will auto-play on save!");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let duration = if args.len() > 2 {
        args[2].parse().unwrap_or(4.0)
    } else {
        4.0
    };

    println!("🎵 Phonon Live Coding");
    println!("=====================");
    println!("Watching: {}", file_path);
    println!("Duration: {} seconds", duration);
    println!("\n✨ Edit the file and save to hear changes!");
    println!("Press Ctrl+C to stop.\n");

    let mut last_modified = SystemTime::now();
    let output_path = "/tmp/phonon_live.wav";

    loop {
        // Check if file has been modified
        if let Ok(metadata) = fs::metadata(file_path) {
            if let Ok(modified) = metadata.modified() {
                if modified > last_modified {
                    last_modified = modified;

                    // Read and process the file
                    match fs::read_to_string(file_path) {
                        Ok(content) => {
                            // Remove comments and empty lines
                            let code = content
                                .lines()
                                .filter(|line| {
                                    !line.trim().starts_with('#') && !line.trim().is_empty()
                                })
                                .collect::<Vec<_>>()
                                .join("\n");

                            if code.trim().is_empty() {
                                println!("⚠️  File is empty or only contains comments");
                                continue;
                            }

                            println!("🔄 Reloading...");
                            println!("Code:\n{}", code);

                            // Render the audio
                            match render_dsp_to_audio_simple(&code, 44100.0, duration) {
                                Ok(buffer) => {
                                    match buffer.write_wav(output_path) {
                                        Ok(_) => {
                                            println!(
                                                "✅ Generated (Peak: {:.3}, RMS: {:.3})",
                                                buffer.peak(),
                                                buffer.rms()
                                            );

                                            // Kill any existing playback
                                            let _ = Command::new("pkill")
                                                .arg("-f")
                                                .arg("play.*phonon_live.wav")
                                                .status();

                                            // Play the new audio in background
                                            let _ = Command::new("play")
                                                .arg(output_path)
                                                .arg("-q") // Quiet mode
                                                .spawn();

                                            println!("🔊 Playing...\n");
                                        }
                                        Err(e) => println!("❌ Failed to save WAV: {}", e),
                                    }
                                }
                                Err(e) => println!("❌ Failed to generate audio: {}\n", e),
                            }
                        }
                        Err(e) => println!("❌ Error reading file: {}", e),
                    }
                }
            }
        }

        // Check every 100ms
        thread::sleep(Duration::from_millis(100));
    }
}
