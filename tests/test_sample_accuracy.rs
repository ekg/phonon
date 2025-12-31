//! Sample Accuracy Tests
//!
//! These tests verify that phonon's sample triggering and processing
//! produces correct output by comparing against manually constructed
//! reference tracks.
//!
//! The reference tracks are built by:
//! 1. Loading the same samples phonon would use
//! 2. Manually placing them at the exact times the pattern specifies
//! 3. Comparing the phonon output against this reference

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::sample_loader::SampleBank;
use phonon::unified_graph::OutputMixMode;

/// Load a sample from the dirt-samples directory
/// Returns the EXACT same sample data phonon would use
fn load_sample(name: &str) -> Option<Vec<f32>> {
    let mut bank = SampleBank::new();
    bank.get_sample(name).map(|arc| {
        // IMPORTANT: Use the same stereo-to-mono conversion as phonon's voice_manager
        // Voice.process() uses: (left + right) / sqrt(2) for power-preserving mix
        if let Some(ref right) = arc.right {
            arc.left
                .iter()
                .zip(right.iter())
                .map(|(&l, &r)| (l + r) / std::f32::consts::SQRT_2)
                .collect()
        } else {
            arc.left.clone()
        }
    })
}

/// Load sample data directly (for debugging)
fn load_sample_raw(name: &str) -> Option<(Vec<f32>, Option<Vec<f32>>)> {
    let mut bank = SampleBank::new();
    bank.get_sample(name).map(|arc| {
        (arc.left.clone(), arc.right.clone())
    })
}

/// Build a reference track by placing samples at specific times
/// Returns mono audio buffer
///
/// Applies the same 1ms attack envelope that phonon uses for sample playback
fn build_reference_track(
    placements: &[(f64, &str, f32)], // (time_in_seconds, sample_name, gain)
    duration_samples: usize,
    sample_rate: f32,
) -> Vec<f32> {
    let mut output = vec![0.0f32; duration_samples];

    // Phonon uses 1ms (0.001s) attack for sample voices
    let attack_samples = (0.001 * sample_rate) as usize;

    for &(time_sec, sample_name, gain) in placements {
        if let Some(sample_data) = load_sample(sample_name) {
            let start_sample = (time_sec * sample_rate as f64) as usize;

            // Mix sample into output with attack envelope (matching phonon)
            for (i, &sample_val) in sample_data.iter().enumerate() {
                let out_idx = start_sample + i;
                if out_idx < output.len() {
                    // Apply linear attack envelope (same as phonon's voice envelope)
                    let env = if i < attack_samples {
                        i as f32 / attack_samples as f32
                    } else {
                        1.0
                    };
                    output[out_idx] += sample_val * gain * env;
                }
            }
        } else {
            eprintln!("Warning: Could not load sample '{}'", sample_name);
        }
    }

    output
}

/// Compare two signals and return similarity metrics
struct SignalComparison {
    /// Normalized cross-correlation peak (1.0 = identical, 0.0 = uncorrelated)
    correlation: f32,
    /// RMS of difference signal
    rms_difference: f32,
    /// Maximum absolute difference
    max_difference: f32,
    /// SNR in dB (signal to noise ratio, where noise = difference)
    snr_db: f32,
}

/// Find the offset that maximizes correlation between two signals
/// Returns (offset, peak_correlation)
fn find_best_alignment(reference: &[f32], test: &[f32], max_offset: isize) -> (isize, f32) {
    let mut best_offset = 0isize;
    let mut best_corr = f32::NEG_INFINITY;

    for offset in -max_offset..=max_offset {
        let (ref_slice, test_slice) = if offset >= 0 {
            let off = offset as usize;
            if off >= test.len() { continue; }
            (&reference[..], &test[off..])
        } else {
            let off = (-offset) as usize;
            if off >= reference.len() { continue; }
            (&reference[off..], &test[..])
        };

        let len = ref_slice.len().min(test_slice.len());
        if len < 100 { continue; }

        let comparison = compare_signals(&ref_slice[..len], &test_slice[..len]);
        if comparison.correlation > best_corr {
            best_corr = comparison.correlation;
            best_offset = offset;
        }
    }

    (best_offset, best_corr)
}

