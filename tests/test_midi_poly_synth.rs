//! End-to-end test for polyphonic MIDI synthesis
//! Tests that `saw ~midi` creates working polyphonic output

use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;
use phonon::midi_input::{MidiEvent, MidiEventQueue, MidiMessageType};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Helper to create a MIDI event queue for testing
fn create_test_queue() -> MidiEventQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

/// Helper to push a note-on event to the queue
fn push_note_on(queue: &MidiEventQueue, note: u8, velocity: u8) {
    let event = MidiEvent {
        message: vec![0x90, note, velocity], // Note on, channel 0
        timestamp_us: 0,
        channel: 0,
        message_type: MidiMessageType::NoteOn { note, velocity },
    };
    queue.lock().unwrap().push_back(event);
}

/// Helper to push a note-off event to the queue
fn push_note_off(queue: &MidiEventQueue, note: u8) {
    let event = MidiEvent {
        message: vec![0x80, note, 0], // Note off, channel 0
        timestamp_us: 0,
        channel: 0,
        message_type: MidiMessageType::NoteOff { note, velocity: 0 },
    };
    queue.lock().unwrap().push_back(event);
}

/// Test that saw ~midi compiles and produces audio when MIDI events are sent
#[test]
fn test_midi_poly_synth_produces_audio() {
    let code = r#"
tempo: 0.5
~piano $ saw ~midi
out $ ~piano * 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Unparsed: {}", rest);

    // Create MIDI event queue
    let queue = create_test_queue();

    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    // Send a MIDI note-on event (middle C = 60, velocity 100)
    push_note_on(&queue, 60, 100);

    // Render some audio
    let mut buffer = vec![0.0f32; 4410]; // 100ms at 44.1kHz
    graph.process_buffer(&mut buffer);

    // Check that we got non-silent audio
    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    let peak = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

    println!("After note-on: RMS={:.4}, Peak={:.4}", rms, peak);

    assert!(rms > 0.001, "Expected audio after note-on, got RMS={}", rms);
    assert!(peak > 0.01, "Expected peak > 0.01 after note-on, got {}", peak);
}

/// Test polyphony - multiple notes produce separate voices
#[test]
fn test_midi_poly_synth_polyphony() {
    let code = r#"
tempo: 0.5
~piano $ saw ~midi
out $ ~piano * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty());

    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    // Play chord - all notes at once
    push_note_on(&queue, 60, 100); // C
    push_note_on(&queue, 64, 100); // E
    push_note_on(&queue, 67, 100); // G

    // Wait for attack phase to complete (50ms at 44.1kHz)
    let mut warmup = vec![0.0f32; 2205];
    graph.process_buffer(&mut warmup);

    // Measure sustained chord
    let mut buffer_chord = vec![0.0f32; 4410];
    graph.process_buffer(&mut buffer_chord);
    let rms_chord = (buffer_chord.iter().map(|x| x * x).sum::<f32>() / buffer_chord.len() as f32).sqrt();

    // Now test single note in fresh graph
    let (_, statements2) = parse_program(code).expect("Parse failed");
    let queue2 = create_test_queue();
    let mut graph2 = compile_program(statements2, 44100.0, Some(queue2.clone()))
        .expect("Compile failed");

    push_note_on(&queue2, 60, 100); // Single note

    // Wait for attack
    let mut warmup2 = vec![0.0f32; 2205];
    graph2.process_buffer(&mut warmup2);

    // Measure single note
    let mut buffer_single = vec![0.0f32; 4410];
    graph2.process_buffer(&mut buffer_single);
    let rms_single = (buffer_single.iter().map(|x| x * x).sum::<f32>() / buffer_single.len() as f32).sqrt();

    println!("Single note RMS: {:.4}", rms_single);
    println!("Chord (3 notes) RMS: {:.4}", rms_chord);
    println!("Ratio: {:.2}x", rms_chord / rms_single);

    // Chord should have at least as much energy as single note
    // (with sqrt scaling, 3 notes / sqrt(3) ≈ 1.73x single note)
    assert!(rms_chord > rms_single * 0.9,
        "Chord should have comparable or more energy. Single={:.4}, Chord={:.4}",
        rms_single, rms_chord);

    // Also verify we actually have sound
    assert!(rms_chord > 0.01, "Chord should produce audio");
    assert!(rms_single > 0.01, "Single note should produce audio");
}

/// Test note-off triggers release
#[test]
fn test_midi_poly_synth_note_off() {
    let code = r#"
tempo: 0.5
~piano $ saw ~midi
out $ ~piano * 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty());

    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    // Note on
    push_note_on(&queue, 60, 100);

    // Render past attack phase
    let mut buffer = vec![0.0f32; 4410]; // 100ms
    graph.process_buffer(&mut buffer);
    let rms_held = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Note off
    push_note_off(&queue, 60);

    // Render release phase
    let mut buffer_release = vec![0.0f32; 44100]; // 1 second for full release
    graph.process_buffer(&mut buffer_release);

    // Check the end of release - should be mostly silent
    let end_slice = &buffer_release[40000..44100]; // Last 100ms
    let rms_end = (end_slice.iter().map(|x| x * x).sum::<f32>() / end_slice.len() as f32).sqrt();

    println!("RMS while held: {:.4}", rms_held);
    println!("RMS at end of release: {:.6}", rms_end);

    assert!(rms_held > 0.01, "Expected audio while note held");
    assert!(rms_end < rms_held * 0.1,
        "Expected audio to decay after note-off. Held={:.4}, End={:.6}",
        rms_held, rms_end);
}

