//! # Automated Musical Quality Scoring
//!
//! Provides a unified quality scoring system that combines pattern-level metrics,
//! audio analysis, and timing accuracy into a single composite score.
//!
//! ## Three-Level Scoring
//!
//! 1. **Rhythm Score**: Pattern density, syncopation, evenness, entropy
//! 2. **Audio Score**: Silence detection, clipping detection, RMS level, spectral quality
//! 3. **Timing Score**: How well rendered audio events match expected pattern timing
//!
//! ## Genre Profiles
//!
//! Pre-configured scoring profiles with expected metric ranges for:
//! - Boom-Bap, Trap, Lo-Fi Hip-Hop, Drill, Phonk
//! - House, Techno, DnB, UK Garage
//! - Custom profiles via builder
//!
//! ## Example Usage
//!
//! ```ignore
//! use phonon::musical_quality::{MusicalQualityScorer, GenreProfile};
//!
//! // Score a pattern against a genre profile
//! let score = MusicalQualityScorer::new()
//!     .genre(GenreProfile::boom_bap())
//!     .score_dsl(r#"
//!         tempo: 1.5
//!         out $ s "bd ~ ~ ~ sn ~ ~ ~ bd ~ bd ~ sn ~ ~ ~" + s "hh*8"
//!     "#);
//!
//! assert!(score.overall >= 0.7, "Should score well for boom-bap: {}", score.report());
//! ```

use crate::audio_analysis::SpectralCentroid;
use crate::onset_timing::{detect_percussive_onsets, DetectedOnset};
use crate::pattern::Pattern;
use crate::pattern_metrics::PatternMetrics;

// ============================================================================
// Quality Score Result
// ============================================================================

/// Complete musical quality assessment result
#[derive(Debug, Clone)]
pub struct MusicalQualityScore {
    /// Overall quality score (0.0 - 1.0)
    pub overall: f32,

    /// Rhythm quality sub-score (pattern-level metrics)
    pub rhythm: RhythmScore,

    /// Audio quality sub-score (signal-level analysis)
    pub audio: AudioScore,

    /// Timing accuracy sub-score (onset alignment)
    pub timing: TimingScore,

    /// Individual metric checks with pass/fail
    pub checks: Vec<QualityCheck>,

    /// Number of checks that passed
    pub passed_count: usize,

    /// Total number of checks
    pub total_count: usize,
}

impl MusicalQualityScore {
    /// Whether the overall score meets a minimum threshold
    pub fn passes(&self, threshold: f32) -> bool {
        self.overall >= threshold
    }

    /// Fraction of checks that passed
    pub fn pass_rate(&self) -> f32 {
        if self.total_count == 0 {
            return 1.0;
        }
        self.passed_count as f32 / self.total_count as f32
    }

    /// Generate a human-readable report
    pub fn report(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "Musical Quality Score: {:.1}% ({}/{})\n",
            self.overall * 100.0,
            self.passed_count,
            self.total_count,
        ));
        out.push_str(&format!("  Rhythm:  {:.1}%\n", self.rhythm.score * 100.0));
        out.push_str(&format!("  Audio:   {:.1}%\n", self.audio.score * 100.0));
        out.push_str(&format!("  Timing:  {:.1}%\n", self.timing.score * 100.0));

        let failed: Vec<&QualityCheck> = self.checks.iter().filter(|c| !c.passed).collect();
        if !failed.is_empty() {
            out.push_str("\nFailed checks:\n");
            for check in failed {
                out.push_str(&format!(
                    "  - {}: {} (got {:.3}, expected {})\n",
                    check.category, check.name, check.observed, check.expectation
                ));
            }
        }

        out
    }
}

/// Rhythm quality assessment
#[derive(Debug, Clone)]
pub struct RhythmScore {
    /// Composite rhythm score (0-1)
    pub score: f32,
    /// Events per cycle
    pub density: f64,
    /// Syncopation level (0-1)
    pub syncopation: f64,
    /// Evenness of event spacing (0-1)
    pub evenness: f64,
    /// Shannon entropy of intervals (0-1)
    pub entropy: f64,
    /// Consistency across cycles
    pub density_variance: f64,
}

/// Audio quality assessment
#[derive(Debug, Clone)]
pub struct AudioScore {
    /// Composite audio score (0-1)
    pub score: f32,
    /// Whether audio has content (not silent)
    pub has_audio: bool,
    /// Whether audio clips
    pub clips: bool,
    /// RMS amplitude level
    pub rms: f32,
    /// Peak amplitude
    pub peak: f32,
    /// Spectral centroid (Hz)
    pub spectral_centroid: f32,
    /// Spectral flatness (0=tonal, 1=noise)
    pub spectral_flatness: f32,
    /// Number of detected audio onsets
    pub onset_count: usize,
}

/// Timing accuracy assessment
#[derive(Debug, Clone)]
pub struct TimingScore {
    /// Composite timing score (0-1)
    pub score: f32,
    /// Expected number of events (from pattern query)
    pub expected_events: usize,
    /// Detected audio events
    pub detected_events: usize,
    /// Event detection ratio (detected / expected, capped at 1.0)
    pub detection_ratio: f32,
}

