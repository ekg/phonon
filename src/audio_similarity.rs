//! Audio Similarity Scoring System
//!
//! Provides robust audio comparison and similarity metrics for pattern validation.
//! This module implements techniques from audio fingerprinting research, adapted
//! for Phonon's pattern verification needs.
//!
//! # Key Features
//!
//! - **Spectral Flux Onset Detection**: More accurate than energy-based detection
//! - **Rhythm Pattern Comparison**: Compare inter-onset interval patterns
//! - **Chroma Features**: Verify melodic/harmonic content
//! - **Multi-dimensional Similarity**: Combine multiple metrics
//!
//! # Usage
//!
//! ```ignore
//! use phonon::audio_similarity::{AudioSimilarityScorer, SimilarityConfig};
//!
//! let config = SimilarityConfig::default();
//! let scorer = AudioSimilarityScorer::new(44100.0, config);
//!
//! let similarity = scorer.compare(&audio1, &audio2);
//! assert!(similarity.overall >= 0.8, "Audio should be similar");
//! ```

use rustfft::{num_complex::Complex, FftPlanner};
use std::collections::VecDeque;
use std::f32::consts::PI;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for audio similarity scoring
#[derive(Debug, Clone)]
pub struct SimilarityConfig {
    /// FFT size for spectral analysis (power of 2, typically 1024-4096)
    pub fft_size: usize,
    /// Hop size for STFT (typically fft_size / 4 for 75% overlap)
    pub hop_size: usize,
    /// Weight for rhythm similarity (0-1)
    pub rhythm_weight: f32,
    /// Weight for spectral similarity (0-1)
    pub spectral_weight: f32,
    /// Weight for chroma similarity (0-1)
    pub chroma_weight: f32,
    /// Weight for envelope similarity (0-1)
    pub envelope_weight: f32,
    /// Onset detection threshold (spectral flux)
    pub onset_threshold: f32,
    /// Minimum peak distance in seconds for onset detection
    pub min_onset_gap: f32,
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 512,
            rhythm_weight: 0.3,
            spectral_weight: 0.3,
            chroma_weight: 0.2,
            envelope_weight: 0.2,
            onset_threshold: 0.15,
            min_onset_gap: 0.05, // 50ms minimum between onsets
        }
    }
}

impl SimilarityConfig {
    /// Configuration optimized for drum/percussion comparison
    pub fn drums() -> Self {
        Self {
            fft_size: 1024,
            hop_size: 256,
            rhythm_weight: 0.5, // Rhythm is most important for drums
            spectral_weight: 0.2,
            chroma_weight: 0.0, // Drums don't have pitch
            envelope_weight: 0.3,
            onset_threshold: 0.1,
            min_onset_gap: 0.03,
        }
    }

    /// Configuration optimized for melodic content
    pub fn melodic() -> Self {
        Self {
            fft_size: 4096, // Higher resolution for pitch
            hop_size: 1024,
            rhythm_weight: 0.2,
            spectral_weight: 0.3,
            chroma_weight: 0.4, // Pitch is most important
            envelope_weight: 0.1,
            onset_threshold: 0.2,
            min_onset_gap: 0.1,
        }
    }
}

// ============================================================================
// Onset Detection using Spectral Flux
// ============================================================================

/// Onset event with timing and strength
#[derive(Debug, Clone, Copy)]
pub struct Onset {
    /// Time in seconds
    pub time: f64,
    /// Onset strength (spectral flux magnitude)
    pub strength: f32,
}

/// Spectral flux onset detector
///
/// Detects note onsets by measuring changes in the frequency spectrum.
/// More accurate than simple energy-based detection, especially for
/// pitched instruments and overlapping sounds.
pub struct SpectralFluxDetector {
    sample_rate: f32,
    fft_size: usize,
    hop_size: usize,
    window: Vec<f32>,
    prev_spectrum: Vec<f32>,
    flux_history: VecDeque<f32>,
    onset_threshold: f32,
    min_onset_gap_samples: usize,
    planner: FftPlanner<f32>,
}

impl SpectralFluxDetector {
    pub fn new(sample_rate: f32, config: &SimilarityConfig) -> Self {
        let fft_size = config.fft_size;

        // Create Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos()))
            .collect();

        Self {
            sample_rate,
            fft_size,
            hop_size: config.hop_size,
            window,
            prev_spectrum: vec![0.0; fft_size / 2 + 1],
            flux_history: VecDeque::with_capacity(50),
            onset_threshold: config.onset_threshold,
            min_onset_gap_samples: (sample_rate * config.min_onset_gap) as usize,
            planner: FftPlanner::new(),
        }
    }

