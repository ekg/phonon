#![allow(dead_code, unused_imports)]
//! Audio Similarity Test Helpers
//!
//! Provides helper functions for using audio similarity scoring in Phonon tests.
//! These integrate with the existing pattern verification infrastructure.

use phonon::audio_similarity::{
    AudioSimilarityScorer, RhythmPattern, SimilarityConfig, SimilarityResult,
};

/// Assert that two audio buffers are rhythmically similar
///
/// This is useful for verifying that pattern transformations like `fast`, `slow`,
/// and `every` produce the expected rhythmic structure.
///
/// # Arguments
/// * `audio_a` - First audio buffer
/// * `audio_b` - Second audio buffer
/// * `sample_rate` - Sample rate in Hz
/// * `min_similarity` - Minimum rhythm similarity (0.0-1.0)
/// * `message` - Error message if assertion fails
pub fn assert_rhythm_similar(
    audio_a: &[f32],
    audio_b: &[f32],
    sample_rate: f32,
    min_similarity: f32,
    message: &str,
) {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::drums());
    let result = scorer.compare(audio_a, audio_b);

    assert!(
        result.rhythm >= min_similarity,
        "{}: Rhythm similarity {:.2}% < required {:.2}%\n  Details: {:?}",
        message,
        result.rhythm * 100.0,
        min_similarity * 100.0,
        result
    );
}

/// Assert that two audio buffers have similar spectral content
///
/// Useful for verifying filter effects and timbral transformations.
///
/// # Arguments
/// * `audio_a` - First audio buffer
/// * `audio_b` - Second audio buffer
/// * `sample_rate` - Sample rate in Hz
/// * `min_similarity` - Minimum spectral similarity (0.0-1.0)
/// * `message` - Error message if assertion fails
pub fn assert_spectral_similar(
    audio_a: &[f32],
    audio_b: &[f32],
    sample_rate: f32,
    min_similarity: f32,
    message: &str,
) {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    let result = scorer.compare(audio_a, audio_b);

    assert!(
        result.spectral >= min_similarity,
        "{}: Spectral similarity {:.2}% < required {:.2}%\n  Details: {:?}",
        message,
        result.spectral * 100.0,
        min_similarity * 100.0,
        result
    );
}

/// Assert that two audio buffers have similar melodic/harmonic content
///
/// Useful for verifying pitch-related transformations and note sequences.
///
/// # Arguments
/// * `audio_a` - First audio buffer
/// * `audio_b` - Second audio buffer
/// * `sample_rate` - Sample rate in Hz
/// * `min_similarity` - Minimum chroma similarity (0.0-1.0)
/// * `message` - Error message if assertion fails
pub fn assert_chroma_similar(
    audio_a: &[f32],
    audio_b: &[f32],
    sample_rate: f32,
    min_similarity: f32,
    message: &str,
) {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::melodic());
    let result = scorer.compare(audio_a, audio_b);

    assert!(
        result.chroma >= min_similarity,
        "{}: Chroma similarity {:.2}% < required {:.2}%\n  Details: {:?}",
        message,
        result.chroma * 100.0,
        min_similarity * 100.0,
        result
    );
}

/// Assert that two audio buffers are overall similar
///
/// Uses weighted combination of rhythm, spectral, chroma, and envelope similarity.
///
/// # Arguments
/// * `audio_a` - First audio buffer
/// * `audio_b` - Second audio buffer
/// * `sample_rate` - Sample rate in Hz
/// * `min_similarity` - Minimum overall similarity (0.0-1.0)
/// * `message` - Error message if assertion fails
pub fn assert_audio_similar(
    audio_a: &[f32],
    audio_b: &[f32],
    sample_rate: f32,
    min_similarity: f32,
    message: &str,
) {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    let result = scorer.compare(audio_a, audio_b);

    assert!(
        result.overall >= min_similarity,
        "{}: Overall similarity {:.2}% < required {:.2}%\n{}",
        message,
        result.overall * 100.0,
        min_similarity * 100.0,
        result.description()
    );
}

/// Assert that two audio buffers are NOT similar
///
/// Useful for verifying that transformations actually change the audio.
///
/// # Arguments
/// * `audio_a` - First audio buffer
/// * `audio_b` - Second audio buffer
/// * `sample_rate` - Sample rate in Hz
/// * `max_similarity` - Maximum overall similarity (0.0-1.0)
/// * `message` - Error message if assertion fails
pub fn assert_audio_different(
    audio_a: &[f32],
    audio_b: &[f32],
    sample_rate: f32,
    max_similarity: f32,
    message: &str,
) {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    let result = scorer.compare(audio_a, audio_b);

    assert!(
        result.overall <= max_similarity,
        "{}: Overall similarity {:.2}% > maximum {:.2}% (should be different)\n{}",
        message,
        result.overall * 100.0,
        max_similarity * 100.0,
        result.description()
    );
}

/// Compare audio and get detailed similarity result
///
/// Returns the full SimilarityResult for custom analysis.
pub fn compare_audio(audio_a: &[f32], audio_b: &[f32], sample_rate: f32) -> SimilarityResult {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    scorer.compare(audio_a, audio_b)
}