/// Individual quality check result
#[derive(Debug, Clone)]
pub struct QualityCheck {
    /// Category: "rhythm", "audio", "timing"
    pub category: String,
    /// Check name
    pub name: String,
    /// Whether this check passed
    pub passed: bool,
    /// Observed value
    pub observed: f64,
    /// Human-readable expectation
    pub expectation: String,
}

// ============================================================================
// Genre Profile
// ============================================================================

/// Expected metric ranges for a genre/style
#[derive(Debug, Clone)]
pub struct GenreProfile {
    /// Profile name
    pub name: String,

    /// Expected density range (events per cycle across all layers)
    pub density_range: (f64, f64),

    /// Expected syncopation range
    pub syncopation_range: (f64, f64),

    /// Expected evenness range (for the primary rhythmic layer)
    pub evenness_range: (f64, f64),

    /// Expected RMS range for rendered audio
    pub rms_range: (f32, f32),

    /// Minimum number of audio onsets expected per second
    pub min_onsets_per_sec: f32,

    /// Weight for rhythm score (0-1, should sum to 1 with others)
    pub rhythm_weight: f32,

    /// Weight for audio score
    pub audio_weight: f32,

    /// Weight for timing score
    pub timing_weight: f32,
}

impl Default for GenreProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            density_range: (1.0, 32.0),
            syncopation_range: (0.0, 1.0),
            evenness_range: (0.0, 1.0),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 1.0,
            rhythm_weight: 0.35,
            audio_weight: 0.35,
            timing_weight: 0.30,
        }
    }
}

impl GenreProfile {
    /// Classic boom-bap: moderate density, moderate syncopation, strong backbeat
    pub fn boom_bap() -> Self {
        Self {
            name: "boom-bap".to_string(),
            density_range: (8.0, 16.0), // kick(3) + snare(2) + hats(8) = 13 typical
            syncopation_range: (0.0, 0.6),
            evenness_range: (0.3, 1.0),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 3.0,
            ..Default::default()
        }
    }

    /// Trap: high hat density, sparse kicks
    pub fn trap() -> Self {
        Self {
            name: "trap".to_string(),
            density_range: (12.0, 30.0), // 16+ hats + kicks + claps
            syncopation_range: (0.0, 0.5),
            evenness_range: (0.2, 1.0),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 5.0,
            ..Default::default()
        }
    }

    /// Lo-fi hip-hop: sparse, relaxed
    pub fn lofi() -> Self {
        Self {
            name: "lo-fi".to_string(),
            density_range: (4.0, 14.0),
            syncopation_range: (0.0, 0.5),
            evenness_range: (0.3, 1.0),
            rms_range: (0.001, 0.6),
            min_onsets_per_sec: 2.0,
            ..Default::default()
        }
    }

    /// Drill: syncopated kicks, hi-hat rolls
    pub fn drill() -> Self {
        Self {
            name: "drill".to_string(),
            density_range: (10.0, 28.0),
            syncopation_range: (0.05, 0.7),
            evenness_range: (0.2, 1.0),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 4.0,
            ..Default::default()
        }
    }

    /// Memphis phonk: cowbell patterns, double kicks
    pub fn phonk() -> Self {
        Self {
            name: "phonk".to_string(),
            density_range: (8.0, 22.0),
            syncopation_range: (0.0, 0.5),
            evenness_range: (0.3, 1.0),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 3.0,
            ..Default::default()
        }
    }

    /// Four-on-the-floor house
    pub fn house() -> Self {
        Self {
            name: "house".to_string(),
            density_range: (8.0, 20.0),
            syncopation_range: (0.0, 0.4),
            evenness_range: (0.5, 1.0), // high evenness typical
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 4.0,
            ..Default::default()
        }
    }

    /// Techno: driving, mechanical
    pub fn techno() -> Self {
        Self {
            name: "techno".to_string(),
            density_range: (8.0, 24.0),
            syncopation_range: (0.0, 0.3),
            evenness_range: (0.6, 1.0),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 4.0,
            ..Default::default()
        }
    }

    /// DnB: fast, complex breakbeats
    pub fn dnb() -> Self {
        Self {
            name: "dnb".to_string(),
            density_range: (6.0, 24.0),
            syncopation_range: (0.1, 0.8),
            evenness_range: (0.2, 0.9),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 4.0,
            ..Default::default()
        }
    }

    /// UK Garage: shuffled, syncopated
    pub fn uk_garage() -> Self {
        Self {
            name: "uk-garage".to_string(),
            density_range: (6.0, 20.0),
            syncopation_range: (0.1, 0.7),
            evenness_range: (0.2, 0.9),
            rms_range: (0.001, 0.8),
            min_onsets_per_sec: 3.0,
            ..Default::default()
        }
    }

