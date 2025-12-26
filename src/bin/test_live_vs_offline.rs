//! Comprehensive test: Live (phonon-audio) vs Offline rendering comparison
//!
//! This test:
//! 1. Renders a pattern OFFLINE (deterministic)
//! 2. Spawns phonon-audio with --record and sends the same pattern
//! 3. Compares the two outputs for audio similarity
//!
//! Note: This binary is Unix-only (requires Unix domain sockets).

#[cfg(not(unix))]
fn main() {
    eprintln!("test_live_vs_offline is only supported on Unix platforms");
    std::process::exit(1);
}

#[cfg(unix)]
use hound::{WavReader, WavSpec, WavWriter};
#[cfg(unix)]
use phonon::compositional_compiler::compile_program;
#[cfg(unix)]
use phonon::compositional_parser::parse_program;
#[cfg(unix)]
use phonon::ipc::{IpcMessage, PatternClient};
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
const SAMPLE_RATE: f32 = 44100.0;
#[cfg(unix)]
const DURATION_SECS: f32 = 5.0;

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The test pattern - can be overridden by command line arg
    let test_code = std::env::args().nth(1).unwrap_or_else(|| {
        r#"
cps: 1.0
o1 $ s "v east*8"
o2 $ s "birds(3,8)" # note "<b3 a2 c3>" # speed 0.25 # delay 0.8 0.5
o3 $ s "bend" # n "<0 1 2 3 4 5 6 7 8 9>" # delay 0.3 0.4
"#
        .to_string()
    });

    eprintln!("=== Live vs Offline Rendering Comparison Test ===\n");
    eprintln!("Test pattern:\n{}\n", test_code);

    // Step 1: Render OFFLINE (deterministic)
    eprintln!("STEP 1: Rendering OFFLINE...");
    let offline_path = "/tmp/phonon_test_offline.wav";
    render_offline(&test_code, offline_path)?;
    eprintln!("   Written to: {}\n", offline_path);

    // Step 2: Render LIVE via phonon-audio
    eprintln!("STEP 2: Rendering LIVE via phonon-audio...");
    let live_path = "/tmp/phonon_test_live.wav";
    render_live(&test_code, live_path)?;
    eprintln!("   Written to: {}\n", live_path);

    // Step 3: Compare the outputs
    eprintln!("STEP 3: Comparing outputs...\n");
    compare_wav_files(offline_path, live_path)?;

    Ok(())
}

#[cfg(unix)]
fn render_offline(code: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (rest, statements) = parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;

    if !rest.trim().is_empty() {
        return Err(format!("Failed to parse entire code, remaining: {}", rest).into());
    }

    let mut graph = compile_program(statements, SAMPLE_RATE, None)?;
    graph.set_cps(1.0);

    let num_samples = (SAMPLE_RATE * DURATION_SECS) as usize;
    let buffer = graph.render(num_samples);

    // Write to WAV
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = WavWriter::create(output_path, spec)?;
    for sample in &buffer {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;

    // Report stats
    let rms = calculate_rms(&buffer);
    let peak = buffer.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));
    eprintln!(
        "   Offline stats: RMS={:.4} ({:.1}dB), Peak={:.4} ({:.1}dB)",
        rms,
        to_db(rms),
        peak,
        to_db(peak)
    );

    Ok(())
}

#[cfg(unix)]
fn render_live(code: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Clean up any existing socket
    let _ = std::fs::remove_file("/tmp/phonon_audio.sock");

    // Spawn audio engine with --record option
    eprintln!("   Spawning phonon-audio with --record...");
    let mut audio_process = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--bin",
            "phonon-audio",
            "--",
            "--record",
            output_path,
        ])
        .spawn()?;

    // Give audio engine time to start
    thread::sleep(Duration::from_millis(1500));

    // Connect to audio engine
    eprintln!("   Connecting to audio engine...");
    let mut client = match PatternClient::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("   Failed to connect: {}. Killing audio process.", e);
            audio_process.kill()?;
            return Err(e.into());
        }
    };

    // Wait for Ready message
    eprintln!("   Waiting for Ready...");
    match client.receive()? {
        IpcMessage::Ready => eprintln!("   Audio engine ready"),
        msg => eprintln!("   Unexpected message: {:?}", msg),
    }

    // Send the pattern
    eprintln!("   Sending pattern...");
    client.send(&IpcMessage::UpdateGraph {
        code: code.to_string(),
    })?;

    // Wait for rendering time plus a small margin
    let wait_time = DURATION_SECS + 0.5;
    eprintln!(
        "   Recording for {:.1} seconds...",
        wait_time
    );
    thread::sleep(Duration::from_secs_f32(wait_time));

    // Shutdown
    eprintln!("   Sending Shutdown...");
    client.send(&IpcMessage::Shutdown)?;

    // Wait for audio process to exit
    audio_process.wait()?;
    eprintln!("   Audio engine stopped");

    // Wait a bit for file to be finalized
    thread::sleep(Duration::from_millis(500));

    Ok(())
}

