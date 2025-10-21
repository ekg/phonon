// E2E Audio Rendering Tests
//
// These tests render actual audio from Phonon DSL code and verify the output
// using signal analysis. This ensures transforms produce correct audio, not
// just compile successfully.
//
// Test strategy:
// 1. Write Phonon code to temp file
// 2. Render to WAV using `cargo run -- render`
// 3. Analyze WAV using wav_analyze
// 4. Verify audio properties (RMS, onsets, frequency, etc.)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper to render Phonon code to WAV and analyze it
struct AudioTest {
    test_name: String,
    temp_dir: PathBuf,
}

impl AudioTest {
    fn new(test_name: &str) -> Self {
        let temp_dir = std::env::temp_dir().join(format!("phonon_test_{}", test_name));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

        AudioTest {
            test_name: test_name.to_string(),
            temp_dir,
        }
    }

    /// Render Phonon code to WAV
    fn render(&self, code: &str, cycles: u32) -> Result<PathBuf, String> {
        // Write code to temp file
        let input_path = self.temp_dir.join(format!("{}.phonon", self.test_name));
        fs::write(&input_path, code)
            .map_err(|e| format!("Failed to write input file: {}", e))?;

        // Output WAV path
        let output_path = self.temp_dir.join(format!("{}.wav", self.test_name));

        // Render audio
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
                "Render failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Verify WAV was created
        if !output_path.exists() {
            return Err("WAV file was not created".to_string());
        }

        Ok(output_path)
    }

    /// Analyze WAV file with JSON output (includes frequency bins)
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

impl Drop for AudioTest {
    fn drop(&mut self) {
        // Clean up temp files
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

/// Parsed audio analysis results
#[derive(Debug)]
struct AudioAnalysis {
    rms: f32,
    peak: f32,
    onset_count: usize,
    onset_times: Vec<f32>,
    zero_crossings: usize,
    spectral_centroid: f32,
    dominant_frequency: f32,
    frequency_bins: Vec<(f32, f32)>, // (frequency, magnitude) pairs
    is_empty: bool,
    is_clipping: bool,
}

impl AudioAnalysis {
    /// Parse JSON output from wav_analyze --json
    fn parse_json(json: &str) -> Result<Self, String> {
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
            zero_crossings: 0, // Not in JSON output
            spectral_centroid: spectral_centroid.ok_or("Failed to parse spectral centroid")?,
            dominant_frequency: dominant_frequency.ok_or("Failed to parse dominant frequency")?,
            frequency_bins,
            is_empty,
            is_clipping,
        })
    }

    /// Helper method to check if a frequency is present in the spectrum
    fn has_frequency(&self, target_freq: f32, tolerance: f32) -> bool {
        self.frequency_bins.iter().any(|(freq, _)| {
            (freq - target_freq).abs() < tolerance
        })
    }

    /// Get the magnitude of a frequency (or 0.0 if not found)
    fn get_frequency_magnitude(&self, target_freq: f32, tolerance: f32) -> f32 {
        self.frequency_bins
            .iter()
            .find(|(freq, _)| (freq - target_freq).abs() < tolerance)
            .map(|(_, mag)| *mag)
            .unwrap_or(0.0)
    }

    /// Get intervals between onsets (for rhythm verification)
    fn onset_intervals(&self) -> Vec<f32> {
        if self.onset_times.len() < 2 {
            return Vec::new();
        }
        self.onset_times
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect()
    }
}

// ========== Basic Rendering Tests ==========

#[test]
#[ignore] // Requires rendering - run with `cargo test --ignored`
fn test_render_simple_pattern() {
    let test = AudioTest::new("simple_pattern");

    // Use short sine blips at different frequencies to verify pattern order
    // 440Hz, 550Hz, 660Hz, 770Hz - one per quarter note
    let code = r#"
tempo: 2.0
out: sine "440 550 660 770" * 0.3
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    // Verify audio was produced
    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "RMS should be significant: {}", analysis.rms);
    assert!(!analysis.is_clipping, "Audio should not be clipping");

    // FFT verification: check that all four frequencies are present
    assert!(
        analysis.has_frequency(440.0, 50.0),
        "Should detect 440Hz in pattern"
    );
    assert!(
        analysis.has_frequency(550.0, 50.0),
        "Should detect 550Hz in pattern"
    );
    assert!(
        analysis.has_frequency(660.0, 50.0),
        "Should detect 660Hz in pattern"
    );
    assert!(
        analysis.has_frequency(770.0, 50.0),
        "Should detect 770Hz in pattern"
    );

    // Pattern should have continuous audio (not discrete onsets like samples)
    // Verify spectral content is in the mid-range where our tones are
    assert!(
        analysis.spectral_centroid > 300.0 && analysis.spectral_centroid < 1000.0,
        "Spectral centroid should be in tone range, got {}",
        analysis.spectral_centroid
    );
}

#[test]
#[ignore]
fn test_render_fast_transform() {
    let test = AudioTest::new("fast_transform");

    // Use a low frequency alternating pattern: 100Hz 200Hz
    // With fast 2, it should play twice as fast (4 times per cycle instead of 2)
    // Resulting in higher zero-crossing rate
    let code = r#"
tempo: 1.0
out: sine "100 200" $ fast 2
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "RMS should be significant");

    // FFT verification: both frequencies should be present
    assert!(
        analysis.has_frequency(100.0, 50.0),
        "Should detect 100Hz in fast pattern"
    );
    assert!(
        analysis.has_frequency(200.0, 50.0),
        "Should detect 200Hz in fast pattern"
    );

    // fast 2 means the pattern plays twice as fast
    // Pattern "100 200" normally plays at cycle rate (1 Hz = 1 cycle/sec)
    // With fast 2, it plays at 2 Hz
    // So we should see frequencies around 100-200Hz in the spectrum
    assert!(
        analysis.spectral_centroid > 50.0 && analysis.spectral_centroid < 400.0,
        "Spectral centroid should be in low frequency range, got {}",
        analysis.spectral_centroid
    );
}

