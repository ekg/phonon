# FFT Test Audit Report

## Summary
Audit of audio tests in Phonon codebase to verify proper use of FFT-based frequency analysis versus simple RMS amplitude checks.

## Tests Using Proper FFT ✅

### 1. test_scale_quantization.rs
- **Status**: ✅ CORRECT
- **Uses**: `rustfft` with proper FFT-based frequency detection
- **Function**: `find_dominant_frequency()` - computes FFT and finds peak frequency
- **Tests**: Scale quantization (major, minor, pentatonic, octave wrapping)
- **Appropriate for**: Verifying musical note frequencies

### 2. test_scale_single_degree.rs  
- **Status**: ✅ CORRECT (inferred from grep results)
- **Uses**: `rustfft`

### 3. test_pattern_audio_e2e.rs
- **Status**: ✅ CORRECT (inferred from grep results)
- **Has**: `compute_fft()` function

## Tests Using RMS Only (Need FFT) ❌

### 1. test_pattern_params_verification.rs
- **Status**: ❌ INCORRECT
- **Issue**: Tests `detune` parameter (frequency modulation) using only RMS amplitude
- **Line 12, 20, 28**: Only computes RMS
- **Problem**: Detune affects FREQUENCY not amplitude - should verify frequency spectrum changes
- **Recommendation**: Add FFT-based frequency detection to verify detune actually changes frequency spread

### 2. test_continuous_pattern_params.rs
- **Status**: ❌ INCORRECT  
- **Test**: `test_supersaw_freq_pattern_actually_cycles`
- **Line 16, 26, 38**: Only computes RMS to verify frequency parameter
- **Problem**: Tests if frequency pattern works by comparing RMS, but frequency changes don't necessarily change RMS significantly
- **Recommendation**: Use FFT to verify the oscillator is actually producing the expected frequencies (110 Hz, 220 Hz)

### 3. test_filter_modulation.rs
- **Status**: ❌ MISLEADING
- **Function**: `compute_spectral_centroid()` claims to compute spectral centroid
- **Reality**: Uses derivative-based approximation (sum of squared sample differences), NOT FFT
- **Lines 8-40**: Pseudo-spectral analysis, not true frequency analysis
- **Problem**: While the derivative method can indicate HF content, it's not a true spectral centroid
- **Recommendation**: Either:
  - Rename function to `compute_hf_energy_ratio()` to be honest about what it does
  - OR implement proper FFT-based spectral centroid: `Σ(f * magnitude(f)) / Σ(magnitude(f))`

### 4. test_voice_dsp_parameters.rs
- **Status**: ✓ ACCEPTABLE
- **Tests**: Gain, pan parameters (amplitude-based)
- **Uses**: RMS and peak amplitude measurements
- **OK because**: Gain and pan are amplitude parameters, so RMS is appropriate
- **Speed tests**: Could benefit from FFT to verify pitch changes, but duration checks are adequate

## Recommendations

### High Priority
1. **Fix `test_pattern_params_verification.rs`**:
   ```rust
   // Add FFT-based frequency detection like test_scale_quantization.rs
   fn find_dominant_frequency(buffer: &[f32], sample_rate: f32) -> f32 { ... }
   
   // Use it to verify detune changes frequency spread
   let freq_spread1 = measure_frequency_spread(buffer1);
   let freq_spread2 = measure_frequency_spread(buffer2);
   assert!(freq_spread2 > freq_spread1 * 1.5);
   ```

2. **Fix `test_continuous_pattern_params.rs`**:
   ```rust
   // Verify actual frequencies in the pattern
   let samples_per_cycle = (44100.0 / 2.0) as usize;
   let segment1 = &buffer[0..samples_per_cycle/2];
   let segment2 = &buffer[samples_per_cycle/2..samples_per_cycle];
   
   let freq1 = find_dominant_frequency(segment1, 44100.0);
   let freq2 = find_dominant_frequency(segment2, 44100.0);
   
   assert!((freq1 - 110.0).abs() < 5.0, "Expected 110 Hz");
   assert!((freq2 - 220.0).abs() < 5.0, "Expected 220 Hz");
   ```