    /// Detect onsets in an audio buffer
    pub fn detect_onsets(&mut self, audio: &[f32]) -> Vec<Onset> {
        let mut onsets = Vec::new();
        let mut flux_values = Vec::new();

        // Reset state
        self.prev_spectrum.fill(0.0);
        self.flux_history.clear();

        // Calculate spectral flux for each frame
        let num_frames = (audio.len().saturating_sub(self.fft_size)) / self.hop_size;

        for frame_idx in 0..num_frames {
            let start = frame_idx * self.hop_size;
            let frame = &audio[start..start + self.fft_size];
            let flux = self.calculate_spectral_flux(frame);
            flux_values.push((start, flux));
        }

        if flux_values.is_empty() {
            return onsets;
        }

        // Calculate adaptive threshold
        let mean_flux: f32 =
            flux_values.iter().map(|(_, f)| f).sum::<f32>() / flux_values.len() as f32;
        let std_dev = {
            let variance: f32 = flux_values
                .iter()
                .map(|(_, f)| (f - mean_flux).powi(2))
                .sum::<f32>()
                / flux_values.len() as f32;
            variance.sqrt()
        };

        let threshold = mean_flux + std_dev * self.onset_threshold * 10.0;

        // Peak picking with minimum distance
        let mut last_onset_sample: i64 = -(self.min_onset_gap_samples as i64);

        for i in 1..flux_values.len().saturating_sub(1) {
            let (sample_idx, flux) = flux_values[i];
            let (_, prev_flux) = flux_values[i - 1];
            let (_, next_flux) = flux_values[i + 1];

            // Local maximum above threshold
            if flux > threshold && flux > prev_flux && flux >= next_flux {
                // Check minimum distance
                if sample_idx as i64 - last_onset_sample >= self.min_onset_gap_samples as i64 {
                    let time = sample_idx as f64 / self.sample_rate as f64;
                    onsets.push(Onset {
                        time,
                        strength: flux,
                    });
                    last_onset_sample = sample_idx as i64;
                }
            }
        }

        onsets
    }

    /// Calculate spectral flux for a single frame
    fn calculate_spectral_flux(&mut self, frame: &[f32]) -> f32 {
        // Apply window and prepare FFT input
        let mut fft_buffer: Vec<Complex<f32>> = frame
            .iter()
            .zip(self.window.iter())
            .map(|(&sample, &window)| Complex::new(sample * window, 0.0))
            .collect();

        fft_buffer.resize(self.fft_size, Complex::new(0.0, 0.0));

        // Perform FFT
        let fft = self.planner.plan_fft_forward(self.fft_size);
        fft.process(&mut fft_buffer);

        // Calculate magnitude spectrum
        let mut spectrum: Vec<f32> = fft_buffer
            .iter()
            .take(self.fft_size / 2 + 1)
            .map(|c| (c.re * c.re + c.im * c.im).sqrt())
            .collect();

        // Calculate half-wave rectified spectral flux
        // Only count increases in spectral energy (positive differences)
        let mut flux = 0.0;
        for (i, &mag) in spectrum.iter().enumerate() {
            let diff = mag - self.prev_spectrum.get(i).copied().unwrap_or(0.0);
            flux += diff.max(0.0); // Half-wave rectification
        }

        // Store current spectrum for next frame
        std::mem::swap(&mut self.prev_spectrum, &mut spectrum);

        // Normalize by number of bins
        flux / (self.fft_size / 2) as f32
    }
}

// ============================================================================
// Rhythm Pattern Extraction and Comparison
// ============================================================================

/// Rhythm pattern represented as inter-onset intervals (IOIs)
#[derive(Debug, Clone)]
pub struct RhythmPattern {
    /// Inter-onset intervals in seconds
    pub intervals: Vec<f64>,
    /// Original onset times
    pub onset_times: Vec<f64>,
}

impl RhythmPattern {
    /// Extract rhythm pattern from onset times
    pub fn from_onsets(onsets: &[Onset]) -> Self {
        let onset_times: Vec<f64> = onsets.iter().map(|o| o.time).collect();
        let intervals: Vec<f64> = onset_times.windows(2).map(|w| w[1] - w[0]).collect();

        Self {
            intervals,
            onset_times,
        }
    }

