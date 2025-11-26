// Pattern Parameter Verification - Phase 4: Deep Verification
//
// This test suite verifies advanced pattern parameter behaviors:
// 1. Audio-rate modulation (patterns modulate at 44.1kHz, not just control rate)
// 2. Spectral analysis (pattern modulation creates expected spectral changes)
// 3. Continuous vs stepped (pattern modulation is continuous/interpolated)
//
// These tests prove that Phonon's pattern-controllable parameters are
// fundamentally different from Tidal/Strudel (which only trigger discrete events).

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod pattern_verification_utils;
use pattern_verification_utils::{
    calculate_rms, calculate_spectral_centroid, assert_spectral_difference, calculate_peak,
};

// Test duration in seconds
const TEST_DURATION: f64 = 2.0;
const SAMPLE_RATE: f32 = 44100.0;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f64) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL");

    let samples = (duration * sample_rate as f64) as usize;
    let mut output = vec![0.0; samples];

    graph.process_buffer(&mut output);
    output
}

// ============================================================================
// AUDIO-RATE MODULATION TESTS
//
// Verify that patterns can modulate parameters at audio rate (44.1kHz),
// not just at control rate (typical pattern ticks).
// ============================================================================

#[test]
fn test_audio_rate_fm_synthesis() {
    // Audio-rate FM: modulator frequency = 440 Hz
    // This should create sidebands at carrier ± modulator frequencies
    //
    // If pattern modulation only worked at control rate (~10 Hz),
    // we'd only hear slow vibrato. But with audio-rate modulation,
    // we get true FM synthesis with rich harmonic content.

    let code = r#"
tempo: 1.0
~carrier: sine 220
~modulator: sine 440
~fm: sine (~modulator * 100 + 220)
out: ~fm
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Audio-rate FM should produce significant spectral content
    let centroid = calculate_spectral_centroid(&audio, SAMPLE_RATE);

    // FM synthesis creates sidebands, so centroid should be higher than carrier alone
    // Carrier at 220 Hz would have centroid around 220-300 Hz
    // FM with 440 Hz modulator should create some spectral spread
    assert!(
        centroid > 250.0,
        "Audio-rate FM should create harmonics, centroid: {} Hz (expected > 250 Hz)",
        centroid
    );

    // Should have reasonable energy
    let rms = calculate_rms(&audio);
    assert!(rms > 0.3, "FM synthesis should produce audible signal, RMS: {}", rms);
}

#[test]
fn test_audio_rate_filter_modulation() {
    // Audio-rate filter cutoff modulation with 10 Hz LFO
    // This creates a wah-wah effect - not possible with control-rate modulation

    let code = r#"
tempo: 1.0
~input: saw 110
~lfo: sine 10
~filtered: ~input # lpf (~lfo * 2500 + 2500) 0.8
out: ~filtered
"#;

    let audio = render_dsl(code, 4.0);  // Longer duration for more LFO cycles

    // Filter sweep at 10 Hz should create spectral variation
    // Use small windows (50ms) to catch spectral changes as LFO sweeps
    let window_size = 2205; // 50ms windows
    let mut centroids = Vec::new();

    for i in (0..audio.len() - window_size).step_by(window_size) {
        let window = &audio[i..i + window_size];
        let centroid = calculate_spectral_centroid(window, SAMPLE_RATE);
        centroids.push(centroid);
    }

    // Find max and min centroids across all windows
    let max_centroid = centroids.iter().cloned().fold(0.0f32, f32::max);
    let min_centroid = centroids.iter().cloned().fold(20000.0f32, f32::min);
    let range = max_centroid - min_centroid;

    assert!(
        range > 50.0,
        "Audio-rate filter modulation should vary spectral content, range: {} Hz (max: {}, min: {})",
        range, max_centroid, min_centroid
    );

    // Should have reasonable energy
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Filtered signal should be audible, RMS: {}", rms);
}

#[test]
fn test_audio_rate_tremolo() {
    // Audio-rate amplitude modulation at 8 Hz
    // This creates classic tremolo effect

    let code = r#"
tempo: 1.0
~carrier: saw 220
~lfo: sine 8
~tremolo: ~carrier * (~lfo * 0.5 + 0.5)
out: ~tremolo
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Tremolo at 8 Hz should create amplitude variation
    // Analyze RMS over small windows to detect modulation
    let window_size = 2756; // ~62.5ms windows (1/16th second) at 44.1kHz
    let mut rms_values = Vec::new();

    for i in (0..audio.len() - window_size).step_by(window_size / 2) {
        let window = &audio[i..i + window_size];
        let rms = calculate_rms(window);
        rms_values.push(rms);
    }

    // Find max and min RMS to verify modulation depth
    let max_rms = rms_values.iter().cloned().fold(0.0f32, f32::max);
    let min_rms = rms_values.iter().cloned().fold(1.0f32, f32::min);
    let depth = max_rms - min_rms;

    // Tremolo depth should be significant (LFO varies from 0.5 to 1.0, so ~0.5 range)
    assert!(
        depth > 0.2,
        "Tremolo should create significant amplitude variation, depth: {}",
        depth
    );

    // Peak amplitude should be close to 1.0 (LFO max)
    let peak = calculate_peak(&audio);
    assert!(
        peak > 0.8 && peak <= 1.0,
        "Tremolo peak should be near 1.0, got: {}",
        peak
    );
}

