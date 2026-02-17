# Audio Fingerprinting for Pattern Validation

**Research Task:** `research-audio-fingerprinting`
**Date:** 2025-01-28

## Executive Summary

This document explores audio fingerprinting techniques for pattern validation in Phonon. After researching the current testing infrastructure and various fingerprinting approaches, we conclude that **audio fingerprinting is not well-suited for Phonon's pattern validation needs**, but we identify several techniques from fingerprinting research that **would** enhance the existing test infrastructure.

## Current Phonon Testing Infrastructure

Phonon already has a comprehensive three-level testing methodology:

### Level 1: Pattern Query Verification
- **Location:** `src/pattern_query.rs`, `tests/pattern_verification_utils.rs`
- **Purpose:** Fast, deterministic pattern logic testing without audio rendering
- **Strengths:** Exact, fast, catches pattern logic bugs

### Level 2: Onset Detection
- **Location:** `tests/pattern_verification_utils.rs`, `tests/audio_verification.rs`
- **Purpose:** Verify audio events occur at expected times
- **Techniques:** RMS-based onset detection with adaptive thresholds
- **Strengths:** Catches timing bugs, missing events, doubled events

### Level 3: Audio Characteristics
- **Location:** `tests/audio_verification.rs`, `src/bin/wav_analyze.rs`
- **Purpose:** Verify signal quality and spectral properties
- **Techniques:** FFT analysis, spectral centroid, RMS, peak detection
- **Strengths:** Catches silence, clipping, filter issues, wrong frequencies

## Audio Fingerprinting Algorithms Researched

### 1. Chromaprint (AcoustID)
- **How it works:** Generates compact hashes from chroma (pitch class) features
- **Purpose:** Music identification (Shazam-style matching)
- **Rust crate:** `rusty-chromaprint`
- **Limitations for Phonon:**
  - Designed for near-identical audio matching
  - Trades precision for search performance
  - Doesn't work well with recorded/degraded audio
  - Focused on melodic/harmonic content (12 pitch classes)

### 2. Shazam-style Spectral Peaks
- **How it works:** Creates "constellation maps" of spectral peaks, pairs peaks to create hashes
- **Purpose:** Robust identification against noise/distortion
- **Strengths:**
  - Very robust to noise
  - Fast database lookup
  - Time-offset invariant
- **Limitations for Phonon:**
  - Designed for matching against a database of songs
  - Requires significant audio (5-30 seconds for reliability)
  - Not suitable for synthesized/generated audio verification

### 3. Perceptual Hashing (pHash)
- **How it works:** Creates hashes that are similar for perceptually-similar content
- **Similarity metric:** Hamming distance between hash values
- **Threshold:** ~0.25-0.35 for audio similarity
- **Limitations for Phonon:**
  - Designed for duplicate detection/copyright
  - Pattern variations are intentional in Phonon, not duplicates
  - Hashes would differ for `fast 2` vs `fast 3` versions of same pattern

## Why Traditional Fingerprinting Doesn't Fit Phonon

### Problem 1: Phonon Patterns Are Generative
Traditional fingerprinting answers: "Is audio A the same as audio B?"
Phonon needs to answer: "Does audio A match pattern specification P?"

Fingerprinting compares two audio signals. Phonon needs to verify that generated audio matches a symbolic pattern description.

### Problem 2: Intentional Variation
Pattern transformations (`fast`, `slow`, `every`, etc.) intentionally create different audio. A fingerprint-based approach would flag these as mismatches.

### Problem 3: Real-time Modulation
Phonon's unique feature is pattern-controlled synthesis parameters. Verifying that `~lfo # sine 2` correctly modulates a filter cutoff requires spectral analysis, not fingerprint matching.

### Problem 4: Scale and Efficiency
Phonon tests are:
- Short duration (1-4 cycles, typically 0.5-4 seconds)
- Deterministic (same pattern → same audio)
- Fast (no database lookup needed)

Fingerprinting is optimized for:
- Longer audio (5-30+ seconds)
- Probabilistic matching
- Large-scale search

## Recommended Enhancements from Fingerprinting Research

While full fingerprinting isn't appropriate, several techniques from the research would enhance Phonon's test infrastructure:

### 1. Spectral Peak Detection (from Shazam)

**What:** Identify prominent time-frequency peaks in spectrograms
**How it helps:** More robust onset detection for percussive sounds

```rust
/// Detect spectral peaks using local maximum in time-frequency plane
pub fn detect_spectral_peaks(
    spectrogram: &[Vec<f32>],
    neighborhood_time: usize,
    neighborhood_freq: usize,
) -> Vec<(usize, usize, f32)>  // (time_bin, freq_bin, magnitude)
```

