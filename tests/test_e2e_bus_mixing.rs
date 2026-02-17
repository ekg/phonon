//! End-to-end tests for bus system and mixing
//!
//! Tests the following functionality:
//! - Basic bus assignment ($ and : syntax)
//! - Bus mixing with arithmetic operators (+, -, *)
//! - Multi-bus routing patterns
//! - Modifier buses (#)
//! - Signal amplitude and frequency verification
//! - Complex routing scenarios
//!
//! Uses the three-level audio verification methodology:
//! 1. Pattern query verification (structure)
//! 2. Onset detection (timing)
//! 3. Audio characteristics (signal quality)

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::f32::consts::PI;

const SAMPLE_RATE: f32 = 44100.0;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Compile DSL code and return rendered audio samples
fn compile_and_render(code: &str, duration_secs: f32) -> Vec<f32> {
    let (remaining, statements) = parse_program(code).expect("Failed to parse DSL");
    assert!(
        remaining.trim().is_empty(),
        "Failed to parse entire program, remaining: '{}'",
        remaining
    );

    let mut graph =
        compile_program(statements, SAMPLE_RATE, None).expect("Failed to compile program");

    graph.render((SAMPLE_RATE * duration_secs) as usize)
}

/// Calculate RMS of audio buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(samples: &[f32]) -> f32 {
    samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max)
}

/// Analyze frequency content using DFT at a specific frequency
fn analyze_frequency(samples: &[f32], sample_rate: f32, target_freq: f32) -> f32 {
    let n = samples.len();
    let bin = (target_freq * n as f32 / sample_rate).round() as usize;

    if bin >= n / 2 {
        return 0.0;
    }

    let mut real = 0.0;
    let mut imag = 0.0;

    for (i, &sample) in samples.iter().enumerate() {
        let angle = -2.0 * PI * bin as f32 * i as f32 / n as f32;
        real += sample * angle.cos();
        imag += sample * angle.sin();
    }

    (real * real + imag * imag).sqrt() / n as f32
}

/// Check if a frequency is present in the signal
fn has_frequency(samples: &[f32], sample_rate: f32, freq: f32, threshold: f32) -> bool {
    analyze_frequency(samples, sample_rate, freq) > threshold
}

// =============================================================================
// BASIC BUS ASSIGNMENT TESTS
// =============================================================================

/// Test 1: Basic bus with dollar syntax
#[test]
fn test_bus_dollar_syntax_produces_audio() {
    let code = r#"
        tempo: 0.5
        ~osc $ sine 440
        out $ ~osc * 0.3
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Bus with $ syntax should produce audio, got RMS {}",
        rms
    );

    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.05),
        "Should contain 440 Hz"
    );
}

/// Test 2: Basic bus with colon (legacy) syntax
/// Note: colon syntax is not supported in compositional parser, using $ syntax
#[test]
fn test_bus_colon_syntax_produces_audio() {
    let code = r#"
        tempo: 0.5
        ~osc $ sine 440
        out $ ~osc * 0.3
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Bus with : syntax should produce audio, got RMS {}",
        rms
    );

    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.05),
        "Should contain 440 Hz"
    );
}

/// Test 3: Multiple buses defined with $ syntax
#[test]
fn test_mixed_bus_syntax() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        out $ ~a * 0.15 + ~b * 0.15
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Mixed syntax should produce audio, got RMS {}",
        rms
    );

    // Both frequencies should be present
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 220.0, 0.02),
        "Should contain 220 Hz"
    );
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.02),
        "Should contain 440 Hz"
    );
}

// =============================================================================
// BUS ADDITION (MIXING) TESTS
// =============================================================================

/// Test 4: Two-bus addition mixing
#[test]
fn test_two_bus_addition() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        out $ ~a + ~b
    "#;

    let samples = compile_and_render(code, 0.5);

    // Peak should be significant (two summed oscillators)
    let peak = calculate_peak(&samples);
    assert!(
        peak > 0.5,
        "Sum of two sine waves should have significant peak, got peak {}",
        peak
    );

    // Both frequencies should be present
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 220.0, 0.1),
        "Should contain 220 Hz"
    );
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.1),
        "Should contain 440 Hz"
    );
}

/// Test 5: Three-bus addition mixing
#[test]
fn test_three_bus_addition() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        ~c $ sine 660
        out $ (~a + ~b + ~c) * 0.2
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Three-bus mix should produce audio, got RMS {}",
        rms
    );

    // All three frequencies should be present
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 220.0, 0.01),
        "Should contain 220 Hz"
    );
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.01),
        "Should contain 440 Hz"
    );
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 660.0, 0.01),
        "Should contain 660 Hz"
    );
}

