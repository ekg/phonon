//! Deep profiling tests for MidiPolySynth
//! Tests voice limits, CPU usage, and latency

use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;
use phonon::midi_input::{MidiEvent, MidiEventQueue, MidiMessageType};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn create_test_queue() -> MidiEventQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

fn push_note_on(queue: &MidiEventQueue, note: u8, velocity: u8) {
    let event = MidiEvent {
        message: vec![0x90, note, velocity],
        timestamp_us: 0,
        channel: 0,
        message_type: MidiMessageType::NoteOn { note, velocity },
    };
    queue.lock().unwrap().push_back(event);
}

fn push_note_off(queue: &MidiEventQueue, note: u8) {
    let event = MidiEvent {
        message: vec![0x80, note, 0],
        timestamp_us: 0,
        channel: 0,
        message_type: MidiMessageType::NoteOff { note, velocity: 0 },
    };
    queue.lock().unwrap().push_back(event);
}

/// Profile: How many voices can we sustain before CPU becomes a problem?
/// Target: Process 256 samples (one audio buffer) faster than real-time
#[test]
fn test_voice_limit_profiling() {
    let code = r#"
tempo: 0.5
~synth $ saw ~midi
out $ ~synth * 0.3
"#;
    let sample_rate = 44100.0f32;
    let buffer_size = 256; // Typical audio buffer size
    let realtime_budget_us = (buffer_size as f64 / sample_rate as f64 * 1_000_000.0) as u64; // ~5804 us

    println!("\n=== VOICE LIMIT PROFILING ===");
    println!("Buffer size: {} samples", buffer_size);
    println!("Real-time budget: {} µs ({:.2} ms)", realtime_budget_us, realtime_budget_us as f64 / 1000.0);
    println!();

    let voice_counts = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512];

    for &num_voices in &voice_counts {
        let (_, statements) = parse_program(code).expect("Parse failed");
        let queue = create_test_queue();
        let mut graph = compile_program(statements, sample_rate, Some(queue.clone()))
            .expect("Compile failed");

        // Trigger all voices (spread across MIDI notes 36-127)
        for i in 0..num_voices {
            let note = 36 + (i % 92) as u8; // Stay in valid MIDI range
            push_note_on(&queue, note, 100);
        }

        // Warmup - let attack complete
        let mut warmup = vec![0.0f32; 4410];
        graph.process_buffer(&mut warmup);

        // Profile: process multiple buffers and measure time
        let iterations = 100;
        let mut buffer = vec![0.0f32; buffer_size];

        let start = Instant::now();
        for _ in 0..iterations {
            graph.process_buffer(&mut buffer);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() as f64 / iterations as f64;
        let cpu_percent = (avg_us / realtime_budget_us as f64) * 100.0;
        let headroom = if avg_us < realtime_budget_us as f64 {
            ((realtime_budget_us as f64 - avg_us) / realtime_budget_us as f64) * 100.0
        } else {
            -((avg_us - realtime_budget_us as f64) / realtime_budget_us as f64) * 100.0
        };

        // Verify audio is being produced
        let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        println!(
            "{:>4} voices: {:>8.1} µs/buffer ({:>5.1}% CPU) | headroom: {:>+6.1}% | RMS: {:.4}",
            num_voices, avg_us, cpu_percent, headroom, rms
        );

        // Fail if we exceed 80% CPU (need headroom for other processing)
        if num_voices <= 64 {
            assert!(cpu_percent < 80.0,
                "CPU usage too high for {} voices: {:.1}%", num_voices, cpu_percent);
        }
    }
}

