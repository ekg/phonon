#![allow(unused_assignments, unused_mut)]
//! Audio rendering module for offline synthesis
//!
//! Provides functionality to render DSL patches to WAV files

use std::fs;
use std::path::Path;

/// Configuration for rendering audio
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Block size for processing
    pub block_size: usize,
    /// Duration in seconds
    pub duration: f32,
    /// Output gain (0.0 to 1.0)
    pub master_gain: f32,
    /// Fade in time in seconds
    pub fade_in: f32,
    /// Fade out time in seconds  
    pub fade_out: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            block_size: 512,
            duration: 1.0,
            master_gain: 1.0,
            fade_in: 0.01,
            fade_out: 0.01,
        }
    }
}

/// Renderer for DSL patches
pub struct Renderer {
    config: RenderConfig,
}

impl Renderer {
    pub fn new(config: RenderConfig) -> Self {
        Self { config }
    }

    /// Render a DSL patch to a WAV file
    pub fn render_to_file(
        &self,
        dsl_code: &str,
        output_path: &Path,
    ) -> Result<RenderStats, String> {
        // Use the working SimpleDspExecutor instead of broken SignalExecutor
        use crate::simple_dsp_executor::render_dsp_to_audio_simple;

        // Strip comments and empty lines from DSL code
        let clean_code = dsl_code
            .lines()
            .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let buffer = render_dsp_to_audio_simple(
            &clean_code,
            self.config.sample_rate as f32,
            self.config.duration,
        )?;

        // Apply master gain and get samples
        let mut all_samples = buffer.data;
        for sample in all_samples.iter_mut() {
            *sample *= self.config.master_gain;
        }

        // Trim to exact duration (shouldn't be needed but just in case)
        let total_samples = (self.config.duration * self.config.sample_rate as f32) as usize;
        all_samples.truncate(total_samples);

        // Apply fade in/out
        self.apply_fades(&mut all_samples);

        // Calculate statistics
        let stats = RenderStats::from_samples(&all_samples);

        // Write to WAV file
        self.write_wav(output_path, &all_samples)?;

        Ok(stats)
    }

    /// Render to memory (returns samples)
    pub fn render_to_buffer(&self, dsl_code: &str) -> Result<Vec<f32>, String> {
        // Use the working SimpleDspExecutor instead of broken SignalExecutor
        use crate::simple_dsp_executor::render_dsp_to_audio_simple;

        // Strip comments and empty lines from DSL code
        let clean_code = dsl_code
            .lines()
            .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let buffer = render_dsp_to_audio_simple(
            &clean_code,
            self.config.sample_rate as f32,
            self.config.duration,
        )?;

        // Apply master gain and fades
        let mut samples = buffer.data;
        for sample in samples.iter_mut() {
            *sample *= self.config.master_gain;
        }

        self.apply_fades(&mut samples);
        Ok(samples)
    }

    /// Apply fade in and fade out to samples
    fn apply_fades(&self, samples: &mut [f32]) {
        let sample_rate = self.config.sample_rate as f32;

        // Fade in
        if self.config.fade_in > 0.0 {
            let fade_in_samples = (self.config.fade_in * sample_rate) as usize;
            for i in 0..fade_in_samples.min(samples.len()) {
                let gain = i as f32 / fade_in_samples as f32;
                samples[i] *= gain;
            }
        }

        // Fade out
        if self.config.fade_out > 0.0 {
            let fade_out_samples = (self.config.fade_out * sample_rate) as usize;
            let start = samples.len().saturating_sub(fade_out_samples);
            for i in 0..fade_out_samples.min(samples.len()) {
                let idx = start + i;
                if idx < samples.len() {
                    let gain = 1.0 - (i as f32 / fade_out_samples as f32);
                    samples[idx] *= gain;
                }
            }
        }
    }

    /// Write samples to WAV file
    fn write_wav(&self, path: &Path, samples: &[f32]) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.config.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV file: {e}"))?;

        for &sample in samples {
            // Clamp to prevent overflow
            let clamped = sample.max(-1.0).min(1.0);
            let scaled = (clamped * 32767.0) as i16;
            writer
                .write_sample(scaled)
                .map_err(|e| format!("Failed to write sample: {e}"))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {e}"))?;

        Ok(())
    }
}

/// Statistics about rendered audio
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub duration: f32,
    pub sample_count: usize,
    pub rms: f32,
    pub peak: f32,
    pub dc_offset: f32,
    pub zero_crossings: usize,
}

impl RenderStats {
    fn from_samples(samples: &[f32]) -> Self {
        let sample_count = samples.len();

        // Calculate RMS
        let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
        let rms = (sum_squares / sample_count as f32).sqrt();

        // Calculate peak
        let peak = samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max);

        // Calculate DC offset
        let dc_offset = samples.iter().sum::<f32>() / sample_count as f32;

        // Count zero crossings
        let mut zero_crossings = 0;
        for i in 1..sample_count {
            if (samples[i - 1] >= 0.0) != (samples[i] >= 0.0) {
                zero_crossings += 1;
            }
        }

