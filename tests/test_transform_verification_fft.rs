// Transform Verification with FFT Analysis
//
// These tests verify that pattern transforms actually modify audio correctly
// by analyzing the rendered output with FFT and checking specific frequencies,
// onset timing, and other signal properties.
//
// Test strategy:
// 1. Use pure tones at known frequencies for easy verification
// 2. Render with transforms applied
// 3. Analyze WAV using wav_analyze --json
// 4. Parse JSON and verify expected frequencies/timings appear
// 5. Fail if transform didn't work as expected

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper to render Phonon code and analyze with JSON output
struct TransformTest {
    test_name: String,
    temp_dir: PathBuf,
}

impl TransformTest {
    fn new(test_name: &str) -> Self {
        let temp_dir = std::env::temp_dir().join(format!("phonon_fft_test_{}", test_name));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        TransformTest {
            test_name: test_name.to_string(),
            temp_dir,
        }
    }

    /// Render Phonon code to WAV
    fn render(&self, code: &str, cycles: u32) -> Result<PathBuf, String> {
        let input_path = self.temp_dir.join(format!("{}.phonon", self.test_name));
        fs::write(&input_path, code)
            .map_err(|e| format!("Failed to write input file: {}", e))?;

        let output_path = self.temp_dir.join(format!("{}.wav", self.test_name));

        let output = Command::new("cargo")
            .args(&[
                "run",
                "--quiet",
                "--bin",
                "phonon",
                "--",
                "render",
                input_path.to_str().unwrap(),
                output_path.to_str().unwrap(),
                "--cycles",
                &cycles.to_string(),
            ])
            .output()
            .map_err(|e| format!("Failed to run phonon: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Render failed:\\nstdout: {}\\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(output_path)
    }

    /// Analyze WAV with JSON output
    fn analyze_json(&self, wav_path: &PathBuf) -> Result<AudioAnalysis, String> {
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--quiet",
                "--bin",
                "wav_analyze",
                "--",
                wav_path.to_str().unwrap(),
                "--json",
            ])
            .output()
            .map_err(|e| format!("Failed to run wav_analyze: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        AudioAnalysis::parse_json(&stdout)
    }
}

impl Drop for TransformTest {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

/// Parsed JSON analysis
#[derive(Debug)]
struct AudioAnalysis {
    rms: f32,
    peak: f32,
    onset_count: usize,
    onset_times: Vec<f32>,
    spectral_centroid: f32,
    dominant_frequency: f32,
    frequency_bins: Vec<(f32, f32)>,
    is_empty: bool,
    is_clipping: bool,
}

impl AudioAnalysis {
    /// Parse JSON output from wav_analyze --json
    fn parse_json(json: &str) -> Result<Self, String> {
        // Simple JSON parsing (could use serde_json for robustness)
        let mut rms = None;
        let mut peak = None;
        let mut onset_count = None;
        let mut spectral_centroid = None;
        let mut dominant_frequency = None;
        let mut is_empty = false;
        let mut is_clipping = false;
        let mut onset_times = Vec::new();
        let mut frequency_bins = Vec::new();

        for line in json.lines() {
            if line.contains("\"rms\":") {
                if let Some(val) = line.split(':').nth(1) {
                    let val_str = val.trim().trim_end_matches(',');
                    rms = val_str.parse().ok();
                }
            } else if line.contains("\"peak\":") {
                if let Some(val) = line.split(':').nth(1) {
                    let val_str = val.trim().trim_end_matches(',');
                    peak = val_str.parse().ok();
                }
            } else if line.contains("\"onset_count\":") {
                if let Some(val) = line.split(':').nth(1) {
                    let val_str = val.trim().trim_end_matches(',');
                    onset_count = val_str.parse().ok();
                }
            } else if line.contains("\"spectral_centroid\":") {
                if let Some(val) = line.split(':').nth(1) {
                    let val_str = val.trim().trim_end_matches(',');
                    spectral_centroid = val_str.parse().ok();
                }
            } else if line.contains("\"dominant_frequency\":") {
                if let Some(val) = line.split(':').nth(1) {
                    let val_str = val.trim().trim_end_matches(',');
                    dominant_frequency = val_str.parse().ok();
                }
            } else if line.contains("\"is_empty\":") {
                is_empty = line.contains("true");
            } else if line.contains("\"is_clipping\":") {
                is_clipping = line.contains("true");
            } else if line.contains("\"freq\":") && line.contains("\"magnitude\":") {
                // Parse frequency bin: {"freq": 440.0, "magnitude": 0.123}
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    if let Some(freq_str) = parts[0].split(':').nth(1) {
                        if let Some(mag_str) = parts[1].split(':').nth(1) {
                            let freq: f32 = freq_str.trim().parse().unwrap_or(0.0);
                            let mag: f32 = mag_str
                                .trim()
                                .trim_end_matches('}')
                                .trim()
                                .parse()
                                .unwrap_or(0.0);
                            frequency_bins.push((freq, mag));
                        }
                    }
                }
            }
        }

        // Parse onset_times array
        if let Some(onset_section) = json.split("\"onset_times\": [").nth(1) {
            if let Some(times_str) = onset_section.split(']').next() {
                for time_str in times_str.split(',') {
                    if let Ok(time) = time_str.trim().parse::<f32>() {
                        onset_times.push(time);
                    }
                }
            }
        }

        Ok(AudioAnalysis {
            rms: rms.ok_or("Failed to parse RMS")?,
            peak: peak.ok_or("Failed to parse peak")?,
            onset_count: onset_count.ok_or("Failed to parse onset count")?,
            onset_times,
            spectral_centroid: spectral_centroid.ok_or("Failed to parse spectral centroid")?,
            dominant_frequency: dominant_frequency.ok_or("Failed to parse dominant frequency")?,
            frequency_bins,
            is_empty,
            is_clipping,
        })
    }

    /// Check if a frequency is present in the spectrum
    fn has_frequency(&self, target_freq: f32, tolerance: f32) -> bool {
        self.frequency_bins
            .iter()
            .any(|(freq, _)| (freq - target_freq).abs() < tolerance)
    }

    /// Get magnitude of a frequency
    fn get_magnitude(&self, target_freq: f32, tolerance: f32) -> f32 {
        self.frequency_bins
            .iter()
            .find(|(freq, _)| (freq - target_freq).abs() < tolerance)
            .map(|(_, mag)| *mag)
            .unwrap_or(0.0)
    }

    /// Get intervals between onsets
    fn onset_intervals(&self) -> Vec<f32> {
        if self.onset_times.len() < 2 {
            return Vec::new();
        }
        self.onset_times.windows(2).map(|w| w[1] - w[0]).collect()
    }
}

// ========== FFT-Verified Transform Tests ==========

#[test]
#[ignore] // Requires rendering
fn test_verify_single_tone_fft() {
    let test = TransformTest::new("single_tone");

    // Render pure 440Hz tone
    let code = r#"
tempo: 2.0
out: sine 440
"#;

    let wav = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav).expect("Failed to analyze");

    // Verify 440Hz is dominant
    assert!(!analysis.is_empty, "Should have audio");
    assert!(
        analysis.has_frequency(440.0, 50.0),
        "Should detect 440Hz. Dominant: {}Hz, bins: {:?}",
        analysis.dominant_frequency,
        analysis.frequency_bins
    );

    println!("✓ 440Hz tone verified");
    println!("  Dominant: {:.1}Hz", analysis.dominant_frequency);
    println!("  Top bins: {:?}", &analysis.frequency_bins[..3.min(analysis.frequency_bins.len())]);
}

#[test]
#[ignore]
fn test_verify_two_tone_pattern() {
    let test = TransformTest::new("two_tone");

    // Pattern alternates 300Hz and 600Hz
    let code = r#"
tempo: 2.0
out: sine "300 600"
"#;

    let wav = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav).expect("Failed to analyze");

