/// Enhanced Audio Verification Module
/// Uses spectrum-analyzer for professional FFT with Hann windowing
///
/// CRITICAL: "We are deaf" - can only verify audio through analysis tools

use hound;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::scaling::divide_by_N;

#[derive(Debug)]
pub struct EnhancedAudioAnalysis {
    pub rms: f32,
    pub peak: f32,
    pub dominant_frequency: f32,
    pub spectral_centroid: f32,
    pub spectral_spread: f32,
    pub spectral_flux: f32,
    pub onset_count: usize,
    pub is_empty: bool,
    pub is_clipping: bool,
}

/// Read WAV file and perform enhanced analysis
pub fn analyze_wav_enhanced(wav_path: &str) -> Result<EnhancedAudioAnalysis, String> {
    let mut reader = hound::WavReader::open(wav_path)
        .map_err(|e| format!("Failed to open WAV file: {}", e))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    // Read all samples
    let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
        reader.samples::<f32>().map(|s| s.unwrap()).collect()
    } else {
        reader.samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect()
    };

    if samples.is_empty() {
        return Err("No samples in WAV file".to_string());
    }

    // Basic statistics
    let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    let is_empty = rms < 0.0001 && peak < 0.001;
    let is_clipping = peak >= 0.999;

    // Enhanced FFT analysis
    let (dominant_frequency, spectral_centroid, spectral_spread) =
        analyze_spectrum_enhanced(&samples, sample_rate)?;

    // Calculate spectral flux (change in spectrum over time)
    let spectral_flux = calculate_spectral_flux(&samples, sample_rate)?;

    // Simple onset detection via energy envelope
    let onset_count = detect_onsets_simple(&samples, sample_rate);

    Ok(EnhancedAudioAnalysis {
        rms,
        peak,
        dominant_frequency,
        spectral_centroid,
        spectral_spread,
        spectral_flux,
        onset_count,
        is_empty,
        is_clipping,
    })
}

/// Enhanced spectrum analysis using spectrum-analyzer with Hann window
fn analyze_spectrum_enhanced(samples: &[f32], sample_rate: u32) -> Result<(f32, f32, f32), String> {
    // Use 8192 samples for high frequency resolution
    let fft_size = samples.len().min(8192).next_power_of_two();
    let chunk = &samples[..fft_size.min(samples.len())];

    // Apply Hann window
    let windowed_samples = hann_window(chunk);

    // Perform FFT
    let spectrum = samples_fft_to_spectrum(
        &windowed_samples,
        sample_rate,
        FrequencyLimit::All,
        Some(&divide_by_N),
    ).map_err(|e| format!("FFT error: {:?}", e))?;

    // Analyze spectrum
    let mut max_magnitude = 0.0_f32;
    let mut dominant_freq = 0.0_f32;
    let mut total_magnitude = 0.0_f32;
    let mut weighted_freq_sum = 0.0_f32;
    let mut weighted_freq_sq_sum = 0.0_f32;

    for (freq, magnitude) in spectrum.data().iter() {
        let mag = magnitude.val();
        total_magnitude += mag;
        weighted_freq_sum += freq.val() * mag;
        weighted_freq_sq_sum += freq.val() * freq.val() * mag;

        if mag > max_magnitude {
            max_magnitude = mag;
            dominant_freq = freq.val();
        }
    }

    // Spectral centroid (brightness)
    let spectral_centroid = if total_magnitude > 0.0 {
        weighted_freq_sum / total_magnitude
    } else {
        0.0
    };

    // Spectral spread (variance)
    let spectral_spread = if total_magnitude > 0.0 {
        let variance = (weighted_freq_sq_sum / total_magnitude) - (spectral_centroid * spectral_centroid);
        variance.max(0.0).sqrt()
    } else {
        0.0
    };

    Ok((dominant_freq, spectral_centroid, spectral_spread))
}

