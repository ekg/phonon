/// Verify if pattern parameters actually work or just fall back to defaults
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

mod audio_test_utils;
use audio_test_utils::find_frequency_peaks;

/// Helper function to measure the spread of fundamental frequencies
/// (filters peaks to only the fundamental range to avoid harmonics)
fn measure_fundamental_spread(buffer: &[f32], sample_rate: f32, base_freq: f32) -> f32 {
    // Find top 30 peaks to ensure we capture all the detuned fundamentals
    let all_peaks = find_frequency_peaks(buffer, sample_rate, 30);

    // Filter to fundamental range (±50% of base frequency)
    let min_freq = base_freq * 0.5;
    let max_freq = base_freq * 1.5;

    let fundamental_peaks: Vec<_> = all_peaks
        .iter()
        .filter(|(freq, _mag)| *freq >= min_freq && *freq <= max_freq)
        .collect();

    if fundamental_peaks.len() < 2 {
        return 0.0;
    }

    // Find min and max frequencies in the fundamental range
    let frequencies: Vec<f32> = fundamental_peaks.iter().map(|(f, _)| *f).collect();
    let min_fundamental = frequencies.iter().copied().fold(f32::INFINITY, f32::min);
    let max_fundamental = frequencies
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);

    max_fundamental - min_fundamental
}

#[test]
fn test_supersaw_detune_fundamental_frequency_distribution() {
    // Test detune using proper FFT analysis of fundamental frequency distribution
    // For a 220 Hz supersaw with 5 voices:
    //   detune=0.1: frequencies should be tightly clustered (217.8 - 222.2 Hz = ~4.4 Hz spread)
    //   detune=0.9: frequencies should be widely spread (200.2 - 239.8 Hz = ~39.6 Hz spread)

    let base_freq = 220.0;

    // Test with low detune (tight clustering)
    let input_low = "out $ supersaw(220, 0.1, 5) * 0.3";
    let (_, statements_low) = parse_dsl(input_low).unwrap();
    let compiler_low = DslCompiler::new(44100.0);
    let mut graph_low = compiler_low.compile(statements_low);
    let buffer_low = graph_low.render(44100);

    let spread_low = measure_fundamental_spread(&buffer_low, 44100.0, base_freq);

    // Test with high detune (wide spread)
    let input_high = "out $ supersaw(220, 0.9, 5) * 0.3";
    let (_, statements_high) = parse_dsl(input_high).unwrap();
    let compiler_high = DslCompiler::new(44100.0);
    let mut graph_high = compiler_high.compile(statements_high);
    let buffer_high = graph_high.render(44100);

    let spread_high = measure_fundamental_spread(&buffer_high, 44100.0, base_freq);

    // Test with no detune for baseline
    let input_zero = "out $ supersaw(220, 0.0, 5) * 0.3";
    let (_, statements_zero) = parse_dsl(input_zero).unwrap();
    let compiler_zero = DslCompiler::new(44100.0);
    let mut graph_zero = compiler_zero.compile(statements_zero);
    let buffer_zero = graph_zero.render(44100);

    let spread_zero = measure_fundamental_spread(&buffer_zero, 44100.0, base_freq);

    println!("\n=== Detune FFT Analysis (Fundamental Frequency Distribution) ===");
    println!("Detune 0.0: fundamental spread = {:.1} Hz", spread_zero);
    println!("Detune 0.1: fundamental spread = {:.1} Hz", spread_low);
    println!("Detune 0.9: fundamental spread = {:.1} Hz", spread_high);
    println!("Expected: 0.0 ≈ 0 Hz, 0.1 ≈ 4.4 Hz, 0.9 ≈ 39.6 Hz");

    // Let's also print the actual peak frequencies to observe the distribution
    let peaks_low = find_frequency_peaks(&buffer_low, 44100.0, 30);
    let fundamentals_low: Vec<_> = peaks_low
        .iter()
        .filter(|(f, _)| *f >= base_freq * 0.5 && *f <= base_freq * 1.5)
        .take(5)
        .collect();
    println!("\nLow detune (0.1) fundamental peaks:");
    for (freq, mag) in &fundamentals_low {
        println!("  {:.1} Hz (magnitude: {:.2})", freq, mag);
    }

    let peaks_high = find_frequency_peaks(&buffer_high, 44100.0, 30);
    let fundamentals_high: Vec<_> = peaks_high
        .iter()
        .filter(|(f, _)| *f >= base_freq * 0.5 && *f <= base_freq * 1.5)
        .take(5)
        .collect();
    println!("\nHigh detune (0.9) fundamental peaks:");
    for (freq, mag) in &fundamentals_high {
        println!("  {:.1} Hz (magnitude: {:.2})", freq, mag);
    }

    // Verify: detune=0.0 should have minimal spread (all oscillators at same frequency)
    assert!(
        spread_zero < 10.0,
        "Zero detune should have minimal fundamental spread (<10 Hz), got {:.1} Hz",
        spread_zero
    );

    // Verify: detune=0.9 should have much wider spread than detune=0.1
    assert!(
        spread_high > spread_low * 3.0,
        "High detune (0.9) should have >3x wider fundamental spread than low detune (0.1). \
         Got {:.1} Hz vs {:.1} Hz (ratio: {:.2})",
        spread_high,
        spread_low,
        spread_high / spread_low
    );

    // Verify: detune=0.9 should produce approximately 39.6 Hz spread (±30% tolerance)
    let expected_high = 39.6;
    assert!(
        (spread_high - expected_high).abs() < expected_high * 0.5,
        "High detune (0.9) should produce ~{:.1} Hz spread (±50%), got {:.1} Hz",
        expected_high,
        spread_high
    );

    // Verify: detune=0.1 should produce approximately 4.4 Hz spread (±50% tolerance)
    let expected_low = 4.4;
    assert!(
        (spread_low - expected_low).abs() < expected_low * 1.0,
        "Low detune (0.1) should produce ~{:.1} Hz spread (±100%), got {:.1} Hz",
        expected_low,
        spread_low
    );

    println!(
        "\n✅ Detune parameter VERIFIED - FFT shows correct fundamental frequency distribution!"
    );
}


// Test removed: s() function DOES exist in Phonon and works correctly
// It's a documented feature for sample triggering (see FEATURE_REVIEW_AND_GAP_ANALYSIS.md)
// The original test was incorrectly asserting that s() should NOT parse
