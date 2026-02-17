# Spectral Analysis for Beat Matching - Research Summary

## Overview

This document summarizes research on spectral analysis techniques for beat matching in Phonon. The goal is to enable automatic tempo detection, beat alignment, and beat-synchronized effects for samples and audio streams.

## Current Codebase Capabilities

### Existing FFT Infrastructure

Phonon already has robust FFT infrastructure:

1. **Dependencies** (`Cargo.toml`):
   - `rustfft = "6.1"` - General-purpose FFT
   - `realfft = "3.3"` - Real-valued FFT (more efficient for audio)
   - `spectrum-analyzer = "1.7"` - Spectral analysis utilities

2. **Existing Implementations**:
   - `src/nodes/spectral_freeze.rs` - FFT-based spectral freezing (1024-point, 75% overlap, Hann window)
   - `src/bin/wav_analyze.rs` - Full spectral analysis CLI tool with BPM estimation
   - `src/audio_analysis.rs` - Real-time pitch, transient, and centroid detection

### Current Onset Detection (`wav_analyze.rs:375-467`)

The existing implementation uses:
- Energy-based onset detection with 20ms windows
- Adaptive threshold: `mean_energy + 1.5 × std_dev`
- Minimum peak distance of 100ms (prevents double-detection)
- BPM estimation from median inter-onset interval

**Limitations**:
- Energy-only detection (not phase-aware)
- Single-band analysis (no frequency separation)
- No beat grid extraction or quantization

---

## Spectral Analysis Algorithms for Beat Matching

### 1. Spectral Flux (Recommended Primary Algorithm)

**How it works**:
Spectral flux measures the rate of change in the power spectrum between consecutive frames. It detects onsets by finding moments where new spectral energy appears.

```
spectral_flux[n] = Σ H(|X[n,k]| - |X[n-1,k]|)
where H(x) = max(0, x) is the half-wave rectifier
```

**Implementation**:
1. STFT with Hann window (1024-2048 samples, 50-75% overlap)
2. Compute magnitude spectrum for each frame
3. Half-wave rectified difference: `max(0, current_mag - prev_mag)`
4. Sum across frequency bins
5. Apply adaptive threshold for peak picking

**Advantages**:
- Better than energy-based for pitched instruments
- Detects spectral changes, not just amplitude changes
- Standard approach used by Essentia, librosa, madmom

### 2. Multi-Band Spectral Flux (For Drum Separation)

**Key insight**: Different instruments occupy different frequency bands:
- **Kick drum**: 60-130 Hz
- **Snare drum**: 150-500 Hz (plus noise content 5-15 kHz)
- **Hi-hats/cymbals**: 8-16 kHz

**Implementation**:
1. Compute STFT
2. Split spectrum into sub-bands (3-8 bands typical)
3. Compute spectral flux for each band independently
4. Optionally weight bands differently for specific detection tasks

```rust
struct MultiBandOnsetDetector {
    bands: [(f32, f32); 4],  // (low_freq, high_freq)
    // Band 0: Kick (60-130 Hz)
    // Band 1: Snare low (130-500 Hz)
    // Band 2: Snare/toms (500-3000 Hz)
    // Band 3: Hi-hat/cymbals (3000-16000 Hz)
}
```

### 3. Superflux Algorithm (Vibrato Suppression)

**Problem**: Standard spectral flux produces false positives from vibrato.

**Solution**: Compare current bin to the *maximum* of neighboring bins in previous frame:
```
superflux[n] = Σ H(|X[n,k]| - max(|X[n-1,k-w:k+w]|))
```

Where `w` is typically 1-3 bins. This suppresses gradual frequency modulation.

**Use case**: Detecting beats in melodic content with vibrato/tremolo.

### 4. Beat Tracking (Beyond Onset Detection)

Onset detection finds *all* note starts. Beat tracking finds the *regular pulse*.

**Approach 1: Autocorrelation**
1. Compute onset detection function (ODF)
2. Autocorrelate ODF to find dominant periodicities
3. Select the strongest periodicity as tempo estimate
4. Track beat phase to find exact beat positions

**Approach 2: Comb Filter Bank**
1. Create bank of comb filters at different BPM values (60-200 BPM)
2. Filter the ODF through each comb
3. BPM with highest output is the tempo estimate

**Approach 3: Dynamic Programming (Viterbi)**
1. Model beats as hidden states
2. Transition probabilities based on tempo model
3. Observation probabilities from onset strength
4. Viterbi decoding finds optimal beat sequence

---

## Recommended Implementation Plan

### Phase 1: Improved Onset Detection (High Priority)

Replace current energy-based detection with spectral flux:

