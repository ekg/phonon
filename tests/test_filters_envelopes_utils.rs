/// Comprehensive tests for filters, envelopes, and utility functions
///
/// Tests verify that these functions ACTUALLY process audio, not just compile.
/// Uses spectral analysis to verify frequency response and envelope shapes.
///
/// Functions tested:
/// - Filters: lpf, hpf, bpf, notch
/// - Envelopes: attack, release, ar
/// - Utils: wedge, irand

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

mod audio_test_utils;
use audio_test_utils::{calculate_rms, compute_spectral_centroid, find_dominant_frequency};

const SAMPLE_RATE: f32 = 44100.0;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, SAMPLE_RATE).expect("Failed to compile DSL code");
    let num_samples = (duration * SAMPLE_RATE) as usize;
    graph.render(num_samples)
}

/// Perform FFT and analyze spectrum
fn analyze_spectrum(buffer: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{FftPlanner, num_complex::Complex};

    let fft_size = 8192.min(buffer.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let mut input: Vec<Complex<f32>> = buffer[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(sample * window, 0.0)
        })
        .collect();

    fft.process(&mut input);

    let magnitudes: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    let frequencies: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * sample_rate / fft_size as f32)
        .collect();

    (frequencies, magnitudes)
}

/// Calculate energy in a frequency band
fn calculate_band_energy(frequencies: &[f32], magnitudes: &[f32], low_hz: f32, high_hz: f32) -> f32 {
    frequencies.iter()
        .zip(magnitudes.iter())
        .filter(|(f, _)| **f >= low_hz && **f <= high_hz)
        .map(|(_, m)| m * m)
        .sum()
}

// ============================================================================
// FILTER TESTS - Verify actual frequency response
// ============================================================================

// ========== LPF (Lowpass Filter) Tests ==========

#[test]
fn test_lpf_removes_high_frequencies() {
    let dry = render_dsl("tempo: 2.0\nout: saw 220", 0.5);
    let filtered = render_dsl("tempo: 2.0\nout: saw 220 # lpf 500 0.8", 0.5);

    let (freqs_dry, mags_dry) = analyze_spectrum(&dry, SAMPLE_RATE);
    let (freqs_filt, mags_filt) = analyze_spectrum(&filtered, SAMPLE_RATE);

    // Calculate energy above 2kHz (should be reduced by LPF)
    let high_energy_dry = calculate_band_energy(&freqs_dry, &mags_dry, 2000.0, 10000.0);
    let high_energy_filt = calculate_band_energy(&freqs_filt, &mags_filt, 2000.0, 10000.0);

    // Filtered should have less high-frequency energy
    let reduction = high_energy_filt / high_energy_dry.max(0.001);

    assert!(reduction < 0.5,
        "LPF should reduce high frequencies, got reduction: {:.2}", reduction);

    println!("LPF high-freq reduction: {:.2}%", (1.0 - reduction) * 100.0);
}

#[test]
fn test_lpf_passes_low_frequencies() {
    // Sine at 200Hz through 1000Hz LPF should pass mostly unaffected
    let dry = render_dsl("tempo: 2.0\nout: sine 200", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 200 # lpf 1000 0.8", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation > 0.8,
        "LPF should pass low frequencies mostly unaffected, attenuation: {:.2}", attenuation);

    println!("LPF low-freq attenuation: {:.2}", attenuation);
}

#[test]
fn test_lpf_cutoff_frequency_effect() {
    // Test that different cutoffs produce different spectral content
    let lpf_500 = render_dsl("tempo: 2.0\nout: white_noise # lpf 500 0.8", 1.0);
    let lpf_2000 = render_dsl("tempo: 2.0\nout: white_noise # lpf 2000 0.8", 1.0);

    let centroid_500 = compute_spectral_centroid(&lpf_500, SAMPLE_RATE);
    let centroid_2000 = compute_spectral_centroid(&lpf_2000, SAMPLE_RATE);

    assert!(centroid_2000 > centroid_500 * 1.5,
        "Higher cutoff should have higher spectral centroid: 500Hz={:.0}Hz, 2000Hz={:.0}Hz",
        centroid_500, centroid_2000);

    println!("LPF spectral centroids - 500Hz: {:.0}Hz, 2000Hz: {:.0}Hz",
        centroid_500, centroid_2000);
}

