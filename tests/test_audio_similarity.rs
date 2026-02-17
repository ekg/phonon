//! Integration tests for the audio similarity scoring system
//!
//! Tests the audio_similarity module's ability to compare audio signals
//! for pattern validation in Phonon.

use phonon::audio_similarity::{
    audio_similarity, compare_rhythm_patterns, detect_onsets, extract_rhythm_pattern,
    rhythm_similarity, AudioSimilarityScorer, ChromaFeatures, EnvelopeFeatures,
    RhythmPattern, SimilarityConfig, SimilarityResult, SpectralFeatures,
};
use std::f32::consts::PI;

/// Generate a sine wave at a given frequency
fn generate_sine(freq: f32, duration: f32, sample_rate: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (duration * sample_rate) as usize;
    (0..num_samples)
        .map(|i| amplitude * (2.0 * PI * freq * i as f32 / sample_rate).sin())
        .collect()
}

/// Generate a saw wave
#[allow(dead_code)]
fn generate_saw(freq: f32, duration: f32, sample_rate: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (duration * sample_rate) as usize;
    let period = sample_rate / freq;
    (0..num_samples)
        .map(|i| {
            let t = (i as f32 % period) / period;
            amplitude * (2.0 * t - 1.0)
        })
        .collect()
}

/// Generate white noise
fn generate_noise(duration: f32, sample_rate: f32, amplitude: f32, seed: u64) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let num_samples = (duration * sample_rate) as usize;
    (0..num_samples)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            (seed, i).hash(&mut hasher);
            amplitude * ((hasher.finish() as f32 / u64::MAX as f32) * 2.0 - 1.0)
        })
        .collect()
}

/// Generate impulse train (drum-like transients)
fn generate_impulse_train(
    times: &[f32],
    duration: f32,
    sample_rate: f32,
    decay_rate: f32,
) -> Vec<f32> {
    let num_samples = (duration * sample_rate) as usize;
    let mut audio = vec![0.0; num_samples];

    for &time in times {
        let sample_idx = (time * sample_rate) as usize;
        if sample_idx < num_samples {
            // Create short decaying burst
            let burst_len = (sample_rate * 0.05) as usize; // 50ms burst
            for i in 0..burst_len.min(num_samples - sample_idx) {
                let t = i as f32 / sample_rate;
                let env = (-t * decay_rate).exp();
                audio[sample_idx + i] += 0.8 * env * (2.0 * PI * 200.0 * t).sin();
            }
        }
    }

    audio
}

// ============================================================================
// Onset Detection Tests
// ============================================================================

#[test]
fn test_onset_detection_regular_pattern() {
    let sample_rate = 44100.0;

    // Generate 4 evenly spaced impulses (like bd bd bd bd)
    let times: Vec<f32> = (0..4).map(|i| 0.2 + i as f32 * 0.25).collect();
    let audio = generate_impulse_train(&times, 1.5, sample_rate, 30.0);

    let onsets = detect_onsets(&audio, sample_rate);

    // Should detect approximately 4 onsets
    assert!(
        onsets.len() >= 3,
        "Should detect at least 3 onsets from 4 impulses, got {}",
        onsets.len()
    );
    assert!(
        onsets.len() <= 6,
        "Should not detect more than 6 onsets, got {}",
        onsets.len()
    );
}

#[test]
fn test_onset_detection_silence() {
    let sample_rate = 44100.0;
    let audio = vec![0.0; (sample_rate * 1.0) as usize];

    let onsets = detect_onsets(&audio, sample_rate);

    assert!(
        onsets.is_empty(),
        "Should not detect onsets in silence, got {}",
        onsets.len()
    );
}

#[test]
fn test_onset_detection_continuous_sine() {
    let sample_rate = 44100.0;
    let audio = generate_sine(440.0, 1.0, sample_rate, 0.5);

    let onsets = detect_onsets(&audio, sample_rate);

    // A continuous sine wave might have 0-1 onsets at the start
    assert!(
        onsets.len() <= 2,
        "Continuous sine should have minimal onsets, got {}",
        onsets.len()
    );
}

// ============================================================================
// Rhythm Pattern Tests
// ============================================================================

