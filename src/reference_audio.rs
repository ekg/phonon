//! Reference Audio Comparison Module
//!
//! Provides tools for comparing audio signals using RMS envelope analysis.
//! Used for golden reference testing and regression detection in audio output.
//!
//! # Key Features
//!
//! - **RMS Envelope Computation**: Calculate time-varying amplitude envelopes
//! - **Point-to-Point Comparison**: Compare envelopes with tolerance bands
//! - **Cross-Correlation**: Time-shift tolerant similarity measurement
//! - **Golden Reference Testing**: Load/save reference files for regression testing
//! - **Detailed Reporting**: Comprehensive mismatch analysis for debugging
//!
//! # Example
//!
//! ```ignore
//! use phonon::reference_audio::{compute_rms_envelope, compare_envelopes, ComparisonConfig};
//!
//! let reference = render_dsl("s \"bd sn\"", 2.0);
//! let test = render_dsl("s \"bd sn\"", 2.0);
//!
//! let ref_env = compute_rms_envelope(&reference, 512, 128);
//! let test_env = compute_rms_envelope(&test, 512, 128);
//!
//! let result = compare_envelopes(&ref_env, &test_env, &ComparisonConfig::default());
//! assert!(result.matches, "Audio differs from reference: {}", result.summary());
//! ```

use std::path::Path;

/// Configuration for envelope comparison
#[derive(Debug, Clone)]
pub struct ComparisonConfig {
    /// Tolerance in dB for point-to-point comparison (default: 0.5 dB)
    pub tolerance_db: f32,
    /// Minimum correlation coefficient for cross-correlation test (default: 0.95)
    pub min_correlation: f32,
    /// Window size in samples for RMS computation (default: 512)
    pub window_size: usize,
    /// Hop size in samples for RMS computation (default: 128)
    pub hop_size: usize,
    /// Sample rate for time calculations (default: 44100.0)
    pub sample_rate: f32,
    /// Maximum allowed time offset in seconds for correlation (default: 0.05)
    pub max_time_offset_secs: f32,
    /// Silence threshold for envelope values (default: 1e-6)
    pub silence_threshold: f32,
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            tolerance_db: 0.5,
            min_correlation: 0.95,
            window_size: 512,
            hop_size: 128,
            sample_rate: 44100.0,
            max_time_offset_secs: 0.05,
            silence_threshold: 1e-6,
        }
    }
}

impl ComparisonConfig {
    /// Create config for deterministic synthesis (tight tolerances)
    pub fn for_synthesis() -> Self {
        Self {
            tolerance_db: 0.1,
            min_correlation: 0.99,
            ..Default::default()
        }
    }

    /// Create config for sample playback (moderate tolerances)
    pub fn for_samples() -> Self {
        Self {
            tolerance_db: 0.5,
            min_correlation: 0.95,
            ..Default::default()
        }
    }

    /// Create config for effects (relaxed tolerances)
    pub fn for_effects() -> Self {
        Self {
            tolerance_db: 1.0,
            min_correlation: 0.90,
            ..Default::default()
        }
    }

    /// Create config for rough matching (very relaxed)
    pub fn rough() -> Self {
        Self {
            tolerance_db: 3.0,
            min_correlation: 0.80,
            ..Default::default()
        }
    }
}

/// Result of envelope comparison
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Whether the comparison passed all tests
    pub matches: bool,
    /// Maximum deviation in dB observed
    pub max_deviation_db: f32,
    /// Number of envelope points that exceeded tolerance
    pub mismatch_count: usize,
    /// Indices of mismatched points
    pub mismatch_indices: Vec<usize>,
    /// Cross-correlation coefficient
    pub correlation: f32,
    /// Best time offset found by cross-correlation (in samples)
    pub best_offset_samples: i32,
    /// RMS of reference envelope
    pub reference_rms: f32,
    /// RMS of test envelope
    pub test_rms: f32,
    /// Length of reference envelope
    pub reference_length: usize,
    /// Length of test envelope
    pub test_length: usize,
    /// Detailed mismatch info for debugging
    pub mismatch_details: Vec<MismatchDetail>,
}