#[cfg(unix)]
fn compare_wav_files(
    offline_path: &str,
    live_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read offline WAV
    let mut offline_reader = WavReader::open(offline_path)?;
    let offline_samples: Vec<f32> = offline_reader
        .samples::<f32>()
        .map(|s| s.unwrap_or(0.0))
        .collect();

    // Read live WAV
    let mut live_reader = WavReader::open(live_path)?;
    let live_spec = live_reader.spec();

    // Live may be stereo (2 channels), so we need to handle that
    let live_samples_raw: Vec<f32> = live_reader
        .samples::<f32>()
        .map(|s| s.unwrap_or(0.0))
        .collect();

    // Convert to mono if stereo
    let live_samples: Vec<f32> = if live_spec.channels == 2 {
        eprintln!("   Live recording is stereo, converting to mono for comparison...");
        live_samples_raw
            .chunks(2)
            .map(|ch| (ch[0] + ch.get(1).unwrap_or(&0.0)) / 2.0)
            .collect()
    } else {
        live_samples_raw
    };

    eprintln!("Offline samples: {}", offline_samples.len());
    eprintln!("Live samples: {}", live_samples.len());

    // Calculate stats
    let offline_rms = calculate_rms(&offline_samples);
    let offline_peak = offline_samples
        .iter()
        .map(|x| x.abs())
        .fold(0.0f32, |a, b| a.max(b));

    let live_rms = calculate_rms(&live_samples);
    let live_peak = live_samples
        .iter()
        .map(|x| x.abs())
        .fold(0.0f32, |a, b| a.max(b));

    eprintln!("\n=== COMPARISON RESULTS ===\n");
    eprintln!(
        "OFFLINE:  RMS={:.4} ({:.1}dB)  Peak={:.4} ({:.1}dB)",
        offline_rms,
        to_db(offline_rms),
        offline_peak,
        to_db(offline_peak)
    );
    eprintln!(
        "LIVE:     RMS={:.4} ({:.1}dB)  Peak={:.4} ({:.1}dB)",
        live_rms,
        to_db(live_rms),
        live_peak,
        to_db(live_peak)
    );

    // Check for issues
    let mut issues: Vec<String> = Vec::new();

    // Issue 1: Live has no audio
    if live_rms < 0.001 {
        issues.push("CRITICAL: Live rendering produced near-silence (RMS < 0.001)".to_string());
    }

    // Issue 2: Massive RMS difference (more than 10dB)
    let rms_diff_db = (to_db(live_rms) - to_db(offline_rms)).abs();
    if rms_diff_db > 10.0 {
        issues.push(format!(
            "WARNING: RMS differs by {:.1}dB (live vs offline)",
            rms_diff_db
        ));
    }

    // Issue 3: Peak clipping in live (but only if offline doesn't clip too)
    if live_peak > 0.99 && offline_peak < 0.95 {
        issues.push("WARNING: Live output is clipping (peak > 0.99) but offline doesn't".to_string());
    }

    // Issue 4: Spectral analysis - check for high-frequency noise (screeching)
    let live_hf_energy = calculate_high_freq_energy(&live_samples);
    let offline_hf_energy = calculate_high_freq_energy(&offline_samples);

    eprintln!("\nHigh-frequency energy (potential screeching indicator):");
    eprintln!("   Offline HF energy: {:.6}", offline_hf_energy);
    eprintln!("   Live HF energy:    {:.6}", live_hf_energy);

    if live_hf_energy > offline_hf_energy * 5.0 && live_hf_energy > 0.01 {
        issues.push("CRITICAL: Live has excessive high-frequency energy (potential screeching)".to_string());
    }

    // Report results
    eprintln!("\n=== VERDICT ===\n");
    if issues.is_empty() {
        eprintln!("✅ PASS: Live rendering matches offline within acceptable tolerances");
    } else {
        eprintln!("❌ FAIL: Issues detected:");
        for issue in &issues {
            eprintln!("   - {}", issue);
        }
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(unix)]
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|x| x * x).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(unix)]
fn to_db(linear: f32) -> f32 {
    20.0 * (linear + 1e-10).log10()
}

#[cfg(unix)]
fn calculate_high_freq_energy(samples: &[f32]) -> f32 {
    // Simple high-pass filter to detect high frequency content
    // Uses a first-order difference (very crude HPF)
    // Frequencies above ~sample_rate/4 will have more energy after this

    if samples.len() < 2 {
        return 0.0;
    }

    let mut hf_energy = 0.0f32;
    for i in 1..samples.len() {
        let diff = samples[i] - samples[i - 1];
        hf_energy += diff * diff;
    }
    hf_energy / samples.len() as f32
}