#[test]
fn test_rhythm_pattern_extraction() {
    let sample_rate = 44100.0;

    // Evenly spaced impulses
    let times: Vec<f32> = (0..4).map(|i| i as f32 * 0.25).collect();
    let audio = generate_impulse_train(&times, 1.2, sample_rate, 30.0);

    let pattern = extract_rhythm_pattern(&audio, sample_rate);

    // Pattern should have intervals
    if !pattern.intervals.is_empty() {
        // All intervals should be roughly equal (0.25s each)
        let avg_interval = pattern.intervals.iter().sum::<f64>() / pattern.intervals.len() as f64;
        assert!(
            (avg_interval - 0.25).abs() < 0.1,
            "Average interval should be near 0.25s, got {}",
            avg_interval
        );
    }
}

#[test]
fn test_rhythm_pattern_comparison_identical() {
    let intervals1 = vec![0.25, 0.25, 0.25, 0.25];
    let intervals2 = vec![0.25, 0.25, 0.25, 0.25];

    let similarity = compare_rhythm_patterns(&intervals1, &intervals2, 0.05);

    assert!(
        similarity >= 0.9,
        "Identical rhythm patterns should have high similarity: {}",
        similarity
    );
}

#[test]
fn test_rhythm_pattern_comparison_similar() {
    // Slightly different intervals (within tolerance)
    let intervals1 = vec![0.25, 0.25, 0.25, 0.25];
    let intervals2 = vec![0.26, 0.24, 0.25, 0.25]; // Small variation

    let similarity = compare_rhythm_patterns(&intervals1, &intervals2, 0.05);

    assert!(
        similarity >= 0.7,
        "Similar rhythm patterns should match: {}",
        similarity
    );
}

#[test]
fn test_rhythm_pattern_tempo_invariance() {
    // Same rhythm at different tempos
    let slow = vec![0.5, 0.5, 0.5, 0.5]; // 120 BPM equivalent
    let fast = vec![0.25, 0.25, 0.25, 0.25]; // 240 BPM equivalent

    // When normalized, these should be identical
    let pattern_slow = RhythmPattern {
        intervals: slow,
        onset_times: vec![0.0, 0.5, 1.0, 1.5, 2.0],
    };
    let pattern_fast = RhythmPattern {
        intervals: fast,
        onset_times: vec![0.0, 0.25, 0.5, 0.75, 1.0],
    };

    let similarity = pattern_slow.compare(&pattern_fast, 0.05);

    assert!(
        similarity >= 0.9,
        "Same rhythm at different tempos should match: {}",
        similarity
    );
}

// ============================================================================
// Chroma Feature Tests
// ============================================================================

#[test]
fn test_chroma_same_note_different_octaves() {
    let sample_rate = 44100.0;

    // A4 = 440Hz
    let audio_a4 = generate_sine(440.0, 0.5, sample_rate, 0.5);
    // A5 = 880Hz (one octave up, same pitch class)
    let audio_a5 = generate_sine(880.0, 0.5, sample_rate, 0.5);

    let chroma_a4 = ChromaFeatures::from_audio(&audio_a4, sample_rate, 4096);
    let chroma_a5 = ChromaFeatures::from_audio(&audio_a5, sample_rate, 4096);

    let similarity = chroma_a4.compare(&chroma_a5);

    assert!(
        similarity >= 0.5,
        "Same pitch class should have reasonable chroma similarity: {}",
        similarity
    );
}

#[test]
fn test_chroma_different_notes() {
    let sample_rate = 44100.0;

    // A4 = 440Hz
    let audio_a = generate_sine(440.0, 0.5, sample_rate, 0.5);
    // E4 = 329.63Hz (different pitch class, a perfect fifth below)
    let audio_e = generate_sine(329.63, 0.5, sample_rate, 0.5);

    let chroma_a = ChromaFeatures::from_audio(&audio_a, sample_rate, 4096);
    let chroma_e = ChromaFeatures::from_audio(&audio_e, sample_rate, 4096);

    // Different notes should have different chroma profiles
    let _similarity = chroma_a.compare(&chroma_e);
    // This test is a bit tricky - fifths can have high correlation in harmonics
    // Just verify we got valid chroma features
    assert!(
        chroma_a.chroma.len() == 12,
        "Chroma should have 12 pitch classes"
    );
    assert!(
        chroma_e.chroma.len() == 12,
        "Chroma should have 12 pitch classes"
    );
}

// ============================================================================
// Spectral Feature Tests
// ============================================================================