impl ComparisonResult {
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        if self.matches {
            format!(
                "MATCH: max deviation {:.2} dB, correlation {:.4}",
                self.max_deviation_db, self.correlation
            )
        } else {
            let reasons: Vec<String> = vec![
                format!("max deviation: {:.2} dB", self.max_deviation_db),
                format!("correlation: {:.4}", self.correlation),
                format!("{} points out of tolerance", self.mismatch_count),
            ];
            format!("MISMATCH: {}", reasons.join(", "))
        }
    }

    /// Generate detailed report for debugging
    pub fn detailed_report(&self, sample_rate: f32, hop_size: usize) -> String {
        let mut report = String::new();

        report.push_str(&format!(
            "=== Envelope Comparison Report ===\n\
             Result: {}\n\
             Reference length: {} points\n\
             Test length: {} points\n\
             Reference RMS: {:.6}\n\
             Test RMS: {:.6}\n\
             Max deviation: {:.2} dB\n\
             Correlation: {:.4}\n\
             Best offset: {} samples ({:.3} ms)\n\
             Mismatched points: {}\n",
            if self.matches { "PASS" } else { "FAIL" },
            self.reference_length,
            self.test_length,
            self.reference_rms,
            self.test_rms,
            self.max_deviation_db,
            self.correlation,
            self.best_offset_samples,
            self.best_offset_samples as f32 * hop_size as f32 / sample_rate * 1000.0,
            self.mismatch_count
        ));

        if !self.mismatch_details.is_empty() {
            report.push_str("\nTop mismatches:\n");
            for (i, detail) in self.mismatch_details.iter().take(10).enumerate() {
                let time_ms = detail.index as f32 * hop_size as f32 / sample_rate * 1000.0;
                report.push_str(&format!(
                    "  {}. index={} ({:.1}ms): ref={:.6}, test={:.6}, dev={:.2}dB\n",
                    i + 1,
                    detail.index,
                    time_ms,
                    detail.reference_value,
                    detail.test_value,
                    detail.deviation_db
                ));
            }
        }

        report
    }
}

/// Detailed information about a single mismatch point
#[derive(Debug, Clone)]
pub struct MismatchDetail {
    /// Index in the envelope
    pub index: usize,
    /// Reference envelope value
    pub reference_value: f32,
    /// Test envelope value
    pub test_value: f32,
    /// Deviation in dB
    pub deviation_db: f32,
}