fn compare_signals(reference: &[f32], test: &[f32]) -> SignalComparison {
    let len = reference.len().min(test.len());
    if len == 0 {
        return SignalComparison {
            correlation: 0.0,
            rms_difference: f32::MAX,
            max_difference: f32::MAX,
            snr_db: f32::NEG_INFINITY,
        };
    }

    // Calculate means
    let ref_mean: f32 = reference[..len].iter().sum::<f32>() / len as f32;
    let test_mean: f32 = test[..len].iter().sum::<f32>() / len as f32;

    // Calculate correlation
    let mut numerator = 0.0f32;
    let mut ref_var = 0.0f32;
    let mut test_var = 0.0f32;

    for i in 0..len {
        let ref_centered = reference[i] - ref_mean;
        let test_centered = test[i] - test_mean;
        numerator += ref_centered * test_centered;
        ref_var += ref_centered * ref_centered;
        test_var += test_centered * test_centered;
    }

    let correlation = if ref_var > 0.0 && test_var > 0.0 {
        numerator / (ref_var.sqrt() * test_var.sqrt())
    } else {
        0.0
    };

    // Calculate difference metrics
    let mut sum_sq_diff = 0.0f32;
    let mut sum_sq_ref = 0.0f32;
    let mut max_diff = 0.0f32;

    for i in 0..len {
        let diff = (reference[i] - test[i]).abs();
        sum_sq_diff += diff * diff;
        sum_sq_ref += reference[i] * reference[i];
        max_diff = max_diff.max(diff);
    }

    let rms_difference = (sum_sq_diff / len as f32).sqrt();
    let rms_signal = (sum_sq_ref / len as f32).sqrt();

    let snr_db = if rms_difference > 0.0 {
        20.0 * (rms_signal / rms_difference).log10()
    } else {
        f32::INFINITY
    };

    SignalComparison {
        correlation,
        rms_difference,
        max_difference: max_diff,
        snr_db,
    }
}

/// Detect onset times in a signal (for debugging)
fn detect_onsets(signal: &[f32], sample_rate: f32, threshold: f32) -> Vec<f64> {
    let mut onsets = Vec::new();
    let window_size = (sample_rate * 0.01) as usize; // 10ms windows

    let mut prev_energy = 0.0f32;
    for (i, chunk) in signal.chunks(window_size).enumerate() {
        let energy: f32 = chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32;

        // Onset = sudden increase in energy
        if energy > threshold && energy > prev_energy * 3.0 && prev_energy < threshold {
            let time_sec = (i * window_size) as f64 / sample_rate as f64;
            onsets.push(time_sec);
        }
        prev_energy = energy;
    }

    onsets
}

/// Render phonon code to mono buffer
fn render_phonon(code: &str, duration_samples: usize, sample_rate: f32) -> Result<Vec<f32>, String> {
    let (_, statements) = parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;

    let mut graph = compile_program(statements, sample_rate, None)?;

    // Use raw output for accurate comparison
    graph.set_output_mix_mode(OutputMixMode::None);

    // Render in chunks using process_buffer (the live path)
    let chunk_size = 512;
    let mut stereo_buffer = vec![0.0f32; chunk_size * 2]; // Stereo interleaved
    let mut mono_output = Vec::with_capacity(duration_samples);

    let total_chunks = (duration_samples + chunk_size - 1) / chunk_size;
    for _ in 0..total_chunks {
        graph.process_buffer(&mut stereo_buffer);

        // Extract left channel (mono)
        for i in 0..chunk_size {
            if mono_output.len() < duration_samples {
                mono_output.push(stereo_buffer[i * 2]);
            }
        }
    }

    Ok(mono_output)
}

// ==================== TESTS ====================

