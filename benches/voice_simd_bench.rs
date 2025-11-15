//! Benchmarks for SIMD-accelerated voice processing
//!
//! Compares scalar vs SIMD implementations to measure speedup
//!
//! Run with: cargo bench --bench voice_simd_bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

#[cfg(target_arch = "x86_64")]
use phonon::voice_simd::*;

/// Scalar reference implementation: Linear interpolation
fn interpolate_samples_scalar(
    positions: &[f32; 8],
    samples_curr: &[f32; 8],
    samples_next: &[f32; 8],
) -> [f32; 8] {
    let mut result = [0.0f32; 8];
    for i in 0..8 {
        let frac = positions[i] - positions[i].floor();
        result[i] = samples_curr[i] * (1.0 - frac) + samples_next[i] * frac;
    }
    result
}

/// Scalar reference implementation: Equal-power panning
fn apply_panning_scalar(
    samples: &[f32; 8],
    pans: &[f32; 8],
) -> ([f32; 8], [f32; 8]) {
    let mut left = [0.0f32; 8];
    let mut right = [0.0f32; 8];

    for i in 0..8 {
        // Convert pan to radians: (pan + 1.0) * PI/4
        let pan_radians = (pans[i] + 1.0) * std::f32::consts::FRAC_PI_4;

        // Equal-power panning
        let left_gain = pan_radians.cos();
        let right_gain = pan_radians.sin();

        left[i] = samples[i] * left_gain;
        right[i] = samples[i] * right_gain;
    }

    (left, right)
}

/// Benchmark sample interpolation
fn bench_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpolation");

    // Test data
    let positions = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5];
    let samples_curr = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let samples_next = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    // Scalar baseline
    group.bench_function("scalar", |b| {
        b.iter(|| {
            black_box(interpolate_samples_scalar(
                black_box(&positions),
                black_box(&samples_curr),
                black_box(&samples_next),
            ))
        })
    });

    // SIMD implementation
    #[cfg(target_arch = "x86_64")]
    if is_avx2_supported() {
        group.bench_function("simd_avx2", |b| {
            b.iter(|| {
                unsafe {
                    black_box(interpolate_samples_simd_x8(
                        black_box(&positions),
                        black_box(&samples_curr),
                        black_box(&samples_next),
                    ))
                }
            })
        });
    }

    group.finish();
}

/// Benchmark panning
fn bench_panning(c: &mut Criterion) {
    let mut group = c.benchmark_group("panning");

    // Test data
    let samples = [0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.8, 0.6];
    let pans = [-1.0, -0.5, 0.0, 0.5, 1.0, 0.0, -0.75, 0.75];

    // Scalar baseline
    group.bench_function("scalar", |b| {
        b.iter(|| {
            black_box(apply_panning_scalar(
                black_box(&samples),
                black_box(&pans),
            ))
        })
    });

    // SIMD implementation
    #[cfg(target_arch = "x86_64")]
    if is_avx2_supported() {
        group.bench_function("simd_avx2", |b| {
            b.iter(|| {
                unsafe {
                    black_box(apply_panning_simd_x8(
                        black_box(&samples),
                        black_box(&pans),
                    ))
                }
            })
        });
    }

    group.finish();
}

/// Benchmark full voice processing pipeline (when integrated)
fn bench_voice_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("voice_pipeline");

    // Simulate processing 64 voices (8 voices × 8 SIMD batches)
    let batch_count = 8;

    // Test data for 8 voices
    let positions = [100.5, 200.3, 300.7, 400.2, 500.9, 600.1, 700.6, 800.4];
    let samples_curr = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    let samples_next = [0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85];
    let pans = [-0.8, -0.3, 0.2, 0.7, -0.5, 0.9, -1.0, 1.0];

    // Scalar baseline: Process 64 voices sequentially
    group.bench_function(BenchmarkId::new("scalar", "64_voices"), |b| {
        b.iter(|| {
            for _ in 0..batch_count {
                // Interpolate
                let samples = interpolate_samples_scalar(&positions, &samples_curr, &samples_next);
                // Pan
                let _stereo = apply_panning_scalar(&samples, &pans);
            }
        })
    });

    // SIMD: Process 64 voices in batches of 8
    #[cfg(target_arch = "x86_64")]
    if is_avx2_supported() {
        group.bench_function(BenchmarkId::new("simd_avx2", "64_voices"), |b| {
            b.iter(|| {
                for _ in 0..batch_count {
                    unsafe {
                        // Interpolate
                        let samples = interpolate_samples_simd_x8(&positions, &samples_curr, &samples_next);
                        // Pan
                        let _stereo = apply_panning_simd_x8(&samples, &pans);
                    }
                }
            })
        });
    }

    group.finish();
}

/// Benchmark realistic buffer processing (512 samples per voice)
fn bench_buffer_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_processing");
    group.sample_size(50); // Fewer samples for longer benchmarks

    const BUFFER_SIZE: usize = 512;
    const NUM_VOICES: usize = 64;

    // Scalar: Process buffer for all voices sequentially
    group.bench_function("scalar_64_voices_512_samples", |b| {
        b.iter(|| {
            let mut total_left = 0.0f32;
            let mut total_right = 0.0f32;

            // Process each voice
            for _voice in 0..NUM_VOICES {
                // Process each sample in buffer
                for _sample in 0..BUFFER_SIZE {
                    // Simplified voice processing (interpolation + panning)
                    let position: f32 = 100.5;
                    let curr: f32 = 0.5;
                    let next: f32 = 0.6;
                    let pan: f32 = 0.3;

                    // Interpolate
                    let frac = position - position.floor();
                    let sample = curr * (1.0 - frac) + next * frac;

                    // Pan
                    let pan_radians = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                    let left = sample * pan_radians.cos();
                    let right = sample * pan_radians.sin();

                    total_left += left;
                    total_right += right;
                }
            }

            black_box((total_left, total_right))
        })
    });

    // SIMD: Process buffer in batches of 8 voices
    #[cfg(target_arch = "x86_64")]
    if is_avx2_supported() {
        group.bench_function("simd_64_voices_512_samples", |b| {
            b.iter(|| {
                let mut total_left = 0.0f32;
                let mut total_right = 0.0f32;

                // Process 8 voices at a time
                for _batch in 0..(NUM_VOICES / 8) {
                    // Process each sample in buffer
                    for _sample in 0..BUFFER_SIZE {
                        unsafe {
                            let positions = [100.5; 8];
                            let samples_curr = [0.5; 8];
                            let samples_next = [0.6; 8];
                            let pans = [0.3; 8];

                            // SIMD operations
                            let samples = interpolate_samples_simd_x8(&positions, &samples_curr, &samples_next);
                            let (left_batch, right_batch) = apply_panning_simd_x8(&samples, &pans);

                            // Sum results
                            for i in 0..8 {
                                total_left += left_batch[i];
                                total_right += right_batch[i];
                            }
                        }
                    }
                }

                black_box((total_left, total_right))
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_interpolation,
    bench_panning,
    bench_voice_pipeline,
    bench_buffer_processing
);
criterion_main!(benches);
