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
tempo: 0.5
~stacked $ stack [sine 220, sine 440]
out $ ~stacked * 0.2
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
tempo: 0.5
~loud $ sine 220 * 0.8
~quiet $ sine 440 * 0.2
~stacked $ stack [~loud, ~quiet]
out $ ~stacked
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
tempo: 0.5
~kick $ s "bd"
~snare $ s "~ sn"
~hh $ s "hh*4"
~drums $ stack [~kick, ~snare, ~hh]
out $ ~drums * 0.3
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
tempo: 0.5
~normal $ s "bd sn"
~fast $ s "bd sn" $ fast 2
~stacked $ stack [~normal, ~fast]
out $ ~stacked * 0.5
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

    // NOTE: Phonon comments use `--`; `#` is the chain operator (it would be a
    // parse error here). Three sample voices at DISTINCT per-voice gains, layered
    // with `stack`, which SUMS its inputs (superposition).
    let code = r#"
tempo: 0.5
-- Three samples with different gain levels (per-voice gain)
~kick $ s "bd" * 0.8
~snare $ s "~ sn" * 1.0
~hh $ s "hh*4" * 0.4
~drums $ stack [~kick, ~snare, ~hh]
out $ ~drums
"#;
    std::fs::write("/tmp/test_stack_gain.ph", code).unwrap();
    render_ph("/tmp/test_stack_gain.ph", "/tmp/test_stack_gain.wav", 2);

    // A single voice (the loudest, snare at gain 1.0) as a summing baseline.
    let single = r#"
tempo: 0.5
out $ s "~ sn" * 1.0
"#;
    std::fs::write("/tmp/test_stack_single.ph", single).unwrap();
    render_ph("/tmp/test_stack_single.ph", "/tmp/test_stack_single.wav", 2);

    let analysis = analyze_wav("/tmp/test_stack_gain.wav");
    let single_analysis = analyze_wav("/tmp/test_stack_single.wav");
    eprintln!("Stack E2E Analysis:\n{}", analysis);

    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Stack should produce audio"
    );

    let stack_rms = extract_rms(&analysis);
    let single_rms = extract_rms(&single_analysis);
    eprintln!("stack RMS = {}, single-voice RMS = {}", stack_rms, single_rms);

    // Audible output.
    assert!(
        stack_rms > 0.05,
        "Stack should produce audible output, got RMS={}",
        stack_rms
    );

    // The heart of this task: `stack` SUMS voices (superposition), it does NOT
    // average them. Summing three voices must yield a level at least as high as
    // the single loudest voice on its own; the old averaging bug (sum / N) would
    // have made the 3-voice stack roughly a third as loud as this baseline.
    // (We do not assert an onset count here: the statistical onset detector uses
    // a relative energy threshold and cannot resolve the quieter hi-hats under a
    // dominant kick — that is a detector limitation orthogonal to `stack`.)
    assert!(
        stack_rms > single_rms,
        "stack RMS ({}) must exceed the single loudest voice RMS ({}) — proves \
         summing, not averaging",
        stack_rms,
        single_rms
    );
}

/// Render a `.ph` file to a `.wav` via the phonon CLI, panicking on failure.
fn render_ph(input: &str, output_wav: &str, duration_secs: u32) {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            input,
            output_wav,
            "--duration",
            &duration_secs.to_string(),
        ])
        .output()
        .expect("Failed to run phonon render");

    if !output.status.success() {
        panic!("Render failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn test_stack_oscillator_frequency_blend() {
    println!("Testing stack creates frequency blend (E2E)...");

    // Stack two frequencies - should see both in spectrum
    let code = r#"
tempo: 0.5
~low $ sine 110 * 0.3
~high $ sine 440 * 0.3
~blend $ stack [~low, ~high]
out $ ~blend
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
tempo: 0.5
~a $ sine 220 * 0.2
~b $ sine 330 * 0.2
~c $ sine 440 * 0.2
~mix $ stack [~a, ~b, ~c]
out $ ~mix
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

#[allow(dead_code)]
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