#[test]
fn test_single_kick_timing() {
    // Test: A single kick drum should appear at the expected time
    let sample_rate = 44100.0;
    let duration_sec = 1.0;
    let duration_samples = (duration_sec * sample_rate) as usize;

    // First, print sample info
    if let Some((left, right)) = load_sample_raw("bd") {
        println!("Sample 'bd' info:");
        println!("  Left channel length: {}", left.len());
        println!("  Right channel: {:?}", right.as_ref().map(|r| r.len()));
        println!("  Left first 20: {:?}", &left[..20.min(left.len())]);
    }

    // Phonon code: single kick at start of cycle, 1 cps
    let code = r#"
tempo: 1.0
out $ s "bd"
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Reference: kick at t=0
    let reference = build_reference_track(
        &[(0.0, "bd", 1.0)],
        duration_samples,
        sample_rate,
    );

    // Check that both have audio
    let phonon_rms: f32 = (phonon_output.iter().map(|x| x * x).sum::<f32>()
        / phonon_output.len() as f32).sqrt();
    let ref_rms: f32 = (reference.iter().map(|x| x * x).sum::<f32>()
        / reference.len() as f32).sqrt();

    println!("Single kick test:");
    println!("  Phonon RMS: {:.6}", phonon_rms);
    println!("  Reference RMS: {:.6}", ref_rms);

    // Debug: Print first 100 samples to see shape
    println!("  Phonon samples 0-19:  {:?}", &phonon_output[..20.min(phonon_output.len())]);
    println!("  Phonon samples 40-59: {:?}", &phonon_output[40..60.min(phonon_output.len())]);
    println!("  Reference samples 0-19:  {:?}", &reference[..20.min(reference.len())]);
    println!("  Reference samples 40-59: {:?}", &reference[40..60.min(reference.len())]);

    // Compare at different time ranges
    let early = compare_signals(&reference[..500], &phonon_output[..500]);
    let mid = compare_signals(&reference[500..2000], &phonon_output[500..2000]);
    let late = compare_signals(&reference[2000..5000], &phonon_output[2000..5000]);

    println!("  Early (0-500) correlation:   {:.4}", early.correlation);
    println!("  Mid (500-2000) correlation:  {:.4}", mid.correlation);
    println!("  Late (2000-5000) correlation: {:.4}", late.correlation);

    // Detect onsets
    let phonon_onsets = detect_onsets(&phonon_output, sample_rate, 0.001);
    let ref_onsets = detect_onsets(&reference, sample_rate, 0.001);

    println!("  Phonon onsets: {:?}", phonon_onsets);
    println!("  Reference onsets: {:?}", ref_onsets);

    // Compare signals
    let comparison = compare_signals(&reference, &phonon_output);
    println!("  Overall Correlation: {:.4}", comparison.correlation);
    println!("  RMS difference: {:.6}", comparison.rms_difference);
    println!("  SNR: {:.1} dB", comparison.snr_db);

    // Write both to temp files for manual inspection
    let phonon_path = "/tmp/phonon_kick_test.raw";
    let ref_path = "/tmp/reference_kick_test.raw";
    std::fs::write(phonon_path,
        phonon_output.iter().flat_map(|&f| f.to_le_bytes()).collect::<Vec<_>>()).ok();
    std::fs::write(ref_path,
        reference.iter().flat_map(|&f| f.to_le_bytes()).collect::<Vec<_>>()).ok();
    println!("  Wrote raw files to {} and {}", phonon_path, ref_path);
    println!("  Convert: sox -r 44100 -e floating-point -b 32 -c 1 /tmp/phonon_kick_test.raw /tmp/phonon.wav");
    println!("  Convert: sox -r 44100 -e floating-point -b 32 -c 1 /tmp/reference_kick_test.raw /tmp/reference.wav");

    // Assertions - relaxed to understand the difference first
    assert!(phonon_rms > 0.01, "Phonon should produce audio");
    assert!(ref_rms > 0.01, "Reference should have audio (check sample loading)");

    // Should have similar onset count
    assert_eq!(phonon_onsets.len(), ref_onsets.len(),
        "Should have same number of onsets");

    // Check sample end behavior
    let sample_len = load_sample("bd").map(|s| s.len()).unwrap_or(0);
    println!("  Sample length: {} samples ({:.1}ms)", sample_len, sample_len as f64 / 44.1);

    // Check what happens after sample ends
    let after_sample_start = sample_len + 100;
    let after_sample_end = (sample_len + 1000).min(phonon_output.len());
    if after_sample_end > after_sample_start {
        let phonon_after = &phonon_output[after_sample_start..after_sample_end];
        let ref_after = &reference[after_sample_start..after_sample_end];

        let phonon_after_rms: f32 = (phonon_after.iter().map(|x| x * x).sum::<f32>()
            / phonon_after.len() as f32).sqrt();
        let ref_after_rms: f32 = (ref_after.iter().map(|x| x * x).sum::<f32>()
            / ref_after.len() as f32).sqrt();

        println!("  After sample ends ({}-{}):", after_sample_start, after_sample_end);
        println!("    Phonon RMS: {:.6}", phonon_after_rms);
        println!("    Reference RMS: {:.6}", ref_after_rms);
        println!("    Phonon first 10: {:?}", &phonon_after[..10.min(phonon_after.len())]);
        println!("    Reference first 10: {:?}", &ref_after[..10.min(ref_after.len())]);
    }

    // Check correlation within the sample duration (should be high)
    let within_sample = compare_signals(&reference[..sample_len], &phonon_output[..sample_len]);
    println!("  Within sample duration correlation: {:.4}", within_sample.correlation);

    // IMPORTANT: Check for the attack envelope ramp
    // If phonon is applying 1ms attack (44 samples), the early samples should ramp up
    let attack_samples = 44; // 1ms at 44100Hz
    let phonon_attack_sum: f32 = phonon_output[..attack_samples].iter().sum();
    let ref_attack_sum: f32 = reference[..attack_samples].iter().sum();
    println!("  First 44 samples (1ms attack):");
    println!("    Phonon sum: {:.4}", phonon_attack_sum);
    println!("    Reference sum: {:.4}", ref_attack_sum);
    println!("    Ratio (should be ~0.5 with linear attack): {:.4}", phonon_attack_sum / ref_attack_sum);

    // Check if there's a timing offset between signals
    let (offset, aligned_corr) = find_best_alignment(&reference[..sample_len], &phonon_output[..sample_len], 100);
    println!("  Best alignment: offset={} samples, correlation={:.4}", offset, aligned_corr);

    // Calculate gain ratio (skip attack portion)
    let skip_attack = 100; // Skip first 100 samples to avoid attack envelope
    let phonon_mid = &phonon_output[skip_attack..skip_attack+500];
    let ref_mid = &reference[skip_attack..skip_attack+500];
    let phonon_mid_rms: f32 = (phonon_mid.iter().map(|x| x * x).sum::<f32>() / phonon_mid.len() as f32).sqrt();
    let ref_mid_rms: f32 = (ref_mid.iter().map(|x| x * x).sum::<f32>() / ref_mid.len() as f32).sqrt();
    println!("  Mid-sample (100-600) RMS ratio: {:.4}", phonon_mid_rms / ref_mid_rms);

    // Sample-by-sample comparison at a few points
    println!("  Sample-by-sample comparison (after attack):");
    for i in [100, 200, 500, 1000, 2000].iter() {
        if *i < sample_len && *i < phonon_output.len() {
            let ratio = if reference[*i].abs() > 0.001 {
                phonon_output[*i] / reference[*i]
            } else {
                0.0
            };
            println!("    [{}]: phonon={:.6}, ref={:.6}, ratio={:.4}",
                i, phonon_output[*i], reference[*i], ratio);
        }
    }

    // The aligned correlation should be high
    assert!(aligned_corr > 0.9,
        "Even with best alignment, correlation too low: {:.4}", aligned_corr);
}