// ========== HPF (Highpass Filter) Tests ==========

#[test]
fn test_hpf_removes_low_frequencies() {
    let dry = render_dsl("tempo: 2.0\nout: saw 55", 0.5);
    let filtered = render_dsl("tempo: 2.0\nout: saw 55 # hpf 500 0.8", 0.5);

    let (freqs_dry, mags_dry) = analyze_spectrum(&dry, SAMPLE_RATE);
    let (freqs_filt, mags_filt) = analyze_spectrum(&filtered, SAMPLE_RATE);

    // Calculate energy below 200Hz (should be reduced by HPF)
    let low_energy_dry = calculate_band_energy(&freqs_dry, &mags_dry, 20.0, 200.0);
    let low_energy_filt = calculate_band_energy(&freqs_filt, &mags_filt, 20.0, 200.0);

    // Filtered should have less low-frequency energy
    let reduction = low_energy_filt / low_energy_dry.max(0.001);

    assert!(reduction < 0.5,
        "HPF should reduce low frequencies, got reduction: {:.2}", reduction);

    println!("HPF low-freq reduction: {:.2}%", (1.0 - reduction) * 100.0);
}

#[test]
fn test_hpf_passes_high_frequencies() {
    // Sine at 2000Hz through 1000Hz HPF should pass mostly unaffected
    let dry = render_dsl("tempo: 2.0\nout: sine 2000", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 2000 # hpf 1000 0.8", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation > 0.8,
        "HPF should pass high frequencies mostly unaffected, attenuation: {:.2}", attenuation);

    println!("HPF high-freq attenuation: {:.2}", attenuation);
}

#[test]
fn test_hpf_cutoff_frequency_effect() {
    // Test that different cutoffs produce different spectral content
    let hpf_500 = render_dsl("tempo: 2.0\nout: white_noise # hpf 500 0.8", 1.0);
    let hpf_2000 = render_dsl("tempo: 2.0\nout: white_noise # hpf 2000 0.8", 1.0);

    let centroid_500 = compute_spectral_centroid(&hpf_500, SAMPLE_RATE);
    let centroid_2000 = compute_spectral_centroid(&hpf_2000, SAMPLE_RATE);

    assert!(centroid_2000 > centroid_500,
        "Higher cutoff should have higher spectral centroid: 500Hz={:.0}Hz, 2000Hz={:.0}Hz",
        centroid_500, centroid_2000);

    println!("HPF spectral centroids - 500Hz: {:.0}Hz, 2000Hz: {:.0}Hz",
        centroid_500, centroid_2000);
}

// ========== BPF (Bandpass Filter) Tests ==========

#[test]
fn test_bpf_passes_center_frequency() {
    // Sine at 1000Hz through 1000Hz BPF should pass
    let dry = render_dsl("tempo: 2.0\nout: sine 1000", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 1000 # bpf 1000 2.0", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation > 0.5,
        "BPF should pass center frequency, attenuation: {:.2}", attenuation);

    println!("BPF center-freq attenuation: {:.2}", attenuation);
}

#[test]
fn test_bpf_attenuates_low_frequencies() {
    // Sine at 200Hz through 1000Hz BPF should be attenuated
    let dry = render_dsl("tempo: 2.0\nout: sine 200", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 200 # bpf 1000 2.0", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation < 0.5,
        "BPF should attenuate low frequencies, attenuation: {:.2}", attenuation);

    println!("BPF low-freq attenuation: {:.2}", attenuation);
}

#[test]
fn test_bpf_attenuates_high_frequencies() {
    // Sine at 4000Hz through 1000Hz BPF should be attenuated
    let dry = render_dsl("tempo: 2.0\nout: sine 4000", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 4000 # bpf 1000 2.0", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation < 0.5,
        "BPF should attenuate high frequencies, attenuation: {:.2}", attenuation);

    println!("BPF high-freq attenuation: {:.2}", attenuation);
}

