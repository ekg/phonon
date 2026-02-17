# Onset Detection Algorithms: Research Summary

This document compares onset detection algorithms for use in Phonon's audio testing verification system.

## Current Implementation Analysis

Phonon currently has **two onset detection implementations**:

### 1. Simple RMS-based Detection (`pattern_verification_utils.rs:54-82`)
```rust
// Window: 10ms (441 samples at 44.1kHz)
// Hop: 2.5ms (25% overlap)
// Method: RMS difference with 0.9 decay factor
let onset_strength = (rms - prev_rms).max(0.0);
if onset_strength > threshold { /* onset detected */ }
prev_rms = rms * 0.9;  // Decay for next comparison
```

**Pros:** Simple, fast, low latency
**Cons:** Susceptible to vibrato/tremolo false positives, poor for soft onsets

### 2. Energy Envelope Detection (`audio_verification_enhanced.rs:180-220`)
```rust
// Window: 5ms
// Min distance: 40ms between onsets
// Threshold: 2% of max energy
// Method: Derivative-based (must be 10% higher than previous)
let is_increasing = energy > prev_energy * 1.1;
```

**Pros:** Better at avoiding sustained note false positives
**Cons:** Fixed threshold relative to max (fails on dynamic range), no spectral information

## State-of-the-Art Algorithms

### 1. Spectral Flux (SF)

The most widely used onset detection function.

**Formula:**
```
SF(t) = sum_k max(0, |X(t,k)| - |X(t-1,k)|)
```
Where `X(t,k)` is magnitude at time `t`, frequency bin `k`.

**Recommended parameters:**
- Window: 1024-2048 samples
- Hop: 256-512 samples (10-23ms at 44.1kHz)
- Log compression: `Y = log(1 + gamma * |X|)` where gamma = 100
- Half-wave rectification: Keep only positive differences

**Use case:** General purpose, works well for most music