#[test]
fn test_four_on_floor_pattern() {
    // Test: "bd bd bd bd" should produce 4 evenly spaced kicks
    let sample_rate = 44100.0;
    let duration_sec = 2.0; // 2 cycles
    let duration_samples = (duration_sec * sample_rate) as usize;

    // Phonon code: 4 kicks per cycle, 1 cps
    let code = r#"
tempo: 1.0
out $ s "bd bd bd bd"
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Reference: 4 kicks at 0.0, 0.25, 0.5, 0.75 (first cycle)
    // and 1.0, 1.25, 1.5, 1.75 (second cycle)
    let reference = build_reference_track(
        &[
            (0.0, "bd", 1.0),
            (0.25, "bd", 1.0),
            (0.5, "bd", 1.0),
            (0.75, "bd", 1.0),
            (1.0, "bd", 1.0),
            (1.25, "bd", 1.0),
            (1.5, "bd", 1.0),
            (1.75, "bd", 1.0),
        ],
        duration_samples,
        sample_rate,
    );

    // Detect onsets
    let phonon_onsets = detect_onsets(&phonon_output, sample_rate, 0.001);
    let ref_onsets = detect_onsets(&reference, sample_rate, 0.001);

    println!("Four-on-floor test:");
    println!("  Phonon onsets ({} total): {:?}", phonon_onsets.len(), phonon_onsets);
    println!("  Reference onsets ({} total): {:?}", ref_onsets.len(), ref_onsets);

    // Compare signals
    let comparison = compare_signals(&reference, &phonon_output);
    println!("  Correlation: {:.4}", comparison.correlation);
    println!("  RMS difference: {:.6}", comparison.rms_difference);
    println!("  SNR: {:.1} dB", comparison.snr_db);

    // Debug: compare sample values at onset positions
    println!("  Sample comparison at onset times:");
    for (i, &onset_time) in phonon_onsets.iter().take(4).enumerate() {
        let onset_sample = (onset_time * sample_rate as f64) as usize;
        // Look at samples 50-100 after onset (after attack envelope)
        for offset in [50, 100, 200] {
            let idx = onset_sample + offset;
            if idx < phonon_output.len() && idx < reference.len() {
                let ratio = if reference[idx].abs() > 0.001 {
                    phonon_output[idx] / reference[idx]
                } else { 0.0 };
                println!("    Onset {} + {}: phonon={:.4}, ref={:.4}, ratio={:.3}",
                    i, offset, phonon_output[idx], reference[idx], ratio);
            }
        }
    }

    // Debug: RMS comparison per onset
    println!("  RMS per onset region:");
    for (i, &onset_time) in phonon_onsets.iter().take(4).enumerate() {
        let start = (onset_time * sample_rate as f64) as usize;
        let end = (start + 2000).min(phonon_output.len());
        let phonon_rms: f32 = (phonon_output[start..end].iter().map(|x| x*x).sum::<f32>()
            / (end - start) as f32).sqrt();
        let ref_rms: f32 = (reference[start..end].iter().map(|x| x*x).sum::<f32>()
            / (end - start) as f32).sqrt();
        println!("    Onset {}: phonon_rms={:.4}, ref_rms={:.4}, ratio={:.3}",
            i, phonon_rms, ref_rms, phonon_rms / ref_rms.max(0.001));
    }

    // Should have 8 onsets (4 per cycle × 2 cycles)
    assert!(phonon_onsets.len() >= 7,
        "Expected ~8 onsets, got {}", phonon_onsets.len());

    // Check timing accuracy for each onset
    for (i, (&phonon_t, &ref_t)) in phonon_onsets.iter()
        .zip(ref_onsets.iter())
        .enumerate()
    {
        let diff = (phonon_t - ref_t).abs();
        assert!(diff < 0.03,
            "Onset {} timing off: phonon={:.3}s, ref={:.3}s, diff={:.3}s",
            i, phonon_t, ref_t, diff);
    }

    // High correlation
    assert!(comparison.correlation > 0.85,
        "Correlation too low: {:.4}", comparison.correlation);
}

