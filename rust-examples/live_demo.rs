//! Simple live coding demo
//! Run with: cargo run --example live_demo <file.phonon>

use fermion::enhanced_parser::EnhancedParser;
use fermion::signal_executor::SignalExecutor;
use fermion::engine::AudioEngine;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file.phonon>", args[0]);
        eprintln!("       The file will be watched and reloaded on changes");
        std::process::exit(1);
    }
    
    let file_path = PathBuf::from(&args[1]);
    if !file_path.exists() {
        eprintln!("File not found: {}", file_path.display());
        std::process::exit(1);
    }
    
    println!("🎵 Phonon Live Coding Demo");
    println!("==========================");
    println!("Watching: {}", file_path.display());
    println!("Edit the file and save to hear changes!");
    println!("Press Ctrl+C to stop\n");
    
    // Initialize audio engine
    let engine = AudioEngine::new()?;
    
    // Track file modifications
    let mut last_modified = fs::metadata(&file_path)?.modified()?;
    let mut current_executor: Option<SignalExecutor> = None;
    
    // Initial load
    match load_patch(&file_path) {
        Ok(executor) => {
            println!("✅ Loaded successfully!");
            current_executor = Some(executor);
        }
        Err(e) => {
            println!("❌ Initial load failed: {}", e);
            println!("   Fix the error and save to retry");
        }
    }
    
    // Main loop
    let mut audio_buffer = Vec::new();
    
    loop {
        // Check for file changes
        if let Ok(metadata) = fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                if modified > last_modified {
                    println!("\n🔄 File changed, reloading...");
                    match load_patch(&file_path) {
                        Ok(executor) => {
                            println!("✅ Reloaded successfully!");
                            current_executor = Some(executor);
                            last_modified = modified;
                        }
                        Err(e) => {
                            println!("❌ Reload failed: {}", e);
                            println!("   Fix the error and save to retry");
                        }
                    }
                }
            }
        }
        
        // Process audio if we have a valid executor
        if let Some(ref mut executor) = current_executor {
            if let Ok(output) = executor.process_block() {
                audio_buffer.clear();
                audio_buffer.extend_from_slice(&output.data);
                
                // Apply some gain reduction to prevent clipping
                for sample in &mut audio_buffer {
                    *sample *= 0.5;
                }
                
                engine.play_synth(audio_buffer.clone(), 1.0);
            }
        }
        
        // Small delay to control CPU usage
        thread::sleep(Duration::from_millis(10));
    }
}

fn load_patch(file_path: &Path) -> Result<SignalExecutor, String> {
    // Read the file
    let dsl_code = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Parse the DSL
    let mut parser = EnhancedParser::new(44100.0);
    let graph = parser.parse(&dsl_code)?;
    
    // Create executor
    let mut executor = SignalExecutor::new(graph, 44100.0, 512);
    executor.initialize()?;
    
    Ok(executor)
}