/// Profile: Per-sample processing time breakdown
#[test]
fn test_per_sample_timing() {
    let code = r#"
tempo: 0.5
~synth $ saw ~midi
out $ ~synth * 0.3
"#;
    let sample_rate = 44100.0f32;

    println!("\n=== PER-SAMPLE TIMING ===");

    for &num_voices in &[1, 8, 32, 64] {
        let (_, statements) = parse_program(code).expect("Parse failed");
        let queue = create_test_queue();
        let mut graph = compile_program(statements, sample_rate, Some(queue.clone()))
            .expect("Compile failed");

        // Trigger voices
        for i in 0..num_voices {
            push_note_on(&queue, 36 + (i % 92) as u8, 100);
        }

        // Warmup
        let mut warmup = vec![0.0f32; 4410];
        graph.process_buffer(&mut warmup);

        // Process 1 sample at a time to measure per-sample cost
        let iterations = 10000;
        let mut sample = vec![0.0f32; 1];

        let start = Instant::now();
        for _ in 0..iterations {
            graph.process_buffer(&mut sample);
        }
        let elapsed = start.elapsed();

        let ns_per_sample = elapsed.as_nanos() as f64 / iterations as f64;
        let max_sample_rate = 1_000_000_000.0 / ns_per_sample;

        println!(
            "{:>2} voices: {:>6.0} ns/sample | max sample rate: {:>7.0} Hz ({:.1}x 44.1kHz)",
            num_voices, ns_per_sample, max_sample_rate, max_sample_rate / 44100.0
        );
    }
}

/// Test: Voice release and cleanup
#[test]
fn test_voice_release_cleanup() {
    let code = r#"
tempo: 0.5
~synth $ saw ~midi
out $ ~synth * 0.3
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    println!("\n=== VOICE RELEASE & CLEANUP ===");

    // Play 16 notes
    for i in 0..16 {
        push_note_on(&queue, 48 + i, 100);
    }

    let mut buffer = vec![0.0f32; 4410];
    graph.process_buffer(&mut buffer);
    let rms_held = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    println!("16 notes held: RMS = {:.4}", rms_held);

    // Release all notes
    for i in 0..16 {
        push_note_off(&queue, 48 + i);
    }

    // Wait for release (300ms release time = ~13230 samples)
    let mut release_buffer = vec![0.0f32; 44100]; // 1 second
    graph.process_buffer(&mut release_buffer);

    // Check end of buffer - should be silent
    let end_slice = &release_buffer[40000..44100];
    let rms_end = (end_slice.iter().map(|x| x * x).sum::<f32>() / end_slice.len() as f32).sqrt();
    println!("After release: RMS = {:.6}", rms_end);

    assert!(rms_end < 0.001, "Voices should be silent after release");

    // Play new notes - should reuse released voices
    for i in 0..8 {
        push_note_on(&queue, 60 + i, 100);
    }

    let mut buffer2 = vec![0.0f32; 4410];
    graph.process_buffer(&mut buffer2);
    let rms_new = (buffer2.iter().map(|x| x * x).sum::<f32>() / buffer2.len() as f32).sqrt();
    println!("8 new notes: RMS = {:.4}", rms_new);

    assert!(rms_new > 0.01, "New notes should produce audio");
}

/// Profile: Latency measurement (MIDI event to audio output)
#[test]
fn test_latency_measurement() {
    let code = r#"
tempo: 0.5
~synth $ saw ~midi
out $ ~synth * 0.5
"#;
    let sample_rate = 44100.0f32;

    println!("\n=== LATENCY MEASUREMENT ===");
    println!("Note: This measures processing latency only, not audio driver latency");
    println!();

    // Measure time from MIDI event push to first non-zero sample
    let buffer_sizes = [64, 128, 256, 512, 1024];

    for &buffer_size in &buffer_sizes {
        // Fresh graph for each test
        let (_, statements) = parse_program(code).expect("Parse failed");
        let queue = create_test_queue();
        let mut graph = compile_program(statements, sample_rate, Some(queue.clone()))
            .expect("Compile failed");

        let start = Instant::now();
        push_note_on(&queue, 60, 100);

        let mut buffer = vec![0.0f32; buffer_size];
        graph.process_buffer(&mut buffer);

        let processing_time = start.elapsed();

        // Find first non-zero sample
        let first_nonzero = buffer.iter().position(|&x| x.abs() > 0.001);
        let samples_to_sound = first_nonzero.unwrap_or(buffer_size);
        let latency_samples = samples_to_sound;
        let latency_ms = (latency_samples as f64 / sample_rate as f64) * 1000.0;
        let buffer_latency_ms = (buffer_size as f64 / sample_rate as f64) * 1000.0;

        println!(
            "Buffer {:>4}: processing={:>6.1}µs | samples to sound={:>3} | latency={:.2}ms | buffer={:.2}ms",
            buffer_size,
            processing_time.as_micros() as f64,
            latency_samples,
            latency_ms,
            buffer_latency_ms
        );
    }

    println!();
    println!("Latency breakdown:");
    println!("  - MIDI parsing: ~0 (instant, same thread)");
    println!("  - Voice allocation: ~0 (instant)");
    println!("  - Attack ramp: ~1-2 samples (envelope starts from 0)");
    println!("  - Audio buffer: depends on driver settings (typically 256-1024 samples)");
    println!();
    println!("Total expected latency = audio_buffer + driver_latency");
    println!("  @ 256 samples: {:.1}ms + driver", 256.0 / 44100.0 * 1000.0);
    println!("  @ 512 samples: {:.1}ms + driver", 512.0 / 44100.0 * 1000.0);
    println!("  @ 1024 samples: {:.1}ms + driver", 1024.0 / 44100.0 * 1000.0);
}

