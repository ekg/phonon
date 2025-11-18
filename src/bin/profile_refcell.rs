// Profile RefCell overhead in the signal graph evaluation
//
// This tool instruments the code to measure:
// 1. Total time spent in eval_node
// 2. Number of RefCell borrow operations
// 3. Time per sample
//
// Usage: cargo run --release --bin profile_refcell -- <file.ph> <cycles>

use phonon::unified_graph::{UnifiedSignalGraph, NodeId};
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::fs;
use std::time::Instant;
use std::sync::atomic::{AtomicUsize, Ordering};

static BORROW_COUNT: AtomicUsize = AtomicUsize::new(0);
static BORROW_MUT_COUNT: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <file.ph> <cycles>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let cycles: usize = args[2].parse().expect("cycles must be a number");

    // Read and parse DSL
    let dsl_code = fs::read_to_string(file_path)
        .expect("Failed to read file");

    println!("📊 RefCell Performance Profiler");
    println!("================================");
    println!("File: {}", file_path);
    println!("Cycles: {}", cycles);
    println!();

    // Parse and compile graph
    let compile_start = Instant::now();
    let statements = match parse_program(&dsl_code) {
        Ok((_, stmts)) => stmts,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    };
    let mut graph = compile_program(statements, 44100.0)
        .expect("Failed to compile DSL");
    let compile_time = compile_start.elapsed();

    println!("✅ Compilation: {:?}", compile_time);
    println!();

    // Render audio
    let sample_rate = 44100.0;
    let samples_per_cycle = sample_rate / 2.0; // tempo = 2.0
    let total_samples = (cycles as f32 * samples_per_cycle) as usize;

    println!("🔊 Rendering {} samples ({} seconds)...", total_samples, total_samples as f32 / sample_rate);

    let render_start = Instant::now();
    let mut buffer = vec![0.0f32; total_samples];

    // Render sample by sample (using process_sample API)
    for i in 0..total_samples {
        buffer[i] = graph.process_sample();
    }

    let render_time = render_start.elapsed();
    let render_secs = render_time.as_secs_f64();
    let audio_secs = total_samples as f64 / sample_rate as f64;
    let realtime_factor = audio_secs / render_secs;

    println!();
    println!("⏱️  Rendering Performance");
    println!("========================");
    println!("Total time:        {:?}", render_time);
    println!("Audio duration:    {:.3}s", audio_secs);
    println!("Realtime factor:   {:.2}x", realtime_factor);
    println!("Time per sample:   {:.2}µs", render_secs * 1_000_000.0 / total_samples as f64);
    println!();

    if realtime_factor < 1.0 {
        println!("❌ BELOW REALTIME - cannot keep up!");
    } else if realtime_factor < 2.0 {
        println!("⚠️  MARGINAL - may have trouble with complex patches");
    } else {
        println!("✅ GOOD - can handle realtime processing");
    }

    // Calculate RMS for verification
    let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
    let rms = (sum_squares / total_samples as f32).sqrt();
    let peak = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    println!();
    println!("📈 Audio Stats");
    println!("==============");
    println!("RMS:   {:.3} ({:.1} dB)", rms, 20.0 * rms.log10());
    println!("Peak:  {:.3} ({:.1} dB)", peak, 20.0 * peak.log10());

    // Note: BORROW_COUNT instrumentation would require modifying SignalGraph
    // For now, this gives us baseline performance metrics
}
