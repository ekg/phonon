/// Test timing continuity during rapid code reloads (Ctrl-X spam simulation)
///
/// This test verifies that pressing Ctrl-X multiple times in phonon edit
/// does NOT cause beat drops, timing jumps, or cycle resets.
///
/// TIME IS IMMUTABLE - the cycle position must never jump.

use phonon::ipc::{PatternClient, IpcMessage};
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

mod pattern_verification_utils;
use pattern_verification_utils::detect_audio_events;

/// Read WAV file and return (samples, sample_rate)
fn read_wav_samples(path: &str) -> (Vec<f32>, f32) {
    let mut reader = hound::WavReader::open(path).expect("Failed to open WAV file");
    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;
    
    let samples: Vec<f32> = reader
        .samples::<f32>()
        .map(|s| s.unwrap_or(0.0))
        .collect();
    
    (samples, sample_rate)
}

/// Start phonon-audio in background with recording enabled
fn start_phonon_audio(output_path: &str) -> Child {
    Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--bin",
            "phonon-audio",
            "--",
            "--record",
            output_path,
        ])
        .spawn()
        .expect("Failed to start phonon-audio")
}

/// Detect timing discontinuities by analyzing onset timing
///
/// Returns: (num_discontinuities, expected_interval, actual_intervals)
fn detect_timing_discontinuities(
    samples: &[f32],
    sample_rate: f32,
    expected_interval: f32,
    tolerance: f32,
) -> (usize, f32, Vec<f32>) {
    // Detect audio events (kick drum hits)
    let onsets = detect_audio_events(samples, sample_rate, 0.1);

    if onsets.len() < 2 {
        return (0, expected_interval, vec![]);
    }

    // Calculate intervals between onsets
    let mut intervals = Vec::new();
    let mut discontinuities = 0;

    for i in 1..onsets.len() {
        let interval = (onsets[i].time - onsets[i - 1].time) as f32;
        intervals.push(interval);

        // Check if interval deviates significantly from expected
        let deviation = (interval - expected_interval).abs();
        if deviation > tolerance {
            discontinuities += 1;
            eprintln!(
                "⚠️  Discontinuity detected: expected {:.4}s, got {:.4}s (deviation: {:.4}s)",
                expected_interval, interval, deviation
            );
        }
    }

    (discontinuities, expected_interval, intervals)
}

#[test]
#[ignore] // Ignore by default (requires audio device)
fn test_no_beat_drops_during_rapid_reloads() {
    println!("🧪 Testing timing continuity during rapid code reloads...");
    println!("This simulates pressing Ctrl-X multiple times rapidly in phonon edit");
    println!();

    let output_path = "/tmp/test_timing_continuity.wav";

    // Clean up any previous test file
    let _ = std::fs::remove_file(output_path);

    // Start phonon-audio with recording
    println!("📡 Starting phonon-audio with recording...");
    let mut audio_process = start_phonon_audio(output_path);

    // Give it time to start and create the Unix socket
    thread::sleep(Duration::from_secs(2));

    // Check if phonon-audio is still running
    if audio_process.try_wait().unwrap().is_some() {
        panic!("phonon-audio failed to start");
    }

    println!("✅ phonon-audio started");

    // Connect to phonon-audio via IPC (like phonon edit does)
    println!("🔌 Connecting to audio engine via IPC...");
    let mut client = match PatternClient::connect() {
        Ok(c) => c,
        Err(e) => {
            audio_process.kill().ok();
            panic!("Failed to connect to audio engine: {}", e);
        }
    };

    // Wait for Ready message
    match client.receive() {
        Ok(IpcMessage::Ready) => println!("✅ Audio engine ready"),
        Ok(msg) => {
            audio_process.kill().ok();
            panic!("Expected Ready, got {:?}", msg);
        }
        Err(e) => {
            audio_process.kill().ok();
            panic!("IPC error: {}", e);
        }
    }

    // Test code: simple kick drum pattern
    // At tempo 2.0, kick hits every 0.25 seconds (4 times per cycle)
    let code = r#"
tempo: 2.0
o1 $ s "808bd(1,4)"
"#;

    println!();
    println!("📝 Test code:");
    println!("{}", code);
    println!();

    // Simulate rapid Ctrl-X presses by sending UpdateGraph multiple times
    // This is the critical test: does timing stay continuous?
    let num_reloads = 10;
    println!("🔄 Simulating {} rapid code reloads (Ctrl-X spam)...", num_reloads);

    for i in 0..num_reloads {
        // Send the same code repeatedly (simulating Ctrl-X)
        let msg = IpcMessage::UpdateGraph {
            code: code.to_string(),
        };
        if let Err(e) = client.send(&msg) {
            audio_process.kill().ok();
            panic!("Failed to send UpdateGraph: {}", e);
        }

        // Small delay between reloads to simulate realistic typing/Ctrl-X timing
        // User reported bug happens when spamming Ctrl-X within one cycle
        // At tempo 2.0, one cycle = 0.5 seconds
        thread::sleep(Duration::from_millis(50)); // 20 reloads per second

        if (i + 1) % 3 == 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }
    println!();

    println!("✅ All {} reloads sent", num_reloads);
    println!();

    // Let it play for a few cycles to accumulate data
    println!("🎵 Recording for 3 seconds...");
    thread::sleep(Duration::from_secs(3));

    // Shutdown
    println!("🛑 Shutting down audio engine...");
    if let Err(e) = client.send(&IpcMessage::Shutdown) {
        eprintln!("Warning: Failed to send shutdown: {}", e);
    }

    // Give it time to finalize the WAV file
    thread::sleep(Duration::from_millis(500));

    // Kill if still running
    audio_process.kill().ok();
    audio_process.wait().ok();

    println!("✅ Audio engine stopped");
    println!();

    // Verify WAV file was created
    if !std::path::Path::new(output_path).exists() {
        panic!("Recording file was not created: {}", output_path);
    }

    println!("📊 Analyzing recorded audio for timing discontinuities...");
    println!();

    // Read WAV file
    let (samples, sample_rate) = read_wav_samples(output_path);

    println!("📈 Recording info:");
    println!("   Duration: {:.2}s", samples.len() as f32 / sample_rate);
    println!("   Sample rate: {} Hz", sample_rate as u32);
    println!("   Total samples: {}", samples.len());
    println!();

    // Expected interval: at tempo 2.0, kick pattern "808bd(1,4)" triggers 4 times per cycle
    // Cycle duration = 1 / tempo = 1 / 2.0 = 0.5 seconds
    // Interval between kicks = 0.5 / 4 = 0.125 seconds
    let tempo = 2.0;
    let kicks_per_cycle = 4.0;
    let cycle_duration = 1.0 / tempo;
    let expected_interval = cycle_duration / kicks_per_cycle;

    println!("⏱️  Expected timing:");
    println!("   Tempo: {} CPS", tempo);
    println!("   Cycle duration: {:.3}s", cycle_duration);
    println!("   Expected interval between kicks: {:.3}s", expected_interval);
    println!();

    // Tolerance: allow 5ms deviation (very tight for 125ms intervals)
    let tolerance = 0.005; // 5ms

    let (discontinuities, _, intervals) =
        detect_timing_discontinuities(&samples, sample_rate, expected_interval as f32, tolerance);

    println!("📊 Timing analysis:");
    println!("   Total kick events detected: {}", intervals.len() + 1);
    println!("   Intervals measured: {}", intervals.len());
    println!("   Timing discontinuities: {}", discontinuities);
    println!();

    if !intervals.is_empty() {
        let avg_interval: f32 = intervals.iter().sum::<f32>() / intervals.len() as f32;
        let min_interval = intervals.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_interval = intervals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        println!("   Average interval: {:.4}s", avg_interval);
        println!("   Min interval: {:.4}s", min_interval);
        println!("   Max interval: {:.4}s", max_interval);
        println!("   Expected interval: {:.4}s", expected_interval);
        println!("   Tolerance: {:.4}s (±{:.1}%)", tolerance, tolerance / expected_interval as f32 * 100.0);
        println!();
    }

    // Test assertions
    assert!(
        intervals.len() >= 10,
        "Not enough kick events detected (got {}, expected at least 10)",
        intervals.len()
    );

    // The critical test: NO timing discontinuities
    if discontinuities > 0 {
        println!("❌ FAILED: {} timing discontinuities detected", discontinuities);
        println!();
        println!("This means the beat drops / cycle resets during code reloads.");
        println!("TIME IS IMMUTABLE - cycle position must never jump!");
        println!();
        panic!("Timing continuity test FAILED: {} discontinuities", discontinuities);
    }

    println!("✅ SUCCESS: No timing discontinuities detected!");
    println!("✅ Timing remains continuous during {} rapid reloads", num_reloads);
    println!();
    println!("TIME IS IMMUTABLE ✨");
}

