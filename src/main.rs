//! Phonon CLI - Command-line interface for the Phonon synthesis system

use clap::{Parser, Subcommand};
use phonon::render::{render_cli, RenderConfig, Renderer};
use phonon::simple_dsp_executor;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "phonon")]
#[command(about = "Phonon modular synthesis system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a DSL file to WAV
    Render {
        /// Input file (.phonon or .dsl) or inline DSL code
        input: String,
        
        /// Output WAV file path
        output: String,
        
        /// Duration in seconds (default: 10.0)
        #[arg(short, long, default_value = "10.0")]
        duration: f32,
        
        /// Number of cycles (overrides duration if specified)
        #[arg(short, long)]
        cycles: Option<u32>,
        
        /// Sample rate in Hz (default: 44100)
        #[arg(short, long, default_value = "44100")]
        sample_rate: u32,
        
        /// Master gain 0.0-1.0 (default: 0.8)
        #[arg(short, long, default_value = "0.8")]
        gain: f32,
        
        /// Fade in time in seconds (default: 0.01)
        #[arg(long, default_value = "0.01")]
        fade_in: f32,
        
        /// Fade out time in seconds (default: 0.01)
        #[arg(long, default_value = "0.01")]
        fade_out: f32,
        
        /// Block size for processing (default: 512)
        #[arg(short, long, default_value = "512")]
        block_size: usize,
    },
    
    /// Play DSL file or code (render and auto-play)
    Play {
        /// Input file (.phonon) or inline DSL code
        input: String,
        
        /// Duration in seconds (default: 4.0)
        #[arg(short, long, default_value = "4.0")]
        duration: f32,
        
        /// Sample rate in Hz (default: 44100)
        #[arg(short, long, default_value = "44100")]
        sample_rate: u32,
        
        /// Master gain 0.0-1.0 (default: 0.8)
        #[arg(short, long, default_value = "0.8")]
        gain: f32,
    },
    
    /// Start live coding session with file watching
    Live {
        /// DSL file to watch and auto-reload
        file: PathBuf,
        
        /// Duration for each render (default: 4.0)
        #[arg(short, long, default_value = "4.0")]
        duration: f32,
        
        /// Enable pattern mode for Strudel-style patterns
        #[arg(short = 'P', long)]
        pattern: bool,
        
        /// OSC port to listen on (optional)
        #[arg(short, long, default_value = "9000")]
        port: u16,
    },
    
    /// Start interactive REPL
    Repl {},
    
    /// Open modal live coding editor
    Edit {
        /// Optional file to load
        file: Option<PathBuf>,
        
        /// Duration for each render (default: 4.0)
        #[arg(short, long, default_value = "4.0")]
        duration: f32,
    },
    
    /// Run tests on DSL files
    Test {
        /// Input file or directory
        input: PathBuf,
    },
    
    /// Send pattern to MIDI device
    Midi {
        /// Pattern to play (mini-notation)
        #[arg(short, long)]
        pattern: Option<String>,
        
        /// MIDI device name (partial match)
        #[arg(short, long)]
        device: Option<String>,
        
        /// Tempo in BPM (default: 120)
        #[arg(short, long, default_value = "120")]
        tempo: f32,
        
        /// Duration in beats (default: 16)
        #[arg(short = 'D', long, default_value = "16")]
        duration: f32,
        
        /// MIDI channel (0-15, default: 0)
        #[arg(short, long, default_value = "0")]
        channel: u8,
        
        /// Note velocity (0-127, default: 64)
        #[arg(short = 'v', long, default_value = "64")]
        velocity: u8,
        
        /// List MIDI devices and exit
        #[arg(short, long)]
        list: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Render {
            input,
            output,
            duration,
            cycles,
            sample_rate,
            gain,
            fade_in,
            fade_out,
            block_size,
        } => {
            // Read DSL code
            let dsl_code = if input == "-" {
                // Read from stdin
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else if input.ends_with(".phonon") || input.ends_with(".dsl") {
                std::fs::read_to_string(&input)?
            } else if std::path::Path::new(&input).exists() {
                // If it's a file path without extension, read it
                std::fs::read_to_string(&input)?
            } else {
                // Treat as inline DSL code
                input.clone()
            };
            
            // Calculate duration from cycles if specified
            let final_duration = if let Some(cycle_count) = cycles {
                // For patterns, assume 1 cycle = 1 second by default
                // This could be made smarter by parsing the pattern tempo
                cycle_count as f32
            } else {
                duration
            };
            
            // Configure renderer
            let config = RenderConfig {
                sample_rate,
                block_size,
                duration: final_duration,
                master_gain: gain,
                fade_in,
                fade_out,
            };
            
            // Create renderer
            let renderer = Renderer::new(config.clone());
            
            // Print info
            println!("🎵 Phonon Renderer");
            println!("==================");
            println!("Input:       {}", if input.ends_with(".phonon") || input.ends_with(".dsl") { &input } else { "<inline DSL>" });
            println!("Output:      {}", output);
            println!("Duration:    {} seconds", final_duration);
            if cycles.is_some() {
                println!("Cycles:      {}", cycles.unwrap());
            }
            println!("Sample rate: {} Hz", sample_rate);
            println!("Block size:  {} samples", block_size);
            println!("Master gain: {:.1}", gain);
            println!("Fades:       {:.3}s in, {:.3}s out", fade_in, fade_out);
            println!();
            
            // Render
            let output_path = PathBuf::from(&output);
            let stats = renderer.render_to_file(&dsl_code, &output_path)?;
            
            // Print statistics
            println!("Render Statistics:");
            println!("------------------");
            println!("Duration:       {:.3} seconds", stats.duration);
            println!("Samples:        {}", stats.sample_count);
            println!("RMS level:      {:.3} ({:.1} dB)", stats.rms, 20.0 * stats.rms.log10());
            println!("Peak level:     {:.3} ({:.1} dB)", stats.peak, 20.0 * stats.peak.log10());
            println!("DC offset:      {:.6}", stats.dc_offset);
            println!("Zero crossings: {}", stats.zero_crossings);
            
            // Estimate frequency if applicable
            if stats.zero_crossings > 100 {
                let est_freq = stats.zero_crossings as f32 / (2.0 * stats.duration);
                println!("Est. frequency: {:.1} Hz", est_freq);
            }
            
            println!();
            println!("✅ Successfully rendered to: {}", output);
            
            // Show file size
            let metadata = std::fs::metadata(&output_path)?;
            let size_kb = metadata.len() as f32 / 1024.0;
            println!("   File size: {:.1} KB", size_kb);
        }
        
        Commands::Play { input, duration, sample_rate, gain } => {
            use crate::simple_dsp_executor::render_dsp_to_audio_simple;
            use std::process::Command;
            
            // Read DSL code
            let dsl_code = if input.ends_with(".phonon") || input.ends_with(".dsl") {
                std::fs::read_to_string(&input)?
            } else if std::path::Path::new(&input).exists() {
                std::fs::read_to_string(&input)?
            } else {
                // Treat as inline DSL code
                input.clone()
            };
            
            // Strip comments and empty lines
            let clean_code = dsl_code.lines()
                .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            
            if clean_code.trim().is_empty() {
                println!("❌ No DSL code to process");
                return Ok(());
            }
            
            println!("🎵 Phonon Player");
            println!("================");
            println!("Input:      {}", if input.ends_with(".phonon") || input.ends_with(".dsl") { &input } else { "<inline DSL>" });
            println!("Duration:   {} seconds", duration);
            println!("Sample rate: {} Hz", sample_rate);
            println!("Gain:       {:.1}", gain);
            println!();
            
            println!("DSL Code:");
            for line in clean_code.lines() {
                println!("  {}", line);
            }
            println!();
            
            // Render audio
            match render_dsp_to_audio_simple(&clean_code, sample_rate as f32, duration) {
                Ok(mut buffer) => {
                    // Apply gain
                    for sample in buffer.data.iter_mut() {
                        *sample *= gain;
                    }
                    
                    let output_path = "/tmp/phonon_play.wav";
                    
                    match buffer.write_wav(output_path) {
                        Ok(_) => {
                            println!("✅ Audio generated!");
                            println!("   Peak: {:.3}", buffer.peak());
                            println!("   RMS: {:.3}", buffer.rms());
                            println!("   Saved to: {}", output_path);
                            
                            println!("\n🔊 Playing...");
                            
                            // Try different players
                            let players = ["play", "aplay", "pw-play", "paplay"];
                            let mut played = false;
                            
                            for player in &players {
                                let result = if *player == "play" {
                                    Command::new(player)
                                        .arg(output_path)
                                        .arg("-q")
                                        .status()
                                } else {
                                    Command::new(player)
                                        .arg(output_path)
                                        .status()
                                };
                                
                                if let Ok(status) = result {
                                    if status.success() {
                                        played = true;
                                        break;
                                    }
                                }
                            }
                            
                            if !played {
                                println!("⚠️  Could not auto-play. Try:");
                                for player in &players {
                                    if *player == "play" {
                                        println!("   {} -q {}", player, output_path);
                                    } else {
                                        println!("   {} {}", player, output_path);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to save WAV: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to generate audio: {}", e);
                }
            }
        }
        
        Commands::Live { file, duration, pattern, port } => {
            use std::time::{Duration as StdDuration, SystemTime};
            use std::thread;
            use std::process::Command;
            use crate::simple_dsp_executor::render_dsp_to_audio_simple;
            
            println!("🎵 Phonon Live Coding");
            println!("====================");
            println!("File: {}", file.display());
            println!("Duration: {}s per render", duration);
            if pattern {
                println!("Mode: Pattern sequencing (not yet implemented)");
            } else {
                println!("Mode: Continuous synthesis");
            }
            println!("OSC:  Port {} (not yet implemented)", port);
            println!();
            println!("✨ Edit {} and save to hear changes!", file.display());
            println!("   Press Ctrl+C to stop.\n");
            
            let mut last_modified = SystemTime::now();
            let output_path = "/tmp/phonon_live.wav";
            
            loop {
                if let Ok(metadata) = std::fs::metadata(&file) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > last_modified {
                            last_modified = modified;
                            
                            match std::fs::read_to_string(&file) {
                                Ok(content) => {
                                    // Strip comments and empty lines
                                    let code = content.lines()
                                        .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    
                                    if !code.trim().is_empty() {
                                        println!("🔄 Reloading...");
                                        
                                        match render_dsp_to_audio_simple(&code, 44100.0, duration) {
                                            Ok(buffer) => {
                                                if let Ok(_) = buffer.write_wav(output_path) {
                                                    println!("✅ Peak: {:.3}, RMS: {:.3}", 
                                                             buffer.peak(), buffer.rms());
                                                    
                                                    // Kill previous playback
                                                    let _ = Command::new("pkill")
                                                        .arg("-f")
                                                        .arg("play.*phonon_live.wav")
                                                        .status();
                                                    
                                                    // Play in background
                                                    let _ = Command::new("play")
                                                        .arg(output_path)
                                                        .arg("-q")
                                                        .spawn();
                                                    
                                                    println!("");
                                                }
                                            }
                                            Err(e) => println!("❌ Error: {}\n", e),
                                        }
                                    }
                                }
                                Err(e) => println!("❌ Read error: {}", e),
                            }
                        }
                    }
                }
                
                thread::sleep(StdDuration::from_millis(100));
            }
        }
        
        Commands::Repl {} => {
            use phonon::live::LiveRepl;
            
            println!("🎵 Phonon Live REPL");
            println!("==================");
            println!();
            
            let mut repl = LiveRepl::new()?;
            repl.run()?;
        }
        
        Commands::Edit { file, duration } => {
            use phonon::modal_editor::ModalEditor;
            
            let mut editor = ModalEditor::new(duration, file.clone())?;
            editor.run()?;
        }
        
        Commands::Test { input } => {
            println!("🧪 Phonon Test Runner");
            println!("====================");
            println!("Input: {}", input.display());
            println!();
            println!("⚠️  Test mode not yet implemented");
            println!("   This will run validation tests on DSL files");
        }
        
        Commands::Midi { 
            pattern, 
            device, 
            tempo, 
            duration, 
            channel, 
            velocity, 
            list 
        } => {
            use phonon::midi_output::{MidiOutputHandler, note_to_midi_message};
            use phonon::mini_notation_v3::parse_mini_notation;
            
            println!("🎹 Phonon MIDI Output");
            println!("====================");
            
            // List devices if requested
            if list {
                let devices = MidiOutputHandler::list_devices()?;
                if devices.is_empty() {
                    println!("No MIDI devices found!");
                    println!("Please connect a MIDI device or start a virtual MIDI port.");
                } else {
                    println!("Available MIDI devices:");
                    for (i, dev) in devices.iter().enumerate() {
                        println!("  [{}] {}", i, dev.name);
                    }
                }
                return Ok(());
            }
            
            // Check if pattern is provided
            let Some(pattern) = pattern else {
                println!("\n⚠️  Please provide a pattern with --pattern");
                println!("   Example: phonon midi --pattern \"c4 e4 g4 c5\"");
                return Ok(());
            };
            
            // Parse pattern
            let pat = parse_mini_notation(&pattern);
            println!("Pattern: {}", pattern);
            println!("Tempo:   {} BPM", tempo);
            println!("Duration: {} beats", duration);
            
            // Connect to MIDI device
            let mut handler = MidiOutputHandler::new()?;
            
            if let Some(device_name) = device {
                println!("Device:  {}", device_name);
                handler.connect(&device_name)?;
            } else {
                // Try to connect to first available device
                let devices = MidiOutputHandler::list_devices()?;
                if devices.is_empty() {
                    println!("\n⚠️  No MIDI devices found!");
                    println!("   Please connect a MIDI device or start a virtual MIDI port.");
                    println!("   You can list devices with: phonon midi --list");
                    return Ok(());
                }
                let device = devices.into_iter().next().unwrap();
                println!("Device:  {} (auto-selected)", device.name);
                handler.connect_to_port(device.port)?;
            }
            
            println!("\n▶️  Playing pattern to MIDI...");
            println!("   Press Ctrl+C to stop\n");
            
            // Play pattern
            handler.play_pattern(
                &pat,
                tempo,
                duration,
                |note_str| note_to_midi_message(note_str, channel, velocity)
            )?;
            
            println!("\n✅ Playback complete!");
        }
    }
    
    Ok(())
}