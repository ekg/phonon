use std::process::Command;

/// Test that the n modifier can select different samples from a bank
#[test]
fn test_n_modifier_sample_selection() {
    println!("Testing n modifier for sample selection...");

    // Test DSL code using n modifier to select specific samples
    let phonon_code = r#"
tempo: 2.0

-- Select specific bd samples using n modifier
-- bd:0 is first sample, bd:1 is second, etc.
out: s "bd" # n "0 1 2 3"
"#;

    std::fs::write("/tmp/test_n_modifier.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_n_modifier.phonon",
            "/tmp/test_n_modifier.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "n modifier render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Analyze the output
    let analysis = analyze_wav("/tmp/test_n_modifier.wav");

    // Should have audio
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "n modifier produced no audio!\n{}",
        analysis
    );

    // Should detect kick events (4 different samples in 2 seconds at 2 cps = 4 cycles)
    let onset_count = extract_onset_count(&analysis);
    assert!(
        onset_count >= 3,
        "Too few onset events for n modifier pattern: {} (expected >=3)",
        onset_count
    );

    println!("✅ n modifier sample selection test passed");
}

/// Test that n modifier works with pattern sequences
#[test]
fn test_n_modifier_with_patterns() {
    println!("Testing n modifier with complex patterns...");

    let phonon_code = r#"
tempo: 2.0

-- Use n to sequence through samples
out: s "bd*4" # n "0 1 2 3"
"#;

    std::fs::write("/tmp/test_n_pattern.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_n_pattern.phonon",
            "/tmp/test_n_pattern.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "n modifier pattern render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_n_pattern.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "n modifier pattern produced no audio!\n{}",
        analysis
    );

    println!("✅ n modifier pattern test passed");
}

/// Test n modifier with constant value
#[test]
fn test_n_modifier_constant() {
    println!("Testing n modifier with constant value...");

    let phonon_code = r#"
tempo: 2.0

-- Select bd:2 (third sample) for all triggers
out: s "bd*4" # n 2
"#;

    std::fs::write("/tmp/test_n_constant.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_n_constant.phonon",
            "/tmp/test_n_constant.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "n modifier constant render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_n_constant.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "n modifier constant produced no audio!\n{}",
        analysis
    );

    println!("✅ n modifier constant test passed");
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
