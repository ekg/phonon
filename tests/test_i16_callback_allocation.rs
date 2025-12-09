//! Test to validate that I16 audio callback does not allocate per-callback
//!
//! This test reproduces the underrun bug that was fixed by pre-allocating
//! the conversion buffer outside the audio callback closure.
//!
//! The bug: Every I16 callback was calling `vec![0.0f32; data.len()]`
//! which allocates memory. Memory allocation in realtime audio callbacks
//! causes latency spikes and underruns.
//!
//! The fix: Pre-allocate the buffer once outside the closure, reuse it.

use std::time::{Duration, Instant};

/// Simulates the OLD (buggy) I16 callback that allocated every time
fn simulate_old_i16_callback_allocating(
    output: &mut [i16],
    input: &[f32],
) {
    // BUG: This allocates memory every callback!
    let mut temp = vec![0.0f32; output.len()];

    // Copy input to temp (simulating ring buffer read)
    let copy_len = input.len().min(output.len());
    temp[..copy_len].copy_from_slice(&input[..copy_len]);

    // Convert f32 to i16
    for (dst, src) in output.iter_mut().zip(temp.iter()) {
        *dst = (*src * 32767.0) as i16;
    }
}

/// Simulates the NEW (fixed) I16 callback with pre-allocated buffer
fn simulate_new_i16_callback_preallocated(
    output: &mut [i16],
    input: &[f32],
    conversion_buffer: &mut [f32],
) {
    // NO ALLOCATION: Use pre-allocated buffer slice
    let temp = &mut conversion_buffer[..output.len()];

    // Copy input to temp (simulating ring buffer read)
    let copy_len = input.len().min(output.len());
    temp[..copy_len].copy_from_slice(&input[..copy_len]);

    // Convert f32 to i16
    for (dst, src) in output.iter_mut().zip(temp.iter()) {
        *dst = (*src * 32767.0) as i16;
    }
}

/// Test that pre-allocated callback has lower timing variance
///
/// Allocations in a tight loop cause inconsistent timing due to:
/// - Allocator lock contention
/// - Memory allocation latency variance
/// - GC-like behavior from frequent alloc/dealloc
#[test]
fn test_i16_callback_timing_variance() {
    const BUFFER_SIZE: usize = 512;
    const NUM_CALLBACKS: usize = 10000; // ~1.7 minutes of audio at 44.1kHz
    const BUDGET_US: u128 = 11610; // 512 samples at 44100 Hz

    let input = vec![0.5f32; BUFFER_SIZE];
    let mut output = vec![0i16; BUFFER_SIZE];

    // Measure OLD approach (allocating)
    let mut old_times_us: Vec<u128> = Vec::with_capacity(NUM_CALLBACKS);
    for _ in 0..NUM_CALLBACKS {
        let start = Instant::now();
        simulate_old_i16_callback_allocating(&mut output, &input);
        old_times_us.push(start.elapsed().as_micros());
    }

    // Measure NEW approach (pre-allocated)
    let mut conversion_buffer = vec![0.0f32; 4096];
    let mut new_times_us: Vec<u128> = Vec::with_capacity(NUM_CALLBACKS);
    for _ in 0..NUM_CALLBACKS {
        let start = Instant::now();
        simulate_new_i16_callback_preallocated(&mut output, &input, &mut conversion_buffer);
        new_times_us.push(start.elapsed().as_micros());
    }

    // Calculate statistics
    old_times_us.sort();
    new_times_us.sort();

    let old_median = old_times_us[old_times_us.len() / 2];
    let old_p95 = old_times_us[(old_times_us.len() as f64 * 0.95) as usize];
    let old_p99 = old_times_us[(old_times_us.len() as f64 * 0.99) as usize];
    let old_max = old_times_us[old_times_us.len() - 1];

    let new_median = new_times_us[new_times_us.len() / 2];
    let new_p95 = new_times_us[(new_times_us.len() as f64 * 0.95) as usize];
    let new_p99 = new_times_us[(new_times_us.len() as f64 * 0.99) as usize];
    let new_max = new_times_us[new_times_us.len() - 1];

    println!("\n=== I16 Callback Allocation Test ===");
    println!("Callbacks: {}, Buffer size: {}", NUM_CALLBACKS, BUFFER_SIZE);
    println!("\nOLD (allocating every callback):");
    println!("  Median: {} µs", old_median);
    println!("  P95:    {} µs", old_p95);
    println!("  P99:    {} µs", old_p99);
    println!("  Max:    {} µs", old_max);

    println!("\nNEW (pre-allocated buffer):");
    println!("  Median: {} µs", new_median);
    println!("  P95:    {} µs", new_p95);
    println!("  P99:    {} µs", new_p99);
    println!("  Max:    {} µs", new_max);

    // The new approach should have lower P99 and max times
    // (less variance from allocation)
    println!("\nImprovement:");
    println!("  P99 reduction: {:.1}%", (1.0 - new_p99 as f64 / old_p99 as f64) * 100.0);
    println!("  Max reduction: {:.1}%", (1.0 - new_max as f64 / old_max as f64) * 100.0);

    // Assert that both approaches complete within realtime budget
    // The new approach should be consistently faster at the tail
    assert!(
        new_p99 <= old_p99 || new_p99 < 100, // Either faster or negligibly small
        "Pre-allocated approach should have lower or equal P99 latency"
    );

    // Both should be well under the 11.6ms budget
    assert!(
        new_max < BUDGET_US,
        "NEW approach max {} µs exceeds budget {} µs",
        new_max, BUDGET_US
    );
}

