/// Test bus triggering from mini-notation patterns
/// User wants to reference continuous synth signals like samples: s "~kick sn hh"

mod audio_verification_enhanced;
use audio_verification_enhanced::*;

#[test]
fn test_bus_trigger_simple() {
    // Test that we can trigger a bus from mini-notation
    // ~kick is a continuous sine wave, s "~kick" should gate it on/off
    let script = r#"
tempo: 1.0
~kick: sine 60
out: s "~kick" * 0.8
"#;

    std::fs::write("/tmp/test_bus_trigger.ph", script).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--", "render", "/tmp/test_bus_trigger.ph", "/tmp/test_bus_trigger.wav", "--duration", "2"])
        .output()
        .expect("Failed to execute render");

    assert!(output.status.success(), "Render failed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify we got audio output with clear transients
    let analysis = analyze_wav_enhanced("/tmp/test_bus_trigger.wav")
        .expect("Failed to analyze output");

    println!("Bus trigger analysis:");
    println!("  RMS: {:.6}", analysis.rms);
    println!("  Peak: {:.6}", analysis.peak);
    println!("  Dominant freq: {:.1} Hz", analysis.dominant_frequency);
    println!("  Onsets: {}", analysis.onset_count);

    // Should have audio
    assert!(analysis.peak > 0.1, "Bus trigger should produce audio, got peak: {}", analysis.peak);

    // Should have fundamental around 60 Hz
    assert!(
        (analysis.dominant_frequency - 60.0).abs() < 20.0,
        "Expected ~60 Hz, got {} Hz",
        analysis.dominant_frequency
    );

    // Should have at least 1 onset (the trigger event)
    assert!(analysis.onset_count >= 1, "Expected at least 1 onset, got {}", analysis.onset_count);
}

#[test]
fn test_bus_trigger_pattern() {
    // Test multiple bus triggers in a pattern
    let script = r#"
tempo: 1.0
~kick: sine 60
~snare: sine 200
out: s "~kick ~snare ~kick ~snare" * 0.8
"#;

    std::fs::write("/tmp/test_bus_trigger_pattern.ph", script).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--", "render", "/tmp/test_bus_trigger_pattern.ph", "/tmp/test_bus_trigger_pattern.wav", "--duration", "2"])
        .output()
        .expect("Failed to execute render");

    assert!(output.status.success(), "Render failed: {}", String::from_utf8_lossy(&output.stderr));

    let analysis = analyze_wav_enhanced("/tmp/test_bus_trigger_pattern.wav")
        .expect("Failed to analyze output");

    println!("Bus pattern analysis:");
    println!("  RMS: {:.6}", analysis.rms);
    println!("  Peak: {:.6}", analysis.peak);
    println!("  Onsets: {}", analysis.onset_count);

    // Should have clear audio
    assert!(analysis.peak > 0.1, "Bus pattern should produce audio");

    // Should have 4 onsets (4 events in pattern)
    assert!(
        analysis.onset_count >= 3,
        "Expected at least 3 onsets for 4-event pattern, got {}",
        analysis.onset_count
    );
}

#[test]
fn test_bus_trigger_mixed_with_samples() {
    // Test mixing bus triggers with regular samples
    let script = r#"
tempo: 1.0
~bass: sine 55
out: s "~bass bd ~bass sn" * 0.8
"#;

    std::fs::write("/tmp/test_bus_mixed.ph", script).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--", "render", "/tmp/test_bus_mixed.ph", "/tmp/test_bus_mixed.wav", "--duration", "2"])
        .output()
        .expect("Failed to execute render");

    assert!(output.status.success(), "Render failed: {}", String::from_utf8_lossy(&output.stderr));

    let analysis = analyze_wav_enhanced("/tmp/test_bus_mixed.wav")
        .expect("Failed to analyze output");

    println!("Mixed bus/sample analysis:");
    println!("  RMS: {:.6}", analysis.rms);
    println!("  Peak: {:.6}", analysis.peak);
    println!("  Onsets: {}", analysis.onset_count);

    // Should have audio with multiple events
    assert!(analysis.peak > 0.1, "Mixed pattern should produce audio");
    assert!(
        analysis.onset_count >= 3,
        "Expected at least 3 onsets, got {}",
        analysis.onset_count
    );
}

#[test]
fn test_bus_trigger_with_fast_subdivision() {
    // Test bus triggering with fast subdivision
    let script = r#"
tempo: 1.0
~hat: sine 8000
out: s "~hat*4" * 0.8
"#;

    std::fs::write("/tmp/test_bus_fast.ph", script).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--", "render", "/tmp/test_bus_fast.ph", "/tmp/test_bus_fast.wav", "--duration", "2"])
        .output()
        .expect("Failed to execute render");

    assert!(output.status.success(), "Render failed: {}", String::from_utf8_lossy(&output.stderr));

    let analysis = analyze_wav_enhanced("/tmp/test_bus_fast.wav")
        .expect("Failed to analyze output");

    println!("Fast subdivision analysis:");
    println!("  RMS: {:.6}", analysis.rms);
    println!("  Peak: {:.6}", analysis.peak);
    println!("  Onsets: {}", analysis.onset_count);

    // Should have audio
    assert!(analysis.peak > 0.1, "Fast pattern should produce audio");

    // Should have at least 3 onsets (4 events, but some may merge)
    assert!(
        analysis.onset_count >= 2,
        "Expected at least 2 onsets for fast pattern, got {}",
        analysis.onset_count
    );
}

#[test]
fn test_nonexistent_bus_graceful_failure() {
    // Test that referencing a non-existent bus doesn't crash
    let script = r#"
tempo: 1.0
out: s "~nonexistent bd" * 0.8
"#;

    std::fs::write("/tmp/test_bad_bus.ph", script).unwrap();

    let output = std::process::Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--", "render", "/tmp/test_bad_bus.ph", "/tmp/test_bad_bus.wav", "--duration", "2"])
        .output()
        .expect("Failed to execute render");

    // Should complete without crashing (but may have warnings)
    assert!(output.status.success(), "Should handle missing bus gracefully");

    // Output should still have the bd sample
    let analysis = analyze_wav_enhanced("/tmp/test_bad_bus.wav")
        .expect("Failed to analyze output");

    assert!(analysis.peak > 0.05, "Should still play bd sample");
}