3. **Fix `test_filter_modulation.rs`**:
   - Option A: Implement proper FFT-based spectral centroid
   - Option B: Rename to `compute_hf_energy_ratio()` and document it's an approximation
   - Use actual FFT to verify filter cutoff frequencies affect the spectrum correctly

### Medium Priority
- Add FFT verification to speed parameter tests to verify pitch changes
- Create a shared `audio_test_utils.rs` module with reusable FFT functions:
  - `find_dominant_frequency()`
  - `compute_spectral_centroid()`
  - `measure_frequency_spread()`
  - `find_peaks_in_spectrum()`

### Low Priority
- Add FFT-based verification to any future DSP parameter tests
- Document in CLAUDE.md that frequency-related tests MUST use FFT

## Code Example: Proper FFT Utility Module

```rust
// tests/audio_test_utils.rs
use rustfft::{num_complex::Complex, FftPlanner};

/// Find the dominant frequency in an audio buffer using FFT
pub fn find_dominant_frequency(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());
    
    let mut complex_input: Vec<Complex<f32>> = buffer
        .iter()
        .map(|&x| Complex { re: x, im: 0.0 })
        .collect();
    
    fft.process(&mut complex_input);
    
    // Find peak in FFT (skip DC bin)
    let magnitudes: Vec<f32> = complex_input[1..complex_input.len() / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();
    
    let max_idx = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    
    (max_idx + 1) as f32 * sample_rate / buffer.len() as f32
}

/// Compute true spectral centroid using FFT
pub fn compute_spectral_centroid(buffer: &[f32], sample_rate: f32) -> f32 {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());
    
    let mut complex_input: Vec<Complex<f32>> = buffer
        .iter()
        .map(|&x| Complex { re: x, im: 0.0 })
        .collect();
    
    fft.process(&mut complex_input);
    
    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;
    
    for (i, c) in complex_input[1..complex_input.len() / 2].iter().enumerate() {
        let magnitude = (c.re * c.re + c.im * c.im).sqrt();
        let frequency = (i + 1) as f32 * sample_rate / buffer.len() as f32;
        weighted_sum += frequency * magnitude;
        magnitude_sum += magnitude;
    }
    
    if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    }
}

/// Measure frequency spread (bandwidth) using FFT
pub fn measure_frequency_spread(buffer: &[f32], sample_rate: f32) -> f32 {
    // Compute FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());
    
    let mut complex_input: Vec<Complex<f32>> = buffer
        .iter()
        .map(|&x| Complex { re: x, im: 0.0 })
        .collect();
    
    fft.process(&mut complex_input);
    
    // Find frequency range containing 90% of energy
    let magnitudes: Vec<f32> = complex_input[1..complex_input.len() / 2]
        .iter()
        .map(|c| c.re * c.re + c.im * c.im)
        .collect();
    
    let total_energy: f32 = magnitudes.iter().sum();
    let threshold = 0.05 * total_energy;
    
    let mut low_idx = 0;
    let mut high_idx = magnitudes.len() - 1;
    
    for (i, &mag) in magnitudes.iter().enumerate() {
        if mag > threshold {
            low_idx = i;
            break;
        }
    }
    
    for (i, &mag) in magnitudes.iter().enumerate().rev() {
        if mag > threshold {
            high_idx = i;
            break;
        }
    }
    
    let low_freq = (low_idx + 1) as f32 * sample_rate / buffer.len() as f32;
    let high_freq = (high_idx + 1) as f32 * sample_rate / buffer.len() as f32;
    
    high_freq - low_freq
}
```

## Conclusion

**Current Status**: 
- ✅ 3 test files use proper FFT
- ❌ 3 test files use inappropriate RMS for frequency verification
- ✓ 1 test file uses RMS appropriately (amplitude parameters)

**Action Items**:
1. Create shared `audio_test_utils.rs` module with FFT utilities
2. Fix `test_pattern_params_verification.rs` to use FFT for detune
3. Fix `test_continuous_pattern_params.rs` to use FFT for frequency verification
4. Fix or rename `compute_spectral_centroid()` in `test_filter_modulation.rs`

**Impact**: These fixes will ensure that frequency-related DSP parameters are properly verified, catching bugs that RMS-only tests would miss.
