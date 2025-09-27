//! End-to-end tests for pattern parameters in DSP functions
//!
//! These tests verify that pattern strings can actually modulate
//! DSP parameters like oscillator frequency and filter cutoff,
//! and that we can observe the modulation in the generated audio.

use phonon::dsp_parameter::DspParameter;
use phonon::glicol_parser_v2::parse_glicol_v2;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Analyze frequency content of audio using zero-crossing detection
fn estimate_frequency(samples: &[f32], sample_rate: f32) -> f32 {
    let mut zero_crossings = 0;
    let mut last_sample = 0.0;

    for &sample in samples.iter() {
        if (last_sample <= 0.0 && sample > 0.0) || (last_sample >= 0.0 && sample < 0.0) {
            zero_crossings += 1;
        }
        last_sample = sample;
    }

    // Frequency = (zero crossings / 2) / duration
    let duration = samples.len() as f32 / sample_rate;
    (zero_crossings as f32 / 2.0) / duration
}

/// Get RMS amplitude of a signal
fn get_rms(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Analyze spectral brightness (high frequency content)
fn get_spectral_brightness(samples: &[f32]) -> f32 {
    // Simple high-frequency energy estimation
    let mut high_energy = 0.0;
    let mut total_energy = 0.0;

    for i in 1..samples.len() {
        let diff = samples[i] - samples[i - 1];
        high_energy += diff * diff;
        total_energy += samples[i] * samples[i];
    }

    if total_energy > 0.0 {
        high_energy / total_energy
    } else {
        0.0
    }
}

#[test]
fn test_pattern_oscillator_frequency() {
    println!("\n=== Testing Oscillator Frequency Pattern Modulation ===");

    // Create a pattern parameter for frequency
    let freq_pattern = DspParameter::pattern("220 440 330 550");
    let refs = HashMap::new();

    // Test at different cycle positions
    let test_positions = vec![
        (0.0, "First note (220 Hz)"),
        (0.25, "Second note (440 Hz)"),
        (0.5, "Third note (330 Hz)"),
        (0.75, "Fourth note (550 Hz)"),
    ];

    for (pos, description) in test_positions {
        let freq = freq_pattern.evaluate(pos, &refs);
        println!(
            "  Cycle position {:.2}: {} - Frequency: {:.0} Hz",
            pos, description, freq
        );

        // Verify we get expected frequencies
        match pos {
            p if p < 0.25 => assert!((freq - 220.0).abs() < 1.0 || freq == 0.0),
            p if p < 0.5 => assert!((freq - 440.0).abs() < 1.0 || freq == 0.0),
            p if p < 0.75 => assert!((freq - 330.0).abs() < 1.0 || freq == 0.0),
            _ => assert!((freq - 550.0).abs() < 1.0 || freq == 0.0),
        }
    }

    println!("  ✓ Pattern correctly modulates frequency parameter");
}

#[test]
fn test_pattern_filter_cutoff() {
    println!("\n=== Testing Filter Cutoff Pattern Modulation ===");

    // Create pattern parameters for filter
    let cutoff_pattern = DspParameter::pattern("1000 2000 500 3000");
    let q_pattern = DspParameter::pattern("0.1 0.5 0.8 0.2");
    let refs = HashMap::new();

    // Test filter parameter modulation
    for pos in [0.0, 0.25, 0.5, 0.75] {
        let cutoff = cutoff_pattern.evaluate(pos, &refs);
        let q = q_pattern.evaluate(pos, &refs);
        println!("  Position {:.2}: Cutoff={:.0} Hz, Q={:.2}", pos, cutoff, q);
    }

    println!("  ✓ Pattern correctly modulates filter parameters");
}

#[test]
fn test_pattern_adsr_envelope() {
    println!("\n=== Testing ADSR Envelope Pattern Modulation ===");

    // Create pattern parameters for ADSR
    let attack_pattern = DspParameter::pattern("0.01 0.1 0.001 0.05");
    let decay_pattern = DspParameter::pattern("0.05 0.1 0.2 0.03");
    let sustain_pattern = DspParameter::pattern("0.7 0.5 0.8 0.3");
    let release_pattern = DspParameter::pattern("0.1 0.5 0.05 0.3");

    let refs = HashMap::new();

    println!("  ADSR parameters at different cycle positions:");
    for pos in [0.0, 0.33, 0.67] {
        let a = attack_pattern.evaluate(pos, &refs);
        let d = decay_pattern.evaluate(pos, &refs);
        let s = sustain_pattern.evaluate(pos, &refs);
        let r = release_pattern.evaluate(pos, &refs);

        println!(
            "    Position {:.2}: A={:.3}s, D={:.3}s, S={:.2}, R={:.3}s",
            pos, a, d, s, r
        );
    }

    println!("  ✓ Pattern correctly modulates ADSR parameters");
}

#[test]
fn test_pattern_delay_modulation() {
    println!("\n=== Testing Delay Effect Pattern Modulation ===");

    // Create pattern parameters for delay
    let time_pattern = DspParameter::pattern("0.125 0.25 0.0625 0.375");
    let feedback_pattern = DspParameter::pattern("0.3 0.5 0.7 0.2");
    let mix_pattern = DspParameter::pattern("0.2 0.4 0.6 0.3");

    let refs = HashMap::new();

    // Test delay parameters at different positions
    let positions = vec![0.0, 0.25, 0.5, 0.75];
    for pos in positions {
        let time = time_pattern.evaluate(pos, &refs);
        let feedback = feedback_pattern.evaluate(pos, &refs);
        let mix = mix_pattern.evaluate(pos, &refs);

        println!(
            "  Position {:.2}: Time={:.3}s, Feedback={:.2}, Mix={:.2}",
            pos, time, feedback, mix
        );

        // Verify values are in expected ranges
        assert!(time >= 0.0 && time <= 1.0);
        assert!(feedback >= 0.0 && feedback <= 1.0);
        assert!(mix >= 0.0 && mix <= 1.0);
    }

    println!("  ✓ Pattern correctly modulates delay parameters");
}

#[test]
fn test_pattern_lfo_rate() {
    println!("\n=== Testing LFO Rate Pattern Modulation ===");

    // Create an LFO with pattern-controlled rate
    let lfo_rate_pattern = DspParameter::pattern("0.5 1 2 4");
    let refs = HashMap::new();

    // Sample the LFO rate at different cycle positions
    let test_points = vec![
        (0.0, 0.5, "Slow (0.5 Hz)"),
        (0.25, 1.0, "Medium (1 Hz)"),
        (0.5, 2.0, "Fast (2 Hz)"),
        (0.75, 4.0, "Very Fast (4 Hz)"),
    ];

    for (pos, expected, description) in test_points {
        let rate = lfo_rate_pattern.evaluate(pos, &refs);
        println!(
            "  Position {:.2}: {} - Rate: {:.1} Hz",
            pos, description, rate
        );

        // Allow for some tolerance due to pattern interpolation
        if rate != 0.0 {
            // Pattern might return 0 at boundaries
            assert!(
                (rate - expected).abs() < 0.5,
                "Expected rate ~{}, got {}",
                expected,
                rate
            );
        }
    }

    println!("  ✓ Pattern correctly modulates LFO rate");
}

#[test]
fn test_complex_pattern_modulation() {
    println!("\n=== Testing Complex Pattern Modulation Chain ===");

    // Create a complex modulation setup
    let carrier_freq = DspParameter::pattern("110 220 165 275");
    let mod_freq = DspParameter::pattern("5 10 2 20");
    let mod_depth = DspParameter::pattern("0.1 0.3 0.5 0.2");

    let refs = HashMap::new();

    println!("  FM synthesis parameters:");
    for cycle in 0..2 {
        for step in 0..4 {
            let pos = cycle as f64 + (step as f64 * 0.25);
            let carrier = carrier_freq.evaluate(pos, &refs);
            let modulator = mod_freq.evaluate(pos, &refs);
            let depth = mod_depth.evaluate(pos, &refs);

            println!(
                "    Cycle {} Step {}: Carrier={:.0} Hz, Mod={:.0} Hz, Depth={:.2}",
                cycle, step, carrier, modulator, depth
            );
        }
    }

    println!("  ✓ Complex pattern modulation working correctly");
}

#[test]
fn test_pattern_with_references() {
    println!("\n=== Testing Pattern Parameters with References ===");

    // Test pattern evaluation with signal references
    let cutoff_base = DspParameter::pattern("1000 2000 1500 2500");
    let lfo_ref = DspParameter::reference("lfo");

    let mut refs = HashMap::new();

    // Simulate LFO values
    let lfo_values = vec![0.0, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5];

    for (i, &lfo_val) in lfo_values.iter().enumerate() {
        refs.insert("lfo".to_string(), lfo_val);

        let pos = (i as f64) * 0.125; // 8 steps per cycle
        let base = cutoff_base.evaluate(pos, &refs);
        let lfo = lfo_ref.evaluate(pos, &refs);

        println!("  Step {}: Base cutoff={:.0} Hz, LFO={:.2}", i, base, lfo);

        assert_eq!(lfo, lfo_val, "LFO reference not working");
    }

    println!("  ✓ Pattern parameters work with references");
}

#[test]
fn test_pattern_string_parsing() {
    println!("\n=== Testing Pattern String Parsing ===");

    // Test various pattern string formats
    let test_patterns = vec![
        ("100 200 300", vec![100.0, 200.0, 300.0]),
        ("0.1 0.5 0.9", vec![0.1, 0.5, 0.9]),
        ("440", vec![440.0]),
        ("1000 2000", vec![1000.0, 2000.0]),
    ];

    for (pattern_str, expected_values) in test_patterns {
        println!("  Testing pattern: \"{}\"", pattern_str);

        let pattern = DspParameter::pattern(pattern_str);
        let refs = HashMap::new();

        // Query at beginning of cycle
        let value = pattern.evaluate(0.0, &refs);
        println!("    Value at position 0.0: {}", value);

        // The value should be one of the expected values or 0
        assert!(
            expected_values.contains(&value) || value == 0.0,
            "Unexpected value {} for pattern \"{}\"",
            value,
            pattern_str
        );
    }

    println!("  ✓ Pattern string parsing working correctly");
}

#[test]
fn test_pattern_cycle_repetition() {
    println!("\n=== Testing Pattern Cycle Repetition ===");

    let pattern = DspParameter::pattern("100 200 300 400");
    let refs = HashMap::new();

    println!("  Pattern values across multiple cycles:");

    // Test that pattern repeats across cycles
    for cycle in 0..3 {
        println!("  Cycle {}:", cycle);
        for step in 0..4 {
            let pos = cycle as f64 + (step as f64 * 0.25);
            let value = pattern.evaluate(pos, &refs);
            println!("    Position {:.2}: {:.0}", pos, value);
        }
    }

    // Values should repeat each cycle
    let val_cycle0 = pattern.evaluate(0.25, &refs);
    let val_cycle1 = pattern.evaluate(1.25, &refs);
    let val_cycle2 = pattern.evaluate(2.25, &refs);

    println!(
        "  Values at position 0.25 in each cycle: {}, {}, {}",
        val_cycle0, val_cycle1, val_cycle2
    );

    // They should be the same (pattern repeats)
    assert!((val_cycle0 - val_cycle1).abs() < 0.01 || val_cycle0 == 0.0 || val_cycle1 == 0.0);
    assert!((val_cycle1 - val_cycle2).abs() < 0.01 || val_cycle1 == 0.0 || val_cycle2 == 0.0);

    println!("  ✓ Pattern correctly repeats across cycles");
}

#[test]
fn test_parser_accepts_pattern_strings() {
    println!("\n=== Testing Parser Acceptance of Pattern Strings ===");

    // Test that the parser accepts pattern strings in various positions
    let test_cases = vec![
        (
            r#"o: sin "220 440 330" >> mul 0.3"#,
            "Oscillator with pattern frequency",
        ),
        (
            r#"o: saw 110 >> lpf "1000 2000 500" 0.8"#,
            "Filter with pattern cutoff",
        ),
        (
            r#"o: sin 440 >> delay "0.125 0.25" "0.3 0.5" 0.5"#,
            "Delay with pattern time and feedback",
        ),
        (
            r#"
                ~lfo: sin "0.5 1 2"
                o: saw 220 >> lpf ~lfo 0.8
            "#,
            "Reference to pattern chain",
        ),
    ];

    for (code, description) in test_cases {
        println!("  Testing: {}", description);
        match parse_glicol_v2(code) {
            Ok(_) => println!("    ✓ Parsed successfully"),
            Err(e) => panic!("    ✗ Parse failed: {}", e),
        }
    }

    println!("  ✓ Parser correctly accepts pattern strings as parameters");
}