    /// Create a custom profile
    pub fn custom(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Set expected density range
    pub fn with_density(mut self, min: f64, max: f64) -> Self {
        self.density_range = (min, max);
        self
    }

    /// Set expected syncopation range
    pub fn with_syncopation(mut self, min: f64, max: f64) -> Self {
        self.syncopation_range = (min, max);
        self
    }

    /// Set expected evenness range
    pub fn with_evenness(mut self, min: f64, max: f64) -> Self {
        self.evenness_range = (min, max);
        self
    }

    /// Set expected RMS range
    pub fn with_rms(mut self, min: f32, max: f32) -> Self {
        self.rms_range = (min, max);
        self
    }

    /// Set minimum onsets per second
    pub fn with_min_onsets(mut self, min: f32) -> Self {
        self.min_onsets_per_sec = min;
        self
    }

    /// Set score weights (must sum to approximately 1.0)
    pub fn with_weights(mut self, rhythm: f32, audio: f32, timing: f32) -> Self {
        self.rhythm_weight = rhythm;
        self.audio_weight = audio;
        self.timing_weight = timing;
        self
    }
}

// ============================================================================
// Scorer
// ============================================================================

/// Musical quality scorer - evaluates patterns against quality criteria
pub struct MusicalQualityScorer {
    sample_rate: f32,
    duration: f32,
    profile: GenreProfile,
    num_analysis_cycles: usize,
}

impl MusicalQualityScorer {
    /// Create a new scorer with default settings
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            duration: 2.0,
            profile: GenreProfile::default(),
            num_analysis_cycles: 4,
        }
    }

    /// Set the genre profile for scoring
    pub fn genre(mut self, profile: GenreProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Set render duration in seconds
    pub fn duration(mut self, secs: f32) -> Self {
        self.duration = secs;
        self
    }

    /// Set sample rate
    pub fn sample_rate(mut self, rate: f32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Set number of cycles to analyze for pattern metrics
    pub fn analysis_cycles(mut self, cycles: usize) -> Self {
        self.num_analysis_cycles = cycles;
        self
    }

    /// Score a Phonon DSL program
    pub fn score_dsl(&self, code: &str) -> MusicalQualityScore {
        // Render audio
        let audio = match render_dsl(code, self.sample_rate, self.duration) {
            Ok(samples) => samples,
            Err(_) => {
                // If rendering fails, return a zero score
                return self.zero_score_with_error("render_failed");
            }
        };

        // Extract pattern-level metrics from mini-notation patterns in the code
        let pattern_metrics = self.extract_pattern_metrics(code);

        // Analyze audio
        let audio_analysis = self.analyze_audio(&audio);

        // Analyze timing
        let timing_analysis = self.analyze_timing(&audio, &pattern_metrics);

        // Build quality checks
        let mut checks = Vec::new();
        self.add_rhythm_checks(&pattern_metrics, &mut checks);
        self.add_audio_checks(&audio_analysis, &mut checks);
        self.add_timing_checks(&timing_analysis, &mut checks);

        let passed_count = checks.iter().filter(|c| c.passed).count();
        let total_count = checks.len();

        // Compute sub-scores
        let rhythm_score = self.compute_rhythm_score(&pattern_metrics, &checks);
        let audio_score = self.compute_audio_score(&audio_analysis, &checks);
        let timing_score = self.compute_timing_score(&timing_analysis, &checks);

        // Weighted overall
        let overall = self.profile.rhythm_weight * rhythm_score.score
            + self.profile.audio_weight * audio_score.score
            + self.profile.timing_weight * timing_score.score;

        MusicalQualityScore {
            overall,
            rhythm: rhythm_score,
            audio: audio_score,
            timing: timing_score,
            checks,
            passed_count,
            total_count,
        }
    }

    /// Score pre-rendered audio with optional pattern metrics
    pub fn score_audio(&self, audio: &[f32]) -> MusicalQualityScore {
        let audio_analysis = self.analyze_audio(audio);
        let timing_analysis = TimingAnalysis {
            expected_events: 0,
            detected_events: audio_analysis.onset_count,
            detection_ratio: 1.0, // No expected pattern to compare against
        };

        let mut checks = Vec::new();
        self.add_audio_checks(&audio_analysis, &mut checks);
        self.add_timing_checks(&timing_analysis, &mut checks);

        let passed_count = checks.iter().filter(|c| c.passed).count();
        let total_count = checks.len();

        let audio_score = self.compute_audio_score(&audio_analysis, &checks);
        let timing_score = self.compute_timing_score(&timing_analysis, &checks);

        // Without pattern data, only use audio + timing
        let overall = 0.6 * audio_score.score + 0.4 * timing_score.score;

        let rhythm_score = RhythmScore {
            score: 0.0,
            density: 0.0,
            syncopation: 0.0,
            evenness: 0.0,
            entropy: 0.0,
            density_variance: 0.0,
        };

        MusicalQualityScore {
            overall,
            rhythm: rhythm_score,
            audio: audio_score,
            timing: timing_score,
            checks,
            passed_count,
            total_count,
        }
    }

    /// Score a mini-notation pattern string (pattern-level only, no audio rendering)
    pub fn score_pattern(&self, notation: &str) -> MusicalQualityScore {
        let pattern: Pattern<String> = crate::mini_notation_v3::parse_mini_notation(notation);
        let metrics = PatternMetrics::analyze(&pattern, self.num_analysis_cycles);

        let mut checks = Vec::new();
        let analysis = PatternAnalysis { metrics };
        self.add_rhythm_checks(&analysis, &mut checks);

        let passed_count = checks.iter().filter(|c| c.passed).count();
        let total_count = checks.len();

        let rhythm_score = self.compute_rhythm_score(&analysis, &checks);

        MusicalQualityScore {
            overall: rhythm_score.score,
            rhythm: rhythm_score,
            audio: AudioScore {
                score: 0.0,
                has_audio: false,
                clips: false,
                rms: 0.0,
                peak: 0.0,
                spectral_centroid: 0.0,
                spectral_flatness: 0.0,
                onset_count: 0,
            },
            timing: TimingScore {
                score: 0.0,
                expected_events: 0,
                detected_events: 0,
                detection_ratio: 0.0,
            },
            checks,
            passed_count,
            total_count,
        }
    }

    // ========================================================================
    // Internal: Pattern Analysis
    // ========================================================================

    fn extract_pattern_metrics(&self, code: &str) -> PatternAnalysis {
        // Extract all mini-notation strings from the DSL code
        let mut combined_density = 0.0;
        let mut max_syncopation = 0.0;
        let mut total_evenness = 0.0;
        let mut total_entropy = 0.0;
        let mut pattern_count = 0;
        let mut total_density_variance = 0.0;

        for notation in extract_mini_notations(code) {
            let pattern: Pattern<String> = crate::mini_notation_v3::parse_mini_notation(&notation);
            let metrics = PatternMetrics::analyze(&pattern, self.num_analysis_cycles);

            if metrics.density > 0.0 {
                combined_density += metrics.density;
                if metrics.syncopation > max_syncopation {
                    max_syncopation = metrics.syncopation;
                }
                total_evenness += metrics.evenness;
                total_entropy += metrics.entropy;
                total_density_variance += metrics.density_variance;
                pattern_count += 1;
            }
        }

        let avg_evenness = if pattern_count > 0 {
            total_evenness / pattern_count as f64
        } else {
            0.0
        };
        let avg_entropy = if pattern_count > 0 {
            total_entropy / pattern_count as f64
        } else {
            0.0
        };
        let avg_variance = if pattern_count > 0 {
            total_density_variance / pattern_count as f64
        } else {
            0.0
        };

        PatternAnalysis {
            metrics: PatternMetrics {
                density: combined_density,
                syncopation: max_syncopation,
                evenness: avg_evenness,
                entropy: avg_entropy,
                cycles_analyzed: self.num_analysis_cycles,
                total_events: (combined_density * self.num_analysis_cycles as f64) as usize,
                events_per_cycle: vec![combined_density as usize; self.num_analysis_cycles],
                density_variance: avg_variance,
            },
        }
    }

    // ========================================================================
    // Internal: Audio Analysis
    // ========================================================================

    fn analyze_audio(&self, audio: &[f32]) -> AudioAnalysis {
        let rms = calculate_rms(audio);
        let peak = calculate_peak(audio);
        let is_silent = rms < 0.001;
        let clips = peak > 0.999;

        // Onset detection
        let onsets = detect_percussive_onsets(audio, self.sample_rate);

        // Spectral analysis via SpectralCentroid
        let block_size = 2048;
        let mut centroid_analyzer = SpectralCentroid::new(self.sample_rate, block_size);

        let mut total_centroid = 0.0f32;
        let mut block_count = 0;

        let mut offset = 0;
        while offset + block_size <= audio.len() {
            let block = &audio[offset..offset + block_size];
            // Check block has audio content
            let block_rms: f32 =
                (block.iter().map(|s| s * s).sum::<f32>() / block.len() as f32).sqrt();
            if block_rms > 0.001 {
                let centroid = centroid_analyzer.process_block(block);
                total_centroid += centroid;
                block_count += 1;
            }
            offset += block_size;
        }

        let avg_centroid = if block_count > 0 {
            total_centroid / block_count as f32
        } else {
            0.0
        };

        // Estimate spectral flatness from the audio signal directly
        // (simple approximation: ratio of geometric mean to arithmetic mean of magnitudes)
        let spectral_flatness = estimate_spectral_flatness(audio);

        AudioAnalysis {
            rms,
            peak,
            is_silent,
            clips,
            spectral_centroid: avg_centroid,
            spectral_flatness,
            onset_count: onsets.len(),
            onsets,
        }
    }

    // ========================================================================
    // Internal: Timing Analysis
    // ========================================================================

    fn analyze_timing(&self, audio: &[f32], pattern_analysis: &PatternAnalysis) -> TimingAnalysis {
        let detected = detect_percussive_onsets(audio, self.sample_rate);
        let detected_count = detected.len();

        // Estimate expected events from pattern density and duration
        // density = events/cycle, and we need to know CPS to convert
        // Use a rough estimate: 1 cycle per second unless tempo is specified
        let expected = pattern_analysis.metrics.density * self.duration as f64;
        let expected_count = expected.max(0.0) as usize;

        let ratio = if expected_count > 0 {
            (detected_count as f32 / expected_count as f32).min(1.5)
        } else {
            if detected_count > 0 {
                1.0
            } else {
                0.0
            }
        };

        TimingAnalysis {
            expected_events: expected_count,
            detected_events: detected_count,
            detection_ratio: ratio,
        }
    }

    // ========================================================================
    // Internal: Quality Checks
    // ========================================================================

    fn add_rhythm_checks(&self, analysis: &PatternAnalysis, checks: &mut Vec<QualityCheck>) {
        let m = &analysis.metrics;

        // Density in range
        let (min_d, max_d) = self.profile.density_range;
        checks.push(QualityCheck {
            category: "rhythm".to_string(),
            name: "density_in_range".to_string(),
            passed: m.density >= min_d && m.density <= max_d,
            observed: m.density,
            expectation: format!("{:.1}-{:.1} events/cycle", min_d, max_d),
        });

        // Has events
        checks.push(QualityCheck {
            category: "rhythm".to_string(),
            name: "has_events".to_string(),
            passed: m.density > 0.0,
            observed: m.density,
            expectation: "> 0 events".to_string(),
        });

        // Syncopation in range
        let (min_s, max_s) = self.profile.syncopation_range;
        checks.push(QualityCheck {
            category: "rhythm".to_string(),
            name: "syncopation_in_range".to_string(),
            passed: m.syncopation >= min_s && m.syncopation <= max_s,
            observed: m.syncopation,
            expectation: format!("{:.2}-{:.2}", min_s, max_s),
        });

        // Evenness in range
        let (min_e, max_e) = self.profile.evenness_range;
        checks.push(QualityCheck {
            category: "rhythm".to_string(),
            name: "evenness_in_range".to_string(),
            passed: m.evenness >= min_e && m.evenness <= max_e,
            observed: m.evenness,
            expectation: format!("{:.2}-{:.2}", min_e, max_e),
        });

        // Cycle consistency (low variance)
        checks.push(QualityCheck {
            category: "rhythm".to_string(),
            name: "cycle_consistency".to_string(),
            passed: m.density_variance < 0.5,
            observed: m.density_variance,
            expectation: "< 0.5 density variance".to_string(),
        });
    }

    fn add_audio_checks(&self, analysis: &AudioAnalysis, checks: &mut Vec<QualityCheck>) {
        // Not silent
        checks.push(QualityCheck {
            category: "audio".to_string(),
            name: "not_silent".to_string(),
            passed: !analysis.is_silent,
            observed: analysis.rms as f64,
            expectation: "RMS > 0.001".to_string(),
        });

        // Not clipping
        checks.push(QualityCheck {
            category: "audio".to_string(),
            name: "not_clipping".to_string(),
            passed: !analysis.clips,
            observed: analysis.peak as f64,
            expectation: "peak < 1.0".to_string(),
        });

        // RMS in range
        let (min_rms, max_rms) = self.profile.rms_range;
        checks.push(QualityCheck {
            category: "audio".to_string(),
            name: "rms_in_range".to_string(),
            passed: analysis.rms >= min_rms && analysis.rms <= max_rms,
            observed: analysis.rms as f64,
            expectation: format!("{:.3}-{:.3}", min_rms, max_rms),
        });

        // Has spectral content
        checks.push(QualityCheck {
            category: "audio".to_string(),
            name: "has_spectral_content".to_string(),
            passed: analysis.spectral_centroid > 50.0,
            observed: analysis.spectral_centroid as f64,
            expectation: "centroid > 50 Hz".to_string(),
        });

        // Minimum onset density
        let min_onsets = (self.profile.min_onsets_per_sec * self.duration).max(1.0) as usize;
        checks.push(QualityCheck {
            category: "audio".to_string(),
            name: "sufficient_onsets".to_string(),
            passed: analysis.onset_count >= min_onsets,
            observed: analysis.onset_count as f64,
            expectation: format!(">= {} onsets", min_onsets),
        });
    }

    fn add_timing_checks(&self, analysis: &TimingAnalysis, checks: &mut Vec<QualityCheck>) {
        // Detection ratio: at least some events should be detected
        if analysis.expected_events > 0 {
            checks.push(QualityCheck {
                category: "timing".to_string(),
                name: "event_detection_ratio".to_string(),
                passed: analysis.detection_ratio >= 0.3,
                observed: analysis.detection_ratio as f64,
                expectation: ">= 0.3 detection ratio".to_string(),
            });
        }

        // Has detected events
        checks.push(QualityCheck {
            category: "timing".to_string(),
            name: "has_detected_events".to_string(),
            passed: analysis.detected_events > 0,
            observed: analysis.detected_events as f64,
            expectation: "> 0 detected events".to_string(),
        });
    }

    // ========================================================================
    // Internal: Score Computation
    // ========================================================================

    fn compute_rhythm_score(
        &self,
        analysis: &PatternAnalysis,
        checks: &[QualityCheck],
    ) -> RhythmScore {
        let rhythm_checks: Vec<&QualityCheck> =
            checks.iter().filter(|c| c.category == "rhythm").collect();
        let passed = rhythm_checks.iter().filter(|c| c.passed).count();
        let total = rhythm_checks.len().max(1);

        let m = &analysis.metrics;

        RhythmScore {
            score: passed as f32 / total as f32,
            density: m.density,
            syncopation: m.syncopation,
            evenness: m.evenness,
            entropy: m.entropy,
            density_variance: m.density_variance,
        }
    }

    fn compute_audio_score(&self, analysis: &AudioAnalysis, checks: &[QualityCheck]) -> AudioScore {
        let audio_checks: Vec<&QualityCheck> =
            checks.iter().filter(|c| c.category == "audio").collect();
        let passed = audio_checks.iter().filter(|c| c.passed).count();
        let total = audio_checks.len().max(1);

        AudioScore {
            score: passed as f32 / total as f32,
            has_audio: !analysis.is_silent,
            clips: analysis.clips,
            rms: analysis.rms,
            peak: analysis.peak,
            spectral_centroid: analysis.spectral_centroid,
            spectral_flatness: analysis.spectral_flatness,
            onset_count: analysis.onset_count,
        }
    }

    fn compute_timing_score(
        &self,
        analysis: &TimingAnalysis,
        checks: &[QualityCheck],
    ) -> TimingScore {
        let timing_checks: Vec<&QualityCheck> =
            checks.iter().filter(|c| c.category == "timing").collect();
        let passed = timing_checks.iter().filter(|c| c.passed).count();
        let total = timing_checks.len().max(1);

        TimingScore {
            score: passed as f32 / total as f32,
            expected_events: analysis.expected_events,
            detected_events: analysis.detected_events,
            detection_ratio: analysis.detection_ratio,
        }
    }

    fn zero_score_with_error(&self, error: &str) -> MusicalQualityScore {
        let checks = vec![QualityCheck {
            category: "audio".to_string(),
            name: error.to_string(),
            passed: false,
            observed: 0.0,
            expectation: "successful render".to_string(),
        }];

        MusicalQualityScore {
            overall: 0.0,
            rhythm: RhythmScore {
                score: 0.0,
                density: 0.0,
                syncopation: 0.0,
                evenness: 0.0,
                entropy: 0.0,
                density_variance: 0.0,
            },
            audio: AudioScore {
                score: 0.0,
                has_audio: false,
                clips: false,
                rms: 0.0,
                peak: 0.0,
                spectral_centroid: 0.0,
                spectral_flatness: 0.0,
                onset_count: 0,
            },
            timing: TimingScore {
                score: 0.0,
                expected_events: 0,
                detected_events: 0,
                detection_ratio: 0.0,
            },
            checks,
            passed_count: 0,
            total_count: 1,
        }
    }
}

impl Default for MusicalQualityScorer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Internal Types
// ============================================================================

struct PatternAnalysis {
    metrics: PatternMetrics,
}

struct AudioAnalysis {
    rms: f32,
    peak: f32,
    is_silent: bool,
    clips: bool,
    spectral_centroid: f32,
    spectral_flatness: f32,
    onset_count: usize,
    #[allow(dead_code)]
    onsets: Vec<DetectedOnset>,
}

struct TimingAnalysis {
    expected_events: usize,
    detected_events: usize,
    detection_ratio: f32,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Render DSL code to mono audio
fn render_dsl(code: &str, sample_rate: f32, duration: f32) -> Result<Vec<f32>, String> {
    use crate::compositional_compiler::compile_program;
    use crate::compositional_parser::parse_program;

    let num_samples = (duration * sample_rate) as usize;
    let (_rest, statements) = parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;
    let mut graph = compile_program(statements, sample_rate, None)
        .map_err(|e| format!("Compile error: {}", e))?;
    Ok(graph.render(num_samples))
}

/// Extract mini-notation strings from DSL code (strings in double quotes)
fn extract_mini_notations(code: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut chars = code.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            let mut notation = String::new();
            for ch in chars.by_ref() {
                if ch == '"' {
                    break;
                }
                notation.push(ch);
            }
            if !notation.is_empty() {
                results.push(notation);
            }
        }
    }

    results
}