    /// Normalize intervals to sum to 1.0 (tempo-invariant representation)
    pub fn normalized(&self) -> Vec<f64> {
        let sum: f64 = self.intervals.iter().sum();
        if sum > 0.0 {
            self.intervals.iter().map(|&i| i / sum).collect()
        } else {
            self.intervals.clone()
        }
    }

    /// Compare with another rhythm pattern
    /// Returns similarity score 0.0 (different) to 1.0 (identical)
    pub fn compare(&self, other: &RhythmPattern, tolerance: f64) -> f64 {
        if self.intervals.is_empty() && other.intervals.is_empty() {
            return 1.0;
        }
        if self.intervals.is_empty() || other.intervals.is_empty() {
            return 0.0;
        }

        // Compare normalized patterns (tempo-invariant)
        let norm_self = self.normalized();
        let norm_other = other.normalized();

        // Handle different lengths
        let (shorter, longer) = if norm_self.len() <= norm_other.len() {
            (&norm_self, &norm_other)
        } else {
            (&norm_other, &norm_self)
        };

        // Try to find best alignment
        let mut best_score: f64 = 0.0;

        for offset in 0..=longer.len().saturating_sub(shorter.len()) {
            let mut matches = 0;
            for (i, &s) in shorter.iter().enumerate() {
                if i + offset < longer.len() {
                    let diff = (s - longer[i + offset]).abs();
                    if diff <= tolerance {
                        matches += 1;
                    }
                }
            }
            let score = matches as f64 / shorter.len().max(longer.len()) as f64;
            best_score = best_score.max(score);
        }

        best_score
    }
}

// ============================================================================
// Chroma Features (Pitch Class Distribution)
// ============================================================================

/// Chroma features representing the distribution of energy across 12 pitch classes
#[derive(Debug, Clone)]
pub struct ChromaFeatures {
    /// 12-element vector for pitch classes C, C#, D, D#, E, F, F#, G, G#, A, A#, B
    pub chroma: Vec<f32>,
}

impl ChromaFeatures {
    /// Calculate chroma features from audio
    pub fn from_audio(audio: &[f32], sample_rate: f32, fft_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);

        // Prepare window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos()))
            .collect();

        let mut chroma = vec![0.0f32; 12];
        let mut frame_count = 0;

        // Process overlapping frames
        let hop_size = fft_size / 4;
        for start in (0..audio.len().saturating_sub(fft_size)).step_by(hop_size) {
            let frame = &audio[start..start + fft_size];

            // Apply window and FFT
            let mut fft_buffer: Vec<Complex<f32>> = frame
                .iter()
                .zip(window.iter())
                .map(|(&s, &w)| Complex::new(s * w, 0.0))
                .collect();
            fft_buffer.resize(fft_size, Complex::new(0.0, 0.0));
            fft.process(&mut fft_buffer);

            // Map FFT bins to pitch classes
            for (bin, c) in fft_buffer.iter().take(fft_size / 2).enumerate() {
                if bin == 0 {
                    continue;
                } // Skip DC

                let freq = bin as f32 * sample_rate / fft_size as f32;
                if freq < 20.0 || freq > 5000.0 {
                    continue;
                } // Skip out-of-range frequencies

                // Convert frequency to pitch class (0-11)
                // Using A4 = 440Hz as reference
                let semitones_from_a4 = 12.0 * (freq / 440.0).log2();
                let pitch_class = ((semitones_from_a4.round() as i32 % 12) + 12) % 12;

                let magnitude = (c.re * c.re + c.im * c.im).sqrt();
                chroma[pitch_class as usize] += magnitude;
            }

            frame_count += 1;
        }

        // Normalize
        if frame_count > 0 {
            let max_val = chroma.iter().fold(0.0f32, |a, &b| a.max(b));
            if max_val > 0.0 {
                for c in &mut chroma {
                    *c /= max_val;
                }
            }
        }

        Self { chroma }
    }

    /// Compare with another chroma feature vector using cosine similarity
    pub fn compare(&self, other: &ChromaFeatures) -> f32 {
        let dot: f32 = self
            .chroma
            .iter()
            .zip(other.chroma.iter())
            .map(|(a, b)| a * b)
            .sum();
        let norm_self: f32 = self.chroma.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_other: f32 = other.chroma.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_self > 0.0 && norm_other > 0.0 {
            dot / (norm_self * norm_other)
        } else {
            0.0
        }
    }
}

// ============================================================================
// Envelope Features
// ============================================================================

