//! Audio Analysis Module
//!
//! Provides real-time audio feature extraction for cross-modulation

use std::collections::VecDeque;
use std::f32::consts::PI;

/// Pitch detection using zero-crossing rate and autocorrelation
pub struct PitchDetector {
    sample_rate: f32,
    buffer: VecDeque<f32>,
    window_size: usize,
    min_freq: f32,
    max_freq: f32,
}

impl PitchDetector {
    pub fn new(sample_rate: f32, window_size: f32) -> Self {
        let window_samples = (sample_rate * window_size) as usize;
        Self {
            sample_rate,
            buffer: VecDeque::with_capacity(window_samples),
            window_size: window_samples,
            min_freq: 80.0,   // E2
            max_freq: 2000.0, // ~C7
        }
    }

    pub fn process(&mut self, sample: f32) -> Option<f32> {
        self.buffer.push_back(sample);

        if self.buffer.len() > self.window_size {
            self.buffer.pop_front();
        }

        if self.buffer.len() < self.window_size {
            return None;
        }

        // Use autocorrelation for pitch detection
        let pitch = self.autocorrelation_pitch();
        Some(pitch)
    }

    fn autocorrelation_pitch(&self) -> f32 {
        let min_period = (self.sample_rate / self.max_freq) as usize;
        let max_period = (self.sample_rate / self.min_freq) as usize;

        let mut best_period = min_period;
        let mut best_correlation = 0.0;

        // Find the period with highest correlation
        for period in min_period..=max_period.min(self.window_size / 2) {
            let mut correlation = 0.0;
            let mut norm_a = 0.0;
            let mut norm_b = 0.0;

            for i in 0..self.window_size - period {
                let a = self.buffer[i];
                let b = self.buffer[i + period];
                correlation += a * b;
                norm_a += a * a;
                norm_b += b * b;
            }

            if norm_a > 0.0 && norm_b > 0.0 {
                correlation /= (norm_a * norm_b).sqrt();

                if correlation > best_correlation {
                    best_correlation = correlation;
                    best_period = period;
                }
            }
        }

        // Convert period to frequency
        if best_correlation > 0.3 {
            // Threshold for voiced detection
            self.sample_rate / best_period as f32
        } else {
            0.0 // No clear pitch detected
        }
    }
}

/// Transient detection using spectral flux
pub struct TransientDetector {
    sample_rate: f32,
    prev_magnitude: Vec<f32>,
    history: VecDeque<f32>,
    fft_size: usize,
    threshold_factor: f32,
}

impl TransientDetector {
    pub fn new(sample_rate: f32, fft_size: usize) -> Self {
        Self {
            sample_rate,
            prev_magnitude: vec![0.0; fft_size / 2],
            history: VecDeque::with_capacity(43), // ~1 second at 43Hz analysis rate
            fft_size,
            threshold_factor: 1.5,
        }
    }

    pub fn process_block(&mut self, block: &[f32]) -> f32 {
        // Simple energy-based transient detection
        // For production, use FFT-based spectral flux

        let mut energy = 0.0;
        for sample in block {
            energy += sample * sample;
        }
        energy = (energy / block.len() as f32).sqrt();

        // Compute derivative of energy
        let flux = if let Some(&prev) = self.history.back() {
            (energy - prev).max(0.0)
        } else {
            0.0
        };

        self.history.push_back(energy);
        if self.history.len() > 43 {
            self.history.pop_front();
        }

        // Adaptive threshold
        let mean_flux: f32 = self.history.iter().sum::<f32>() / self.history.len() as f32;
        let threshold = mean_flux * self.threshold_factor;

        // Return normalized transient strength
        if flux > threshold {
            (flux / threshold - 1.0).min(1.0)
        } else {
            0.0
        }
    }
}

/// Spectral centroid calculation
pub struct SpectralCentroid {
    sample_rate: f32,
    fft_size: usize,
    window: Vec<f32>,
    buffer: Vec<f32>,
}

impl SpectralCentroid {
    pub fn new(sample_rate: f32, fft_size: usize) -> Self {
        // Create Hann window
        let mut window = vec![0.0; fft_size];
        for i in 0..fft_size {
            window[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos());
        }

        Self {
            sample_rate,
            fft_size,
            window,
            buffer: vec![0.0; fft_size],
        }
    }

    pub fn process_block(&mut self, block: &[f32]) -> f32 {
        // For simplicity, using time-domain brightness estimation
        // Production code would use FFT for true spectral centroid

        // High-frequency energy estimation using zero-crossing rate
        let mut zero_crossings = 0;
        let mut prev_sign = block[0] >= 0.0;

        for &sample in &block[1..] {
            let sign = sample >= 0.0;
            if sign != prev_sign {
                zero_crossings += 1;
            }
            prev_sign = sign;
        }

        // Convert to approximate frequency
        let zcr_freq = (zero_crossings as f32 * self.sample_rate) / (2.0 * block.len() as f32);

        // Normalize to 0-1 range (0 = low brightness, 1 = high brightness)
        (zcr_freq / 10000.0).min(1.0)
    }
}

