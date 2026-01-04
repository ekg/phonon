//! Tests for SynthPattern transposition (n modifier)

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn render_and_get_dominant_freq(code: &str, duration: f32) -> Option<f32> {
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let wav_path = format!("/tmp/transpose_test_{}.wav", test_id);

    let mut child = Command::new("./target/release/phonon")
        .args(["render", "-", &wav_path, "-d", &duration.to_string()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    use std::io::Write;
    child.stdin.take()?.write_all(code.as_bytes()).ok()?;
    let result = child.wait_with_output().ok()?;

    if !result.status.success() {
        eprintln!("Render failed: {}", String::from_utf8_lossy(&result.stderr));
        return None;
    }

    // Analyze the output
    let analyze = Command::new("./target/release/wav_analyze")
        .arg(&wav_path)
        .output()
        .ok()?;

    let output_str = String::from_utf8_lossy(&analyze.stdout);
    for line in output_str.lines() {
        if line.contains("Dominant Freq:") {
            let freq_str = line.split(':').nth(1)?.trim();
            let freq_str = freq_str.trim_end_matches(" Hz");
            return freq_str.parse().ok();
        }
    }
    None
}

#[test]
fn test_synth_pattern_n_transposition_zero() {
    // C4 with no transposition should be ~262 Hz
    let freq = render_and_get_dominant_freq("out $ saw \"c4\" # n \"0\"", 0.5);
    assert!(freq.is_some(), "Should render audio");
    let freq = freq.unwrap();
    assert!(freq > 250.0 && freq < 275.0, "C4 should be ~262 Hz, got {} Hz", freq);
}

#[test]
fn test_synth_pattern_n_transposition_octave_up() {
    // C4 + 12 semitones = C5 (~523 Hz)
    let freq = render_and_get_dominant_freq("out $ saw \"c4\" # n \"12\"", 0.5);
    assert!(freq.is_some(), "Should render audio");
    let freq = freq.unwrap();
    assert!(freq > 500.0 && freq < 550.0, "C5 should be ~523 Hz, got {} Hz", freq);
}

#[test]
fn test_synth_pattern_n_transposition_octave_down() {
    // C4 - 12 semitones = C3 (~131 Hz)
    let freq = render_and_get_dominant_freq("out $ saw \"c4\" # n \"-12\"", 0.5);
    assert!(freq.is_some(), "Should render audio");
    let freq = freq.unwrap();
    assert!(freq > 120.0 && freq < 145.0, "C3 should be ~131 Hz, got {} Hz", freq);
}

#[test]
fn test_mtof_pattern_arithmetic() {
    // mtof converts MIDI to frequency after pattern addition
    // C4 (60) + 0 = 60 -> ~262 Hz
    let freq = render_and_get_dominant_freq("out $ saw (mtof (\"c4\" + \"0\"))", 0.5);
    assert!(freq.is_some(), "Should render audio");
    let freq = freq.unwrap();
    assert!(freq > 250.0 && freq < 275.0, "C4 via mtof should be ~262 Hz, got {} Hz", freq);
}

#[test]
fn test_mtof_pattern_arithmetic_transpose() {
    // C4 (60) + 12 = 72 -> C5 (~523 Hz)
    let freq = render_and_get_dominant_freq("out $ saw (mtof (\"c4\" + \"12\"))", 0.5);
    assert!(freq.is_some(), "Should render audio");
    let freq = freq.unwrap();
    assert!(freq > 500.0 && freq < 550.0, "C5 via mtof should be ~523 Hz, got {} Hz", freq);
}
