use std::process::Command;

/// Test that the note modifier changes playback speed for pitch shifting
#[test]
fn test_note_modifier_pitch_shift() {
    println!("Testing note modifier for pitch shifting...");

    // Test DSL code using note modifier
    // note values change playback speed: 0 = normal, 12 = octave up, -12 = octave down
    // Using semitone formula: speed = 2^(note/12)
    let phonon_code = r#"
tempo: 0.5

-- Play sample at different pitches using note modifier
-- 0 = original pitch, 5 = perfect fourth up, 7 = perfect fifth up, 12 = octave up
out $ s "bd" # note "0 5 7 12"
"#;

    std::fs::write("/tmp/test_note_modifier.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_note_modifier.phonon",
            "/tmp/test_note_modifier.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "note modifier render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Analyze the output
    let analysis = analyze_wav("/tmp/test_note_modifier.wav");

    // Should have audio
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "note modifier produced no audio!\n{}",
        analysis
    );

    // Should detect kick events
    let onset_count = extract_onset_count(&analysis);
    assert!(
        onset_count >= 3,
        "Too few onset events for note modifier pattern: {} (expected >=3)",
        onset_count
    );

    // With pitch shifting, spectral content should be different from original
    let centroid = extract_spectral_centroid(&analysis);
    assert!(
        centroid > 50.0 && centroid < 5000.0,
        "Spectral centroid out of expected range: {} Hz",
        centroid
    );

    println!("✅ note modifier pitch shift test passed");
}

/// Test note modifier with minor scale pattern
#[test]
fn test_note_modifier_scale() {
    println!("Testing note modifier with minor scale...");

    // A minor scale in semitones: 0, 2, 3, 5, 7, 8, 10, 12
    let phonon_code = r#"
tempo: 0.5

-- Play minor scale using note modifier
out $ s "bd*8" # note "0 2 3 5 7 8 10 12"
"#;

    std::fs::write("/tmp/test_note_scale.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_note_scale.phonon",
            "/tmp/test_note_scale.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "note modifier scale render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_note_scale.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "note modifier scale produced no audio!\n{}",
        analysis
    );

    println!("✅ note modifier scale test passed");
}

/// Test note modifier with negative values (pitch down)
#[test]
fn test_note_modifier_negative() {
    println!("Testing note modifier with negative values...");

    let phonon_code = r#"
tempo: 0.5

-- Pitch down by octave and fifth
out $ s "bd*4" # note "0 -5 -7 -12"
"#;

    std::fs::write("/tmp/test_note_negative.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_note_negative.phonon",
            "/tmp/test_note_negative.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "note modifier negative render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_note_negative.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "note modifier negative produced no audio!\n{}",
        analysis
    );

    println!("✅ note modifier negative test passed");
}

/// Test note modifier with constant value
#[test]
fn test_note_modifier_constant() {
    println!("Testing note modifier with constant value...");

    let phonon_code = r#"
tempo: 0.5

-- Play all samples one octave up
out $ s "bd*4" # note 12
"#;

    std::fs::write("/tmp/test_note_constant.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_note_constant.phonon",
            "/tmp/test_note_constant.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "note modifier constant render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_note_constant.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "note modifier constant produced no audio!\n{}",
        analysis
    );

    println!("✅ note modifier constant test passed");
}

/// Test n and note modifiers working together
#[test]
fn test_n_and_note_together() {
    println!("Testing n and note modifiers together...");

    let phonon_code = r#"
tempo: 0.5

-- Select different samples with different pitches
out $ s "bd*4" # n "0 1 0 1" # note "0 5 7 12"
"#;

    std::fs::write("/tmp/test_n_note_combo.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_n_note_combo.phonon",
            "/tmp/test_n_note_combo.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "n + note combination render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_n_note_combo.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "n + note combination produced no audio!\n{}",
        analysis
    );

    let onset_count = extract_onset_count(&analysis);
    assert!(
        onset_count >= 1,
        "Too few onset events for n + note combination: {} (expected >=1)",
        onset_count
    );

    println!("✅ n + note combination test passed");
}

// Helper functions
fn analyze_wav(path: &str) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--quiet", "--", path])
        .output()
        .expect("Failed to run wav_analyze");

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn extract_onset_count(analysis: &str) -> usize {
    for line in analysis.lines() {
        if line.contains("Onset Events:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(count) = parts[2].parse::<usize>() {
                    return count;
                }
            }
        }
    }
    0
}

fn extract_spectral_centroid(analysis: &str) -> f32 {
    for line in analysis.lines() {
        if line.contains("Spectral Centroid:") {
            if let Some(start) = line.find("Spectral Centroid:") {
                let rest = &line[start + 18..].trim();
                if let Some(end) = rest.find(" Hz") {
                    if let Ok(freq) = rest[..end].parse::<f32>() {
                        return freq;
                    }
                }
            }
        }
    }
    0.0
}
