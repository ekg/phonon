/// Manual profiler to identify synthesis bottlenecks
/// Instruments key hot paths and measures time spent

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load test pattern
    let pattern_code = std::fs::read_to_string("q.ph")?;

    println!("=== PROFILING SYNTHESIS PERFORMANCE ===\n");

    // Compile
    let compile_start = Instant::now();
    let (_, statements) = parse_program(&pattern_code)
        .map_err(|e| format!("Parse error: {:?}", e))?;
    let mut graph = compile_program(statements, 44100.0, None)?;
    println!("✓ Compilation: {:.2}ms\n", compile_start.elapsed().as_micros() as f64 / 1000.0);

    // Warm up (fill voice pool, caches, etc.)
    println!("Warming up (processing 10000 samples)...");
    let warmup_start = Instant::now();
    let mut warmup_buffer = vec![0.0; 10000];
    graph.process_buffer(&mut warmup_buffer);
    println!("Warmup: {:.2}ms ({:.2} samples/ms)\n",
        warmup_start.elapsed().as_micros() as f64 / 1000.0,
        10000.0 / (warmup_start.elapsed().as_micros() as f64 / 1000.0));

    // Profile buffer processing (this is what the background thread does)
    println!("=== PROFILING BUFFER PROCESSING ===");
    let buffer_sizes = [512, 1024, 2048, 4096];

    for &size in &buffer_sizes {
        let mut buffer = vec![0.0; size];
        let start = Instant::now();

        // Process 10 buffers to get average
        for _ in 0..10 {
            graph.process_buffer(&mut buffer);
        }

        let elapsed_ms = start.elapsed().as_micros() as f64 / 1000.0;
        let total_samples = size * 10;
        let samples_per_ms = total_samples as f64 / elapsed_ms;
        let realtime_factor = samples_per_ms / 44.1; // 44.1 samples/ms at 44.1kHz

        println!("Buffer size {}: {:.2}ms for {} samples ({:.0} samples/ms, {:.1}x realtime)",
            size, elapsed_ms, total_samples, samples_per_ms, realtime_factor);
    }

    println!("\n=== VOICE POOL STATUS ===");
    println!("(Voice count information not accessible from external binary)");

    // Calculate required performance
    println!("\n=== PERFORMANCE REQUIREMENTS ===");
    println!("At 44.1kHz, we need to process 44,100 samples/second");
    println!("With 512-sample buffers, that's ~86 buffers/second");
    println!("Each buffer must complete in <11.6ms to avoid underruns");

    // Measure single buffer time repeatedly
    println!("\n=== SINGLE BUFFER TIMING (100 iterations) ===");
    let mut buffer = vec![0.0; 512];
    let mut times = Vec::new();

    for _ in 0..100 {
        let start = Instant::now();
        graph.process_buffer(&mut buffer);
        times.push(start.elapsed().as_micros() as f64 / 1000.0);
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = times[0];
    let max = times[times.len() - 1];
    let median = times[times.len() / 2];
    let p95 = times[(times.len() as f64 * 0.95) as usize];
    let avg: f64 = times.iter().sum::<f64>() / times.len() as f64;

    println!("Min: {:.2}ms", min);
    println!("Median: {:.2}ms", median);
    println!("Average: {:.2}ms", avg);
    println!("P95: {:.2}ms", p95);
    println!("Max: {:.2}ms", max);
    println!("\nTarget: <11.6ms per buffer");

    if p95 > 11.6 {
        println!("\n⚠️  UNDERRUNS LIKELY: P95 time ({:.2}ms) exceeds budget (11.6ms)", p95);
    } else {
        println!("\n✓ Performance OK: P95 time ({:.2}ms) within budget (11.6ms)", p95);
    }

    Ok(())
}