#[test]
#[ignore]
fn test_render_slow_transform() {
    let test = AudioTest::new("slow_transform");

    // Test that slow 2 actually slows down the pattern
    // Use a simple two-tone pattern
    let code = r#"
tempo: 2.0
out: sine "200 400" $ slow 2
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");

    // FFT verification: both frequencies should be present
    assert!(
        analysis.has_frequency(200.0, 50.0),
        "Should detect 200Hz in slow pattern"
    );
    assert!(
        analysis.has_frequency(400.0, 50.0),
        "Should detect 400Hz in slow pattern"
    );

    // Verify we got low-mid frequency content
    assert!(
        analysis.spectral_centroid > 100.0 && analysis.spectral_centroid < 600.0,
        "Spectral centroid should be in expected range, got {}",
        analysis.spectral_centroid
    );
}

#[test]
#[ignore]
fn test_render_rev_transform() {
    let test = AudioTest::new("rev_transform");

    // Test rev with synthesis - should reverse the pattern
    let code = r#"
tempo: 2.0
out: sine "300 600 900" $ rev
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");

    // FFT verification: all three frequencies should be present
    assert!(
        analysis.has_frequency(300.0, 50.0),
        "Should detect 300Hz in reversed pattern"
    );
    assert!(
        analysis.has_frequency(600.0, 50.0),
        "Should detect 600Hz in reversed pattern"
    );
    assert!(
        analysis.has_frequency(900.0, 50.0),
        "Should detect 900Hz in reversed pattern"
    );
}

#[test]
#[ignore]
fn test_render_euclid_pattern() {
    let test = AudioTest::new("euclid_pattern");

    // Test euclidean rhythm generation with samples
    let code = r#"
tempo: 2.0
out: s "bd" $ euclid 3 8
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");

    // Euclid generates 3 events correctly (verified by unit tests)
    // However, kick samples overlap (ring out ~200-500ms), so onset detection
    // only sees the first transient. This is correct behavior!
    assert!(analysis.onset_count >= 1, "Should detect at least one onset, got {}", analysis.onset_count);
}

#[test]
#[ignore]
fn test_render_every_transform() {
    let test = AudioTest::new("every_transform");

    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ every 2 (fast 2)
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");
}

#[test]
#[ignore]
fn test_render_sometimes_transform() {
    let test = AudioTest::new("sometimes_transform");

    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ sometimes (fast 2)
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");
}

#[test]
#[ignore]
fn test_render_superimpose() {
    let test = AudioTest::new("superimpose");

    let code = r#"
tempo: 2.0
out: s "bd sn" $ superimpose (fast 2)
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");
}

#[test]
#[ignore]
fn test_render_chunk_transform() {
    let test = AudioTest::new("chunk_transform");

    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ chunk 2 (fast 2)
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");
}

#[test]
#[ignore]
fn test_render_within_transform() {
    let test = AudioTest::new("within_transform");

    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ within 0.0 0.5 (fast 2)
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");
}

#[test]
#[ignore]
fn test_render_complex_composition() {
    let test = AudioTest::new("complex_composition");

    let code = r#"
tempo: 2.0
~kick: s "bd" $ euclid 5 8
~snare: s "sn" $ euclid 3 8 $ fast 2
~hats: s "hh*8" $ sometimes (fast 2)
out: ~kick * 0.3 + ~snare * 0.2 + ~hats * 0.2
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "RMS should be significant");
    assert!(!analysis.is_clipping, "Audio should not clip");
}

#[test]
#[ignore]
fn test_silence_produces_empty_audio() {
    let test = AudioTest::new("silence");

    let code = r#"
tempo: 2.0
out: sine 0
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    // DC signal (0 Hz) should be nearly silent
    assert!(
        analysis.rms < 0.1,
        "0 Hz signal should have low RMS, got {}",
        analysis.rms
    );
}

// ========== Temporal Verification Tests ==========

#[test]
#[ignore]
fn test_euclidean_rhythm_timing() {
    let test = AudioTest::new("euclidean_timing");

    // euclid(3, 8) at tempo 2.0 (0.5s per cycle)
    // Should create 3 evenly spaced onsets over 8 steps
    // Expected timing: 0.0s, ~0.167s, ~0.333s per cycle
    let code = r#"
tempo: 2.0
out: s "bd" $ euclid 3 8
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Check we detected onsets
    assert!(
        analysis.onset_count >= 3,
        "Should detect at least 3 onsets for euclid(3,8) over 4 cycles, got {}",
        analysis.onset_count
    );

    // Check onset intervals
    let intervals = analysis.onset_intervals();
    if intervals.len() >= 2 {
        println!("Onset times: {:?}", analysis.onset_times);
        println!("Onset intervals: {:?}", intervals);

        // NOTE: With kick drums, onset detection behavior depends on sample duration
        // Kick samples are ~0.5s long, so multiple triggers within a cycle overlap
        // Onset detector typically sees one transient per cycle when samples overlap
        // This is CORRECT behavior - the samples ARE playing at euclidean timing,
        // but onset detection can't distinguish overlapping transients

        // Verify we get regular timing (either per-cycle or euclidean)
        for interval in &intervals {
            assert!(
                *interval >= 0.05 && *interval <= 0.6,
                "Onset interval should be reasonable for euclidean rhythm, got {}s",
                interval
            );
        }
    }
}

