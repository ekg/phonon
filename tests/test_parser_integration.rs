use std::fs;
use std::process::Command;

// Test complete drum kit mixing through the parser
#[test]
#[ignore = "Hangs indefinitely - fundsp noise() issue (same as fchorus)"]
fn test_drum_kit_mixing() {
    println!("Testing drum kit with multiple mixed signals...");

    let phonon_code = r#"
cps: 2.0

-- Drum kit with kick, snare, and hats
~kick: noise # lpf "100 ~ ~ ~ 100 ~ ~ ~" 20
~snare: noise # hpf "~ ~ 2000 ~" 10 # lpf 5000 5
~hats: noise # hpf 8000 10

-- Mix all drums
out: ~kick * 0.5 + ~snare * 0.3 + ~hats * 0.05
"#;

    fs::write("/tmp/test_drums.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_drums.phonon",
            "/tmp/test_drums.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "Drum kit render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Analyze
    let analysis = analyze_wav("/tmp/test_drums.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Drum kit produced no audio!\n{}",
        analysis
    );

    // Should have high frequency content from hats
    let centroid = extract_spectral_centroid(&analysis);
    assert!(
        centroid > 1000.0,
        "Spectral centroid too low for drums: {} Hz (expected >1000 Hz from hats)",
        centroid
    );

    // Should detect kick beats
    let onset_count = extract_onset_count(&analysis);
    assert!(
        onset_count >= 2,
        "Too few onset events for drum pattern: {} (expected >=2)",
        onset_count
    );

    println!("✅ Drum kit mixing test passed");
}

// Test that all arithmetic operations work together
#[test]
fn test_arithmetic_precedence() {
    println!("Testing arithmetic precedence and complex expressions...");

    let phonon_code = r#"
cps: 2.0

-- Test order of operations
~sig1: sine 220
~sig2: sine 440
~sig3: sine 880

-- Should be (sig1 * 0.3) + (sig2 * 0.2) + (sig3 * 0.1)
out: ~sig1 * 0.3 + ~sig2 * 0.2 + ~sig3 * 0.1
"#;

    fs::write("/tmp/test_precedence.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_precedence.phonon",
            "/tmp/test_precedence.wav",
            "--duration",
            "0.5",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "Precedence test render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_precedence.wav");

    // Check that signal levels are correct
    let rms = extract_rms(&analysis);
    // With gains 0.3, 0.2, 0.1 on sine waves, total RMS should be ~0.25
    assert!(
        rms > 0.15 && rms < 0.35,
        "RMS suggests wrong precedence: {} (expected 0.15-0.35)",
        rms
    );

    println!("✅ Arithmetic precedence test passed");
}

// Test bus references work without tilde prefix
#[test]
fn test_bus_reference_formats() {
    println!("Testing different bus reference formats...");

    let test_cases = vec![
        // With tilde prefix in definition and reference
        ("cps: 2.0\n~bass: saw 110\nout: ~bass * 0.2", "tilde_both"),
        // With tilde and no explicit output (auto-routing)
        ("cps: 2.0\n~bass: saw 110", "auto_route"),
        // Multiple buses mixed
        (
            "cps: 2.0\n~bass: saw 110\n~lead: square 440\nout: ~bass * 0.3 + ~lead * 0.1",
            "mixed",
        ),
    ];

    for (code, name) in test_cases {
        println!("  Testing case: {}", name);

        let path = format!("/tmp/test_bus_{}.phonon", name);
        let wav = format!("/tmp/test_bus_{}.wav", name);

        fs::write(&path, code).unwrap();

        let output = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "phonon",
                "--quiet",
                "--",
                "render",
                &path,
                &wav,
                "--duration",
                "0.5",
            ])
            .output()
            .expect("Failed to run phonon render");

        assert!(
            output.status.success(),
            "Bus format '{}' failed: {}",
            name,
            String::from_utf8_lossy(&output.stderr)
        );

        let analysis = analyze_wav(&wav);

        assert!(
            analysis.contains("✅ Contains audio signal"),
            "Bus format '{}' produced no audio!\n{}",
            name,
            analysis
        );

        // All should produce ~110 Hz
        let freq = extract_dominant_freq(&analysis);
        assert!(
            (freq - 110.0).abs() < 50.0,
            "Wrong frequency for '{}': {} Hz (expected ~110 Hz)",
            name,
            freq
        );
    }

    println!("✅ Bus reference format test passed");
}

// Test pattern modulation works in mixed signals
#[test]
fn test_pattern_modulation_in_mix() {
    println!("Testing pattern modulation in mixed signals...");

    let phonon_code = r#"
cps: 2.0

-- Pattern-modulated signals
~bass: saw "55 82.5" # lpf "500 1000" 3
~lead: square "440 550 660 550"

-- Mix them
out: ~bass * 0.4 + ~lead * 0.1
"#;

    fs::write("/tmp/test_pattern_mix.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_pattern_mix.phonon",
            "/tmp/test_pattern_mix.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "Pattern mix render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_pattern_mix.wav");

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Pattern mix produced no audio!\n{}",
        analysis
    );

    // With frequency patterns, the spectral centroid should vary
    // Square wave at 440-660 Hz + bass at 55-82.5 Hz should give reasonable centroid
    let centroid = extract_spectral_centroid(&analysis);
    assert!(
        centroid > 100.0 && centroid < 5000.0,
        "Spectral centroid suggests pattern issue: {} Hz (expected 100-5000 Hz)",
        centroid
    );

    println!("✅ Pattern modulation in mix test passed");
}

// Helper functions
fn analyze_wav(path: &str) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--quiet", "--", path])
        .output()
        .expect("Failed to run wav_analyze");

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn extract_rms(analysis: &str) -> f32 {
    for line in analysis.lines() {
        if line.contains("RMS Level:") {
            if let Some(start) = line.find("RMS Level:") {
                let rest = &line[start + 10..].trim();
                if let Some(end) = rest.find(' ') {
                    if let Ok(rms) = rest[..end].parse::<f32>() {
                        return rms;
                    }
                }
            }
        }
    }
    0.0
}

fn extract_dominant_freq(analysis: &str) -> f32 {
    for line in analysis.lines() {
        if line.contains("Dominant Freq:") {
            if let Some(start) = line.find("Dominant Freq:") {
                let rest = &line[start + 14..].trim();
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