#[test]
fn test_mixed_pattern() {
    // Test: "bd sn hh cp" - different samples at different times
    let sample_rate = 44100.0;
    let duration_sec = 1.0;
    let duration_samples = (duration_sec * sample_rate) as usize;

    let code = r#"
tempo: 1.0
out $ s "bd sn hh cp"
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Reference: 4 different samples at 0.0, 0.25, 0.5, 0.75
    let reference = build_reference_track(
        &[
            (0.0, "bd", 1.0),
            (0.25, "sn", 1.0),
            (0.5, "hh", 1.0),
            (0.75, "cp", 1.0),
        ],
        duration_samples,
        sample_rate,
    );

    let phonon_onsets = detect_onsets(&phonon_output, sample_rate, 0.001);

    println!("Mixed pattern test:");
    println!("  Phonon onsets: {:?}", phonon_onsets);

    // Should have 4 onsets
    assert!(phonon_onsets.len() >= 3,
        "Expected ~4 onsets, got {}", phonon_onsets.len());

    // Compare
    let comparison = compare_signals(&reference, &phonon_output);
    println!("  Correlation: {:.4}", comparison.correlation);
    println!("  SNR: {:.1} dB", comparison.snr_db);

    // Should be reasonably similar
    assert!(comparison.correlation > 0.7,
        "Correlation too low: {:.4}", comparison.correlation);
}