/// Amplitude envelope of audio signal
#[derive(Debug, Clone)]
pub struct EnvelopeFeatures {
    /// RMS values per frame
    pub rms: Vec<f32>,
    /// Frame duration in seconds
    pub frame_duration: f32,
}

impl EnvelopeFeatures {
    /// Extract amplitude envelope from audio
    pub fn from_audio(audio: &[f32], sample_rate: f32, frame_size: usize) -> Self {
        let frame_duration = frame_size as f32 / sample_rate;
        let mut rms = Vec::new();

        for chunk in audio.chunks(frame_size) {
            let frame_rms = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
            rms.push(frame_rms);
        }

        Self {
            rms,
            frame_duration,
        }
    }

    /// Compare envelopes using correlation
    pub fn compare(&self, other: &EnvelopeFeatures) -> f32 {
        if self.rms.is_empty() || other.rms.is_empty() {
            return 0.0;
        }

        // Use the shorter length
        let len = self.rms.len().min(other.rms.len());
        let a = &self.rms[..len];
        let b = &other.rms[..len];

        // Calculate correlation coefficient
        let mean_a: f32 = a.iter().sum::<f32>() / len as f32;
        let mean_b: f32 = b.iter().sum::<f32>() / len as f32;

        let mut cov = 0.0;
        let mut var_a = 0.0;
        let mut var_b = 0.0;

        for i in 0..len {
            let da = a[i] - mean_a;
            let db = b[i] - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }

        if var_a > 0.0 && var_b > 0.0 {
            (cov / (var_a.sqrt() * var_b.sqrt())).max(0.0).min(1.0)
        } else {
            0.0
        }
    }
}

// ============================================================================
// Spectral Features
// ============================================================================

/// Spectral features for audio comparison
#[derive(Debug, Clone)]
pub struct SpectralFeatures {
    /// Average spectral centroid in Hz
    pub centroid: f32,
    /// Spectral spread (bandwidth) in Hz
    pub spread: f32,
    /// Spectral flatness (0 = tonal, 1 = noise)
    pub flatness: f32,
    /// Spectral rolloff frequency (85% energy threshold)
    pub rolloff: f32,
}

impl SpectralFeatures {
    /// Calculate spectral features from audio
    pub fn from_audio(audio: &[f32], sample_rate: f32, fft_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);

        let window: Vec<f32> = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos()))
            .collect();

        let mut total_centroid = 0.0f64;
        let mut total_spread = 0.0f64;
        let mut total_flatness = 0.0f64;
        let mut total_rolloff = 0.0f64;
        let mut frame_count = 0;

        let hop_size = fft_size / 4;

        for start in (0..audio.len().saturating_sub(fft_size)).step_by(hop_size) {
            let frame = &audio[start..start + fft_size];

            let mut fft_buffer: Vec<Complex<f32>> = frame
                .iter()
                .zip(window.iter())
                .map(|(&s, &w)| Complex::new(s * w, 0.0))
                .collect();
            fft_buffer.resize(fft_size, Complex::new(0.0, 0.0));
            fft.process(&mut fft_buffer);

            let magnitudes: Vec<f32> = fft_buffer
                .iter()
                .take(fft_size / 2)
                .map(|c| (c.re * c.re + c.im * c.im).sqrt())
                .collect();

            let bin_freq = sample_rate / fft_size as f32;

            // Spectral centroid
            let (centroid, magnitude_sum) = {
                let mut weighted_sum = 0.0f64;
                let mut mag_sum = 0.0f64;
                for (i, &mag) in magnitudes.iter().enumerate() {
                    let freq = i as f64 * bin_freq as f64;
                    weighted_sum += freq * mag as f64;
                    mag_sum += mag as f64;
                }
                if mag_sum > 0.0 {
                    (weighted_sum / mag_sum, mag_sum)
                } else {
                    (0.0, 0.0)
                }
            };
            total_centroid += centroid;

            // Spectral spread
            if magnitude_sum > 0.0 {
                let mut variance_sum = 0.0f64;
                for (i, &mag) in magnitudes.iter().enumerate() {
                    let freq = i as f64 * bin_freq as f64;
                    let diff = freq - centroid;
                    variance_sum += diff * diff * mag as f64;
                }
                total_spread += (variance_sum / magnitude_sum).sqrt();
            }

            // Spectral flatness
            let epsilon = 1e-10f64;
            let mut log_sum = 0.0f64;
            let mut arith_sum = 0.0f64;
            for &mag in &magnitudes {
                let m = (mag as f64).max(epsilon);
                log_sum += m.ln();
                arith_sum += m;
            }
            let n = magnitudes.len() as f64;
            let geo_mean = (log_sum / n).exp();
            let arith_mean = arith_sum / n;
            if arith_mean > epsilon {
                total_flatness += geo_mean / arith_mean;
            }

            // Spectral rolloff (85% energy)
            let total_energy: f32 = magnitudes.iter().map(|m| m * m).sum();
            let threshold = 0.85 * total_energy;
            let mut cumulative = 0.0;
            for (i, &mag) in magnitudes.iter().enumerate() {
                cumulative += mag * mag;
                if cumulative >= threshold {
                    total_rolloff += i as f64 * bin_freq as f64;
                    break;
                }
            }

            frame_count += 1;
        }

        if frame_count > 0 {
            let fc = frame_count as f64;
            Self {
                centroid: (total_centroid / fc) as f32,
                spread: (total_spread / fc) as f32,
                flatness: (total_flatness / fc) as f32,
                rolloff: (total_rolloff / fc) as f32,
            }
        } else {
            Self {
                centroid: 0.0,
                spread: 0.0,
                flatness: 0.0,
                rolloff: 0.0,
            }
        }
    }

    /// Compare spectral features with another
    pub fn compare(&self, other: &SpectralFeatures) -> f32 {
        // Normalize differences relative to typical ranges
        let centroid_diff = (self.centroid - other.centroid).abs() / 2000.0; // 0-2kHz range
        let spread_diff = (self.spread - other.spread).abs() / 2000.0;
        let flatness_diff = (self.flatness - other.flatness).abs(); // 0-1 range
        let rolloff_diff = (self.rolloff - other.rolloff).abs() / 10000.0; // 0-10kHz range

        // Average normalized similarity
        let avg_diff = (centroid_diff + spread_diff + flatness_diff + rolloff_diff) / 4.0;

        (1.0 - avg_diff).max(0.0).min(1.0)
    }
}

