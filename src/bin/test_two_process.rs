//! Test for two-process architecture
//!
//! Spawns phonon-audio, connects to it, sends a test pattern, waits, then shuts down.
//!
//! Note: This binary is Unix-only (requires Unix domain sockets).

#[cfg(not(unix))]
fn main() {
    eprintln!("test_two_process is only supported on Unix platforms (Linux, macOS)");
    std::process::exit(1);
}

#[cfg(unix)]
use phonon::ipc::{IpcMessage, PatternClient};
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🧪 Testing two-process architecture");

    // Spawn audio engine
    eprintln!("📦 Spawning audio engine...");
    let mut audio_process = Command::new("cargo")
        .args(&["run", "--release", "--bin", "phonon-audio"])
        .spawn()?;

    // Give audio engine time to start and bind socket
    thread::sleep(Duration::from_millis(500));

    // Connect to audio engine
    eprintln!("🔌 Connecting to audio engine...");
    let mut client = PatternClient::connect()?;

    // Wait for Ready message
    eprintln!("⏳ Waiting for Ready message...");
    match client.receive()? {
        IpcMessage::Ready => eprintln!("✅ Audio engine ready"),
        msg => eprintln!("⚠️  Unexpected message: {:?}", msg),
    }

    // Send a simple test pattern
    let test_code = r#"
bpm: 120
o1: s "bd sn bd sn"
"#;

    eprintln!("📤 Sending test pattern...");
    client.send(&IpcMessage::UpdateGraph {
        code: test_code.to_string(),
    })?;

    eprintln!("🎵 Playing for 5 seconds...");
    thread::sleep(Duration::from_secs(5));

    // Test pattern update (simulate live coding)
    let updated_code = r#"
bpm: 120
o1: s "bd*4 sn*2"
"#;

    eprintln!("📤 Sending updated pattern...");
    client.send(&IpcMessage::UpdateGraph {
        code: updated_code.to_string(),
    })?;

    eprintln!("🎵 Playing updated pattern for 3 seconds...");
    thread::sleep(Duration::from_secs(3));

    // Test hush
    eprintln!("🔇 Sending Hush...");
    client.send(&IpcMessage::Hush)?;

    thread::sleep(Duration::from_secs(1));

    // Shutdown
    eprintln!("👋 Sending Shutdown...");
    client.send(&IpcMessage::Shutdown)?;

    // Wait for audio process to exit
    eprintln!("⏳ Waiting for audio engine to exit...");
    audio_process.wait()?;

    eprintln!("✅ Test completed successfully!");

    Ok(())
}