        Self {
            duration: sample_count as f32 / 44100.0, // Assumes 44.1kHz
            sample_count,
            rms,
            peak,
            dc_offset,
            zero_crossings,
        }
    }

    pub fn print_summary(&self) {
        println!("Render Statistics:");
        println!("  Duration:       {:.3} seconds", self.duration);
        println!("  Samples:        {}", self.sample_count);
        println!("  RMS:           {:.3}", self.rms);
        println!("  Peak:          {:.3}", self.peak);
        println!("  DC Offset:     {:.6}", self.dc_offset);
        println!("  Zero Crossings: {}", self.zero_crossings);

        // Estimate frequency from zero crossings
        if self.duration > 0.0 {
            let est_freq = self.zero_crossings as f32 / (2.0 * self.duration);
            println!("  Est. Frequency: {est_freq:.1} Hz");
        }
    }
}

/// CLI interface for rendering
pub fn render_cli(
    input: &str,
    output: &str,
    duration: Option<f32>,
    sample_rate: Option<u32>,
    gain: Option<f32>,
) -> Result<(), String> {
    // Read DSL code
    let dsl_code = if input == "-" {
        // Read from stdin
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Failed to read from stdin: {e}"))?;
        buffer
    } else if input.ends_with(".phonon") || input.ends_with(".dsl") {
        // Read from file
        fs::read_to_string(input).map_err(|e| format!("Failed to read file {input}: {e}"))?
    } else {
        // Treat as inline DSL code
        input.to_string()
    };

    // Configure renderer
    let mut config = RenderConfig::default();
    if let Some(d) = duration {
        config.duration = d;
    }
    if let Some(sr) = sample_rate {
        config.sample_rate = sr;
    }
    if let Some(g) = gain {
        config.master_gain = g;
    }

    // Create renderer
    let renderer = Renderer::new(config.clone());

    // Determine output path
    let output_path = if output == "-" {
        Path::new("/tmp/phonon_render.wav")
    } else {
        Path::new(output)
    };

    println!("Rendering DSL to {}", output_path.display());
    println!("  Duration: {} seconds", config.duration);
    println!("  Sample rate: {} Hz", config.sample_rate);
    println!("  Master gain: {:.1}", config.master_gain);
    println!();

    // Render
    let stats = renderer.render_to_file(&dsl_code, output_path)?;

    // Print statistics
    stats.print_summary();

    println!("\n✓ Render complete: {}", output_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_render_sine_wave() {
        let dsl = r#"
~osc: sin 440
out: ~osc >> mul 0.5
"#;

        let config = RenderConfig {
            duration: 1.0,
            ..Default::default()
        };

        let renderer = Renderer::new(config);
        let samples = renderer.render_to_buffer(dsl).expect("Failed to render");

        // Should have 44100 samples for 1 second at 44.1kHz
        assert_eq!(samples.len(), 44100);

        // Check RMS is reasonable for sine wave
        let stats = RenderStats::from_samples(&samples);
        assert!(
            stats.rms > 0.3 && stats.rms < 0.4,
            "RMS should be ~0.35 for 0.5 amplitude sine"
        );

        // Check frequency estimation
        let est_freq = stats.zero_crossings as f32 / 2.0;
        assert!(
            (est_freq - 440.0).abs() < 10.0,
            "Frequency should be close to 440 Hz"
        );
    }

    #[test]
    fn test_render_to_file() {
        let dsl = r#"
out: saw 220 >> lpf 1000 0.7 >> mul 0.3
"#;

        let config = RenderConfig {
            duration: 0.5,
            ..Default::default()
        };

        let renderer = Renderer::new(config);
        let output_path = PathBuf::from("/tmp/test_render.wav");

        let stats = renderer
            .render_to_file(dsl, &output_path)
            .expect("Failed to render to file");

        // Check file was created
        assert!(output_path.exists());

        // Check file size is reasonable
        let metadata = std::fs::metadata(&output_path).expect("Failed to get metadata");
        let expected_size = 44100 / 2 * 2 + 44; // ~0.5 seconds * 2 bytes per sample + WAV header
        assert!(
            metadata.len() > expected_size as u64 / 2,
            "File should have data"
        );

        // Check stats
        assert_eq!(stats.sample_count, 22050); // 0.5 seconds at 44.1kHz
        assert!(stats.peak > 0.0, "Should have non-zero peak");
    }

    #[test]
    fn test_fades() {
        let dsl = r#"
~osc: sin 440
out: ~osc >> mul 1.0
"#;

        let config = RenderConfig {
            duration: 0.1,
            fade_in: 0.01,
            fade_out: 0.01,
            ..Default::default()
        };

        let renderer = Renderer::new(config);
        let samples = renderer.render_to_buffer(dsl).expect("Failed to render");

        // First samples should be faded in (near zero)
        assert!(samples[0].abs() < 0.01, "First sample should be near zero");
        assert!(samples[10].abs() < 0.1, "Early samples should be faded");

        // Last samples should be faded out
        let last_idx = samples.len() - 1;
        assert!(
            samples[last_idx].abs() < 0.01,
            "Last sample should be near zero"
        );
        assert!(
            samples[last_idx - 10].abs() < 0.1,
            "Late samples should be faded"
        );

        // Middle should have significant amplitude (sine wave won't be exactly 1.0)
        let mid_idx = samples.len() / 2;
        let mid_section_max = samples[mid_idx - 50..mid_idx + 50]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f32, f32::max);
        assert!(
            mid_section_max > 0.5,
            "Middle section should have significant amplitude"
        );
    }
}