// ============================================================================
// Audio Similarity Scorer
// ============================================================================

/// Complete audio similarity comparison result
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    /// Overall similarity score (0-1, weighted combination)
    pub overall: f32,
    /// Rhythm pattern similarity (0-1)
    pub rhythm: f32,
    /// Spectral similarity (0-1)
    pub spectral: f32,
    /// Chroma/pitch class similarity (0-1)
    pub chroma: f32,
    /// Envelope/dynamics similarity (0-1)
    pub envelope: f32,
    /// Number of onsets detected in audio 1
    pub onsets_a: usize,
    /// Number of onsets detected in audio 2
    pub onsets_b: usize,
}

impl SimilarityResult {
    /// Check if similarity meets a threshold
    pub fn is_similar(&self, threshold: f32) -> bool {
        self.overall >= threshold
    }

    /// Human-readable description
    pub fn description(&self) -> String {
        let level = if self.overall >= 0.9 {
            "highly similar"
        } else if self.overall >= 0.7 {
            "similar"
        } else if self.overall >= 0.5 {
            "somewhat similar"
        } else if self.overall >= 0.3 {
            "different"
        } else {
            "very different"
        };

        format!(
            "{} (overall: {:.1}%, rhythm: {:.1}%, spectral: {:.1}%, chroma: {:.1}%, envelope: {:.1}%)",
            level,
            self.overall * 100.0,
            self.rhythm * 100.0,
            self.spectral * 100.0,
            self.chroma * 100.0,
            self.envelope * 100.0
        )
    }
}

/// Audio similarity scorer for comparing two audio signals
pub struct AudioSimilarityScorer {
    sample_rate: f32,
    config: SimilarityConfig,
}

impl AudioSimilarityScorer {
    /// Create a new similarity scorer
    pub fn new(sample_rate: f32, config: SimilarityConfig) -> Self {
        Self {
            sample_rate,
            config,
        }
    }

