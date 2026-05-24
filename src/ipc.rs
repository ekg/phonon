//! IPC Communication for Two-Process Architecture
//!
//! Pattern Engine (phonon edit) ←→ Audio Engine (phonon-audio)
//!
//! Communication via Unix Domain Socket for:
//! - Low latency (< 1ms)
//! - Bidirectional messaging
//! - Clean process separation

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

/// Message types sent between pattern and audio engines
#[derive(Debug, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Pattern → Audio: Update the active signal graph
    UpdateGraph {
        /// DSL code string to compile
        /// We send code instead of compiled graph because:
        /// 1. UnifiedSignalGraph has non-serializable state (RefCell, Arc)
        /// 2. Compilation is fast enough (~1-2ms)
        /// 3. Audio engine compiles independently → clean separation
        /// 4. Easier to debug (human-readable code)
        code: String,
    },

    /// Pattern → Audio: Silence all outputs
    Hush,

    /// Pattern → Audio: Stop all synthesis and clear voices
    Panic,

    /// Pattern → Audio: Set global tempo
    SetTempo { cps: f32 },

    /// Audio → Pattern: Audio engine is ready to receive
    Ready,

    /// Audio → Pattern: Audio underrun detected
    Underrun { count: usize },

    /// Either direction: Graceful shutdown
    Shutdown,
}