/// Calculate spectral flux (measure of spectral change over time)
fn calculate_spectral_flux(samples: &[f32], sample_rate: u32) -> Result<f32, String> {
    let window_size = 2048;
    let hop_size = 512;

    if samples.len() < window_size * 2 {
        return Ok(0.0); // Not enough samples
    }

    let mut flux_values = Vec::new();
    let mut prev_spectrum: Option<Vec<f32>> = None;

    for i in (0..samples.len() - window_size).step_by(hop_size) {
        let chunk = &samples[i..i + window_size];
        let windowed = hann_window(chunk);

        let spectrum = samples_fft_to_spectrum(
            &windowed,
            sample_rate,
            FrequencyLimit::Range(20.0, 10000.0),
            Some(&divide_by_N),
        ).ok();

        if let Some(spec) = spectrum {
            let magnitudes: Vec<f32> = spec.data().iter()
                .map(|(_, mag)| mag.val())
                .collect();

            if let Some(prev) = &prev_spectrum {
                // Calculate flux as sum of squared differences
                let flux: f32 = magnitudes.iter().zip(prev.iter())
                    .map(|(curr, prev)| (curr - prev).max(0.0).powi(2))
                    .sum();
                flux_values.push(flux);
            }

            prev_spectrum = Some(magnitudes);
        }
    }

    // Return mean flux
    if flux_values.is_empty() {
        Ok(0.0)
    } else {
        Ok(flux_values.iter().sum::<f32>() / flux_values.len() as f32)
    }
}

/// Simple onset detection via energy envelope
fn detect_onsets_simple(samples: &[f32], sample_rate: u32) -> usize {
    let window_ms = 5.0;
    let window_samples = ((sample_rate as f32 * window_ms) / 1000.0) as usize;
    let min_distance_samples = (sample_rate as f32 * 0.05) as usize; // 50ms

    let mut envelope = Vec::new();
    for chunk in samples.chunks(window_samples) {
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        envelope.push(rms);
    }

    // Adaptive threshold
    let mean = envelope.iter().sum::<f32>() / envelope.len() as f32;
    let std_dev = (envelope.iter()
        .map(|e| (e - mean).powi(2))
        .sum::<f32>() / envelope.len() as f32)
        .sqrt();
    let threshold = mean + std_dev * 1.5;

    // Count onsets
    let mut onset_count = 0;
    let mut last_onset = 0;
    for (i, &energy) in envelope.iter().enumerate() {
        if energy > threshold && (i - last_onset) > (min_distance_samples / window_samples) {
            onset_count += 1;
            last_onset = i;
        }
    }

    onset_count
}

/// Verify audio exists (not silence)
pub fn verify_audio_exists_enhanced(wav_path: &str) -> Result<EnhancedAudioAnalysis, String> {
    let analysis = analyze_wav_enhanced(wav_path)?;

    if analysis.is_empty {
        return Err(format!(
            "Audio is silent! RMS: {:.6}, Peak: {:.6}",
            analysis.rms, analysis.peak
        ));
    }

    Ok(analysis)
}

/// Verify oscillator frequency with enhanced FFT
pub fn verify_oscillator_frequency_enhanced(
    wav_path: &str,
    expected_freq: f32,
    tolerance_hz: f32,
) -> Result<(), String> {
    let analysis = analyze_wav_enhanced(wav_path)?;

    if analysis.is_empty {
        return Err("Audio is silent - oscillator not working".to_string());
    }

    let freq_diff = (analysis.dominant_frequency - expected_freq).abs();
    if freq_diff > tolerance_hz {
        return Err(format!(
            "Frequency mismatch! Expected: {:.1} Hz, Got: {:.1} Hz (diff: {:.1} Hz)",
            expected_freq, analysis.dominant_frequency, freq_diff
        ));
    }

    Ok(())
}

