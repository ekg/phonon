//! A/B Comparison Testing Framework
//!
//! Provides a structured framework for comparing audio outputs side-by-side.
//! Supports three comparison modes:
//!
//! 1. **Pattern vs Pattern**: Compare two Phonon DSL patterns
//! 2. **Pattern vs Reference**: Compare a pattern against a golden reference WAV
//! 3. **Transform Verification**: Verify a transform changes audio in expected ways
//!
//! # Example
//!
//! ```ignore
//! use phonon::ab_test::{ABTest, Verdict};
//!
//! // Compare two patterns
//! let result = ABTest::compare_patterns("s \"bd sn\"", "s \"bd sn\" $ fast 2")
//!     .duration(2.0)
//!     .expect_different()
//!     .run();
//! assert!(result.passed(), "{}", result.report());
//!
//! // Verify a transform changes rhythm but not timbre
//! let result = ABTest::verify_transform("s \"bd sn hh cp\"", "fast 2")
//!     .expect_rhythm_different()
//!     .expect_spectral_similar(0.7)
//!     .run();
//! assert!(result.passed(), "{}", result.report());
//! ```

use crate::audio_similarity::{
    AudioSimilarityScorer, SimilarityConfig, SimilarityResult,
    SpectralFeatures,
};
use crate::reference_audio::{
    self, compute_rms_envelope, ComparisonConfig, EnvelopeStats,
};

/// Default sample rate for rendering
const DEFAULT_SAMPLE_RATE: f32 = 44100.0;
/// Default render duration in seconds
const DEFAULT_DURATION: f32 = 2.0;

// ============================================================================
// Core Types
// ============================================================================

/// Verdict for a single assertion within an A/B test
#[derive(Debug, Clone)]
pub struct AssertionResult {
    /// Name of the assertion (e.g., "rhythm_similar")
    pub name: String,
    /// Whether this assertion passed
    pub passed: bool,
    /// Observed value
    pub observed: f32,
    /// Required threshold
    pub threshold: f32,
    /// Direction of comparison
    pub direction: CompareDirection,
    /// Human-readable explanation
    pub detail: String,
}

/// Direction of comparison for an assertion
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompareDirection {
    /// Observed should be >= threshold
    AtLeast,
    /// Observed should be <= threshold
    AtMost,
}

/// Detailed result of an A/B comparison test
#[derive(Debug, Clone)]
pub struct ABResult {
    /// Label for audio A
    pub label_a: String,
    /// Label for audio B
    pub label_b: String,
    /// Full similarity result from the scorer
    pub similarity: SimilarityResult,
    /// Spectral features of audio A
    pub spectral_a: SpectralSummary,
    /// Spectral features of audio B
    pub spectral_b: SpectralSummary,
    /// Envelope statistics for audio A
    pub envelope_stats_a: EnvelopeStats,
    /// Envelope statistics for audio B
    pub envelope_stats_b: EnvelopeStats,
    /// RMS of audio A
    pub rms_a: f32,
    /// RMS of audio B
    pub rms_b: f32,
    /// Individual assertion results
    pub assertions: Vec<AssertionResult>,
}

/// Summary of spectral features for reporting
#[derive(Debug, Clone)]
pub struct SpectralSummary {
    pub centroid: f32,
    pub spread: f32,
    pub flatness: f32,
    pub rolloff: f32,
}

impl From<SpectralFeatures> for SpectralSummary {
    fn from(f: SpectralFeatures) -> Self {
        Self {
            centroid: f.centroid,
            spread: f.spread,
            flatness: f.flatness,
            rolloff: f.rolloff,
        }
    }
}

impl ABResult {
    /// Did all assertions pass?
    pub fn passed(&self) -> bool {
        self.assertions.iter().all(|a| a.passed)
    }

    /// How many assertions failed?
    pub fn failure_count(&self) -> usize {
        self.assertions.iter().filter(|a| !a.passed).count()
    }