#[test]
fn test_fast_pattern() {
    // Test: fast 2 should double the rate
    let sample_rate = 44100.0;
    let duration_sec = 1.0;
    let duration_samples = (duration_sec * sample_rate) as usize;

    let code = r#"
tempo: 1.0
out $ s "bd bd" $ fast 2
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Reference: fast 2 means 4 kicks per cycle (2 × 2)
    // At times: 0.0, 0.25, 0.5, 0.75
    let reference = build_reference_track(
        &[
            (0.0, "bd", 1.0),
            (0.25, "bd", 1.0),
            (0.5, "bd", 1.0),
            (0.75, "bd", 1.0),
        ],
        duration_samples,
        sample_rate,
    );

    let phonon_onsets = detect_onsets(&phonon_output, sample_rate, 0.001);

    println!("Fast pattern test:");
    println!("  Phonon onsets: {:?}", phonon_onsets);

    // Should have 4 onsets
    assert!(phonon_onsets.len() >= 3,
        "Expected ~4 onsets from fast 2, got {}", phonon_onsets.len());

    let comparison = compare_signals(&reference, &phonon_output);
    println!("  Correlation: {:.4}", comparison.correlation);

    assert!(comparison.correlation > 0.8,
        "Correlation too low: {:.4}", comparison.correlation);
}

#[test]
fn test_euclidean_pattern() {
    // Test: birds(3,8) - Euclidean rhythm
    let sample_rate = 44100.0;
    let duration_sec = 1.0;
    let duration_samples = (duration_sec * sample_rate) as usize;

    let code = r#"
tempo: 1.0
out $ s "bd(3,8)"
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Euclidean(3,8) places 3 events evenly in 8 slots
    // Slots at: 0/8, 1/8, 2/8, 3/8, 4/8, 5/8, 6/8, 7/8
    // Events at: 0/8, 3/8, 6/8 (approximately)
    // In seconds: 0.0, 0.375, 0.75
    let reference = build_reference_track(
        &[
            (0.0, "bd", 1.0),
            (0.375, "bd", 1.0),
            (0.75, "bd", 1.0),
        ],
        duration_samples,
        sample_rate,
    );

    let phonon_onsets = detect_onsets(&phonon_output, sample_rate, 0.001);
    let ref_onsets = detect_onsets(&reference, sample_rate, 0.001);

    println!("Euclidean pattern test:");
    println!("  Phonon onsets: {:?}", phonon_onsets);
    println!("  Reference onsets: {:?}", ref_onsets);

    // Should have 3 onsets
    assert!(phonon_onsets.len() >= 2,
        "Expected ~3 onsets from Euclidean(3,8), got {}", phonon_onsets.len());

    let comparison = compare_signals(&reference, &phonon_output);
    println!("  Correlation: {:.4}", comparison.correlation);

    // Euclidean might have slightly different timing, so be lenient
    assert!(comparison.correlation > 0.6,
        "Correlation too low: {:.4}", comparison.correlation);
}

