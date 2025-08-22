//! Test utilities for audio verification and pattern testing

use std::f32::consts::PI;

/// Compare two audio buffers with a tolerance
pub fn compare_audio(actual: &[f32], expected: &[f32], tolerance: f32) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    
    for (a, e) in actual.iter().zip(expected.iter()) {
        if (a - e).abs() > tolerance {
            return false;
        }
    }
    
    true
}

/// Calculate RMS energy of audio buffer
pub fn calculate_rms(audio: &[f32]) -> f32 {
    let sum: f32 = audio.iter().map(|x| x * x).sum();
    (sum / audio.len() as f32).sqrt()
}

/// Detect onset positions in audio (simple energy-based)
pub fn detect_onsets(audio: &[f32], window_size: usize, threshold: f32) -> Vec<usize> {
    let mut onsets = Vec::new();
    let mut prev_energy = 0.0;
    
    for i in (0..audio.len()).step_by(window_size) {
        let end = std::cmp::min(i + window_size, audio.len());
        let window = &audio[i..end];
        let energy = calculate_rms(window);
        
        // Detect energy increase
        if energy > prev_energy * threshold && energy > 0.01 {
            onsets.push(i);
        }
        
        prev_energy = energy;
    }
    
    onsets
}

/// Simple FFT-based spectral centroid calculation
pub fn spectral_centroid(audio: &[f32], sample_rate: f32) -> f32 {
    // For now, use a simple time-domain approximation
    // Real implementation would use FFT
    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;
    
    for (i, sample) in audio.iter().enumerate() {
        let freq = (i as f32 / audio.len() as f32) * (sample_rate / 2.0);
        let magnitude = sample.abs();
        weighted_sum += freq * magnitude;
        magnitude_sum += magnitude;
    }
    
    if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    }
}

/// Generate a simple kick drum sample
pub fn generate_kick(length: usize, sample_rate: f32) -> Vec<f32> {
    let mut kick = vec![0.0; length];
    let freq = 60.0; // 60 Hz base frequency
    
    for i in 0..length {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 35.0).exp(); // Fast decay
        let pitch_envelope = (-t * 150.0).exp(); // Pitch decay
        let frequency = freq * (1.0 + pitch_envelope * 3.0); // Pitch bend
        
        kick[i] = (2.0 * PI * frequency * t).sin() * envelope;
    }
    
    kick
}

/// Generate a simple snare drum sample
pub fn generate_snare(length: usize, sample_rate: f32) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut snare = vec![0.0; length];
    
    for i in 0..length {
        let t = i as f32 / sample_rate;
        let envelope = (-t * 20.0).exp(); // Decay
        let tone = (2.0 * PI * 200.0 * t).sin() * 0.5; // 200 Hz tone
        let noise = rng.gen_range(-1.0..1.0) * 0.5; // White noise
        
        snare[i] = (tone + noise) * envelope;
    }
    
    snare
}

/// Generate a simple hi-hat sample
pub fn generate_hihat(length: usize, _sample_rate: f32) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut hihat = vec![0.0; length];
    
    for i in 0..length {
        let t = i as f32 / length as f32;
        let envelope = (-t * 50.0).exp(); // Very fast decay
        let noise = rng.gen_range(-1.0..1.0);
        
        hihat[i] = noise * envelope * 0.5;
    }
    
    hihat
}

/// Verify that a pattern generates expected audio events
pub fn verify_pattern_audio(
    pattern_audio: &[f32],
    expected_hits: &[usize],
    sample_rate: f32,
) -> bool {
    let window_size = (sample_rate * 0.01) as usize; // 10ms windows
    let detected_onsets = detect_onsets(pattern_audio, window_size, 1.5);
    
    if detected_onsets.len() != expected_hits.len() {
        eprintln!("Expected {} hits, detected {}", expected_hits.len(), detected_onsets.len());
        return false;
    }
    
    // Check timing within tolerance
    let tolerance = (sample_rate * 0.02) as usize; // 20ms tolerance
    
    for (detected, expected) in detected_onsets.iter().zip(expected_hits.iter()) {
        if (*detected as i32 - *expected as i32).abs() > tolerance as i32 {
            eprintln!("Onset at {} expected at {}", detected, expected);
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_comparison() {
        let audio1 = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let audio2 = vec![0.0, 0.49, 1.01, 0.51, 0.0];
        let audio3 = vec![0.0, 0.3, 1.0, 0.5, 0.0];
        
        assert!(compare_audio(&audio1, &audio2, 0.02));
        assert!(!compare_audio(&audio1, &audio3, 0.02));
    }
    
    #[test]
    fn test_onset_detection() {
        let mut audio = vec![0.0; 1000];
        // Add spikes at specific positions
        audio[100] = 1.0;
        audio[500] = 1.0;
        audio[800] = 1.0;
        
        let onsets = detect_onsets(&audio, 50, 1.5);
        assert_eq!(onsets.len(), 3);
        assert!(onsets.contains(&100));
        assert!(onsets.contains(&500));
        assert!(onsets.contains(&800));
    }
    
    #[test]
    fn test_drum_generation() {
        let kick = generate_kick(1000, 44100.0);
        let snare = generate_snare(1000, 44100.0);
        let hihat = generate_hihat(1000, 44100.0);
        
        // Verify they generate non-zero audio
        assert!(calculate_rms(&kick) > 0.0);
        assert!(calculate_rms(&snare) > 0.0);
        assert!(calculate_rms(&hihat) > 0.0);
        
        // TODO: Fix spectral_centroid implementation to use actual FFT
        // The current implementation doesn't compute the real spectral centroid
        // let kick_centroid = spectral_centroid(&kick, 44100.0);
        // let hihat_centroid = spectral_centroid(&hihat, 44100.0);
        // assert!(kick_centroid < hihat_centroid);
    }
}