    /// Generate a human-readable report
    pub fn report(&self) -> String {
        let mut lines = Vec::new();
        let status = if self.passed() { "PASS" } else { "FAIL" };

        lines.push(format!("=== A/B Test: {} ===", status));
        lines.push(format!("  A: {}", self.label_a));
        lines.push(format!("  B: {}", self.label_b));
        lines.push(String::new());

        // Similarity overview
        lines.push(format!(
            "  Similarity: {:.1}% overall (rhythm: {:.1}%, spectral: {:.1}%, chroma: {:.1}%, envelope: {:.1}%)",
            self.similarity.overall * 100.0,
            self.similarity.rhythm * 100.0,
            self.similarity.spectral * 100.0,
            self.similarity.chroma * 100.0,
            self.similarity.envelope * 100.0,
        ));
        lines.push(format!(
            "  Onsets: A={}, B={}",
            self.similarity.onsets_a, self.similarity.onsets_b
        ));
        lines.push(format!("  RMS: A={:.4}, B={:.4}", self.rms_a, self.rms_b));

        // Spectral comparison
        lines.push(format!(
            "  Spectral centroid: A={:.0}Hz, B={:.0}Hz",
            self.spectral_a.centroid, self.spectral_b.centroid
        ));
        lines.push(format!(
            "  Spectral flatness: A={:.3}, B={:.3}",
            self.spectral_a.flatness, self.spectral_b.flatness
        ));

        lines.push(String::new());

        // Assertion results
        lines.push("  Assertions:".to_string());
        for a in &self.assertions {
            let icon = if a.passed { "+" } else { "X" };
            let dir = match a.direction {
                CompareDirection::AtLeast => ">=",
                CompareDirection::AtMost => "<=",
            };
            lines.push(format!(
                "    [{}] {}: {:.3} {} {:.3} -- {}",
                icon, a.name, a.observed, dir, a.threshold, a.detail
            ));
        }

        lines.join("\n")
    }
}

// ============================================================================
// A/B Test Builder
// ============================================================================

/// Assertion to evaluate after comparison
#[derive(Debug, Clone)]
struct Assertion {
    name: String,
    /// Which field to check
    field: AssertionField,
    /// Threshold value
    threshold: f32,
    /// Direction
    direction: CompareDirection,
}

#[derive(Debug, Clone)]
enum AssertionField {
    Overall,
    Rhythm,
    Spectral,
    Chroma,
    Envelope,
    RmsRatio,
    OnsetCountA,
    OnsetCountB,
}

/// Source of audio for one side of the A/B test
#[derive(Debug, Clone)]
enum AudioSource {
    /// Render from Phonon DSL code
    Dsl(String),
    /// Load from raw samples
    Samples(Vec<f32>),
    /// Load from WAV file
    WavFile(String),
}

/// Builder for constructing A/B comparison tests
pub struct ABTest {
    source_a: AudioSource,
    source_b: AudioSource,
    label_a: String,
    label_b: String,
    sample_rate: f32,
    duration: f32,
    similarity_config: SimilarityConfig,
    assertions: Vec<Assertion>,
}

impl ABTest {
    // ---- Constructors ----

    /// Compare two Phonon DSL patterns
    pub fn compare_patterns(code_a: &str, code_b: &str) -> Self {
        Self {
            source_a: AudioSource::Dsl(code_a.to_string()),
            source_b: AudioSource::Dsl(code_b.to_string()),
            label_a: truncate_label(code_a),
            label_b: truncate_label(code_b),
            sample_rate: DEFAULT_SAMPLE_RATE,
            duration: DEFAULT_DURATION,
            similarity_config: SimilarityConfig::default(),
            assertions: Vec::new(),
        }
    }

    /// Compare a pattern against pre-rendered audio samples
    pub fn compare_pattern_to_audio(code: &str, reference: Vec<f32>) -> Self {
        Self {
            source_a: AudioSource::Dsl(code.to_string()),
            source_b: AudioSource::Samples(reference),
            label_a: truncate_label(code),
            label_b: "reference audio".to_string(),
            sample_rate: DEFAULT_SAMPLE_RATE,
            duration: DEFAULT_DURATION,
            similarity_config: SimilarityConfig::default(),
            assertions: Vec::new(),
        }
    }

    /// Compare a pattern against a golden reference WAV file
    pub fn compare_pattern_to_wav(code: &str, wav_path: &str) -> Self {
        Self {
            source_a: AudioSource::Dsl(code.to_string()),
            source_b: AudioSource::WavFile(wav_path.to_string()),
            label_a: truncate_label(code),
            label_b: wav_path.to_string(),
            sample_rate: DEFAULT_SAMPLE_RATE,
            duration: DEFAULT_DURATION,
            similarity_config: SimilarityConfig::default(),
            assertions: Vec::new(),
        }
    }

