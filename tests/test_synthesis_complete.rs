use std::process::Command;
use std::fs;

fn analyze_wav(path: &str) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "wav_analyze", "--", path])
        .output()
        .expect("Failed to run wav_analyze");

    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_synth_with_adsr() {
    let phonon_code = r#"
cps: 2.0
~d1: synth "c4 e4 g4" "saw" 0.01 0.1 0.7 0.3
"#;

    fs::write("/tmp/test_synth_adsr.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", "/tmp/test_synth_adsr.phonon",
                "/tmp/test_synth_adsr.wav", "--duration", "2"])
        .output()
        .expect("Failed to run phonon render");

    assert!(output.status.success(), "Render should succeed");

    let analysis = analyze_wav("/tmp/test_synth_adsr.wav");
    assert!(analysis.contains("✅ Contains audio signal"),
            "Synth with ADSR should produce audio");
}

#[test]
fn test_synth_with_per_channel_effects() {
    let phonon_code = r#"
cps: 2.0
~d1: synth "c4*4" "saw" 0.01 0.1 0.7 0.3 # lpf 1200 0.8
"#;

    fs::write("/tmp/test_synth_effects.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", "/tmp/test_synth_effects.phonon",
                "/tmp/test_synth_effects.wav", "--duration", "2"])
        .output()
        .expect("Failed to run phonon render");

    assert!(output.status.success(), "Render should succeed");

    let analysis = analyze_wav("/tmp/test_synth_effects.wav");
    assert!(analysis.contains("✅ Contains audio signal"),
            "Synth with effects should produce audio");
}

#[test]
fn test_multi_channel_synth_with_master_effects() {
    let phonon_code = r#"
cps: 1.0
~d1: synth "c3*4" "square" 0.001 0.05 0.0 0.1
~d2: synth "c5 e5 g5" "saw" 0.01 0.1 0.7 0.3 # lpf 1200 0.8
~master: (~d1 * 0.6 + ~d2 * 0.4) # reverb 0.5 0.5 0.2
"#;

    fs::write("/tmp/test_multi_synth.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", "/tmp/test_multi_synth.phonon",
                "/tmp/test_multi_synth.wav", "--duration", "3"])
        .output()
        .expect("Failed to run phonon render");

    assert!(output.status.success(), "Render should succeed");

    let analysis = analyze_wav("/tmp/test_multi_synth.wav");
    assert!(analysis.contains("✅ Contains audio signal"),
            "Multi-channel synth with master effects should produce audio");
}

#[test]
fn test_send_return_with_synth() {
    let phonon_code = r#"
cps: 2.0
~drums: synth "c4*8" "saw" 0.001 0.05 0.0 0.1
~reverb_send: ~drums # reverb 0.8 0.5 1.0
~master: ~drums * 0.7 + ~reverb_send * 0.3
"#;

    fs::write("/tmp/test_send_return.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", "/tmp/test_send_return.phonon",
                "/tmp/test_send_return.wav", "--duration", "2"])
        .output()
        .expect("Failed to run phonon render");

    assert!(output.status.success(), "Render should succeed");

    let analysis = analyze_wav("/tmp/test_send_return.wav");
    assert!(analysis.contains("✅ Contains audio signal"),
            "Send/return routing with synth should produce audio");
}

#[test]
fn test_different_envelope_types() {
    let phonon_code = r#"
cps: 1.0
~percussive: synth "c4" "saw" 0.001 0.05 0.0 0.1
~sustained: synth "e4" "saw" 0.01 0.1 0.7 0.3
~pad: synth "g4" "sine" 0.5 0.3 0.8 1.0
~master: ~percussive + ~sustained + ~pad
"#;

    fs::write("/tmp/test_envelope_types.phonon", phonon_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "phonon", "--quiet", "--",
                "render", "/tmp/test_envelope_types.phonon",
                "/tmp/test_envelope_types.wav", "--duration", "3"])
        .output()
        .expect("Failed to run phonon render");

    assert!(output.status.success(), "Render should succeed");

    let analysis = analyze_wav("/tmp/test_envelope_types.wav");
    assert!(analysis.contains("✅ Contains audio signal"),
            "Different envelope types should all produce audio");
}