#[test]
#[ignore] // Ignore by default (requires audio device)
fn test_detect_timing_discontinuities_function() {
    println!("🧪 Testing timing discontinuity detection algorithm...");
    println!();

    // Create synthetic audio with known discontinuity
    let sample_rate = 44100.0;
    let duration = 2.0;
    let num_samples = (sample_rate * duration) as usize;
    let mut samples = vec![0.0f32; num_samples];

    // Expected interval: 0.125 seconds (8 Hz)
    let expected_interval = 0.125;
    let interval_samples = (expected_interval * sample_rate) as usize;

    // Place regular kicks every 0.125 seconds
    let kicks = vec![
        0,                   // t=0.000s
        interval_samples,    // t=0.125s
        interval_samples * 2, // t=0.250s
        interval_samples * 3, // t=0.375s
        // DISCONTINUITY: skip one kick, simulate cycle reset
        interval_samples * 4, // t=0.500s (should be at interval_samples * 4)
        interval_samples * 6, // t=0.750s (jumped ahead - discontinuity!)
        interval_samples * 7, // t=0.875s
        interval_samples * 8, // t=1.000s
    ];

    // Place impulses (kick drums)
    for &pos in &kicks {
        if pos < num_samples {
            samples[pos] = 1.0;
            // Decay
            for i in 1..100 {
                if pos + i < num_samples {
                    let i_f32 = i as f32;
                    samples[pos + i] = (-i_f32 * 0.05).exp();
                }
            }
        }
    }

    println!("📊 Synthetic audio:");
    println!("   Sample rate: {} Hz", sample_rate);
    println!("   Duration: {}s", duration);
    println!("   Expected interval: {}s", expected_interval);
    println!("   Kicks placed at indices: {:?}", kicks);
    println!();

    let tolerance = 0.01; // 10ms tolerance
    let (discontinuities, _, intervals) =
        detect_timing_discontinuities(&samples, sample_rate, expected_interval, tolerance);

    println!("📊 Detection results:");
    println!("   Discontinuities detected: {}", discontinuities);
    println!("   Intervals: {:?}", intervals);
    println!();

    // We expect to detect 1 discontinuity (the jump from kick 4 to kick 5)
    assert_eq!(
        discontinuities, 1,
        "Expected to detect exactly 1 discontinuity in synthetic audio"
    );

    println!("✅ Discontinuity detection algorithm works correctly!");
}