/// Compute RMS envelope with specified window and hop size
///
/// # Arguments
/// * `audio` - Audio samples to analyze
/// * `window_size` - Size of RMS computation window in samples
/// * `hop_size` - Step size between windows in samples
///
/// # Returns
/// Vector of RMS values for each window position
///
/// # Example
/// ```ignore
/// let audio = render_audio();
/// let envelope = compute_rms_envelope(&audio, 512, 128);
/// ```
pub fn compute_rms_envelope(audio: &[f32], window_size: usize, hop_size: usize) -> Vec<f32> {
    if audio.len() < window_size {
        // Return single RMS value for short audio
        if audio.is_empty() {
            return vec![];
        }
        let rms = (audio.iter().map(|&x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
        return vec![rms];
    }

    audio
        .windows(window_size)
        .step_by(hop_size)
        .map(|window| {
            let sum_sq: f32 = window.iter().map(|x| x * x).sum();
            (sum_sq / window.len() as f32).sqrt()
        })
        .collect()
}

/// Compute RMS envelope in dB scale
pub fn compute_rms_envelope_db(audio: &[f32], window_size: usize, hop_size: usize) -> Vec<f32> {
    compute_rms_envelope(audio, window_size, hop_size)
        .into_iter()
        .map(|rms| {
            if rms > 0.0 {
                20.0 * rms.log10()
            } else {
                -120.0 // Floor at -120 dB
            }
        })
        .collect()
}

/// Compare two envelopes with configurable tolerance
///
/// Performs both point-to-point comparison and cross-correlation analysis.
///
/// # Arguments
/// * `reference` - Reference envelope (expected/golden)
/// * `test` - Test envelope (actual output)
/// * `config` - Comparison configuration
///
/// # Returns
/// Detailed comparison result
pub fn compare_envelopes(
    reference: &[f32],
    test: &[f32],
    config: &ComparisonConfig,
) -> ComparisonResult {
    let mut max_deviation_db: f32 = 0.0;
    let mut mismatch_indices = Vec::new();
    let mut mismatch_details = Vec::new();

    // Calculate RMS of envelopes for statistics
    let reference_rms = if !reference.is_empty() {
        (reference.iter().map(|&x| x * x).sum::<f32>() / reference.len() as f32).sqrt()
    } else {
        0.0
    };

    let test_rms = if !test.is_empty() {
        (test.iter().map(|&x| x * x).sum::<f32>() / test.len() as f32).sqrt()
    } else {
        0.0
    };

    // Point-to-point comparison
    let min_len = reference.len().min(test.len());
    for i in 0..min_len {
        let r = reference[i];
        let t = test[i];

        let deviation_db = if r > config.silence_threshold {
            20.0 * (t / r).log10()
        } else if t > config.silence_threshold {
            // Reference is silent but test is not
            f32::INFINITY
        } else {
            // Both silent
            0.0
        };

        let abs_deviation = deviation_db.abs();
        if abs_deviation > max_deviation_db && abs_deviation.is_finite() {
            max_deviation_db = abs_deviation;
        }

        if abs_deviation > config.tolerance_db {
            mismatch_indices.push(i);
            mismatch_details.push(MismatchDetail {
                index: i,
                reference_value: r,
                test_value: t,
                deviation_db,
            });
        }
    }

    // Sort mismatch details by absolute deviation (largest first)
    mismatch_details.sort_by(|a, b| {
        b.deviation_db
            .abs()
            .partial_cmp(&a.deviation_db.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Cross-correlation analysis
    let max_offset_samples =
        (config.max_time_offset_secs * config.sample_rate / config.hop_size as f32) as i32;
    let (correlation, best_offset) =
        cross_correlate_with_offset(reference, test, max_offset_samples);

    // Determine if comparison passes
    let point_comparison_passes = mismatch_indices.is_empty()
        || (mismatch_indices.len() as f32 / (min_len.max(1) as f32)) < 0.05; // Allow 5% outliers

    let correlation_passes = correlation >= config.min_correlation;

    let length_similar =
        (reference.len() as f32 - test.len() as f32).abs() / (reference.len().max(1) as f32) < 0.1; // Within 10%

    // RMS energy ratio check: catch amplitude differences that correlation misses
    // (Pearson correlation is shape-only, ignoring scale)
    let energy_ratio_db = if reference_rms > 0.0 && test_rms > 0.0 {
        20.0 * (test_rms / reference_rms).log10()
    } else if reference_rms > 0.0 || test_rms > 0.0 {
        f32::INFINITY
    } else {
        0.0
    };
    let energy_similar = energy_ratio_db.abs() <= config.tolerance_db;

    let matches = point_comparison_passes && correlation_passes && length_similar && energy_similar;

    ComparisonResult {
        matches,
        max_deviation_db,
        mismatch_count: mismatch_indices.len(),
        mismatch_indices,
        correlation,
        best_offset_samples: best_offset,
        reference_rms,
        test_rms,
        reference_length: reference.len(),
        test_length: test.len(),
        mismatch_details,
    }
}

/// Calculate cross-correlation between two envelopes
///
/// Returns Pearson correlation coefficient (0-1 for similar signals)
pub fn cross_correlate(env1: &[f32], env2: &[f32]) -> f32 {
    if env1.is_empty() || env2.is_empty() {
        return 0.0;
    }

    let n = env1.len().min(env2.len());

    let mean1: f32 = env1.iter().take(n).sum::<f32>() / n as f32;
    let mean2: f32 = env2.iter().take(n).sum::<f32>() / n as f32;

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

/// Calculate cross-correlation with time offset search
///
/// Finds the best alignment between envelopes within the specified offset range.
///
/// # Returns
/// (best_correlation, best_offset_in_samples)
pub fn cross_correlate_with_offset(reference: &[f32], test: &[f32], max_offset: i32) -> (f32, i32) {
    if reference.is_empty() || test.is_empty() {
        return (0.0, 0);
    }

    let mut best_correlation = cross_correlate(reference, test);
    let mut best_offset: i32 = 0;

    // Try positive offsets (test is delayed relative to reference)
    for offset in 1..=max_offset {
        let offset_usize = offset as usize;
        if offset_usize >= test.len() {
            break;
        }

        let corr = cross_correlate(reference, &test[offset_usize..]);
        if corr > best_correlation {
            best_correlation = corr;
            best_offset = offset;
        }
    }

    // Try negative offsets (reference is delayed relative to test)
    for offset in 1..=max_offset {
        let offset_usize = offset as usize;
        if offset_usize >= reference.len() {
            break;
        }

        let corr = cross_correlate(&reference[offset_usize..], test);
        if corr > best_correlation {
            best_correlation = corr;
            best_offset = -offset;
        }
    }

    (best_correlation, best_offset)
}

/// Dynamic Time Warping distance between envelopes
///
/// Useful for comparing signals with timing variations (tempo changes, etc.)
/// Returns normalized distance (lower = more similar)
pub fn dtw_distance(env1: &[f32], env2: &[f32]) -> f32 {
    if env1.is_empty() || env2.is_empty() {
        return f32::INFINITY;
    }

    let n = env1.len();
    let m = env2.len();

    // Use f64 internally for better numerical stability
    let mut dtw = vec![vec![f64::INFINITY; m + 1]; n + 1];
    dtw[0][0] = 0.0;

    for i in 1..=n {
        for j in 1..=m {
            let cost = ((env1[i - 1] - env2[j - 1]) as f64).abs();
            dtw[i][j] = cost + dtw[i - 1][j].min(dtw[i][j - 1]).min(dtw[i - 1][j - 1]);
        }
    }

    // Normalize by path length
    (dtw[n][m] / (n + m) as f64) as f32
}

/// Statistical summary of an envelope
#[derive(Debug, Clone, Default)]
pub struct EnvelopeStats {
    pub mean: f32,
    pub std_dev: f32,
    pub min: f32,
    pub max: f32,
    pub percentile_25: f32,
    pub percentile_75: f32,
    pub length: usize,
}

impl EnvelopeStats {
    /// Compute statistics for an envelope
    pub fn from_envelope(envelope: &[f32]) -> Self {
        if envelope.is_empty() {
            return Self::default();
        }

        let n = envelope.len() as f32;
        let mean: f32 = envelope.iter().sum::<f32>() / n;
        let variance: f32 = envelope.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / n;
        let std_dev = variance.sqrt();

        let min = envelope.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = envelope.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Sort for percentiles
        let mut sorted: Vec<f32> = envelope.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p25_idx = (envelope.len() as f32 * 0.25) as usize;
        let p75_idx = (envelope.len() as f32 * 0.75) as usize;

        Self {
            mean,
            std_dev,
            min,
            max,
            percentile_25: sorted.get(p25_idx).copied().unwrap_or(0.0),
            percentile_75: sorted.get(p75_idx).copied().unwrap_or(0.0),
            length: envelope.len(),
        }
    }

    /// Compare statistics with tolerance
    pub fn matches(&self, other: &EnvelopeStats, tolerance: f32) -> bool {
        let mean_match = if self.mean > 0.0 {
            ((self.mean - other.mean) / self.mean).abs() < tolerance
        } else {
            other.mean.abs() < tolerance
        };

        let stddev_match = if self.std_dev > 0.0 {
            ((self.std_dev - other.std_dev) / self.std_dev).abs() < tolerance
        } else {
            other.std_dev.abs() < tolerance
        };

        mean_match && stddev_match
    }
}

// ============================================================================
// Golden Reference File Support
// ============================================================================

/// Load audio from a WAV file
///
/// Returns mono samples and sample rate.
pub fn load_wav<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32), String> {
    let reader = hound::WavReader::open(path.as_ref())
        .map_err(|e| format!("Failed to open WAV file: {}", e))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|s| s.unwrap_or(0.0))
            .collect(),
        hound::SampleFormat::Int => {
            let max_val = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.unwrap_or(0) as f32 / max_val)
                .collect()
        }
    };

    // Mix to mono if multi-channel
    let mono_samples = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / spec.channels as f32)
            .collect()
    } else {
        samples
    };

    Ok((mono_samples, sample_rate))
}