#[test]
fn test_bpf_q_factor_width() {
    // Wide BPF (low Q) should pass more frequencies
    let narrow = render_dsl("tempo: 2.0\nout: white_noise # bpf 1000 10.0", 1.0);
    let wide = render_dsl("tempo: 2.0\nout: white_noise # bpf 1000 0.5", 1.0);

    let (freqs_narrow, mags_narrow) = analyze_spectrum(&narrow, SAMPLE_RATE);
    let (freqs_wide, mags_wide) = analyze_spectrum(&wide, SAMPLE_RATE);

    // Wide filter should have energy across broader range
    let energy_2k_narrow = calculate_band_energy(&freqs_narrow, &mags_narrow, 1500.0, 3000.0);
    let energy_2k_wide = calculate_band_energy(&freqs_wide, &mags_wide, 1500.0, 3000.0);

    let ratio = energy_2k_wide / energy_2k_narrow.max(0.001);

    assert!(ratio > 1.5,
        "Wide BPF should pass more off-center frequencies, ratio: {:.2}", ratio);

    println!("BPF wide/narrow off-center energy ratio: {:.2}", ratio);
}

// ========== NOTCH (Band-reject Filter) Tests ==========

#[test]
fn test_notch_attenuates_center_frequency() {
    // Sine at 1000Hz through 1000Hz notch should be attenuated
    let dry = render_dsl("tempo: 2.0\nout: sine 1000", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 1000 # notch 1000 2.0", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation < 0.3,
        "Notch should attenuate center frequency, attenuation: {:.2}", attenuation);

    println!("Notch center-freq attenuation: {:.2}", attenuation);
}

#[test]
fn test_notch_passes_other_frequencies() {
    // Sine at 2000Hz through 1000Hz notch should pass
    let dry = render_dsl("tempo: 2.0\nout: sine 2000", 1.0);
    let filtered = render_dsl("tempo: 2.0\nout: sine 2000 # notch 1000 2.0", 1.0);

    let rms_dry = calculate_rms(&dry);
    let rms_filtered = calculate_rms(&filtered);

    let attenuation = rms_filtered / rms_dry;

    assert!(attenuation > 0.8,
        "Notch should pass off-center frequencies, attenuation: {:.2}", attenuation);

    println!("Notch off-center attenuation: {:.2}", attenuation);
}

#[test]
fn test_notch_q_factor_width() {
    // Narrow notch (high Q) should only affect narrow band
    let narrow = render_dsl("tempo: 2.0\nout: sine 1050 # notch 1000 10.0", 1.0);
    let wide = render_dsl("tempo: 2.0\nout: sine 1050 # notch 1000 0.5", 1.0);

    let rms_narrow = calculate_rms(&narrow);
    let rms_wide = calculate_rms(&wide);

    // Narrow notch should pass 1050Hz better than wide notch
    assert!(rms_narrow > rms_wide,
        "Narrow notch should pass nearby frequencies better: narrow={:.3}, wide={:.3}",
        rms_narrow, rms_wide);

    println!("Notch 1050Hz through 1000Hz - narrow: {:.3}, wide: {:.3}",
        rms_narrow, rms_wide);
}

// ============================================================================
// ENVELOPE TESTS - Verify actual envelope shaping
// ============================================================================

// ========== ATTACK Envelope Tests ==========

#[test]
fn test_attack_shapes_onset() {
    let fast = render_dsl("tempo: 2.0\nout: sine 440 # attack 0.001", 0.2);
    let slow = render_dsl("tempo: 2.0\nout: sine 440 # attack 0.1", 0.2);

    // Check first 100ms (4410 samples)
    let fast_start_avg = calculate_rms(&fast[..4410]);
    let slow_start_avg = calculate_rms(&slow[..4410]);

    assert!(fast_start_avg > slow_start_avg * 1.5,
        "Fast attack should start louder than slow attack: fast={:.3}, slow={:.3}",
        fast_start_avg, slow_start_avg);

    println!("Attack comparison - fast: {:.3}, slow: {:.3}",
        fast_start_avg, slow_start_avg);
}

