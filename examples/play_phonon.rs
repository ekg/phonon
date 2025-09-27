//! Simple player for .phonon files using the working SimpleDspExecutor

use phonon::simple_dsp_executor::render_dsp_to_audio_simple;
use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <file.phonon> [duration_seconds]", args[0]);
        println!("       {} -c '<dsl code>' [duration_seconds]", args[0]);
        std::process::exit(1);
    }

    let duration = if args.len() > 2 {
        args[2].parse().unwrap_or(4.0)
    } else {
        4.0
    };

    // Get the DSL code
    let code = if args[1] == "-c" {
        if args.len() < 3 {
            eprintln!("Error: -c requires DSL code argument");
            std::process::exit(1);
        }
        args[2].clone()
    } else {
        // Read from file
        match fs::read_to_string(&args[1]) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", args[1], e);
                std::process::exit(1);
            }
        }
    };

    // Remove comments and empty lines
    let code = code
        .lines()
        .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    println!("🎵 Phonon Player");
    println!("================");
    println!("Duration: {} seconds", duration);
    println!("\nDSL Code:");
    println!("{}", code);

    // Render the audio
    match render_dsp_to_audio_simple(&code, 44100.0, duration) {
        Ok(buffer) => {
            let output_path = "/tmp/phonon_output.wav";

            // Write WAV file
            match buffer.write_wav(output_path) {
                Ok(_) => {
                    println!("\n✅ Audio generated!");
                    println!("   Peak: {:.3}", buffer.peak());
                    println!("   RMS: {:.3}", buffer.rms());
                    println!("   Saved to: {}", output_path);

                    // Try to play the file
                    println!("\n🔊 Playing audio...");

                    // Try different audio players in order of preference
                    let players = ["play", "aplay", "ffplay", "paplay", "pw-play"];
                    let mut played = false;

                    for player in &players {
                        let result = if player == &"ffplay" {
                            Command::new(player)
                                .args(&["-nodisp", "-autoexit", output_path])
                                .status()
                        } else {
                            Command::new(player).arg(output_path).status()
                        };

                        if let Ok(status) = result {
                            if status.success() {
                                played = true;
                                break;
                            }
                        }
                    }

                    if !played {
                        println!("⚠️  Could not auto-play. Use one of:");
                        for player in &players {
                            if player == &"ffplay" {
                                println!("   {} -nodisp -autoexit {}", player, output_path);
                            } else {
                                println!("   {} {}", player, output_path);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to save WAV: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to generate audio: {}", e);
            std::process::exit(1);
        }
    }
}