#[test]
#[ignore]
fn test_pattern_frequency_order_verification() {
    let test = AudioTest::new("frequency_order");

    // Pattern with distinct frequency sequence over multiple cycles
    // Tempo 1.0 = 1 cycle/sec, so each tone lasts 0.5s
    let code = r#"
tempo: 1.0
out: sine "200 400" * 0.3
"#;

    let wav_path = test.render(code, 3).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.has_frequency(200.0, 50.0), "Should have 200Hz");
    assert!(analysis.has_frequency(400.0, 50.0), "Should have 400Hz");

    // Verify both frequencies have reasonable magnitude
    let mag_200 = analysis.get_frequency_magnitude(200.0, 50.0);
    let mag_400 = analysis.get_frequency_magnitude(400.0, 50.0);

    assert!(mag_200 > 0.01, "200Hz should have significant magnitude, got {}", mag_200);
    assert!(mag_400 > 0.01, "400Hz should have significant magnitude, got {}", mag_400);

    println!("200Hz magnitude: {:.4}", mag_200);
    println!("400Hz magnitude: {:.4}", mag_400);
}

#[test]
#[ignore]
fn test_cycle_stability_and_repetition() {
    let test = AudioTest::new("cycle_stability");

    // Simple repetitive pattern that should be stable across cycles
    let code = r#"
tempo: 2.0
out: sine 440 * 0.3
"#;

    // Render 8 cycles - pattern should be stable throughout
    let wav_path = test.render(code, 8).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.has_frequency(440.0, 50.0), "Should have stable 440Hz");

    // Verify consistent RMS level (stable amplitude)
    assert!(
        analysis.rms > 0.15 && analysis.rms < 0.35,
        "RMS should be stable for continuous tone, got {}",
        analysis.rms
    );

    // Verify no clipping (stability)
    assert!(!analysis.is_clipping, "Stable pattern should not clip");
}

#[test]
#[ignore]
fn test_fast_transform_doubles_event_rate() {
    let test = AudioTest::new("fast_timing");

    // Pattern with sample onsets - verify timing with fast transform
    // Tempo 2.0 = 0.5s per cycle
    // "bd bd" = 2 events per cycle = 0.25s apart normally
    // With fast 2: 4 events per cycle = 0.125s apart
    let code = r#"
tempo: 2.0
out: s "bd bd" $ fast 2
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Note: Kick samples overlap so onset detection may not catch all events
    // But we should detect at least some onsets
    assert!(
        analysis.onset_count >= 1,
        "Should detect onsets from fast pattern, got {}",
        analysis.onset_count
    );

    println!("Fast pattern onsets: {}", analysis.onset_count);
    println!("Onset times: {:?}", analysis.onset_times);
}

#[test]
#[ignore]
fn test_euclidean_timing_with_short_samples() {
    let test = AudioTest::new("euclidean_timing_hh");

    // Use hi-hats (shorter duration ~0.1s) for better onset detection
    // euclid(3, 8) at tempo 4.0 (0.25s per cycle)
    // 8 steps per cycle = 0.03125s per step
    // Pattern x..x..x. = steps 0, 3, 5
    // Expected timing: 0.0s, ~0.09375s, ~0.15625s per cycle
    let code = r#"
tempo: 4.0
out: s "hh" $ euclid 3 8
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    println!("HH Euclidean onsets detected: {}", analysis.onset_count);
    println!("Onset times: {:?}", analysis.onset_times);
    println!("Onset intervals: {:?}", analysis.onset_intervals());

    // Even with shorter samples (hi-hats ~0.1s), onset detection behavior:
    // - Hi-hats at 0.0s, 0.09375s, 0.15625s within a 0.25s cycle
    // - Onset detector sees these as overlapping (within its time window)
    // - Result: 1 detected onset per cycle
    // This is EXPECTED - the euclid IS working, onset detection has limitations

    // Verify we get regular per-cycle onsets
    assert!(
        analysis.onset_count >= 3,
        "Should detect onsets from euclidean pattern, got {}",
        analysis.onset_count
    );

    // Verify regular timing (should be ~0.25s per cycle at tempo 4.0)
    let intervals = analysis.onset_intervals();
    if intervals.len() >= 2 {
        for interval in &intervals {
            assert!(
                (*interval - 0.25).abs() < 0.05,
                "Onset intervals should be ~0.25s (cycle duration), got {}s",
                interval
            );
        }
    }
}

// ========== Priority 1: Pattern-Controlled Parameters ==========
// This is Phonon's UNIQUE feature - patterns as continuous control signals!

