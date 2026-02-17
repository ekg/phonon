# Music Information Retrieval (MIR) Libraries in Rust

## Research Summary

This document catalogs Rust libraries for Music Information Retrieval (MIR) - the field of extracting meaningful information from audio signals. This research is relevant to Phonon for potential features like beat-synced effects, audio-reactive patterns, live input analysis, and intelligent playlist/sample selection.

---

## Currently Used in Phonon

The project already uses several audio analysis crates (from `Cargo.toml`):

| Crate | Version | Purpose |
|-------|---------|---------|
| `rustfft` | 6.1 | Fast Fourier Transform |
| `realfft` | 3.3 | Real-valued FFT for spectral processing |
| `spectrum-analyzer` | 1.7 | Frequency spectrum analysis |
| `audio-processor-analysis` | 2.4 | Audio analysis processors |
| `audio-processor-traits` | 4.3 | Audio processor trait abstractions |
| `num-complex` | 0.4 | Complex numbers for FFT |
| `fundsp` | 0.18 | Audio DSP library |

---

## Comprehensive MIR Crate Catalog

### 1. Song Analysis & Similarity

#### bliss-audio
- **Crate**: [bliss-audio](https://crates.io/crates/bliss-audio) (v0.11.1)
- **Repository**: [Polochon-street/bliss-rs](https://github.com/Polochon-street/bliss-rs)
- **Purpose**: Analyze songs and compute distances between them for playlist generation
- **Features**:
  - Tempo, timbre, and chroma extraction
  - Euclidean distance between song features
  - Customizable Mahalanobis distance for tailored similarity
  - SQLite database integration for song libraries
  - Python bindings available
- **Dependencies**: Uses aubio-rs internally for spectral/timbral analysis
- **Relevance to Phonon**: Could enable "intelligent" sample selection based on audio characteristics, or auto-matching samples by timbre/tempo

#### audio-similarity-search
- **Crate**: [audio-similarity-search](https://crates.io/crates/audio-similarity-search)
- **Purpose**: Similarity search across audio files using feature vectors
- **Backend**: Uses `arroy` (Rust port of Spotify's Annoy library) with LMDB
- **Relevance to Phonon**: Fast approximate nearest-neighbor search for sample recommendation

---

### 2. Audio Feature Extraction (aubio bindings)

#### aubio-rs / aubio
- **Crate**: [aubio-rs](https://crates.io/crates/aubio-rs), [aubio](https://crates.io/crates/aubio)
- **Repository**: [katyo/aubio-rs](https://github.com/katyo/aubio-rs)
- **Purpose**: Safe Rust bindings to the comprehensive aubio C library
- **Features**:
  - **Onset detection** (segmenting audio at attack points)
  - **Pitch detection** (multiple algorithms)
  - **Beat/tempo tracking** (real-time capable)
  - **MFCC** extraction (mel-frequency cepstral coefficients)
  - **FFT and phase vocoder**
- **Build options**: `pkg-config` to use system aubio, or `builtin` feature
- **Relevance to Phonon**: **HIGH** - The most comprehensive MIR toolkit. Could enable beat-synced effects, pitch-following synthesis, onset-triggered events

#### bliss-audio-aubio-rs
- **Crate**: [bliss-audio-aubio-rs](https://crates.io/crates/bliss-audio-aubio-rs)
- **Purpose**: aubio-rs fork maintained for bliss-audio internal use
- **Note**: If using bliss-audio, this comes bundled

---

### 3. Pitch Detection

#### pitch-detection
- **Crate**: [pitch-detection](https://crates.io/crates/pitch-detection)
- **Docs**: [docs.rs/pitch-detection](https://docs.rs/pitch-detection)
- **Purpose**: Fundamental frequency estimation from audio buffers
- **Algorithms**: McLeod pitch method
- **Features**: WASM-compatible
- **Relevance to Phonon**: Single-voice pitch tracking for melodic input

#### pitch
- **Crate**: [pitch](https://crates.io/crates/pitch)
- **Docs**: [lib.rs/crates/pitch](https://lib.rs/crates/pitch)
- **Purpose**: Bitstream Autocorrelation Function (BCF) pitch detection
- **Features**: Cross-platform, no_std capable, fast and accurate
- **Relevance to Phonon**: Alternative pitch detection for embedded/WASM contexts

#### pyin-rs
- **Repository**: [Sytronik/pyin-rs](https://github.com/Sytronik/pyin-rs)
- **Purpose**: pYIN algorithm (probabilistic YIN) for pitch estimation
- **Output**: Pitch estimate per frame + voiced/unvoiced probability
- **Compatibility**: Matches librosa's implementation
- **Relevance to Phonon**: High-quality pitch tracking with confidence scores

---

### 4. Beat & Tempo Detection

#### beat-detector
- **Crate**: [beat-detector](https://crates.io/crates/beat-detector)
- **Docs**: [docs.rs/beat-detector](https://docs.rs/beat-detector)
- **Purpose**: Live audio beat detection
- **Features**: Multiple strategies, device input support
- **Relevance to Phonon**: **HIGH** - Could sync patterns to external audio input

#### tempor
- **Crate**: [tempor](https://lib.rs/crates/tempor)
- **Purpose**: Tempo-related utilities (tap tempo, tempo tracking)

---

### 5. Chord & Harmony Detection

#### chord_detector
- **Crate**: [chord_detector](https://crates.io/crates/chord_detector)
- **Purpose**: Real-time chord detection from audio
- **Features**:
  - Stream-based 12-bin chromagram computation
  - Chord matching with bleed suppression
  - Zero-allocation in hot path
  - Supports: Major, Minor, Power, Dom7, Maj7, Min7, Dim, Aug, Sus2, Sus4
- **Based on**: A. M. Stark & M. D. Plumbley, "Real-Time Chord Recognition For Live Performance" (ICMC 2009)
- **Relevance to Phonon**: Enable chord-following synthesis or harmonic analysis of input

#### ferrous-waves
- **Repository**: [willibrandon/ferrous-waves](https://github.com/willibrandon/ferrous-waves)
- **Purpose**: High-fidelity audio analysis library
- **Features**:
  - Key detection with confidence
  - Chord progression analysis
  - Harmonic complexity metrics
  - Spectral/temporal analysis
  - Audio fingerprinting
- **Relevance to Phonon**: Comprehensive harmonic analysis

---

### 6. Key Detection

#### libkeyfinder-sys
- **Crate**: [libkeyfinder-sys](https://crates.io/crates/libkeyfinder-sys)
- **Purpose**: Rust bindings to libKeyFinder (C++11 library)
- **Use case**: Musical key estimation for digital audio
- **Relevance to Phonon**: Detect key of samples for harmonic mixing

#### stratum-dsp
- **Crate**: [stratum-dsp](https://crates.io/crates/stratum-dsp)
- **Purpose**: Professional-grade audio analysis for DJ applications
- **Features**: BPM detection, key detection, beat tracking
- **Relevance to Phonon**: **HIGH** - All-in-one DJ-style analysis

---

### 7. MFCC & Spectral Features

#### mfcc
- **Crate**: [mfcc](https://lib.rs/crates/mfcc)
- **Repository**: [bytesnake/mfcc](https://github.com/bytesnake/mfcc)
- **Purpose**: Mel Frequency Cepstral Coefficients calculation
- **FFT backends**: `rustfft` or `fftw`
- **Use cases**: ASR, speaker recognition, room classification
- **Relevance to Phonon**: Timbral feature extraction for sample analysis

#### mfcc-rust (secretsauceai)
- **Repository**: [secretsauceai/mfcc-rust](https://github.com/secretsauceai/mfcc-rust)
- **Purpose**: Mel spectrogram and MFCC matching librosa's output
- **Compatibility**: Designed for ML model interop (train in Python, infer in Rust)
- **Relevance to Phonon**: If using pre-trained models

---

### 8. FFT & Spectrum Analysis

#### rustfft ✓ (already in Phonon)
- **Crate**: [rustfft](https://crates.io/crates/rustfft) (v6.x)
- **Purpose**: Pure Rust FFT implementation
- **Notes**: High performance, no external dependencies

#### realfft ✓ (already in Phonon)
- **Crate**: [realfft](https://crates.io/crates/realfft)
- **Purpose**: Real-to-complex FFT wrapper around rustfft
- **Benefits**: 2x speedup for real-valued data (audio)

#### spectrum-analyzer ✓ (already in Phonon)
- **Crate**: [spectrum-analyzer](https://crates.io/crates/spectrum-analyzer)
- **Purpose**: Easy frequency spectrum extraction
- **Features**: Window functions (Hann, Hamming), no_std capable

#### spectrograms
- **Repository**: [jmg049/Spectrograms](https://github.com/jmg049/Spectrograms)
- **Purpose**: Spectrogram computation
- **Backends**: realfft or fftw

---

### 9. Audio Fingerprinting

#### rusty-chromaprint
- **Crate**: [rusty-chromaprint](https://crates.io/crates/rusty-chromaprint)
- **Purpose**: Pure Rust port of Chromaprint (AcoustID project)
- **Use case**: Identify near-identical audio
- **Relevance to Phonon**: Duplicate detection, sample identification

#### khalzam
- **Crate**: [khalzam](https://docs.rs/khalzam)
- **Purpose**: Audio recognition/indexing library
- **Features**: Speed and efficiency focused
- **Relevance to Phonon**: Fast sample matching

#### songrec-lib
- **Crate**: [songrec-lib](https://lib.rs/crates/songrec-lib)
- **Purpose**: Headless Shazam client library
- **Use case**: Audio recognition against Shazam database

---

### 10. Audio DSP Foundations

#### dasp
- **Crate**: [dasp](https://crates.io/crates/dasp)
- **Repository**: [RustAudio/dasp](https://github.com/RustAudio/dasp)
- **Purpose**: Digital Audio Signal Processing fundamentals
- **Features**:
  - Sample type conversions
  - Signal trait for streaming audio
  - Frame and channel abstractions
  - Resampling
  - No heap allocations, no_std capable
- **Relevance to Phonon**: Low-level audio infrastructure

#### fundsp ✓ (already in Phonon)
- **Crate**: [fundsp](https://crates.io/crates/fundsp)
- **Repository**: [SamiPerttu/fundsp](https://github.com/SamiPerttu/fundsp)
- **Purpose**: Audio DSP with inline graph notation
- **Features**:
  - Composable audio networks as Rust types
  - Compile-time connectivity checking
  - Analytic frequency response computation
  - Many built-in oscillators, filters, effects

---

### 11. Source Separation

#### charon-audio
- **Crate**: [charon-audio](https://crates.io/crates/charon-audio)
- **Purpose**: Pure Rust music source separation (Demucs-inspired)
- **Features**:
  - Stem separation (drums, bass, vocals, other)
  - No Python dependencies
  - ONNX Runtime and HuggingFace Candle backends
  - KNN-based audio similarity search
- **Relevance to Phonon**: **HIGH** - Could enable stem extraction from samples

---

### 12. Audio I/O & Visualization

#### audio-visualizer
- **Purpose**: Plot audio waveforms and spectra for algorithm development
- **Source**: [phip1611.de blog](https://phip1611.de/blog/live-audio-visualization-with-rust-in-a-gui-window/)

---

## Recommended Additions for Phonon

Based on this research, here are prioritized recommendations:

### High Priority (Immediate Value)

1. **aubio-rs** - Comprehensive MIR toolkit
   - Enables: onset detection, pitch tracking, beat tracking
   - Use case: Beat-synced effects, audio-reactive patterns

2. **beat-detector** - Live beat detection
   - Enables: Sync Phonon patterns to external audio
   - Use case: DJ-style live performance

3. **bliss-audio** - Song analysis
   - Enables: Intelligent sample recommendation
   - Use case: "Find similar samples" feature

### Medium Priority (Future Features)

4. **chord_detector** - Real-time chord detection
   - Enables: Harmonic-aware synthesis
   - Use case: Auto-harmonizing patterns

5. **stratum-dsp** - DJ-grade analysis
   - Enables: BPM/key detection for samples
   - Use case: Sample library organization

6. **charon-audio** - Source separation
   - Enables: Extract stems from samples
   - Use case: Isolate drums/bass/vocals from tracks

### Low Priority (Specialized)

7. **rusty-chromaprint** - Audio fingerprinting
8. **libkeyfinder-sys** - Key detection
9. **pyin-rs** - High-quality pitch tracking

---

## Implementation Notes

### Pattern Integration Strategy

Phonon's core strength is pattern-as-control-signal. MIR features should expose audio analysis as patterns:

```phonon
-- Hypothetical future syntax
~input $ audio_in 1                      -- Audio input bus
~beats $ onset_detect ~input              -- Pattern of beat events
~pitch $ pitch_track ~input               -- Continuous pitch pattern

-- Use analysis to control synthesis
out $ s "bd" $ every (~beats) (fast 2)    -- Double tempo on beats
out $ sine ~pitch # gain 0.3              -- Follow input pitch
```

### Dependency Considerations

- **aubio-rs**: Requires aubio C library (can build from source via `builtin` feature)
- **bliss-audio**: Heavy dependency chain, uses aubio internally
- **Pure Rust options**: pitch-detection, rusty-chromaprint, chord_detector (no C deps)

---

## References

- [RustAudio GitHub Organization](https://github.com/RustAudio)
- [lib.rs Audio Category](https://lib.rs/multimedia/audio)
- [crates.io Audio Keyword](https://crates.io/keywords/audio)
- [aubio Library](https://aubio.org/)
- [Essentia (C++ reference)](https://essentia.upf.edu/)
- [librosa (Python reference)](https://librosa.org/)