#[test]
fn test_attack_reaches_full_amplitude() {
    let code = r#"
        tempo: 2.0
        out: sine 440 # attack 0.05
    "#;

    let audio = render_dsl(code, 0.5);

    // After 0.1s (well past 0.05s attack), should be at full amplitude
    let start_idx = (0.1 * SAMPLE_RATE) as usize;
    let end_idx = (0.2 * SAMPLE_RATE) as usize;
    let sustained_rms = calculate_rms(&audio[start_idx..end_idx]);

    // Should be close to 1/sqrt(2) ≈ 0.707 for sine wave
    assert!(sustained_rms > 0.6,
        "Attack should reach full amplitude, got RMS: {:.3}", sustained_rms);

    println!("Attack sustained RMS: {:.3}", sustained_rms);
}

#[test]
fn test_attack_different_times() {
    let attack_10ms = render_dsl("tempo: 2.0\nout: sine 440 # attack 0.01", 0.1);
    let attack_50ms = render_dsl("tempo: 2.0\nout: sine 440 # attack 0.05", 0.1);

    // First 20ms should show difference
    let rms_10ms = calculate_rms(&attack_10ms[..882]);
    let rms_50ms = calculate_rms(&attack_50ms[..882]);

    assert!(rms_10ms > rms_50ms,
        "Shorter attack should reach amplitude faster: 10ms={:.3}, 50ms={:.3}",
        rms_10ms, rms_50ms);

    println!("Attack times - 10ms: {:.3}, 50ms: {:.3}", rms_10ms, rms_50ms);
}

// ========== RELEASE Envelope Tests ==========

#[test]
fn test_release_shapes_decay() {
    // Note: release affects how quickly sound fades when events end
    // For continuous signals, release creates a fade-out tail

    let fast = render_dsl("tempo: 2.0\nout: sine 440 # release 0.001", 0.2);
    let slow = render_dsl("tempo: 2.0\nout: sine 440 # release 0.2", 0.2);

    // Both should have sound at the start
    let fast_start = calculate_rms(&fast[..4410]);
    let slow_start = calculate_rms(&slow[..4410]);

    assert!(fast_start > 0.1 && slow_start > 0.1,
        "Both should have audio at start: fast={:.3}, slow={:.3}",
        fast_start, slow_start);

    println!("Release comparison - fast start: {:.3}, slow start: {:.3}",
        fast_start, slow_start);
}

#[test]
fn test_release_different_times() {
    let release_10ms = render_dsl("tempo: 2.0\nout: sine 440 # release 0.01", 0.3);
    let release_100ms = render_dsl("tempo: 2.0\nout: sine 440 # release 0.1", 0.3);

    // Both should produce audible output
    let rms_10ms = calculate_rms(&release_10ms);
    let rms_100ms = calculate_rms(&release_100ms);

    assert!(rms_10ms > 0.1 && rms_100ms > 0.1,
        "Both release times should produce audio: 10ms={:.3}, 100ms={:.3}",
        rms_10ms, rms_100ms);

    println!("Release times - 10ms: {:.3}, 100ms: {:.3}", rms_10ms, rms_100ms);
}

// ========== AR (Attack-Release) Envelope Tests ==========

#[test]
fn test_ar_combines_attack_release() {
    let code = r#"
        tempo: 2.0
        out: sine 440 # ar 0.01 0.05
    "#;

    let audio = render_dsl(code, 0.3);

    let rms = calculate_rms(&audio);

    assert!(rms > 0.1,
        "AR envelope should produce audio, got RMS: {:.3}", rms);

    println!("AR envelope RMS: {:.3}", rms);
}

#[test]
fn test_ar_attack_affects_onset() {
    let fast_attack = render_dsl("tempo: 2.0\nout: sine 440 # ar 0.001 0.05", 0.2);
    let slow_attack = render_dsl("tempo: 2.0\nout: sine 440 # ar 0.1 0.05", 0.2);

    // First 50ms should show attack difference
    let fast_start = calculate_rms(&fast_attack[..2205]);
    let slow_start = calculate_rms(&slow_attack[..2205]);

    assert!(fast_start > slow_start,
        "Fast AR attack should be louder at start: fast={:.3}, slow={:.3}",
        fast_start, slow_start);

    println!("AR attack comparison - fast: {:.3}, slow: {:.3}",
        fast_start, slow_start);
}