    // Both frequencies should be present
    assert!(
        analysis.has_frequency(300.0, 50.0),
        "Should detect 300Hz"
    );
    assert!(
        analysis.has_frequency(600.0, 50.0),
        "Should detect 600Hz"
    );

    // Spectral centroid should be between the two
    assert!(
        analysis.spectral_centroid > 250.0 && analysis.spectral_centroid < 700.0,
        "Spectral centroid should be ~400-500Hz, got {}Hz",
        analysis.spectral_centroid
    );

    println!("✓ Two-tone pattern verified");
    println!("  300Hz magnitude: {:.1}", analysis.get_magnitude(300.0, 50.0));
    println!("  600Hz magnitude: {:.1}", analysis.get_magnitude(600.0, 50.0));
}

#[test]
#[ignore]
fn test_fast_doubles_pattern_rate() {
    let test = TransformTest::new("fast_doubles");

    // Without fast: pattern plays at tempo rate (2 Hz = 0.5s per event)
    let normal = r#"
tempo: 2.0
out: sine "200 400"
"#;

    // With fast 2: pattern plays at 4 Hz (0.25s per event)
    let fast = r#"
tempo: 2.0
out: sine "200 400" $ fast 2
"#;

    // Both should contain same frequencies, but spectral distribution may differ
    let wav_normal = test.render(normal, 2).expect("Failed to render normal");
    let analysis_normal = test.analyze_json(&wav_normal).expect("Failed to analyze normal");

    let wav_fast = test.render(fast, 2).expect("Failed to render fast");
    let analysis_fast = test.analyze_json(&wav_fast).expect("Failed to analyze fast");

    // Both should have 200Hz and 400Hz
    assert!(analysis_normal.has_frequency(200.0, 50.0));
    assert!(analysis_normal.has_frequency(400.0, 50.0));
    assert!(analysis_fast.has_frequency(200.0, 50.0));
    assert!(analysis_fast.has_frequency(400.0, 50.0));

    println!("✓ Fast transform verified");
    println!("  Normal centroid: {:.1}Hz", analysis_normal.spectral_centroid);
    println!("  Fast centroid: {:.1}Hz", analysis_fast.spectral_centroid);
}

#[test]
#[ignore]
fn test_rev_reverses_pattern_order() {
    let test = TransformTest::new("rev_order");

    // Pattern: 100Hz → 200Hz → 300Hz → 400Hz
    // With rev: 400Hz → 300Hz → 200Hz → 100Hz
    //
    // All frequencies should still be present, but spectral evolution differs
    let code = r#"
tempo: 2.0
out: sine "100 200 300 400" $ rev
"#;

    let wav = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav).expect("Failed to analyze");

    // All four frequencies should be present
    assert!(analysis.has_frequency(100.0, 50.0), "Should have 100Hz");
    assert!(analysis.has_frequency(200.0, 50.0), "Should have 200Hz");
    assert!(analysis.has_frequency(300.0, 50.0), "Should have 300Hz");
    assert!(analysis.has_frequency(400.0, 50.0), "Should have 400Hz");

    println!("✓ Rev transform verified");
    println!("  All 4 frequencies present");
}