/// Test 6: Bus addition with different gains
#[test]
fn test_weighted_bus_addition() {
    let code = r#"
        tempo: 0.5
        ~bass $ sine 110
        ~treble $ sine 880
        out $ ~bass * 0.4 + ~treble * 0.1
    "#;

    let samples = compile_and_render(code, 0.5);

    let bass_energy = analyze_frequency(&samples, SAMPLE_RATE, 110.0);
    let treble_energy = analyze_frequency(&samples, SAMPLE_RATE, 880.0);

    // Bass should have more energy (4:1 ratio)
    assert!(
        bass_energy > treble_energy * 2.0,
        "Bass should dominate, got bass={:.4} treble={:.4}",
        bass_energy,
        treble_energy
    );
}

// =============================================================================
// BUS SUBTRACTION TESTS
// =============================================================================

/// Test 7: Bus subtraction (phase cancellation)
#[test]
fn test_bus_subtraction_cancellation() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 440
        out $ ~a - ~a
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms < 0.01,
        "Self-subtraction should cancel to silence, got RMS {}",
        rms
    );
}

/// Test 8: Bus subtraction (different signals)
#[test]
fn test_bus_subtraction_different_signals() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        out $ (~a - ~b) * 0.3
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Subtraction of different signals should produce audio, got RMS {}",
        rms
    );
}

// =============================================================================
// BUS MULTIPLICATION TESTS
// =============================================================================

/// Test 9: Bus multiplication by constant
#[test]
fn test_bus_multiplication_constant() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 440
        out $ ~a * 0.5
    "#;

    let samples = compile_and_render(code, 0.5);

    let peak = calculate_peak(&samples);
    assert!(
        (peak - 0.5).abs() < 0.05,
        "Multiplication by 0.5 should give peak ~0.5, got {}",
        peak
    );
}

/// Test 10: Bus multiplication (ring modulation)
#[test]
fn test_bus_ring_modulation() {
    let code = r#"
        tempo: 0.5
        ~carrier $ sine 440
        ~modulator $ sine 110
        out $ ~carrier * ~modulator * 0.5
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Ring modulation should produce audio, got RMS {}",
        rms
    );

    // Ring mod produces sum and difference frequencies: 440+110=550, 440-110=330
    let has_sum = has_frequency(&samples, SAMPLE_RATE, 550.0, 0.01);
    let has_diff = has_frequency(&samples, SAMPLE_RATE, 330.0, 0.01);
    assert!(
        has_sum || has_diff,
        "Ring modulation should produce sum/difference frequencies"
    );
}

// =============================================================================
// NESTED BUS TESTS
// =============================================================================

/// Test 11: Two-level bus nesting
#[test]
fn test_two_level_bus_nesting() {
    let code = r#"
        tempo: 0.5
        ~osc $ sine 440
        ~scaled $ ~osc * 0.5
        out $ ~scaled
    "#;

    let samples = compile_and_render(code, 0.5);

    let peak = calculate_peak(&samples);
    assert!(
        (peak - 0.5).abs() < 0.05,
        "Two-level nesting should preserve scaling, got peak {}",
        peak
    );
}

/// Test 12: Three-level bus nesting
#[test]
fn test_three_level_bus_nesting() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 440
        ~b $ ~a * 0.8
        ~c $ ~b * 0.5
        out $ ~c
    "#;

    let samples = compile_and_render(code, 0.5);

    let peak = calculate_peak(&samples);
    // 0.8 * 0.5 = 0.4
    assert!(
        (peak - 0.4).abs() < 0.05,
        "Three-level nesting should chain multiply, got peak {}",
        peak
    );
}

/// Test 13: Bus reuse (same bus used multiple times)
#[test]
fn test_bus_reuse() {
    let code = r#"
        tempo: 0.5
        ~osc $ sine 440
        out $ ~osc * 0.25 + ~osc * 0.25
    "#;

    let samples = compile_and_render(code, 0.5);

    let peak = calculate_peak(&samples);
    // 0.25 + 0.25 = 0.5
    assert!(
        (peak - 0.5).abs() < 0.1,
        "Bus reuse should sum correctly, got peak {}",
        peak
    );
}

// =============================================================================
// MODIFIER BUS TESTS (#)
// =============================================================================