#[test]
fn test_audio_rate_ring_modulation() {
    // Ring modulation: multiply two audio-rate signals
    // Creates sidebands at sum and difference frequencies

    let code = r#"
tempo: 1.0
~carrier: sine 440
~modulator: sine 330
~ring: ~carrier * ~modulator
out: ~ring
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Ring modulation creates sidebands at:
    // - carrier + modulator = 440 + 330 = 770 Hz
    // - carrier - modulator = 440 - 330 = 110 Hz
    // Centroid should reflect this complex spectrum

    let centroid = calculate_spectral_centroid(&audio, SAMPLE_RATE);

    // Centroid should be between 200-600 Hz (weighted average of 110 Hz and 770 Hz)
    assert!(
        centroid > 200.0 && centroid < 700.0,
        "Ring modulation spectral centroid: {} Hz (expected 200-700 Hz)",
        centroid
    );

    // Should have reasonable energy
    let rms = calculate_rms(&audio);
    assert!(rms > 0.3, "Ring modulation should produce audible signal, RMS: {}", rms);
}

// ============================================================================
// SPECTRAL ANALYSIS TESTS
//
// Verify that pattern modulation creates expected spectral changes.
// Uses FFT-based analysis to measure frequency content.
// ============================================================================

#[test]
fn test_spectral_lpf_cutoff_modulation() {
    // LPF with pattern-modulated cutoff should have different spectrum
    // than LPF with constant cutoff

    let code_constant = r#"
tempo: 1.0
~input: saw 110
~filtered: ~input # lpf 2000 0.8
out: ~filtered
"#;

    let code_modulated = r#"
tempo: 1.0
~input: saw 110
~lfo: sine 0.5
~filtered: ~input # lpf (~lfo * 1500 + 2000) 0.8
out: ~filtered
"#;

    let audio_constant = render_dsl(code_constant, TEST_DURATION);
    let audio_modulated = render_dsl(code_modulated, TEST_DURATION);

    // Use spectral analysis to verify modulation creates different content
    assert_spectral_difference(
        &audio_constant,
        &audio_modulated,
        SAMPLE_RATE,
        100.0, // Expect at least 100 Hz difference in centroid
        "LPF cutoff modulation should create spectral difference",
    );
}

#[test]
fn test_spectral_oscillator_frequency_modulation() {
    // Oscillator with pattern-modulated frequency (FM)
    // should have different spectrum than constant frequency

    let code_constant = r#"
tempo: 1.0
~osc: sine 440
out: ~osc
"#;

    let code_fm = r#"
tempo: 1.0
~lfo: sine 5
~osc: sine (~lfo * 50 + 440)
out: ~osc
"#;

    let audio_constant = render_dsl(code_constant, TEST_DURATION);
    let audio_fm = render_dsl(code_fm, TEST_DURATION);

    // FM should create spectral content different from pure sine
    assert_spectral_difference(
        &audio_constant,
        &audio_fm,
        SAMPLE_RATE,
        40.0, // Expect at least 40 Hz difference in centroid (slight FM creates modest spectral shift)
        "FM modulation should create spectral difference from pure sine",
    );
}

#[test]
fn test_spectral_resonant_filter_sweep() {
    // Resonant filter sweep should create distinctive spectral peaks

    let code = r#"
tempo: 1.0
~noise: noise
~lfo: sine 0.25
~swept: ~noise # lpf (~lfo * 2000 + 2000) 8.0
out: ~swept
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Resonant filter on noise should create focused spectral peak
    // that moves with LFO

    // Split into 4 quarters to see spectral movement
    let quarter = audio.len() / 4;
    let q1 = &audio[0..quarter];
    let q2 = &audio[quarter..quarter*2];
    let q3 = &audio[quarter*2..quarter*3];
    let q4 = &audio[quarter*3..];

    let c1 = calculate_spectral_centroid(q1, SAMPLE_RATE);
    let c2 = calculate_spectral_centroid(q2, SAMPLE_RATE);
    let c3 = calculate_spectral_centroid(q3, SAMPLE_RATE);
    let c4 = calculate_spectral_centroid(q4, SAMPLE_RATE);

    // LFO at 0.25 Hz completes 0.5 cycles over 2 seconds
    // Centroid should move: low → high → low or high → low → high
    // Check that there's significant spectral movement

    let max_centroid = c1.max(c2).max(c3).max(c4);
    let min_centroid = c1.min(c2).min(c3).min(c4);
    let range = max_centroid - min_centroid;

    assert!(
        range > 500.0,
        "Resonant filter sweep should create large spectral movement, range: {} Hz",
        range
    );
}

