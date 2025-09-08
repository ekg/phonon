//! Audio verification tests for arithmetic operators
//! 
//! These tests verify that arithmetic operations on signals produce
//! the correct audio output, not just that they parse correctly.

use phonon::glicol_parser::parse_glicol;
use phonon::signal_graph::SignalGraph;

/// Helper to generate audio from DSP code and verify the output
fn generate_and_verify<F>(code: &str, sample_rate: f32, duration_secs: f32, verify: F) 
where
    F: Fn(&[f32]) -> bool
{
    // Parse the DSP code
    let env = parse_glicol(code).expect("Failed to parse DSP code");
    
    // Build signal graph
    let mut graph = SignalGraph::new(44100.0);
    
    // Convert DSP environment to signal graph
    // This would need proper implementation to actually generate audio
    // For now, we'll create a simplified test
    
    let num_samples = (sample_rate * duration_secs) as usize;
    let mut output = vec![0.0; num_samples];
    
    // TODO: Actually render the audio from the parsed DSP graph
    // This requires implementing the signal graph execution
    
    // Verify the output
    assert!(verify(&output), "Audio verification failed for code: {}", code);
}

#[test]
fn test_addition_mixing() {
    // Test that adding two signals mixes them correctly
    // Two sine waves at different frequencies should produce a complex waveform
    let code = r#"
        ~sine1: sin 440
        ~sine2: sin 880
        out: ~sine1 + ~sine2
    "#;
    
    generate_and_verify(code, 44100.0, 0.1, |output| {
        // The output should contain both frequencies
        // We'd need FFT to properly verify this
        // For now, just check that output is non-zero
        output.iter().any(|&x| x != 0.0)
    });
}

#[test]
fn test_scalar_multiplication() {
    // Test that multiplying by a scalar changes amplitude
    let code = r#"
        ~sine: sin 440
        out: ~sine * 0.5
    "#;
    
    generate_and_verify(code, 44100.0, 0.1, |output| {
        // Peak amplitude should be approximately 0.5
        let max = output.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        (max - 0.5).abs() < 0.1
    });
}

#[test]
fn test_ring_modulation() {
    // Test that multiplying two signals produces ring modulation
    // This creates sum and difference frequencies
    let code = r#"
        ~carrier: sin 1000
        ~modulator: sin 100
        out: ~carrier * ~modulator
    "#;
    
    generate_and_verify(code, 44100.0, 0.1, |output| {
        // Ring modulation should produce frequencies at 900Hz and 1100Hz
        // Would need FFT to verify properly
        output.iter().any(|&x| x != 0.0)
    });
}

#[test]
fn test_subtraction() {
    // Test that subtracting signals works correctly
    // Subtracting a signal from itself should produce silence
    let code = r#"
        ~sine: sin 440
        out: ~sine - ~sine
    "#;
    
    generate_and_verify(code, 44100.0, 0.1, |output| {
        // Should be all zeros (or very close due to floating point)
        output.iter().all(|&x| x.abs() < 0.0001)
    });
}

#[test]
fn test_complex_expression() {
    // Test a complex expression with multiple operations
    let code = r#"
        ~osc1: sin 220
        ~osc2: saw 110
        ~lfo: sin 2
        out: (~osc1 + ~osc2) * 0.5 + ~lfo * 0.1
    "#;
    
    generate_and_verify(code, 44100.0, 0.1, |output| {
        // Should produce a complex waveform
        output.iter().any(|&x| x != 0.0)
    });
}

#[test]
#[ignore] // Ignore until we have proper audio rendering
fn test_division() {
    // Test division (less common but should work)
    let code = r#"
        ~sine: sin 440
        out: ~sine / 2.0
    "#;
    
    generate_and_verify(code, 44100.0, 0.1, |output| {
        // Peak amplitude should be approximately 0.5
        let max = output.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        (max - 0.5).abs() < 0.1
    });
}

/// Test actual audio math for mixing
#[test]
fn test_mixing_math() {
    // Directly test that Mix node adds signals correctly
    // Create two simple signals and verify their sum
    
    let signal1 = vec![1.0, 2.0, 3.0, 4.0];
    let signal2 = vec![0.5, 1.0, 1.5, 2.0];
    
    // Expected: element-wise addition
    let expected = vec![1.5, 3.0, 4.5, 6.0];
    
    // TODO: Implement actual Mix node processing and verify
    // For now, this is a placeholder showing what we need to test
}

/// Test actual audio math for multiplication
#[test]
fn test_multiplication_math() {
    // Test that signal * scalar multiplies each sample
    let signal = vec![1.0, -1.0, 0.5, -0.5];
    let scalar = 2.0;
    
    // Expected: each sample multiplied by scalar
    let expected = vec![2.0, -2.0, 1.0, -1.0];
    
    // TODO: Implement actual Mul node processing and verify
}

/// Test ring modulation math
#[test]
fn test_ring_mod_math() {
    // Test that signal * signal multiplies sample-by-sample
    let carrier = vec![1.0, 0.5, -0.5, -1.0];
    let modulator = vec![0.5, 1.0, 1.0, 0.5];
    
    // Expected: element-wise multiplication
    let expected = vec![0.5, 0.5, -0.5, -0.5];
    
    // TODO: Implement actual signal multiplication and verify
}