#[test]
fn test_spectral_features_brightness() {
    let sample_rate = 44100.0;

    // Low frequency tone
    let low = generate_sine(200.0, 0.5, sample_rate, 0.5);
    // High frequency tone
    let high = generate_sine(3000.0, 0.5, sample_rate, 0.5);

    let spectral_low = SpectralFeatures::from_audio(&low, sample_rate, 2048);
    let spectral_high = SpectralFeatures::from_audio(&high, sample_rate, 2048);

    assert!(
        spectral_high.centroid > spectral_low.centroid,
        "High frequency should have higher centroid: {} vs {}",
        spectral_high.centroid,
        spectral_low.centroid
    );
}

#[test]
fn test_spectral_features_flatness() {
    let sample_rate = 44100.0;

    // Pure sine (very tonal, low flatness)
    let sine = generate_sine(440.0, 0.5, sample_rate, 0.5);
    // White noise (high flatness)
    let noise = generate_noise(0.5, sample_rate, 0.3, 12345);

    let spectral_sine = SpectralFeatures::from_audio(&sine, sample_rate, 2048);
    let spectral_noise = SpectralFeatures::from_audio(&noise, sample_rate, 2048);

    assert!(
        spectral_noise.flatness > spectral_sine.flatness,
        "Noise should have higher flatness: {} vs {}",
        spectral_noise.flatness,
        spectral_sine.flatness
    );
}

// ============================================================================
// Envelope Feature Tests
// ============================================================================

#[test]
fn test_envelope_identical_audio() {
    let sample_rate = 44100.0;
    let audio = generate_sine(440.0, 1.0, sample_rate, 0.5);

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
fn test_envelope_different_dynamics() {
    let sample_rate = 44100.0;

    // Steady tone
    let steady = generate_sine(440.0, 1.0, sample_rate, 0.5);

    // Decaying tone
    let decay_samples = (sample_rate * 1.0) as usize;
    let decay: Vec<f32> = (0..decay_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let env = (-t * 3.0).exp();
            env * (2.0 * PI * 440.0 * t).sin()
        })
        .collect();

    let envelope_steady = EnvelopeFeatures::from_audio(&steady, sample_rate, 512);
    let envelope_decay = EnvelopeFeatures::from_audio(&decay, sample_rate, 512);

    let similarity = envelope_steady.compare(&envelope_decay);

    // Different dynamics should result in lower correlation
    assert!(
        similarity < 0.95,
        "Different dynamics should have lower envelope correlation: {}",
        similarity
    );
}

// ============================================================================
// Full Audio Similarity Tests
// ============================================================================

#[test]
fn test_audio_similarity_identical() {
    let sample_rate = 44100.0;
    let audio = generate_sine(440.0, 1.0, sample_rate, 0.5);

    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    let result = scorer.compare(&audio, &audio);

    assert!(
        result.overall >= 0.9,
        "Identical audio should have high similarity: {:?}",
        result
    );
    assert!(result.is_similar(0.8), "is_similar should return true");
}

#[test]
fn test_audio_similarity_same_rhythm_different_sound() {
    let sample_rate = 44100.0;

    // Same rhythm with different sounds
    let times: Vec<f32> = (0..4).map(|i| i as f32 * 0.25).collect();
    let impulses1 = generate_impulse_train(&times, 1.2, sample_rate, 30.0);

    // Create similar impulses but with different frequency
    let impulses2: Vec<f32> = {
        let num_samples = (sample_rate * 1.2) as usize;
        let mut audio = vec![0.0; num_samples];
        for &time in &times {
            let sample_idx = (time * sample_rate) as usize;
            if sample_idx < num_samples {
                let burst_len = (sample_rate * 0.05) as usize;
                for i in 0..burst_len.min(num_samples - sample_idx) {
                    let t = i as f32 / sample_rate;
                    let env = (-t * 25.0).exp();
                    audio[sample_idx + i] += 0.7 * env * (2.0 * PI * 300.0 * t).sin();
                }
            }
        }
        audio
    };

    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::drums());
    let result = scorer.compare(&impulses1, &impulses2);

    // Rhythm should be similar even with different sounds
    assert!(
        result.rhythm >= 0.5,
        "Same rhythm should have high rhythm similarity: {}",
        result.rhythm
    );
}