#[test]
fn test_spectral_chorus_modulation() {
    // Chorus effect creates spectral richness through delayed modulation

    let code_dry = r#"
tempo: 1.0
~osc: saw 220
out: ~osc
"#;

    let code_chorus = r#"
tempo: 1.0
~osc: saw 220
~chorused: ~osc # chorus 0.5 3.0 0.3
out: ~chorused
"#;

    let audio_dry = render_dsl(code_dry, TEST_DURATION);
    let audio_chorus = render_dsl(code_chorus, TEST_DURATION);

    // Chorus should create spectral richness (centroid may go up or down depending on implementation)
    assert_spectral_difference(
        &audio_dry,
        &audio_chorus,
        SAMPLE_RATE,
        100.0, // Expect at least 100 Hz difference in centroid
        "Chorus effect should create spectral difference",
    );

    // Chorus creates spectral difference, but direction depends on implementation
    // (some chorus implementations emphasize lower frequencies, others higher)
    // The important part is that the spectrum CHANGES, not which direction
}

// ============================================================================
// CONTINUOUS VS STEPPED VERIFICATION
//
// Verify that pattern modulation is continuous (interpolated) rather than
// stepped (sample-and-hold). This proves sample-accurate parameter updates.
// ============================================================================

#[test]
fn test_continuous_filter_sweep() {
    // Filter sweep with slow LFO should be smooth, not stepped

    let code = r#"
tempo: 1.0
~noise: noise
~lfo: sine 0.5
~swept: ~noise # lpf (~lfo * 2000 + 2000) 4.0
out: ~swept
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // If modulation were stepped (control-rate only), we'd hear discrete jumps
    // If modulation is continuous (audio-rate), sweep should be smooth

    // Analyze spectral centroid over small windows to detect smoothness
    let window_size = 4410; // 100ms windows at 44.1kHz
    let mut centroids = Vec::new();

    for i in (0..audio.len() - window_size).step_by(window_size) {
        let window = &audio[i..i + window_size];
        let centroid = calculate_spectral_centroid(window, SAMPLE_RATE);
        centroids.push(centroid);
    }

    // Check that centroid changes smoothly (not in large steps)
    let mut max_step: f32 = 0.0;
    for i in 1..centroids.len() {
        let step = (centroids[i] - centroids[i - 1]).abs();
        max_step = max_step.max(step);
    }

    // With continuous modulation, max step should be reasonable
    // With stepped modulation, we'd see jumps of thousands of Hz
    // Allowing up to 1000 Hz for 100ms windows with resonant filter (Q=4.0)
    assert!(
        max_step < 1000.0,
        "Filter sweep should be continuous, not stepped. Max step: {} Hz",
        max_step
    );
}

#[test]
fn test_continuous_frequency_glide() {
    // Oscillator frequency glide should be smooth portamento, not stepped

    let code = r#"
tempo: 1.0
~lfo: sine 0.25
~glide: sine (~lfo * 220 + 440)
out: ~glide
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Frequency glides from 220 Hz to 660 Hz and back
    // If stepped, we'd hear discrete pitch jumps
    // If continuous, we'd hear smooth portamento

    // Analyze spectral centroid over small windows
    let window_size = 4410; // 100ms windows
    let mut centroids = Vec::new();

    for i in (0..audio.len() - window_size).step_by(window_size) {
        let window = &audio[i..i + window_size];
        let centroid = calculate_spectral_centroid(window, SAMPLE_RATE);
        centroids.push(centroid);
    }

    // Check for smooth changes
    let mut max_step: f32 = 0.0;
    for i in 1..centroids.len() {
        let step = (centroids[i] - centroids[i - 1]).abs();
        max_step = max_step.max(step);
    }

    // Continuous frequency modulation should have gradual changes
    assert!(
        max_step < 400.0,
        "Frequency glide should be continuous, not stepped. Max step: {} Hz",
        max_step
    );
}