**Sources:**
- [librosa onset_strength documentation](https://librosa.org/doc/main/generated/librosa.onset.onset_strength.html)
- [Spectral-Based Novelty - FMP](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C6/C6S1_NoveltySpectral.html)

### 2. SuperFlux (Maximum Filter Vibrato Suppression)

Enhanced spectral flux that suppresses vibrato false positives.

**Key innovation:** Uses a maximum filter across frequency bins before computing flux:
```
max_spec = maximum_filter(spec, size=[1, max_bins])
diff_spec = spec[t] - max_spec[t-diff_frames]
```

**Parameters (from reference implementation):**
- Frame size: 2048 samples
- FPS: 200 (hop = 220 samples at 44.1kHz)
- Filterbank: 24 bands/octave (quarter-tone resolution)
- Frequency range: 30-17000 Hz
- Max bins: 3 (for maximum filter)
- Diff frames: 1 (compare to previous frame)
- Peak picking: threshold=1.1, combine=30ms, pre_avg=150ms, pre_max=10ms, post_max=50ms

**Use case:** Music with vibrato (vocals, strings, wind instruments)

**Source:** [Böck & Widmer 2013 - Maximum Filter Vibrato Suppression](https://github.com/CPJKU/SuperFlux)

### 3. High Frequency Content (HFC)

Emphasizes percussive transients by weighting bins by frequency.

**Formula:**
```
HFC(t) = sum_k (k * |X(t,k)|^2)
```
Where `k` is the bin index (frequency weighting).

**Use case:** Percussive instruments (drums, plucked strings)
**Limitation:** Poor for low-frequency or sustained onsets

**Source:** [aubio-rs OnsetMode::Hfc](https://docs.rs/aubio-rs/latest/aubio_rs/enum.OnsetMode.html)

### 4. Complex Domain

Uses both magnitude AND phase information.

**Formula:**
```
CD(t) = sum_k ||X(t,k)| - |X_predicted(t,k)| * exp(i*phi_predicted)|
```
Where predicted values come from linear phase extrapolation.

**Use case:** Pitched/tonal instruments, soft onsets
**Limitation:** Higher computational cost

**Sources:**
- [madmom complex domain](https://madmom.readthedocs.io/en/v0.16/modules/features/onsets.html)
- [aubio-rs OnsetMode::Complex](https://docs.rs/aubio-rs/latest/aubio_rs/enum.OnsetMode.html)

### 5. Phase Deviation / Weighted Phase Deviation

Detects onsets from phase discontinuities.

**Principle:** During steady-state, phase evolves linearly. Onsets cause phase discontinuities.

**Use case:** Pitched instruments where energy-based methods fail
**Limitation:** Sensitive to noise, requires good SNR

### 6. Deep Learning Approaches (CNN/RNN)

Modern state-of-the-art uses neural networks.

**Approaches:**
- CNNs on log-Mel spectrograms
- Bidirectional RNNs/LSTMs
- CRNNs (combined CNN + RNN)

**Performance:** Best F1 scores, but requires trained models
**Limitation:** Computational cost, model dependencies

**Sources:**
- [Bidirectional RNN for String Instruments](https://dl.acm.org/doi/fullHtml/10.1145/3616195.3616206)
- [madmom RNNOnsetProcessor](https://madmom.readthedocs.io/en/v0.16/modules/features/onsets.html)

## Algorithm Comparison

| Algorithm | Percussive | Tonal | Vibrato | Speed | Complexity |
|-----------|------------|-------|---------|-------|------------|
| Energy (current) | OK | Poor | Poor | Fast | Very Low |
| RMS Diff (current) | OK | Poor | Poor | Fast | Very Low |
| Spectral Flux | Good | Good | Fair | Fast | Low |
| SuperFlux | Good | Good | **Best** | Medium | Medium |
| HFC | **Best** | Poor | Poor | Fast | Low |
| Complex Domain | Fair | **Best** | Good | Slow | High |
| Phase Deviation | Fair | Good | Fair | Medium | Medium |
| CNN/RNN | **Best** | **Best** | **Best** | Slow | Very High |

## Recommendations for Phonon

### Immediate Improvement (Low effort, high impact)

**Implement Spectral Flux** to replace RMS-based detection:

```rust
fn spectral_flux_onset_detection(
    audio: &[f32],
    sample_rate: f32,
    fft_size: usize,      // 2048
    hop_size: usize,      // 512
    gamma: f32,           // 100.0 for log compression
    threshold: f32,       // 1.0-1.5
) -> Vec<f64> {
    // 1. Compute STFT with Hann window
    // 2. Apply log compression: log(1 + gamma * |X|)
    // 3. Compute positive differences: max(0, X[t] - X[t-1])
    // 4. Sum across frequency bins
    // 5. Peak pick with adaptive threshold
}
```

**Why:** Uses existing `spectrum-analyzer` + `rustfft` dependencies, significant accuracy improvement.

### Medium-term Enhancement (Medium effort)

**Add SuperFlux variant** for music with vibrato:

1. Add maximum filter across frequency bins
2. Use filterbank with 24 bands/octave
3. Implement proper peak picking with temporal smoothing

### Optional: Add aubio-rs

The `aubio-rs` crate provides:
- 9 onset detection modes
- Well-tested C library bindings
- Would need to add as optional dependency

**Pros:** Battle-tested, comprehensive
**Cons:** External C dependency, larger binary

### Testing Considerations

For Phonon's test verification:

1. **Percussive samples (bd, sn, hh):** HFC or standard Spectral Flux
2. **Synthesizer patterns (saw, sine):** Complex Domain or Spectral Flux
3. **Modulated signals (LFO):** SuperFlux (vibrato suppression)

## Implementation Priority

1. **P0:** Spectral flux with log compression (replaces current)
2. **P1:** HFC mode for drum tests (simple addition)
3. **P2:** SuperFlux for vibrato-heavy tests
4. **P3:** Consider aubio-rs for comprehensive solution

## References

- [librosa Onset Detection](https://librosa.org/doc/latest/onset.html)
- [madmom Onset Features](https://madmom.readthedocs.io/en/v0.16/modules/features/onsets.html)
- [aubio-rs Documentation](https://docs.rs/aubio-rs/latest/aubio_rs/enum.OnsetMode.html)
- [SuperFlux Reference Implementation](https://github.com/CPJKU/SuperFlux)
- [NINOS2: Spectral Sparsity Method](https://asmp-eurasipjournals.springeropen.com/articles/10.1186/s13636-021-00214-7)
- [Onset Detection Tutorial (Bello et al.)](http://www.iro.umontreal.ca/~pift6080/H09/documents/papers/bello_onset_tutorial.pdf)
