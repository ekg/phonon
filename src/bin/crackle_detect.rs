/// Crackle / discontinuity detector for rendered WAV files.
///
/// Checks for:
/// 1. Sample-to-sample jumps exceeding a threshold (clicks/pops)
/// 2. Repeated runs of identical samples (buffer stalls)
/// 3. Zero-crossing rate anomalies (distortion regions)
/// 4. Clipping (samples at ±1.0)
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: crackle_detect <wav_file> [--threshold 0.1]");
        std::process::exit(1);
    }

    let filename = &args[1];
    let threshold: f32 = args
        .iter()
        .position(|a| a == "--threshold")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.1);

    let reader = hound::WavReader::open(filename).unwrap_or_else(|e| {
        eprintln!("Failed to open {filename}: {e}");
        std::process::exit(2);
    });

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.into_samples::<f32>().filter_map(|s| s.ok()).collect(),
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (1u32 << (bits - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
    };

    let duration = samples.len() as f32 / sample_rate;
    println!("=== Crackle Detection: {filename} ===");
    println!("Samples: {}  Duration: {duration:.3}s  Rate: {sample_rate}", samples.len());
    println!("Threshold: {threshold}");
    println!();

    // 1. Detect large sample-to-sample jumps (clicks/pops)
    let mut jumps: Vec<(usize, f32, f32, f32)> = Vec::new(); // (index, prev, curr, delta)
    for i in 1..samples.len() {
        let delta = (samples[i] - samples[i - 1]).abs();
        if delta > threshold {
            jumps.push((i, samples[i - 1], samples[i], delta));
        }
    }

    println!("--- 1. DISCONTINUITIES (sample jumps > {threshold}) ---");
    if jumps.is_empty() {
        println!("  None found ✅");
    } else {
        println!("  Found {} discontinuities ⚠️", jumps.len());
        // Show distribution over time
        let bucket_secs = 0.5;
        let num_buckets = (duration / bucket_secs).ceil() as usize;
        let mut buckets = vec![0usize; num_buckets];
        let mut max_delta: f32 = 0.0;
        for &(idx, _, _, delta) in &jumps {
            let t = idx as f32 / sample_rate;
            let b = (t / bucket_secs).min(num_buckets as f32 - 1.0) as usize;
            buckets[b] += 1;
            if delta > max_delta {
                max_delta = delta;
            }
        }
        println!("  Max jump: {max_delta:.6}");
        println!("  Timeline (jumps per {bucket_secs}s window):");
        for (i, &count) in buckets.iter().enumerate() {
            if count > 0 {
                let t = i as f32 * bucket_secs;
                let bar = "#".repeat((count as f32).log2().ceil() as usize + 1);
                println!("    {t:6.1}s: {count:5} {bar}");
            }
        }
        // Show first 10 worst
        let mut worst = jumps.clone();
        worst.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());
        println!("  Top 10 worst jumps:");
        for &(idx, prev, curr, delta) in worst.iter().take(10) {
            let t = idx as f32 / sample_rate;
            println!("    @{t:.4}s (sample {idx}): {prev:.6} → {curr:.6}  Δ={delta:.6}");
        }
    }
    println!();

    // 2. Detect buffer stalls (runs of identical samples)
    let mut stalls: Vec<(usize, usize, f32)> = Vec::new(); // (start, length, value)
    let min_stall = 32; // 32+ identical samples is suspicious
    let mut run_start = 0;
    let mut run_len = 1;
    for i in 1..samples.len() {
        if samples[i] == samples[i - 1] {
            run_len += 1;
        } else {
            if run_len >= min_stall && samples[run_start].abs() > 0.001 {
                stalls.push((run_start, run_len, samples[run_start]));
            }
            run_start = i;
            run_len = 1;
        }
    }
    if run_len >= min_stall && samples[run_start].abs() > 0.001 {
        stalls.push((run_start, run_len, samples[run_start]));
    }

    println!("--- 2. BUFFER STALLS ({}+ identical non-zero samples) ---", min_stall);
    if stalls.is_empty() {
        println!("  None found ✅");
    } else {
        println!("  Found {} stalls ⚠️", stalls.len());
        for (i, &(start, len, val)) in stalls.iter().enumerate().take(20) {
            let t = start as f32 / sample_rate;
            let dur_ms = len as f32 / sample_rate * 1000.0;
            println!("    #{i}: @{t:.4}s  len={len} ({dur_ms:.2}ms)  value={val:.6}");
        }
    }
    println!();

    // 3. Clipping detection
    let clip_threshold = 0.999;
    let clipped: Vec<usize> = samples
        .iter()
        .enumerate()
        .filter(|(_, &s)| s.abs() > clip_threshold)
        .map(|(i, _)| i)
        .collect();

    println!("--- 3. CLIPPING (|sample| > {clip_threshold}) ---");
    if clipped.is_empty() {
        println!("  None found ✅");
    } else {
        println!("  {} clipped samples ⚠️", clipped.len());
        let clip_pct = clipped.len() as f32 / samples.len() as f32 * 100.0;
        println!("  ({clip_pct:.4}% of total)");
    }
    println!();

    // 4. Zero-crossing rate analysis (detect distortion regions)
    let window = (sample_rate * 0.01) as usize; // 10ms windows
    let mut zcr_values: Vec<(f32, f32)> = Vec::new(); // (time, zcr)
    for chunk_start in (0..samples.len().saturating_sub(window)).step_by(window) {
        let chunk = &samples[chunk_start..chunk_start + window];
        let mut crossings = 0u32;
        for j in 1..chunk.len() {
            if (chunk[j] >= 0.0) != (chunk[j - 1] >= 0.0) {
                crossings += 1;
            }
        }
        let zcr = crossings as f32 / window as f32 * sample_rate;
        let t = chunk_start as f32 / sample_rate;
        zcr_values.push((t, zcr));
    }

    // Find anomalous ZCR spikes (> 3 stddev from mean)
    if !zcr_values.is_empty() {
        let mean_zcr: f32 = zcr_values.iter().map(|(_, z)| z).sum::<f32>() / zcr_values.len() as f32;
        let var: f32 = zcr_values.iter().map(|(_, z)| (z - mean_zcr).powi(2)).sum::<f32>()
            / zcr_values.len() as f32;
        let stddev = var.sqrt();

        let anomalies: Vec<&(f32, f32)> = zcr_values
            .iter()
            .filter(|(_, z)| (z - mean_zcr).abs() > 3.0 * stddev)
            .collect();

        println!("--- 4. ZERO-CROSSING RATE ---");
        println!("  Mean: {mean_zcr:.0} Hz  Stddev: {stddev:.0} Hz");
        if anomalies.is_empty() {
            println!("  No anomalies ✅");
        } else {
            println!("  {} anomalous windows (>3σ) ⚠️", anomalies.len());
            for &&(t, zcr) in anomalies.iter().take(10) {
                println!("    @{t:.3}s: {zcr:.0} Hz");
            }
        }
    }
    println!();

    // 5. Summary
    let has_issues = !jumps.is_empty() || !stalls.is_empty() || !clipped.is_empty();
    if has_issues {
        println!("⚠️  CRACKLE DETECTED — see above for details");
    } else {
        println!("✅ No crackle artifacts detected");
    }
}