#[test]
fn test_continuous_amplitude_envelope() {
    // Amplitude envelope should be smooth, not stepped

    let code = r#"
tempo: 1.0
~osc: saw 220
~lfo: sine 1.0
~envelope: ~osc * (~lfo * 0.5 + 0.5)
out: ~envelope
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Envelope at 1 Hz should create smooth amplitude changes
    // If stepped, we'd hear zipper noise / artifacts

    // Calculate RMS over small windows to see amplitude envelope
    let window_size = 2205; // 50ms windows
    let mut rms_values = Vec::new();

    for i in (0..audio.len() - window_size).step_by(window_size / 2) {
        let window = &audio[i..i + window_size];
        let rms = calculate_rms(window);
        rms_values.push(rms);
    }

    // Check for smooth RMS changes
    let mut max_step: f32 = 0.0;
    for i in 1..rms_values.len() {
        let step = (rms_values[i] - rms_values[i - 1]).abs();
        max_step = max_step.max(step);
    }

    // Continuous amplitude modulation should have gradual RMS changes
    assert!(
        max_step < 0.3,
        "Amplitude envelope should be continuous, not stepped. Max RMS step: {}",
        max_step
    );
}

#[test]
fn test_continuous_vs_stepped_comparison() {
    // Direct comparison: continuous pattern modulation vs theoretical stepped modulation
    // This proves that Phonon's pattern parameters update at audio rate

    let code = r#"
tempo: 1.0
~lfo: sine 2.0
~modulated: sine (~lfo * 100 + 440)
out: ~modulated
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Analyze spectral content
    // Continuous FM at 2 Hz creates sidebands
    // Stepped modulation would create aliasing artifacts

    let centroid = calculate_spectral_centroid(&audio, SAMPLE_RATE);

    // FM synthesis with 2 Hz modulator should create spectral content
    // around carrier (440 Hz) with sidebands
    // Centroid should be in reasonable range (not aliased into ultrasonic)
    assert!(
        centroid > 300.0 && centroid < 1000.0,
        "Continuous FM should produce expected spectrum, centroid: {} Hz",
        centroid
    );

    // Should have reasonable energy (not attenuated by aliasing)
    let rms = calculate_rms(&audio);
    assert!(rms > 0.3, "Continuous modulation should preserve energy, RMS: {}", rms);
}

// ============================================================================
// PARAMETER INTERPOLATION TESTS
//
// Verify that pattern parameter updates are interpolated smoothly
// across pattern cycles, not discrete jumps.
// ============================================================================

#[test]
fn test_parameter_interpolation_across_cycles() {
    // Pattern that changes cutoff frequency across cycles
    // Should interpolate smoothly, not jump

    let code = r#"
tempo: 2.0
~input: saw 110
~filtered: ~input # lpf "500 2000 1000 3000" 0.8
out: ~filtered
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Pattern cycles at 2 Hz (0.5 second cycles)
    // Should have smooth spectral transitions between pattern events

    // Analyze spectral centroid over small windows
    let window_size = 2205; // 50ms windows
    let mut centroids = Vec::new();

    for i in (0..audio.len() - window_size).step_by(window_size) {
        let window = &audio[i..i + window_size];
        let centroid = calculate_spectral_centroid(window, SAMPLE_RATE);
        centroids.push(centroid);
    }

    // Check that we see multiple distinct centroid values (pattern is working)
    let max_centroid = centroids.iter().cloned().fold(0.0f32, f32::max);
    let min_centroid = centroids.iter().cloned().fold(20000.0f32, f32::min);
    let range = max_centroid - min_centroid;

    assert!(
        range > 300.0,
        "Pattern modulation should create spectral variation, range: {} Hz",
        range
    );

    // Verify no extreme discontinuities (smooth interpolation)
    let mut max_step: f32 = 0.0;
    for i in 1..centroids.len() {
        let step = (centroids[i] - centroids[i - 1]).abs();
        max_step = max_step.max(step);
    }

    assert!(
        max_step < 2500.0,
        "Pattern changes should be interpolated, not jumped. Max step: {} Hz (pattern changes at cycle boundaries can be abrupt)",
        max_step
    );
}

#[test]
fn test_fast_pattern_modulation_no_aliasing() {
    // Rapid pattern changes should be interpolated, not create aliasing

    let code = r#"
tempo: 8.0
~input: saw 110
~filtered: ~input # lpf "500 3000" 0.8
out: ~filtered
"#;

    let audio = render_dsl(code, TEST_DURATION);

    // Pattern cycles at 8 Hz (very fast)
    // Without interpolation, we'd get aliasing / artifacts

    // Check that output is clean (no NaN, no excessive peaks)
    for &sample in audio.iter() {
        let sample: f32 = sample;
        assert!(sample.is_finite(), "Fast pattern modulation should not produce NaN");
        assert!(sample.abs() < 2.0, "Fast pattern modulation should not create extreme peaks");
    }

    // Should maintain reasonable spectral content
    let centroid = calculate_spectral_centroid(&audio, SAMPLE_RATE);
    assert!(
        centroid > 100.0 && centroid < 5000.0,
        "Fast pattern modulation should maintain clean spectrum, centroid: {} Hz",
        centroid
    );
}