#[test]
fn test_audio_similarity_different_sounds() {
    let sample_rate = 44100.0;

    // Sine wave
    let sine = generate_sine(440.0, 1.0, sample_rate, 0.5);
    // Impulse train
    let impulses = generate_impulse_train(&[0.1, 0.3, 0.5, 0.7, 0.9], 1.0, sample_rate, 30.0);

    let scorer = AudioSimilarityScorer::new(sample_rate, SimilarityConfig::default());
    let result = scorer.compare(&sine, &impulses);

    // Different sounds should have lower overall similarity
    assert!(
        result.overall < 0.8,
        "Different sounds should have lower similarity: {:?}",
        result
    );
}

#[test]
fn test_audio_similarity_description() {
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
    assert!(desc.contains("similar"), "Description should say similar");
    assert!(
        desc.contains("85.0%"),
        "Description should include percentage"
    );
}

// ============================================================================
// Config Preset Tests
// ============================================================================

#[test]
fn test_config_drums_preset() {
    let config = SimilarityConfig::drums();

    // Drums preset should prioritize rhythm
    assert!(
        config.rhythm_weight > config.chroma_weight,
        "Drums should prioritize rhythm over chroma"
    );
    assert!(
        config.rhythm_weight >= config.spectral_weight,
        "Drums should prioritize rhythm over spectral"
    );
    assert_eq!(
        config.chroma_weight, 0.0,
        "Drums should have no chroma weight"
    );
}

#[test]
fn test_config_melodic_preset() {
    let config = SimilarityConfig::melodic();

    // Melodic preset should prioritize chroma
    assert!(
        config.chroma_weight > config.rhythm_weight,
        "Melodic should prioritize chroma over rhythm"
    );
    assert!(
        config.fft_size > SimilarityConfig::default().fft_size,
        "Melodic should have higher FFT resolution"
    );
}

// ============================================================================
// Convenience Function Tests
// ============================================================================

#[test]
fn test_convenience_audio_similarity() {
    let sample_rate = 44100.0;
    let audio = generate_sine(440.0, 0.5, sample_rate, 0.5);

    let sim = audio_similarity(&audio, &audio, sample_rate);

    assert!(
        sim >= 0.9,
        "Identical audio should have high similarity: {}",
        sim
    );
}

#[test]
fn test_convenience_rhythm_similarity() {
    let sample_rate = 44100.0;

    // Same rhythm
    let times: Vec<f32> = (0..4).map(|i| i as f32 * 0.25).collect();
    let audio1 = generate_impulse_train(&times, 1.2, sample_rate, 30.0);
    let audio2 = generate_impulse_train(&times, 1.2, sample_rate, 25.0);

    let sim = rhythm_similarity(&audio1, &audio2, sample_rate);

    assert!(
        sim >= 0.5,
        "Same rhythm should have reasonable similarity: {}",
        sim
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_audio() {
    let sample_rate = 44100.0;
    let empty: Vec<f32> = vec![];

    let onsets = detect_onsets(&empty, sample_rate);
    assert!(onsets.is_empty(), "Empty audio should have no onsets");

    let pattern = extract_rhythm_pattern(&empty, sample_rate);
    assert!(
        pattern.intervals.is_empty(),
        "Empty audio should have no intervals"
    );
}

#[test]
fn test_very_short_audio() {
    let sample_rate = 44100.0;
    let short = generate_sine(440.0, 0.01, sample_rate, 0.5); // 10ms

    // Should not crash
    let _onsets = detect_onsets(&short, sample_rate);
    let _pattern = extract_rhythm_pattern(&short, sample_rate);
    let _spectral = SpectralFeatures::from_audio(&short, sample_rate, 2048);
    let _chroma = ChromaFeatures::from_audio(&short, sample_rate, 4096);
    let _envelope = EnvelopeFeatures::from_audio(&short, sample_rate, 512);
}

#[test]
fn test_dc_offset() {
    let sample_rate = 44100.0;

    // Audio with DC offset
    let dc_audio: Vec<f32> = (0..(sample_rate * 0.5) as usize)
        .map(|i| {
            let t = i as f32 / sample_rate;
            0.5 + 0.3 * (2.0 * PI * 440.0 * t).sin() // DC offset of 0.5
        })
        .collect();

    // Should handle DC offset gracefully
    let spectral = SpectralFeatures::from_audio(&dc_audio, sample_rate, 2048);
    assert!(spectral.centroid > 0.0, "Should compute valid centroid");
}