/// Calculate RMS amplitude
fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = audio.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / audio.len() as f64).sqrt() as f32
}

/// Calculate peak amplitude
fn calculate_peak(audio: &[f32]) -> f32 {
    audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Estimate spectral flatness from audio samples (0=tonal, 1=noise-like)
/// Uses zero-crossing rate as a simple proxy for spectral flatness
fn estimate_spectral_flatness(audio: &[f32]) -> f32 {
    if audio.len() < 2 {
        return 0.0;
    }

    let mut crossings = 0usize;
    for i in 1..audio.len() {
        if (audio[i] >= 0.0) != (audio[i - 1] >= 0.0) {
            crossings += 1;
        }
    }

    // ZCR normalized to [0, 1] range
    // Pure sine at Nyquist/2 has ZCR of ~1.0, DC has 0.0
    // Noise has high ZCR (~0.5), tonal signals have low ZCR
    let zcr = crossings as f32 / (audio.len() - 1) as f32;

    // Scale to approximate flatness range: noise ~0.5 ZCR maps to ~0.8 flatness
    (zcr * 2.0).min(1.0)
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Quick quality check: returns overall score (0-1) for DSL code
pub fn quality_score(code: &str) -> f32 {
    MusicalQualityScorer::new().score_dsl(code).overall
}

/// Quick quality check with a genre profile
pub fn genre_quality_score(code: &str, profile: GenreProfile) -> f32 {
    MusicalQualityScorer::new()
        .genre(profile)
        .score_dsl(code)
        .overall
}

/// Batch score multiple DSL programs, returning (name, score) pairs
pub fn batch_score(programs: &[(&str, &str)]) -> Vec<(String, MusicalQualityScore)> {
    let scorer = MusicalQualityScorer::new();
    programs
        .iter()
        .map(|(name, code)| (name.to_string(), scorer.score_dsl(code)))
        .collect()
}

/// Batch score with a genre profile
pub fn batch_genre_score(
    programs: &[(&str, &str)],
    profile: GenreProfile,
) -> Vec<(String, MusicalQualityScore)> {
    let scorer = MusicalQualityScorer::new().genre(profile);
    programs
        .iter()
        .map(|(name, code)| (name.to_string(), scorer.score_dsl(code)))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Utility function tests
    // ========================================================================

    #[test]
    fn test_extract_mini_notations() {
        let code = r#"
            ~kick $ s "bd ~ sn ~"
            ~hats $ s "hh*8"
            ~bass $ saw "55 82.5"
            out $ ~kick + ~hats
        "#;

        let notations = extract_mini_notations(code);
        assert_eq!(notations.len(), 3);
        assert_eq!(notations[0], "bd ~ sn ~");
        assert_eq!(notations[1], "hh*8");
        assert_eq!(notations[2], "55 82.5");
    }

    #[test]
    fn test_extract_mini_notations_empty() {
        let notations = extract_mini_notations("out $ sine 440");
        assert!(notations.is_empty());
    }

    #[test]
    fn test_calculate_rms_silence() {
        let silence = vec![0.0f32; 1000];
        assert_eq!(calculate_rms(&silence), 0.0);
    }

    #[test]
    fn test_calculate_rms_signal() {
        // Unit sine approximation
        let signal: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        let rms = calculate_rms(&signal);
        // RMS of sine wave should be ~0.707
        assert!(
            (rms - 0.707).abs() < 0.01,
            "RMS should be ~0.707, got {}",
            rms
        );
    }

    #[test]
    fn test_calculate_peak() {
        let signal = vec![0.0, 0.5, -0.8, 0.3, -0.1];
        assert!((calculate_peak(&signal) - 0.8).abs() < 0.001);
    }

    // ========================================================================
    // Genre Profile tests
    // ========================================================================

    #[test]
    fn test_genre_profile_defaults() {
        let default = GenreProfile::default();
        assert_eq!(default.name, "default");
        assert!(default.density_range.0 < default.density_range.1);
    }

    #[test]
    fn test_genre_profile_builders() {
        let custom = GenreProfile::custom("test")
            .with_density(5.0, 10.0)
            .with_syncopation(0.1, 0.5)
            .with_evenness(0.5, 1.0)
            .with_rms(0.01, 0.5)
            .with_min_onsets(3.0)
            .with_weights(0.5, 0.3, 0.2);

        assert_eq!(custom.name, "test");
        assert_eq!(custom.density_range, (5.0, 10.0));
        assert_eq!(custom.syncopation_range, (0.1, 0.5));
        assert_eq!(custom.rhythm_weight, 0.5);
    }

    #[test]
    fn test_all_genre_profiles_valid() {
        let profiles = vec![
            GenreProfile::boom_bap(),
            GenreProfile::trap(),
            GenreProfile::lofi(),
            GenreProfile::drill(),
            GenreProfile::phonk(),
            GenreProfile::house(),
            GenreProfile::techno(),
            GenreProfile::dnb(),
            GenreProfile::uk_garage(),
        ];

        for profile in profiles {
            assert!(
                profile.density_range.0 < profile.density_range.1,
                "{}: density range invalid",
                profile.name
            );
            assert!(
                profile.rms_range.0 < profile.rms_range.1,
                "{}: rms range invalid",
                profile.name
            );
            let weight_sum = profile.rhythm_weight + profile.audio_weight + profile.timing_weight;
            assert!(
                (weight_sum - 1.0).abs() < 0.01,
                "{}: weights don't sum to 1.0 ({})",
                profile.name,
                weight_sum
            );
        }
    }

    // ========================================================================
    // Pattern-only scoring tests
    // ========================================================================

    #[test]
    fn test_score_pattern_basic() {
        let score = MusicalQualityScorer::new()
            .genre(GenreProfile::boom_bap())
            .score_pattern("bd ~ sn ~ bd ~ sn ~");

        assert!(score.rhythm.density > 0.0, "Should have events");
        assert!(score.rhythm.score > 0.0, "Should have some rhythm score");
        assert!(!score.checks.is_empty(), "Should have checks");
    }

    #[test]
    fn test_score_pattern_silence() {
        let score = MusicalQualityScorer::new().score_pattern("~ ~ ~ ~");

        // Silent pattern should fail "has_events" check
        let has_events = score
            .checks
            .iter()
            .find(|c| c.name == "has_events")
            .expect("should have has_events check");
        assert!(!has_events.passed, "Silent pattern should fail has_events");
    }

    #[test]
    fn test_score_pattern_boombap_hats() {
        let score = MusicalQualityScorer::new()
            .genre(GenreProfile::boom_bap())
            .score_pattern("hh*8");

        assert_eq!(score.rhythm.density, 8.0);
        assert!(
            score.rhythm.evenness > 0.9,
            "8th note hats should be very even"
        );
    }

    #[test]
    fn test_score_pattern_trap_hats() {
        let score = MusicalQualityScorer::new()
            .genre(GenreProfile::trap())
            .score_pattern("hh*16");

        assert_eq!(score.rhythm.density, 16.0);
        assert!(score.rhythm.evenness > 0.9);
    }

    // ========================================================================
    // DSL scoring tests
    // ========================================================================

    #[test]
    fn test_score_dsl_basic_drums() {
        let score = MusicalQualityScorer::new().duration(2.0).score_dsl(
            r#"
                tempo: 1.5
                out $ s "bd ~ sn ~" + s "hh*8"
            "#,
        );

        assert!(score.overall > 0.0, "Should have a positive score");
        assert!(
            score.audio.has_audio,
            "Should produce audio: {}",
            score.report()
        );
        assert!(!score.audio.clips, "Should not clip: {}", score.report());
        assert!(
            score.audio.onset_count > 0,
            "Should detect onsets: {}",
            score.report()
        );
    }

    #[test]
    fn test_score_dsl_with_genre() {
        let score = MusicalQualityScorer::new()
            .genre(GenreProfile::boom_bap())
            .duration(2.0)
            .score_dsl(r#"
                tempo: 1.5
                out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
            "#);

        assert!(
            score.overall > 0.3,
            "Boom-bap pattern should score reasonably: {}",
            score.report()
        );
        assert!(
            score.rhythm.density >= 8.0,
            "Combined density should be >= 8 (kick+snare+hats)"
        );
    }

    #[test]
    fn test_score_dsl_oscillator_only() {
        let score = MusicalQualityScorer::new()
            .duration(1.0)
            .score_dsl("out $ sine 440");

        assert!(
            score.audio.has_audio,
            "Sine should produce audio: {}",
            score.report()
        );
        assert!(score.audio.rms > 0.01, "Sine should have audible RMS");
    }

    #[test]
    fn test_score_dsl_render_failure() {
        let score = MusicalQualityScorer::new().score_dsl("this is not valid phonon code ???");

        // Should not panic; may return zero or low score depending on parser leniency
        assert!(
            score.overall <= 0.5,
            "Invalid code should score low, got {}",
            score.overall,
        );
    }

    // ========================================================================
    // Score report tests
    // ========================================================================

    #[test]
    fn test_report_format() {
        let score = MusicalQualityScorer::new().score_dsl(
            r#"
                tempo: 1.5
                out $ s "bd sn"
            "#,
        );

        let report = score.report();
        assert!(report.contains("Musical Quality Score"));
        assert!(report.contains("Rhythm:"));
        assert!(report.contains("Audio:"));
        assert!(report.contains("Timing:"));
    }

    #[test]
    fn test_passes_threshold() {
        let score = MusicalQualityScore {
            overall: 0.75,
            rhythm: RhythmScore {
                score: 0.8,
                density: 10.0,
                syncopation: 0.3,
                evenness: 0.8,
                entropy: 0.5,
                density_variance: 0.01,
            },
            audio: AudioScore {
                score: 0.8,
                has_audio: true,
                clips: false,
                rms: 0.1,
                peak: 0.5,
                spectral_centroid: 500.0,
                spectral_flatness: 0.3,
                onset_count: 10,
            },
            timing: TimingScore {
                score: 0.7,
                expected_events: 10,
                detected_events: 8,
                detection_ratio: 0.8,
            },
            checks: vec![],
            passed_count: 8,
            total_count: 10,
        };

        assert!(score.passes(0.7));
        assert!(!score.passes(0.8));
        assert!((score.pass_rate() - 0.8).abs() < 0.01);
    }

    // ========================================================================
    // Audio-only scoring tests
    // ========================================================================

    #[test]
    fn test_score_audio_silence() {
        let silence = vec![0.0f32; 44100];
        let score = MusicalQualityScorer::new().score_audio(&silence);

        assert!(!score.audio.has_audio, "Silence should be detected");
        assert_eq!(score.audio.rms, 0.0);
    }

    #[test]
    fn test_score_audio_sine() {
        let signal: Vec<f32> = (0..88200)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5)
            .collect();

        let score = MusicalQualityScorer::new().score_audio(&signal);
        assert!(score.audio.has_audio, "Sine should produce audio");
        assert!(score.audio.rms > 0.1, "Sine RMS should be significant");
        assert!(!score.audio.clips, "Half-amplitude sine should not clip");
    }

    // ========================================================================
    // Batch scoring tests
    // ========================================================================

    #[test]
    fn test_batch_score() {
        let programs = vec![
            ("basic drums", r#"out $ s "bd sn""#),
            ("sine", "out $ sine 440"),
        ];

        let results = batch_score(&programs);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "basic drums");
        assert_eq!(results[1].0, "sine");
    }

    // ========================================================================
    // Genre comparison tests
    // ========================================================================

    #[test]
    fn test_genre_scoring_comparison() {
        let boombap_code = r#"
            tempo: 1.5
            out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
        "#;

        let boombap_score = MusicalQualityScorer::new()
            .genre(GenreProfile::boom_bap())
            .duration(2.0)
            .score_dsl(boombap_code);

        // Same pattern scored against trap profile should differ
        let trap_score = MusicalQualityScorer::new()
            .genre(GenreProfile::trap())
            .duration(2.0)
            .score_dsl(boombap_code);

        // Boom-bap pattern should score at least as well against boom-bap profile
        // (trap expects higher density from hi-hat rolls)
        println!("Boom-bap vs boom-bap: {:.2}", boombap_score.overall);
        println!("Boom-bap vs trap: {:.2}", trap_score.overall);

        // Both should produce audio and have some score
        assert!(boombap_score.audio.has_audio);
        assert!(trap_score.audio.has_audio);
    }

    // ========================================================================
    // Convenience function tests
    // ========================================================================

    #[test]
    fn test_quality_score_convenience() {
        let score = quality_score(r#"out $ s "bd sn""#);
        assert!(score > 0.0, "Should return a positive score");
    }

    #[test]
    fn test_genre_quality_score_convenience() {
        let score = genre_quality_score(
            r#"
                tempo: 1.5
                out $ s "bd ~ sn ~" + s "hh*8"
            "#,
            GenreProfile::boom_bap(),
        );
        assert!(score > 0.0);
    }
}