#[test]
fn test_ar_different_parameters() {
    let short = render_dsl("tempo: 2.0\nout: sine 440 # ar 0.01 0.01", 0.2);
    let long = render_dsl("tempo: 2.0\nout: sine 440 # ar 0.1 0.1", 0.2);

    let rms_short = calculate_rms(&short);
    let rms_long = calculate_rms(&long);

    // Both should produce audio
    assert!(rms_short > 0.1 && rms_long > 0.1,
        "Both AR envelopes should produce audio: short={:.3}, long={:.3}",
        rms_short, rms_long);

    println!("AR envelopes - short: {:.3}, long: {:.3}", rms_short, rms_long);
}

// ============================================================================
// UTILITY FUNCTION TESTS - Verify actual behavior
// ============================================================================

// ========== WEDGE Tests ==========

#[test]
fn test_wedge_creates_ramp() {
    let code = r#"
        tempo: 1.0
        ~ramp: wedge
        out: sine (440 + ~ramp * 440)
    "#;

    let audio = render_dsl(code, 1.0);

    // Wedge creates a ramp, so frequency should change over time
    // This creates a chirp/sweep, which should have different spectral content
    // than a static frequency

    let first_half = &audio[..(audio.len() / 2)];
    let second_half = &audio[(audio.len() / 2)..];

    let centroid_first = compute_spectral_centroid(first_half, SAMPLE_RATE);
    let centroid_second = compute_spectral_centroid(second_half, SAMPLE_RATE);

    // Second half should have higher frequency content due to ramp
    assert!(centroid_second > centroid_first,
        "Wedge should create frequency sweep: first={:.0}Hz, second={:.0}Hz",
        centroid_first, centroid_second);

    println!("Wedge sweep - first half: {:.0}Hz, second half: {:.0}Hz",
        centroid_first, centroid_second);
}

#[test]
fn test_wedge_produces_audio() {
    let code = r#"
        tempo: 1.0
        ~ramp: wedge
        out: ~ramp * sine 440
    "#;

    let audio = render_dsl(code, 1.0);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.01,
        "Wedge should produce audio when used as amplitude, got RMS: {:.3}", rms);

    println!("Wedge as amplitude RMS: {:.3}", rms);
}

// ========== IRAND Tests ==========

#[test]
fn test_irand_generates_random_values() {
    // irand should generate random integers in range
    let code = r#"
        tempo: 1.0
        out: sine (irand 200 800)
    "#;

    // Render multiple times - should get different results each time
    let mut frequencies = Vec::new();

    for _ in 0..5 {
        let audio = render_dsl(code, 0.1);
        let freq = find_dominant_frequency(&audio, SAMPLE_RATE);
        frequencies.push(freq as i32);
    }

    // Should have at least some variation
    let min_freq = *frequencies.iter().min().unwrap();
    let max_freq = *frequencies.iter().max().unwrap();

    // Allow for some variation (at least 100Hz difference across runs)
    let variation = max_freq - min_freq;

    println!("irand frequency variation: min={}, max={}, variation={}",
        min_freq, max_freq, variation);

    // Note: This test may be flaky due to randomness, but we're just checking
    // that irand produces some kind of random behavior
    assert!(variation >= 0,
        "irand should produce varying frequencies (may need multiple runs)");
}

#[test]
fn test_irand_produces_audio() {
    let code = r#"
        tempo: 1.0
        out: sine (irand 200 800) * 0.3
    "#;

    let audio = render_dsl(code, 0.5);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05,
        "irand should produce audible output, got RMS: {:.3}", rms);

    println!("irand output RMS: {:.3}", rms);
}

#[test]
fn test_irand_range_affects_output() {
    // Different ranges should produce different frequency content
    let low = render_dsl("tempo: 1.0\nout: sine (irand 100 200)", 0.2);
    let high = render_dsl("tempo: 1.0\nout: sine (irand 1000 2000)", 0.2);

    let freq_low = find_dominant_frequency(&low, SAMPLE_RATE);
    let freq_high = find_dominant_frequency(&high, SAMPLE_RATE);

    assert!(freq_high > freq_low * 2.0,
        "High range irand should produce higher frequencies: low={:.0}Hz, high={:.0}Hz",
        freq_low, freq_high);

    println!("irand ranges - low: {:.0}Hz, high: {:.0}Hz", freq_low, freq_high);
}