```rust
pub struct SpectralFluxDetector {
    fft_size: usize,           // 2048 typical
    hop_size: usize,           // 512 typical (75% overlap)
    sample_rate: f32,
    window: Vec<f32>,          // Hann window
    prev_spectrum: Vec<f32>,   // Previous frame magnitudes

    // FFT planner (realfft for efficiency)
    r2c: Arc<dyn RealToComplex<f32>>,

    // Peak picking
    onset_threshold: f32,      // Adaptive or fixed
    min_onset_gap_samples: usize,
}

impl SpectralFluxDetector {
    pub fn process_block(&mut self, samples: &[f32]) -> Vec<OnsetEvent> {
        // 1. Window and FFT
        // 2. Compute magnitudes
        // 3. Half-wave rectified difference vs prev_spectrum
        // 4. Sum to get spectral flux
        // 5. Peak picking with adaptive threshold
        // 6. Return onset times
    }
}
```

### Phase 2: Multi-Band Detection (Medium Priority)

Add frequency-band separation for drum-specific detection:

```rust
pub struct MultiBandOnsetDetector {
    detectors: Vec<SpectralFluxDetector>,
    band_ranges: Vec<(f32, f32)>,  // (low_hz, high_hz) for each band
}

impl MultiBandOnsetDetector {
    pub fn new_drum_optimized(sample_rate: f32) -> Self {
        Self {
            band_ranges: vec![
                (20.0, 130.0),      // Kick
                (130.0, 500.0),     // Snare body
                (500.0, 3000.0),    // Toms/snare crack
                (3000.0, 16000.0),  // Hi-hats/cymbals
            ],
            // ...
        }
    }

    pub fn process_block(&mut self, samples: &[f32]) -> DrumOnsets {
        // Returns separate onset streams for each band
    }
}
```

### Phase 3: Beat Grid Extraction (Lower Priority)

Extract regular beat grid from onsets:

```rust
pub struct BeatTracker {
    onset_buffer: Vec<f32>,      // Buffer of onset strengths
    tempo_range: (f32, f32),     // BPM range to search (e.g., 60-200)

    // State
    current_tempo: f32,
    current_phase: f32,
    confidence: f32,
}

impl BeatTracker {
    pub fn process_onsets(&mut self, onsets: &[OnsetEvent]) -> Option<BeatGrid> {
        // 1. Autocorrelate onset function
        // 2. Find dominant period
        // 3. Estimate phase
        // 4. Return beat grid if confident
    }
}
```

---

## Available Rust Crates

### Primary Option: `beat-detector`

- [crates.io](https://crates.io/crates/beat-detector)
- Pure Rust, no external dependencies
- Real-time capable
- Two detection strategies available

### Alternative: `aubio-rs`

- [crates.io](https://docs.rs/aubio-rs)
- Rust bindings to aubio (C library)
- Industry-standard algorithms
- More features but requires C library

### DJ-Grade: `stratum-dsp`

- [crates.io](https://crates.io/crates/stratum-dsp)
- Professional-grade for DJ applications
- BPM detection, key detection, beat tracking

**Recommendation**: Start with implementing spectral flux directly (we have all dependencies), then evaluate `beat-detector` or `stratum-dsp` if more sophistication is needed.

---

## Integration with Phonon

### Use Case 1: Sample BPM Detection

```phonon
-- Automatically detect BPM of a sample
~loop $ s "break:5" # bpm_detect
-- Returns detected BPM as a control value
```

### Use Case 2: Beat-Synchronized Playback

```phonon
-- Stretch/compress sample to match tempo
~loop $ s "break:5" # stretch_to_tempo
```

### Use Case 3: Onset-Triggered Events

```phonon
-- Trigger pattern on detected kicks
~kicks $ onset_detect "kick" "break:5"
out $ s ~kicks # bd
```

### Use Case 4: Beat Grid Quantization

```phonon
-- Quantize loose performance to grid
~drums $ s "bd ~ sn ~" # quantize 0.1  -- 10% grid pull
```

---

## Performance Considerations

1. **FFT Size**: 2048 samples @ 44.1kHz = 46ms resolution, 21Hz bin width
2. **Hop Size**: 512 samples = 11.6ms, good for 120 BPM (500ms/beat)
3. **Real-time**: With 512-sample hop, need to process ~86 FFTs/second
4. **Memory**: 2048-point complex FFT = 16KB per buffer

The existing `realfft` infrastructure in Phonon is well-suited for this workload.

---

## References

1. Bello et al. (2005) "A Tutorial on Onset Detection in Music Signals"
2. Dixon (2006) "Onset Detection Revisited"
3. Böck & Widmer (2013) "Maximum Filter Vibrato Suppression for Onset Detection" (Superflux)
4. McFee & Ellis (2014) "Better Beat Tracking Through Robust Onset Aggregation"
5. [Essentia Documentation - OnsetDetection](https://essentia.upf.edu/reference/streaming_OnsetDetection.html)
6. [madmom - Python Audio Feature Extraction](https://madmom.readthedocs.io/)

---

## Next Steps

1. **Implement `SpectralFluxDetector`** in `src/audio_analysis.rs`
2. **Add tests** using known drum loops with verified BPM
3. **Integrate with wav_analyze.rs** for validation
4. **Add DSL syntax** for beat detection/matching operations
5. **Create musical examples** demonstrating beat-synchronized effects

---

*Document created: 2026-01-28*
*Task: research-spectral-analysis*