/// Test different waveforms work with ~midi
#[test]
fn test_midi_poly_synth_waveforms() {
    for waveform in &["saw", "sine", "tri", "square"] {
        let code = format!(r#"
tempo: 0.5
~synth $ {} ~midi
out $ ~synth * 0.3
"#, waveform);

        let (rest, statements) = parse_program(&code).expect("Parse failed");
        assert!(rest.trim().is_empty(), "Failed to parse {} ~midi", waveform);

        let queue = create_test_queue();
        let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
            .expect(&format!("Compile failed for {} ~midi", waveform));

        push_note_on(&queue, 60, 100);

        let mut buffer = vec![0.0f32; 4410];
        graph.process_buffer(&mut buffer);

        let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
        println!("{} ~midi: RMS={:.4}", waveform, rms);

        assert!(rms > 0.001, "{} ~midi should produce audio, got RMS={}", waveform, rms);
    }
}

/// Test pitch accuracy - different MIDI notes should produce different frequencies
#[test]
fn test_midi_poly_synth_pitch_accuracy() {
    let code = r#"
tempo: 0.5
~synth $ sine ~midi
out $ ~synth * 0.5
"#;

    // Test A4 (440 Hz) - MIDI note 69
    let (_, statements) = parse_program(code).expect("Parse failed");
    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    push_note_on(&queue, 69, 100); // A4 = 440 Hz

    let mut buffer = vec![0.0f32; 44100]; // 1 second
    graph.process_buffer(&mut buffer);

    // Count zero crossings to estimate frequency
    let mut crossings = 0;
    for i in 1..buffer.len() {
        if buffer[i-1] <= 0.0 && buffer[i] > 0.0 {
            crossings += 1;
        }
    }

    let estimated_freq = crossings as f32; // ~440 crossings per second for 440Hz
    println!("A4 (MIDI 69): Estimated freq from zero crossings: {} Hz", estimated_freq);

    // Should be close to 440 Hz (allow some tolerance)
    assert!(estimated_freq > 400.0 && estimated_freq < 480.0,
        "Expected ~440 Hz, got {} Hz", estimated_freq);
}

/// Test scale locking - notes outside scale get quantized to nearest scale note
#[test]
fn test_midi_poly_synth_scale_locking() {
    // C major scale locking
    let code = r#"
tempo: 0.5
~synth $ sine ~midi:c:major
out $ ~synth * 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Unparsed: {}", rest);

    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    // Play C# (MIDI 61) - should be quantized to C (60) or D (62) in C major
    push_note_on(&queue, 61, 100);

    let mut buffer = vec![0.0f32; 44100]; // 1 second
    graph.process_buffer(&mut buffer);

    // Count zero crossings to estimate frequency
    let mut crossings = 0;
    for i in 1..buffer.len() {
        if buffer[i-1] <= 0.0 && buffer[i] > 0.0 {
            crossings += 1;
        }
    }

    let estimated_freq = crossings as f32;
    println!("C# (MIDI 61) with C major scale lock: Estimated freq: {} Hz", estimated_freq);

    // C4 = 261.63 Hz, D4 = 293.66 Hz
    // The note should be quantized to either C or D (nearest scale degrees)
    let c4_freq = 261.63;
    let d4_freq = 293.66;

    // Allow +/- 15 Hz tolerance for either C or D
    let is_near_c = (estimated_freq - c4_freq).abs() < 15.0;
    let is_near_d = (estimated_freq - d4_freq).abs() < 15.0;

    assert!(is_near_c || is_near_d,
        "C# should be quantized to C (~262 Hz) or D (~294 Hz), got {} Hz",
        estimated_freq);
}

/// Test scale locking with different scales
#[test]
fn test_midi_poly_synth_pentatonic_scale() {
    // C pentatonic scale locking (C D E G A)
    let code = r#"
tempo: 0.5
~synth $ sine ~midi:c:pentatonic
out $ ~synth * 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Unparsed: {}", rest);

    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    // Play F (MIDI 65) - should be quantized to E (64) or G (67) in C pentatonic
    push_note_on(&queue, 65, 100);

    let mut buffer = vec![0.0f32; 44100];
    graph.process_buffer(&mut buffer);

    // Count zero crossings
    let mut crossings = 0;
    for i in 1..buffer.len() {
        if buffer[i-1] <= 0.0 && buffer[i] > 0.0 {
            crossings += 1;
        }
    }

    let estimated_freq = crossings as f32;
    println!("F (MIDI 65) with C pentatonic scale lock: Estimated freq: {} Hz", estimated_freq);

    // E4 = 329.63 Hz, G4 = 392.00 Hz
    let e4_freq = 329.63;
    let g4_freq = 392.00;

    let is_near_e = (estimated_freq - e4_freq).abs() < 20.0;
    let is_near_g = (estimated_freq - g4_freq).abs() < 20.0;

    assert!(is_near_e || is_near_g,
        "F should be quantized to E (~330 Hz) or G (~392 Hz), got {} Hz",
        estimated_freq);
}
