# RMS Envelope Comparison Techniques for Audio Testing

## Executive Summary

This document explores RMS (Root Mean Square) envelope comparison techniques for verifying audio output in automated testing. The goal is to enable robust, deterministic testing of audio signals without relying on subjective listening tests.

## 1. Background: What is RMS Envelope?

### 1.1 Definition

RMS (Root Mean Square) measures the average power of a signal over time:

```
RMS = sqrt( (1/N) * Σ(x[i]²) )
```

An **RMS envelope** is the RMS value computed over sliding windows, producing a time-varying amplitude curve that represents the signal's energy evolution.

### 1.2 RMS vs Peak Envelope

| Feature | RMS Envelope | Peak/Amplitude Envelope |
|---------|-------------|------------------------|
| Sensitivity to outliers | Low (smoothed) | High (follows peaks) |
| Perceptual correlation | Better matches perceived loudness | Matches waveform contour |
| Computation | More expensive (squares + sqrt) | Cheaper (max) |
| Use cases | Level metering, compression, testing | Onset detection, transient analysis |

The RMS envelope is preferred for audio testing because:
1. It's less sensitive to phase variations
2. It correlates with perceived loudness
3. It's more stable for automated comparison

## 2. Comparison Techniques

### 2.1 Point-to-Point Comparison with Tolerance

The simplest approach: compute RMS envelopes for both reference and test signals, then compare point-by-point within a tolerance band.

```rust
fn compare_rms_envelopes(
    reference: &[f32],
    test: &[f32],
    tolerance_db: f32,
) -> bool {
    let tolerance_linear = 10.0_f32.powf(tolerance_db / 20.0);

    for (r, t) in reference.iter().zip(test.iter()) {
        let ratio = if *r > 0.0 { t / r } else { 1.0 };
        if ratio < (1.0 / tolerance_linear) || ratio > tolerance_linear {
            return false;
        }
    }
    true
}
```

**Typical tolerances:**
- Bit-exact reproduction: 0 dB
- Perceptually identical: ±0.5 dB
- Similar loudness: ±1-2 dB
- Rough match: ±3-6 dB

### 2.2 Cross-Correlation

Measures similarity between signals as a function of time shift. Useful when signals may be slightly offset in time.

```rust
fn cross_correlate_envelopes(env1: &[f32], env2: &[f32]) -> f32 {
    let n = env1.len().min(env2.len());

    let mean1: f32 = env1.iter().sum::<f32>() / n as f32;
    let mean2: f32 = env2.iter().sum::<f32>() / n as f32;

    let mut cov = 0.0;
    let mut var1 = 0.0;
    let mut var2 = 0.0;

    for i in 0..n {
        let d1 = env1[i] - mean1;
        let d2 = env2[i] - mean2;
        cov += d1 * d2;
        var1 += d1 * d1;
        var2 += d2 * d2;
    }

    if var1 > 0.0 && var2 > 0.0 {
        cov / (var1.sqrt() * var2.sqrt())
    } else {
        0.0
    }
}
```

**Interpretation:**
- 1.0 = Perfect positive correlation
- 0.0 = No correlation
- -1.0 = Perfect negative correlation (inverted)

For audio testing, correlation > 0.95 typically indicates a good match.

### 2.3 Dynamic Time Warping (DTW)

DTW finds optimal alignment between sequences of different lengths or timing. Useful when:
- Timing variations are acceptable
- Testing musical transformations (tempo changes, time-stretching)

```rust
fn dtw_distance(env1: &[f32], env2: &[f32]) -> f32 {
    let n = env1.len();
    let m = env2.len();
    let mut dtw = vec![vec![f32::INFINITY; m + 1]; n + 1];
    dtw[0][0] = 0.0;

    for i in 1..=n {
        for j in 1..=m {
            let cost = (env1[i-1] - env2[j-1]).abs();
            dtw[i][j] = cost + dtw[i-1][j].min(dtw[i][j-1]).min(dtw[i-1][j-1]);
        }
    }

    dtw[n][m] / (n + m) as f32  // Normalized
}
```

### 2.4 Statistical Comparison

Compare statistical properties rather than exact values:

```rust
struct EnvelopeStats {
    mean: f32,
    std_dev: f32,
    min: f32,
    max: f32,
    percentile_25: f32,
    percentile_75: f32,
}

fn compare_stats(ref_stats: &EnvelopeStats, test_stats: &EnvelopeStats, tolerance: f32) -> bool {
    let mean_match = (ref_stats.mean - test_stats.mean).abs() < tolerance * ref_stats.mean;
    let stddev_match = (ref_stats.std_dev - test_stats.std_dev).abs() < tolerance * ref_stats.std_dev;
    mean_match && stddev_match
}
```

## 3. RMS Window Size Considerations

### 3.1 Window Size vs. Frequency Response

| Window Size | Frequency Response | Use Case |
|-------------|-------------------|----------|
| 1-5 ms | Fast, ripply at low freq | Transient detection |
| 10-50 ms | Good balance | General testing |
| 100-300 ms | Smooth, slow response | VU metering, loudness |
| 300-400 ms | Very smooth | EBU R128 momentary |

### 3.2 Cascaded Running Sums

For smooth response without latency, cascade multiple running sums:

```rust
struct CascadedRMS {
    stages: Vec<RunningSumStage>,
}

impl CascadedRMS {
    fn new(sample_rate: f32, base_window_ms: f32) -> Self {
        // 5 stages spaced within an octave for optimal smoothing
        let ratios = [1.0, 1.149, 1.32, 1.516, 1.741];
        let stages = ratios.iter()
            .map(|r| RunningSumStage::new(sample_rate, base_window_ms * r))
            .collect();
        Self { stages }
    }
}
```

## 4. Testing Methodology

### 4.1 Three-Level Audio Testing (Phonon Standard)

**Level 1: Pattern Query Verification**
- Test pattern logic without rendering audio
- Fast, deterministic, exact
- Catches logic bugs early

**Level 2: Onset/Event Detection**
- Verify events occur at correct times
- Uses RMS envelope for onset detection
- Catches timing and trigger issues

**Level 3: Audio Characteristics (RMS Comparison)**
- Compare overall signal characteristics
- RMS, peak, spectral centroid
- Catches synthesis and DSP issues

### 4.2 Golden Reference Testing

Store known-good reference outputs:

```rust
#[test]
fn test_against_golden_reference() {
    let test_output = render_dsl("sine 440", 1.0);
    let golden = load_golden_reference("sine_440_1sec.wav");

    let test_env = compute_rms_envelope(&test_output, 512, 128);
    let golden_env = compute_rms_envelope(&golden, 512, 128);

    let correlation = cross_correlate_envelopes(&test_env, &golden_env);
    assert!(correlation > 0.99, "Output differs from golden reference");
}
```

### 4.3 Differential Testing

Compare two implementations:

```rust
#[test]
fn test_fast_vs_slow_implementation() {
    let input = generate_test_signal();

    let output_fast = process_fast(&input);
    let output_slow = process_slow(&input);

    let env_fast = compute_rms_envelope(&output_fast, 256, 64);
    let env_slow = compute_rms_envelope(&output_slow, 256, 64);

    for (f, s) in env_fast.iter().zip(env_slow.iter()) {
        assert!((f - s).abs() < 0.001, "Implementations differ");
    }
}
```

## 5. Implementation Recommendations for Phonon

### 5.1 Proposed RMS Envelope Comparison Module