/// Verify LFO modulation using spectral flux
pub fn verify_lfo_modulation_enhanced(wav_path: &str, min_flux: f32) -> Result<(), String> {
    let analysis = analyze_wav_enhanced(wav_path)?;

    if analysis.is_empty {
        return Err("Audio is silent".to_string());
    }

    // LFO modulation should create spectral changes
    if analysis.spectral_flux < min_flux {
        return Err(format!(
            "LFO modulation too weak! Spectral flux: {:.6} (expected >= {:.6})",
            analysis.spectral_flux, min_flux
        ));
    }

    if analysis.spectral_spread < 100.0 {
        return Err(format!(
            "LFO modulation not significant! Spectral spread: {:.1} Hz (expected >= 100 Hz)",
            analysis.spectral_spread
        ));
    }

    Ok(())
}

/// Verify sample playback using PEAK detection
/// Works for both sparse and dense patterns
/// Sparse patterns have low RMS but correct peak amplitude
pub fn verify_sample_playback(wav_path: &str, min_peak: f32) -> Result<EnhancedAudioAnalysis, String> {
    let analysis = analyze_wav_enhanced(wav_path)?;

    // Check peak amplitude (works for sparse and dense patterns)
    if analysis.peak < min_peak {
        return Err(format!(
            "Sample peak too low! Peak: {:.6} (expected >= {:.6})",
            analysis.peak, min_peak
        ));
    }

    // Sanity check: not clipping
    if analysis.is_clipping {
        return Err(format!(
            "Audio is clipping! Peak: {:.3} >= 0.999",
            analysis.peak
        ));
    }

    Ok(analysis)
}

/// Verify dense sample pattern using onset detection
/// For patterns with frequent events (bd*8, hh*16, etc.)
pub fn verify_dense_sample_pattern(
    wav_path: &str,
    min_onsets: usize,
    min_peak: f32,
) -> Result<(), String> {
    let analysis = analyze_wav_enhanced(wav_path)?;

    // Check peak first
    if analysis.peak < min_peak {
        return Err(format!(
            "Sample peak too low! Peak: {:.6} (expected >= {:.6})",
            analysis.peak, min_peak
        ));
    }

    // Check onset count for dense patterns
    if analysis.onset_count < min_onsets {
        return Err(format!(
            "Too few onsets detected! Got: {}, Expected: >= {}",
            analysis.onset_count, min_onsets
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn create_test_wav(samples: &[f32], sample_rate: u32, path: &str) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for &sample in samples {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn test_enhanced_fft_440hz() {
        // Generate 440 Hz sine wave
        let sample_rate = 44100;
        let duration = 1.0;
        let freq = 440.0;
        let num_samples = (sample_rate as f32 * duration) as usize;

        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * freq * t).sin() * 0.5
            })
            .collect();

        let path = "/tmp/test_enhanced_440hz.wav";
        create_test_wav(&samples, sample_rate, path);

        let result = verify_oscillator_frequency_enhanced(path, 440.0, 10.0);
        assert!(result.is_ok(), "Enhanced FFT should detect 440 Hz: {:?}", result);
    }

    #[test]
    fn test_spectral_flux_detection() {
        // Generate modulated sine (simulating LFO)
        let sample_rate = 44100;
        let duration = 2.0;
        let num_samples = (sample_rate as f32 * duration) as usize;

        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let carrier_freq = 440.0;
                let lfo_freq = 2.0;
                let modulation = 200.0 * (2.0 * PI * lfo_freq * t).sin();
                (2.0 * PI * (carrier_freq + modulation) * t).sin() * 0.5
            })
            .collect();

        let path = "/tmp/test_spectral_flux.wav";
        create_test_wav(&samples, sample_rate, path);

        let analysis = analyze_wav_enhanced(path).unwrap();
        println!("Spectral flux: {:.6}", analysis.spectral_flux);
        println!("Spectral spread: {:.1} Hz", analysis.spectral_spread);

        assert!(analysis.spectral_flux > 0.0001, "Should detect spectral changes");
        assert!(analysis.spectral_spread > 50.0, "Should have significant spread");
    }
}
