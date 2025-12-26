//! Test audio output between hybrid and legacy architectures
//! Records audio from both paths and compares them

use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;
const DURATION_SECS: f64 = 5.0;

fn render_to_raw(content: &str, use_hybrid: bool) -> Vec<f32> {
    // Set environment variable to control architecture
    // The code checks ENABLE_HYBRID_ARCH (not DISABLE)
    if use_hybrid {
        std::env::set_var("ENABLE_HYBRID_ARCH", "1");
    } else {
        std::env::remove_var("ENABLE_HYBRID_ARCH");
    }

    // Parse and compile
    let (_, statements) = parse_program(content).expect("Parse failed");
    let mut graph = compile_program(statements, SAMPLE_RATE, None).expect("Compile failed");

    // Don't use wall-clock timing for deterministic results
    // graph.enable_wall_clock_timing();
    graph.preload_samples();

    // Render audio
    let total_samples = (DURATION_SECS * SAMPLE_RATE as f64) as usize;
    let total_buffers = total_samples / BUFFER_SIZE;
    let mut all_samples = Vec::with_capacity(total_samples);
    let mut buffer = [0.0f32; BUFFER_SIZE];

    let start = Instant::now();

    for _ in 0..total_buffers {
        graph.process_buffer(&mut buffer);
        all_samples.extend_from_slice(&buffer);
    }

    let elapsed = start.elapsed();
    let arch_name = if use_hybrid { "Hybrid" } else { "Legacy" };
    eprintln!(
        "{}: Rendered {} samples in {:?}",
        arch_name,
        all_samples.len(),
        elapsed
    );

    all_samples
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: test_arch_audio <file.ph>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let content = std::fs::read_to_string(file_path).expect("Failed to read file");

    println!("=== Architecture Audio Comparison ===");
    println!("File: {}", file_path);
    println!("Duration: {}s", DURATION_SECS);
    println!();

    // Render with legacy path first
    println!("Rendering with LEGACY path...");
    let legacy_samples = render_to_raw(&content, false);

    // Render with hybrid path
    println!("Rendering with HYBRID path...");
    let hybrid_samples = render_to_raw(&content, true);

    // Compare
    println!();
    println!("=== Comparison ===");

    let legacy_rms = calculate_rms(&legacy_samples);
    let hybrid_rms = calculate_rms(&hybrid_samples);

    println!("Legacy RMS: {:.6} ({:.1} dB)", legacy_rms, 20.0 * legacy_rms.log10());
    println!("Hybrid RMS: {:.6} ({:.1} dB)", hybrid_rms, 20.0 * hybrid_rms.log10());

    let rms_diff = (legacy_rms - hybrid_rms).abs();
    let rms_diff_db = 20.0 * (rms_diff / legacy_rms.max(0.0001)).log10();
    println!("RMS Difference: {:.6} ({:.1} dB)", rms_diff, rms_diff_db);

    // Count silent samples
    let legacy_silent = legacy_samples.iter().filter(|&&s| s.abs() < 0.0001).count();
    let hybrid_silent = hybrid_samples.iter().filter(|&&s| s.abs() < 0.0001).count();
    println!();
    println!("Legacy silent samples: {} ({:.1}%)", legacy_silent, legacy_silent as f64 * 100.0 / legacy_samples.len() as f64);
    println!("Hybrid silent samples: {} ({:.1}%)", hybrid_silent, hybrid_silent as f64 * 100.0 / hybrid_samples.len() as f64);

    // Sample-by-sample difference
    let mut max_diff = 0.0f32;
    let mut sum_diff = 0.0f32;
    for (i, (&l, &h)) in legacy_samples.iter().zip(hybrid_samples.iter()).enumerate() {
        let diff = (l - h).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        sum_diff += diff;

        // Print first few differences > 0.01
        if diff > 0.01 && i < 50000 {
            eprintln!("  Sample {}: legacy={:.4}, hybrid={:.4}, diff={:.4}", i, l, h, diff);
        }
    }

    println!();
    println!("Max sample difference: {:.6}", max_diff);
    println!("Avg sample difference: {:.6}", sum_diff / legacy_samples.len() as f32);

    // Write raw audio files for external comparison
    let legacy_path = "/tmp/arch_legacy.raw";
    let hybrid_path = "/tmp/arch_hybrid.raw";

    let mut legacy_file = File::create(legacy_path).expect("Failed to create legacy file");
    for sample in &legacy_samples {
        legacy_file.write_all(&sample.to_le_bytes()).expect("Write failed");
    }

    let mut hybrid_file = File::create(hybrid_path).expect("Failed to create hybrid file");
    for sample in &hybrid_samples {
        hybrid_file.write_all(&sample.to_le_bytes()).expect("Write failed");
    }

    println!();
    println!("Raw audio written to:");
    println!("  {}", legacy_path);
    println!("  {}", hybrid_path);
    println!();
    println!("Convert to WAV with: sox -r 44100 -e float -b 32 -c 1 FILE.raw FILE.wav");

    // Verdict
    println!();
    if rms_diff < 0.001 && max_diff < 0.1 {
        println!("✅ MATCH: Audio is nearly identical");
    } else if rms_diff < 0.01 {
        println!("⚠️  CLOSE: Minor differences detected");
    } else {
        println!("❌ MISMATCH: Significant audio differences!");
    }
}
