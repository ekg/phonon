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
fn test_master_bus_auto_sum() {
    // Test that when ~master is undefined, all buses auto-sum to output
    let phonon_code = r#"
cps: 2.0
~drums: saw 110
~bass: saw 55
"#;

    fs::write("/tmp/test_master_auto_sum.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_master_auto_sum.phonon",
            "/tmp/test_master_auto_sum.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_master_auto_sum.wav");
    println!("{}", analysis);

    // Should contain audio from both drums and bass
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Auto-sum master should produce audio"
    );
}

#[test]
fn test_master_bus_explicit_mix() {
    // Test that explicit ~master definition controls the mix
    let phonon_code = r#"
cps: 2.0
~drums: saw 110
~bass: saw 55
~master: ~drums * 0.8 + ~bass * 0.2
"#;

    fs::write("/tmp/test_master_explicit.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_master_explicit.phonon",
            "/tmp/test_master_explicit.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_master_explicit.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Explicit master mix should produce audio"
    );
}

#[test]
fn test_master_bus_processing() {
    // Test that master bus can have effects applied
    let phonon_code = r#"
cps: 2.0
~drums: saw 110
~bass: saw 55
~master: (~drums + ~bass) # lpf 2000 0.8
"#;

    fs::write("/tmp/test_master_processing.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_master_processing.phonon",
            "/tmp/test_master_processing.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_master_processing.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Master with effects should produce audio"
    );
}

#[test]
fn test_send_aux_routing() {
    // Test send/aux pattern using buses
    let phonon_code = r#"
cps: 2.0
~drums: saw 110
~reverb_send: ~drums # reverb 0.8 0.5 0.3
~master: ~drums * 0.7 + ~reverb_send * 0.3
"#;

    fs::write("/tmp/test_send_aux.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_send_aux.phonon",
            "/tmp/test_send_aux.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_send_aux.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Send/aux routing should produce audio"
    );
}

#[test]
fn test_multi_output_routing() {
    // Test numbered outputs for multi-channel hardware
    let phonon_code = r#"
cps: 2.0
~drums: saw 110
~bass: saw 55
~out1: ~drums
~out2: ~bass
~out3: ~drums + ~bass
~master: ~out3
"#;

    fs::write("/tmp/test_multi_output.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_multi_output.phonon",
            "/tmp/test_multi_output.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Phonon render failed");
    }

    let analysis = analyze_wav("/tmp/test_multi_output.wav");
    println!("{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Multi-output routing should produce audio"
    );
}