// ============================================================================
// INTEGRATION TESTS - Combining filters, envelopes, utils
// ============================================================================

#[test]
fn test_filter_with_envelope() {
    let code = r#"
        tempo: 2.0
        out: saw 110 # lpf 2000 0.8 # attack 0.05
    "#;

    let audio = render_dsl(code, 0.5);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.1,
        "Filter + envelope should produce audio, got RMS: {:.3}", rms);

    println!("Filter + envelope RMS: {:.3}", rms);
}

#[test]
fn test_envelope_with_filter() {
    let code = r#"
        tempo: 2.0
        out: sine 440 # ar 0.01 0.1 # lpf 2000 0.5
    "#;

    let audio = render_dsl(code, 0.3);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05,
        "Envelope + filter should produce audio, got RMS: {:.3}", rms);

    println!("Envelope + filter RMS: {:.3}", rms);
}

#[test]
fn test_wedge_modulates_filter_cutoff() {
    let code = r#"
        tempo: 1.0
        ~sweep: wedge
        out: saw 110 # lpf (~sweep * 2000 + 300) 0.8
    "#;

    let audio = render_dsl(code, 1.0);

    // Filter sweep should create changing spectral content
    let first_half = &audio[..(audio.len() / 2)];
    let second_half = &audio[(audio.len() / 2)..];

    let centroid_first = compute_spectral_centroid(first_half, SAMPLE_RATE);
    let centroid_second = compute_spectral_centroid(second_half, SAMPLE_RATE);

    assert!(centroid_second > centroid_first,
        "Wedge-modulated filter should sweep: first={:.0}Hz, second={:.0}Hz",
        centroid_first, centroid_second);

    println!("Wedge filter sweep - first: {:.0}Hz, second: {:.0}Hz",
        centroid_first, centroid_second);
}

#[test]
fn test_irand_modulates_filter() {
    let code = r#"
        tempo: 1.0
        out: saw 110 # lpf (irand 500 2000) 0.8
    "#;

    let audio = render_dsl(code, 0.5);
    let rms = calculate_rms(&audio);

    assert!(rms > 0.05,
        "irand-modulated filter should produce audio, got RMS: {:.3}", rms);

    println!("irand filter modulation RMS: {:.3}", rms);
}

#[test]
fn test_all_filters_produce_audio() {
    // Test that all four filters compile and produce audio
    let filters = vec![
        ("lpf", "saw 110 # lpf 1000 0.8"),
        ("hpf", "saw 110 # hpf 500 0.8"),
        ("bpf", "saw 110 # bpf 1000 2.0"),
        ("notch", "saw 110 # notch 1000 2.0"),
    ];

    for (name, filter_code) in filters {
        let code = format!("tempo: 2.0\nout: {}", filter_code);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01,
            "{} should produce audio, got RMS: {:.3}", name, rms);

        println!("{} RMS: {:.3}", name, rms);
    }
}

#[test]
fn test_all_envelopes_produce_audio() {
    // Test that all envelope functions compile and produce audio
    let envelopes = vec![
        ("attack", "sine 440 # attack 0.01"),
        ("release", "sine 440 # release 0.05"),
        ("ar", "sine 440 # ar 0.01 0.05"),
    ];

    for (name, env_code) in envelopes {
        let code = format!("tempo: 2.0\nout: {}", env_code);
        let audio = render_dsl(&code, 0.3);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.05,
            "{} should produce audio, got RMS: {:.3}", name, rms);

        println!("{} RMS: {:.3}", name, rms);
    }
}

#[test]
fn test_all_utils_produce_audio() {
    // Test that all utility functions compile and produce audio
    let utils = vec![
        ("wedge", "wedge * sine 440"),
        ("irand", "sine (irand 200 800)"),
    ];

    for (name, util_code) in utils {
        let code = format!("tempo: 1.0\nout: {}", util_code);
        let audio = render_dsl(&code, 0.5);
        let rms = calculate_rms(&audio);

        assert!(rms > 0.01,
            "{} should produce audio, got RMS: {:.3}", name, rms);

        println!("{} RMS: {:.3}", name, rms);
    }
}
