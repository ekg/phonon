//! phonon-perf: Performance profiler for Phonon DSL files
//!
//! Runs the EXACT same code path as phonon-edit but without UI,
//! outputting audio to null and measuring synthesis performance.
//!
//! Usage: phonon-perf <file.ph> [duration_secs]
//!
//! This tool is essential for:
//! - Measuring the impact of optimizations
//! - Detecting performance regressions
//! - Profiling specific .ph files before/after changes

use std::cell::RefCell;
use std::env;
use std::fs;
use std::time::{Duration, Instant};

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;
const BUFFER_SIZE: usize = 512;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: phonon-perf <file.ph> [duration_secs]");
        eprintln!("");
        eprintln!("Runs the EXACT same code path as phonon-edit:");
        eprintln!("  1. Parse and compile the .ph file");
        eprintln!("  2. Enable wall-clock timing (realtime mode)");
        eprintln!("  3. Preload samples");
        eprintln!("  4. Process audio buffers at realtime pace");
        eprintln!("");
        eprintln!("Outputs detailed performance metrics including:");
        eprintln!("  - Min/Max/Avg/Median/P95 buffer processing times");
        eprintln!("  - Underrun count and percentage");
        eprintln!("  - Voice count tracking");
        eprintln!("");
        eprintln!("Example: phonon-perf l.ph 30");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let duration_secs: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10.0);

    // Read the file
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path, e);
            std::process::exit(1);
        }
    };

    println!("\n=== Phonon Performance Profiler ===");
    println!("File: {}", file_path);
    println!("Duration: {}s", duration_secs);
    println!("");

    // === STEP 1: Parse ===
    println!("Step 1: Parsing...");
    let parse_start = Instant::now();
    let statements = match parse_program(&content) {
        Ok((_, stmts)) => stmts,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    };
    let parse_time = parse_start.elapsed();
    println!("  Parsed in {:?} ({} statements)", parse_time, statements.len());

    // === STEP 2: Compile ===
    println!("Step 2: Compiling...");
    let compile_start = Instant::now();
    let mut graph = match compile_program(statements, SAMPLE_RATE, None) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            std::process::exit(1);
        }
    };
    let compile_time = compile_start.elapsed();
    println!("  Compiled in {:?}", compile_time);

    // === STEP 3: Disable wall-clock timing for accurate benchmarking ===
    // (phonon-edit uses wall-clock for realtime sync, but our perf tool simulates time)
    println!("Step 3: Disabling wall-clock timing (simulated time)...");
    graph.use_wall_clock = false;

    // === STEP 4: Preload samples (EXACTLY like phonon-edit) ===
    println!("Step 4: Preloading samples...");
    let preload_start = Instant::now();
    graph.preload_samples();
    let preload_time = preload_start.elapsed();
    println!("  Preloaded in {:?}", preload_time);

    // Wrap in RefCell to simulate modal editor's GraphCell
    let graph_cell = RefCell::new(graph);

    // === STEP 5: Realtime audio simulation ===
    let buffers_per_second = SAMPLE_RATE / BUFFER_SIZE as f32;
    let total_buffers = (duration_secs * buffers_per_second as f64) as usize;
    let buffer_duration = Duration::from_secs_f64(BUFFER_SIZE as f64 / SAMPLE_RATE as f64);
    let budget_us = buffer_duration.as_micros();

    println!("");
    println!("=== Starting Realtime Simulation ===");
    println!("Total buffers: {}", total_buffers);
    println!("Buffer size: {} samples", BUFFER_SIZE);
    println!("Budget per buffer: {:?} ({} µs)", buffer_duration, budget_us);
    println!("");

    let start = Instant::now();
    let mut buffer = [0.0f32; BUFFER_SIZE];
    let mut processed = 0;
    let mut underruns = 0;
    let mut max_time_us = 0u128;
    let mut total_time_us = 0u128;
    let mut times_us: Vec<u128> = Vec::with_capacity(total_buffers);
    let mut voice_counts: Vec<usize> = Vec::with_capacity(total_buffers / 86); // sample every ~1s

    // Process buffers at realtime pace
    for i in 0..total_buffers {
        let expected_time = Duration::from_secs_f64(i as f64 / buffers_per_second as f64);

        // Wait until we're at the right time (simulating realtime audio callback)
        while start.elapsed() < expected_time {
            std::thread::sleep(Duration::from_micros(50));
        }

        let chunk_start = Instant::now();

        match graph_cell.try_borrow_mut() {
            Ok(mut graph) => {
                graph.process_buffer(&mut buffer);
                processed += 1;

                // Sample voice count periodically (every ~1 second)
                if i % 86 == 0 {
                    voice_counts.push(graph.active_voice_count());
                }
            }
            Err(_) => {
                underruns += 1;
                continue;
            }
        }

        let chunk_time = chunk_start.elapsed();

        // Check for extreme jitter (might indicate GC pressure from allocations)
        if i > 0 && chunk_time.as_micros() > times_us.last().copied().unwrap_or(0) * 3 {
            // Time spiked 3x from previous - likely allocation/GC
            if underruns <= 20 {
                eprintln!(
                    "  📈 Spike at buffer {}: {:?} (3x+ previous)",
                    i, chunk_time
                );
            }
        }
        let chunk_us = chunk_time.as_micros();
        times_us.push(chunk_us);
        total_time_us += chunk_us;

        if chunk_us > max_time_us {
            max_time_us = chunk_us;
        }

        // Check if we exceeded budget (underrun)
        if chunk_time > buffer_duration {
            underruns += 1;
            if underruns <= 10 {
                let voice_count = graph_cell
                    .try_borrow()
                    .map(|g| g.active_voice_count())
                    .unwrap_or(0);
                eprintln!(
                    "  ⚠️  Underrun #{}: buffer {} took {:?} ({}% budget) | voices: {}",
                    underruns,
                    i,
                    chunk_time,
                    chunk_us * 100 / budget_us,
                    voice_count
                );
            } else if underruns == 11 {
                eprintln!("  ... (suppressing further underrun messages)");
            }
        }

        // Progress update every 5 seconds
        if i > 0 && i % (5 * 86) == 0 {
            let current_secs = i as f64 / buffers_per_second as f64;
            let avg_voice_count = if voice_counts.is_empty() {
                0
            } else {
                voice_counts.iter().sum::<usize>() / voice_counts.len()
            };
            println!(
                "  [{:>5.1}s] {} buffers, {} underruns, ~{} voices",
                current_secs, processed, underruns, avg_voice_count
            );
        }
    }

    let total_elapsed = start.elapsed();

    // Calculate statistics
    times_us.sort();
    let min_us = times_us.first().copied().unwrap_or(0);
    let median_us = times_us.get(times_us.len() / 2).copied().unwrap_or(0);
    let p95_us = times_us
        .get((times_us.len() as f64 * 0.95) as usize)
        .copied()
        .unwrap_or(0);
    let p99_us = times_us
        .get((times_us.len() as f64 * 0.99) as usize)
        .copied()
        .unwrap_or(0);
    let avg_us = if processed > 0 {
        total_time_us / processed as u128
    } else {
        0
    };

    let avg_voices = if voice_counts.is_empty() {
        0
    } else {
        voice_counts.iter().sum::<usize>() / voice_counts.len()
    };
    let max_voices = voice_counts.iter().max().copied().unwrap_or(0);

    println!("");
    println!("=== RESULTS ===");
    println!("");
    println!("Duration: {:?}", total_elapsed);
    println!("Buffers processed: {}/{}", processed, total_buffers);
    println!(
        "Underruns: {} ({:.2}%)",
        underruns,
        underruns as f64 * 100.0 / total_buffers as f64
    );
    println!("");
    println!("Voice counts:");
    println!("  Average: {} voices", avg_voices);
    println!("  Peak:    {} voices", max_voices);
    println!("");
    println!("Buffer timing (budget: {} µs = {:.2}ms):", budget_us, budget_us as f64 / 1000.0);
    println!(
        "  Min:    {:>6} µs ({:>5.1}% of budget)",
        min_us,
        min_us as f64 * 100.0 / budget_us as f64
    );
    println!(
        "  Avg:    {:>6} µs ({:>5.1}% of budget)",
        avg_us,
        avg_us as f64 * 100.0 / budget_us as f64
    );
    println!(
        "  Median: {:>6} µs ({:>5.1}% of budget)",
        median_us,
        median_us as f64 * 100.0 / budget_us as f64
    );
    println!(
        "  P95:    {:>6} µs ({:>5.1}% of budget)",
        p95_us,
        p95_us as f64 * 100.0 / budget_us as f64
    );
    println!(
        "  P99:    {:>6} µs ({:>5.1}% of budget)",
        p99_us,
        p99_us as f64 * 100.0 / budget_us as f64
    );
    println!(
        "  Max:    {:>6} µs ({:>5.1}% of budget)",
        max_time_us,
        max_time_us as f64 * 100.0 / budget_us as f64
    );
    println!("");

    // CPU utilization estimate
    let cpu_percent = avg_us as f64 * 100.0 / budget_us as f64;
    let headroom = 100.0 - cpu_percent;

    if underruns > 0 {
        println!("❌ FAILED: {} underruns detected!", underruns);
        println!("   Average CPU: {:.1}% (headroom: {:.1}%)", cpu_percent, headroom);
        std::process::exit(1);
    } else {
        println!("✅ PASSED: No underruns!");
        println!("   Average CPU: {:.1}% (headroom: {:.1}%)", cpu_percent, headroom);
    }
}
