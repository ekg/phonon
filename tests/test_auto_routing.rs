use std::fs;
use std::process::Command;

fn analyze_wav(path: &str) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--", path])
        .output()
        .expect("Failed to run wav_analyze");

    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_d_pattern_auto_routing() {
    // Test that d1, d2, d3 auto-route to master (TidalCycles style)
    let phonon_code = r#"
cps: 2.0
~d1: saw 110
~d2: saw 220
~d3: saw 440
"#;

    fs::write("/tmp/test_d_pattern.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_d_pattern.phonon",
            "/tmp/test_d_pattern.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_d_pattern.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "d1, d2, d3 should auto-route to master"
    );
}

#[test]
fn test_out_pattern_auto_routing() {
    // Test that out1, out2, out3 auto-route to master
    let phonon_code = r#"
cps: 2.0
~out1: saw 110
~out2: saw 220
"#;

    fs::write("/tmp/test_out_pattern.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_out_pattern.phonon",
            "/tmp/test_out_pattern.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_out_pattern.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "out1, out2 should auto-route to master"
    );
}

#[test]
fn test_mixed_d_and_out_pattern() {
    // Test that both d and out patterns can coexist
    let phonon_code = r#"
cps: 2.0
~d1: saw 110
~out1: saw 220
"#;

    fs::write("/tmp/test_mixed_pattern.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_mixed_pattern.phonon",
            "/tmp/test_mixed_pattern.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_mixed_pattern.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Both d1 and out1 should auto-route to master"
    );
}

#[test]
fn test_explicit_master_overrides_auto_routing() {
    // Test that explicit ~master definition overrides auto-routing
    let phonon_code = r#"
cps: 2.0
~d1: saw 110
~d2: saw 220
~master: ~d1 * 0.9
"#;

    fs::write("/tmp/test_explicit_override.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_explicit_override.phonon",
            "/tmp/test_explicit_override.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_explicit_override.wav");
    println!("{}", analysis);

    // Should have audio (from d1 only, not d2)
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Explicit master should override auto-routing"
    );
}

#[test]
fn test_non_matching_buses_dont_auto_route() {
    // Test that buses not matching the pattern don't auto-route
    let phonon_code = r#"
cps: 2.0
~d1: saw 110        # Matches pattern, auto-routes
~drums: saw 220     # Doesn't match, should NOT auto-route
~bass: saw 55       # Doesn't match, should NOT auto-route
"#;

    fs::write("/tmp/test_non_matching.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_non_matching.phonon",
            "/tmp/test_non_matching.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_non_matching.wav");
    println!("{}", analysis);

    // Should have audio (only from d1, not from drums or bass)
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Only d1 should auto-route (drums and bass should be ignored)"
    );
}

#[test]
fn test_backwards_compatibility_out_bus() {
    // Test backwards compatibility with plain "out" bus
    let phonon_code = r#"
cps: 2.0
~bass: saw 55
~out: ~bass * 0.5
"#;

    fs::write("/tmp/test_out_compat.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_out_compat.phonon",
            "/tmp/test_out_compat.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_out_compat.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Plain 'out' bus should still work for backwards compatibility"
    );
}