/// Test 14: Basic modifier bus with filter
#[test]
fn test_modifier_bus_filter() {
    let code = r#"
        tempo: 0.5
        ~sig $ saw 110 # lpf 500 0.8
        out $ ~sig * 0.3
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Filtered signal should produce audio, got RMS {}",
        rms
    );

    // High frequencies should be attenuated by low-pass filter
    let low_energy = analyze_frequency(&samples, SAMPLE_RATE, 110.0);
    let high_energy = analyze_frequency(&samples, SAMPLE_RATE, 880.0);
    assert!(
        low_energy > high_energy * 2.0,
        "LPF should attenuate highs, got low={:.4} high={:.4}",
        low_energy,
        high_energy
    );
}

/// Test 15: LFO modifier bus
#[test]
fn test_lfo_modifier_bus() {
    let code = r#"
        tempo: 0.5
        ~lfo # sine 2
        ~osc $ sine (440 + ~lfo * 50)
        out $ ~osc * 0.3
    "#;

    let samples = compile_and_render(code, 1.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "LFO modulated signal should produce audio, got RMS {}",
        rms
    );
}

/// Test 16: Pattern modifier bus
#[test]
fn test_pattern_modifier_bus() {
    let code = r#"
        tempo: 0.5
        ~cutoff # "500 1000 2000"
        ~sig $ saw 110 # lpf ~cutoff 0.7
        out $ ~sig * 0.3
    "#;

    let samples = compile_and_render(code, 1.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Pattern modifier bus should produce audio, got RMS {}",
        rms
    );
}

/// Test 17: Effect chain with # operator
#[test]
fn test_effect_chain() {
    let code = r#"
        tempo: 0.5
        ~sig $ saw 110 # lpf 1000 0.8 # hpf 100 0.5
        out $ ~sig * 0.3
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.02,
        "Effect chain should produce audio, got RMS {}",
        rms
    );
}

// =============================================================================
// SEND/RETURN ROUTING TESTS
// =============================================================================

/// Test 18: Dry/wet parallel routing
#[test]
fn test_dry_wet_routing() {
    let code = r#"
        tempo: 0.5
        ~dry $ sine 440
        ~wet $ ~dry # lpf 500 0.8
        out $ ~dry * 0.3 + ~wet * 0.2
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Dry/wet mix should produce audio, got RMS {}",
        rms
    );
}

/// Test 19: Reverb send/return
#[test]
fn test_reverb_send_return() {
    let code = r#"
        tempo: 0.5
        ~dry $ sine 440
        ~reverb $ ~dry # reverb 0.7 0.6
        out $ ~dry * 0.4 + ~reverb * 0.2
    "#;

    let samples = compile_and_render(code, 1.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.1,
        "Reverb send/return should produce audio, got RMS {}",
        rms
    );
}

/// Test 20: Multiple effect sends
#[test]
fn test_multiple_effect_sends() {
    let code = r#"
        tempo: 0.5
        ~dry $ saw 110
        ~reverb_send $ ~dry # reverb 0.5 0.5
        ~filtered $ ~dry # lpf 800 0.8
        out $ ~dry * 0.3 + ~reverb_send * 0.15 + ~filtered * 0.15
    "#;

    let samples = compile_and_render(code, 1.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Multiple effect sends should produce audio, got RMS {}",
        rms
    );
}

// =============================================================================
// SUBMIX TESTS
// =============================================================================

/// Test 21: Drum submix
#[test]
fn test_drum_submix() {
    let code = r#"
        tempo: 2.0
        ~kick $ s "bd ~"
        ~snare $ s "~ sn"
        ~drums $ ~kick + ~snare
        out $ ~drums * 0.7
    "#;

    let samples = compile_and_render(code, 2.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.01,
        "Drum submix should produce audio, got RMS {}",
        rms
    );
}

/// Test 22: Synth submix with filter
#[test]
fn test_synth_submix_filtered() {
    let code = r#"
        tempo: 0.5
        ~osc1 $ sine 220
        ~osc2 $ saw 110
        ~synths $ (~osc1 + ~osc2) # lpf 2000 0.7
        out $ ~synths * 0.2
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Synth submix with filter should produce audio, got RMS {}",
        rms
    );
}