    /// Compare two audio buffers and return similarity scores
    pub fn compare(&self, audio_a: &[f32], audio_b: &[f32]) -> SimilarityResult {
        // Detect onsets
        let mut onset_detector = SpectralFluxDetector::new(self.sample_rate, &self.config);
        let onsets_a = onset_detector.detect_onsets(audio_a);

        let mut onset_detector = SpectralFluxDetector::new(self.sample_rate, &self.config);
        let onsets_b = onset_detector.detect_onsets(audio_b);

        // Calculate rhythm similarity
        let rhythm_a = RhythmPattern::from_onsets(&onsets_a);
        let rhythm_b = RhythmPattern::from_onsets(&onsets_b);
        let rhythm_sim = rhythm_a.compare(&rhythm_b, 0.1) as f32;

        // Calculate spectral similarity
        let spectral_a =
            SpectralFeatures::from_audio(audio_a, self.sample_rate, self.config.fft_size);
        let spectral_b =
            SpectralFeatures::from_audio(audio_b, self.sample_rate, self.config.fft_size);
        let spectral_sim = spectral_a.compare(&spectral_b);

        // Calculate chroma similarity
        let chroma_a = ChromaFeatures::from_audio(audio_a, self.sample_rate, self.config.fft_size);
        let chroma_b = ChromaFeatures::from_audio(audio_b, self.sample_rate, self.config.fft_size);
        let chroma_sim = chroma_a.compare(&chroma_b);

        // Calculate envelope similarity
        let envelope_a =
            EnvelopeFeatures::from_audio(audio_a, self.sample_rate, self.config.hop_size);
        let envelope_b =
            EnvelopeFeatures::from_audio(audio_b, self.sample_rate, self.config.hop_size);
        let envelope_sim = envelope_a.compare(&envelope_b);

        // Weighted combination
        let total_weight = self.config.rhythm_weight
            + self.config.spectral_weight
            + self.config.chroma_weight
            + self.config.envelope_weight;

        let overall = if total_weight > 0.0 {
            (rhythm_sim * self.config.rhythm_weight
                + spectral_sim * self.config.spectral_weight
                + chroma_sim * self.config.chroma_weight
                + envelope_sim * self.config.envelope_weight)
                / total_weight
        } else {
            0.0
        };

        SimilarityResult {
            overall,
            rhythm: rhythm_sim,
            spectral: spectral_sim,
            chroma: chroma_sim,
            envelope: envelope_sim,
            onsets_a: onsets_a.len(),
            onsets_b: onsets_b.len(),
        }
    }