impl IpcMessage {
    /// Serialize message to bytes for transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("Failed to serialize message: {}", e))
    }

    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes).map_err(|e| format!("Failed to deserialize message: {}", e))
    }

    /// Send message over Unix socket
    pub fn send(&self, stream: &mut UnixStream) -> Result<(), String> {
        let bytes = self.to_bytes()?;

        // Send length prefix (4 bytes) then data
        let len = bytes.len() as u32;
        stream
            .write_all(&len.to_le_bytes())
            .map_err(|e| format!("Failed to write length: {}", e))?;

        stream
            .write_all(&bytes)
            .map_err(|e| format!("Failed to write data: {}", e))?;

        stream
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Receive message from Unix socket
    pub fn receive(stream: &mut UnixStream) -> Result<Self, String> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .map_err(|e| format!("Failed to read length: {}", e))?;

        let len = u32::from_le_bytes(len_bytes) as usize;

        // Sanity check: max 100MB per message
        if len > 100_000_000 {
            return Err(format!("Message too large: {} bytes", len));
        }

        // Read data
        let mut data = vec![0u8; len];
        stream
            .read_exact(&mut data)
            .map_err(|e| format!("Failed to read data: {}", e))?;

        Self::from_bytes(&data)
    }

    /// Try to receive a message without blocking
    /// Returns Ok(Some(msg)) if message available, Ok(None) if no message, Err on error
    pub fn try_receive(stream: &mut UnixStream) -> Result<Option<Self>, String> {
        // Temporarily set non-blocking
        stream
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        // Try to read length prefix
        let mut len_bytes = [0u8; 4];
        let result = match stream.read_exact(&mut len_bytes) {
            Ok(_) => {
                let len = u32::from_le_bytes(len_bytes) as usize;

                // Sanity check
                if len > 100_000_000 {
                    Err(format!("Message too large: {} bytes", len))
                } else {
                    // Read data
                    let mut data = vec![0u8; len];
                    stream
                        .read_exact(&mut data)
                        .map_err(|e| format!("Failed to read data: {}", e))?;

                    Self::from_bytes(&data).map(Some)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(format!("Failed to read length: {}", e)),
        };

        // Restore blocking mode
        stream
            .set_nonblocking(false)
            .map_err(|e| format!("Failed to restore blocking: {}", e))?;

        result
    }

    /// Drain all pending UpdateGraph messages, keeping only the most recent one.
    /// Non-UpdateGraph messages (Hush, Panic, SetTempo, Shutdown) are never dropped:
    /// they are accumulated in arrival order and returned alongside the latest UpdateGraph.
    /// Returns a non-empty Vec; callers must dispatch all messages in order.
    pub fn receive_coalesced(stream: &mut UnixStream) -> Result<Vec<Self>, String> {
        // Receive first message (blocking)
        let first_msg = Self::receive(stream)?;

        // If it's not an UpdateGraph, return it immediately without draining
        if !matches!(first_msg, IpcMessage::UpdateGraph { .. }) {
            return Ok(vec![first_msg]);
        }

        // It's an UpdateGraph — drain any additional pending messages
        let mut latest_update = first_msg;
        let mut side_buffer: Vec<IpcMessage> = Vec::new();
        let mut drained_count = 0;

        loop {
            match Self::try_receive(stream)? {
                Some(msg) => {
                    if let IpcMessage::UpdateGraph { .. } = msg {
                        latest_update = msg;
                        drained_count += 1;
                    } else {
                        // Hush, Panic, SetTempo, Shutdown — never drop these
                        side_buffer.push(msg);
                    }
                }
                None => break,
            }
        }

        if drained_count > 0 {
            eprintln!(
                "🔄 Drained {} stale UpdateGraph message(s), processing most recent",
                drained_count
            );
        }

        // Return non-UpdateGraph messages first (arrival order), then the latest graph update
        let mut result = side_buffer;
        result.push(latest_update);
        Ok(result)
    }
}

/// Get the Unix socket path for IPC
/// Uses /tmp/phonon.sock
/// Note: Only one instance can run at a time with this approach
pub fn socket_path() -> PathBuf {
    PathBuf::from("/tmp/phonon.sock")
}

/// Audio engine socket server
pub struct AudioServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl AudioServer {
    /// Create new audio server listening on Unix socket
    pub fn new() -> Result<Self, String> {
        let path = socket_path();

        // Remove old socket if it exists
        let _ = std::fs::remove_file(&path);

        let listener =
            UnixListener::bind(&path).map_err(|e| format!("Failed to bind socket: {}", e))?;

        eprintln!("🎵 Audio server listening on: {}", path.display());

        Ok(Self {
            listener,
            socket_path: path,
        })
    }

    /// Wait for pattern engine to connect
    pub fn accept(&self) -> Result<UnixStream, String> {
        let (stream, _) = self
            .listener
            .accept()
            .map_err(|e| format!("Failed to accept connection: {}", e))?;

        eprintln!("✅ Pattern engine connected");

        Ok(stream)
    }
}

impl Drop for AudioServer {
    fn drop(&mut self) {
        // Clean up socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Pattern engine socket client
pub struct PatternClient {
    stream: UnixStream,
}

impl PatternClient {
    /// Connect to audio engine
    pub fn connect() -> Result<Self, String> {
        let path = socket_path();

        // Retry connection for up to 5 seconds (audio engine might be starting)
        let mut attempts = 0;
        let stream = loop {
            match UnixStream::connect(&path) {
                Ok(s) => break s,
                Err(e) => {
                    attempts += 1;
                    if attempts > 50 {
                        return Err(format!("Failed to connect after 50 attempts: {}", e));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        };

        eprintln!("✅ Connected to audio engine");

        Ok(Self { stream })
    }

    /// Send message to audio engine
    pub fn send(&mut self, msg: &IpcMessage) -> Result<(), String> {
        msg.send(&mut self.stream)
    }

    /// Receive message from audio engine (non-blocking would be better for production)
    pub fn receive(&mut self) -> Result<IpcMessage, String> {
        IpcMessage::receive(&mut self.stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn test_message_serialization() {
        let msg = IpcMessage::Hush;
        let bytes = msg.to_bytes().unwrap();
        let decoded = IpcMessage::from_bytes(&bytes).unwrap();

        match decoded {
            IpcMessage::Hush => {}
            _ => panic!("Wrong message type"),
        }
    }

    /// Send [UpdateGraph(A), UpdateGraph(B), Hush, UpdateGraph(C)].
    /// receive_coalesced must return Hush alongside UpdateGraph(C), not drop Hush.
    #[test]
    fn test_receive_coalesced_preserves_hush() {
        use std::sync::mpsc;

        let path = format!("/tmp/phonon_test_hush_{}.sock", std::process::id());
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        // Channels to coordinate: writer signals when all messages are sent,
        // reader signals when it's done so writer can close the connection.
        let (all_sent_tx, all_sent_rx) = mpsc::channel::<()>();
        let (reader_done_tx, reader_done_rx) = mpsc::channel::<()>();

        let path2 = path.clone();
        let writer = std::thread::spawn(move || {
            let mut client = UnixStream::connect(&path2).unwrap();
            IpcMessage::UpdateGraph { code: "A".to_string() }.send(&mut client).unwrap();
            IpcMessage::UpdateGraph { code: "B".to_string() }.send(&mut client).unwrap();
            IpcMessage::Hush.send(&mut client).unwrap();
            IpcMessage::UpdateGraph { code: "C".to_string() }.send(&mut client).unwrap();
            all_sent_tx.send(()).unwrap(); // all messages in kernel buffer
            reader_done_rx.recv().unwrap(); // keep connection open until reader is done
        });

        let (mut stream, _) = listener.accept().unwrap();
        all_sent_rx.recv().unwrap(); // all 4 messages now in the socket buffer

        let messages = IpcMessage::receive_coalesced(&mut stream).unwrap();

        reader_done_tx.send(()).unwrap(); // writer may now close
        writer.join().unwrap();

        // Expect [Hush, UpdateGraph(C)] — Hush not dropped, only newest graph kept
        assert_eq!(messages.len(), 2, "expected Hush + latest UpdateGraph, got {:?}", messages);
        assert!(matches!(messages[0], IpcMessage::Hush), "first message must be Hush");
        match &messages[1] {
            IpcMessage::UpdateGraph { code } => assert_eq!(code, "C"),
            other => panic!("second message must be UpdateGraph(C), got {:?}", other),
        }

        let _ = std::fs::remove_file(&path);
    }

    /// Send [Panic, UpdateGraph(A)].
    /// Panic arrives first and is not an UpdateGraph, so receive_coalesced returns it immediately.
    #[test]
    fn test_receive_coalesced_preserves_panic() {
        use std::sync::mpsc;

        let path = format!("/tmp/phonon_test_panic_{}.sock", std::process::id());
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        let (all_sent_tx, all_sent_rx) = mpsc::channel::<()>();
        let (reader_done_tx, reader_done_rx) = mpsc::channel::<()>();

        let path2 = path.clone();
        let writer = std::thread::spawn(move || {
            let mut client = UnixStream::connect(&path2).unwrap();
            IpcMessage::Panic.send(&mut client).unwrap();
            IpcMessage::UpdateGraph { code: "A".to_string() }.send(&mut client).unwrap();
            all_sent_tx.send(()).unwrap();
            reader_done_rx.recv().unwrap();
        });

        let (mut stream, _) = listener.accept().unwrap();
        all_sent_rx.recv().unwrap();

        let messages = IpcMessage::receive_coalesced(&mut stream).unwrap();

        reader_done_tx.send(()).unwrap();
        writer.join().unwrap();

        // Panic is not UpdateGraph so receive_coalesced returns it immediately without draining
        assert_eq!(messages.len(), 1, "expected only Panic, got {:?}", messages);
        assert!(matches!(messages[0], IpcMessage::Panic), "message must be Panic");

        let _ = std::fs::remove_file(&path);
    }
}