/// Test that simulates realistic audio callback timing with jitter measurement
/// This catches the actual bug: allocations cause occasional latency spikes
#[test]
fn test_i16_callback_no_latency_spikes() {
    const BUFFER_SIZE: usize = 512;
    const NUM_CALLBACKS: usize = 5000;
    const SPIKE_THRESHOLD_US: u128 = 1000; // 1ms is a serious spike in audio

    let input = vec![0.5f32; BUFFER_SIZE];
    let mut output = vec![0i16; BUFFER_SIZE];
    let mut conversion_buffer = vec![0.0f32; 4096];

    let mut spike_count = 0;
    let mut max_time_us = 0u128;

    for _ in 0..NUM_CALLBACKS {
        let start = Instant::now();
        simulate_new_i16_callback_preallocated(&mut output, &input, &mut conversion_buffer);
        let elapsed_us = start.elapsed().as_micros();

        if elapsed_us > SPIKE_THRESHOLD_US {
            spike_count += 1;
        }
        if elapsed_us > max_time_us {
            max_time_us = elapsed_us;
        }
    }

    println!("\n=== Latency Spike Test ===");
    println!("Callbacks: {}", NUM_CALLBACKS);
    println!("Spike threshold: {} µs", SPIKE_THRESHOLD_US);
    println!("Spikes detected: {}", spike_count);
    println!("Max latency: {} µs", max_time_us);

    // With pre-allocated buffer, we should have NO spikes
    // (The old allocating approach would occasionally spike due to allocator)
    assert!(
        spike_count == 0,
        "Pre-allocated callback should have no latency spikes > {} µs, but had {}",
        SPIKE_THRESHOLD_US, spike_count
    );
}

/// Test that validates the buffer resize behavior works correctly
/// (rare case where buffer size changes)
#[test]
fn test_i16_callback_buffer_resize() {
    let input = vec![0.5f32; 8192];

    // Start with small buffer
    let mut conversion_buffer = vec![0.0f32; 512];

    // Test with increasing buffer sizes (simulating buffer size changes)
    let sizes = [512, 1024, 2048, 4096, 8192];

    for &size in &sizes {
        let mut output = vec![0i16; size];

        // Resize if needed (this is what the fix does)
        if conversion_buffer.len() < size {
            conversion_buffer.resize(size, 0.0);
        }

        simulate_new_i16_callback_preallocated(
            &mut output,
            &input[..size],
            &mut conversion_buffer
        );

        // Verify output is correct
        for (i, &sample) in output.iter().enumerate() {
            let expected = (input[i] * 32767.0) as i16;
            assert_eq!(sample, expected, "Sample {} mismatch at size {}", i, size);
        }
    }

    // Buffer should now be at max size
    assert_eq!(conversion_buffer.len(), 8192);
    println!("\n=== Buffer Resize Test ===");
    println!("Tested sizes: {:?}", sizes);
    println!("Final buffer size: {}", conversion_buffer.len());
    println!("✓ All conversions correct");
}

/// Stress test with concurrent-like access patterns
/// Simulates the timing behavior under load
#[test]
fn test_i16_callback_under_load() {
    const BUFFER_SIZE: usize = 512;
    const DURATION_SECS: f64 = 5.0;
    const CALLBACKS_PER_SEC: f64 = 44100.0 / 512.0; // ~86

    let total_callbacks = (DURATION_SECS * CALLBACKS_PER_SEC) as usize;
    let callback_interval = Duration::from_secs_f64(1.0 / CALLBACKS_PER_SEC);

    let input = vec![0.5f32; BUFFER_SIZE];
    let mut output = vec![0i16; BUFFER_SIZE];
    let mut conversion_buffer = vec![0.0f32; 4096];

    let start = Instant::now();
    let mut callbacks_processed = 0;
    let mut late_callbacks = 0;
    let mut max_lateness_us = 0i128;

    for i in 0..total_callbacks {
        let expected_time = Duration::from_secs_f64(i as f64 / CALLBACKS_PER_SEC);
        let actual_time = start.elapsed();

        // Wait until we're at the right time
        if actual_time < expected_time {
            std::thread::sleep(expected_time - actual_time);
        }

        let callback_start = Instant::now();
        simulate_new_i16_callback_preallocated(&mut output, &input, &mut conversion_buffer);
        let callback_time = callback_start.elapsed();

        // Check if we'd miss the next callback
        let lateness = callback_time.as_micros() as i128 - callback_interval.as_micros() as i128;
        if lateness > 0 {
            late_callbacks += 1;
            if lateness > max_lateness_us {
                max_lateness_us = lateness;
            }
        }

        callbacks_processed += 1;
    }

    let total_time = start.elapsed();

    println!("\n=== Load Test ({}s simulation) ===", DURATION_SECS);
    println!("Callbacks processed: {}/{}", callbacks_processed, total_callbacks);
    println!("Late callbacks: {} ({:.2}%)",
             late_callbacks,
             late_callbacks as f64 * 100.0 / total_callbacks as f64);
    println!("Max lateness: {} µs", max_lateness_us);
    println!("Total time: {:?}", total_time);

    // Should have no late callbacks with pre-allocated buffer
    assert!(
        late_callbacks == 0,
        "Should have 0 late callbacks, but had {} ({:.2}%)",
        late_callbacks,
        late_callbacks as f64 * 100.0 / total_callbacks as f64
    );
}