    /// Compare two pre-rendered audio buffers
    pub fn compare_audio(audio_a: Vec<f32>, audio_b: Vec<f32>) -> Self {
        Self {
            source_a: AudioSource::Samples(audio_a),
            source_b: AudioSource::Samples(audio_b),
            label_a: "audio A".to_string(),
            label_b: "audio B".to_string(),
            sample_rate: DEFAULT_SAMPLE_RATE,
            duration: DEFAULT_DURATION,
            similarity_config: SimilarityConfig::default(),
            assertions: Vec::new(),
        }
    }

    /// Verify a transform applied to a base pattern.
    ///
    /// Renders `base_code` as A and `base_code $ transform_code` as B.
    pub fn verify_transform(base_code: &str, transform_code: &str) -> Self {
        let code_b = format!("{} $ {}", base_code.trim(), transform_code.trim());
        Self {
            source_a: AudioSource::Dsl(base_code.to_string()),
            source_b: AudioSource::Dsl(code_b),
            label_a: truncate_label(base_code),
            label_b: format!("...$ {}", truncate_label(transform_code)),
            sample_rate: DEFAULT_SAMPLE_RATE,
            duration: DEFAULT_DURATION,
            similarity_config: SimilarityConfig::default(),
            assertions: Vec::new(),
        }
    }

    // ---- Configuration ----

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

    /// Set label for audio A
    pub fn label_a(mut self, label: &str) -> Self {
        self.label_a = label.to_string();
        self
    }

    /// Set label for audio B
    pub fn label_b(mut self, label: &str) -> Self {
        self.label_b = label.to_string();
        self
    }

    /// Use drum-optimized similarity config
    pub fn drums(mut self) -> Self {
        self.similarity_config = SimilarityConfig::drums();
        self
    }

    /// Use melodic-optimized similarity config
    pub fn melodic(mut self) -> Self {
        self.similarity_config = SimilarityConfig::melodic();
        self
    }

    /// Use custom similarity config
    pub fn config(mut self, config: SimilarityConfig) -> Self {
        self.similarity_config = config;
        self
    }

    // ---- Assertion builders ----

