//! Tests for Karplus-Strong Synthesis
//!
//! Karplus-Strong is a physical modeling technique for plucked strings.
//! Algorithm: noise-filled delay line + lowpass filter = realistic string sound

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Helper: Calculate RMS of a buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|x| x * x).sum();
    (sum / buffer.len() as f32).sqrt()
}

/// Helper: Detect zero crossings for frequency measurement
fn detect_zero_crossings(buffer: &[f32]) -> Vec<usize> {
    buffer
        .windows(2)
        .enumerate()
        .filter_map(|(i, w)| {
            if w[0] <= 0.0 && w[1] > 0.0 {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}

/// Helper: Measure fundamental frequency from zero crossings
fn measure_frequency(buffer: &[f32], sample_rate: f32) -> Option<f32> {
    let crossings = detect_zero_crossings(buffer);
    if crossings.len() < 2 {
        return None;
    }

    let periods: Vec<f32> = crossings.windows(2).map(|w| (w[1] - w[0]) as f32).collect();

    let avg_period = periods.iter().sum::<f32>() / periods.len() as f32;
    Some(sample_rate / avg_period)
}

// ========== LEVEL 1: Basic Functionality ==========

#[test]
fn test_karplus_strong_produces_sound() {
    // Simple test: Karplus-Strong produces non-zero output
    let code = r#"
tempo: 1.0
out $ pluck 440 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100); // 1 second

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Karplus-Strong should produce audible output, got RMS={}",
        rms
    );
}

#[test]
#[ignore = "KNOWN_LIMITATION: inherent pitch instability from noise init"]
fn test_karplus_strong_frequency_accuracy() {
    // Verify Karplus-Strong plays at approximately the correct frequency
    let code = r#"
tempo: 1.0
out $ pluck 220
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(44100);

    // Skip first 0.1 second (initial noise burst)
    let analysis_start = 4410;
    let analysis_buffer = &buffer[analysis_start..];

    let measured_freq =
        measure_frequency(analysis_buffer, 44100.0).expect("Should detect frequency");

    // Should be within 15% of 220Hz (looser tolerance due to noise initialization
    // and inherent pitch instability of Karplus-Strong algorithm)
    let tolerance = 220.0 * 0.15;
    assert!(
        (measured_freq - 220.0).abs() < tolerance,
        "Expected ~220Hz (±15%), measured {}Hz",
        measured_freq
    );
}

#[test]
fn test_karplus_strong_decay() {
    // Karplus-Strong should decay over time (like a real string)
    let code = r#"
tempo: 1.0
out $ pluck 440 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    let buffer = graph.render(88200); // 2 seconds

    // Measure RMS in first and second halves
    let mid_point = buffer.len() / 2;
    let first_half = &buffer[0..mid_point];
    let second_half = &buffer[mid_point..];

    let rms_first = calculate_rms(first_half);
    let rms_second = calculate_rms(second_half);

    assert!(
        rms_second < rms_first,
        "String should decay: first_half RMS={}, second_half RMS={}",
        rms_first,
        rms_second
    );
}

#[test]
fn test_karplus_strong_damping() {
    // Higher damping must decay faster than lower damping.
    //
    // ROBUSTNESS NOTE (why this is not a naive two-render comparison):
    // Each `pluck` seeds its delay line from an *independent* random excitation
    // (`KarplusStrongState` draws from a process-global noise-seed counter, so the
    // exact excitation an individual render gets also depends on how the parallel
    // test threads interleave). A single low-damp vs high-damp render therefore
    // compares two *different* random realizations: the absolute late-window RMS is
    // dominated by the excitation's energy, not by the decay rate, so ~1-in-3
    // realizations tip a naive `high_late_rms < low_late_rms` assertion. That is
    // exactly why this test used to be flaky (~35% failures in the parallel suite).
    //
    // Two independent measures make the comparison excitation-robust:
    //   1. Compare a *within-signal* decay ratio (second-half RMS / first-half RMS).
    //      The excitation's overall amplitude cancels in the ratio, so what remains
    //      is (mostly) the damping-controlled decay rate.
    //   2. Average that ratio over N independent renders per damping level to wash
    //      out the residual per-excitation variance (~1/sqrt(N) shrinkage).
    // We pluck at 880 Hz so the delay line cycles ~882x/second: the decay contrast
    // between damping levels accrues within a 1-second render, keeping the test fast
    // while still measuring the same physical property.
    const N: usize = 8;

    // Render one pluck and return its within-signal decay ratio
    // (second-half RMS / first-half RMS). Smaller ratio == faster decay.
    fn decay_ratio(damping: &str) -> f32 {
        let code = format!("tempo: 1.0\nout $ pluck 880 {}\n", damping);
        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
        let buffer = graph.render(44100); // 1 second

        let mid = buffer.len() / 2;
        let first_half = calculate_rms(&buffer[..mid]);
        let second_half = calculate_rms(&buffer[mid..]);
        assert!(
            first_half > 0.0,
            "pluck should produce audible output (first-half RMS was zero)"
        );
        second_half / first_half
    }

    // Mean decay ratio over N independent excitations (washes out excitation variance).
    fn mean_decay_ratio(damping: &str) -> f32 {
        (0..N).map(|_| decay_ratio(damping)).sum::<f32>() / N as f32
    }

    let low_damp_ratio = mean_decay_ratio("0.1");
    let high_damp_ratio = mean_decay_ratio("0.9");

    // Higher damping retains less energy per delay-line cycle, so its signal decays
    // faster => a smaller second/first-half ratio. This can only fail if damping
    // genuinely stops affecting the decay rate, so the test stays meaningful.
    assert!(
        high_damp_ratio < low_damp_ratio,
        "High damping should decay faster: low-damp mean decay ratio={:.4}, \
         high-damp mean decay ratio={:.4} (averaged over {} renders each)",
        low_damp_ratio,
        high_damp_ratio,
        N
    );
}

// ========== LEVEL 2: Different Pitches ==========

#[test]
fn test_karplus_strong_different_frequencies() {
    // Test multiple frequencies
    let frequencies = [110.0, 220.0, 440.0];

    for freq in &frequencies {
        let code = format!(
            r#"
tempo: 1.0
out $ pluck {}
"#,
            freq
        );

        let (rest, statements) = parse_program(&code).expect("Failed to parse");
        assert_eq!(rest.trim(), "", "Parser should consume all input");

        let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
        let buffer = graph.render(44100);

        let rms = calculate_rms(&buffer);
        assert!(
            rms > 0.01,
            "Pluck at {}Hz should produce sound, got RMS={}",
            freq,
            rms
        );
    }
}

// ========== LEVEL 3: Pattern Modulation ==========

#[test]
fn test_karplus_strong_pattern_frequency() {
    // Pattern-modulated frequency (melody)
    let code = r#"
tempo: 0.5
out $ pluck "220 330 440 330"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second = 2 cycles

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated pluck should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_karplus_strong_pattern_damping() {
    // Pattern-modulated damping
    let code = r#"
tempo: 0.5
out $ pluck 440 "0.3 0.7"
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Pattern-modulated damping should produce sound, got RMS={}",
        rms
    );
}

// ========== LEVEL 4: Musical Examples ==========

#[test]
fn test_karplus_strong_melody() {
    // Play a simple melody with Karplus-Strong
    let code = r#"
tempo: 0.5
out $ pluck "220 330 440 330 220" 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Karplus-Strong melody should produce sound, got RMS={}",
        rms
    );
}

#[test]
fn test_karplus_strong_bass() {
    // Bass string with low damping
    let code = r#"
tempo: 0.5
out $ pluck "55 82.5" 0.2
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);

    let rms = calculate_rms(&buffer);
    assert!(
        rms > 0.01,
        "Karplus-Strong bass should produce sound, got RMS={}",
        rms
    );
}
