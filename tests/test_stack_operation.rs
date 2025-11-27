//! Test stack operation for combining patterns
//! This is THE KEY to per-voice operations in Phonon

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::process::Command;

// ========== Basic Stack Tests ==========

#[test]
fn test_stack_basic_oscillators() {
    println!("Testing basic stack with oscillators...");

    let code = r#"
tempo: 2.0
~stacked: stack [sine 220, sine 440]
out: ~stacked * 0.2
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second
    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("Stacked oscillators RMS: {}", rms);

    assert!(rms > 0.05, "Stacked oscillators should produce audio");
}

#[test]
fn test_stack_with_different_gains() {
    println!("Testing stack with per-voice gain control...");

    // This is the key use case: per-voice gain!
    let code = r#"
tempo: 2.0
~loud: sine 220 * 0.8
~quiet: sine 440 * 0.2
~stacked: stack [~loud, ~quiet]
out: ~stacked
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    eprintln!("Stack with different gains RMS: {}", rms);
    assert!(rms > 0.15, "Stack should combine both signals");
    assert!(
        rms < 0.7,
        "But shouldn't be too loud (0.8 + 0.2 = ~0.6 RMS expected)"
    );
}

#[test]
fn test_stack_samples() {
    println!("Testing stack with sample patterns...");

    let code = r#"
tempo: 2.0
~kick: s "bd"
~snare: s "~ sn"
~hh: s "hh*4"
~drums: stack [~kick, ~snare, ~hh]
out: ~drums * 0.3
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(88200); // 2 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    eprintln!("Stacked drums RMS: {}", rms);
    assert!(rms > 0.01, "Stacked drum pattern should produce audio");
}

#[test]
fn test_stack_with_transforms() {
    println!("Testing stack with pattern transforms...");

    let code = r#"
tempo: 2.0
~normal: s "bd sn"
~fast: s "bd sn" $ fast 2
~stacked: stack [~normal, ~fast]
out: ~stacked * 0.5
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(88200); // 2 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    eprintln!("Stack with transforms RMS: {}", rms);
    assert!(rms > 0.01, "Stack with transforms should work");
}

// ========== Audio Analysis E2E Tests ==========

#[test]
fn test_stack_per_voice_gain_e2e() {
    println!("Testing per-voice gain control via stack (E2E)...");

    let code = r#"
tempo: 2.0
# Three samples with different gain levels
~kick: s "bd" * 0.8
~snare: s "~ sn" * 1.0
~hh: s "hh*4" * 0.4
~drums: stack [~kick, ~snare, ~hh]
out: ~drums
"#;

    std::fs::write("/tmp/test_stack_gain.ph", code).unwrap();

    // Render with phonon
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_stack_gain.ph",
            "/tmp/test_stack_gain.wav",
            "--duration",
            "2",
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        panic!("Render failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Analyze audio
    let analysis = analyze_wav("/tmp/test_stack_gain.wav");

    eprintln!("Stack E2E Analysis:\n{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Stack should produce audio"
    );

    // Should have multiple onset events (kick, snare, hi-hats)
    let onset_count = extract_onset_count(&analysis);
    assert!(
        onset_count > 5,
        "Should detect multiple drum hits, got {}",
        onset_count
    );

    // Check RMS is reasonable
    let rms = extract_rms(&analysis);
    assert!(
        rms > 0.05,
        "Stack should produce audible output, got RMS={}",
        rms
    );
}

#[test]
fn test_stack_oscillator_frequency_blend() {
    println!("Testing stack creates frequency blend (E2E)...");

    // Stack two frequencies - should see both in spectrum
    let code = r#"
tempo: 2.0
~low: sine 110 * 0.3
~high: sine 440 * 0.3
~blend: stack [~low, ~high]
out: ~blend
"#;

    std::fs::write("/tmp/test_stack_freq.ph", code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_stack_freq.ph",
            "/tmp/test_stack_freq.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon");

    assert!(
        output.status.success(),
        "Render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let analysis = analyze_wav("/tmp/test_stack_freq.wav");
    eprintln!("Frequency blend analysis:\n{}", analysis);

    assert!(analysis.contains("✅ Contains audio signal"));

    let rms = extract_rms(&analysis);
    assert!(
        rms > 0.15 && rms < 0.45,
        "Should blend both frequencies, RMS={}",
        rms
    );
}

#[test]
fn test_stack_three_way_mix() {
    println!("Testing stack with three patterns (E2E)...");

    let code = r#"
tempo: 2.0
~a: sine 220 * 0.2
~b: sine 330 * 0.2
~c: sine 440 * 0.2
~mix: stack [~a, ~b, ~c]
out: ~mix
"#;

    std::fs::write("/tmp/test_stack_three.ph", code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_stack_three.ph",
            "/tmp/test_stack_three.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon");

    assert!(output.status.success());

    let analysis = analyze_wav("/tmp/test_stack_three.wav");
    eprintln!("Three-way stack:\n{}", analysis);

    assert!(analysis.contains("✅ Contains audio signal"));

    // Should have reasonable RMS from all three (3 * 0.2 = 0.6 amplitude, but with interference ~0.15-0.25 RMS)
    let rms = extract_rms(&analysis);
    assert!(
        rms > 0.15,
        "Three-way mix should have good level, got {}",
        rms
    );
}

// ========== Helper Functions ==========

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