/// Save audio to a WAV file
pub fn save_wav<P: AsRef<Path>>(path: P, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(path.as_ref(), spec)
        .map_err(|e| format!("Failed to create WAV file: {}", e))?;

    for &sample in samples {
        writer
            .write_sample(sample)
            .map_err(|e| format!("Failed to write sample: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

    Ok(())
}

/// Load envelope from a binary file (compact storage)
pub fn load_envelope<P: AsRef<Path>>(path: P) -> Result<Vec<f32>, String> {
    let bytes =
        std::fs::read(path.as_ref()).map_err(|e| format!("Failed to read envelope file: {}", e))?;

    if bytes.len() % 4 != 0 {
        return Err("Invalid envelope file: size not multiple of 4".to_string());
    }

    Ok(bytes
        .chunks(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect())
}

/// Save envelope to a binary file
pub fn save_envelope<P: AsRef<Path>>(path: P, envelope: &[f32]) -> Result<(), String> {
    let bytes: Vec<u8> = envelope.iter().flat_map(|&x| x.to_le_bytes()).collect();

    std::fs::write(path.as_ref(), bytes)
        .map_err(|e| format!("Failed to write envelope file: {}", e))
}

/// Compare audio against a golden reference WAV file
///
/// # Arguments
/// * `test_audio` - Audio samples to test
/// * `reference_path` - Path to golden reference WAV file
/// * `config` - Comparison configuration
///
/// # Returns
/// Comparison result
pub fn compare_against_reference<P: AsRef<Path>>(
    test_audio: &[f32],
    reference_path: P,
    config: &ComparisonConfig,
) -> Result<ComparisonResult, String> {
    let (reference_audio, _sample_rate) = load_wav(reference_path)?;

    let ref_envelope = compute_rms_envelope(&reference_audio, config.window_size, config.hop_size);
    let test_envelope = compute_rms_envelope(test_audio, config.window_size, config.hop_size);

    Ok(compare_envelopes(&ref_envelope, &test_envelope, config))
}

/// Create a golden reference from audio
///
/// Saves both the full WAV and a compact envelope file.
pub fn create_golden_reference<P: AsRef<Path>>(
    audio: &[f32],
    base_path: P,
    sample_rate: u32,
    config: &ComparisonConfig,
) -> Result<(), String> {
    let base = base_path.as_ref();

    // Save full WAV
    let wav_path = base.with_extension("wav");
    save_wav(&wav_path, audio, sample_rate)?;

    // Save envelope for quick comparison
    let envelope = compute_rms_envelope(audio, config.window_size, config.hop_size);
    let env_path = base.with_extension("env");
    save_envelope(&env_path, &envelope)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine(freq: f32, sample_rate: f32, duration: f32, amplitude: f32) -> Vec<f32> {
        let num_samples = (sample_rate * duration) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                amplitude * (2.0 * PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_compute_rms_envelope() {
        let audio = generate_sine(440.0, 44100.0, 1.0, 0.5);
        let envelope = compute_rms_envelope(&audio, 512, 128);

        // Should have multiple envelope points
        assert!(!envelope.is_empty());

        // RMS of sine wave should be amplitude / sqrt(2)
        let expected_rms = 0.5 / 2.0_f32.sqrt();
        let mean_rms: f32 = envelope.iter().sum::<f32>() / envelope.len() as f32;
        assert!(
            (mean_rms - expected_rms).abs() < 0.05,
            "Mean RMS {:.4} should be close to {:.4}",
            mean_rms,
            expected_rms
        );
    }

    #[test]
    fn test_compare_identical_envelopes() {
        let audio = generate_sine(440.0, 44100.0, 0.5, 0.5);
        let envelope = compute_rms_envelope(&audio, 512, 128);

        let config = ComparisonConfig::default();
        let result = compare_envelopes(&envelope, &envelope, &config);

        assert!(result.matches, "Identical envelopes should match");
        assert_eq!(result.mismatch_count, 0);
        assert!(
            result.correlation > 0.99,
            "Correlation should be ~1.0 for identical signals"
        );
    }

    #[test]
    fn test_compare_similar_envelopes() {
        let audio1 = generate_sine(440.0, 44100.0, 0.5, 0.5);
        let audio2 = generate_sine(440.0, 44100.0, 0.5, 0.48); // Slightly quieter

        let env1 = compute_rms_envelope(&audio1, 512, 128);
        let env2 = compute_rms_envelope(&audio2, 512, 128);

        let config = ComparisonConfig::for_samples();
        let result = compare_envelopes(&env1, &env2, &config);

        assert!(
            result.matches,
            "Similar envelopes should match with sample tolerance"
        );
    }

    #[test]
    fn test_compare_different_envelopes() {
        let audio1 = generate_sine(440.0, 44100.0, 0.5, 0.5);
        let audio2 = generate_sine(440.0, 44100.0, 0.5, 0.1); // Much quieter

        let env1 = compute_rms_envelope(&audio1, 512, 128);
        let env2 = compute_rms_envelope(&audio2, 512, 128);

        let config = ComparisonConfig::for_synthesis();
        let result = compare_envelopes(&env1, &env2, &config);

        assert!(
            !result.matches,
            "Very different envelopes should not match: {}",
            result.summary()
        );
    }

    #[test]
    fn test_cross_correlation() {
        let audio = generate_sine(440.0, 44100.0, 0.5, 0.5);
        let envelope = compute_rms_envelope(&audio, 512, 128);

        let corr = cross_correlate(&envelope, &envelope);
        assert!(
            (corr - 1.0).abs() < 0.001,
            "Self-correlation should be 1.0, got {}",
            corr
        );
    }

    #[test]
    fn test_cross_correlation_with_offset() {
        let audio = generate_sine(440.0, 44100.0, 0.5, 0.5);
        let envelope = compute_rms_envelope(&audio, 512, 128);

        // Create shifted version
        let mut shifted = vec![0.0; 5];
        shifted.extend(envelope.iter());

        let (corr, offset) = cross_correlate_with_offset(&envelope, &shifted, 10);
        assert!(corr > 0.95, "Should find good correlation with offset");
        assert!(offset > 0, "Should detect positive offset");
    }

    #[test]
    fn test_dtw_distance() {
        let audio1 = generate_sine(440.0, 44100.0, 0.5, 0.5);
        let audio2 = generate_sine(440.0, 44100.0, 0.5, 0.5);

        let env1 = compute_rms_envelope(&audio1, 512, 128);
        let env2 = compute_rms_envelope(&audio2, 512, 128);

        let dist = dtw_distance(&env1, &env2);
        assert!(
            dist < 0.001,
            "DTW distance for identical signals should be ~0"
        );
    }

    #[test]
    fn test_envelope_stats() {
        let audio = generate_sine(440.0, 44100.0, 1.0, 0.5);
        let envelope = compute_rms_envelope(&audio, 512, 128);
        let stats = EnvelopeStats::from_envelope(&envelope);

        assert!(stats.mean > 0.0);
        assert!(stats.std_dev >= 0.0);
        assert!(stats.min <= stats.mean);
        assert!(stats.max >= stats.mean);
        assert!(stats.percentile_25 <= stats.percentile_75);
    }

    #[test]
    fn test_comparison_result_summary() {
        let result = ComparisonResult {
            matches: true,
            max_deviation_db: 0.3,
            mismatch_count: 0,
            mismatch_indices: vec![],
            correlation: 0.98,
            best_offset_samples: 0,
            reference_rms: 0.35,
            test_rms: 0.34,
            reference_length: 100,
            test_length: 100,
            mismatch_details: vec![],
        };

        let summary = result.summary();
        assert!(summary.contains("MATCH"));
    }

    #[test]
    fn test_empty_envelope_handling() {
        let empty: Vec<f32> = vec![];
        let non_empty = vec![0.5; 100];

        let config = ComparisonConfig::default();

        // Empty vs empty should handle gracefully
        let result = compare_envelopes(&empty, &empty, &config);
        assert!(result.reference_length == 0);

        // Empty vs non-empty
        let result2 = compare_envelopes(&empty, &non_empty, &config);
        assert!(!result2.matches);
    }

    #[test]
    fn test_different_config_presets() {
        // Just verify presets create valid configs
        let synthesis = ComparisonConfig::for_synthesis();
        assert!(synthesis.tolerance_db < 0.5);

        let samples = ComparisonConfig::for_samples();
        assert!(samples.min_correlation >= 0.9);

        let effects = ComparisonConfig::for_effects();
        assert!(effects.tolerance_db >= 1.0);

        let rough = ComparisonConfig::rough();
        assert!(rough.tolerance_db >= 2.0);
    }
}