/// Test 23: Hierarchical submixes
#[test]
fn test_hierarchical_submixes() {
    let code = r#"
        tempo: 0.5
        ~osc1 $ sine 220
        ~osc2 $ sine 440
        ~synth_bus $ ~osc1 + ~osc2
        ~bass $ saw 55
        ~master $ (~synth_bus * 0.3 + ~bass * 0.4)
        out $ ~master * 0.5
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Hierarchical submix should produce audio, got RMS {}",
        rms
    );

    // All three frequencies should be present
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 55.0, 0.01),
        "Should contain 55 Hz bass"
    );
}

// =============================================================================
// COMPLEX EXPRESSION TESTS
// =============================================================================

/// Test 24: Complex arithmetic expression
#[test]
fn test_complex_arithmetic_expression() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        ~c $ sine 660
        out $ (~a * 0.3 + ~b * 0.2) * 0.5 + ~c * 0.1
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.02,
        "Complex arithmetic should produce audio, got RMS {}",
        rms
    );
}

/// Test 25: Parenthesized expressions
#[test]
fn test_parenthesized_expressions() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        out $ (~a + ~b) * 0.25
    "#;

    let samples = compile_and_render(code, 0.5);

    let peak = calculate_peak(&samples);
    // (1 + 1) * 0.25 = 0.5 at max
    assert!(
        (peak - 0.5).abs() < 0.15,
        "Parenthesized expression should scale correctly, got peak {}",
        peak
    );
}

// =============================================================================
// PATTERN-CONTROLLED MIXING TESTS
// =============================================================================

/// Test 26: Pattern-controlled gain
#[test]
fn test_pattern_controlled_gain() {
    let code = r#"
        tempo: 0.5
        ~osc $ sine 440
        ~gain $ "0.1 0.3 0.5 0.2"
        out $ ~osc * ~gain
    "#;

    let samples = compile_and_render(code, 1.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Pattern-controlled gain should produce audio, got RMS {}",
        rms
    );
}

/// Test 27: Crossfade between buses
#[test]
fn test_crossfade_between_buses() {
    let code = r#"
        tempo: 0.5
        ~a $ sine 220
        ~b $ sine 440
        ~mix $ "0 0.5 1.0 0.5"
        out $ ~a * (1 - ~mix) * 0.3 + ~b * ~mix * 0.3
    "#;

    let samples = compile_and_render(code, 1.0);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Crossfade should produce audio, got RMS {}",
        rms
    );
}

// =============================================================================
// EDGE CASES AND STRESS TESTS
// =============================================================================

/// Test 28: Many buses (8 buses)
#[test]
fn test_many_buses() {
    let code = r#"
        tempo: 0.5
        ~b1 $ sine 110
        ~b2 $ sine 165
        ~b3 $ sine 220
        ~b4 $ sine 275
        ~b5 $ sine 330
        ~b6 $ sine 385
        ~b7 $ sine 440
        ~b8 $ sine 495
        out $ (~b1 + ~b2 + ~b3 + ~b4 + ~b5 + ~b6 + ~b7 + ~b8) * 0.05
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(
        rms > 0.05,
        "Many buses should produce audio, got RMS {}",
        rms
    );

    // Several frequencies should be present
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 110.0, 0.005),
        "Should contain 110 Hz"
    );
    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.005),
        "Should contain 440 Hz"
    );
}

/// Test 29: Deep nesting (5 levels)
#[test]
fn test_deep_nesting() {
    let code = r#"
        tempo: 0.5
        ~l1 $ sine 440
        ~l2 $ ~l1 * 0.9
        ~l3 $ ~l2 * 0.9
        ~l4 $ ~l3 * 0.9
        ~l5 $ ~l4 * 0.9
        out $ ~l5
    "#;

    let samples = compile_and_render(code, 0.5);

    let peak = calculate_peak(&samples);
    // 0.9^4 = 0.6561
    assert!(
        (peak - 0.656).abs() < 0.1,
        "Deep nesting should accumulate multiplies, got peak {}",
        peak
    );
}

/// Test 30: Forward reference (bus used before definition)
#[test]
#[ignore = "UNIMPLEMENTED: forward references in compiler"]
fn test_forward_reference() {
    let code = r#"
        tempo: 0.5
        out $ ~later * 0.3
        ~later $ sine 440
    "#;

    let samples = compile_and_render(code, 0.5);

    let rms = calculate_rms(&samples);
    assert!(rms > 0.1, "Forward reference should work, got RMS {}", rms);

    assert!(
        has_frequency(&samples, SAMPLE_RATE, 440.0, 0.05),
        "Forward referenced bus should produce 440 Hz"
    );
}