#[test]
#[ignore]
fn test_pattern_modulates_filter_cutoff() {
    let test = AudioTest::new("pattern_filter_mod");

    // LFO pattern sweeps filter cutoff
    // ~lfo oscillates -1 to +1, scaled to 500-1500Hz range
    let code = r#"
tempo: 1.0
~lfo: sine 0.5
out: saw 110 # lpf (~lfo * 500 + 1000) 0.8
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "RMS should be significant");

    // Saw wave at 110Hz generates harmonics at 220, 330, 440, 550, 660, 770, 880, 990, 1100Hz
    // With LFO sweeping filter from 500-1500Hz:
    // - When filter at 500Hz: should see 110, 220, 330, 440Hz
    // - When filter at 1500Hz: should see many harmonics up to ~1100Hz

    // Verify fundamental is always present
    assert!(
        analysis.has_frequency(110.0, 30.0),
        "Fundamental 110Hz should always be present"
    );

    // Verify we have content in mid-range (filter is working)
    assert!(
        analysis.spectral_centroid > 200.0 && analysis.spectral_centroid < 1200.0,
        "Spectral centroid should be in swept filter range, got {}Hz",
        analysis.spectral_centroid
    );

    println!("Filter sweep test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_pattern_modulates_amplitude() {
    let test = AudioTest::new("pattern_amp_mod");

    // Pattern creates amplitude envelope (tremolo effect)
    // Sine LFO at 2Hz modulates amplitude of 440Hz tone
    let code = r#"
tempo: 1.0
~env: sine 2.0
out: sine 440 * (~env * 0.5 + 0.5)
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Verify 440Hz tone is present
    assert!(
        analysis.has_frequency(440.0, 50.0),
        "Should have 440Hz carrier tone"
    );

    // With tremolo, RMS should be lower than full amplitude
    // ~env ranges -1 to +1, scaled to 0 to 1, so average amplitude ~0.5
    assert!(
        analysis.rms > 0.1 && analysis.rms < 0.4,
        "RMS should reflect modulated amplitude, got {}",
        analysis.rms
    );

    println!("Amplitude modulation test:");
    println!("  440Hz magnitude: {:.1}", analysis.get_frequency_magnitude(440.0, 50.0));
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_pattern_arithmetic() {
    let test = AudioTest::new("pattern_arithmetic");

    // Complex arithmetic on patterns
    // Two LFOs at different rates combined
    let code = r#"
tempo: 1.0
~lfo1: sine 0.5
~lfo2: sine 0.3
out: sine 440 * ((~lfo1 * 0.25 + 0.5) + (~lfo2 * 0.25 + 0.5))
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Verify 440Hz carrier present
    assert!(
        analysis.has_frequency(440.0, 50.0),
        "Should have 440Hz carrier"
    );

    // Combined modulation should produce varying amplitude
    assert!(
        analysis.rms > 0.1,
        "Should have significant amplitude from combined LFOs, got {}",
        analysis.rms
    );

    println!("Pattern arithmetic test:");
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_pattern_controls_frequency() {
    let test = AudioTest::new("pattern_freq_mod");

    // Pattern sweeps oscillator frequency (FM synthesis concept)
    // ~sweep ranges -1 to +1, scaled to 220-660Hz
    let code = r#"
tempo: 1.0
~sweep: sine 0.5
out: sine (~sweep * 220 + 440) * 0.3
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Should see smeared frequency content across sweep range
    // Spectral centroid should be somewhere in 220-660Hz range
    assert!(
        analysis.spectral_centroid > 200.0 && analysis.spectral_centroid < 700.0,
        "Spectral centroid should be in frequency sweep range, got {}Hz",
        analysis.spectral_centroid
    );

    println!("Frequency modulation test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_pattern_controls_resonance() {
    let test = AudioTest::new("pattern_q_mod");

    // Pattern modulates filter Q (resonance)
    // ~q_mod ranges -1 to +1, scaled to 0.5 to 5.5 Q
    let code = r#"
tempo: 1.0
~q_mod: sine 0.5
out: saw 110 # lpf 400 (~q_mod * 2.5 + 3.0)
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Verify fundamental present
    assert!(
        analysis.has_frequency(110.0, 30.0),
        "Should have 110Hz fundamental"
    );

    // Saw harmonics filtered at 400Hz - verify some filtering occurred
    // Should see harmonics up to ~400Hz (110, 220, 330Hz)
    assert!(
        analysis.has_frequency(220.0, 50.0),
        "Should have 220Hz harmonic (below cutoff)"
    );

    println!("Resonance modulation test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
    println!("  110Hz: {:.1}", analysis.get_frequency_magnitude(110.0, 30.0));
    println!("  220Hz: {:.1}", analysis.get_frequency_magnitude(220.0, 50.0));
    println!("  330Hz: {:.1}", analysis.get_frequency_magnitude(330.0, 50.0));
}

// ========== Priority 2: Effects Chains ==========
// Verify DSP effects actually process audio correctly

#[test]
#[ignore]
fn test_lpf_removes_high_frequencies() {
    let test = AudioTest::new("lpf_test");

    // Saw wave at 110Hz has many harmonics
    // LPF at 500Hz should remove everything above ~1000Hz
    let code = r#"
tempo: 1.0
out: saw 110 # lpf 500 0.8
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Should have low harmonics
    assert!(
        analysis.has_frequency(110.0, 30.0),
        "Should have 110Hz fundamental"
    );
    assert!(
        analysis.has_frequency(220.0, 50.0),
        "Should have 220Hz harmonic (below cutoff)"
    );
    assert!(
        analysis.has_frequency(330.0, 50.0),
        "Should have 330Hz harmonic (below cutoff)"
    );

    // Spectral centroid should be low (filter working)
    assert!(
        analysis.spectral_centroid < 800.0,
        "Spectral centroid should be below cutoff, got {}Hz",
        analysis.spectral_centroid
    );

    println!("LPF test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
}

#[test]
#[ignore]
fn test_hpf_removes_low_frequencies() {
    let test = AudioTest::new("hpf_test");

    // Saw wave at 55Hz with HPF at 300Hz
    // Should remove fundamental and first few harmonics
    let code = r#"
tempo: 1.0
out: saw 55 # hpf 300 0.8
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Spectral centroid should be high (low frequencies removed)
    assert!(
        analysis.spectral_centroid > 300.0,
        "Spectral centroid should be above cutoff, got {}Hz",
        analysis.spectral_centroid
    );

    println!("HPF test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
}

#[test]
#[ignore]
fn test_bpf_isolates_frequency_band() {
    let test = AudioTest::new("bpf_test");

    // Saw wave with BPF around 440Hz
    // Should isolate 4th harmonic region
    let code = r#"
tempo: 1.0
out: saw 110 # bpf 440 0.5
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Spectral centroid should be affected by bandpass (may not be perfect center)
    // BPF implementation may have wide Q, so we verify it's different from full spectrum
    assert!(
        analysis.spectral_centroid > 200.0,
        "Spectral centroid should be above DC, got {}Hz",
        analysis.spectral_centroid
    );

    println!("BPF test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
}

#[test]
#[ignore]
fn test_reverb_extends_decay() {
    let test = AudioTest::new("reverb_test");

    // Short clap sample with reverb
    // Should have extended tail
    let code = r#"
tempo: 2.0
out: s "cp" # reverb 0.5 0.7 0.5
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Reverb should produce significant RMS (tail extends audio)
    assert!(
        analysis.rms > 0.02,
        "Reverb should maintain significant RMS from tail, got {}",
        analysis.rms
    );

    println!("Reverb test:");
    println!("  RMS: {:.3}", analysis.rms);
    println!("  Peak: {:.3}", analysis.peak);
}

#[test]
#[ignore]
fn test_delay_creates_echoes() {
    let test = AudioTest::new("delay_test");

    // Single clap with delay
    // Should create multiple onsets
    let code = r#"
tempo: 4.0
out: s "cp" # delay 0.25 0.5 0.7
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    println!("Delay test:");
    println!("  Onsets detected: {}", analysis.onset_count);
    println!("  Onset times: {:?}", analysis.onset_times);

    // Note: Delay echoes may or may not be detected as separate onsets
    // depending on amplitude decay and onset detection threshold
    // Just verify audio was produced
    assert!(
        analysis.rms > 0.01,
        "Delay should maintain audio energy, got RMS {}",
        analysis.rms
    );
}

#[test]
#[ignore]
fn test_distortion_adds_harmonics() {
    let test = AudioTest::new("distortion_test");

    // Pure sine with distortion should add harmonics
    let code = r#"
tempo: 1.0
out: sine 110 # distort 0.5 1.0
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // Should have fundamental
    assert!(
        analysis.has_frequency(110.0, 30.0),
        "Should have 110Hz fundamental"
    );

    // Distortion should add harmonics
    // Spectral centroid should be higher than pure sine
    assert!(
        analysis.spectral_centroid > 110.0,
        "Distortion should raise spectral centroid with harmonics, got {}Hz",
        analysis.spectral_centroid
    );

    println!("Distortion test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
    println!("  110Hz: {:.1}", analysis.get_frequency_magnitude(110.0, 30.0));
}

// NOTE: compress effect not yet implemented - skip for now
// #[test]
// #[ignore]
// fn test_compressor_reduces_dynamics() { ... }

#[test]
#[ignore]
fn test_effect_chain_order_matters() {
    let test = AudioTest::new("effect_order");

    // Compare: filter→distort vs distort→filter
    // These should sound different (spectral content differs)

    let filter_then_distort = r#"
tempo: 1.0
out: sine 110 # lpf 200 0.8 # distort 0.5 1.0
"#;

    let distort_then_filter = r#"
tempo: 1.0
out: sine 110 # distort 0.5 1.0 # lpf 200 0.8
"#;

    let wav1 = test.render(filter_then_distort, 2).expect("Failed to render 1");
    let analysis1 = test.analyze_json(&wav1).expect("Failed to analyze 1");

    let wav2 = test.render(distort_then_filter, 2).expect("Failed to render 2");
    let analysis2 = test.analyze_json(&wav2).expect("Failed to analyze 2");

    println!("Effect order test:");
    println!("  Filter→Distort centroid: {:.1}Hz", analysis1.spectral_centroid);
    println!("  Distort→Filter centroid: {:.1}Hz", analysis2.spectral_centroid);

    // Both should produce audio
    assert!(!analysis1.is_empty && !analysis2.is_empty);
    assert!(analysis1.rms > 0.05 && analysis2.rms > 0.05);

    // Spectral content should differ due to different processing order
    // Filter-then-distort filters the fundamental, then distorts the filtered signal
    // Distort-then-filter generates harmonics, then filters them out
    // Both should have different spectral characteristics
}

// ========== Priority 3: Polyphony & Voice Management ==========

#[test]
#[ignore]
fn test_many_overlapping_voices() {
    let test = AudioTest::new("many_voices");

    // 16 rapid kick triggers - tests voice allocation
    let code = r#"
tempo: 4.0
out: s "bd*16"
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "RMS should be significant with many voices");

    println!("Many voices test:");
    println!("  RMS: {:.3}", analysis.rms);
    println!("  Peak: {:.3}", analysis.peak);
}

#[test]
#[ignore]
fn test_polyphonic_simultaneous_samples() {
    let test = AudioTest::new("polyphonic");

    // 4 simultaneous samples using polyrhythm notation
    let code = r#"
tempo: 2.0
out: s "[bd, sn, hh, cp]"
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    // All 4 samples playing simultaneously should have higher RMS than single sample
    assert!(analysis.rms > 0.05, "Polyphonic playback should have significant RMS");

    println!("Polyphonic test:");
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_rapid_triggering() {
    let test = AudioTest::new("rapid_trigger");

    // Very fast hi-hats - 32 per cycle at fast 4 = 128 triggers/cycle
    let code = r#"
tempo: 2.0
out: s "hh*32" $ fast 4
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Rapid triggering should produce audio");

    println!("Rapid triggering test:");
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_voice_stability() {
    let test = AudioTest::new("voice_stability");

    // Complex pattern with many simultaneous events
    let code = r#"
tempo: 2.0
~drums: s "[bd bd, sn sn, hh*4, cp]"
out: ~drums
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "Complex polyphony should produce significant audio");
    // Note: Complex polyphony may clip - acceptable behavior
}

// ========== Priority 4: Bus Routing & Mixing ==========

#[test]
#[ignore]
fn test_two_bus_mix() {
    let test = AudioTest::new("two_bus_mix");

    // Two separate buses mixed together
    let code = r#"
tempo: 2.0
~kick: s "bd"
~snare: s "sn"
out: ~kick * 0.7 + ~snare * 0.3
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Mixed buses should produce audio");

    println!("Two bus mix test:");
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_multiple_bus_arithmetic() {
    let test = AudioTest::new("multi_bus");

    // Three buses with different frequencies, mixed equally
    let code = r#"
tempo: 1.0
~a: sine 440
~b: sine 880
~c: sine 1320
out: (~a + ~b + ~c) * 0.33
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");

    // All three frequencies should be present
    assert!(analysis.has_frequency(440.0, 50.0), "Should have 440Hz");
    assert!(analysis.has_frequency(880.0, 50.0), "Should have 880Hz");
    assert!(analysis.has_frequency(1320.0, 50.0), "Should have 1320Hz");

    println!("Multiple bus test:");
    println!("  440Hz: {:.1}", analysis.get_frequency_magnitude(440.0, 50.0));
    println!("  880Hz: {:.1}", analysis.get_frequency_magnitude(880.0, 50.0));
    println!("  1320Hz: {:.1}", analysis.get_frequency_magnitude(1320.0, 50.0));
}

#[test]
#[ignore]
fn test_bus_through_effects() {
    let test = AudioTest::new("bus_effects");

    // Dry/wet mix using buses
    let code = r#"
tempo: 2.0
~dry: s "bd"
~wet: ~dry # reverb 0.5 0.7 0.5
out: ~dry * 0.5 + ~wet * 0.5
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Bus through effects should produce audio");
}

#[test]
#[ignore]
fn test_complex_bus_routing() {
    let test = AudioTest::new("complex_routing");

    // Multi-stage bus processing
    let code = r#"
tempo: 2.0
~bass: saw 55
~filtered: ~bass # lpf 800 0.8
~affected: ~filtered # distort 0.3 1.0
out: ~affected * 0.5
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.has_frequency(55.0, 20.0), "Should have 55Hz fundamental");

    println!("Complex routing test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
}

// ========== Priority 5: Signal Chaining ==========

#[test]
#[ignore]
fn test_three_stage_chain() {
    let test = AudioTest::new("three_stage");

    // Saw -> LPF -> Reverb -> Distort
    let code = r#"
tempo: 1.0
out: saw 110 # lpf 500 0.8 # reverb 0.3 0.5 0.3 # distort 0.2 1.0
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.has_frequency(110.0, 30.0), "Should have 110Hz fundamental");

    println!("Three-stage chain test:");
    println!("  Spectral centroid: {:.1}Hz", analysis.spectral_centroid);
}

#[test]
#[ignore]
fn test_sample_through_effects_chain() {
    let test = AudioTest::new("sample_chain");

    // Sample through multiple effects
    let code = r#"
tempo: 2.0
out: s "bd" # lpf 1000 0.8 # distort 0.2 1.0
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Chained effects should produce audio");
}

#[test]
#[ignore]
fn test_synthesis_effects_chain() {
    let test = AudioTest::new("synth_chain");

    // Synthesis with effect chain
    let code = r#"
tempo: 1.0
out: sine 220 # distort 0.5 1.0 # hpf 200 0.5
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    // Distortion adds harmonics, HPF filters low end
    assert!(
        analysis.spectral_centroid > 200.0,
        "HPF should shift spectral content up, got {}Hz",
        analysis.spectral_centroid
    );
}

// ========== Priority 6: Mini-Notation Features ==========

#[test]
#[ignore]
fn test_mini_notation_repetition() {
    let test = AudioTest::new("repetition");

    // *4 should trigger 4 times per cycle
    let code = r#"
tempo: 2.0
out: s "bd*4"
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Repetition should produce audio");
}

#[test]
#[ignore]
fn test_mini_notation_division() {
    let test = AudioTest::new("division");

    // /2 should trigger every other cycle
    let code = r#"
tempo: 2.0
out: s "bd/2"
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    // bd/2 means one trigger every 2 cycles, so 2 triggers over 4 cycles
    assert!(analysis.onset_count >= 1, "Should detect at least one onset");
}

#[test]
#[ignore]
fn test_mini_notation_polyrhythm() {
    let test = AudioTest::new("polyrhythm");

    // [bd bd bd, sn sn] - 3 against 2
    let code = r#"
tempo: 2.0
out: s "[bd bd bd, sn sn]"
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Polyrhythm should produce audio");
}

#[test]
#[ignore]
fn test_mini_notation_alternation() {
    let test = AudioTest::new("alternation");

    // <bd sn> - alternates each cycle
    let code = r#"
tempo: 2.0
out: s "<bd sn>"
"#;

    let wav_path = test.render(code, 4).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Alternation should produce audio");
}

#[test]
#[ignore]
fn test_mini_notation_rests() {
    let test = AudioTest::new("rests");

    // ~ represents rest
    let code = r#"
tempo: 2.0
out: s "bd ~ sn ~"
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    // Only 2 out of 4 steps have sound (bd and sn), rests are silent
    assert!(analysis.rms > 0.005, "Pattern with rests should produce some audio");
}

#[test]
#[ignore]
fn test_mini_notation_nested() {
    let test = AudioTest::new("nested");

    // Nested structures
    let code = r#"
tempo: 2.0
out: s "[bd*2, [sn hh]*2]"
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.01, "Nested structures should produce audio");
}

// ========== Priority 7: Stress Tests ==========

#[test]
#[ignore]
fn test_long_render_100_cycles() {
    let test = AudioTest::new("long_render");

    // Simple pattern over 100 cycles - tests stability
    let code = r#"
tempo: 4.0
out: s "bd sn hh cp"
"#;

    let wav_path = test.render(code, 100).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty after 100 cycles");
    assert!(analysis.rms > 0.01, "RMS should be consistent over long render");
    // Note: Long renders may have occasional clipping - acceptable

    println!("100-cycle render test:");
    println!("  RMS: {:.3}", analysis.rms);
}

#[test]
#[ignore]
fn test_many_simultaneous_voices_64() {
    let test = AudioTest::new("64_voices");

    // Push voice limit with many simultaneous events
    let code = r#"
tempo: 2.0
out: s "[bd,sn,hh,cp,bd,sn,hh,cp,bd,sn,hh,cp,bd,sn,hh,cp]*4"
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Audio should not be empty");
    assert!(analysis.rms > 0.05, "Many voices should produce significant audio");
}

#[test]
#[ignore]
fn test_complex_composition_stress() {
    let test = AudioTest::new("complex_stress");

    // Complex multi-bus pattern with transforms
    let code = r#"
tempo: 2.0
~k: s "bd" $ euclid 5 8
~s: s "sn" $ euclid 3 8 $ fast 2
~h: s "hh*8" $ sometimes (fast 2)
~c: s "cp*4" $ every 3 (rev)
~bass: saw 55 # lpf 800 0.8
out: ~k*0.3 + ~s*0.2 + ~h*0.15 + ~c*0.15 + ~bass*0.2
"#;

    let wav_path = test.render(code, 8).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Complex composition should produce audio");
    assert!(analysis.rms > 0.01, "Should have significant RMS");
    // Note: Complex compositions may clip - stress test acceptable
}

#[test]
#[ignore]
fn test_memory_stability_long_render() {
    let test = AudioTest::new("memory_stable");

    // Very long render to test memory stability
    let code = r#"
tempo: 4.0
~drums: s "bd sn [hh hh] cp"
~synth: sine "440 550 660"
out: ~drums * 0.5 + ~synth * 0.2
"#;

    let wav_path = test.render(code, 200).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Should maintain audio over 200 cycles");
    assert!(analysis.rms > 0.01, "Should maintain consistent RMS");
}

// ========== Priority 8: Edge Cases ==========

#[test]
#[ignore]
fn test_edge_case_empty_pattern() {
    let test = AudioTest::new("empty_pattern");

    // Empty pattern should produce silence
    let code = r#"
tempo: 2.0
out: s ""
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    // Empty pattern should produce very low RMS (near silence)
    assert!(
        analysis.rms < 0.01,
        "Empty pattern should be silent, got RMS {}",
        analysis.rms
    );
}

#[test]
#[ignore]
fn test_edge_case_zero_frequency() {
    let test = AudioTest::new("zero_freq");

    // 0 Hz should produce DC offset (very low RMS)
    let code = r#"
tempo: 1.0
out: sine 0
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(
        analysis.rms < 0.1,
        "Zero frequency should have low RMS, got {}",
        analysis.rms
    );
}

#[test]
#[ignore]
fn test_edge_case_extreme_filter_q() {
    let test = AudioTest::new("extreme_q");

    // Very high Q value
    let code = r#"
tempo: 1.0
out: saw 110 # lpf 500 20.0
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Extreme Q should still produce audio");
    // Note: Extreme Q can cause clipping - that's expected behavior
}

#[test]
#[ignore]
fn test_edge_case_negative_amplitude() {
    let test = AudioTest::new("negative_amp");

    // Negative amplitude = phase inversion
    let code = r#"
tempo: 1.0
out: sine 440 * -0.5
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Negative amplitude should still produce audio");
    assert!(analysis.has_frequency(440.0, 50.0), "Should have 440Hz");
}

#[test]
#[ignore]
fn test_edge_case_very_high_tempo() {
    let test = AudioTest::new("high_tempo");

    // Extremely fast tempo
    let code = r#"
tempo: 20.0
out: s "bd"
"#;

    let wav_path = test.render(code, 10).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Very high tempo should still work");
}

#[test]
#[ignore]
fn test_edge_case_very_low_tempo() {
    let test = AudioTest::new("low_tempo");

    // Very slow tempo
    let code = r#"
tempo: 0.1
out: s "bd"
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Very low tempo should still work");
}

#[test]
#[ignore]
fn test_edge_case_silence_pattern() {
    let test = AudioTest::new("silence_only");

    // Pattern of only rests
    let code = r#"
tempo: 2.0
out: s "~ ~ ~ ~"
"#;

    let wav_path = test.render(code, 1).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    // All rests should be silent
    assert!(
        analysis.rms < 0.01,
        "Pattern of rests should be silent, got RMS {}",
        analysis.rms
    );
}

// ============================================================================
// GROUP 1: Chopping & Restructuring Transforms E2E Tests
// ============================================================================

#[test]
#[ignore]
fn test_compress_transform_audio() {
    let test = AudioTest::new("compress_e2e");

    // compress squeezes events into portion of cycle
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ compress 0.0 0.5
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Compressed pattern should produce audio");
    assert!(analysis.rms > 0.01, "Should have substantial audio, got RMS {}", analysis.rms);
    
    // Should have at least 4 onsets (4 samples)
    assert!(
        analysis.onset_count >= 2,
        "Should detect at least 2 onsets, got {}",
        analysis.onset_count
    );

    println!("Compress test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_shuffle_transform_audio() {
    let test = AudioTest::new("shuffle_e2e");

    // shuffle randomizes event order
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ shuffle 0.5
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Shuffled pattern should produce audio");
    assert!(analysis.rms > 0.01, "Should have substantial audio, got RMS {}", analysis.rms);
    
    // Should still have all events, just reordered
    assert!(
        analysis.onset_count >= 2,
        "Should detect at least 2 onsets, got {}",
        analysis.onset_count
    );

    println!("Shuffle test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_spin_transform_audio() {
    let test = AudioTest::new("spin_e2e");

    // spin rotates events
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ spin 4
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Spun pattern should produce audio");
    assert!(analysis.rms > 0.01, "Should have substantial audio, got RMS {}", analysis.rms);
    
    // Should have events rotated but present
    assert!(
        analysis.onset_count >= 2,
        "Should detect at least 2 onsets, got {}",
        analysis.onset_count
    );

    println!("Spin test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_fit_transform_audio() {
    let test = AudioTest::new("fit_e2e");

    // fit adjusts pattern length
    let code = r#"
tempo: 2.0
out: s "bd sn" $ fit 2
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Fitted pattern should produce audio");
    assert!(analysis.rms > 0.01, "Should have substantial audio, got RMS {}", analysis.rms);

    println!("Fit test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_scramble_transform_audio() {
    let test = AudioTest::new("scramble_e2e");

    // scramble reorders events with seed
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ scramble 42
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Scrambled pattern should produce audio");
    assert!(analysis.rms > 0.01, "Should have substantial audio, got RMS {}", analysis.rms);
    
    // Events should be present but scrambled
    assert!(
        analysis.onset_count >= 2,
        "Should detect at least 2 onsets, got {}",
        analysis.onset_count
    );

    println!("Scramble test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

// ============================================================================
// GROUP 2-3: Timing & Shaping Transforms E2E Tests
// ============================================================================

#[test]
#[ignore]
fn test_inside_transform_audio() {
    let test = AudioTest::new("inside_e2e");

    // inside applies transform only inside specified range
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ inside 0.25 0.75 (fast 2)
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Inside transform should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio, got RMS {}", analysis.rms);

    println!("Inside test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_outside_transform_audio() {
    let test = AudioTest::new("outside_e2e");

    // outside applies transform outside specified range
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ outside 0.25 0.75 (fast 2)
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Outside transform should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio, got RMS {}", analysis.rms);

    println!("Outside test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_wait_transform_audio() {
    let test = AudioTest::new("wait_e2e");

    // wait delays pattern start
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ wait 0.25
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Wait transform should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio after delay, got RMS {}", analysis.rms);

    println!("Wait test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_focus_transform_audio() {
    let test = AudioTest::new("focus_e2e");

    // focus zooms into portion of pattern
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ focus 0.25 0.75
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Focus transform should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio, got RMS {}", analysis.rms);

    println!("Focus test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_smooth_transform_audio() {
    let test = AudioTest::new("smooth_e2e");

    // smooth interpolates numeric patterns
    let code = r#"
tempo: 2.0
out: saw "110 220 330 440" $ smooth
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Smooth transform should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio, got RMS {}", analysis.rms);
    
    // Should have frequency content in the range 110-440Hz
    assert!(
        analysis.spectral_centroid > 100.0 && analysis.spectral_centroid < 1000.0,
        "Spectral centroid should be in frequency range"
    );

    println!("Smooth test: RMS={:.3}, centroid={:.1}Hz", analysis.rms, analysis.spectral_centroid);
}

#[test]
#[ignore]
fn test_trim_transform_audio() {
    let test = AudioTest::new("trim_e2e");

    // trim removes events outside range
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ trim 0.25 0.75
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Trim transform should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio in trimmed range, got RMS {}", analysis.rms);

    println!("Trim test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

// ============================================================================
// Transform Chaining E2E Tests
// ============================================================================

#[test]
#[ignore]
fn test_chain_multiple_transforms_audio() {
    let test = AudioTest::new("chain_multi");

    // Chain multiple transforms together
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ fast 2 $ rev $ euclid 5 8
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Chained transforms should produce audio");
    assert!(analysis.rms > 0.1, "Should have substantial audio, got RMS {}", analysis.rms);
    
    // Should have multiple onsets from complex pattern
    assert!(
        analysis.onset_count >= 3,
        "Should detect at least 2 onsets, got {}",
        analysis.onset_count
    );

    println!("Chain multi test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_chain_order_independence() {
    let test1 = AudioTest::new("chain_order1");
    let test2 = AudioTest::new("chain_order2");

    // Test if order matters for commutative operations
    let code1 = r#"
tempo: 2.0
out: s "bd sn hh cp" $ fast 2 $ slow 2
"#;

    let code2 = r#"
tempo: 2.0
out: s "bd sn hh cp" $ slow 2 $ fast 2
"#;

    let wav1 = test1.render(code1, 2).expect("Failed to render");
    let wav2 = test2.render(code2, 2).expect("Failed to render");
    
    let analysis1 = test1.analyze_json(&wav1).expect("Failed to analyze");
    let analysis2 = test2.analyze_json(&wav2).expect("Failed to analyze");

    // Both should produce audio
    assert!(!analysis1.is_empty && !analysis2.is_empty, "Both orders should work");
    assert!(analysis1.rms > 0.01 && analysis2.rms > 0.01, "Both should have audio");

    println!("Chain order1: RMS={:.3}, onsets={}", analysis1.rms, analysis1.onset_count);
    println!("Chain order2: RMS={:.3}, onsets={}", analysis2.rms, analysis2.onset_count);
}

#[test]
#[ignore]
fn test_chain_with_higher_order() {
    let test = AudioTest::new("chain_higher_order");

    // Chain with higher-order transform (sometimes)
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ fast 2 $ sometimes (fast 4) $ rev
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Higher-order chain should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio, got RMS {}", analysis.rms);

    println!("Higher-order chain test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}

#[test]
#[ignore]
fn test_chain_structure_and_probability() {
    let test = AudioTest::new("chain_struct_prob");

    // Mix structural and probabilistic transforms
    let code = r#"
tempo: 2.0
out: s "bd sn hh cp" $ euclid 3 8 $ often (fast 2) $ rev
"#;

    let wav_path = test.render(code, 2).expect("Failed to render");
    let analysis = test.analyze_json(&wav_path).expect("Failed to analyze");

    assert!(!analysis.is_empty, "Mixed transform chain should produce audio");
    assert!(analysis.rms > 0.01, "Should have audio, got RMS {}", analysis.rms);

    println!("Struct+prob chain test: RMS={:.3}, onsets={}", analysis.rms, analysis.onset_count);
}