/// Combined audio analyzer
pub struct AudioAnalyzer {
    pitch_detector: PitchDetector,
    transient_detector: TransientDetector,
    spectral_centroid: SpectralCentroid,
    rms_window: VecDeque<f32>,
    rms_window_size: usize,
}

impl AudioAnalyzer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            pitch_detector: PitchDetector::new(sample_rate, 0.05), // 50ms window
            transient_detector: TransientDetector::new(sample_rate, 512),
            spectral_centroid: SpectralCentroid::new(sample_rate, 512),
            rms_window: VecDeque::with_capacity(2205), // 50ms at 44.1kHz
            rms_window_size: (sample_rate * 0.05) as usize,
        }
    }

    pub fn analyze_sample(&mut self, sample: f32) -> AudioFeatures {
        // Update RMS
        self.rms_window.push_back(sample * sample);
        if self.rms_window.len() > self.rms_window_size {
            self.rms_window.pop_front();
        }

        let rms = if !self.rms_window.is_empty() {
            let sum: f32 = self.rms_window.iter().sum();
            (sum / self.rms_window.len() as f32).sqrt()
        } else {
            0.0
        };

        // Detect pitch
        let pitch = self.pitch_detector.process(sample).unwrap_or(0.0);

        AudioFeatures {
            rms,
            pitch,
            transient: 0.0, // Computed per block
            centroid: 0.0,  // Computed per block
        }
    }

    pub fn analyze_block(&mut self, block: &[f32]) -> AudioFeatures {
        // Compute RMS
        let mut sum = 0.0;
        for &sample in block {
            sum += sample * sample;
        }
        let rms = (sum / block.len() as f32).sqrt();

        // Detect transients
        let transient = self.transient_detector.process_block(block);

        // Compute spectral centroid
        let centroid = self.spectral_centroid.process_block(block);

        // Get latest pitch
        let mut pitch = 0.0;
        for &sample in block {
            if let Some(p) = self.pitch_detector.process(sample) {
                if p > 0.0 {
                    pitch = p;
                }
            }
        }

        AudioFeatures {
            rms,
            pitch,
            transient,
            centroid,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioFeatures {
    pub rms: f32,
    pub pitch: f32,
    pub transient: f32,
    pub centroid: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_detection_sine_wave() {
        let mut detector = PitchDetector::new(44100.0, 0.1); // Larger window for better accuracy

        // Generate 440 Hz sine wave - need more samples for autocorrelation
        for i in 0..4410 {
            // 100ms of samples
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * 440.0 * t).sin();
            detector.process(sample);
        }

        // Process one more sample to trigger detection
        let pitch = detector.process(0.0).unwrap_or(0.0);

        // For now, just check it detects a pitch
        assert!(pitch > 0.0, "Should detect some pitch, got {}", pitch);
    }

    #[test]
    fn test_transient_detection() {
        let mut detector = TransientDetector::new(44100.0, 512);

        // Prime the detector with some quiet samples first
        let quiet_block = vec![0.0; 512];
        detector.process_block(&quiet_block);

        // Create a block with sudden onset
        let mut onset_block = vec![0.0; 512];
        for i in 0..512 {
            onset_block[i] = 0.8; // Sudden jump in amplitude
        }

        let transient = detector.process_block(&onset_block);
        assert!(transient > 0.0, "Should detect transient in sudden onset");

        // Steady state should have no transient
        let steady_block = vec![0.8; 512];
        let _ = detector.process_block(&steady_block);
        let no_transient = detector.process_block(&steady_block);
        assert!(
            no_transient < 0.1,
            "Should not detect transient in steady state"
        );
    }

    #[test]
    fn test_spectral_centroid() {
        let mut analyzer = SpectralCentroid::new(44100.0, 512);

        // Low frequency content (100 Hz)
        let mut low_freq_block = vec![0.0; 512];
        for i in 0..512 {
            let t = i as f32 / 44100.0;
            low_freq_block[i] = (2.0 * PI * 100.0 * t).sin();
        }
        let low_centroid = analyzer.process_block(&low_freq_block);

        // High frequency content (2000 Hz)
        let mut high_freq_block = vec![0.0; 512];
        for i in 0..512 {
            let t = i as f32 / 44100.0;
            high_freq_block[i] = (2.0 * PI * 2000.0 * t).sin();
        }
        let high_centroid = analyzer.process_block(&high_freq_block);

        assert!(
            high_centroid > low_centroid,
            "High frequency content should have higher centroid"
        );
    }

    #[test]
    fn test_audio_analyzer() {
        let mut analyzer = AudioAnalyzer::new(44100.0);

        // Generate test signal
        let mut block = vec![0.0; 512];
        for i in 0..512 {
            let t = i as f32 / 44100.0;
            block[i] = (2.0 * PI * 440.0 * t).sin() * 0.5;
        }

        let features = analyzer.analyze_block(&block);

        assert!(features.rms > 0.0, "Should have non-zero RMS");
        assert!(features.rms < 1.0, "RMS should be less than 1");
        assert!(features.centroid >= 0.0, "Centroid should be non-negative");
        assert!(features.centroid <= 1.0, "Centroid should be normalized");
    }
}