    /// Assert A and B are similar (overall >= threshold)
    pub fn expect_similar(mut self, min_similarity: f32) -> Self {
        self.assertions.push(Assertion {
            name: "overall_similar".to_string(),
            field: AssertionField::Overall,
            threshold: min_similarity,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert A and B are different (overall <= threshold)
    pub fn expect_different(self) -> Self {
        self.expect_max_similarity(0.5)
    }

    /// Assert overall similarity is at most this value
    pub fn expect_max_similarity(mut self, max_similarity: f32) -> Self {
        self.assertions.push(Assertion {
            name: "overall_different".to_string(),
            field: AssertionField::Overall,
            threshold: max_similarity,
            direction: CompareDirection::AtMost,
        });
        self
    }

    /// Assert rhythm similarity is at least this value
    pub fn expect_rhythm_similar(mut self, min: f32) -> Self {
        self.assertions.push(Assertion {
            name: "rhythm_similar".to_string(),
            field: AssertionField::Rhythm,
            threshold: min,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert rhythm similarity is at most this value
    pub fn expect_rhythm_different(mut self) -> Self {
        self.assertions.push(Assertion {
            name: "rhythm_different".to_string(),
            field: AssertionField::Rhythm,
            threshold: 0.5,
            direction: CompareDirection::AtMost,
        });
        self
    }

    /// Assert spectral similarity is at least this value
    pub fn expect_spectral_similar(mut self, min: f32) -> Self {
        self.assertions.push(Assertion {
            name: "spectral_similar".to_string(),
            field: AssertionField::Spectral,
            threshold: min,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert spectral similarity is at most this value
    pub fn expect_spectral_different(mut self, max: f32) -> Self {
        self.assertions.push(Assertion {
            name: "spectral_different".to_string(),
            field: AssertionField::Spectral,
            threshold: max,
            direction: CompareDirection::AtMost,
        });
        self
    }

    /// Assert chroma similarity is at least this value
    pub fn expect_chroma_similar(mut self, min: f32) -> Self {
        self.assertions.push(Assertion {
            name: "chroma_similar".to_string(),
            field: AssertionField::Chroma,
            threshold: min,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert envelope similarity is at least this value
    pub fn expect_envelope_similar(mut self, min: f32) -> Self {
        self.assertions.push(Assertion {
            name: "envelope_similar".to_string(),
            field: AssertionField::Envelope,
            threshold: min,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert the RMS ratio (B/A) is at least this value
    pub fn expect_louder(mut self, min_ratio: f32) -> Self {
        self.assertions.push(Assertion {
            name: "louder".to_string(),
            field: AssertionField::RmsRatio,
            threshold: min_ratio,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert the RMS ratio (B/A) is at most this value
    pub fn expect_quieter(mut self, max_ratio: f32) -> Self {
        self.assertions.push(Assertion {
            name: "quieter".to_string(),
            field: AssertionField::RmsRatio,
            threshold: max_ratio,
            direction: CompareDirection::AtMost,
        });
        self
    }

    /// Assert audio A has at least this many onsets
    pub fn expect_onsets_a_at_least(mut self, min: usize) -> Self {
        self.assertions.push(Assertion {
            name: "onsets_a_min".to_string(),
            field: AssertionField::OnsetCountA,
            threshold: min as f32,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Assert audio B has at least this many onsets
    pub fn expect_onsets_b_at_least(mut self, min: usize) -> Self {
        self.assertions.push(Assertion {
            name: "onsets_b_min".to_string(),
            field: AssertionField::OnsetCountB,
            threshold: min as f32,
            direction: CompareDirection::AtLeast,
        });
        self
    }

    /// Add a custom assertion
    pub fn assert_field(
        mut self,
        name: &str,
        field: &str,
        direction: CompareDirection,
        threshold: f32,
    ) -> Self {
        let field = match field {
            "overall" => AssertionField::Overall,
            "rhythm" => AssertionField::Rhythm,
            "spectral" => AssertionField::Spectral,
            "chroma" => AssertionField::Chroma,
            "envelope" => AssertionField::Envelope,
            "rms_ratio" => AssertionField::RmsRatio,
            "onsets_a" => AssertionField::OnsetCountA,
            "onsets_b" => AssertionField::OnsetCountB,
            _ => return self, // Unknown field, skip
        };
        self.assertions.push(Assertion {
            name: name.to_string(),
            field,
            threshold,
            direction,
        });
        self
    }

    // ---- Execution ----

    /// Run the A/B test and return detailed results
    pub fn run(self) -> ABResult {
        // Resolve audio sources
        let audio_a = resolve_audio(&self.source_a, self.sample_rate, self.duration);
        let audio_b = resolve_audio(&self.source_b, self.sample_rate, self.duration);

        self.run_with_audio(&audio_a, &audio_b)
    }

    /// Run the A/B test with pre-resolved audio buffers
    fn run_with_audio(self, audio_a: &[f32], audio_b: &[f32]) -> ABResult {
        // Compute similarity
        let scorer = AudioSimilarityScorer::new(self.sample_rate, self.similarity_config.clone());
        let similarity = scorer.compare(audio_a, audio_b);

        // Compute spectral features
        let fft_size = self.similarity_config.fft_size;
        let spectral_a = SpectralFeatures::from_audio(audio_a, self.sample_rate, fft_size);
        let spectral_b = SpectralFeatures::from_audio(audio_b, self.sample_rate, fft_size);

        // Compute envelope stats
        let env_a = compute_rms_envelope(audio_a, 512, 128);
        let env_b = compute_rms_envelope(audio_b, 512, 128);
        let envelope_stats_a = EnvelopeStats::from_envelope(&env_a);
        let envelope_stats_b = EnvelopeStats::from_envelope(&env_b);

        // Compute RMS
        let rms_a = calculate_rms(audio_a);
        let rms_b = calculate_rms(audio_b);

        let rms_ratio = if rms_a > 0.0 { rms_b / rms_a } else { 0.0 };

        // Evaluate assertions
        let assertions: Vec<AssertionResult> = self
            .assertions
            .iter()
            .map(|a| {
                let observed = match a.field {
                    AssertionField::Overall => similarity.overall,
                    AssertionField::Rhythm => similarity.rhythm,
                    AssertionField::Spectral => similarity.spectral,
                    AssertionField::Chroma => similarity.chroma,
                    AssertionField::Envelope => similarity.envelope,
                    AssertionField::RmsRatio => rms_ratio,
                    AssertionField::OnsetCountA => similarity.onsets_a as f32,
                    AssertionField::OnsetCountB => similarity.onsets_b as f32,
                };

                let passed = match a.direction {
                    CompareDirection::AtLeast => observed >= a.threshold,
                    CompareDirection::AtMost => observed <= a.threshold,
                };

                let dir_str = match a.direction {
                    CompareDirection::AtLeast => "at least",
                    CompareDirection::AtMost => "at most",
                };

                let detail = if passed {
                    format!("OK: {:.3} is {} {:.3}", observed, dir_str, a.threshold)
                } else {
                    format!(
                        "FAILED: {:.3} is NOT {} {:.3}",
                        observed, dir_str, a.threshold
                    )
                };

                AssertionResult {
                    name: a.name.clone(),
                    passed,
                    observed,
                    threshold: a.threshold,
                    direction: a.direction,
                    detail,
                }
            })
            .collect();

        ABResult {
            label_a: self.label_a,
            label_b: self.label_b,
            similarity,
            spectral_a: spectral_a.into(),
            spectral_b: spectral_b.into(),
            envelope_stats_a,
            envelope_stats_b,
            rms_a,
            rms_b,
            assertions,
        }
    }
}

// ============================================================================
// Batch A/B Testing
// ============================================================================

/// A single test case in a batch comparison
pub struct ABTestCase {
    pub name: String,
    pub test: ABTest,
}

/// Results from a batch of A/B tests
#[derive(Debug)]
pub struct BatchResult {
    pub results: Vec<(String, ABResult)>,
}

impl BatchResult {
    /// Did all tests in the batch pass?
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|(_, r)| r.passed())
    }

    /// Number of tests that passed
    pub fn pass_count(&self) -> usize {
        self.results.iter().filter(|(_, r)| r.passed()).count()
    }

    /// Number of tests that failed
    pub fn fail_count(&self) -> usize {
        self.results.iter().filter(|(_, r)| !r.passed()).count()
    }

    /// Generate a summary report
    pub fn report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "=== Batch A/B Test Results: {}/{} passed ===",
            self.pass_count(),
            self.results.len()
        ));
        lines.push(String::new());

        for (name, result) in &self.results {
            let status = if result.passed() { "PASS" } else { "FAIL" };
            lines.push(format!(
                "  [{}] {} -- overall: {:.1}%, rhythm: {:.1}%, spectral: {:.1}%",
                status,
                name,
                result.similarity.overall * 100.0,
                result.similarity.rhythm * 100.0,
                result.similarity.spectral * 100.0,
            ));
            if !result.passed() {
                for a in &result.assertions {
                    if !a.passed {
                        lines.push(format!("         ^ {}", a.detail));
                    }
                }
            }
        }

        lines.join("\n")
    }
}

/// Run a batch of A/B tests
pub fn run_batch(cases: Vec<ABTestCase>) -> BatchResult {
    let results = cases
        .into_iter()
        .map(|case| (case.name, case.test.run()))
        .collect();
    BatchResult { results }
}

// ============================================================================
// Golden Reference Management
// ============================================================================

/// Create or update a golden reference WAV from a Phonon pattern.
///
/// Renders the pattern and saves it to the specified path.
pub fn create_reference(
    code: &str,
    output_path: &str,
    sample_rate: f32,
    duration: f32,
) -> Result<(), String> {
    let audio = render_dsl_audio(code, sample_rate, duration)?;
    reference_audio::save_wav(output_path, &audio, sample_rate as u32)
}

/// Compare a Phonon pattern against a golden reference WAV.
///
/// Returns a detailed comparison result using envelope analysis.
pub fn compare_to_reference(
    code: &str,
    reference_path: &str,
    sample_rate: f32,
    duration: f32,
) -> Result<reference_audio::ComparisonResult, String> {
    let audio = render_dsl_audio(code, sample_rate, duration)?;
    reference_audio::compare_against_reference(&audio, reference_path, &ComparisonConfig::default())
}

// ============================================================================
// Matrix Comparison
// ============================================================================

/// Compare a base pattern against multiple variations.
///
/// Useful for testing how transforms affect a pattern across multiple dimensions.
///
/// # Example
///
/// ```ignore
/// let results = compare_matrix(
///     "s \"bd sn hh cp\"",
///     &["fast 2", "slow 2", "rev", "every 2 rev"],
///     2.0,
/// );
/// for (transform, result) in &results {
///     println!("{}: overall={:.1}%", transform, result.similarity.overall * 100.0);
/// }
/// ```
pub fn compare_matrix(
    base_code: &str,
    transforms: &[&str],
    duration: f32,
) -> Vec<(String, ABResult)> {
    let sample_rate = DEFAULT_SAMPLE_RATE;

    // Render base once
    let base_audio = match render_dsl_audio(base_code, sample_rate, duration) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to render base pattern: {}", e);
            return Vec::new();
        }
    };

    transforms
        .iter()
        .map(|&transform| {
            let transformed_code = format!("{} $ {}", base_code.trim(), transform);
            let transformed_audio =
                render_dsl_audio(&transformed_code, sample_rate, duration).unwrap_or_default();

            let result = ABTest::compare_audio(base_audio.clone(), transformed_audio)
                .label_a("base")
                .label_b(transform)
                .duration(duration)
                .run();

            (transform.to_string(), result)
        })
        .collect()
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Render Phonon DSL code to audio samples
fn render_dsl_audio(code: &str, sample_rate: f32, duration: f32) -> Result<Vec<f32>, String> {
    use crate::compositional_compiler::compile_program;
    use crate::compositional_parser::parse_program;

    let num_samples = (duration * sample_rate) as usize;
    let (rest, statements) = parse_program(code).map_err(|e| format!("Parse error: {:?}", e))?;
    if !rest.trim().is_empty() {
        return Err(format!("Incomplete parse, remaining: '{}'", rest));
    }
    let mut graph = compile_program(statements, sample_rate, None)
        .map_err(|e| format!("Compile error: {}", e))?;
    Ok(graph.render(num_samples))
}

/// Resolve an AudioSource to raw samples
fn resolve_audio(source: &AudioSource, sample_rate: f32, duration: f32) -> Vec<f32> {
    match source {
        AudioSource::Dsl(code) => render_dsl_audio(code, sample_rate, duration)
            .unwrap_or_else(|e| panic!("Failed to render DSL: {}", e)),
        AudioSource::Samples(samples) => samples.clone(),
        AudioSource::WavFile(path) => {
            let (samples, _sr) = reference_audio::load_wav(path)
                .unwrap_or_else(|e| panic!("Failed to load WAV '{}': {}", path, e));
            samples
        }
    }
}

/// Truncate a string for use as a label
fn truncate_label(s: &str) -> String {
    let clean = s.trim().replace('\n', " ");
    if clean.len() <= 60 {
        clean
    } else {
        format!("{}...", &clean[..57])
    }
}

/// Calculate RMS of audio buffer
fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    (audio.iter().map(|&x| x * x).sum::<f32>() / audio.len() as f32).sqrt()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn sine(freq: f32, duration: f32, sample_rate: f32) -> Vec<f32> {
        let n = (duration * sample_rate) as usize;
        (0..n)
            .map(|i| 0.5 * (2.0 * PI * freq * i as f32 / sample_rate).sin())
            .collect()
    }

    fn impulse_train(times: &[f32], duration: f32, sample_rate: f32) -> Vec<f32> {
        let n = (duration * sample_rate) as usize;
        let mut audio = vec![0.0; n];
        for &t in times {
            let idx = (t * sample_rate) as usize;
            if idx < n {
                for i in 0..100.min(n - idx) {
                    let s = i as f32 / sample_rate;
                    audio[idx + i] += 0.8 * (-s * 30.0).exp() * (2.0 * PI * 200.0 * s).sin();
                }
            }
        }
        audio
    }

    #[test]
    fn test_identical_audio_similar() {
        let audio = sine(440.0, 1.0, 44100.0);
        let result = ABTest::compare_audio(audio.clone(), audio)
            .expect_similar(0.9)
            .run();
        assert!(result.passed(), "{}", result.report());
    }

    #[test]
    fn test_different_audio_detected() {
        let a = sine(440.0, 1.0, 44100.0);
        let b = impulse_train(&[0.1, 0.3, 0.5, 0.7], 1.0, 44100.0);
        let result = ABTest::compare_audio(a, b).expect_max_similarity(0.7).run();
        assert!(result.passed(), "{}", result.report());
    }

    #[test]
    fn test_same_pitch_different_octave() {
        let a = sine(440.0, 0.5, 44100.0);
        let b = sine(880.0, 0.5, 44100.0);
        let result = ABTest::compare_audio(a, b)
            .melodic()
            .expect_chroma_similar(0.5)
            .run();
        assert!(result.passed(), "{}", result.report());
    }

    #[test]
    fn test_rms_comparison() {
        let loud = sine(440.0, 0.5, 44100.0);
        let quiet: Vec<f32> = loud.iter().map(|&x| x * 0.25).collect();
        let result = ABTest::compare_audio(loud, quiet).expect_quieter(0.5).run();
        assert!(result.passed(), "{}", result.report());
    }

    #[test]
    fn test_result_report_format() {
        let audio = sine(440.0, 0.5, 44100.0);
        let result = ABTest::compare_audio(audio.clone(), audio)
            .label_a("sine 440")
            .label_b("sine 440 (copy)")
            .expect_similar(0.9)
            .run();
        let report = result.report();
        assert!(report.contains("PASS"));
        assert!(report.contains("sine 440"));
    }

    #[test]
    fn test_multiple_assertions() {
        let a = sine(440.0, 1.0, 44100.0);
        let b = sine(440.0, 1.0, 44100.0);
        let result = ABTest::compare_audio(a, b)
            .expect_similar(0.9)
            .expect_spectral_similar(0.8)
            .expect_envelope_similar(0.8)
            .run();
        assert!(result.passed(), "{}", result.report());
        assert_eq!(result.assertions.len(), 3);
    }

    #[test]
    fn test_batch_comparison() {
        let a = sine(440.0, 0.5, 44100.0);
        let b = sine(440.0, 0.5, 44100.0);
        let c = sine(220.0, 0.5, 44100.0);

        let batch = run_batch(vec![
            ABTestCase {
                name: "same frequency".to_string(),
                test: ABTest::compare_audio(a.clone(), b).expect_similar(0.9),
            },
            ABTestCase {
                name: "different frequency".to_string(),
                test: ABTest::compare_audio(a, c).expect_spectral_different(0.8),
            },
        ]);

        assert_eq!(batch.results.len(), 2);
        let report = batch.report();
        assert!(report.contains("same frequency"));
        assert!(report.contains("different frequency"));
    }

    #[test]
    fn test_empty_assertions_always_pass() {
        let audio = sine(440.0, 0.5, 44100.0);
        let result = ABTest::compare_audio(audio.clone(), audio).run();
        assert!(result.passed()); // No assertions = pass
        assert_eq!(result.failure_count(), 0);
    }

    #[test]
    fn test_spectral_summary() {
        let audio = sine(440.0, 0.5, 44100.0);
        let result = ABTest::compare_audio(audio.clone(), audio).run();
        assert!(result.spectral_a.centroid > 0.0);
        assert!(result.spectral_b.centroid > 0.0);
    }

    #[test]
    fn test_compare_direction() {
        assert_eq!(CompareDirection::AtLeast, CompareDirection::AtLeast);
        assert_ne!(CompareDirection::AtLeast, CompareDirection::AtMost);
    }

    #[test]
    fn test_drums_config() {
        let times = vec![0.0, 0.25, 0.5, 0.75];
        let a = impulse_train(&times, 1.0, 44100.0);
        let b = impulse_train(&times, 1.0, 44100.0);
        let result = ABTest::compare_audio(a, b)
            .drums()
            .expect_rhythm_similar(0.5)
            .run();
        assert!(result.passed(), "{}", result.report());
    }

    #[test]
    fn test_truncate_label() {
        assert_eq!(truncate_label("short"), "short");
        let long = "a".repeat(100);
        let label = truncate_label(&long);
        assert!(label.len() <= 63); // 57 + "..."
        assert!(label.ends_with("..."));
    }

    #[test]
    fn test_calculate_rms() {
        let audio = vec![0.0; 100];
        assert_eq!(calculate_rms(&audio), 0.0);
        assert_eq!(calculate_rms(&[]), 0.0);

        let audio = vec![1.0; 100];
        assert!((calculate_rms(&audio) - 1.0).abs() < 0.001);
    }
}
