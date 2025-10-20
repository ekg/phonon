//! Tests for OSC live coding server
//!
//! Tests the OSC server functionality on port 7770 with /eval, /hush, /panic

use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Helper to send OSC message to server
fn send_osc_message(
    port: u16,
    addr: &str,
    args: Vec<OscType>,
) -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let msg = OscMessage {
        addr: addr.to_string(),
        args,
    };
    let packet = OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet)?;
    socket.send_to(&buf, format!("127.0.0.1:{}", port))?;
    Ok(())
}

#[test]
fn test_osc_server_eval_message() {
    // This test will fail until we implement the OSC server

    // Start a mock OSC server to test message parsing
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received_messages.clone();

    let test_port = 7771; // Use different port for test to avoid conflicts

    // Spawn server thread
    let server_handle = thread::spawn(move || {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", test_port)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let mut buf = [0u8; 1024];
        match socket.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                if let Ok((_remaining, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                    if let OscPacket::Message(msg) = packet {
                        received_clone.lock().unwrap().push(msg);
                    }
                }
            }
            Err(_) => {}
        }
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Send /eval message with Phonon code
    let phonon_code = "~d1 = sine(440) * 0.2";
    send_osc_message(
        test_port,
        "/eval",
        vec![OscType::String(phonon_code.to_string())],
    )
    .unwrap();

    // Wait for server to process
    server_handle.join().unwrap();

    // Check that message was received
    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 1, "Should receive one message");
    assert_eq!(messages[0].addr, "/eval", "Should receive /eval message");
    assert_eq!(messages[0].args.len(), 1, "Should have one argument");

    if let OscType::String(code) = &messages[0].args[0] {
        assert_eq!(code, phonon_code, "Should receive correct Phonon code");
    } else {
        panic!("Expected string argument");
    }
}

#[test]
fn test_osc_server_hush_message() {
    // Test that /hush message is received correctly
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received_messages.clone();

    let test_port = 7772;

    let server_handle = thread::spawn(move || {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", test_port)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let mut buf = [0u8; 1024];
        match socket.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                if let Ok((_remaining, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                    if let OscPacket::Message(msg) = packet {
                        received_clone.lock().unwrap().push(msg);
                    }
                }
            }
            Err(_) => {}
        }
    });

    thread::sleep(Duration::from_millis(100));

    // Send /hush message (no arguments - stops all audio)
    send_osc_message(test_port, "/hush", vec![]).unwrap();

    server_handle.join().unwrap();

    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 1, "Should receive one message");
    assert_eq!(messages[0].addr, "/hush", "Should receive /hush message");
    assert_eq!(messages[0].args.len(), 0, "Should have no arguments");
}

#[test]
fn test_osc_server_panic_message() {
    // Test that /panic message is received correctly
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received_messages.clone();

    let test_port = 7773;

    let server_handle = thread::spawn(move || {
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", test_port)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let mut buf = [0u8; 1024];
        match socket.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                if let Ok((_remaining, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                    if let OscPacket::Message(msg) = packet {
                        received_clone.lock().unwrap().push(msg);
                    }
                }
            }
            Err(_) => {}
        }
    });

    thread::sleep(Duration::from_millis(100));

    // Send /panic message (emergency stop - kills all audio immediately)
    send_osc_message(test_port, "/panic", vec![]).unwrap();

    server_handle.join().unwrap();

    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), 1, "Should receive one message");
    assert_eq!(messages[0].addr, "/panic", "Should receive /panic message");
    assert_eq!(messages[0].args.len(), 0, "Should have no arguments");
}

#[test]
#[ignore = "Integration test - requires full server implementation"]
fn test_osc_eval_compiles_code() {
    // This test verifies that /eval actually compiles and updates the audio graph
    // Will implement after basic message handling works
    todo!("Implement after OSC server integration with UnifiedSignalGraph");
}

#[test]
#[ignore = "Integration test - requires full server implementation"]
fn test_osc_hush_stops_audio() {
    // This test verifies that /hush actually stops audio output
    // Will implement after basic message handling works
    todo!("Implement after OSC server integration with audio engine");
}

#[test]
#[ignore = "Integration test - requires full server implementation"]
fn test_osc_panic_kills_audio() {
    // This test verifies that /panic immediately kills all audio
    // Will implement after basic message handling works
    todo!("Implement after OSC server integration with audio engine");
}
