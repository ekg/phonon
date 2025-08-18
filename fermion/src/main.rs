//! Fermion - The Rust synthesis engine for Phonon Forge

use clap::{Parser, Subcommand};
use tracing::{info};

mod synth;
mod server;

use synth::SynthEngine;
use server::OscServer;

#[derive(Parser)]
#[command(name = "fermion")]
#[command(about = "Fermion: Rust synthesis engine for Phonon Forge")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the synthesis server
    Serve {
        #[arg(short, long, default_value = "57120")]
        port: u16,
    },
    
    /// Play a test sound
    Test {
        #[arg(default_value = "440")]
        freq: f32,
        
        #[arg(default_value = "1.0")]
        duration: f32,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Serve { port } => {
            info!("Starting Fermion on port {}", port);
            let server = OscServer::new(port);
            server.run().await?;
        }
        
        Commands::Test { freq, duration } => {
            info!("Testing at {} Hz for {} seconds", freq, duration);
            let mut engine = SynthEngine::new();
            engine.play_test(freq, duration)?;
        }
    }
    
    Ok(())
}