    /// Quick check if two audio signals are similar (above threshold)
    pub fn is_similar(&self, audio_a: &[f32], audio_b: &[f32], threshold: f32) -> bool {
        self.compare(audio_a, audio_b).is_similar(threshold)
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Detect onsets in audio using spectral flux
pub fn detect_onsets(audio: &[f32], sample_rate: f32) -> Vec<Onset> {
    let config = SimilarityConfig::default();
    let mut detector = SpectralFluxDetector::new(sample_rate, &config);
    detector.detect_onsets(audio)
}

/// Extract rhythm pattern from audio
pub fn extract_rhythm_pattern(audio: &[f32], sample_rate: f32) -> RhythmPattern {
    let onsets = detect_onsets(audio, sample_rate);
    RhythmPattern::from_onsets(&onsets)
}

/// Compare two rhythm patterns with tolerance
/// Returns similarity score 0.0 (different) to 1.0 (identical)
pub fn compare_rhythm_patterns(expected: &[f64], actual: &[f64], tolerance: f64) -> f64 {
    if expected.is_empty() && actual.is_empty() {
        return 1.0;
    }
    if expected.is_empty() || actual.is_empty() {
        return 0.0;
    }

    // Normalize both patterns
    let sum_exp: f64 = expected.iter().sum();
    let sum_act: f64 = actual.iter().sum();

    let norm_exp: Vec<f64> = if sum_exp > 0.0 {
        expected.iter().map(|&x| x / sum_exp).collect()
    } else {
        expected.to_vec()
    };

    let norm_act: Vec<f64> = if sum_act > 0.0 {
        actual.iter().map(|&x| x / sum_act).collect()
    } else {
        actual.to_vec()
    };

    // Compare element by element
    let len = norm_exp.len().min(norm_act.len());
    let mut matches = 0;

    for i in 0..len {
        if (norm_exp[i] - norm_act[i]).abs() <= tolerance {
            matches += 1;
        }
    }

    matches as f64 / norm_exp.len().max(norm_act.len()) as f64
}

/// Quick audio similarity check
pub fn audio_similarity(audio_a: &[f32], audio_b: &[f32], sample_rate: f32) -> f32 {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    scorer.compare(audio_a, audio_b).overall
}

/// Check if two audio signals are rhythmically similar
pub fn rhythm_similarity(audio_a: &[f32], audio_b: &[f32], sample_rate: f32) -> f32 {
    let pattern_a = extract_rhythm_pattern(audio_a, sample_rate);
    let pattern_b = extract_rhythm_pattern(audio_b, sample_rate);
    pattern_a.compare(&pattern_b, 0.1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_sine(freq: f32, duration: f32, sample_rate: f32) -> Vec<f32> {
        let num_samples = (duration * sample_rate) as usize;
        (0..num_samples)
            .map(|i| (2.0 * PI * freq * i as f32 / sample_rate).sin() * 0.5)
            .collect()
    }

    fn generate_impulse_train(times: &[f32], duration: f32, sample_rate: f32) -> Vec<f32> {
        let num_samples = (duration * sample_rate) as usize;
        let mut audio = vec![0.0; num_samples];

        for &time in times {
            let sample_idx = (time * sample_rate) as usize;
            if sample_idx < num_samples {
                // Create short attack envelope
                for i in 0..100.min(num_samples - sample_idx) {
                    let t = i as f32 / 100.0;
                    let env = (-t * 20.0).exp(); // Fast decay
                    audio[sample_idx + i] += 0.8 * env * (2.0 * PI * 200.0 * t).sin();
                }
            }
        }

        audio
    }

    #[test]
    fn test_spectral_flux_onset_detection() {
        let sample_rate = 44100.0;

        // Generate impulse train at 0.25, 0.5, 0.75 seconds
        let audio = generate_impulse_train(&[0.25, 0.5, 0.75], 1.0, sample_rate);

        let config = SimilarityConfig::default();
        let mut detector = SpectralFluxDetector::new(sample_rate, &config);
        let onsets = detector.detect_onsets(&audio);

        assert!(
            onsets.len() >= 2,
            "Should detect at least 2 onsets, got {}",
            onsets.len()
        );

        // Check approximate timing of first onset
        if !onsets.is_empty() {
            assert!(
                (onsets[0].time - 0.25).abs() < 0.1,
                "First onset should be near 0.25s, got {:.3}s",
                onsets[0].time
            );
        }
    }

    #[test]
    fn test_rhythm_pattern_comparison_identical() {
        let pattern_a = RhythmPattern {
            intervals: vec![0.25, 0.25, 0.25, 0.25],
            onset_times: vec![0.0, 0.25, 0.5, 0.75, 1.0],
        };

        let pattern_b = RhythmPattern {
            intervals: vec![0.25, 0.25, 0.25, 0.25],
            onset_times: vec![0.0, 0.25, 0.5, 0.75, 1.0],
        };

        let similarity = pattern_a.compare(&pattern_b, 0.05);
        assert!(
            similarity >= 0.9,
            "Identical patterns should have high similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_rhythm_pattern_comparison_different() {
        let pattern_a = RhythmPattern {
            intervals: vec![0.25, 0.25, 0.25, 0.25], // Regular quarter notes
            onset_times: vec![0.0, 0.25, 0.5, 0.75, 1.0],
        };

        let pattern_b = RhythmPattern {
            intervals: vec![0.5, 0.5], // Half notes
            onset_times: vec![0.0, 0.5, 1.0],
        };

        let similarity = pattern_a.compare(&pattern_b, 0.05);
        assert!(
            similarity < 0.5,
            "Different patterns should have low similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_rhythm_pattern_tempo_invariant() {
        // Same rhythm at different tempos
        let pattern_slow = RhythmPattern {
            intervals: vec![0.5, 0.5, 0.5, 0.5],
            onset_times: vec![0.0, 0.5, 1.0, 1.5, 2.0],
        };

        let pattern_fast = RhythmPattern {
            intervals: vec![0.25, 0.25, 0.25, 0.25],
            onset_times: vec![0.0, 0.25, 0.5, 0.75, 1.0],
        };

        // Normalized patterns should be identical
        let similarity = pattern_slow.compare(&pattern_fast, 0.05);
        assert!(
            similarity >= 0.9,
            "Same rhythm at different tempos should match: {}",
            similarity
        );
    }

    #[test]
    fn test_chroma_features_same_pitch() {
        let sample_rate = 44100.0;

        // A4 = 440Hz
        let audio_a = generate_sine(440.0, 0.5, sample_rate);
        // A5 = 880Hz (same pitch class, one octave up)
        let audio_b = generate_sine(880.0, 0.5, sample_rate);

        let chroma_a = ChromaFeatures::from_audio(&audio_a, sample_rate, 4096);
        let chroma_b = ChromaFeatures::from_audio(&audio_b, sample_rate, 4096);

        let similarity = chroma_a.compare(&chroma_b);
        assert!(
            similarity >= 0.6,
            "Same pitch class should have high chroma similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_chroma_features_different_pitch() {
        let sample_rate = 44100.0;

        // A4 = 440Hz
        let audio_a = generate_sine(440.0, 0.5, sample_rate);
        // C4 ≈ 262Hz (different pitch class)
        let audio_b = generate_sine(262.0, 0.5, sample_rate);

        let chroma_a = ChromaFeatures::from_audio(&audio_a, sample_rate, 4096);
        let chroma_b = ChromaFeatures::from_audio(&audio_b, sample_rate, 4096);

        let similarity = chroma_a.compare(&chroma_b);
        assert!(
            similarity < 0.9,
            "Different pitch classes should have lower similarity: {}",
            similarity
        );
    }

    #[test]
    fn test_spectral_features_low_vs_high() {
        let sample_rate = 44100.0;

        let low_audio = generate_sine(200.0, 0.5, sample_rate);
        let high_audio = generate_sine(2000.0, 0.5, sample_rate);

        let spectral_low = SpectralFeatures::from_audio(&low_audio, sample_rate, 2048);
        let spectral_high = SpectralFeatures::from_audio(&high_audio, sample_rate, 2048);

        assert!(
            spectral_high.centroid > spectral_low.centroid,
            "High frequency should have higher centroid: {} vs {}",
            spectral_high.centroid,
            spectral_low.centroid
        );
    }

    #[test]
    fn test_envelope_similarity_identical() {
        let sample_rate = 44100.0;
        let audio = generate_sine(440.0, 1.0, sample_rate);

        let envelope_a = EnvelopeFeatures::from_audio(&audio, sample_rate, 512);
        let envelope_b = EnvelopeFeatures::from_audio(&audio, sample_rate, 512);

        let similarity = envelope_a.compare(&envelope_b);
        assert!(
            similarity >= 0.99,
            "Identical audio should have identical envelope: {}",
            similarity
        );
    }

    #[test]
    fn test_audio_similarity_identical() {
        let sample_rate = 44100.0;
        let audio = generate_sine(440.0, 1.0, sample_rate);

        let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
        let result = scorer.compare(&audio, &audio);

        assert!(
            result.overall >= 0.9,
            "Identical audio should have high similarity: {:?}",
            result
        );
    }

    #[test]
    fn test_audio_similarity_different() {
        let sample_rate = 44100.0;

        // Sine wave vs impulse train
        let sine = generate_sine(440.0, 1.0, sample_rate);
        let impulses = generate_impulse_train(&[0.1, 0.3, 0.5, 0.7, 0.9], 1.0, sample_rate);

        let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
        let result = scorer.compare(&sine, &impulses);

        assert!(
            result.overall < 0.7,
            "Different audio should have lower similarity: {:?}",
            result
        );
    }

    #[test]
    fn test_convenience_functions() {
        let sample_rate = 44100.0;
        let audio = generate_impulse_train(&[0.25, 0.5, 0.75], 1.0, sample_rate);

        // Test detect_onsets
        let onsets = detect_onsets(&audio, sample_rate);
        assert!(!onsets.is_empty(), "Should detect onsets");

        // Test extract_rhythm_pattern
        let pattern = extract_rhythm_pattern(&audio, sample_rate);
        assert!(
            !pattern.intervals.is_empty(),
            "Should extract rhythm pattern"
        );

        // Test compare_rhythm_patterns
        let intervals1 = vec![0.25, 0.25, 0.25];
        let intervals2 = vec![0.25, 0.25, 0.25];
        let sim = compare_rhythm_patterns(&intervals1, &intervals2, 0.05);
        assert!(sim >= 0.9, "Identical intervals should match: {}", sim);
    }

    #[test]
    fn test_similarity_config_presets() {
        let drums = SimilarityConfig::drums();
        assert!(
            drums.rhythm_weight > drums.chroma_weight,
            "Drums should prioritize rhythm"
        );

        let melodic = SimilarityConfig::melodic();
        assert!(
            melodic.chroma_weight > melodic.rhythm_weight,
            "Melodic should prioritize chroma"
        );
    }

    #[test]
    fn test_similarity_result_description() {
        let result = SimilarityResult {
            overall: 0.85,
            rhythm: 0.9,
            spectral: 0.8,
            chroma: 0.85,
            envelope: 0.85,
            onsets_a: 4,
            onsets_b: 4,
        };

        let desc = result.description();
        assert!(
            desc.contains("similar"),
            "Should describe as similar: {}",
            desc
        );
    }
}
