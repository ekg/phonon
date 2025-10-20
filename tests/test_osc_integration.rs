//! Integration tests for OSC live server
//!
//! Tests the full OSC server workflow: start server, send messages, verify responses

use phonon::osc_live_server::{apply_command_to_graph, LiveCommand, OscLiveServer};
use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

/// Helper to send OSC message
fn send_osc(port: u16, addr: &str, args: Vec<OscType>) -> Result<(), Box<dyn std::error::Error>> {
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
fn test_osc_server_starts_and_receives() {
    // Start OSC server on a test port
    let test_port = 7774;
    let (mut server, receiver) = OscLiveServer::new(test_port).unwrap();
    server.start().unwrap();

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Send /eval message
    let code = "cps: 1.0\n~d1: sine(220)";
    send_osc(test_port, "/eval", vec![OscType::String(code.to_string())]).unwrap();

    // Wait for message to be processed
    thread::sleep(Duration::from_millis(100));

    // Receive command
    let cmd = receiver.recv_timeout(Duration::from_secs(1));
    assert!(cmd.is_ok(), "Should receive command from OSC server");

    if let Ok(LiveCommand::Eval {
        code: received_code,
    }) = cmd
    {
        assert_eq!(received_code, code, "Should receive correct code");
    } else {
        panic!("Expected Eval command");
    }

    server.stop();
}

#[test]
fn test_osc_server_hush() {
    let test_port = 7775;
    let (mut server, receiver) = OscLiveServer::new(test_port).unwrap();
    server.start().unwrap();

    thread::sleep(Duration::from_millis(100));

    // Send /hush message
    send_osc(test_port, "/hush", vec![]).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Receive command
    let cmd = receiver.recv_timeout(Duration::from_secs(1));
    assert!(cmd.is_ok());
    assert!(matches!(cmd.unwrap(), LiveCommand::Hush));

    server.stop();
}

#[test]
fn test_osc_server_panic() {
    let test_port = 7776;
    let (mut server, receiver) = OscLiveServer::new(test_port).unwrap();
    server.start().unwrap();

    thread::sleep(Duration::from_millis(100));

    // Send /panic message
    send_osc(test_port, "/panic", vec![]).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Receive command
    let cmd = receiver.recv_timeout(Duration::from_secs(1));
    assert!(cmd.is_ok());
    assert!(matches!(cmd.unwrap(), LiveCommand::Panic));

    server.stop();
}

#[test]
#[ignore = "Parser issue with multi-line strings in apply_command_to_graph - needs investigation"]
fn test_eval_creates_working_graph() {
    // Test that /eval actually creates a functional audio graph
    // Use auto-routing pattern: ~d1 should route to master automatically
    let code = r#"cps: 2.0
~d1: sine 440"#;
    let cmd = LiveCommand::Eval {
        code: code.to_string(),
    };

    let graph_opt = apply_command_to_graph(&cmd, 44100.0);
    if graph_opt.is_none() {
        eprintln!("Failed to create graph from code:");
        eprintln!("{}", code);
        panic!("Should create graph from eval");
    }

    let mut graph = graph_opt.unwrap();
    assert_eq!(graph.get_cps(), 2.0, "Should set CPS correctly");

    // Verify the graph has an output (via auto-routing)
    if !graph.has_output() {
        eprintln!("Graph has no output set!");
        eprintln!("Buses: {:?}", graph.get_all_bus_names());
    }
    assert!(
        graph.has_output(),
        "Auto-routing should set output from ~d1"
    );

    // Process some samples to verify graph works
    let mut has_audio = false;
    for _ in 0..44100 {
        let sample = graph.process_sample();
        if sample.abs() > 0.001 {
            has_audio = true;
            break;
        }
    }

    assert!(
        has_audio,
        "Graph should produce audio from ~d1 via auto-routing"
    );
}

#[test]
fn test_hush_stops_audio() {
    // Test that hush creates a silent graph
    let cmd = LiveCommand::Hush;
    let graph = apply_command_to_graph(&cmd, 44100.0);
    assert!(graph.is_some());

    let mut graph = graph.unwrap();

    // Verify all samples are zero
    for _ in 0..1000 {
        let sample = graph.process_sample();
        assert_eq!(sample, 0.0, "Hush should produce silence");
    }
}

#[test]
fn test_panic_stops_audio_immediately() {
    // Test that panic creates a silent graph
    let cmd = LiveCommand::Panic;
    let graph = apply_command_to_graph(&cmd, 44100.0);
    assert!(graph.is_some());

    let mut graph = graph.unwrap();

    // Verify all samples are zero
    for _ in 0..1000 {
        let sample = graph.process_sample();
        assert_eq!(sample, 0.0, "Panic should produce immediate silence");
    }
}

#[test]
fn test_multiple_eval_commands() {
    // Test that we can send multiple eval commands in sequence
    let test_port = 7777;
    let (mut server, receiver) = OscLiveServer::new(test_port).unwrap();
    server.start().unwrap();

    thread::sleep(Duration::from_millis(100));

    // Send first eval
    send_osc(
        test_port,
        "/eval",
        vec![OscType::String("cps: 1.0\n~d1: sine(110)".to_string())],
    )
    .unwrap();

    thread::sleep(Duration::from_millis(50));

    // Send second eval
    send_osc(
        test_port,
        "/eval",
        vec![OscType::String("cps: 2.0\n~d2: saw(220)".to_string())],
    )
    .unwrap();

    thread::sleep(Duration::from_millis(100));

    // Should receive two commands
    let cmd1 = receiver.recv_timeout(Duration::from_secs(1));
    assert!(cmd1.is_ok());

    let cmd2 = receiver.recv_timeout(Duration::from_secs(1));
    assert!(cmd2.is_ok());

    server.stop();
}

#[test]
#[ignore = "Requires actual audio device and extended testing"]
fn test_osc_live_coding_session() {
    // This would test a full live coding session
    // - Start OSC server
    // - Send eval commands to build up a composition
    // - Use hush to clear
    // - Test panic for emergency stop
    // - Verify audio output
    todo!("Implement full live coding session test with audio verification");
}