```rust
pub mod audio_testing {
    /// Compute RMS envelope with specified window and hop size
    pub fn compute_rms_envelope(
        audio: &[f32],
        window_size: usize,
        hop_size: usize,
    ) -> Vec<f32> {
        audio.windows(window_size)
            .step_by(hop_size)
            .map(|window| {
                let sum_sq: f32 = window.iter().map(|x| x * x).sum();
                (sum_sq / window.len() as f32).sqrt()
            })
            .collect()
    }

    /// Compare envelopes with tolerance band
    pub fn envelopes_match(
        reference: &[f32],
        test: &[f32],
        tolerance_db: f32,
    ) -> EnvelopeMatchResult {
        let tolerance = 10.0_f32.powf(tolerance_db / 20.0);
        let mut max_deviation_db = 0.0_f32;
        let mut mismatch_indices = Vec::new();

        for (i, (r, t)) in reference.iter().zip(test.iter()).enumerate() {
            let deviation_db = if *r > 1e-6 {
                20.0 * (t / r).log10()
            } else if *t > 1e-6 {
                f32::INFINITY
            } else {
                0.0
            };

            max_deviation_db = max_deviation_db.max(deviation_db.abs());

            if deviation_db.abs() > tolerance_db {
                mismatch_indices.push(i);
            }
        }

        EnvelopeMatchResult {
            matches: mismatch_indices.is_empty(),
            max_deviation_db,
            mismatch_count: mismatch_indices.len(),
            mismatch_indices,
        }
    }

    pub struct EnvelopeMatchResult {
        pub matches: bool,
        pub max_deviation_db: f32,
        pub mismatch_count: usize,
        pub mismatch_indices: Vec<usize>,
    }
}
```

### 5.2 Recommended Test Patterns

1. **Steady-state tests**: Compare RMS of held notes/drones
2. **Transient tests**: Verify attack/decay envelope shapes
3. **Pattern timing tests**: Verify event timing via onset detection
4. **Modulation tests**: Verify LFO affects filter/amplitude correctly
5. **Regression tests**: Compare against golden references

### 5.3 Suggested Tolerances for Phonon

| Test Type | Suggested Tolerance |
|-----------|-------------------|
| Deterministic synthesis | 0.1 dB |
| Sample playback | 0.5 dB |
| Effects (reverb, etc.) | 1.0 dB |
| Pattern timing | ±5 ms |
| Modulation depth | 10% |

## 6. Existing Rust Crates

### 6.1 Golden Testing
- **goldentests**: General-purpose golden file testing
- **gilder**: Assertion library with golden file support

### 6.2 Audio Analysis
- **spectrum-analyzer**: FFT-based spectral analysis
- **audio-processor-analysis**: Envelope follower, FFT, transient detection
- **audio-processor-testing-helpers**: Test utilities for audio processors

### 6.3 DTW
- **dtw**: Dynamic time warping implementation

## 7. References

### Academic/Technical
- Julius O. Smith III, "Physical Audio Signal Processing"
- Will Pirkle, "Designing Audio Effect Plugins in C++"
- EBU R128, "Loudness normalisation and permitted maximum level"

### Online Resources
- [RMS Energy vs Amplitude Envelope](https://www.analyticsvidhya.com/blog/2022/05/comparison-of-the-rms-energy-and-the-amplitude-envelope/)
- [Dynamic Time Warping Explained](https://www.databricks.com/blog/2019/04/30/understanding-dynamic-time-warping.html)
- [Cross-Correlation Techniques](https://www.numberanalytics.com/blog/cross-correlation-techniques-signal-analysis)
- [KVR: Real-time RMS Best Practices](https://www.kvraudio.com/forum/viewtopic.php?t=460756)
- [KVR: IIR RMS Envelope Detector](https://www.kvraudio.com/forum/viewtopic.php?t=536648)
- [librosa RMS Documentation](https://librosa.org/doc/main/generated/librosa.feature.rms.html)
- [Golden Master Testing in Rust](https://blog.anp.lol/rust/2017/08/18/golden-master-regression-in-rust/)

## 8. Conclusion

For Phonon's audio testing needs, the recommended approach is:

1. **Use RMS envelopes** (not peak) for comparing audio outputs
2. **Combine multiple techniques**: point-to-point comparison for deterministic tests, correlation for timing-tolerant tests
3. **Choose appropriate window sizes**: 10-50ms for general testing
4. **Set realistic tolerances**: ±0.5 dB for synthesis, ±1 dB for effects
5. **Implement golden reference testing** for regression detection
6. **Use statistical comparison** as a fallback when exact matching isn't possible

The existing `pattern_verification_utils.rs` provides a solid foundation. Consider extending it with:
- Dedicated `EnvelopeComparison` module
- Golden reference file support
- DTW for time-warped comparisons
- Detailed mismatch reporting for debugging