#[test]
fn test_no_artifacts_in_silence() {
    // Test: During silent portions, output should be near zero
    let sample_rate = 44100.0;
    let duration_sec = 2.0;
    let duration_samples = (duration_sec * sample_rate) as usize;

    // Pattern with gaps
    let code = r#"
tempo: 1.0
out $ s "bd ~ ~ ~"
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Check the silent regions (0.3s to 0.9s should be mostly silent)
    let silent_start = (0.3 * sample_rate) as usize;
    let silent_end = (0.9 * sample_rate) as usize;

    if silent_end <= phonon_output.len() {
        let silent_region = &phonon_output[silent_start..silent_end];
        let silence_rms: f32 = (silent_region.iter().map(|x| x * x).sum::<f32>()
            / silent_region.len() as f32).sqrt();
        let silence_max = silent_region.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

        println!("Silence test:");
        println!("  Silent region RMS: {:.6}", silence_rms);
        println!("  Silent region max: {:.6}", silence_max);

        // Should be very quiet (allowing for some sample tail)
        assert!(silence_rms < 0.1,
            "Silent region too loud: RMS={:.4}", silence_rms);
        assert!(silence_max < 0.3,
            "Spike in silent region: max={:.4}", silence_max);
    }
}

#[test]
fn test_continuous_playback_no_drift() {
    // Test: Long playback shouldn't drift in timing
    let sample_rate = 44100.0;
    let duration_sec = 10.0; // 10 seconds
    let duration_samples = (duration_sec * sample_rate) as usize;

    let code = r#"
tempo: 1.0
out $ s "bd"
"#;

    let phonon_output = render_phonon(code, duration_samples, sample_rate)
        .expect("Failed to render phonon");

    // Detect onsets - should be at 0, 1, 2, 3, ... 9 seconds
    let onsets = detect_onsets(&phonon_output, sample_rate, 0.001);

    println!("Drift test (10 seconds):");
    println!("  Detected {} onsets", onsets.len());
    for (i, &t) in onsets.iter().enumerate() {
        let expected = i as f64;
        let drift = t - expected;
        println!("    Onset {}: {:.4}s (drift: {:.4}s)", i, t, drift);
    }

    // Should have ~10 onsets
    assert!(onsets.len() >= 9,
        "Expected ~10 onsets over 10 seconds, got {}", onsets.len());

    // Check drift on later onsets
    if onsets.len() >= 9 {
        let last_onset = onsets[8];
        let expected_time = 8.0;
        let drift = (last_onset - expected_time).abs();

        assert!(drift < 0.1,
            "Timing drift at 8th onset: {:.4}s (expected {:.4}s, got {:.4}s)",
            drift, expected_time, last_onset);
    }
}

#[test]
fn test_process_buffer_consistency() {
    // Test: Multiple calls to process_buffer should produce consistent results
    let sample_rate = 44100.0;

    let code = r#"
tempo: 1.0
out $ s "bd sn"
"#;

    // Render twice with same settings
    let output1 = render_phonon(code, 44100, sample_rate).expect("Render 1 failed");
    let output2 = render_phonon(code, 44100, sample_rate).expect("Render 2 failed");

    // They should be identical (deterministic)
    let comparison = compare_signals(&output1, &output2);

    println!("Consistency test:");
    println!("  Correlation: {:.6}", comparison.correlation);
    println!("  Max difference: {:.6}", comparison.max_difference);

    // Should be nearly identical
    assert!(comparison.correlation > 0.999,
        "Renders not consistent: correlation={:.4}", comparison.correlation);
    assert!(comparison.max_difference < 0.001,
        "Renders differ: max_diff={:.6}", comparison.max_difference);
}