/// Compare audio for drums/percussion with appropriate config
pub fn compare_drums(audio_a: &[f32], audio_b: &[f32], sample_rate: f32) -> SimilarityResult {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::drums());
    scorer.compare(audio_a, audio_b)
}

/// Compare audio for melodic content with appropriate config
pub fn compare_melodic(audio_a: &[f32], audio_b: &[f32], sample_rate: f32) -> SimilarityResult {
    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::melodic());
    scorer.compare(audio_a, audio_b)
}

/// Verify onset count is within expected range
///
/// # Arguments
/// * `audio` - Audio buffer to analyze
/// * `sample_rate` - Sample rate in Hz
/// * `expected_min` - Minimum expected onset count
/// * `expected_max` - Maximum expected onset count
/// * `message` - Error message if assertion fails
pub fn assert_onset_count_in_range(
    audio: &[f32],
    sample_rate: f32,
    expected_min: usize,
    expected_max: usize,
    message: &str,
) {
    use phonon::audio_similarity::detect_onsets;

    let onsets = detect_onsets(audio, sample_rate);

    assert!(
        onsets.len() >= expected_min && onsets.len() <= expected_max,
        "{}: Onset count {} not in range [{}, {}]",
        message,
        onsets.len(),
        expected_min,
        expected_max
    );
}

/// Extract onset times from audio
///
/// Returns a vector of onset times in seconds.
pub fn get_onset_times(audio: &[f32], sample_rate: f32) -> Vec<f64> {
    use phonon::audio_similarity::detect_onsets;
    detect_onsets(audio, sample_rate)
        .iter()
        .map(|o| o.time)
        .collect()
}

/// Verify rhythm pattern matches expected intervals
///
/// # Arguments
/// * `audio` - Audio buffer to analyze
/// * `sample_rate` - Sample rate in Hz
/// * `expected_intervals` - Expected inter-onset intervals in seconds
/// * `tolerance` - Tolerance for interval matching (0.0-1.0, normalized)
/// * `message` - Error message if assertion fails
pub fn assert_rhythm_matches(
    audio: &[f32],
    sample_rate: f32,
    expected_intervals: &[f64],
    tolerance: f64,
    message: &str,
) {
    use phonon::audio_similarity::{compare_rhythm_patterns, extract_rhythm_pattern};

    let pattern = extract_rhythm_pattern(audio, sample_rate);
    let similarity = compare_rhythm_patterns(expected_intervals, &pattern.intervals, tolerance);

    assert!(
        similarity >= 0.7,
        "{}: Rhythm pattern doesn't match (similarity: {:.2}%)\n  Expected: {:?}\n  Got: {:?}",
        message,
        similarity * 100.0,
        expected_intervals,
        pattern.intervals
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine(freq: f32, duration: f32, sample_rate: f32) -> Vec<f32> {
        let num_samples = (duration * sample_rate) as usize;
        (0..num_samples)
            .map(|i| 0.5 * (2.0 * PI * freq * i as f32 / sample_rate).sin())
            .collect()
    }

    fn generate_impulses(times: &[f32], duration: f32, sample_rate: f32) -> Vec<f32> {
        let num_samples = (duration * sample_rate) as usize;
        let mut audio = vec![0.0; num_samples];

        for &time in times {
            let idx = (time * sample_rate) as usize;
            if idx < num_samples {
                for i in 0..100.min(num_samples - idx) {
                    let t = i as f32 / sample_rate;
                    audio[idx + i] += 0.8 * (-t * 30.0).exp() * (2.0 * PI * 200.0 * t).sin();
                }
            }
        }

        audio
    }

    #[test]
    fn test_assert_audio_similar() {
        let sample_rate = 44100.0;
        let audio = generate_sine(440.0, 1.0, sample_rate);

        // Should not panic for identical audio
        assert_audio_similar(&audio, &audio, sample_rate, 0.9, "Identical audio");
    }

    #[test]
    fn test_assert_rhythm_similar() {
        let sample_rate = 44100.0;
        let times: Vec<f32> = (0..4).map(|i| i as f32 * 0.25).collect();
        let audio1 = generate_impulses(&times, 1.2, sample_rate);
        let audio2 = generate_impulses(&times, 1.2, sample_rate);

        assert_rhythm_similar(&audio1, &audio2, sample_rate, 0.5, "Same rhythm pattern");
    }

    #[test]
    fn test_compare_audio() {
        let sample_rate = 44100.0;
        let audio = generate_sine(440.0, 0.5, sample_rate);

        let result = compare_audio(&audio, &audio, sample_rate);
        assert!(result.overall >= 0.9);
    }

    #[test]
    fn test_get_onset_times() {
        let sample_rate = 44100.0;
        let times: Vec<f32> = (0..4).map(|i| 0.1 + i as f32 * 0.25).collect();
        let audio = generate_impulses(&times, 1.2, sample_rate);

        let onset_times = get_onset_times(&audio, sample_rate);

        // Should detect some onsets
        assert!(!onset_times.is_empty(), "Should detect onsets");
    }
}