**Use case:** Better sample playback verification - current RMS-based onset detection can miss quiet onsets or double-count reverberant samples.

### 2. Chroma Features (from Chromaprint)

**What:** Map spectral energy to 12 pitch classes (C, C#, D, etc.)
**How it helps:** Verify melodic patterns produce correct pitch content

```rust
/// Calculate chroma (pitch class) features
pub fn calculate_chroma(audio: &[f32], sample_rate: u32) -> Vec<[f32; 12]>
```

**Use case:** Testing melodic sequences like `"60 64 67"` (C major triad MIDI notes) - verify the audio actually contains these pitch classes.

### 3. Temporal Hash Sequences (from fingerprinting)

**What:** Instead of one hash, create a sequence of frame hashes
**How it helps:** Verify temporal evolution of audio matches pattern

```rust
/// Create sequence of frame fingerprints for temporal pattern matching
pub fn fingerprint_sequence(
    audio: &[f32],
    sample_rate: u32,
    frame_duration_ms: f32,
) -> Vec<u64>  // Sequence of frame hashes
```

**Use case:** For patterns with `every 4 (rev)`, verify that the 4th repetition is actually reversed by comparing frame hash sequences.

### 4. Spectral Flux Enhancement

**What:** Measure rate of change of spectrum (already partially implemented)
**How it helps:** More sensitive onset detection, modulation verification

The current `audio_verification_enhanced.rs` has basic spectral flux, but could be enhanced with:
- Half-wave rectification (only detect increases)
- Frequency-band specific flux
- Adaptive normalization

### 5. Rhythm Pattern Matching

**What:** Extract inter-onset intervals, match against expected pattern
**How it helps:** Verify rhythmic correctness independent of sample content

```rust
/// Extract rhythm pattern as sequence of inter-onset intervals
pub fn extract_rhythm_pattern(onset_times: &[f32]) -> Vec<f32> {
    onset_times.windows(2).map(|w| w[1] - w[0]).collect()
}

/// Compare two rhythm patterns with tolerance
pub fn compare_rhythm_patterns(
    expected: &[f32],
    actual: &[f32],
    tolerance: f32,
) -> f32  // 0.0 = no match, 1.0 = perfect match
```

**Use case:** Verify `s "bd sn hh cp"` produces 4 equally-spaced events per cycle.

## Implementation Roadmap

### Phase 1: Enhanced Onset Detection (Low Effort, High Value)
1. Add spectral peak-based onset detection
2. Combine with existing RMS-based detection for hybrid approach
3. Add rhythm pattern extraction and comparison

### Phase 2: Spectral Verification (Medium Effort, High Value)
1. Add chroma feature calculation
2. Enhance spectral flux with frequency band analysis
3. Add pitch class verification for melodic patterns

### Phase 3: Temporal Pattern Matching (Higher Effort)
1. Frame-level fingerprinting for temporal verification
2. Pattern transformation verification (rev, fast, etc.)
3. Modulation envelope extraction and comparison

## Rust Libraries to Consider

| Library | Purpose | Status |
|---------|---------|--------|
| `rustfft` | FFT computation | **Already used** |
| `rusty-chromaprint` | Chromaprint fingerprinting | Consider for chroma features |
| `spectrum_analyzer` | Enhanced spectral analysis | Already in deps |
| `pitch_detection` | Pitch/frequency detection | Consider for melodic tests |
| `aubio-rs` | Onset/tempo detection | Consider for rhythm analysis |

## Conclusion

Traditional audio fingerprinting is designed for content identification in large databases, which doesn't match Phonon's pattern validation needs. However, several component techniques from fingerprinting research can enhance the existing test infrastructure:

1. **Spectral peak detection** for better onset detection
2. **Chroma features** for melodic verification
3. **Rhythm pattern matching** for temporal correctness
4. **Enhanced spectral flux** for modulation verification

The recommended approach is to selectively incorporate these techniques into the existing three-level testing methodology rather than implementing a full fingerprinting system.

## References

- [Chromaprint/AcoustID](https://acoustid.org/chromaprint)
- [Shazam Algorithm (Wang 2003)](https://www.ee.columbia.edu/~dpwe/papers/Wang03-shazam.pdf)
- [pHash Perceptual Hashing](https://www.phash.org/)
- [Audio Fingerprinting Survey](https://arxiv.org/html/2408.14155v1)
- [Perceptual Audio Hashing Functions](https://link.springer.com/content/pdf/10.1155/ASP.2005.1780.pdf)