/// Profile: Waveform comparison (which is fastest?)
#[test]
fn test_waveform_performance() {
    let sample_rate = 44100.0f32;
    let buffer_size = 256;
    let iterations = 1000;
    let num_voices = 32;

    println!("\n=== WAVEFORM PERFORMANCE (32 voices) ===");

    for waveform in &["saw", "sine", "square", "tri"] {
        let code = format!(r#"
tempo: 0.5
~synth $ {} ~midi
out $ ~synth * 0.3
"#, waveform);

        let (_, statements) = parse_program(&code).expect("Parse failed");
        let queue = create_test_queue();
        let mut graph = compile_program(statements, sample_rate, Some(queue.clone()))
            .expect("Compile failed");

        // Trigger voices
        for i in 0..num_voices {
            push_note_on(&queue, 36 + (i % 92) as u8, 100);
        }

        // Warmup
        let mut warmup = vec![0.0f32; 4410];
        graph.process_buffer(&mut warmup);

        let mut buffer = vec![0.0f32; buffer_size];

        let start = Instant::now();
        for _ in 0..iterations {
            graph.process_buffer(&mut buffer);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() as f64 / iterations as f64;

        println!("{:>6}: {:>6.1} µs/buffer", waveform, avg_us);
    }
}

/// Test: Rapid note triggering (keyboard mashing)
#[test]
fn test_rapid_note_triggering() {
    let code = r#"
tempo: 0.5
~synth $ saw ~midi
out $ ~synth * 0.3
"#;

    println!("\n=== RAPID NOTE TRIGGERING ===");

    let (_, statements) = parse_program(code).expect("Parse failed");
    let queue = create_test_queue();
    let mut graph = compile_program(statements, 44100.0, Some(queue.clone()))
        .expect("Compile failed");

    // Simulate rapid key presses - 100 notes in quick succession
    for i in 0..100 {
        let note = 36 + (i % 49) as u8; // C2 to C6
        push_note_on(&queue, note, 100);
    }

    let mut buffer = vec![0.0f32; 256];

    let start = Instant::now();
    graph.process_buffer(&mut buffer);
    let first_buffer = start.elapsed();

    // Process more buffers
    for _ in 0..100 {
        graph.process_buffer(&mut buffer);
    }
    let total = start.elapsed();

    let rms = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("100 simultaneous notes:");
    println!("  First buffer: {:>6.1} µs", first_buffer.as_micros());
    println!("  Avg buffer:   {:>6.1} µs", total.as_micros() as f64 / 101.0);
    println!("  RMS output:   {:.4}", rms);

    // Should not crash and should produce audio
    assert!(rms > 0.001, "Should produce audio with many voices");
}

/// Test: Memory usage estimation
#[test]
fn test_memory_estimation() {
    use std::mem::size_of;
    use phonon::unified_graph::MidiPolyVoice;

    println!("\n=== MEMORY ESTIMATION ===");

    let voice_size = size_of::<MidiPolyVoice>();
    println!("MidiPolyVoice size: {} bytes", voice_size);

    for num_voices in [16, 64, 256, 1024] {
        let memory_kb = (voice_size * num_voices) as f64 / 1024.0;
        println!("  {:>4} voices: {:.1} KB", num_voices, memory_kb);
    }
}
